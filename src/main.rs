use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::env;
use std::fs;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{traits::*, HeapRb};
use gilrs::{Gilrs, Button, Axis};

use nes_emulator::bus::Bus;
use nes_emulator::cartridge::Cartridge;
use nes_emulator::cpu::Cpu;
use nes_emulator::joypad::JoypadButton;

fn main() {
    let (rom_path, cartridge) = load_rom();
    println!("Loaded: {}", rom_path);

    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    // TV dimensions for Sony Trinitron CRT frame (1080p scale for 4K monitors)
    const TV_WIDTH: usize = 1280;
    const TV_HEIGHT: usize = 960;
    const CONSOLE_HEIGHT: usize = 200; // Console overlay below TV
    const WINDOW_WIDTH: usize = TV_WIDTH;
    const WINDOW_HEIGHT: usize = TV_HEIGHT + CONSOLE_HEIGHT; // 1160 total
    const SCREEN_W: usize = 960;   // Exact 4:3 (960/720 = 4/3)
    const SCREEN_H: usize = 720;   // 3x NES height (240*3)
    const SCREEN_X: usize = 160;   // (1280 - 960) / 2
    const SCREEN_Y: usize = 70;    // Top bezel thinner than bottom
    
    let mut window = Window::new(
        "NES Emulator",
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
        WindowOptions {
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    )
    .expect("Failed to create window");

    window.set_target_fps(60);

    // Initialize gamepad support
    let mut gilrs = Gilrs::new().ok();
    if let Some(ref g) = gilrs {
        for (_id, gamepad) in g.gamepads() {
            println!("Controller: {} (connected: {})", gamepad.name(), gamepad.is_connected());
        }
    }

    // Audio ring buffer — lock-free, single producer / single consumer
    let ring = HeapRb::<f32>::new(8192); // ~170ms at 48kHz
    let (mut producer, mut consumer) = ring.split();
    let mut actual_sample_rate = 44100u32;

    let _stream = {
        let host = cpal::default_host();
        let device = host.default_output_device();

        if let Some(device) = device {
            let supported_config = device.default_output_config();
            match supported_config {
                Ok(supported) => {
                    let sample_rate = supported.sample_rate().0;
                    actual_sample_rate = sample_rate;
                    let channels = supported.channels() as usize;

                    let config = cpal::StreamConfig {
                        channels: channels as u16,
                        sample_rate: cpal::SampleRate(sample_rate),
                        buffer_size: cpal::BufferSize::Default,
                    };

                    let mut last_sample: f32 = 0.0;
                    let stream = device.build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            for frame in data.chunks_mut(channels) {
                                let sample = consumer.try_pop().unwrap_or_else(|| {
                                    last_sample *= 0.995;
                                    last_sample
                                });
                                last_sample = sample;
                                for s in frame.iter_mut() {
                                    *s = sample;
                                }
                            }
                        },
                        |err| eprintln!("Audio error: {}", err),
                        None,
                    );

                    match stream {
                        Ok(s) => {
                            let _ = s.play();
                            println!("Audio: {}Hz, {} channels", sample_rate, channels);
                            Some(s)
                        }
                        Err(e) => {
                            eprintln!("Audio disabled: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Audio disabled: {}", e);
                    None
                }
            }
        } else {
            eprintln!("No audio device found");
            None
        }
    };

    bus.set_apu_sample_rate(actual_sample_rate);
    println!("APU sample rate set to {}Hz", actual_sample_rate);

    // Pre-fill ring buffer to absorb timing jitter
    for _ in 0..800 {
        let _ = producer.try_push(0.0);
    }

    let mut crt_buffer: Vec<u32> = vec![0; SCREEN_W * SCREEN_H];
    
    // Build static TV frame once at startup (zero per-frame cost)
    let mut tv_frame_bg = Vec::new();
    build_tv_frame(&mut tv_frame_bg);
    build_console_overlay(&mut tv_frame_bg, TV_HEIGHT, WINDOW_WIDTH, WINDOW_HEIGHT);
    let mut composite_buffer = vec![0u32; WINDOW_WIDTH * WINDOW_HEIGHT];
    
    // Pre-compute vignette lookup table (same every frame)
    let vignette_table = {
        let mut table = vec![0u16; SCREEN_W * SCREEN_H];
        for y in 0..SCREEN_H {
            for x in 0..SCREEN_W {
                let fx = (x as f32 / SCREEN_W as f32) - 0.5;
                let fy = (y as f32 / SCREEN_H as f32) - 0.5;
                let v = 1.0 - (fx * fx + fy * fy) * 1.2;
                table[y * SCREEN_W + x] = (v.max(0.3).min(1.0) * 256.0) as u16;
            }
        }
        table
    };
    let mut crt_enabled = true;
    let mut mouse_was_down = false;

    while window.is_open() {
        loop {
            cpu.clock(&mut bus);
            bus.tick(1);
            bus.tick_apu();

            if bus.ppu.frame_complete() {
                break;
            }
        }

        // End APU frame and get band-limited samples
        bus.apu.end_frame();

        // Push audio samples — drop if buffer is full (never block the game)
        {
            let samples = bus.apu.drain_samples();
            for &sample in &samples {
                let _ = producer.try_push(sample);
            }
        }

        if crt_enabled {
            crt_filter(&bus.ppu.frame_data, &mut crt_buffer, &vignette_table);
        } else {
            scale_simple(&bus.ppu.frame_data, &mut crt_buffer);
        }
        
        // Composite game output into TV frame
        composite_screen(&tv_frame_bg, &crt_buffer, &mut composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT);

        window
            .update_with_buffer(&composite_buffer, WINDOW_WIDTH, WINDOW_HEIGHT)
            .expect("Failed to update window");

        // Mouse click handling for console interactions
        if let Some((mx, my)) = window.get_mouse_pos(minifb::MouseMode::Discard) {
            let mx = mx as usize;
            let my = my as usize;
            
            let mouse_down = window.get_mouse_down(minifb::MouseButton::Left);
            let mouse_clicked = mouse_down && !mouse_was_down;
            mouse_was_down = mouse_down;
            
            if mouse_clicked && mx < WINDOW_WIDTH && my < WINDOW_HEIGHT {
                let console_x = (WINDOW_WIDTH - 800) / 2;
                let body_y = TV_HEIGHT + 20;
                
                // RESET button hit test
                let rst_x = console_x + 150;
                let rst_y = body_y + 48;
                if mx >= rst_x && mx < rst_x + 50 && my >= rst_y && my < rst_y + 14 {
                    cpu.reset(&mut bus);
                    println!("CPU Reset");
                }
                
                // Cartridge slot hit test
                let slot_x = console_x + 800 / 2 - 100;
                let slot_y = body_y + 4;
                if mx >= slot_x && mx < slot_x + 200 && my >= slot_y && my < slot_y + 26 {
                    let file = rfd::FileDialog::new()
                        .set_title("Insert Cartridge")
                        .add_filter("NES ROMs", &["nes"])
                        .add_filter("All Files", &["*"])
                        .pick_file();
                    
                    if let Some(path) = file {
                        let rom_data = std::fs::read(&path);
                        if let Ok(data) = rom_data {
                            match Cartridge::new(&data) {
                                Ok(cart) => {
                                    bus = Bus::new(cart);
                                    bus.set_apu_sample_rate(actual_sample_rate);
                                    cpu = Cpu::new();
                                    cpu.reset(&mut bus);
                                    println!("Loaded: {}", path.display());
                                }
                                Err(e) => {
                                    rfd::MessageDialog::new()
                                        .set_title("ROM Error")
                                        .set_description(&format!("{}", e))
                                        .set_level(rfd::MessageLevel::Error)
                                        .show();
                                }
                            }
                        }
                    }
                }
            }
        }

        handle_input(&window, &mut bus, &mut gilrs);

        if window.is_key_pressed(Key::F1, KeyRepeat::No) {
            crt_enabled = !crt_enabled;
        }

        if window.is_key_down(Key::Escape) {
            break;
        }
    }
}

fn load_rom() -> (String, Cartridge) {
    let args: Vec<String> = env::args().collect();
    let mut initial_path = args.get(1).cloned();

    loop {
        let rom_path = match initial_path.take() {
            Some(path) => path,
            None => {
                let file = rfd::FileDialog::new()
                    .set_title("Select NES ROM")
                    .add_filter("NES ROMs", &["nes"])
                    .add_filter("All Files", &["*"])
                    .pick_file();
                match file {
                    Some(p) => p.to_string_lossy().to_string(),
                    None => {
                        eprintln!("No ROM selected. Exiting.");
                        std::process::exit(0);
                    }
                }
            }
        };

        let rom_data = match fs::read(&rom_path) {
            Ok(data) => data,
            Err(e) => {
                rfd::MessageDialog::new()
                    .set_title("Error")
                    .set_description(&format!("Failed to read file:\n{}", e))
                    .set_level(rfd::MessageLevel::Error)
                    .show();
                continue;
            }
        };

        match Cartridge::new(&rom_data) {
            Ok(cart) => return (rom_path, cart),
            Err(e) => {
                rfd::MessageDialog::new()
                    .set_title("ROM Error")
                    .set_description(&format!("Failed to load ROM:\n{}\n\nPlease select a different ROM.", e))
                    .set_level(rfd::MessageLevel::Error)
                    .show();
                continue;
            }
        }
    }
}

fn handle_input(window: &Window, bus: &mut Bus, gilrs: &mut Option<Gilrs>) {
    let keys = window.get_keys();
    
    // Start with keyboard state
    let mut a_pressed = keys.contains(&Key::Z);
    let mut b_pressed = keys.contains(&Key::X);
    let mut select_pressed = keys.contains(&Key::Space);
    let mut start_pressed = keys.contains(&Key::Enter);
    let mut up_pressed = keys.contains(&Key::Up);
    let mut down_pressed = keys.contains(&Key::Down);
    let mut left_pressed = keys.contains(&Key::Left);
    let mut right_pressed = keys.contains(&Key::Right);
    
    // Poll gamepad events and read state
    if let Some(ref mut g) = gilrs {
        // Process pending events (required by gilrs)
        while let Some(_event) = g.next_event() {}
        
        // Read first connected gamepad
        if let Some((_id, gamepad)) = g.gamepads().find(|(_, gp)| gp.is_connected()) {
            // D-pad buttons
            up_pressed |= gamepad.is_pressed(Button::DPadUp);
            down_pressed |= gamepad.is_pressed(Button::DPadDown);
            left_pressed |= gamepad.is_pressed(Button::DPadLeft);
            right_pressed |= gamepad.is_pressed(Button::DPadRight);
            
            // Left analog stick (with deadzone)
            let stick_x = gamepad.value(Axis::LeftStickX);
            let stick_y = gamepad.value(Axis::LeftStickY);
            let deadzone = 0.3;
            
            if stick_x < -deadzone { left_pressed = true; }
            if stick_x > deadzone { right_pressed = true; }
            if stick_y > deadzone { up_pressed = true; }
            if stick_y < -deadzone { down_pressed = true; }
            
            // Face buttons — clean 1:1 mapping + turbo alternatives
            // Xbox A (South) = NES A, Xbox B (East) = NES B
            a_pressed |= gamepad.is_pressed(Button::South);  // Xbox A → NES A
            a_pressed |= gamepad.is_pressed(Button::North);  // Xbox Y → NES A (turbo alt)
            
            b_pressed |= gamepad.is_pressed(Button::East);   // Xbox B → NES B
            b_pressed |= gamepad.is_pressed(Button::West);   // Xbox X → NES B (turbo alt)
            
            // Start / Select
            start_pressed |= gamepad.is_pressed(Button::Start);
            select_pressed |= gamepad.is_pressed(Button::Select);
            select_pressed |= gamepad.is_pressed(Button::Mode);
        }
    }
    
    // Apply all input to joypad
    bus.joypad1.set_button_pressed(JoypadButton::A, a_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::B, b_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Select, select_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Start, start_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Up, up_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Down, down_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Left, left_pressed);
    bus.joypad1.set_button_pressed(JoypadButton::Right, right_pressed);
}

fn crt_filter(input: &[u32], output: &mut Vec<u32>, vignette_table: &[u16]) {
    const SCREEN_W: usize = 960;
    const SCREEN_H: usize = 720;
    
    output.resize(SCREEN_W * SCREEN_H, 0);
    
    for dst_y in 0..SCREEN_H {
        // Map to source with sub-pixel precision (fixed point 8.8)
        let src_yf = (dst_y as u32 * 240 * 256) / SCREEN_H as u32; // 8.8 fixed
        let src_y0 = (src_yf >> 8) as usize;
        let src_y1 = (src_y0 + 1).min(239);
        let frac_y = (src_yf & 0xFF) as u32;
        
        // Scanline effect — based on position within the 3-pixel group
        let scan_pos = dst_y % 3;
        let scan_mul: u32 = match scan_pos {
            0 => 256,   // Full brightness
            1 => 230,   // Slight dim
            2 => 120,   // Dark gap between scanlines
            _ => 256,
        };
        
        let dst_row = dst_y * SCREEN_W;
        
        for dst_x in 0..SCREEN_W {
            let src_xf = (dst_x as u32 * 256 * 256) / SCREEN_W as u32; // 8.8 fixed
            let src_x0 = (src_xf >> 8) as usize;
            let src_x1 = (src_x0 + 1).min(255);
            let frac_x = (src_xf & 0xFF) as u32;
            
            // Bilinear interpolation — 4 source pixels
            let p00 = input[src_y0 * 256 + src_x0];
            let p10 = input[src_y0 * 256 + src_x1];
            let p01 = input[src_y1 * 256 + src_x0];
            let p11 = input[src_y1 * 256 + src_x1];
            
            let inv_fx = 256 - frac_x;
            let inv_fy = 256 - frac_y;
            
            // Interpolate each channel
            let r00 = (p00 >> 16) & 0xFF; let r10 = (p10 >> 16) & 0xFF;
            let r01 = (p01 >> 16) & 0xFF; let r11 = (p11 >> 16) & 0xFF;
            let mut r = (r00 * inv_fx * inv_fy + r10 * frac_x * inv_fy 
                       + r01 * inv_fx * frac_y + r11 * frac_x * frac_y) >> 16;
            
            let g00 = (p00 >> 8) & 0xFF; let g10 = (p10 >> 8) & 0xFF;
            let g01 = (p01 >> 8) & 0xFF; let g11 = (p11 >> 8) & 0xFF;
            let mut g = (g00 * inv_fx * inv_fy + g10 * frac_x * inv_fy 
                       + g01 * inv_fx * frac_y + g11 * frac_x * frac_y) >> 16;
            
            let b00 = p00 & 0xFF; let b10 = p10 & 0xFF;
            let b01 = p01 & 0xFF; let b11 = p11 & 0xFF;
            let mut b = (b00 * inv_fx * inv_fy + b10 * frac_x * inv_fy 
                       + b01 * inv_fx * frac_y + b11 * frac_x * frac_y) >> 16;
            
            // Brightness boost to compensate for scanline darkening
            r = (r * 290) >> 8;
            g = (g * 290) >> 8;
            b = (b * 290) >> 8;
            
            // Subtle RGB phosphor tint (less aggressive than before)
            let sub = dst_x % 3;
            match sub {
                0 => { r = (r * 270) >> 8; g = (g * 240) >> 8; b = (b * 240) >> 8; }
                1 => { r = (r * 240) >> 8; g = (g * 270) >> 8; b = (b * 240) >> 8; }
                _ => { r = (r * 240) >> 8; g = (g * 240) >> 8; b = (b * 270) >> 8; }
            }
            
            // Scanline
            r = (r * scan_mul) >> 8;
            g = (g * scan_mul) >> 8;
            b = (b * scan_mul) >> 8;
            
            // Vignette
            let vig = vignette_table[dst_y * SCREEN_W + dst_x] as u32;
            r = (r * vig) >> 8;
            g = (g * vig) >> 8;
            b = (b * vig) >> 8;
            
            output[dst_row + dst_x] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
        }
    }
}

fn scale_simple(input: &[u32], output: &mut Vec<u32>) {
    const SCREEN_W: usize = 960;
    const SCREEN_H: usize = 720;
    
    output.resize(SCREEN_W * SCREEN_H, 0);
    for y in 0..SCREEN_H {
        let src_y = y * 240 / SCREEN_H;
        for x in 0..SCREEN_W {
            let src_x = x * 256 / SCREEN_W;
            output[y * SCREEN_W + x] = input[src_y * 256 + src_x.min(255)];
        }
    }
}

fn build_tv_frame(frame: &mut Vec<u32>) {
    const TV_WIDTH: usize = 1280;
    const TV_HEIGHT: usize = 960;
    const CONSOLE_HEIGHT: usize = 200;
    const WINDOW_WIDTH: usize = TV_WIDTH;
    const WINDOW_HEIGHT: usize = TV_HEIGHT + CONSOLE_HEIGHT;
    const SCREEN_W: usize = 960;
    const SCREEN_H: usize = 720;
    const SCREEN_X: usize = 160;
    const SCREEN_Y: usize = 70;
    
    // Sony Trinitron color palette
    const BEZEL_BASE: u32     = 0x505050;  // Charcoal grey front face
    const BEZEL_HIGHLIGHT: u32 = 0x686868;  // Top/left bevel
    const BEZEL_SHADOW: u32   = 0x383838;  // Bottom/right shadow
    const BEZEL_INNER_RIM: u32 = 0x1A1A1A; // Dark lip around screen
    const SPEAKER_SLOT: u32   = 0x1E1E1E;  // Grille holes
    const BADGE_PLATE: u32    = 0x3A3A3A;  // Brand badge bg
    const BADGE_TEXT: u32     = 0xAAAAAA;  // Silver lettering
    const BUTTON_FACE: u32    = 0x3A3A3A;
    const LED_GREEN: u32      = 0x00CC44;  // Power on
    
    frame.resize(WINDOW_WIDTH * WINDOW_HEIGHT, 0);
    
    for y in 0..TV_HEIGHT {
        for x in 0..TV_WIDTH {
            let idx = y * WINDOW_WIDTH + x;
            
            // Screen area — will be overwritten with game
            let in_screen = x >= SCREEN_X && x < SCREEN_X + SCREEN_W 
                         && y >= SCREEN_Y && y < SCREEN_Y + SCREEN_H;
            
            if in_screen {
                // Check for rounded corners (radius ~10px)
                let corner_r = 10usize;
                let sx = x - SCREEN_X;
                let sy = y - SCREEN_Y;
                let in_corner = 
                    (sx < corner_r && sy < corner_r && sq_dist(sx, sy, corner_r, corner_r) > corner_r * corner_r) ||
                    (sx >= SCREEN_W - corner_r && sy < corner_r && sq_dist(sx, sy, SCREEN_W - corner_r - 1, corner_r) > corner_r * corner_r) ||
                    (sx < corner_r && sy >= SCREEN_H - corner_r && sq_dist(sx, sy, corner_r, SCREEN_H - corner_r - 1) > corner_r * corner_r) ||
                    (sx >= SCREEN_W - corner_r && sy >= SCREEN_H - corner_r && sq_dist(sx, sy, SCREEN_W - corner_r - 1, SCREEN_H - corner_r - 1) > corner_r * corner_r);
                
                if in_corner {
                    frame[idx] = BEZEL_INNER_RIM;
                } else {
                    frame[idx] = 0x000000; // screen area
                }
            } else {
                // === BEZEL ===
                let mut color = BEZEL_BASE;
                
                // Outer bevel — top and left edges lighter
                if y < 4 || x < 4 {
                    color = BEZEL_HIGHLIGHT;
                }
                // Outer shadow — bottom and right edges darker
                if y >= TV_HEIGHT - 4 || x >= TV_WIDTH - 4 {
                    color = BEZEL_SHADOW;
                }
                
                // 3D gradient — top half slightly lighter
                if y < TV_HEIGHT / 3 {
                    let factor = (TV_HEIGHT / 3 - y) as u32;
                    let boost = (factor * 12 / (TV_HEIGHT / 3) as u32).min(12);
                    let r = ((color >> 16) & 0xFF) + boost;
                    let g = ((color >> 8) & 0xFF) + boost;
                    let b = (color & 0xFF) + boost;
                    color = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
                }
                
                // Inner rim — dark recessed lip around screen (5px wide)
                let dist_to_screen_x = if x < SCREEN_X { SCREEN_X - x } 
                    else if x >= SCREEN_X + SCREEN_W { x - (SCREEN_X + SCREEN_W) + 1 } 
                    else { 999 };
                let dist_to_screen_y = if y < SCREEN_Y { SCREEN_Y - y }
                    else if y >= SCREEN_Y + SCREEN_H { y - (SCREEN_Y + SCREEN_H) + 1 }
                    else { 999 };
                let dist_to_screen = dist_to_screen_x.min(dist_to_screen_y);
                
                if dist_to_screen <= 5 {
                    let t = dist_to_screen as u32;
                    // Blend from BEZEL_INNER_RIM to current color
                    let rim_r = ((BEZEL_INNER_RIM >> 16) & 0xFF);
                    let rim_g = ((BEZEL_INNER_RIM >> 8) & 0xFF);
                    let rim_b = (BEZEL_INNER_RIM & 0xFF);
                    let cur_r = ((color >> 16) & 0xFF);
                    let cur_g = ((color >> 8) & 0xFF);
                    let cur_b = (color & 0xFF);
                    let r = rim_r + (cur_r - rim_r) * t / 5;
                    let g = rim_g + (cur_g - rim_g) * t / 5;
                    let b = rim_b + (cur_b - rim_b) * t / 5;
                    color = (r << 16) | (g << 8) | b;
                }
                
                // === BOTTOM BEZEL DETAILS ===
                let bottom_start = SCREEN_Y + SCREEN_H;
                
                // Speaker grille — horizontal slots below screen
                if y >= bottom_start + 25 && y < bottom_start + 65 
                   && x >= SCREEN_X + 200 && x < SCREEN_X + SCREEN_W - 200 {
                    if (y - bottom_start - 25) % 4 < 2 {
                        color = SPEAKER_SLOT;
                    }
                }
                
                // Brand badge — centered below speaker
                if y >= bottom_start + 80 && y < bottom_start + 100
                   && x >= TV_WIDTH / 2 - 60 && x < TV_WIDTH / 2 + 60 {
                    // Badge background
                    color = BADGE_PLATE;
                    // "NES" text approximation — 3 letter blocks
                    let bx = x - (TV_WIDTH / 2 - 60);
                    let by = y - (bottom_start + 80);
                    // Simple pixel text: N, E, S
                    if by >= 4 && by < 16 {
                        let lx = bx % 35;
                        if lx >= 8 && lx < 28 {
                            // Thin horizontal bars to suggest embossed text
                            if by == 5 || by == 10 || by == 15 {
                                color = BADGE_TEXT;
                            }
                        }
                    }
                }
                
                // Control buttons — right side of bottom bezel
                let btn_x = SCREEN_X + SCREEN_W - 60;
                if x >= btn_x && x < btn_x + 20 && y >= bottom_start + 30 && y < bottom_start + 100 {
                    let btn_local_y = y - (bottom_start + 30);
                    let btn_index = btn_local_y / 16;
                    let btn_offset = btn_local_y % 16;
                    if btn_index < 4 && btn_offset >= 2 && btn_offset < 12 {
                        let bx = x - btn_x;
                        if bx >= 4 && bx < 16 {
                            color = BUTTON_FACE;
                            // Top highlight
                            if btn_offset == 2 { color = 0x6A6A6A; }
                            // Bottom shadow
                            if btn_offset == 11 { color = 0x222222; }
                        }
                    }
                }
                
                // Power LED — bottom left
                if y >= bottom_start + 40 && y < bottom_start + 48
                   && x >= SCREEN_X + 30 && x < SCREEN_X + 38 {
                    let lx = x - (SCREEN_X + 30);
                    let ly = y - (bottom_start + 40);
                    // Circular LED (rough)
                    if (lx >= 2 && lx < 6) && (ly >= 2 && ly < 6) {
                        color = LED_GREEN;
                    } else if (lx >= 1 && lx < 7) && (ly >= 1 && ly < 7) {
                        // Glow halo
                        color = 0x004D1A; // dim green glow
                    }
                }
                
                frame[idx] = color;
            }
        }
    }
}

fn sq_dist(x1: usize, y1: usize, x2: usize, y2: usize) -> usize {
    let dx = if x1 > x2 { x1 - x2 } else { x2 - x1 };
    let dy = if y1 > y2 { y1 - y2 } else { y2 - y1 };
    dx * dx + dy * dy
}

fn composite_screen(tv_frame: &[u32], game_output: &[u32], result: &mut Vec<u32>, window_width: usize, window_height: usize) {
    const SCREEN_W: usize = 960;
    const SCREEN_H: usize = 720;
    const SCREEN_X: usize = 160;
    const SCREEN_Y: usize = 70;
    
    result.resize(window_width * window_height, 0);
    result.copy_from_slice(tv_frame);
    
    // Copy game output into screen area
    for y in 0..SCREEN_H {
        let src_start = y * SCREEN_W;
        let dst_start = (y + SCREEN_Y) * window_width + SCREEN_X;
        result[dst_start..dst_start + SCREEN_W]
            .copy_from_slice(&game_output[src_start..src_start + SCREEN_W]);
    }
}

fn build_console_overlay(frame: &mut Vec<u32>, tv_height: usize, window_width: usize, window_height: usize) {
    let console_y = tv_height; // starts right below TV
    let console_w = 800;       // console is 800px wide, centered
    let console_h = 160;
    let console_x = (window_width - console_w) / 2; // centered
    
    // "Shelf" / surface the console sits on
    for y in console_y..window_height {
        for x in 0..window_width {
            let idx = y * window_width + x;
            // Wood shelf texture (warm brown)
            let grain = ((x * 7 + y * 3) % 20) as u32;
            let r = 90 + grain;
            let g = 65 + grain;
            let b = 45 + grain / 2;
            frame[idx] = (r << 16) | (g << 8) | b;
        }
    }
    
    // Console body — main rectangle
    let body_y = console_y + 20; // 20px from top of console area
    let body_h = 120;
    
    for y in body_y..body_y + body_h {
        for x in console_x..console_x + console_w {
            let idx = y * window_width + x;
            let local_y = y - body_y;
            
            // Top dark stripe (first 30px) 
            if local_y < 30 {
                // Dark charcoal stripe
                let shade = if local_y < 2 { 0x353535u32 } // top edge shadow
                    else if local_y >= 28 { 0x353535u32 }   // bottom edge
                    else { 0x484848u32 };                    // stripe body
                frame[idx] = shade;
                
                // Cartridge slot — dark recessed rectangle in center
                let slot_x = console_x + console_w / 2 - 100;
                let slot_w = 200;
                if x >= slot_x && x < slot_x + slot_w && local_y >= 4 && local_y < 26 {
                    frame[idx] = 0x1A1A1A; // deep dark slot
                    // Slot inner edge highlight
                    if local_y == 4 || local_y == 25 || x == slot_x || x == slot_x + slot_w - 1 {
                        frame[idx] = 0x0E0E0E;
                    }
                    // Cartridge visible inside (lighter rectangle if ROM loaded)
                    if x >= slot_x + 20 && x < slot_x + slot_w - 20 && local_y >= 6 && local_y < 24 {
                        frame[idx] = 0x808080; // grey cartridge top
                        // Label on cartridge
                        if x >= slot_x + 50 && x < slot_x + slot_w - 50 && local_y >= 10 && local_y < 20 {
                            frame[idx] = 0xD4AA00; // gold label
                        }
                    }
                }
            } else {
                // Light grey body
                let local_x = x - console_x;
                let mut color = 0xC8C8C8u32;
                
                // Subtle top-to-bottom gradient
                let dim = (local_y - 30) as u32 / 4;
                let r = 200u32.saturating_sub(dim);
                let g = 200u32.saturating_sub(dim);
                let b = 200u32.saturating_sub(dim);
                color = (r << 16) | (g << 8) | b;
                
                // Top edge of body — highlight
                if local_y == 30 { color = 0xE0E0E0; }
                // Bottom edge — shadow
                if local_y >= body_h - 3 { color = 0x909090; }
                // Left/right edges
                if local_x < 3 || local_x >= console_w - 3 { 
                    color = 0xA0A0A0;
                }
                
                // POWER button — far left
                let pwr_x = console_x + 40;
                let pwr_y = body_y + 45;
                if x >= pwr_x && x < pwr_x + 40 && y >= pwr_y && y < pwr_y + 18 {
                    let bx = x - pwr_x;
                    let by = y - pwr_y;
                    color = 0x606060; // button face
                    if by == 0 { color = 0x808080; } // top highlight
                    if by == 17 { color = 0x404040; } // bottom shadow
                    if bx == 0 || bx == 39 { color = 0x505050; }
                }
                
                // Power LED — left of power button
                let led_x = console_x + 25;
                let led_y = body_y + 50;
                if x >= led_x && x < led_x + 6 && y >= led_y && y < led_y + 6 {
                    let dx = x - led_x;
                    let dy = y - led_y;
                    if dx >= 1 && dx < 5 && dy >= 1 && dy < 5 {
                        color = 0x00CC44; // green LED
                    } else {
                        color = 0x004D1A; // glow
                    }
                }
                
                // RESET button — center-left area
                let rst_x = console_x + 150;
                let rst_y = body_y + 48;
                if x >= rst_x && x < rst_x + 50 && y >= rst_y && y < rst_y + 14 {
                    let bx = x - rst_x;
                    let by = y - rst_y;
                    color = 0x707070;
                    if by == 0 { color = 0x909090; }
                    if by == 13 { color = 0x505050; }
                    if bx == 0 || bx == 49 { color = 0x606060; }
                }
                
                // Labels — "POWER" text area
                if x >= pwr_x && x < pwr_x + 40 && y >= pwr_y + 20 && y < pwr_y + 26 {
                    // Tiny dot pattern suggesting text
                    if (x - pwr_x) % 4 < 2 && (y - pwr_y - 20) % 3 < 2 {
                        color = 0x888888;
                    }
                }
                
                // "RESET" text area  
                if x >= rst_x && x < rst_x + 50 && y >= rst_y + 16 && y < rst_y + 22 {
                    if (x - rst_x) % 4 < 2 && (y - rst_y - 16) % 3 < 2 {
                        color = 0x888888;
                    }
                }
                
                // Controller ports — two dark rectangles at bottom
                let port1_x = console_x + 250;
                let port2_x = console_x + 480;
                let port_y = body_y + 90;
                let port_w = 70;
                let port_h = 20;
                
                for (px, _label) in [(port1_x, "1"), (port2_x, "2")] {
                    if x >= px && x < px + port_w && y >= port_y && y < port_y + port_h {
                        color = 0x2A2A2A; // dark port
                        let bx = x - px;
                        let by = y - port_y;
                        if by == 0 { color = 0x1A1A1A; } // top shadow
                        if by == port_h - 1 { color = 0x3A3A3A; } // bottom lip
                        // Inner pins
                        if bx >= 10 && bx < port_w - 10 && by >= 5 && by < port_h - 5 {
                            if bx % 6 < 3 { color = 0x555555; } // pins
                        }
                    }
                }
                
                frame[idx] = color;
            }
        }
    }
}
