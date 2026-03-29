use oxidenes::cpu::Cpu;
use oxidenes::bus::Bus;
use oxidenes::cartridge::Cartridge;

fn make_minimal_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 16 + 16384 + 8192];
    rom[0] = 0x4E; rom[1] = 0x45; rom[2] = 0x53; rom[3] = 0x1A;
    rom[4] = 1; rom[5] = 1; rom[6] = 0; rom[7] = 0;
    rom
}

fn make_test_cpu_bus() -> (Cpu, Bus) {
    let cart = Cartridge::new(&make_minimal_rom()).unwrap();
    let bus = Bus::new(cart);
    let cpu = Cpu::new();
    (cpu, bus)
}

#[test]
fn cpu_initial_state() {
    let cpu = Cpu::new();
    assert_eq!(cpu.a, 0);
    assert_eq!(cpu.x, 0);
    assert_eq!(cpu.y, 0);
    assert_eq!(cpu.sp, 0xFD);
    assert_eq!(cpu.status, 0x24);
    assert_eq!(cpu.pc, 0);
}

#[test]
fn cpu_flag_set_get() {
    let mut cpu = Cpu::new();
    let flags: [(u8, &str); 8] = [
        (0x01, "carry"), (0x02, "zero"), (0x04, "interrupt"),
        (0x08, "decimal"), (0x10, "break"), (0x20, "unused"),
        (0x40, "overflow"), (0x80, "negative"),
    ];
    for (flag, name) in &flags {
        cpu.set_flag(*flag, true);
        assert!(cpu.get_flag(*flag), "{} flag should be set", name);
        cpu.set_flag(*flag, false);
        assert!(!cpu.get_flag(*flag), "{} flag should be clear", name);
    }
}

#[test]
fn cpu_lda_immediate() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x42); // operand
    cpu.execute(&mut bus, 0xA9); // LDA #$42
    assert_eq!(cpu.a, 0x42);
    assert!(!cpu.get_flag(0x02)); // zero flag clear
    assert!(!cpu.get_flag(0x80)); // negative flag clear
}

#[test]
fn cpu_lda_zero_sets_zero_flag() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x00);
    cpu.execute(&mut bus, 0xA9); // LDA #$00
    assert_eq!(cpu.a, 0x00);
    assert!(cpu.get_flag(0x02)); // zero flag set
}

#[test]
fn cpu_lda_negative_sets_negative_flag() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x80);
    cpu.execute(&mut bus, 0xA9); // LDA #$80
    assert_eq!(cpu.a, 0x80);
    assert!(cpu.get_flag(0x80)); // negative flag set
}

#[test]
fn cpu_ldx_immediate() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0xFF);
    cpu.execute(&mut bus, 0xA2); // LDX #$FF
    assert_eq!(cpu.x, 0xFF);
    assert!(cpu.get_flag(0x80)); // negative flag set
}

#[test]
fn cpu_ldy_immediate() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x00);
    cpu.execute(&mut bus, 0xA0); // LDY #$00
    assert_eq!(cpu.y, 0x00);
    assert!(cpu.get_flag(0x02)); // zero flag set
}

#[test]
fn cpu_sta_zeropage() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x55;
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x10); // address operand
    cpu.execute(&mut bus, 0x85); // STA $10
    assert_eq!(bus.cpu_read(0x0010), 0x55);
}

#[test]
fn cpu_stx_zeropage() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.x = 0xAA;
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x20);
    cpu.execute(&mut bus, 0x86); // STX $20
    assert_eq!(bus.cpu_read(0x0020), 0xAA);
}

#[test]
fn cpu_sty_zeropage() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.y = 0xBB;
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x30);
    cpu.execute(&mut bus, 0x84); // STY $30
    assert_eq!(bus.cpu_read(0x0030), 0xBB);
}

#[test]
fn cpu_adc_basic() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x10;
    cpu.set_flag(0x01, false); // clear carry
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x20);
    cpu.execute(&mut bus, 0x69); // ADC #$20
    assert_eq!(cpu.a, 0x30);
    assert!(!cpu.get_flag(0x01)); // carry clear
    assert!(!cpu.get_flag(0x40)); // overflow clear
}

#[test]
fn cpu_adc_with_carry_in() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x10;
    cpu.set_flag(0x01, true); // set carry
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x20);
    cpu.execute(&mut bus, 0x69); // ADC #$20
    assert_eq!(cpu.a, 0x31); // 0x10 + 0x20 + 1 = 0x31
}

