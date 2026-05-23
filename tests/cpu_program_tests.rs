mod common;

use common::synthetic_rom::make_ines_rom;
use oxidenes::bus::Bus;
use oxidenes::cartridge::Cartridge;
use oxidenes::cpu::Cpu;

fn make_program_rom(program: &[u8]) -> Vec<u8> {
    let mut rom = make_ines_rom(1, 1, 0, 0);
    let prg_start = 16;
    rom[prg_start..prg_start + program.len()].copy_from_slice(program);

    // Reset vector for a mirrored 16 KiB NROM PRG bank.
    let reset_vector = prg_start + 0x3ffc;
    rom[reset_vector] = 0x00;
    rom[reset_vector + 1] = 0xC0;
    rom
}

#[test]
fn synthetic_cpu_program_executes_fetch_decode_and_branch_loop() {
    let program = [
        0xA9, 0x00, // LDA #$00
        0x85, 0x10, // STA $10
        0xA2, 0x03, // LDX #$03
        0xA5, 0x10, // LDA $10
        0x69, 0x02, // ADC #$02
        0x85, 0x10, // STA $10
        0xCA, // DEX
        0xD0, 0xF7, // BNE $C006
        0xEA, // NOP
    ];
    let cart = Cartridge::new(&make_program_rom(&program)).expect("synthetic ROM loads");
    let mut bus = Bus::new(cart);
    let mut cpu = Cpu::new();

    cpu.reset(&mut bus);

    for _ in 0..300 {
        cpu.clock(&mut bus);
        if cpu.pc == 0xC010 && cpu.cycles == 0 {
            break;
        }
    }

    assert_eq!(bus.cpu_read(0x0010), 0x06);
    assert_eq!(cpu.x, 0x00);
    assert_eq!(cpu.pc, 0xC010);
    assert!(cpu.get_flag(0x02), "DEX should leave zero flag set");
}
