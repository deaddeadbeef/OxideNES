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
                let console_x = (WINDOW_WIDTH - 700) / 2;
                let body_y = TV_HEIGHT + 15;
                
                // RESET button hit test
                let rst_x = console_x + 160;
                let rst_y = body_y + 52;
                if mx >= rst_x && mx < rst_x + 55 && my >= rst_y && my < rst_y + 14 {
                    cpu.reset(&mut bus);
                    println!("CPU Reset");
                }
                
                // Cartridge slot hit test
                let slot_x = console_x + 700 / 2 - 90;
                let slot_y = body_y + 5;
                if mx >= slot_x && mx < slot_x + 180 && my >= slot_y && my < slot_y + 25 {
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
    
    frame.resize(WINDOW_WIDTH * WINDOW_HEIGHT, 0);
    
    // Background — dark room/wall color
    for i in 0..WINDOW_WIDTH * TV_HEIGHT {
        frame[i] = 0x1A1A22; // dark bluish-grey wall
    }
    
    // TV outer shell dimensions — slightly larger than bezel
    let tv_outer_x = SCREEN_X - 50;
    let tv_outer_y = SCREEN_Y - 45;
    let tv_outer_w = SCREEN_W + 100;
    let tv_outer_h = SCREEN_H + 145; // extra bottom for controls
    let tv_outer_r = tv_outer_x + tv_outer_w;
    let tv_outer_b = tv_outer_y + tv_outer_h;
    
    for y in 0..TV_HEIGHT {
        for x in 0..WINDOW_WIDTH {
            let idx = y * WINDOW_WIDTH + x;
            
            // Inside TV outer shell?
            if x >= tv_outer_x && x < tv_outer_r && y >= tv_outer_y && y < tv_outer_b {
                let lx = x - tv_outer_x;
                let ly = y - tv_outer_y;
                
                // Rounded corners — skip pixels outside radius
                let corner_r = 20usize;
                let in_corner_tl = lx < corner_r && ly < corner_r && sq_dist(lx, ly, corner_r, corner_r) > corner_r * corner_r;
                let in_corner_tr = lx >= tv_outer_w - corner_r && ly < corner_r && sq_dist(lx, ly, tv_outer_w - corner_r - 1, corner_r) > corner_r * corner_r;
                let in_corner_bl = lx < corner_r && ly >= tv_outer_h - corner_r && sq_dist(lx, ly, corner_r, tv_outer_h - corner_r - 1) > corner_r * corner_r;
                let in_corner_br = lx >= tv_outer_w - corner_r && ly >= tv_outer_h - corner_r && sq_dist(lx, ly, tv_outer_w - corner_r - 1, tv_outer_h - corner_r - 1) > corner_r * corner_r;
                
                if in_corner_tl || in_corner_tr || in_corner_bl || in_corner_br {
                    continue; // Leave as wall color — rounded corner
                }
                
                // Main bezel color with smooth gradient
                let grad_y = (ly as f32 / tv_outer_h as f32);
                let base = 72.0 - grad_y * 20.0; // darker toward bottom
                let mut r = base as u32;
                let mut g = base as u32;
                let mut b = (base + 2.0) as u32; // very slight blue tint
                
                // Outer edge bevel — 4px lighter on top/left, darker on bottom/right
                if ly < 4 { r += 25; g += 25; b += 25; }
                if lx < 4 && ly >= 4 { r += 15; g += 15; b += 15; }
                if ly >= tv_outer_h - 4 { r = r.saturating_sub(15); g = g.saturating_sub(15); b = b.saturating_sub(15); }
                if lx >= tv_outer_w - 4 && ly < tv_outer_h - 4 { r = r.saturating_sub(10); g = g.saturating_sub(10); b = b.saturating_sub(10); }
                
                // Inner bezel around screen — 8px beveled inset
                let dist_to_screen = {
                    let dx = if x < SCREEN_X { SCREEN_X - x } else if x >= SCREEN_X + SCREEN_W { x - (SCREEN_X + SCREEN_W) + 1 } else { 999 };
                    let dy = if y < SCREEN_Y { SCREEN_Y - y } else if y >= SCREEN_Y + SCREEN_H { y - (SCREEN_Y + SCREEN_H) + 1 } else { 999 };
                    dx.min(dy)
                };
                
                if dist_to_screen <= 8 && dist_to_screen > 0 {
                    let shadow = (8 - dist_to_screen) as u32 * 6;
                    r = r.saturating_sub(shadow);
                    g = g.saturating_sub(shadow);
                    b = b.saturating_sub(shadow);
                }
                
                // Screen area itself
                let in_screen = x >= SCREEN_X && x < SCREEN_X + SCREEN_W 
                             && y >= SCREEN_Y && y < SCREEN_Y + SCREEN_H;
                if in_screen {
                    // Rounded screen corners
                    let scr_r = 8usize;
                    let sx = x - SCREEN_X;
                    let sy = y - SCREEN_Y;
                    let scr_corner = 
                        (sx < scr_r && sy < scr_r && sq_dist(sx, sy, scr_r, scr_r) > scr_r * scr_r) ||
                        (sx >= SCREEN_W - scr_r && sy < scr_r && sq_dist(sx, sy, SCREEN_W - scr_r - 1, scr_r) > scr_r * scr_r) ||
                        (sx < scr_r && sy >= SCREEN_H - scr_r && sq_dist(sx, sy, scr_r, SCREEN_H - scr_r - 1) > scr_r * scr_r) ||
                        (sx >= SCREEN_W - scr_r && sy >= SCREEN_H - scr_r && sq_dist(sx, sy, SCREEN_W - scr_r - 1, SCREEN_H - scr_r - 1) > scr_r * scr_r);
                    
                    if scr_corner {
                        // Dark bezel in screen corners
                        frame[idx] = 0x0A0A0A;
                    } else {
                        frame[idx] = 0x000000;
                    }
                    continue;
                }
                
                // Bottom panel — below screen, has speaker + controls
                if y >= SCREEN_Y + SCREEN_H + 10 {
                    // Speaker grille — centered horizontal slots
                    let speaker_x_start = SCREEN_X + SCREEN_W / 2 - 150;
                    let speaker_x_end = SCREEN_X + SCREEN_W / 2 + 150;
                    let speaker_y_start = SCREEN_Y + SCREEN_H + 20;
                    let speaker_y_end = speaker_y_start + 50;
                    
                    if x >= speaker_x_start && x < speaker_x_end && y >= speaker_y_start && y < speaker_y_end {
                        let slot_y = (y - speaker_y_start) % 5;
                        if slot_y < 2 {
                            // Slot holes — very dark with subtle depth
                            let slot_depth = if slot_y == 0 { 0x0E0E0Eu32 } else { 0x181818u32 };
                            frame[idx] = slot_depth;
                            continue;
                        }
                    }
                    
                    // Power LED — small green dot, bottom left
                    let led_cx = tv_outer_x + 50;
                    let led_cy = SCREEN_Y + SCREEN_H + 45;
                    let led_dx = if x > led_cx { x - led_cx } else { led_cx - x };
                    let led_dy = if y > led_cy { y - led_cy } else { led_cy - y };
                    if led_dx * led_dx + led_dy * led_dy <= 12 {
                        frame[idx] = 0x00DD55; // bright green
                        continue;
                    } else if led_dx * led_dx + led_dy * led_dy <= 30 {
                        frame[idx] = 0x004418; // green glow
                        continue;
                    }
                    
                    // Brand badge — subtle embossed rectangle
                    let badge_x = SCREEN_X + SCREEN_W / 2 - 50;
                    let badge_y = SCREEN_Y + SCREEN_H + 78;
                    if x >= badge_x && x < badge_x + 100 && y >= badge_y && y < badge_y + 16 {
                        let bx = x - badge_x;
                        let by = y - badge_y;
                        if by == 0 || bx == 0 { frame[idx] = 0x555555; continue; }
                        if by == 15 || bx == 99 { frame[idx] = 0x3A3A3A; continue; }
                        frame[idx] = 0x454545;
                        continue;
                    }
                }
                
                frame[idx] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
            }
            // else: stays as wall color
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

fn draw_text(frame: &mut Vec<u32>, text: &str, start_x: usize, start_y: usize, color: u32, stride: usize) {
    let font: std::collections::HashMap<char, [u8; 5]> = [
        ('A', [0b111, 0b101, 0b111, 0b101, 0b101]),
        ('B', [0b110, 0b101, 0b110, 0b101, 0b110]),
        ('C', [0b111, 0b100, 0b100, 0b100, 0b111]),
        ('D', [0b110, 0b101, 0b101, 0b101, 0b110]),
        ('E', [0b111, 0b100, 0b110, 0b100, 0b111]),
        ('F', [0b111, 0b100, 0b110, 0b100, 0b100]),
        ('G', [0b111, 0b100, 0b101, 0b101, 0b111]),
        ('H', [0b101, 0b101, 0b111, 0b101, 0b101]),
        ('I', [0b111, 0b010, 0b010, 0b010, 0b111]),
        ('J', [0b001, 0b001, 0b001, 0b101, 0b111]),
        ('K', [0b101, 0b110, 0b100, 0b110, 0b101]),
        ('L', [0b100, 0b100, 0b100, 0b100, 0b111]),
        ('M', [0b101, 0b111, 0b111, 0b101, 0b101]),
        ('N', [0b101, 0b111, 0b111, 0b101, 0b101]),
        ('O', [0b111, 0b101, 0b101, 0b101, 0b111]),
        ('P', [0b111, 0b101, 0b111, 0b100, 0b100]),
        ('Q', [0b111, 0b101, 0b101, 0b111, 0b001]),
        ('R', [0b111, 0b101, 0b111, 0b110, 0b101]),
        ('S', [0b111, 0b100, 0b111, 0b001, 0b111]),
        ('T', [0b111, 0b010, 0b010, 0b010, 0b010]),
        ('U', [0b101, 0b101, 0b101, 0b101, 0b111]),
        ('V', [0b101, 0b101, 0b101, 0b101, 0b010]),
        ('W', [0b101, 0b101, 0b111, 0b111, 0b101]),
        ('X', [0b101, 0b101, 0b010, 0b101, 0b101]),
        ('Y', [0b101, 0b101, 0b010, 0b010, 0b010]),
        ('Z', [0b111, 0b001, 0b010, 0b100, 0b111]),
        ('0', [0b111, 0b101, 0b101, 0b101, 0b111]),
        ('1', [0b010, 0b110, 0b010, 0b010, 0b111]),
        ('2', [0b111, 0b001, 0b111, 0b100, 0b111]),
        (' ', [0b000, 0b000, 0b000, 0b000, 0b000]),
    ].iter().cloned().collect();

    let mut cursor_x = start_x;
    for ch in text.chars() {
        if let Some(glyph) = font.get(&ch) {
            for (row, &bits) in glyph.iter().enumerate() {
                for col in 0..3 {
                    if bits & (0b100 >> col) != 0 {
                        let px = cursor_x + col;
                        let py = start_y + row;
                        if px < stride && py * stride + px < frame.len() {
                            frame[py * stride + px] = color;
                        }
                    }
                }
            }
        }
        cursor_x += 4; // 3px char + 1px gap
    }
}

fn build_console_overlay(frame: &mut Vec<u32>, tv_height: usize, window_width: usize, window_height: usize) {
    let console_y = tv_height;
    let console_w = 700;
    let console_h = 140;
    let console_x = (window_width - console_w) / 2;
    
    // Shelf/surface
    for y in console_y..window_height {
        for x in 0..window_width {
            let idx = y * window_width + x;
            // Wood grain — warm oak
            let base_r = 85u32;
            let base_g = 62u32;
            let base_b = 40u32;
            let grain1 = ((x.wrapping_mul(13) + y.wrapping_mul(7)) % 15) as u32;
            let grain2 = ((x.wrapping_mul(3) + y.wrapping_mul(11)) % 8) as u32;
            frame[idx] = ((base_r + grain1).min(120) << 16) | ((base_g + grain2).min(85) << 8) | (base_b + grain1 / 2).min(65);
        }
    }
    
    let body_y = console_y + 15;
    let body_r = console_x + console_w;
    let body_b = body_y + console_h;
    
    for y in body_y..body_b {
        for x in console_x..body_r {
            let idx = y * window_width + x;
            let lx = x - console_x;
            let ly = y - body_y;
            
            // Rounded corners for console body
            let cr = 10usize;
            let skip = 
                (lx < cr && ly < cr && sq_dist(lx, ly, cr, cr) > cr * cr) ||
                (lx >= console_w - cr && ly < cr && sq_dist(lx, ly, console_w - cr - 1, cr) > cr * cr) ||
                (lx < cr && ly >= console_h - cr && sq_dist(lx, ly, cr, console_h - cr - 1) > cr * cr) ||
                (lx >= console_w - cr && ly >= console_h - cr && sq_dist(lx, ly, console_w - cr - 1, console_h - cr - 1) > cr * cr);
            if skip { continue; }
            
            // Top dark stripe (cartridge area) — first 35px
            if ly < 35 {
                let mut c = 0x3C3C3Cu32;
                // Top bevel
                if ly < 2 { c = 0x505050; }
                if ly >= 33 { c = 0x2A2A2A; }
                
                // Cartridge slot — centered dark rectangle
                let slot_x = console_w / 2 - 90;
                let slot_w = 180;
                if lx >= slot_x && lx < slot_x + slot_w && ly >= 5 && ly < 30 {
                    c = 0x111111;
                    // Slot edges
                    if ly == 5 || ly == 29 || lx == slot_x || lx == slot_x + slot_w - 1 {
                        c = 0x080808;
                    }
                    // Cartridge inside slot
                    if lx >= slot_x + 15 && lx < slot_x + slot_w - 15 && ly >= 7 && ly < 28 {
                        c = 0x6E6E6E; // grey cart top
                        // Cart label
                        if lx >= slot_x + 40 && lx < slot_x + slot_w - 40 && ly >= 11 && ly < 24 {
                            c = 0xCC9922; // gold label
                            // Label border
                            if ly == 11 || ly == 23 || lx == slot_x + 40 || lx == slot_x + slot_w - 41 {
                                c = 0x997711;
                            }
                        }
                    }
                }
                
                frame[idx] = c;
            } else {
                // Light body — smooth gradient
                let grad = ly as f32 / console_h as f32;
                let base = (195.0 - grad * 30.0) as u32;
                let mut r = base;
                let mut g = base;
                let mut b = (base as f32 * 0.97) as u32; // very slight warm tint
                
                // Top highlight of body
                if ly == 35 { r += 20; g += 20; b += 20; }
                // Bottom shadow
                if ly >= console_h - 3 { r = r.saturating_sub(30); g = g.saturating_sub(30); b = b.saturating_sub(30); }
                // Side shadows
                if lx < 5 { r = r.saturating_sub(15); g = g.saturating_sub(15); b = b.saturating_sub(15); }
                if lx >= console_w - 5 { r = r.saturating_sub(20); g = g.saturating_sub(20); b = b.saturating_sub(20); }
                
                // POWER button — raised rectangle with 3D effect
                let pwr_x = 60usize;
                let pwr_y = 50usize;
                let pwr_w = 45usize;
                let pwr_h = 18usize;
                if lx >= pwr_x && lx < pwr_x + pwr_w && ly >= pwr_y && ly < pwr_y + pwr_h {
                    let bx = lx - pwr_x;
                    let by = ly - pwr_y;
                    r = 85; g = 85; b = 85;
                    if by == 0 { r = 110; g = 110; b = 110; } // top highlight
                    if by == pwr_h - 1 { r = 50; g = 50; b = 50; } // bottom shadow
                    if bx == 0 { r = 100; g = 100; b = 100; }
                    if bx == pwr_w - 1 { r = 55; g = 55; b = 55; }
                }
                
                // Power LED — circular
                let led_x = 45usize;
                let led_y = 56usize;
                if lx >= led_x && lx < led_x + 8 && ly >= led_y && ly < led_y + 8 {
                    let dx = (lx - led_x) as i32 - 3;
                    let dy = (ly - led_y) as i32 - 3;
                    if dx * dx + dy * dy <= 6 {
                        r = 0; g = 220; b = 68;
                    } else if dx * dx + dy * dy <= 14 {
                        r = 0; g = 80; b = 25;
                    }
                }
                
                // RESET button — smaller, recessed look
                let rst_x = 160usize;
                let rst_y = 52usize;
                let rst_w = 55usize;
                let rst_h = 14usize;
                if lx >= rst_x && lx < rst_x + rst_w && ly >= rst_y && ly < rst_y + rst_h {
                    let by = ly - rst_y;
                    r = 100; g = 100; b = 100;
                    if by == 0 { r = 70; g = 70; b = 70; } // top shadow (recessed)
                    if by == rst_h - 1 { r = 120; g = 120; b = 120; } // bottom highlight
                }
                
                // Controller ports — two trapezoidal ports
                let port_y = 85usize;
                let port_h = 22usize;
                let port_w = 65usize;
                let port1_x = console_w / 2 - 100;
                let port2_x = console_w / 2 + 35;
                
                for port_x in [port1_x, port2_x] {
                    if lx >= port_x && lx < port_x + port_w && ly >= port_y && ly < port_y + port_h {
                        let bx = lx - port_x;
                        let by = ly - port_y;
                        r = 30; g = 30; b = 30;
                        // Inner cavity
                        if bx >= 5 && bx < port_w - 5 && by >= 3 && by < port_h - 3 {
                            r = 18; g = 18; b = 18;
                            // Connector pins
                            if by >= 7 && by < port_h - 7 && bx >= 12 && bx < port_w - 12 {
                                if (bx - 12) % 5 < 3 {
                                    r = 80; g = 75; b = 60; // brass pins
                                }
                            }
                        }
                        // Port border
                        if by == 0 { r = 20; g = 20; b = 20; }
                        if by == port_h - 1 { r = 50; g = 50; b = 50; }
                    }
                }
                
                // Decorative lines/ridges on body
                if ly == 75 && lx >= 30 && lx < console_w - 30 {
                    r = r.saturating_sub(20); g = g.saturating_sub(20); b = b.saturating_sub(20);
                }
                if ly == 76 && lx >= 30 && lx < console_w - 30 {
                    r = (r + 10).min(255); g = (g + 10).min(255); b = (b + 10).min(255);
                }
                
                frame[idx] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
            }
        }
    }
    
    // Shadow under console
    for y in body_b..body_b.min(body_b + 6) {
        for x in console_x + 10..body_r - 10 {
            if y < window_height {
                let idx = y * window_width + x;
                let shadow_alpha = (body_b + 6 - y) as u32 * 8;
                let existing = frame[idx];
                let er = ((existing >> 16) & 0xFF).saturating_sub(shadow_alpha);
                let eg = ((existing >> 8) & 0xFF).saturating_sub(shadow_alpha);
                let eb = (existing & 0xFF).saturating_sub(shadow_alpha);
                frame[idx] = (er << 16) | (eg << 8) | eb;
            }
        }
    }
    
    // Labels — adjust positions to match new console layout
    let console_x_text = (window_width - 700) / 2;
    let body_y_text = tv_height + 15;

    draw_text(frame, "POWER", console_x_text + 55, body_y_text + 70, 0x808080, window_width);
    draw_text(frame, "RESET", console_x_text + 160, body_y_text + 68, 0x808080, window_width);
    draw_text(frame, "INSERT CARTRIDGE", console_x_text + 700/2 - 32, body_y_text + 32, 0x555555, window_width);

    let port1_x = console_x_text + 700/2 - 100;
    let port2_x = console_x_text + 700/2 + 35;
    draw_text(frame, "1", port1_x + 30, body_y_text + 110, 0x808080, window_width);
    draw_text(frame, "2", port2_x + 30, body_y_text + 110, 0x808080, window_width);
}
