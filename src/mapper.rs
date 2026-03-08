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

pub struct Mapper004 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    
    // Bank select
    bank_select: u8,
    prg_bank_mode: bool,
    chr_inversion: bool,
    
    // Bank registers R0-R7
    registers: [u8; 8],
    
    // PRG banking
    prg_banks: usize,
    
    // IRQ
    irq_counter: u8,
    irq_reload: u8,
    irq_enabled: bool,
    irq_pending: bool,
    irq_reload_flag: bool,
    
    // Mirroring control
    mirror_mode: u8,
}

impl Mapper004 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x2000; // 8KB banks
        let has_chr_ram = chr_rom.is_empty();
        Mapper004 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x2000],
            mirroring,
            bank_select: 0,
            prg_bank_mode: false,
            chr_inversion: false,
            registers: [0; 8],
            prg_banks,
            irq_counter: 0,
            irq_reload: 0,
            irq_enabled: false,
            irq_pending: false,
            irq_reload_flag: false,
            mirror_mode: if mirroring == crate::cartridge::Mirroring::Vertical { 0 } else { 1 },
        }
    }
    
    fn prg_bank_offset(&self, bank: usize) -> usize {
        let bank = bank % self.prg_banks;
        bank * 0x2000
    }
    
    fn chr_bank_offset(&self, bank: usize) -> usize {
        let chr_banks = self.chr_rom.len() / 0x0400;
        if chr_banks == 0 { return 0; }
        let bank = bank % chr_banks;
        bank * 0x0400
    }
}

impl Mapper for Mapper004 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0x9FFF => {
                let bank = if self.prg_bank_mode {
                    self.prg_banks - 2 // second-to-last bank
                } else {
                    self.registers[6] as usize
                };
                let offset = self.prg_bank_offset(bank);
                self.prg_rom[offset + (addr - 0x8000) as usize]
            }
            0xA000..=0xBFFF => {
                let bank = self.registers[7] as usize;
                let offset = self.prg_bank_offset(bank);
                self.prg_rom[offset + (addr - 0xA000) as usize]
            }
            0xC000..=0xDFFF => {
                let bank = if self.prg_bank_mode {
                    self.registers[6] as usize
                } else {
                    self.prg_banks - 2
                };
                let offset = self.prg_bank_offset(bank);
                self.prg_rom[offset + (addr - 0xC000) as usize]
            }
            0xE000..=0xFFFF => {
                let bank = self.prg_banks - 1; // last bank always
                let offset = self.prg_bank_offset(bank);
                self.prg_rom[offset + (addr - 0xE000) as usize]
            }
            _ => 0,
        }
    }
    
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            0x8000..=0x9FFF => {
                if addr & 1 == 0 {
                    // Bank select ($8000)
                    self.bank_select = data & 0x07;
                    self.prg_bank_mode = data & 0x40 != 0;
                    self.chr_inversion = data & 0x80 != 0;
                } else {
                    // Bank data ($8001)
                    self.registers[self.bank_select as usize] = data;
                }
            }
            0xA000..=0xBFFF => {
                if addr & 1 == 0 {
                    // Mirroring ($A000)
                    self.mirror_mode = data & 1;
                } else {
                    // PRG RAM protect ($A001) - ignored for now
                }
            }
            0xC000..=0xDFFF => {
                if addr & 1 == 0 {
                    // IRQ latch ($C000)
                    self.irq_reload = data;
                } else {
                    // IRQ reload ($C001)
                    self.irq_counter = 0;
                    self.irq_reload_flag = true;
                }
            }
            0xE000..=0xFFFF => {
                if addr & 1 == 0 {
                    // IRQ disable ($E000)
                    self.irq_enabled = false;
                    self.irq_pending = false;
                } else {
                    // IRQ enable ($E001)
                    self.irq_enabled = true;
                }
            }
            _ => {}
        }
    }
    
    fn read_chr(&self, addr: u16) -> u8 {
        let bank = if self.chr_inversion {
            match addr {
                0x0000..=0x03FF => self.registers[2] as usize,
                0x0400..=0x07FF => self.registers[3] as usize,
                0x0800..=0x0BFF => self.registers[4] as usize,
                0x0C00..=0x0FFF => self.registers[5] as usize,
                0x1000..=0x13FF => (self.registers[0] & 0xFE) as usize,
                0x1400..=0x17FF => (self.registers[0] | 0x01) as usize,
                0x1800..=0x1BFF => (self.registers[1] & 0xFE) as usize,
                0x1C00..=0x1FFF => (self.registers[1] | 0x01) as usize,
                _ => 0,
            }
        } else {
            match addr {
                0x0000..=0x03FF => (self.registers[0] & 0xFE) as usize,
                0x0400..=0x07FF => (self.registers[0] | 0x01) as usize,
                0x0800..=0x0BFF => (self.registers[1] & 0xFE) as usize,
                0x0C00..=0x0FFF => (self.registers[1] | 0x01) as usize,
                0x1000..=0x13FF => self.registers[2] as usize,
                0x1400..=0x17FF => self.registers[3] as usize,
                0x1800..=0x1BFF => self.registers[4] as usize,
                0x1C00..=0x1FFF => self.registers[5] as usize,
                _ => 0,
            }
        };
        let offset = self.chr_bank_offset(bank);
        let local_addr = (addr & 0x03FF) as usize;
        if offset + local_addr < self.chr_rom.len() {
            self.chr_rom[offset + local_addr]
        } else {
            0
        }
    }
    
    fn write_chr(&mut self, addr: u16, data: u8) {
        // Only write if CHR RAM
        if addr < 0x2000 && self.chr_rom.len() <= 0x2000 {
            self.chr_rom[addr as usize] = data;
        }
    }
    
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        if self.mirror_mode == 0 {
            crate::cartridge::Mirroring::Vertical
        } else {
            crate::cartridge::Mirroring::Horizontal
        }
    }
}

