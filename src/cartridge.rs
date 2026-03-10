use crate::mapper::{Mapper, Mapper000, Mapper001, Mapper002, Mapper003, Mapper004, Mapper007, Mapper009, Mapper010, Mapper011, Mapper066, Mapper069, Mapper071, Mapper079, Mapper206};

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
    pub has_battery: bool,
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

        let is_nes2 = (flags7 & 0x0C) == 0x08;
        let mapper_id = if is_nes2 && rom_data.len() > 8 {
            // NES 2.0: extended mapper from flags8
            let flags8 = rom_data[8];
            ((flags8 as u16 & 0x0F) << 8) | (flags7 as u16 & 0xF0) | ((flags6 as u16) >> 4)
        } else {
            ((flags7 & 0xF0) | (flags6 >> 4)) as u16
        };
        let mirroring = if flags6 & 0x08 != 0 {
            Mirroring::FourScreen
        } else if flags6 & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let has_battery = flags6 & 0x02 != 0;
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
            3 => Box::new(Mapper003::new(prg_rom, chr_rom, mirroring)),
            4 => Box::new(Mapper004::new(prg_rom, chr_rom, mirroring)),
            7 => Box::new(Mapper007::new(prg_rom, chr_rom, mirroring)),
            9 => Box::new(Mapper009::new(prg_rom, chr_rom, mirroring)),
            10 => Box::new(Mapper010::new(prg_rom, chr_rom, mirroring)),
            11 => Box::new(Mapper011::new(prg_rom, chr_rom, mirroring)),
            66 => Box::new(Mapper066::new(prg_rom, chr_rom, mirroring)),
            69 => Box::new(Mapper069::new(prg_rom, chr_rom, mirroring)),
            71 => Box::new(Mapper071::new(prg_rom, chr_rom, mirroring)),
            79 => Box::new(Mapper079::new(prg_rom, chr_rom, mirroring)),
            206 => Box::new(Mapper206::new(prg_rom, chr_rom, mirroring)),
            _ => {
                let popular = match mapper_id {
                    5 => "Castlevania III, Just Breed",
                    16 => "Dragon Ball Z (JP)",
                    18 => "Jaleco games",
                    19 => "Namco 163 games",
                    21 | 22 | 23 | 25 => "Konami VRC games",
                    24 | 26 => "Konami VRC6 (Castlevania III JP)",
                    34 => "Deadly Towers, Impossible Mission II",
                    48 => "Taito games",
                    64 | 158 => "Tengen games",
                    65 => "Irem games",
                    67 => "Sunsoft-3 games",
                    68 => "After Burner",
                    85 => "Konami VRC7 (Lagrange Point)",
                    _ => "",
                };
                let hint = if popular.is_empty() {
                    format!("Unsupported mapper: {}", mapper_id)
                } else {
                    format!("Unsupported mapper: {} (used by: {})", mapper_id, popular)
                };
                return Err(format!("{}. Supported: 0,1,2,3,4,7,9,10,11,66,69,71,79,206", hint));
            }
        };

        Ok(Cartridge { mapper, has_battery })
    }
}
