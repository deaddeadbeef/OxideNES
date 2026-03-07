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
                let console_x = (WINDOW_WIDTH - 800) / 2;
                let body_y = TV_HEIGHT + 18;
                
                // RESET button hit test
                let rst_x = console_x + 170;
                let rst_y = body_y + 55;
                if mx >= rst_x && mx < rst_x + 65 && my >= rst_y && my < rst_y + 18 {
                    cpu.reset(&mut bus);
                    println!("CPU Reset");
                }
                
                // Cartridge slot hit test
                let slot_x = console_x + 800 / 2 - 100;
                let slot_y = body_y + 5;
                if mx >= slot_x && mx < slot_x + 200 && my >= slot_y && my < slot_y + 28 {
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
    const SCREEN_W: usize = 860;
    const SCREEN_H: usize = 645;
    
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
    let console_w = 800;
    let console_h = 150;
    let console_x = (window_width - console_w) / 2;

    // Shelf/surface — rich wood grain
    for y in console_y..window_height {
        for x in 0..window_width {
            let idx = y * window_width + x;
            let ry = (y - console_y) as f32;
            // Slight vertical gradient for depth on shelf
            let depth = (1.0 - ry / (window_height - console_y) as f32 * 0.25).max(0.7);
            let base_r = (90.0 * depth) as u32;
            let base_g = (65.0 * depth) as u32;
            let base_b = (42.0 * depth) as u32;
            let grain1 = ((x.wrapping_mul(13) + y.wrapping_mul(7)) % 12) as u32;
            let grain2 = ((x.wrapping_mul(3) + y.wrapping_mul(11)) % 8) as u32;
            let grain3 = ((x.wrapping_mul(97) ^ y.wrapping_mul(53)) % 6) as u32;
            frame[idx] = ((base_r + grain1 + grain3).min(130) << 16) | ((base_g + grain2).min(90) << 8) | (base_b + grain1 / 2 + grain3 / 3).min(70);
        }
    }

    // Shelf front edge highlight
    for x in 0..window_width {
        let idx = console_y * window_width + x;
        let existing = frame[idx];
        let er = (((existing >> 16) & 0xFF) + 20).min(255);
        let eg = (((existing >> 8) & 0xFF) + 15).min(255);
        let eb = ((existing & 0xFF) + 10).min(255);
        frame[idx] = (er << 16) | (eg << 8) | eb;
    }

    let body_y = console_y + 18;
    let body_r = console_x + console_w;
    let body_b = body_y + console_h;

    // Console shadow on shelf (drawn before console body)
    for y in body_b..(body_b + 10).min(window_height) {
        for x in (console_x + 8)..(body_r - 8) {
            let idx = y * window_width + x;
            let shadow_alpha = ((body_b + 10 - y) as u32 * 6).min(50);
            let existing = frame[idx];
            let er = ((existing >> 16) & 0xFF).saturating_sub(shadow_alpha);
            let eg = ((existing >> 8) & 0xFF).saturating_sub(shadow_alpha);
            let eb = (existing & 0xFF).saturating_sub(shadow_alpha);
            frame[idx] = (er << 16) | (eg << 8) | eb;
        }
    }
    // Side shadows
    for y in body_y..body_b {
        for dx in 0..6usize {
            let shadow = ((6 - dx) as u32 * 5).min(25);
            // Left side shadow
            let xl = console_x.saturating_sub(dx + 1);
            if xl < window_width && y < window_height {
                let idx = y * window_width + xl;
                let existing = frame[idx];
                let er = ((existing >> 16) & 0xFF).saturating_sub(shadow);
                let eg = ((existing >> 8) & 0xFF).saturating_sub(shadow);
                let eb = (existing & 0xFF).saturating_sub(shadow);
                frame[idx] = (er << 16) | (eg << 8) | eb;
            }
            // Right side shadow
            let xr = body_r + dx;
            if xr < window_width && y < window_height {
                let idx = y * window_width + xr;
                let existing = frame[idx];
                let er = ((existing >> 16) & 0xFF).saturating_sub(shadow);
                let eg = ((existing >> 8) & 0xFF).saturating_sub(shadow);
                let eb = (existing & 0xFF).saturating_sub(shadow);
                frame[idx] = (er << 16) | (eg << 8) | eb;
            }
        }
    }

    for y in body_y..body_b {
        for x in console_x..body_r {
            let idx = y * window_width + x;
            let lx = x - console_x;
            let ly = y - body_y;

            // Rounded corners for console body
            let cr = 12usize;
            let skip =
                (lx < cr && ly < cr && sq_dist(lx, ly, cr, cr) > cr * cr)
                || (lx >= console_w - cr && ly < cr && sq_dist(lx, ly, console_w - cr - 1, cr) > cr * cr)
                || (lx < cr && ly >= console_h - cr && sq_dist(lx, ly, cr, console_h - cr - 1) > cr * cr)
                || (lx >= console_w - cr && ly >= console_h - cr && sq_dist(lx, ly, console_w - cr - 1, console_h - cr - 1) > cr * cr);
            if skip { continue; }

            // Top dark stripe (cartridge area) — first 40px
            if ly < 40 {
                let stripe_grad = ly as f32 / 40.0;
                let mut c_r = (0x38 as f32 + stripe_grad * 8.0) as u32;
                let mut c_g = c_r;
                let mut c_b = c_r;

                // Top bevel
                if ly < 3 { c_r += 18; c_g += 18; c_b += 18; }
                if ly >= 37 { c_r = c_r.saturating_sub(12); c_g = c_g.saturating_sub(12); c_b = c_b.saturating_sub(12); }

                // Side bevels
                if lx < 4 { c_r += 8; c_g += 8; c_b += 8; }
                if lx >= console_w - 4 { c_r = c_r.saturating_sub(8); c_g = c_g.saturating_sub(8); c_b = c_b.saturating_sub(8); }

                // Cartridge slot — centered dark rectangle with depth
                let slot_x = console_w / 2 - 100;
                let slot_w = 200;
                if lx >= slot_x && lx < slot_x + slot_w && ly >= 5 && ly < 33 {
                    let bx = lx - slot_x;
                    let by = ly - 5;
                    c_r = 0x10; c_g = 0x10; c_b = 0x10;
                    // Slot beveled edges (inset look)
                    if by < 2 { c_r = 0x08; c_g = 0x08; c_b = 0x08; }
                    if by >= 26 { c_r = 0x1A; c_g = 0x1A; c_b = 0x1A; }
                    if bx < 2 { c_r = 0x08; c_g = 0x08; c_b = 0x08; }
                    if bx >= slot_w - 2 { c_r = 0x1A; c_g = 0x1A; c_b = 0x1A; }
                    // Cartridge visible inside slot
                    if bx >= 15 && bx < slot_w - 15 && by >= 4 && by < 25 {
                        let cart_by = by - 4;
                        c_r = 0x6A; c_g = 0x6A; c_b = 0x6A;
                        // Cart top highlight
                        if cart_by < 2 { c_r = 0x78; c_g = 0x78; c_b = 0x78; }
                        // Cart label area
                        if bx >= 35 && bx < slot_w - 35 && by >= 8 && by < 22 {
                            c_r = 0xCC; c_g = 0x99; c_b = 0x22;
                            let lbl_bx = bx - 35;
                            let lbl_by = by - 8;
                            // Label border
                            if lbl_by == 0 || lbl_by == 13 || lbl_bx == 0 || lbl_bx == slot_w - 71 {
                                c_r = 0xAA; c_g = 0x80; c_b = 0x18;
                            }
                        }
                    }
                }

                // Plastic texture
                let tex = ((x.wrapping_mul(3571) ^ y.wrapping_mul(2311)) % 4) as u32;
                c_r = c_r.saturating_sub(tex / 2);

                frame[idx] = (c_r.min(255) << 16) | (c_g.min(255) << 8) | c_b.min(255);
            } else {
                // Light body — smooth gradient with warm tint
                let grad = (ly - 40) as f32 / (console_h - 40) as f32;
                let base = (200.0 - grad * 35.0) as u32;
                let mut r = base;
                let mut g = base;
                let mut b = (base as f32 * 0.96) as u32; // slight warm tint

                // Horizontal curvature — lighter at center
                let hx = (lx as f32 / console_w as f32 - 0.5).abs();
                let hboost = ((1.0 - hx * 1.8).max(0.0) * 6.0) as u32;
                r += hboost; g += hboost; b += hboost;

                // Top highlight of body
                if ly == 40 { r += 25; g += 25; b += 25; }
                if ly == 41 { r += 12; g += 12; b += 12; }
                // Bottom shadow
                if ly >= console_h - 4 {
                    let sd = (ly - (console_h - 4)) as u32 * 10;
                    r = r.saturating_sub(sd); g = g.saturating_sub(sd); b = b.saturating_sub(sd);
                }
                // Side shadows
                if lx < 6 { let sd = (6 - lx) as u32 * 4; r = r.saturating_sub(sd); g = g.saturating_sub(sd); b = b.saturating_sub(sd); }
                if lx >= console_w - 6 { let sd = (lx - (console_w - 6)) as u32 * 5; r = r.saturating_sub(sd); g = g.saturating_sub(sd); b = b.saturating_sub(sd); }

                // POWER button — larger, raised with 3D effect
                let pwr_x = 55usize;
                let pwr_y = 52usize;
                let pwr_w = 60usize;
                let pwr_h = 22usize;
                if lx >= pwr_x && lx < pwr_x + pwr_w && ly >= pwr_y && ly < pwr_y + pwr_h {
                    let bx = lx - pwr_x;
                    let by = ly - pwr_y;
                    let btn_grad = by as f32 / pwr_h as f32;
                    r = (95.0 - btn_grad * 20.0) as u32;
                    g = r; b = r;
                    if by < 2 { r = 115; g = 115; b = 115; }
                    if by >= pwr_h - 2 { r = 48; g = 48; b = 48; }
                    if bx < 2 { r = 105; g = 105; b = 105; }
                    if bx >= pwr_w - 2 { r = 52; g = 52; b = 52; }
                }

                // Power LED — circular with glow
                let led_cx = 38usize;
                let led_cy = 60usize;
                if lx < led_cx + 12 && ly < led_cy + 12 && lx + 12 > led_cx && ly + 12 > led_cy {
                    let dx = lx as i32 - led_cx as i32;
                    let dy = ly as i32 - led_cy as i32;
                    let dist = dx * dx + dy * dy;
                    if dist <= 9 {
                        r = 0; g = 230; b = 72;
                    } else if dist <= 20 {
                        r = 0; g = 100; b = 30;
                    } else if dist <= 50 {
                        let gf = (50 - dist) as u32;
                        g = (g + gf / 2).min(255);
                    }
                }

                // RESET button — recessed with proper depth
                let rst_x = 170usize;
                let rst_y = 55usize;
                let rst_w = 65usize;
                let rst_h = 18usize;
                if lx >= rst_x && lx < rst_x + rst_w && ly >= rst_y && ly < rst_y + rst_h {
                    let bx = lx - rst_x;
                    let by = ly - rst_y;
                    r = 105; g = 105; b = 105;
                    // Recessed: top/left shadow, bottom/right highlight
                    if by < 2 { r = 72; g = 72; b = 72; }
                    if by >= rst_h - 2 { r = 125; g = 125; b = 125; }
                    if bx < 2 { r = 78; g = 78; b = 78; }
                    if bx >= rst_w - 2 { r = 118; g = 118; b = 118; }
                }

                // Controller ports — two with proper depth
                let port_y = 90usize;
                let port_h = 26usize;
                let port_w = 72usize;
                let port1_x = console_w / 2 - 110;
                let port2_x = console_w / 2 + 38;

                for port_x in [port1_x, port2_x] {
                    if lx >= port_x && lx < port_x + port_w && ly >= port_y && ly < port_y + port_h {
                        let bx = lx - port_x;
                        let by = ly - port_y;
                        r = 35; g = 35; b = 35;
                        // Outer bezel of port
                        if by < 2 { r = 25; g = 25; b = 25; }
                        if by >= port_h - 2 { r = 55; g = 55; b = 55; }
                        if bx < 2 { r = 28; g = 28; b = 28; }
                        if bx >= port_w - 2 { r = 48; g = 48; b = 48; }
                        // Inner cavity
                        if bx >= 6 && bx < port_w - 6 && by >= 4 && by < port_h - 4 {
                            r = 15; g = 15; b = 15;
                            // Connector pins
                            if by >= 8 && by < port_h - 8 && bx >= 14 && bx < port_w - 14 {
                                if (bx - 14) % 5 < 3 {
                                    r = 85; g = 78; b = 55; // brass pins
                                }
                            }
                        }
                    }
                }

                // Decorative ridge line
                if ly == 82 && lx >= 25 && lx < console_w - 25 {
                    r = r.saturating_sub(25); g = g.saturating_sub(25); b = b.saturating_sub(25);
                }
                if ly == 83 && lx >= 25 && lx < console_w - 25 {
                    r = (r + 12).min(255); g = (g + 12).min(255); b = (b + 12).min(255);
                }

                // Plastic texture
                let tex = ((x.wrapping_mul(5813) ^ y.wrapping_mul(3947)) % 4) as u32;
                r = r.saturating_sub(tex / 2);

                frame[idx] = (r.min(255) << 16) | (g.min(255) << 8) | b.min(255);
            }
        }
    }

    // Labels — adjust positions to match new console layout
    let console_x_text = console_x;
    let body_y_text = body_y;

    draw_text(frame, "POWER", console_x_text + 50, body_y_text + 76, 0x808080, window_width);
    draw_text(frame, "RESET", console_x_text + 172, body_y_text + 76, 0x808080, window_width);
    draw_text(frame, "INSERT CARTRIDGE", console_x_text + console_w / 2 - 32, body_y_text + 35, 0x555555, window_width);

    let port1_x = console_x_text + console_w / 2 - 110;
    let port2_x = console_x_text + console_w / 2 + 38;
    draw_text(frame, "1", port1_x + 33, body_y_text + 120, 0x808080, window_width);
    draw_text(frame, "2", port2_x + 33, body_y_text + 120, 0x808080, window_width);
}
