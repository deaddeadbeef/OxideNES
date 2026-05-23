use oxidenes::bus::Bus;
use oxidenes::cartridge::Cartridge;
use oxidenes::cpu::Cpu;
use std::fs;

#[test]
fn nestest_official_opcodes() {
    let rom_data = fs::read("nestest.nes").expect("nestest.nes not found in project root");
    let cart = Cartridge::new(&rom_data).expect("Failed to load nestest.nes");
    let mut bus = Bus::new(cart);
    let mut cpu = Cpu::new();

    // nestest automation mode: start at $C000 instead of reset vector
    cpu.pc = 0xC000;
    cpu.status = 0x24;
    cpu.sp = 0xFD;

    // Run enough instructions to complete official opcode tests
    for _ in 0..30000 {
        cpu.clock(&mut bus);
        if cpu.pc == 0xC66E {
            break;
        }
    }

    let result = bus.cpu_read(0x0002);
    assert_eq!(
        result, 0x00,
        "nestest official opcodes FAILED: error code 0x{:02X} at PC=0x{:04X}",
        result, cpu.pc
    );
}

#[test]
fn nestest_runs_without_crash() {
    let rom_data = fs::read("nestest.nes").expect("nestest.nes not found in project root");
    let cart = Cartridge::new(&rom_data).expect("Failed to load nestest.nes");
    let mut bus = Bus::new(cart);
    let mut cpu = Cpu::new();

    cpu.pc = 0xC000;
    cpu.status = 0x24;
    cpu.sp = 0xFD;

    for _ in 0..50000 {
        cpu.clock(&mut bus);
    }
}
