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

    // TV dimensions for 90s CRT frame
    const TV_WIDTH: usize = 800;
    const TV_HEIGHT: usize = 680;
    
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

    let mut crt_buffer: Vec<u32> = vec![0; 512 * 480];
    
    // Build static TV frame once at startup (zero per-frame cost)
    let mut tv_frame_bg = Vec::new();
    build_tv_frame(&mut tv_frame_bg);
    let mut composite_buffer = vec![0u32; TV_WIDTH * TV_HEIGHT];
    
    // Pre-compute vignette lookup table (same every frame)
    let vignette_table = {
        let dst_w = 512;
        let dst_h = 480;
        let mut table = vec![0u16; dst_w * dst_h];
        for dst_y in 0..dst_h {
            for dst_x in 0..dst_w {
                let fx = (dst_x as f32 / dst_w as f32) - 0.5;
                let fy = (dst_y as f32 / dst_h as f32) - 0.5;
                let v = (1.0 - (fx * fx + fy * fy) * 1.2).clamp(0.3, 1.0);
                // Store as fixed-point: 0..256 maps to 0.0..1.0
                table[dst_y * dst_w + dst_x] = (v * 256.0) as u16;
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
            scale_2x(&bus.ppu.frame_data, &mut crt_buffer);
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
            // Xbox A (South) = NES B, Xbox B (East) = NES A
            a_pressed |= gamepad.is_pressed(Button::East);   // Xbox B → NES A
            a_pressed |= gamepad.is_pressed(Button::North);  // Xbox Y → NES A (turbo alt)
            
            b_pressed |= gamepad.is_pressed(Button::South);  // Xbox A → NES B
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

fn crt_filter(input: &[u32], output: &mut Vec<u32>, vignette: &[u16]) {
    let src_w = 256;
    let src_h = 240;
    let dst_w = 512;
    let dst_h = 480;

    output.resize(dst_w * dst_h, 0);

    // Phosphor pattern lookup: [sub_pixel][channel] = multiplier (fixed-point >>8)
    // sub=0: R*1.15, G*0.85, B*0.85
    // sub=1: R*0.85, G*1.15, B*0.85
    // sub=2: R*0.85, G*0.85, B*1.15
    const PHOSPHOR: [[u16; 3]; 3] = [
        [294, 217, 217],  // 1.15*256, 0.85*256, 0.85*256
        [217, 294, 217],
        [217, 217, 294],
    ];

    // Scanline multiplier: normal=0.55*256=141, bright=1.0*256=256 (but we apply brightness 1.2 first)
    // Combined brightness + scanline: normal_line=1.2*256=307, scanline=1.2*0.55*256=169
    const BRIGHT: u16 = 307;  // 1.2 * 256
    const SCANLINE: u16 = 169;  // 1.2 * 0.55 * 256

    for dst_y in 0..dst_h {
        let src_y = dst_y / 2;
        let line_mul = if dst_y % 2 == 1 { SCANLINE } else { BRIGHT };
        let row_offset = src_y * src_w;
        let dst_row = dst_y * dst_w;

        for dst_x in 0..dst_w {
            let src_x = dst_x / 2;

            let pixel = if src_y < src_h && src_x < src_w {
                input[row_offset + src_x]
            } else {
                0
            };

            let r = ((pixel >> 16) & 0xFF) as u32;
            let g = ((pixel >> 8) & 0xFF) as u32;
            let b = (pixel & 0xFF) as u32;

            // Combined: brightness * scanline * phosphor * vignette
            // All are fixed-point >>8, so we need to shift appropriately
            let phos = &PHOSPHOR[dst_x % 3];
            let vig = vignette[dst_row + dst_x] as u32;

            // Split shifts to avoid u32 overflow: (channel * line_mul * phosphor) >> 16, then * vig >> 8
            let ri = (((r * line_mul as u32 * phos[0] as u32) >> 16) * vig >> 8).min(255);
            let gi = (((g * line_mul as u32 * phos[1] as u32) >> 16) * vig >> 8).min(255);
            let bi = (((b * line_mul as u32 * phos[2] as u32) >> 16) * vig >> 8).min(255);

            output[dst_row + dst_x] = (ri << 16) | (gi << 8) | bi;
        }
    }
}

fn scale_2x(input: &[u32], output: &mut Vec<u32>) {
    output.resize(512 * 480, 0);
    for y in 0..240 {
        for x in 0..256 {
            let pixel = input[y * 256 + x];
            let dst_x = x * 2;
            let dst_y = y * 2;
            output[dst_y * 512 + dst_x] = pixel;
            output[dst_y * 512 + dst_x + 1] = pixel;
            output[(dst_y + 1) * 512 + dst_x] = pixel;
            output[(dst_y + 1) * 512 + dst_x + 1] = pixel;
        }
    }
}

fn build_tv_frame(frame: &mut Vec<u32>) {
    const TV_WIDTH: usize = 800;
    const TV_HEIGHT: usize = 680;
    const SCREEN_X: usize = 144;  // (800 - 512) / 2
    const SCREEN_Y: usize = 60;
    const SCREEN_W: usize = 512;
    const SCREEN_H: usize = 480;
    
    frame.resize(TV_WIDTH * TV_HEIGHT, 0);
    
    for y in 0..TV_HEIGHT {
        for x in 0..TV_WIDTH {
            let idx = y * TV_WIDTH + x;
            
            // Check if in screen area
            let in_screen = x >= SCREEN_X && x < SCREEN_X + SCREEN_W 
                         && y >= SCREEN_Y && y < SCREEN_Y + SCREEN_H;
            
            if in_screen {
                // Screen area — will be overwritten with game output
                frame[idx] = 0x000000;
            } else {
                // TV bezel
                let edge_dist_x = x.min(TV_WIDTH - 1 - x) as f32 / TV_WIDTH as f32;
                let edge_dist_y = y.min(TV_HEIGHT - 1 - y) as f32 / TV_HEIGHT as f32;
                let edge_dist = edge_dist_x.min(edge_dist_y);
                
                // Base bezel color (dark warm gray, like a 90s TV)
                let base_r: u32 = 55;
                let base_g: u32 = 52;
                let base_b: u32 = 50;
                
                // Slight gradient for 3D effect — lighter toward top-left
                let highlight = if y < TV_HEIGHT / 2 { 
                    (15.0 * (1.0 - y as f32 / (TV_HEIGHT as f32 / 2.0))) as u32
                } else { 0 };
                
                // Darken at very edges (rounded corner illusion)
                let corner_dark = if edge_dist < 0.02 {
                    ((0.02 - edge_dist) / 0.02 * 30.0) as u32
                } else { 0 };
                
                // Inner bevel near screen
                let screen_dist_x = if x < SCREEN_X { SCREEN_X - x } 
                                   else if x >= SCREEN_X + SCREEN_W { x - (SCREEN_X + SCREEN_W) + 1 }
                                   else { 999 };
                let screen_dist_y = if y < SCREEN_Y { SCREEN_Y - y }
                                   else if y >= SCREEN_Y + SCREEN_H { y - (SCREEN_Y + SCREEN_H) + 1 }
                                   else { 999 };
                let screen_dist = screen_dist_x.min(screen_dist_y);
                
                let bevel = if screen_dist < 8 {
                    (8 - screen_dist) as u32 * 3
                } else { 0 };
                
                let r = (base_r + highlight).saturating_sub(corner_dark).saturating_sub(bevel).min(255);
                let g = (base_g + highlight).saturating_sub(corner_dark).saturating_sub(bevel).min(255);
                let b = (base_b + highlight).saturating_sub(corner_dark).saturating_sub(bevel).min(255);
                
                frame[idx] = (r << 16) | (g << 8) | b;
                
                // Brand label area at bottom center
                if y >= SCREEN_Y + SCREEN_H + 20 && y < SCREEN_Y + SCREEN_H + 35 
                   && x >= TV_WIDTH / 2 - 30 && x < TV_WIDTH / 2 + 30 {
                    // Slight lighter area for "brand" badge
                    frame[idx] = 0x4A4745;
                }
            }
        }
    }
}

fn composite_screen(tv_frame: &[u32], game_output: &[u32], result: &mut Vec<u32>) {
    const TV_WIDTH: usize = 800;
    const TV_HEIGHT: usize = 680;
    const SCREEN_X: usize = 144;
    const SCREEN_Y: usize = 60;
    const SCREEN_W: usize = 512;
    const SCREEN_H: usize = 480;
    
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
