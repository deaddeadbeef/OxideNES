use oxidenes::cartridge::{Cartridge, Mirroring};
use oxidenes::mapper::MapperEnum;

mod common;
use common::synthetic_rom::{make_ines_rom, prg_bank_marker, FIXTURE_PROVENANCE};

fn make_rom(prg_banks: u8, chr_banks: u8, mapper: u8, flags: u8) -> Vec<u8> {
    let prg_size = prg_banks as usize * 16384;
    let chr_size = chr_banks as usize * 8192;
    let mut rom = vec![0u8; 16 + prg_size + chr_size];
    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = prg_banks;
    rom[5] = chr_banks;
    rom[6] = ((mapper & 0x0F) << 4) | (flags & 0x0F);
    rom[7] = mapper & 0xF0;
    for i in 0..prg_size {
        rom[16 + i] = (i & 0xFF) as u8;
    }
    for i in 0..chr_size {
        rom[16 + prg_size + i] = ((i >> 2) & 0xFF) as u8;
    }
    rom
}

fn assert_mapper_variant(mapper: u16, mapper_enum: &MapperEnum) {
    match mapper {
        0 => assert!(matches!(mapper_enum, MapperEnum::Mapper000(_))),
        1 => assert!(matches!(mapper_enum, MapperEnum::Mapper001(_))),
        2 => assert!(matches!(mapper_enum, MapperEnum::Mapper002(_))),
        3 => assert!(matches!(mapper_enum, MapperEnum::Mapper003(_))),
        4 => assert!(matches!(mapper_enum, MapperEnum::Mapper004(_))),
        5 => assert!(matches!(mapper_enum, MapperEnum::Mapper005(_))),
        7 => assert!(matches!(mapper_enum, MapperEnum::Mapper007(_))),
        9 => assert!(matches!(mapper_enum, MapperEnum::Mapper009(_))),
        10 => assert!(matches!(mapper_enum, MapperEnum::Mapper010(_))),
        11 => assert!(matches!(mapper_enum, MapperEnum::Mapper011(_))),
        19 => assert!(matches!(mapper_enum, MapperEnum::Mapper019(_))),
        24 => assert!(matches!(mapper_enum, MapperEnum::Mapper024(_))),
        26 => assert!(matches!(mapper_enum, MapperEnum::Mapper026(_))),
        34 => assert!(matches!(mapper_enum, MapperEnum::Mapper034(_))),
        66 => assert!(matches!(mapper_enum, MapperEnum::Mapper066(_))),
        69 => assert!(matches!(mapper_enum, MapperEnum::Mapper069(_))),
        71 => assert!(matches!(mapper_enum, MapperEnum::Mapper071(_))),
        79 => assert!(matches!(mapper_enum, MapperEnum::Mapper079(_))),
        85 => assert!(matches!(mapper_enum, MapperEnum::Mapper085(_))),
        206 => assert!(matches!(mapper_enum, MapperEnum::Mapper206(_))),
        _ => panic!("missing synthetic mapper assertion for mapper {mapper}"),
    }
}

#[test]
fn synthetic_fixture_provenance_is_declared() {
    assert!(FIXTURE_PROVENANCE.contains("Generated synthetic iNES fixture"));
    assert!(FIXTURE_PROVENANCE.contains("no ROM content"));
}

#[test]
fn supported_mappers_construct_from_synthetic_ines_headers() {
    let supported_mappers = [
        0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 19, 24, 26, 34, 66, 69, 71, 79, 85, 206,
    ];

    for mapper in supported_mappers {
        let (prg_banks, chr_banks) = if mapper == 0 { (2, 1) } else { (4, 4) };
        let rom = make_ines_rom(prg_banks, chr_banks, mapper, 0);
        let cart = Cartridge::new(&rom).unwrap_or_else(|err| {
            panic!("synthetic mapper {mapper} fixture should construct: {err}")
        });

        assert_mapper_variant(mapper, &cart.mapper);
    }
}
/// Write a 5-bit value to an MMC1 register via the serial shift interface.
fn mmc1_write(mapper: &mut MapperEnum, addr: u16, value: u8) {
    for bit in 0..5 {
        mapper.write_prg(addr, (value >> bit) & 1);
    }
}

