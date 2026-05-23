#[allow(dead_code)]
pub const FIXTURE_PROVENANCE: &str =
    "Generated synthetic iNES fixture: deterministic byte patterns only, no ROM content.";

pub fn prg_bank_marker(bank: u8) -> u8 {
    0x80 | (bank & 0x3F)
}

pub fn chr_bank_marker(bank: u8) -> u8 {
    0x40 | (bank & 0x3F)
}

pub fn make_ines_rom(prg_banks: u8, chr_banks: u8, mapper: u16, flags6_low: u8) -> Vec<u8> {
    assert!(
        mapper <= 0xFF,
        "synthetic iNES helper supports mapper IDs up to 255"
    );

    let prg_size = prg_banks as usize * 16384;
    let chr_size = chr_banks as usize * 8192;
    let mut rom = vec![0u8; 16 + prg_size + chr_size];

    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = prg_banks;
    rom[5] = chr_banks;
    rom[6] = (((mapper as u8) & 0x0F) << 4) | (flags6_low & 0x0F);
    rom[7] = (mapper as u8) & 0xF0;

    for bank in 0..prg_banks as usize {
        let start = 16 + bank * 16384;
        let end = start + 16384;
        rom[start..end].fill(prg_bank_marker(bank as u8));
    }

    for bank in 0..chr_banks as usize {
        let start = 16 + prg_size + bank * 8192;
        let end = start + 8192;
        rom[start..end].fill(chr_bank_marker(bank as u8));
    }

    rom
}
