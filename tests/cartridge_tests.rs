use oxidenes::cartridge::{Cartridge, Mirroring};
use oxidenes::mapper::MapperEnum;

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

#[test]
fn valid_ines_header() {
    let rom = make_rom(1, 1, 0, 0);
    assert!(
        Cartridge::new(&rom).is_ok(),
        "Valid iNES ROM should parse successfully"
    );
}

#[test]
fn invalid_magic_bytes() {
    let mut rom = make_rom(1, 1, 0, 0);
    rom[0] = 0x00;
    let result = Cartridge::new(&rom);
    assert!(result.is_err(), "Invalid magic should fail");
    assert!(
        result.unwrap_err().contains("iNES"),
        "Error should mention iNES"
    );
}

#[test]
fn too_short_data() {
    let rom = vec![0x4E, 0x45, 0x53, 0x1A]; // only 4 bytes
    assert!(Cartridge::new(&rom).is_err(), "Too-short ROM should fail");
}

#[test]
fn horizontal_mirroring() {
    let rom = make_rom(1, 1, 0, 0x00); // bit 0 = 0 -> horizontal
    let cart = Cartridge::new(&rom).unwrap();
    assert_eq!(cart.mapper.mirroring(), Mirroring::Horizontal);
}

#[test]
fn vertical_mirroring() {
    let rom = make_rom(1, 1, 0, 0x01); // bit 0 = 1 -> vertical
    let cart = Cartridge::new(&rom).unwrap();
    assert_eq!(cart.mapper.mirroring(), Mirroring::Vertical);
}

#[test]
fn battery_flag() {
    let rom = make_rom(1, 1, 0, 0x02); // bit 1 = battery
    let cart = Cartridge::new(&rom).unwrap();
    assert!(
        cart.has_battery,
        "has_battery should be true when flag is set"
    );
}

#[test]
fn no_battery_flag() {
    let rom = make_rom(1, 1, 0, 0x00);
    let cart = Cartridge::new(&rom).unwrap();
    assert!(
        !cart.has_battery,
        "has_battery should be false when flag is clear"
    );
}

#[test]
fn chr_ram_auto_creation() {
    let rom = make_rom(1, 0, 0, 0); // chr_banks = 0
    let cart = Cartridge::new(&rom);
    assert!(
        cart.is_ok(),
        "chr_banks=0 should succeed (CHR RAM auto-created)"
    );
    let mut cart = cart.unwrap();
    cart.mapper.write_chr(0x0000, 0xAB);
    assert_eq!(
        cart.mapper.read_chr(0x0000),
        0xAB,
        "CHR RAM should be writable"
    );
}

#[test]
fn mapper_0_creation() {
    let rom = make_rom(1, 1, 0, 0);
    let cart = Cartridge::new(&rom).unwrap();
    assert!(
        matches!(cart.mapper, MapperEnum::Mapper000(_)),
        "Expected Mapper000 variant"
    );
}

#[test]
fn mapper_1_creation() {
    let rom = make_rom(2, 1, 1, 0);
    let cart = Cartridge::new(&rom).unwrap();
    assert!(
        matches!(cart.mapper, MapperEnum::Mapper001(_)),
        "Expected Mapper001 variant"
    );
}

#[test]
fn mapper_2_creation() {
    let rom = make_rom(2, 1, 2, 0);
    let cart = Cartridge::new(&rom).unwrap();
    assert!(
        matches!(cart.mapper, MapperEnum::Mapper002(_)),
        "Expected Mapper002 variant"
    );
}

#[test]
fn mapper_4_creation() {
    let rom = make_rom(2, 2, 4, 0);
    let cart = Cartridge::new(&rom).unwrap();
    assert!(
        matches!(cart.mapper, MapperEnum::Mapper004(_)),
        "Expected Mapper004 variant"
    );
}

#[test]
fn unsupported_mapper_error() {
    let rom = make_rom(1, 1, 255, 0);
    let result = Cartridge::new(&rom);
    assert!(result.is_err(), "Unsupported mapper should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("Unsupported mapper") && err.contains("255"),
        "Error should mention unsupported mapper 255, got: {err}"
    );
}