// -- Mapper 0 (NROM) -------------------------------------------------------

#[test]
fn mapper0_prg_read() {
    // 1 PRG bank (16KB): $C000-$FFFF mirrors $8000-$BFFF
    let rom = make_rom(1, 1, 0, 0);
    let cart = Cartridge::new(&rom).unwrap();
    // PRG data pattern: (offset & 0xFF)
    assert_eq!(cart.mapper.read_prg(0x8000), 0x00);
    assert_eq!(cart.mapper.read_prg(0x8001), 0x01);
    assert_eq!(cart.mapper.read_prg(0x80FF), 0xFF);
    // Mirror: $C000 should equal $8000
    assert_eq!(
        cart.mapper.read_prg(0xC000),
        cart.mapper.read_prg(0x8000),
        "$C000 should mirror $8000 with 1 PRG bank"
    );
    assert_eq!(
        cart.mapper.read_prg(0xFFFF),
        cart.mapper.read_prg(0xBFFF),
        "$FFFF should mirror $BFFF with 1 PRG bank"
    );
}

#[test]
fn mapper0_prg_read_32k() {
    // 2 PRG banks (32KB): no mirroring
    let mut rom = make_rom(2, 1, 0, 0);
    rom[16] = 0xAA; // Bank 0 marker
    rom[16 + 16384] = 0xBB; // Bank 1 marker
    let cart = Cartridge::new(&rom).unwrap();
    assert_eq!(cart.mapper.read_prg(0x8000), 0xAA, "Bank 0 at $8000");
    assert_eq!(cart.mapper.read_prg(0xC000), 0xBB, "Bank 1 at $C000");
    assert_ne!(
        cart.mapper.read_prg(0x8000),
        cart.mapper.read_prg(0xC000),
        "$8000 and $C000 should differ with 32KB PRG"
    );
}

#[test]
fn mapper0_chr_read() {
    let rom = make_rom(1, 1, 0, 0);
    let cart = Cartridge::new(&rom).unwrap();
    // CHR data pattern: ((offset >> 2) & 0xFF)
    assert_eq!(cart.mapper.read_chr(0x0000), 0); // (0>>2) = 0
    assert_eq!(cart.mapper.read_chr(0x0004), 1); // (4>>2) = 1
    assert_eq!(cart.mapper.read_chr(0x0008), 2); // (8>>2) = 2
    assert_eq!(cart.mapper.read_chr(0x0100), 64); // (256>>2) = 64
}

// -- Mapper 2 (UxROM) ------------------------------------------------------

#[test]
fn mapper2_bank_switch() {
    // 8 PRG banks (128KB)
    let mut rom = make_rom(8, 1, 2, 0);
    for bank in 0u8..8 {
        rom[16 + bank as usize * 16384] = 0xA0 + bank;
    }
    let mut cart = Cartridge::new(&rom).unwrap();
    // Default: bank 0 at $8000, last bank (7) at $C000
    assert_eq!(
        cart.mapper.read_prg(0x8000),
        0xA0,
        "Default bank 0 at $8000"
    );
    assert_eq!(
        cart.mapper.read_prg(0xC000),
        0xA7,
        "Last bank fixed at $C000"
    );
    // Switch to bank 3
    cart.mapper.write_prg(0x8000, 3);
    assert_eq!(
        cart.mapper.read_prg(0x8000),
        0xA3,
        "Bank 3 at $8000 after switch"
    );
    assert_eq!(
        cart.mapper.read_prg(0xC000),
        0xA7,
        "Last bank still at $C000"
    );
    // Switch to bank 5
    cart.mapper.write_prg(0x8000, 5);
    assert_eq!(cart.mapper.read_prg(0x8000), 0xA5, "Bank 5 at $8000");
}

// -- Mapper 3 (CNROM) ------------------------------------------------------

