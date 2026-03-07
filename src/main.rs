use minifb::{Key, Scale, Window, WindowOptions};
use std::env;
use std::fs;

use nes_emulator::bus::Bus;
use nes_emulator::cartridge::Cartridge;
use nes_emulator::cpu::Cpu;
use nes_emulator::joypad::JoypadButton;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: nes-emulator <rom_file.nes>");
        std::process::exit(1);
    }

    let rom_data = fs::read(&args[1]).expect("Failed to read ROM file");
    let cartridge = Cartridge::new(&rom_data).expect("Failed to load ROM");

    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut window = Window::new(
        "NES Emulator",
        256,
        240,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    )
    .expect("Failed to create window");

    // ~60fps (16.6ms per frame)
    window.set_target_fps(60);

    let mut total_cycles: usize = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Handle input
        handle_input(&window, &mut bus);

        // Run CPU/PPU until a frame is complete
        loop {
            if bus.dma_active() {
                bus.dma_tick(total_cycles % 2 == 1);
                total_cycles += 1;
            } else {
                cpu.clock(&mut bus);
                total_cycles += 1;
            }

            // Check NMI from PPU
            if bus.poll_nmi() {
                cpu.nmi(&mut bus);
            }

            // Tick PPU (3 PPU cycles per CPU cycle); returns true if NMI triggered
            if bus.tick(1) {
                cpu.nmi(&mut bus);
            }

            if bus.ppu.frame_complete() {
                break;
            }
        }

        // Update window with frame data
        window
            .update_with_buffer(&bus.ppu.frame_data, 256, 240)
            .expect("Failed to update window");
    }
}

fn handle_input(window: &Window, bus: &mut Bus) {
    let key_map = [
        (Key::Z, JoypadButton::A),
        (Key::X, JoypadButton::B),
        (Key::Space, JoypadButton::Select),
        (Key::Enter, JoypadButton::Start),
        (Key::Up, JoypadButton::Up),
        (Key::Down, JoypadButton::Down),
        (Key::Left, JoypadButton::Left),
        (Key::Right, JoypadButton::Right),
    ];

    for (key, button) in &key_map {
        bus.joypad1
            .set_button_pressed(*button, window.is_key_down(*key));
    }
}
