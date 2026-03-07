use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::env;
use std::fs;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use nes_emulator::bus::Bus;
use nes_emulator::cartridge::Cartridge;
use nes_emulator::cpu::Cpu;
use nes_emulator::joypad::JoypadButton;

struct RingBuffer {
    data: Vec<f32>,
    capacity: usize,
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
}

impl RingBuffer {
    fn new(capacity: usize) -> Self {
        RingBuffer {
            data: vec![0.0; capacity],
            capacity,
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
        }
    }

    fn available_read(&self) -> usize {
        let w = self.write_pos.load(Ordering::Acquire);
        let r = self.read_pos.load(Ordering::Acquire);
        if w >= r { w - r } else { self.capacity - r + w }
    }

    fn available_write(&self) -> usize {
        self.capacity - 1 - self.available_read()
    }

    fn push(&self, sample: f32) -> bool {
        if self.available_write() == 0 {
            return false; // Buffer full, drop sample
        }
        let pos = self.write_pos.load(Ordering::Relaxed);
        // Safety: single producer (main thread only writes)
        unsafe {
            let ptr = self.data.as_ptr() as *mut f32;
            *ptr.add(pos) = sample;
        }
        self.write_pos.store((pos + 1) % self.capacity, Ordering::Release);
        true
    }

    fn pop(&self) -> Option<f32> {
        if self.available_read() == 0 {
            return None;
        }
        let pos = self.read_pos.load(Ordering::Relaxed);
        let sample = self.data[pos];
        self.read_pos.store((pos + 1) % self.capacity, Ordering::Release);
        Some(sample)
    }
}

// Safety: single producer (main thread), single consumer (audio thread)
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

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

    // Audio ring buffer — lock-free, single producer / single consumer
    let ring = Arc::new(RingBuffer::new(8192)); // ~170ms at 48kHz
    let ring_audio = ring.clone();
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

                    let ring_cb = ring_audio.clone();
                    let mut last_sample: f32 = 0.0;
                    let stream = device.build_output_stream(
                        &config,
                        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                            for frame in data.chunks_mut(channels) {
                                let sample = if let Some(s) = ring_cb.pop() {
                                    last_sample = s;
                                    s
                                } else {
                                    last_sample *= 0.995;
                                    last_sample
                                };
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
    for _ in 0..2000 {
        ring.push(0.0);
    }

    let mut crt_buffer: Vec<u32> = vec![0; 512 * 480];
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

        // Push audio samples with backpressure — this syncs frame rate to audio
        {
            let samples = bus.apu.drain_samples();
            for &sample in &samples {
                // If ring buffer is nearly full, wait for audio callback to consume
                while ring.available_write() < 2 {
                    std::thread::sleep(std::time::Duration::from_micros(50));
                }
                ring.push(sample);
            }
        }

        if crt_enabled {
            crt_filter(&bus.ppu.frame_data, &mut crt_buffer);
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

fn crt_filter(input: &[u32], output: &mut Vec<u32>) {
    let src_w = 256;
    let src_h = 240;
    let dst_w = 512;
    let dst_h = 480;

    output.resize(dst_w * dst_h, 0);

    for dst_y in 0..dst_h {
        let src_y = dst_y / 2;
        let is_scanline = dst_y % 2 == 1;

        for dst_x in 0..dst_w {
            let src_x = dst_x / 2;

            let pixel = if src_y < src_h && src_x < src_w {
                input[src_y * src_w + src_x]
            } else {
                0
            };

            let mut r = ((pixel >> 16) & 0xFF) as f32;
            let mut g = ((pixel >> 8) & 0xFF) as f32;
            let mut b = (pixel & 0xFF) as f32;

            // Brightness boost to compensate for scanline darkening
            r = (r * 1.2).min(255.0);
            g = (g * 1.2).min(255.0);
            b = (b * 1.2).min(255.0);

            // Horizontal color bleed: blend slightly with neighbor pixel
            if src_x > 0 && src_x < src_w - 1 {
                let left = input[src_y * src_w + src_x - 1];
                let right = input[src_y * src_w + src_x + 1];
                let lr = ((left >> 16) & 0xFF) as f32;
                let lg = ((left >> 8) & 0xFF) as f32;
                let lb = (left & 0xFF) as f32;
                let rr = ((right >> 16) & 0xFF) as f32;
                let rg = ((right >> 8) & 0xFF) as f32;
                let rb = (right & 0xFF) as f32;

                r = r * 0.85 + (lr + rr) * 0.075;
                g = g * 0.85 + (lg + rg) * 0.075;
                b = b * 0.85 + (lb + rb) * 0.075;
            }

            // RGB phosphor pattern: emphasize one color channel per subpixel
            let sub = dst_x % 3;
            match sub {
                0 => { r *= 1.15; g *= 0.85; b *= 0.85; }
                1 => { r *= 0.85; g *= 1.15; b *= 0.85; }
                _ => { r *= 0.85; g *= 0.85; b *= 1.15; }
            }

            // Scanline effect: darken every other row
            if is_scanline {
                r *= 0.55;
                g *= 0.55;
                b *= 0.55;
            }

            // Vignette: darken edges
            let fx = (dst_x as f32 / dst_w as f32) - 0.5;
            let fy = (dst_y as f32 / dst_h as f32) - 0.5;
            let vignette = (1.0 - (fx * fx + fy * fy) * 1.2).clamp(0.3, 1.0);

            r *= vignette;
            g *= vignette;
            b *= vignette;

            // Clamp and pack
            let ri = (r as u32).min(255);
            let gi = (g as u32).min(255);
            let bi = (b as u32).min(255);

            output[dst_y * dst_w + dst_x] = (ri << 16) | (gi << 8) | bi;
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
