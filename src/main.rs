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

    // TV dimensions — chunky CRT with thick bezels for premium look
    const TV_WIDTH: usize = 1280;
    const TV_HEIGHT: usize = 960;
    const CONSOLE_HEIGHT: usize = 200;
    const WINDOW_WIDTH: usize = TV_WIDTH;
    const WINDOW_HEIGHT: usize = TV_HEIGHT + CONSOLE_HEIGHT; // 1160 total
    const SCREEN_W: usize = 860;   // Slightly smaller screen = thicker bezels
    const SCREEN_H: usize = 645;   // 4:3 ratio maintained
    const SCREEN_X: usize = 210;   // Centered with thick side bezels
    const SCREEN_Y: usize = 85;    // Thick top bezel
    
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
                // Console layout for hit-testing (matches new design)
                let console_w = 900;
                let console_x = (WINDOW_WIDTH - console_w) / 2;
                let body_y = TV_HEIGHT + 20;
                
                // Cartridge slot: centered in top stripe
                let slot_lx = console_w / 2 - 160;
                let slot_x = console_x + slot_lx;
                let slot_y = body_y + 8;
                let slot_w = 320;
                let slot_h = 34;
                if mx >= slot_x && mx < slot_x + slot_w && my >= slot_y && my < slot_y + slot_h {
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
                
                // Reset button hit test
                let rst_lx = console_w - 170 + 30;
                let rst_x = console_x + rst_lx;
                let rst_y = body_y + 68;
                let rst_w = 80;
                let rst_h = 22;
                if mx >= rst_x && mx < rst_x + rst_w && my >= rst_y && my < rst_y + rst_h {
                    cpu.reset(&mut bus);
                    println!("CPU Reset");
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
    const SCREEN_W: usize = 860;
    const SCREEN_H: usize = 645;
    
    output.resize(SCREEN_W * SCREEN_H, 0);
    
    for dst_y in 0..SCREEN_H {
        // Source Y with sub-pixel precision (fixed point 16.8)
        let src_yf = (dst_y as u32 * 240 * 256) / SCREEN_H as u32;
        let src_y0 = (src_yf >> 8) as usize;
        let src_y1 = (src_y0 + 1).min(239);
        let frac_y = (src_yf & 0xFF) as u32;
        
        // Scanline effect — gentle brightness variation
        // On a 32" flat CRT, scanlines were visible but soft
        let scan_mul: u32 = match dst_y % 3 {
            0 => 255,  // Full brightness
            1 => 245,  // Very slight dim
            2 => 195,  // Gentle scanline gap (not harsh)
            _ => 255,
        };
        
        let dst_row = dst_y * SCREEN_W;
        
        for dst_x in 0..SCREEN_W {
            let src_xf = (dst_x as u32 * 256 * 256) / SCREEN_W as u32;
            let src_x0 = (src_xf >> 8) as usize;
            let src_x1 = (src_x0 + 1).min(255);
            let frac_x = (src_xf & 0xFF) as u32;
            
            // Bilinear interpolation — the key to soft CRT look
            let p00 = input[src_y0 * 256 + src_x0];
            let p10 = input[src_y0 * 256 + src_x1];
            let p01 = input[src_y1 * 256 + src_x0];
            let p11 = input[src_y1 * 256 + src_x1];
            
            let inv_fx = 256 - frac_x;
            let inv_fy = 256 - frac_y;
            
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
            
            // Horizontal blur — blend with left and right neighbors for CRT bloom
            // This is what makes it look soft like a CRT, not sharp like LCD
            if src_x0 > 0 && src_x0 < 255 {
                let left = input[src_y0 * 256 + src_x0 - 1];
                let right = input[src_y0 * 256 + src_x1.min(255)];
                let lr = (left >> 16) & 0xFF; let rr = (right >> 16) & 0xFF;
                let lg = (left >> 8) & 0xFF;  let rg = (right >> 8) & 0xFF;
                let lb = left & 0xFF;          let rb = right & 0xFF;
                // 80% center + 10% each neighbor
                r = (r * 205 + lr * 25 + rr * 25) >> 8;
                g = (g * 205 + lg * 25 + rg * 25) >> 8;
                b = (b * 205 + lb * 25 + rb * 25) >> 8;
            }
            
            // Brightness boost to compensate for scanline dimming
            r = (r * 275) >> 8;
            g = (g * 275) >> 8;
            b = (b * 275) >> 8;
            
            // Warm color temperature — slight warm shift like a real CRT
            // CRTs had slightly warm whites, not blue-white like LCDs
            r = (r * 262) >> 8;  // boost red slightly
            g = (g * 256) >> 8;  // green neutral
            b = (b * 242) >> 8;  // reduce blue slightly
            
            // Scanline — gentle variation, not harsh bands
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
    const SCREEN_W: usize = 860;
    const SCREEN_H: usize = 645;
    
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
    const SCREEN_W: usize = 860;
    const SCREEN_H: usize = 645;
    const SCREEN_X: usize = 210;
    const SCREEN_Y: usize = 85;

    frame.resize(WINDOW_WIDTH * WINDOW_HEIGHT, 0);

    // Wall background — warm dark with subtle texture
    for y in 0..TV_HEIGHT {
        for x in 0..WINDOW_WIDTH {
            let idx = y * WINDOW_WIDTH + x;
            let noise = ((x * 17 + y * 31) % 7) as u32;
            frame[idx] = ((0x22 + noise) << 16) | ((0x20 + noise) << 8) | (0x1E + noise);
        }
    }

    // TV body outer bounds
    let tv_x1 = 30usize;
    let tv_y1 = 15usize;
    let tv_x2 = WINDOW_WIDTH - 30;
    let tv_y2 = TV_HEIGHT - 15;
    let tv_w = tv_x2 - tv_x1;
    let tv_h = tv_y2 - tv_y1;
    let corner_r = 25usize;

    for y in tv_y1..tv_y2 {
        for x in tv_x1..tv_x2 {
            let idx = y * WINDOW_WIDTH + x;
            let lx = x - tv_x1;
            let ly = y - tv_y1;

            // Rounded corners
            if (lx < corner_r && ly < corner_r && sq_dist(lx, ly, corner_r, corner_r) > corner_r * corner_r)
                || (lx >= tv_w - corner_r && ly < corner_r && sq_dist(lx, ly, tv_w - corner_r, corner_r) > corner_r * corner_r)
                || (lx < corner_r && ly >= tv_h - corner_r && sq_dist(lx, ly, corner_r, tv_h - corner_r) > corner_r * corner_r)
                || (lx >= tv_w - corner_r && ly >= tv_h - corner_r && sq_dist(lx, ly, tv_w - corner_r, tv_h - corner_r) > corner_r * corner_r)
            {
                continue;
            }

            // Base color with vertical gradient (lighter top, darker bottom)
            let gy = ly as f32 / tv_h as f32;
            let base_val = 78.0 - gy * 22.0;
            // Horizontal curvature: slightly lighter in center
            let gx = (lx as f32 / tv_w as f32 - 0.5).abs();
            let center_boost = (1.0 - gx * 1.5).max(0.0) * 8.0;
            let val = (base_val + center_boost) as u32;

            let mut r = val;
            let mut g = val;
            let mut b = val + 1; // very slight cool tint

            // Outer rim — 6px bevel
            if ly < 6 {
                let boost = (6 - ly) as u32 * 4;
                r += boost; g += boost; b += boost;
            }
            if ly >= tv_h - 6 {
                let dim = (ly - (tv_h - 6)) as u32 * 5;
                r = r.saturating_sub(dim); g = g.saturating_sub(dim); b = b.saturating_sub(dim);
            }
            if lx < 6 {
                let boost = (6 - lx) as u32 * 3;
                r += boost; g += boost; b += boost;
            }
            if lx >= tv_w - 6 {
                let dim = (lx - (tv_w - 6)) as u32 * 4;
                r = r.saturating_sub(dim); g = g.saturating_sub(dim); b = b.saturating_sub(dim);
            }

            // Subtle plastic texture
            let tex = ((x.wrapping_mul(7919) ^ y.wrapping_mul(6271)) % 5) as u32;
            r = r.saturating_sub(tex).saturating_add(tex / 2);

            // Screen area check
            let in_screen = x >= SCREEN_X && x < SCREEN_X + SCREEN_W && y >= SCREEN_Y && y < SCREEN_Y + SCREEN_H;

            // Inner shadow ring around screen (15px gradient from dark to bezel)
            if !in_screen {
                let dx = if x < SCREEN_X { SCREEN_X - x } else if x >= SCREEN_X + SCREEN_W { x - (SCREEN_X + SCREEN_W) + 1 } else { 999 };
                let dy = if y < SCREEN_Y { SCREEN_Y - y } else if y >= SCREEN_Y + SCREEN_H { y - (SCREEN_Y + SCREEN_H) + 1 } else { 999 };
                let d = dx.min(dy);
                if d <= 15 {
                    let shadow_strength = ((15 - d) as f32 / 15.0 * 45.0) as u32;
                    r = r.saturating_sub(shadow_strength);
                    g = g.saturating_sub(shadow_strength);
                    b = b.saturating_sub(shadow_strength);
                    // Innermost 3px are very dark
                    if d <= 3 {
                        r = r.saturating_sub(20);
                        g = g.saturating_sub(20);
                        b = b.saturating_sub(20);
                    }
                }
            }

            if in_screen {
                // Screen corners rounded
                let scr_r = 12usize;
                let sx = x - SCREEN_X;
                let sy = y - SCREEN_Y;
                let scr_corner =
                    (sx < scr_r && sy < scr_r && sq_dist(sx, sy, scr_r, scr_r) > scr_r * scr_r)
                    || (sx >= SCREEN_W - scr_r && sy < scr_r && sq_dist(sx, sy, SCREEN_W - scr_r, scr_r) > scr_r * scr_r)
                    || (sx < scr_r && sy >= SCREEN_H - scr_r && sq_dist(sx, sy, scr_r, SCREEN_H - scr_r) > scr_r * scr_r)
                    || (sx >= SCREEN_W - scr_r && sy >= SCREEN_H - scr_r && sq_dist(sx, sy, SCREEN_W - scr_r, SCREEN_H - scr_r) > scr_r * scr_r);
                if scr_corner {
                    frame[idx] = 0x080808;
                } else {
                    frame[idx] = 0x000000;
                }
                continue;
            }

            // Bottom panel details (below screen)
            let bottom_start = SCREEN_Y + SCREEN_H + 20;

            // Speaker: dot grid pattern
            let spk_x1 = WINDOW_WIDTH / 2 - 180;
            let spk_x2 = WINDOW_WIDTH / 2 + 180;
            let spk_y1 = bottom_start + 5;
            let spk_y2 = spk_y1 + 55;
            if x >= spk_x1 && x < spk_x2 && y >= spk_y1 && y < spk_y2 {
                let dot_x = (x - spk_x1) % 8;
                let dot_y = (y - spk_y1) % 8;
                // Circular dot holes
                let dcx = dot_x as i32 - 3;
                let dcy = dot_y as i32 - 3;
                if dcx * dcx + dcy * dcy <= 4 {
                    frame[idx] = 0x151515;
                    continue;
                } else if dcx * dcx + dcy * dcy <= 7 {
                    r = r.saturating_sub(15);
                    g = g.saturating_sub(15);
                    b = b.saturating_sub(15);
                }
            }

            // Power LED
            let led_cx = tv_x1 + 65;
            let led_cy = bottom_start + 30;
            let ldx = x as i32 - led_cx as i32;
            let ldy = y as i32 - led_cy as i32;
            let led_dist = ldx * ldx + ldy * ldy;
            if led_dist <= 16 {
                frame[idx] = 0x00EE55;
                continue;
            } else if led_dist <= 80 {
                let glow = (80 - led_dist) as u32;
                frame[idx] = (r.saturating_sub(glow / 3) << 16) | ((g + glow / 2).min(255) << 8) | b.saturating_sub(glow / 4);
                continue;
            }

            // Brand badge
            let badge_cx = WINDOW_WIDTH / 2;
            let badge_y1 = bottom_start + 68;
            let badge_y2 = badge_y1 + 18;
            let badge_hw = 55;
            if x >= badge_cx - badge_hw && x < badge_cx + badge_hw && y >= badge_y1 && y < badge_y2 {
                let by = y - badge_y1;
                if by == 0 { frame[idx] = 0x5A5A5A; continue; }
                if by == 17 { frame[idx] = 0x353535; continue; }
                frame[idx] = 0x444444;
                continue;
            }

            frame[idx] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
        }
    }

    // Drop shadow under TV on wall
    for y in tv_y2..(tv_y2 + 12).min(TV_HEIGHT) {
        for x in tv_x1 + 15..tv_x2 - 15 {
            let idx = y * WINDOW_WIDTH + x;
            let shadow = ((tv_y2 + 12 - y) as u32 * 3).min(30);
            let existing = frame[idx];
            let er = ((existing >> 16) & 0xFF).saturating_sub(shadow);
            let eg = ((existing >> 8) & 0xFF).saturating_sub(shadow);
            let eb = (existing & 0xFF).saturating_sub(shadow);
            frame[idx] = (er << 16) | (eg << 8) | eb;
        }
    }
}

fn sq_dist(x1: usize, y1: usize, x2: usize, y2: usize) -> usize {
    let dx = if x1 > x2 { x1 - x2 } else { x2 - x1 };
    let dy = if y1 > y2 { y1 - y2 } else { y2 - y1 };
    dx * dx + dy * dy
}

fn composite_screen(tv_frame: &[u32], game_output: &[u32], result: &mut Vec<u32>, window_width: usize, window_height: usize) {
    const SCREEN_W: usize = 860;
    const SCREEN_H: usize = 645;
    const SCREEN_X: usize = 210;
    const SCREEN_Y: usize = 85;
    
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
    let console_w = 900;
    let console_h = 160;
    let console_x = (window_width - console_w) / 2;
    let body_y = console_y + 20;
    let body_b = body_y + console_h;
    
    // Surface/shelf — dark matte surface (not wood grain)
    for y in console_y..window_height {
        for x in 0..window_width {
            let idx = y * window_width + x;
            frame[idx] = 0x1C1C1C; // dark surface
        }
    }
    
    // Console body — two-tone: dark top bar + light bottom
    for y in body_y..body_b {
        for x in console_x..console_x + console_w {
            let idx = y * window_width + x;
            let lx = x - console_x;
            let ly = y - body_y;
            
            // Rounded corners (8px)
            let cr = 8usize;
            let in_body_w = console_w;
            let in_body_h = console_h;
            if (lx < cr && ly < cr && sq_dist(lx, ly, cr, cr) > cr * cr) ||
               (lx >= in_body_w - cr && ly < cr && sq_dist(lx, ly, in_body_w - cr, cr) > cr * cr) ||
               (lx < cr && ly >= in_body_h - cr && sq_dist(lx, ly, cr, in_body_h - cr) > cr * cr) ||
               (lx >= in_body_w - cr && ly >= in_body_h - cr && sq_dist(lx, ly, in_body_w - cr, in_body_h - cr) > cr * cr) {
                continue;
            }
            
            if ly < 50 {
                // === TOP DARK STRIPE — cartridge area ===
                let mut c: u32 = 0x2E2E2E;
                
                // Top edge highlight
                if ly == 0 { c = 0x404040; }
                // Bottom edge of stripe
                if ly == 49 { c = 0x222222; }
                
                // Cartridge slot — wide centered area
                let slot_x = console_w / 2 - 160;
                let slot_w = 320;
                if lx >= slot_x && lx < slot_x + slot_w && ly >= 8 && ly < 42 {
                    c = 0x0C0C0C; // deep black slot
                    
                    // Slot border
                    if ly == 8 || ly == 41 { c = 0x050505; }
                    if lx == slot_x || lx == slot_x + slot_w - 1 { c = 0x050505; }
                    
                    // Cartridge body visible inside
                    if lx >= slot_x + 25 && lx < slot_x + slot_w - 25 && ly >= 10 && ly < 40 {
                        // Cartridge shell
                        let cart_grad = ((ly - 10) as f32 / 30.0 * 15.0) as u32;
                        c = (0x65 - cart_grad) << 16 | (0x65 - cart_grad) << 8 | (0x68 - cart_grad);
                        
                        // Gold label on cartridge
                        if lx >= slot_x + 65 && lx < slot_x + slot_w - 65 && ly >= 15 && ly < 35 {
                            let label_grad = ((ly - 15) as u32 * 2).min(20);
                            c = (0xCC - label_grad) << 16 | (0x99 - label_grad) << 8 | (0x22);
                            // Label border
                            if ly == 15 || ly == 34 || lx == slot_x + 65 || lx == slot_x + slot_w - 66 {
                                c = 0x886611;
                            }
                        }
                    }
                }
                
                frame[idx] = c;
            } else {
                // === BOTTOM LIGHT BODY ===
                let grad = ((ly - 50) as f32 / (console_h - 50) as f32 * 12.0) as u32;
                let base = 0xBE - grad;
                let mut r = base;
                let mut g = base;
                let mut b = (base as f32 * 0.97) as u32;
                
                // Top edge of light body — bright line
                if ly == 50 { r = 0xD5; g = 0xD5; b = 0xD3; }
                // Bottom edge
                if ly >= console_h - 2 { r = 0x88; g = 0x88; b = 0x86; }
                // Side edges
                if lx < 3 || lx >= console_w - 3 { r -= 15; g -= 15; b -= 15; }
                
                // === POWER SECTION — left side ===
                let pwr_section_x = 30;
                let pwr_section_w = 120;
                
                // Power LED (circular, 6px radius)
                let led_cx = pwr_section_x + 20;
                let led_cy = 75;
                if ly >= led_cy - 5 && ly <= led_cy + 5 && lx >= led_cx - 5 && lx <= led_cx + 5 {
                    let dx = lx as i32 - led_cx as i32;
                    let dy = ly as i32 - led_cy as i32;
                    let dist = dx * dx + dy * dy;
                    if dist <= 16 {
                        frame[idx] = 0x00DD55; continue;
                    } else if dist <= 36 {
                        frame[idx] = 0x003D15; continue;
                    }
                }
                
                // Power button — raised pill shape
                let btn_x = pwr_section_x + 35;
                let btn_y = 68;
                let btn_w = 70;
                let btn_h = 22;
                if lx >= btn_x && lx < btn_x + btn_w && ly >= btn_y && ly < btn_y + btn_h {
                    let bx = lx - btn_x;
                    let by = ly - btn_y;
                    // Pill shape rounded ends
                    let pill_r = btn_h / 2;
                    let in_pill = if bx < pill_r {
                        sq_dist(bx, by, pill_r, pill_r) <= pill_r * pill_r
                    } else if bx >= btn_w - pill_r {
                        sq_dist(bx, by, btn_w - pill_r, pill_r) <= pill_r * pill_r
                    } else {
                        true
                    };
                    if in_pill {
                        r = 0x5A; g = 0x5A; b = 0x5A;
                        if by < 3 { r = 0x70; g = 0x70; b = 0x70; }
                        if by >= btn_h - 3 { r = 0x42; g = 0x42; b = 0x42; }
                    }
                }
                
                // Divider line after power section
                if lx == pwr_section_x + pwr_section_w && ly >= 55 && ly < console_h - 10 {
                    r = 0x99; g = 0x99; b = 0x97;
                }
                
                // === RESET SECTION — right of center ===
                let rst_section_x = console_w - 170;
                
                // Reset button — pill shape
                let rst_x = rst_section_x + 30;
                let rst_y = 68;
                let rst_w = 80;
                let rst_h = 22;
                if lx >= rst_x && lx < rst_x + rst_w && ly >= rst_y && ly < rst_y + rst_h {
                    let bx = lx - rst_x;
                    let by = ly - rst_y;
                    let pill_r = rst_h / 2;
                    let in_pill = if bx < pill_r {
                        sq_dist(bx, by, pill_r, pill_r) <= pill_r * pill_r
                    } else if bx >= rst_w - pill_r {
                        sq_dist(bx, by, rst_w - pill_r, pill_r) <= pill_r * pill_r
                    } else {
                        true
                    };
                    if in_pill {
                        r = 0x6E; g = 0x6E; b = 0x6E;
                        if by < 3 { r = 0x85; g = 0x85; b = 0x85; }
                        if by >= rst_h - 3 { r = 0x52; g = 0x52; b = 0x52; }
                    }
                }
                
                // Divider line before reset
                if lx == rst_section_x && ly >= 55 && ly < console_h - 10 {
                    r = 0x99; g = 0x99; b = 0x97;
                }
                
                // === CONTROLLER PORTS — center bottom ===
                let port_y_start = 105;
                let port_h = 28;
                let port_w = 80;
                let port_gap = 40;
                let ports_total = port_w * 2 + port_gap;
                let port1_x = (console_w - ports_total) / 2;
                let port2_x = port1_x + port_w + port_gap;
                
                for px in [port1_x, port2_x] {
                    if lx >= px && lx < px + port_w && ly >= port_y_start && ly < port_y_start + port_h {
                        let bx = lx - px;
                        let by = ly - port_y_start;
                        
                        // Port housing — dark rounded rectangle
                        let pr = 5usize;
                        let in_port = if bx < pr && by < pr { sq_dist(bx, by, pr, pr) <= pr * pr }
                            else if bx >= port_w - pr && by < pr { sq_dist(bx, by, port_w - pr, pr) <= pr * pr }
                            else if bx < pr && by >= port_h - pr { sq_dist(bx, by, pr, port_h - pr) <= pr * pr }
                            else if bx >= port_w - pr && by >= port_h - pr { sq_dist(bx, by, port_w - pr, port_h - pr) <= pr * pr }
                            else { true };
                        
                        if in_port {
                            r = 0x22; g = 0x22; b = 0x22;
                            // Inner area with pins
                            if bx >= 8 && bx < port_w - 8 && by >= 6 && by < port_h - 6 {
                                r = 0x15; g = 0x15; b = 0x15;
                                // Gold pins
                                if by >= 10 && by < port_h - 10 && (bx - 8) % 6 < 3 {
                                    r = 0x8A; g = 0x7A; b = 0x4A;
                                }
                            }
                        }
                    }
                }
                
                frame[idx] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
            }
        }
    }
    
    // Drop shadow under console body
    for y in body_b..body_b + 8 {
        if y >= window_height { break; }
        for x in console_x + 5..console_x + console_w - 5 {
            let idx = y * window_width + x;
            let shadow = ((body_b + 8 - y) as u32 * 3).min(20);
            let existing = frame[idx];
            let er = ((existing >> 16) & 0xFF).saturating_sub(shadow);
            let eg = ((existing >> 8) & 0xFF).saturating_sub(shadow);
            let eb = (existing & 0xFF).saturating_sub(shadow);
            frame[idx] = (er << 16) | (eg << 8) | eb;
        }
    }
    
    // Text labels — using draw_text function
    let body_y_offset = body_y;
    draw_text(frame, "POWER", console_x + 45, body_y_offset + 95, 0x707070, window_width);
    draw_text(frame, "RESET", console_x + console_w - 130, body_y_offset + 95, 0x707070, window_width);
    draw_text(frame, "CLICK TO INSERT CARTRIDGE", console_x + console_w / 2 - 50, body_y_offset + 45, 0x444444, window_width);
    
    let ports_total_w = 80 * 2 + 40;
    let port1_lx = (console_w - ports_total_w) / 2;
    let port2_lx = port1_lx + 80 + 40;
    draw_text(frame, "1P", console_x + port1_lx + 35, body_y_offset + 137, 0x707070, window_width);
    draw_text(frame, "2P", console_x + port2_lx + 35, body_y_offset + 137, 0x707070, window_width);
}