pub struct Mapper002 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    prg_banks: usize,
    bank_select: u8,
}

impl Mapper002 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x4000;
        let has_chr_ram = chr_rom.is_empty();
        Mapper002 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x2000],
            mirroring,
            prg_banks,
            bank_select: 0,
        }
    }
}

impl Mapper for Mapper002 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xBFFF => {
                let offset = self.bank_select as usize * 0x4000;
                self.prg_rom[offset + (addr - 0x8000) as usize]
            }
            0xC000..=0xFFFF => {
                // Last bank is fixed
                let offset = (self.prg_banks - 1) * 0x4000;
                self.prg_rom[offset + (addr - 0xC000) as usize]
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            0x8000..=0xFFFF => {
                self.bank_select = data & 0x0F;
            }
            _ => {}
        }
    }

    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_rom[addr as usize]
    }

    fn write_chr(&mut self, addr: u16, data: u8) {
        self.chr_rom[addr as usize] = data;
    }

    fn mirroring(&self) -> crate::cartridge::Mirroring {
        self.mirroring
    }
}

pub struct Mapper001 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    
    shift_register: u8,
    write_count: u8,
    
    control: u8,
    chr_bank_0: u8,
    chr_bank_1: u8,
    prg_bank: u8,
    
    prg_banks: usize,
    chr_banks: usize,
}

impl Mapper001 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x4000;
        let chr_size = chr_rom.len();
        let has_chr_ram = chr_rom.is_empty();
        let chr_banks = if has_chr_ram { 0 } else { chr_size / 0x1000 };
        Mapper001 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x2000],
            mirroring,
            shift_register: 0x10,
            write_count: 0,
            control: 0x0C, // PRG fixed last bank mode
            chr_bank_0: 0,
            chr_bank_1: 0,
            prg_bank: 0,
            prg_banks,
            chr_banks,
        }
    }
    
    fn load_register(&mut self, addr: u16, data: u8) {
        if data & 0x80 != 0 {
            self.shift_register = 0x10;
            self.write_count = 0;
            self.control |= 0x0C;
            return;
        }
        
        self.shift_register >>= 1;
        self.shift_register |= (data & 1) << 4;
        self.write_count += 1;
        
        if self.write_count == 5 {
            let value = self.shift_register;
            match addr {
                0x8000..=0x9FFF => self.control = value,
                0xA000..=0xBFFF => self.chr_bank_0 = value,
                0xC000..=0xDFFF => self.chr_bank_1 = value,
                0xE000..=0xFFFF => self.prg_bank = value & 0x0F,
                _ => {}
            }
            self.shift_register = 0x10;
            self.write_count = 0;
        }
    }
}

impl Mapper for Mapper001 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xBFFF => {
                let prg_mode = (self.control >> 2) & 3;
                let bank = match prg_mode {
                    0 | 1 => (self.prg_bank & 0xFE) as usize, // 32KB mode
                    2 => 0, // fixed first bank
                    3 => self.prg_bank as usize, // switchable
                    _ => 0,
                };
                let bank = bank % self.prg_banks;
                let offset = if prg_mode <= 1 {
                    // 32KB mode: addr maps across 32KB
                    bank * 0x4000 + (addr - 0x8000) as usize
                } else {
                    bank * 0x4000 + (addr - 0x8000) as usize
                };
                if offset < self.prg_rom.len() {
                    self.prg_rom[offset]
                } else {
                    0
                }
            }
            0xC000..=0xFFFF => {
                let prg_mode = (self.control >> 2) & 3;
                let bank = match prg_mode {
                    0 | 1 => ((self.prg_bank & 0xFE) as usize) + 1, // 32KB mode, second half
                    2 => self.prg_bank as usize, // switchable
                    3 => self.prg_banks - 1, // fixed last bank
                    _ => self.prg_banks - 1,
                };
                let bank = bank % self.prg_banks;
                let offset = bank * 0x4000 + (addr - 0xC000) as usize;
                if offset < self.prg_rom.len() {
                    self.prg_rom[offset]
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
    
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            0x8000..=0xFFFF => self.load_register(addr, data),
            _ => {}
        }
    }
    
    fn read_chr(&self, addr: u16) -> u8 {
        let chr_mode = self.control & 0x10 != 0;
        if self.chr_banks == 0 {
            // CHR RAM
            return self.chr_rom[addr as usize];
        }
        
        let bank = if chr_mode {
            // 4KB mode
            match addr {
                0x0000..=0x0FFF => self.chr_bank_0 as usize,
                0x1000..=0x1FFF => self.chr_bank_1 as usize,
                _ => 0,
            }
        } else {
            // 8KB mode
            match addr {
                0x0000..=0x0FFF => (self.chr_bank_0 & 0xFE) as usize,
                0x1000..=0x1FFF => ((self.chr_bank_0 & 0xFE) + 1) as usize,
                _ => 0,
            }
        };
        
        let offset = (bank % self.chr_banks) * 0x1000 + (addr & 0x0FFF) as usize;
        if offset < self.chr_rom.len() {
            self.chr_rom[offset]
        } else {
            0
        }
    }
    
    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_banks == 0 {
            self.chr_rom[addr as usize] = data;
        }
    }
    
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        match self.control & 3 {
            0 => crate::cartridge::Mirroring::SingleScreenLower,
            1 => crate::cartridge::Mirroring::SingleScreenUpper,
            2 => crate::cartridge::Mirroring::Vertical,
            3 => crate::cartridge::Mirroring::Horizontal,
            _ => unreachable!(),
        }
    }
}
