use crate::mapper::{
    Mapper000, Mapper001, Mapper002, Mapper003, Mapper004, Mapper005, Mapper007, Mapper009,
    Mapper010, Mapper011, Mapper019, Mapper034, Mapper066, Mapper069, Mapper071, Mapper079,
    Mapper085, Mapper206, MapperEnum, MapperVRC6,
};
use crate::romdb::RomDatabase;

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
    pub mapper: MapperEnum,
    pub has_battery: bool,
    pub rom_title: Option<String>,
}

impl std::fmt::Debug for Cartridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cartridge")
            .field("has_battery", &self.has_battery)
            .field("rom_title", &self.rom_title)
            .finish()
    }
}

impl Cartridge {
    pub fn new(rom_data: &[u8]) -> Result<Self, String> {
        Self::new_with_romdb(rom_data, None)
    }

    pub fn new_with_romdb(rom_data: &[u8], romdb: Option<&RomDatabase>) -> Result<Self, String> {
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

        // Detect dirty headers (e.g., "DiskDude!" watermark in bytes 7-15)
        // In valid iNES 1.0, bytes 12-15 must be zero
        let has_dirty_header =
            !is_nes2 && rom_data.len() > 15 && rom_data[12..16].iter().any(|&b| b != 0);

        let mut mapper_id = if is_nes2 && rom_data.len() > 8 {
            // NES 2.0: extended mapper from flags8
            let flags8 = rom_data[8];
            ((flags8 as u16 & 0x0F) << 8) | (flags7 as u16 & 0xF0) | ((flags6 as u16) >> 4)
        } else if has_dirty_header {
            // Dirty header: only trust lower nibble from flags6
            (flags6 >> 4) as u16
        } else {
            ((flags7 & 0xF0) | (flags6 >> 4)) as u16
        };

        if has_dirty_header {
            eprintln!("Warning: ROM has dirty iNES header (bytes 12-15 non-zero), using mapper {} from flags6 only", mapper_id);
        }
        let mut mirroring = if flags6 & 0x08 != 0 {
            Mirroring::FourScreen
        } else if flags6 & 0x01 != 0 {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let mut has_battery = flags6 & 0x02 != 0;
        let has_trainer = flags6 & 0x04 != 0;
        let prg_start = 16 + if has_trainer { 512 } else { 0 };

        if rom_data.len() < prg_start {
            return Err("Invalid ROM: file too short for trainer data".to_string());
        }

        // Compute CRC32 of ROM data (excluding header) for database lookup
        let crc = crc32fast::hash(&rom_data[prg_start..]);
        let mut rom_title: Option<String> = None;

        if let Some(db) = romdb {
            if let Some(entry) = db.lookup(crc) {
                rom_title = Some(entry.title.clone());
                let db_mapper = entry.mapper;
                let db_mirroring = match entry.mirroring.as_str() {
                    "vertical" => Mirroring::Vertical,
                    "four_screen" => Mirroring::FourScreen,
                    _ => Mirroring::Horizontal,
                };
                if db_mapper != mapper_id || db_mirroring != mirroring {
                    eprintln!(
                        "ROM DB: \"{}\" — correcting mapper {}→{}, mirroring {:?}→{:?}",
                        entry.title, mapper_id, db_mapper, mirroring, db_mirroring
                    );
                    mapper_id = db_mapper;
                    mirroring = db_mirroring;
                }
                has_battery = entry.battery;
                eprintln!("ROM DB: Identified \"{}\" (CRC: {:08X})", entry.title, crc);
            }
        }
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

        let mapper: MapperEnum = match mapper_id {
            0 => MapperEnum::Mapper000(Mapper000::new(prg_rom, chr_rom, mirroring)),
            1 => MapperEnum::Mapper001(Mapper001::new(prg_rom, chr_rom, mirroring)),
            2 => MapperEnum::Mapper002(Mapper002::new(prg_rom, chr_rom, mirroring)),
            3 => MapperEnum::Mapper003(Mapper003::new(prg_rom, chr_rom, mirroring)),
            4 => MapperEnum::Mapper004(Mapper004::new(prg_rom, chr_rom, mirroring)),
            5 => MapperEnum::Mapper005(Mapper005::new(prg_rom, chr_rom, mirroring)),
            7 => MapperEnum::Mapper007(Mapper007::new(prg_rom, chr_rom, mirroring)),
            9 => MapperEnum::Mapper009(Mapper009::new(prg_rom, chr_rom, mirroring)),
            10 => MapperEnum::Mapper010(Mapper010::new(prg_rom, chr_rom, mirroring)),
            11 => MapperEnum::Mapper011(Mapper011::new(prg_rom, chr_rom, mirroring)),
            19 => MapperEnum::Mapper019(Mapper019::new(prg_rom, chr_rom, mirroring)),
            24 => MapperEnum::Mapper024(MapperVRC6::new(prg_rom, chr_rom, mirroring, false)),
            26 => MapperEnum::Mapper026(MapperVRC6::new(prg_rom, chr_rom, mirroring, true)),
            34 => MapperEnum::Mapper034(Mapper034::new(prg_rom, chr_rom, mirroring)),
            66 => MapperEnum::Mapper066(Mapper066::new(prg_rom, chr_rom, mirroring)),
            69 => MapperEnum::Mapper069(Mapper069::new(prg_rom, chr_rom, mirroring)),
            71 => MapperEnum::Mapper071(Mapper071::new(prg_rom, chr_rom, mirroring)),
            79 => MapperEnum::Mapper079(Mapper079::new(prg_rom, chr_rom, mirroring)),
            85 => MapperEnum::Mapper085(Mapper085::new(prg_rom, chr_rom, mirroring)),
            206 => MapperEnum::Mapper206(Mapper206::new(prg_rom, chr_rom, mirroring)),
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
                return Err(format!(
                    "{}. Supported: 0,1,2,3,4,5,7,9,10,11,19,24,26,34,66,69,71,79,85,206",
                    hint
                ));
            }
        };

        Ok(Cartridge {
            mapper,
            has_battery,
            rom_title,
        })
    }
}