#[test]
fn mapper3_chr_switch() {
    // 1 PRG bank, 4 CHR banks
    let mut rom = make_rom(1, 4, 3, 0);
    let prg_size = 16384;
    for bank in 0u8..4 {
        rom[16 + prg_size + bank as usize * 8192] = 0xB0 + bank;
    }
    let mut cart = Cartridge::new(&rom).unwrap();
    assert_eq!(cart.mapper.read_chr(0x0000), 0xB0, "Default CHR bank 0");
    // Switch to CHR bank 2
    cart.mapper.write_prg(0x8000, 2);
    assert_eq!(
        cart.mapper.read_chr(0x0000),
        0xB2,
        "CHR bank 2 after switch"
    );
    // Switch to CHR bank 3
    cart.mapper.write_prg(0x8000, 3);
    assert_eq!(
        cart.mapper.read_chr(0x0000),
        0xB3,
        "CHR bank 3 after switch"
    );
}

// -- Mapper 1 (MMC1) -------------------------------------------------------

#[test]
fn mapper1_shift_register() {
    // 4 PRG banks (64KB)
    let mut rom = make_rom(4, 1, 1, 0);
    for bank in 0u8..4 {
        rom[16 + bank as usize * 16384] = 0xC0 + bank;
    }
    let mut cart = Cartridge::new(&rom).unwrap();
    // Reset MMC1
    cart.mapper.write_prg(0x8000, 0x80);
    // Set control to mode 3 (fix last bank at $C000, switch $8000)
    mmc1_write(&mut cart.mapper, 0x8000, 0x0C);
    // Set PRG bank to 0
    mmc1_write(&mut cart.mapper, 0xE000, 0);
    assert_eq!(cart.mapper.read_prg(0x8000), 0xC0, "PRG bank 0 at $8000");
    assert_eq!(
        cart.mapper.read_prg(0xC000),
        0xC3,
        "Last bank (3) fixed at $C000"
    );
    // Write first 4 of 5 bits for bank 2 (0b00010)
    for bit in 0..4u8 {
        cart.mapper.write_prg(0xE000, (2 >> bit) & 1);
    }
    assert_eq!(
        cart.mapper.read_prg(0x8000),
        0xC0,
        "Bank should still be 0 after only 4 serial writes"
    );
    // 5th serial bit for bank 2 (0b00010) is zero and completes the shift.
    cart.mapper.write_prg(0xE000, 0);
    assert_eq!(
        cart.mapper.read_prg(0x8000),
        0xC2,
        "Bank should switch to 2 after 5th serial write"
    );
}

// -- Mirroring --------------------------------------------------------------

#[test]
fn mapper0_mirroring() {
    let rom_h = make_rom(1, 1, 0, 0x00);
    let cart_h = Cartridge::new(&rom_h).unwrap();
    assert_eq!(cart_h.mapper.mirroring(), Mirroring::Horizontal);

    let rom_v = make_rom(1, 1, 0, 0x01);
    let cart_v = Cartridge::new(&rom_v).unwrap();
    assert_eq!(cart_v.mapper.mirroring(), Mirroring::Vertical);
}

// -- SRAM -------------------------------------------------------------------

#[test]
fn mapper_sram_get_set() {
    // Mapper 1 with battery flag
    let rom = make_rom(2, 1, 1, 0x02);
    let mut cart = Cartridge::new(&rom).unwrap();
    assert!(cart.has_battery);

    let sram = cart.mapper.get_sram();
    if sram.is_empty() {
        return; // mapper doesn't expose SRAM via this API
    }

    let mut new_sram = vec![0x00; sram.len()];
    new_sram[0] = 0x12;
    new_sram[1] = 0x34;
    new_sram[sram.len() - 1] = 0xFF;
    cart.mapper.set_sram(&new_sram);

    let retrieved = cart.mapper.get_sram();
    assert_eq!(retrieved[0], 0x12, "SRAM byte 0 should persist");
    assert_eq!(retrieved[1], 0x34, "SRAM byte 1 should persist");
    assert_eq!(
        retrieved[retrieved.len() - 1],
        0xFF,
        "SRAM last byte should persist"
    );
}

