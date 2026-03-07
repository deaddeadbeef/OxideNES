use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::env;
use std::fs;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::{traits::*, HeapRb};

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

    let mut window = Window::new(
        "NES Emulator",
        512,
        480,
        WindowOptions {
            scale: Scale::X1,
            ..WindowOptions::default()
        },
    )
    .expect("Failed to create window");

    window.set_target_fps(60);

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
    let mut audio_diag_printed = false;

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
            if !audio_diag_printed {
                eprintln!("[Audio] First frame: {} samples produced, ring capacity: 8192, sample_rate: {}",
                    samples.len(), actual_sample_rate);
                audio_diag_printed = true;
            }
            for &sample in &samples {
                let _ = producer.try_push(sample);
            }
        }

        if crt_enabled {
            crt_filter(&bus.ppu.frame_data, &mut crt_buffer, &vignette_table);
        } else {
            scale_2x(&bus.ppu.frame_data, &mut crt_buffer);
        }

        window
            .update_with_buffer(&crt_buffer, 512, 480)
            .expect("Failed to update window");

        handle_input(&window, &mut bus);

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

fn handle_input(window: &Window, bus: &mut Bus) {
    let key_map: [(Key, JoypadButton); 8] = [
        (Key::Z, JoypadButton::A),
        (Key::X, JoypadButton::B),
        (Key::Space, JoypadButton::Select),
        (Key::Enter, JoypadButton::Start),
        (Key::Up, JoypadButton::Up),
        (Key::Down, JoypadButton::Down),
        (Key::Left, JoypadButton::Left),
        (Key::Right, JoypadButton::Right),
    ];

    let keys = window.get_keys();
    for (key, button) in &key_map {
        bus.joypad1.set_button_pressed(*button, keys.contains(key));
    }
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

            let mut r = ((pixel >> 16) & 0xFF) as u32;
            let mut g = ((pixel >> 8) & 0xFF) as u32;
            let mut b = (pixel & 0xFF) as u32;

            // Color bleed with neighbors (integer approximation)
            // Original: r = r*0.85 + (lr+rr)*0.075
            // Fixed-point: r = (r*217 + (lr+rr)*19) >> 8
            if src_x > 0 && src_x < src_w - 1 {
                let left = input[row_offset + src_x - 1];
                let right = input[row_offset + src_x + 1];
                let lr = ((left >> 16) & 0xFF) as u32;
                let lg = ((left >> 8) & 0xFF) as u32;
                let lb = (left & 0xFF) as u32;
                let rr = ((right >> 16) & 0xFF) as u32;
                let rg = ((right >> 8) & 0xFF) as u32;
                let rb = (right & 0xFF) as u32;

                r = (r * 217 + (lr + rr) * 19) >> 8;
                g = (g * 217 + (lg + rg) * 19) >> 8;
                b = (b * 217 + (lb + rb) * 19) >> 8;
            }

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
