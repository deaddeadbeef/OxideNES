use crate::mapper::{Mapper, Mapper000, Mapper001, Mapper002, Mapper004};

const INES_MAGIC: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A]; // "NES\x1a"

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
    SingleScreenLower,
    SingleScreenUpper,
}

pub struct Cartridge {
    pub mapper: Box<dyn Mapper>,
}

impl Cartridge {
    pub fn new(rom_data: &[u8]) -> Result<Self, String> {
        if rom_data.len() < 16 {
            return Err("ROM too small".to_string());
        }
        if rom_data[0..4] != INES_MAGIC {
            return Err("Not a valid iNES file".to_string());
        }

        let prg_rom_size = rom_data[4] as usize * 16384;
        let chr_rom_size = rom_data[5] as usize * 8192;
        let flags6 = rom_data[6];
        let flags7 = rom_data[7];

        let mapper_id = (flags7 & 0xF0) | (flags6 >> 4);
        let mirroring = if flags6 & 0x08 != 0 {
            Mirroring::FourScreen
        } else if flags6 & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let has_trainer = flags6 & 0x04 != 0;
        let prg_start = 16 + if has_trainer { 512 } else { 0 };
        let chr_start = prg_start + prg_rom_size;

        if rom_data.len() < chr_start + chr_rom_size {
            return Err("ROM file truncated".to_string());
        }

        let prg_rom = rom_data[prg_start..prg_start + prg_rom_size].to_vec();
        let chr_rom = if chr_rom_size > 0 {
            rom_data[chr_start..chr_start + chr_rom_size].to_vec()
        } else {
            Vec::new()
        };

        let mapper: Box<dyn Mapper> = match mapper_id {
            0 => Box::new(Mapper000::new(prg_rom, chr_rom, mirroring)),
            1 => Box::new(Mapper001::new(prg_rom, chr_rom, mirroring)),
            2 => Box::new(Mapper002::new(prg_rom, chr_rom, mirroring)),
            4 => Box::new(Mapper004::new(prg_rom, chr_rom, mirroring)),
            _ => return Err(format!("Unsupported mapper: {}. Supported: 0 (NROM), 1 (MMC1), 2 (UxROM), 4 (MMC3)", mapper_id)),
        };

        Ok(Cartridge { mapper })
    }
}
