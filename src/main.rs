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
    const SCREEN_W: usize = 1024;  // 4x NES width (256*4)
    const SCREEN_H: usize = 720;   // 3x NES height (240*3) — slight 4:3 stretch
    const SCREEN_X: usize = 128;   // (1280 - 1024) / 2
    const SCREEN_Y: usize = 70;    // Top bezel thinner than bottom
    
    let mut window = Window::new(
        "NES Emulator",
        TV_WIDTH,
        TV_HEIGHT,
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
    let mut composite_buffer = vec![0u32; TV_WIDTH * TV_HEIGHT];
    
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
        composite_screen(&tv_frame_bg, &crt_buffer, &mut composite_buffer);

        window
            .update_with_buffer(&composite_buffer, TV_WIDTH, TV_HEIGHT)
            .expect("Failed to update window");

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
    const SCREEN_W: usize = 1024;
    const SCREEN_H: usize = 720;
    
    output.resize(SCREEN_W * SCREEN_H, 0);
    
    for dst_y in 0..SCREEN_H {
        let src_y = dst_y * 240 / SCREEN_H;
        let is_scanline = dst_y % 3 == 2; // Every 3rd line (since 3x scale)
        let scan_mul: u16 = if is_scanline { 160 } else { 256 }; // Lighter scanlines at 3x
        
        let row_offset = src_y * 256;
        let dst_row = dst_y * SCREEN_W;
        
        for dst_x in 0..SCREEN_W {
            let src_x = dst_x * 256 / SCREEN_W;
            
            let pixel = input[row_offset + src_x.min(255)];
            let mut r = ((pixel >> 16) & 0xFF) as u16;
            let mut g = ((pixel >> 8) & 0xFF) as u16;
            let mut b = (pixel & 0xFF) as u16;
            
            // Brightness boost
            r = (r * 307) >> 8;
            g = (g * 307) >> 8;
            b = (b * 307) >> 8;
            
            // RGB phosphor
            let sub = dst_x % 3;
            match sub {
                0 => { r = (r * 294) >> 8; g = (g * 218) >> 8; b = (b * 218) >> 8; }
                1 => { r = (r * 218) >> 8; g = (g * 294) >> 8; b = (b * 218) >> 8; }
                _ => { r = (r * 218) >> 8; g = (g * 218) >> 8; b = (b * 294) >> 8; }
            }
            
            // Scanline
            r = (r * scan_mul) >> 8;
            g = (g * scan_mul) >> 8;
            b = (b * scan_mul) >> 8;
            
            // Vignette
            let vig = vignette_table[dst_y * SCREEN_W + dst_x] as u32;
            r = ((r as u32 * vig) >> 8) as u16;
            g = ((g as u32 * vig) >> 8) as u16;
            b = ((b as u32 * vig) >> 8) as u16;
            
            output[dst_row + dst_x] = (r.min(255) as u32) << 16 | (g.min(255) as u32) << 8 | b.min(255) as u32;
        }
    }
}

fn scale_simple(input: &[u32], output: &mut Vec<u32>) {
    const SCREEN_W: usize = 1024;
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
    const SCREEN_W: usize = 1024;
    const SCREEN_H: usize = 720;
    const SCREEN_X: usize = 128;
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
    
    frame.resize(TV_WIDTH * TV_HEIGHT, 0);
    
    for y in 0..TV_HEIGHT {
        for x in 0..TV_WIDTH {
            let idx = y * TV_WIDTH + x;
            
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

fn composite_screen(tv_frame: &[u32], game_output: &[u32], result: &mut Vec<u32>) {
    const TV_WIDTH: usize = 1280;
    const TV_HEIGHT: usize = 960;
    const SCREEN_W: usize = 1024;
    const SCREEN_H: usize = 720;
    const SCREEN_X: usize = 128;
    const SCREEN_Y: usize = 70;
    
    result.resize(TV_WIDTH * TV_HEIGHT, 0);
    result.copy_from_slice(tv_frame);
    
    // Copy game output into screen area
    for y in 0..SCREEN_H {
        let src_start = y * SCREEN_W;
        let dst_start = (y + SCREEN_Y) * TV_WIDTH + SCREEN_X;
        result[dst_start..dst_start + SCREEN_W]
            .copy_from_slice(&game_output[src_start..src_start + SCREEN_W]);
    }
}