#[test]
fn cpu_adc_carry_out() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0xFF;
    cpu.set_flag(0x01, false);
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x01);
    cpu.execute(&mut bus, 0x69); // ADC #$01
    assert_eq!(cpu.a, 0x00); // wraps
    assert!(cpu.get_flag(0x01)); // carry set
    assert!(cpu.get_flag(0x02)); // zero set
}

#[test]
fn cpu_adc_overflow() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x50;
    cpu.set_flag(0x01, false);
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x50);
    cpu.execute(&mut bus, 0x69); // ADC #$50
    assert_eq!(cpu.a, 0xA0);
    assert!(cpu.get_flag(0x40)); // overflow set (positive + positive = negative)
    assert!(cpu.get_flag(0x80)); // negative set
}

#[test]
fn cpu_sbc_basic() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x50;
    cpu.set_flag(0x01, true); // set carry (no borrow)
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x20);
    cpu.execute(&mut bus, 0xE9); // SBC #$20
    assert_eq!(cpu.a, 0x30);
    assert!(cpu.get_flag(0x01)); // carry set (no borrow)
}

#[test]
fn cpu_sbc_with_borrow() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x50;
    cpu.set_flag(0x01, false); // clear carry = borrow
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x20);
    cpu.execute(&mut bus, 0xE9); // SBC #$20
    assert_eq!(cpu.a, 0x2F); // 0x50 - 0x20 - 1 = 0x2F
}

#[test]
fn cpu_and_immediate() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0xFF;
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x0F);
    cpu.execute(&mut bus, 0x29); // AND #$0F
    assert_eq!(cpu.a, 0x0F);
}

#[test]
fn cpu_ora_immediate() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0xF0;
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x0F);
    cpu.execute(&mut bus, 0x09); // ORA #$0F
    assert_eq!(cpu.a, 0xFF);
}

#[test]
fn cpu_eor_immediate() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0xFF;
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0xF0);
    cpu.execute(&mut bus, 0x49); // EOR #$F0
    assert_eq!(cpu.a, 0x0F);
}

#[test]
fn cpu_inx() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.x = 0x41;
    cpu.execute(&mut bus, 0xE8); // INX (implicit, no operand)
    assert_eq!(cpu.x, 0x42);
}

#[test]
fn cpu_inx_wraps() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.x = 0xFF;
    cpu.execute(&mut bus, 0xE8); // INX
    assert_eq!(cpu.x, 0x00);
    assert!(cpu.get_flag(0x02)); // zero flag
}

#[test]
fn cpu_iny() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.y = 0x05;
    cpu.execute(&mut bus, 0xC8); // INY
    assert_eq!(cpu.y, 0x06);
}

#[test]
fn cpu_dex() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.x = 0x00;
    cpu.execute(&mut bus, 0xCA); // DEX
    assert_eq!(cpu.x, 0xFF);
    assert!(cpu.get_flag(0x80)); // negative flag
}

#[test]
fn cpu_dey() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.y = 0x01;
    cpu.execute(&mut bus, 0x88); // DEY
    assert_eq!(cpu.y, 0x00);
    assert!(cpu.get_flag(0x02)); // zero flag
}

#[test]
fn cpu_tax() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x42;
    cpu.execute(&mut bus, 0xAA); // TAX
    assert_eq!(cpu.x, 0x42);
}

#[test]
fn cpu_tay() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x42;
    cpu.execute(&mut bus, 0xA8); // TAY
    assert_eq!(cpu.y, 0x42);
}

#[test]
fn cpu_txa() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.x = 0x42;
    cpu.execute(&mut bus, 0x8A); // TXA
    assert_eq!(cpu.a, 0x42);
}

#[test]
fn cpu_tya() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.y = 0x42;
    cpu.execute(&mut bus, 0x98); // TYA
    assert_eq!(cpu.a, 0x42);
}

#[test]
fn cpu_pha_pla() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x42;
    cpu.execute(&mut bus, 0x48); // PHA
    cpu.a = 0x00;
    cpu.execute(&mut bus, 0x68); // PLA
    assert_eq!(cpu.a, 0x42);
}

#[test]
fn cpu_php_plp() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.set_flag(0x01, true); // carry
    cpu.set_flag(0x40, true); // overflow
    cpu.execute(&mut bus, 0x08); // PHP
    cpu.status = 0x00;
    cpu.execute(&mut bus, 0x28); // PLP
    // PHP pushes status with BREAK and UNUSED set, PLP clears BREAK
    assert!(cpu.get_flag(0x01)); // carry restored
    assert!(cpu.get_flag(0x40)); // overflow restored
}

