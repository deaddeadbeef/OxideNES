pub trait Mapper {
    fn read_prg(&self, addr: u16) -> u8;
    fn write_prg(&mut self, addr: u16, data: u8);
    fn read_chr(&self, addr: u16) -> u8;
    fn write_chr(&mut self, addr: u16, data: u8);
    fn mirroring(&self) -> crate::cartridge::Mirroring;
}

pub struct Mapper000 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    prg_banks: usize,
}

impl Mapper000 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x4000;
        let has_chr_ram = chr_rom.is_empty();
        Mapper000 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            chr_ram: Vec::new(),
            prg_ram: vec![0; 0x2000],
            mirroring,
            prg_banks,
        }
    }
}

impl Mapper for Mapper000 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xFFFF => {
                let mut mapped_addr = (addr - 0x8000) as usize;
                if self.prg_banks == 1 {
                    mapped_addr &= 0x3FFF; // mirror 16KB
                }
                self.prg_rom[mapped_addr]
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            _ => {} // ROM, ignore writes
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_rom[addr as usize]
    }

    fn write_chr(&mut self, addr: u16, data: u8) {
        // CHR RAM (when no CHR ROM on cart)
        if self.chr_rom.len() == 0x2000 {
            self.chr_rom[addr as usize] = data;
        }
    }

    fn mirroring(&self) -> crate::cartridge::Mirroring {
        self.mirroring
    }
}