// -- Save / Load State ------------------------------------------------------

#[test]
fn mapper_save_load_state() {
    // Mapper 0 with CHR RAM so we can write and verify state
    let rom = make_rom(1, 0, 0, 0);
    let mut cart = Cartridge::new(&rom).unwrap();

    cart.mapper.write_chr(0x0000, 0xAB);
    assert_eq!(cart.mapper.read_chr(0x0000), 0xAB);

    let state = cart.mapper.save_state();

    cart.mapper.write_chr(0x0000, 0xCD);
    assert_eq!(cart.mapper.read_chr(0x0000), 0xCD);

    cart.mapper.load_state(&state);

    if !state.is_empty() {
        assert_eq!(
            cart.mapper.read_chr(0x0000),
            0xAB,
            "CHR RAM should be restored after load_state"
        );
    }
}

#[test]
fn mapper4_save_load_state_preserves_prg_ram() {
    let rom = make_rom(2, 1, 4, 0x02);
    let mut cart = Cartridge::new(&rom).unwrap();
    assert!(cart.has_battery);

    cart.mapper.write_prg(0x6000, 0x3C);
    cart.mapper.write_prg(0x67FF, 0xC3);
    cart.mapper.write_prg(0x7FFF, 0xA7);
    let state = cart.mapper.save_state();

    cart.mapper.write_prg(0x6000, 0x00);
    cart.mapper.write_prg(0x67FF, 0x00);
    cart.mapper.write_prg(0x7FFF, 0x00);
    assert_eq!(cart.mapper.read_prg(0x6000), 0x00);
    assert_eq!(cart.mapper.read_prg(0x67FF), 0x00);
    assert_eq!(cart.mapper.read_prg(0x7FFF), 0x00);

    cart.mapper.load_state(&state);

    assert_eq!(cart.mapper.read_prg(0x6000), 0x3C);
    assert_eq!(cart.mapper.read_prg(0x67FF), 0xC3);
    assert_eq!(cart.mapper.read_prg(0x7FFF), 0xA7);
}

// -- Mapper 4 (MMC3) IRQ ---------------------------------------------------

#[test]
fn mapper4_irq_countdown() {
    let rom = make_rom(2, 2, 4, 0);
    let mut cart = Cartridge::new(&rom).unwrap();

    cart.mapper.write_prg(0xC000, 3); // IRQ latch = 3
    cart.mapper.write_prg(0xC001, 0); // request counter reload
    cart.mapper.write_prg(0xE001, 0); // enable IRQ

    assert!(
        !cart.mapper.irq_pending(),
        "IRQ should not be pending before clocking"
    );

    let mut irq_at = None;
    for i in 1..=10 {
        cart.mapper.clock_scanline();
        if cart.mapper.irq_pending() {
            irq_at = Some(i);
            break;
        }
    }
    assert!(
        irq_at.is_some(),
        "IRQ should fire after clocking scanlines with latch=3"
    );

    cart.mapper.irq_clear();
    assert!(
        !cart.mapper.irq_pending(),
        "IRQ should clear after irq_clear()"
    );
}

#[test]
fn mapper7_switches_32k_prg_bank_and_single_screen_mirroring() {
    let rom = make_ines_rom(4, 0, 7, 0);
    let mut cart = Cartridge::new(&rom).unwrap();

    assert_eq!(cart.mapper.read_prg(0x8000), prg_bank_marker(0));
    assert_eq!(cart.mapper.mirroring(), Mirroring::SingleScreenLower);

    cart.mapper.write_prg(0x8000, 0x01);
    assert_eq!(
        cart.mapper.read_prg(0x8000),
        prg_bank_marker(2),
        "mapper 7 switches 32KB PRG banks"
    );
    assert_eq!(cart.mapper.mirroring(), Mirroring::SingleScreenLower);

    cart.mapper.write_prg(0x8000, 0x10);
    assert_eq!(cart.mapper.read_prg(0x8000), prg_bank_marker(0));
    assert_eq!(cart.mapper.mirroring(), Mirroring::SingleScreenUpper);
}