#[test]
fn cpu_jmp_absolute() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x00); // low byte of target
    bus.cpu_write(0x0201, 0x04); // high byte = $0400
    cpu.execute(&mut bus, 0x4C); // JMP $0400
    assert_eq!(cpu.pc, 0x0400);
}

#[test]
fn cpu_cmp_equal() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x42;
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x42);
    cpu.execute(&mut bus, 0xC9); // CMP #$42
    assert!(cpu.get_flag(0x02)); // zero flag (equal)
    assert!(cpu.get_flag(0x01)); // carry flag (A >= M)
}

#[test]
fn cpu_cmp_greater() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x50;
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x30);
    cpu.execute(&mut bus, 0xC9); // CMP #$30
    assert!(!cpu.get_flag(0x02)); // zero clear
    assert!(cpu.get_flag(0x01)); // carry set (A >= M)
}

#[test]
fn cpu_cmp_less() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.a = 0x10;
    cpu.pc = 0x0200;
    bus.cpu_write(0x0200, 0x30);
    cpu.execute(&mut bus, 0xC9); // CMP #$30
    assert!(!cpu.get_flag(0x02)); // zero clear
    assert!(!cpu.get_flag(0x01)); // carry clear (A < M)
}

#[test]
fn cpu_sec_clc() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.execute(&mut bus, 0x38); // SEC
    assert!(cpu.get_flag(0x01));
    cpu.execute(&mut bus, 0x18); // CLC
    assert!(!cpu.get_flag(0x01));
}

#[test]
fn cpu_sei_cli() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.execute(&mut bus, 0x78); // SEI
    assert!(cpu.get_flag(0x04));
    cpu.execute(&mut bus, 0x58); // CLI
    assert!(!cpu.get_flag(0x04));
}

#[test]
fn cpu_clv() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.set_flag(0x40, true);
    cpu.execute(&mut bus, 0xB8); // CLV
    assert!(!cpu.get_flag(0x40));
}

#[test]
fn cpu_nmi_pushes_state() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.pc = 0x0200;
    let old_sp = cpu.sp;
    // NMI vector is at 0xFFFA-0xFFFB in cartridge ROM space
    // Just verify SP decreases by 3 (PC high, PC low, status pushed)
    cpu.nmi(&mut bus);
    assert_eq!(cpu.sp, old_sp.wrapping_sub(3));
    assert_eq!(cpu.cycles, 7);
}

#[test]
fn cpu_irq_blocked_when_disabled() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.set_flag(0x04, true); // interrupt disable
    cpu.pc = 0x0200;
    let old_pc = cpu.pc;
    let old_sp = cpu.sp;
    cpu.irq(&mut bus);
    // IRQ should be ignored
    assert_eq!(cpu.pc, old_pc);
    assert_eq!(cpu.sp, old_sp);
}

#[test]
fn cpu_irq_fires_when_enabled() {
    let (mut cpu, mut bus) = make_test_cpu_bus();
    cpu.set_flag(0x04, false); // clear interrupt disable
    cpu.pc = 0x0200;
    let old_sp = cpu.sp;
    cpu.irq(&mut bus);
    // IRQ should push PC + status (3 bytes) and load IRQ vector
    assert_eq!(cpu.sp, old_sp.wrapping_sub(3));
    assert_eq!(cpu.cycles, 7);
    assert!(cpu.get_flag(0x04)); // interrupt disable should be set after IRQ
}

#[test]
fn cpu_save_load_state_roundtrip() {
    let mut cpu = Cpu::new();
    cpu.a = 0x42;
    cpu.x = 0x10;
    cpu.y = 0x20;
    cpu.sp = 0xF0;
    cpu.pc = 0x1234;
    cpu.status = 0x65;

    let state = cpu.save_state();

    let mut cpu2 = Cpu::new();
    assert!(cpu2.load_state(&state));
    assert_eq!(cpu2.a, 0x42);
    assert_eq!(cpu2.x, 0x10);
    assert_eq!(cpu2.y, 0x20);
    assert_eq!(cpu2.sp, 0xF0);
    assert_eq!(cpu2.pc, 0x1234);
    assert_eq!(cpu2.status, 0x65);
}
