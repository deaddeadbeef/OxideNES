pub trait Mapper {
    fn read_prg(&self, addr: u16) -> u8;
    fn write_prg(&mut self, addr: u16, data: u8);
    fn read_chr(&self, addr: u16) -> u8;
    fn write_chr(&mut self, addr: u16, data: u8);
    fn mirroring(&self) -> crate::cartridge::Mirroring;
    fn clock_scanline(&mut self) {}  // NEW - default no-op
    fn irq_pending(&self) -> bool { false }  // NEW - default false
    fn irq_clear(&mut self) {}  // NEW - default no-op
    
    // Save state support - SRAM/PRG RAM access
    fn get_sram(&self) -> Vec<u8> { Vec::new() }
    fn set_sram(&mut self, _data: &[u8]) {}
    
    // Save state support - mapper state
    fn save_state(&self) -> Vec<u8> { Vec::new() }
    fn load_state(&mut self, _data: &[u8]) {}
}

pub struct Mapper000 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
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
    
    fn get_sram(&self) -> Vec<u8> {
        self.prg_ram.clone()
    }
    
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
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

    fn clock_scanline(&mut self) {
        if self.irq_counter == 0 || self.irq_reload_flag {
            self.irq_counter = self.irq_reload;
            self.irq_reload_flag = false;
        } else {
            self.irq_counter -= 1;
        }
        
        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }
    }

    fn irq_pending(&self) -> bool {
        self.irq_pending
    }

    fn irq_clear(&mut self) {
        self.irq_pending = false;
    }
    
    fn get_sram(&self) -> Vec<u8> {
        self.prg_ram.clone()
    }
    
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
    }
    
    fn save_state(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.bank_select);
        data.push(if self.prg_bank_mode { 1 } else { 0 });
        data.push(if self.chr_inversion { 1 } else { 0 });
        data.extend_from_slice(&self.registers);
        data.push(self.irq_counter);
        data.push(self.irq_reload);
        data.push(if self.irq_enabled { 1 } else { 0 });
        data.push(if self.irq_pending { 1 } else { 0 });
        data.push(if self.irq_reload_flag { 1 } else { 0 });
        data.push(self.mirror_mode);
        data
    }
    
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 16 {
            self.bank_select = data[0];
            self.prg_bank_mode = data[1] != 0;
            self.chr_inversion = data[2] != 0;
            self.registers.copy_from_slice(&data[3..11]);
            self.irq_counter = data[11];
            self.irq_reload = data[12];
            self.irq_enabled = data[13] != 0;
            self.irq_pending = data[14] != 0;
            self.irq_reload_flag = data[15] != 0;
            if data.len() >= 17 {
                self.mirror_mode = data[16];
            }
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
    
    fn get_sram(&self) -> Vec<u8> {
        self.prg_ram.clone()
    }
    
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
    }
    
    fn save_state(&self) -> Vec<u8> {
        vec![self.bank_select]
    }
    
    fn load_state(&mut self, data: &[u8]) {
        if !data.is_empty() {
            self.bank_select = data[0];
        }
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
    
    fn get_sram(&self) -> Vec<u8> {
        self.prg_ram.clone()
    }
    
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
    }
    
    fn save_state(&self) -> Vec<u8> {
        vec![
            self.shift_register,
            self.write_count,
            self.control,
            self.chr_bank_0,
            self.chr_bank_1,
            self.prg_bank,
        ]
    }
    
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 6 {
            self.shift_register = data[0];
            self.write_count = data[1];
            self.control = data[2];
            self.chr_bank_0 = data[3];
            self.chr_bank_1 = data[4];
            self.prg_bank = data[5];
        }
    }
}

pub struct Mapper003 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    chr_bank: u8,
    prg_banks: usize,
}

impl Mapper003 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x4000;
        Mapper003 {
            prg_rom,
            chr_rom,
            mirroring,
            chr_bank: 0,
            prg_banks,
        }
    }
}

impl Mapper for Mapper003 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xBFFF => {
                self.prg_rom[(addr - 0x8000) as usize % self.prg_rom.len()]
            }
            0xC000..=0xFFFF => {
                if self.prg_banks > 1 {
                    self.prg_rom[(addr - 0x8000) as usize]
                } else {
                    // Mirror 16KB
                    self.prg_rom[(addr - 0xC000) as usize]
                }
            }
            _ => 0,
        }
    }
    
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.chr_bank = data & 0x03; // Usually 2 bits, supporting up to 4 banks
        }
    }
    
    fn read_chr(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() {
            return 0;
        }
        let chr_banks = self.chr_rom.len() / 0x2000;
        let bank = (self.chr_bank as usize) % chr_banks;
        let offset = bank * 0x2000 + (addr as usize);
        if offset < self.chr_rom.len() {
            self.chr_rom[offset]
        } else {
            0
        }
    }
    
    fn write_chr(&mut self, _addr: u16, _data: u8) {
        // CHR ROM is read-only for CNROM
    }
    
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        self.mirroring
    }
    
    fn save_state(&self) -> Vec<u8> {
        vec![self.chr_bank]
    }
    
    fn load_state(&mut self, data: &[u8]) {
        if !data.is_empty() {
            self.chr_bank = data[0];
        }
    }
}

pub struct Mapper007 {
    prg_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    prg_bank: u8,
    mirroring_bit: bool, // false = lower, true = upper
}

impl Mapper007 {
    pub fn new(prg_rom: Vec<u8>, _chr_rom: Vec<u8>, _mirroring: crate::cartridge::Mirroring) -> Self {
        Mapper007 {
            prg_rom,
            chr_ram: vec![0; 0x2000],
            prg_bank: 0,
            mirroring_bit: false,
        }
    }
}

impl Mapper for Mapper007 {
    fn read_prg(&self, addr: u16) -> u8 {
        if addr >= 0x8000 {
            let bank = self.prg_bank as usize;
            let prg_banks = self.prg_rom.len() / 0x8000; // 32KB banks
            let bank = bank % prg_banks;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            if offset < self.prg_rom.len() {
                self.prg_rom[offset]
            } else {
                0
            }
        } else {
            0
        }
    }
    
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.prg_bank = data & 0x07; // bits 0-2: PRG bank
            self.mirroring_bit = data & 0x10 != 0; // bit 4: mirroring
        }
    }
    
    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_ram[addr as usize & 0x1FFF]
    }
    
    fn write_chr(&mut self, addr: u16, data: u8) {
        self.chr_ram[addr as usize & 0x1FFF] = data;
    }
    
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        if self.mirroring_bit {
            crate::cartridge::Mirroring::SingleScreenUpper
        } else {
            crate::cartridge::Mirroring::SingleScreenLower
        }
    }
    
    fn save_state(&self) -> Vec<u8> {
        vec![self.prg_bank, if self.mirroring_bit { 1 } else { 0 }]
    }
    
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 2 {
            self.prg_bank = data[0];
            self.mirroring_bit = data[1] != 0;
        }
    }
}

// === Mapper 9: MMC2 / PxROM (Mike Tyson's Punch-Out!!) ===
// CHR bank switching triggered by PPU reads of specific tiles (latch-based)

pub struct Mapper009 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    prg_banks: usize,
    
    prg_bank: u8,
    
    // CHR latches: two pairs of banks, switched by reading $FD/$FE tiles
    chr_bank_0_fd: u8,
    chr_bank_0_fe: u8,
    chr_bank_1_fd: u8,
    chr_bank_1_fe: u8,
    latch_0: std::cell::Cell<bool>, // false = $FD, true = $FE
    latch_1: std::cell::Cell<bool>,
    
    mirror_mode: u8,
}

impl Mapper009 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x2000;
        Mapper009 {
            prg_rom, chr_rom,
            prg_ram: vec![0; 0x2000],
            mirroring, prg_banks,
            prg_bank: 0,
            chr_bank_0_fd: 0, chr_bank_0_fe: 0,
            chr_bank_1_fd: 0, chr_bank_1_fe: 0,
            latch_0: std::cell::Cell::new(true), latch_1: std::cell::Cell::new(true),
            mirror_mode: 0,
        }
    }
}

impl Mapper for Mapper009 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0x9FFF => {
                let bank = self.prg_bank as usize % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0x8000) as usize]
            }
            0xA000..=0xBFFF => {
                let bank = (self.prg_banks - 3) % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0xA000) as usize]
            }
            0xC000..=0xDFFF => {
                let bank = (self.prg_banks - 2) % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0xC000) as usize]
            }
            0xE000..=0xFFFF => {
                let bank = self.prg_banks - 1;
                self.prg_rom[bank * 0x2000 + (addr - 0xE000) as usize]
            }
            _ => 0,
        }
    }
    
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            0xA000..=0xAFFF => self.prg_bank = data & 0x0F,
            0xB000..=0xBFFF => self.chr_bank_0_fd = data & 0x1F,
            0xC000..=0xCFFF => self.chr_bank_0_fe = data & 0x1F,
            0xD000..=0xDFFF => self.chr_bank_1_fd = data & 0x1F,
            0xE000..=0xEFFF => self.chr_bank_1_fe = data & 0x1F,
            0xF000..=0xFFFF => self.mirror_mode = data & 0x01,
            _ => {}
        }
    }
    
    fn read_chr(&self, addr: u16) -> u8 {
        let bank = match addr {
            0x0000..=0x0FFF => {
                if self.latch_0.get() { self.chr_bank_0_fe } else { self.chr_bank_0_fd }
            }
            0x1000..=0x1FFF => {
                if self.latch_1.get() { self.chr_bank_1_fe } else { self.chr_bank_1_fd }
            }
            _ => 0,
        } as usize;
        
        let chr_banks = self.chr_rom.len() / 0x1000;
        if chr_banks == 0 { return 0; }
        let bank = bank % chr_banks;
        let offset = bank * 0x1000 + (addr & 0x0FFF) as usize;
        let result = if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 };
        
        // Update latches based on tile address fetched
        match addr {
            0x0FD8..=0x0FDF => self.latch_0.set(false),
            0x0FE8..=0x0FEF => self.latch_0.set(true),
            0x1FD8..=0x1FDF => self.latch_1.set(false),
            0x1FE8..=0x1FEF => self.latch_1.set(true),
            _ => {}
        }
        
        result
    }
    
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        if self.mirror_mode == 0 {
            crate::cartridge::Mirroring::Vertical
        } else {
            crate::cartridge::Mirroring::Horizontal
        }
    }
    
    fn get_sram(&self) -> Vec<u8> { self.prg_ram.clone() }
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
    }
    
    fn save_state(&self) -> Vec<u8> {
        vec![
            self.prg_bank, self.chr_bank_0_fd, self.chr_bank_0_fe,
            self.chr_bank_1_fd, self.chr_bank_1_fe,
            if self.latch_0.get() { 1 } else { 0 }, if self.latch_1.get() { 1 } else { 0 },
            self.mirror_mode,
        ]
    }
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 8 {
            self.prg_bank = data[0];
            self.chr_bank_0_fd = data[1]; self.chr_bank_0_fe = data[2];
            self.chr_bank_1_fd = data[3]; self.chr_bank_1_fe = data[4];
            self.latch_0.set(data[5] != 0); self.latch_1.set(data[6] != 0);
            self.mirror_mode = data[7];
        }
    }
}

// === Mapper 11: Color Dreams ===
// Simple mapper: 32KB PRG + 8KB CHR bank switching

pub struct Mapper011 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_bank: u8,
    chr_bank: u8,
    mirroring: crate::cartridge::Mirroring,
}

impl Mapper011 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        Mapper011 { prg_rom, chr_rom, prg_bank: 0, chr_bank: 0, mirroring }
    }
}

impl Mapper for Mapper011 {
    fn read_prg(&self, addr: u16) -> u8 {
        if addr >= 0x8000 {
            let prg_banks = self.prg_rom.len() / 0x8000;
            if prg_banks == 0 { return 0; }
            let bank = (self.prg_bank as usize) % prg_banks;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
        } else { 0 }
    }
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.prg_bank = (data >> 4) & 0x03;
            self.chr_bank = data & 0x0F;
        }
    }
    fn read_chr(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() { return 0; }
        let chr_banks = self.chr_rom.len() / 0x2000;
        if chr_banks == 0 { return 0; }
        let bank = (self.chr_bank as usize) % chr_banks;
        let offset = bank * 0x2000 + (addr as usize & 0x1FFF);
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    fn mirroring(&self) -> crate::cartridge::Mirroring { self.mirroring }
    fn save_state(&self) -> Vec<u8> { vec![self.prg_bank, self.chr_bank] }
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 2 { self.prg_bank = data[0]; self.chr_bank = data[1]; }
    }
}

// === Mapper 66: GxROM (SMB/Duck Hunt combo) ===
// 32KB PRG + 8KB CHR bank switching via single register

pub struct Mapper066 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_bank: u8,
    chr_bank: u8,
    mirroring: crate::cartridge::Mirroring,
}

impl Mapper066 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        Mapper066 { prg_rom, chr_rom, prg_bank: 0, chr_bank: 0, mirroring }
    }
}

impl Mapper for Mapper066 {
    fn read_prg(&self, addr: u16) -> u8 {
        if addr >= 0x8000 {
            let prg_banks = self.prg_rom.len() / 0x8000;
            if prg_banks == 0 { return 0; }
            let bank = (self.prg_bank as usize) % prg_banks;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
        } else { 0 }
    }
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.chr_bank = data & 0x03;
            self.prg_bank = (data >> 4) & 0x03;
        }
    }
    fn read_chr(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() { return 0; }
        let chr_banks = self.chr_rom.len() / 0x2000;
        if chr_banks == 0 { return 0; }
        let bank = (self.chr_bank as usize) % chr_banks;
        let offset = bank * 0x2000 + (addr as usize & 0x1FFF);
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    fn mirroring(&self) -> crate::cartridge::Mirroring { self.mirroring }
    fn save_state(&self) -> Vec<u8> { vec![self.prg_bank, self.chr_bank] }
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 2 { self.prg_bank = data[0]; self.chr_bank = data[1]; }
    }
}

// === Mapper 71: Camerica/Codemasters (Micro Machines, Fire Hawk) ===
// 16KB switchable + 16KB fixed last bank. Some variants support mirroring control.

pub struct Mapper071 {
    prg_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    prg_bank: u8,
    prg_banks: usize,
    mirroring: crate::cartridge::Mirroring,
    mirror_override: Option<bool>,
}

impl Mapper071 {
    pub fn new(prg_rom: Vec<u8>, _chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x4000;
        Mapper071 {
            prg_rom, chr_ram: vec![0; 0x2000],
            prg_bank: 0, prg_banks, mirroring,
            mirror_override: None,
        }
    }
}

impl Mapper for Mapper071 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xBFFF => {
                let bank = (self.prg_bank as usize) % self.prg_banks;
                self.prg_rom[bank * 0x4000 + (addr - 0x8000) as usize]
            }
            0xC000..=0xFFFF => {
                let bank = self.prg_banks - 1;
                self.prg_rom[bank * 0x4000 + (addr - 0xC000) as usize]
            }
            _ => 0,
        }
    }
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x9000..=0x9FFF => {
                // Mirroring control (Codemasters variant)
                self.mirror_override = Some(data & 0x10 != 0);
            }
            0xC000..=0xFFFF => {
                self.prg_bank = data & 0x0F;
            }
            _ => {}
        }
    }
    fn read_chr(&self, addr: u16) -> u8 { self.chr_ram[addr as usize & 0x1FFF] }
    fn write_chr(&mut self, addr: u16, data: u8) { self.chr_ram[addr as usize & 0x1FFF] = data; }
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        match self.mirror_override {
            Some(true) => crate::cartridge::Mirroring::SingleScreenUpper,
            Some(false) => crate::cartridge::Mirroring::SingleScreenLower,
            None => self.mirroring,
        }
    }
    fn save_state(&self) -> Vec<u8> {
        vec![self.prg_bank, match self.mirror_override { Some(true) => 2, Some(false) => 1, None => 0 }]
    }
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 2 {
            self.prg_bank = data[0];
            self.mirror_override = match data[1] { 2 => Some(true), 1 => Some(false), _ => None };
        }
    }
}

// === Mapper 69: FME-7 / Sunsoft-5B ===
// Used by Gimmick!, Batman: Return of the Joker (JP), etc.
// 8KB PRG banks, 1KB CHR banks, IRQ counter, optional audio expansion

pub struct Mapper069 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    prg_banks: usize,
    
    command: u8,
    chr_banks: [u8; 8],
    prg_bank_6: u8,
    prg_bank_8: u8,
    prg_bank_a: u8,
    prg_bank_c: u8,
    prg_ram_enabled: bool,
    prg_ram_select: bool,
    mirror_mode: u8,
    
    irq_enabled: bool,
    irq_counter_enabled: bool,
    irq_counter: u16,
    irq_pending_flag: bool,
}

impl Mapper069 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x2000;
        let has_chr_ram = chr_rom.is_empty();
        Mapper069 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x2000],
            mirroring, prg_banks,
            command: 0,
            chr_banks: [0; 8],
            prg_bank_6: 0, prg_bank_8: 0, prg_bank_a: 0, prg_bank_c: 0,
            prg_ram_enabled: false, prg_ram_select: false,
            mirror_mode: 0,
            irq_enabled: false, irq_counter_enabled: false,
            irq_counter: 0, irq_pending_flag: false,
        }
    }
}

impl Mapper for Mapper069 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_select {
                    self.prg_ram[(addr - 0x6000) as usize]
                } else {
                    let bank = (self.prg_bank_6 as usize) % self.prg_banks;
                    let offset = bank * 0x2000 + (addr - 0x6000) as usize;
                    if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                }
            }
            0x8000..=0x9FFF => {
                let bank = (self.prg_bank_8 as usize) % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0x8000) as usize]
            }
            0xA000..=0xBFFF => {
                let bank = (self.prg_bank_a as usize) % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0xA000) as usize]
            }
            0xC000..=0xDFFF => {
                let bank = (self.prg_bank_c as usize) % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0xC000) as usize]
            }
            0xE000..=0xFFFF => {
                let bank = (self.prg_banks - 1) % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0xE000) as usize]
            }
            _ => 0,
        }
    }
    
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_select && self.prg_ram_enabled {
                    self.prg_ram[(addr - 0x6000) as usize] = data;
                }
            }
            0x8000..=0x9FFF => self.command = data & 0x0F,
            0xA000..=0xBFFF => {
                match self.command {
                    0x00..=0x07 => self.chr_banks[self.command as usize] = data,
                    0x08 => {
                        self.prg_ram_select = data & 0x40 != 0;
                        self.prg_ram_enabled = data & 0x80 != 0;
                        self.prg_bank_6 = data & 0x3F;
                    }
                    0x09 => self.prg_bank_8 = data & 0x3F,
                    0x0A => self.prg_bank_a = data & 0x3F,
                    0x0B => self.prg_bank_c = data & 0x3F,
                    0x0C => self.mirror_mode = data & 0x03,
                    0x0D => {
                        self.irq_enabled = data & 0x01 != 0;
                        self.irq_counter_enabled = data & 0x80 != 0;
                        self.irq_pending_flag = false;
                    }
                    0x0E => {
                        self.irq_counter = (self.irq_counter & 0xFF00) | data as u16;
                    }
                    0x0F => {
                        self.irq_counter = (self.irq_counter & 0x00FF) | ((data as u16) << 8);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    
    fn read_chr(&self, addr: u16) -> u8 {
        let bank_idx = (addr / 0x0400) as usize;
        if bank_idx >= 8 { return 0; }
        let bank = self.chr_banks[bank_idx] as usize;
        let chr_banks = self.chr_rom.len() / 0x0400;
        if chr_banks == 0 { return 0; }
        let bank = bank % chr_banks;
        let offset = bank * 0x0400 + (addr & 0x03FF) as usize;
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }
    
    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_rom.len() <= 0x2000 {
            self.chr_rom[addr as usize & 0x1FFF] = data;
        }
    }
    
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        match self.mirror_mode {
            0 => crate::cartridge::Mirroring::Vertical,
            1 => crate::cartridge::Mirroring::Horizontal,
            2 => crate::cartridge::Mirroring::SingleScreenLower,
            3 => crate::cartridge::Mirroring::SingleScreenUpper,
            _ => self.mirroring,
        }
    }
    
    fn clock_scanline(&mut self) {
        if self.irq_counter_enabled {
            if self.irq_counter == 0 {
                if self.irq_enabled {
                    self.irq_pending_flag = true;
                }
            } else {
                self.irq_counter -= 1;
            }
        }
    }
    
    fn irq_pending(&self) -> bool { self.irq_pending_flag }
    fn irq_clear(&mut self) { self.irq_pending_flag = false; }
    
    fn get_sram(&self) -> Vec<u8> { self.prg_ram.clone() }
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
    }
    
    fn save_state(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(self.command);
        data.extend_from_slice(&self.chr_banks);
        data.push(self.prg_bank_6); data.push(self.prg_bank_8);
        data.push(self.prg_bank_a); data.push(self.prg_bank_c);
        data.push(if self.prg_ram_enabled { 1 } else { 0 });
        data.push(if self.prg_ram_select { 1 } else { 0 });
        data.push(self.mirror_mode);
        data.push(if self.irq_enabled { 1 } else { 0 });
        data.push(if self.irq_counter_enabled { 1 } else { 0 });
        data.extend_from_slice(&self.irq_counter.to_le_bytes());
        data.push(if self.irq_pending_flag { 1 } else { 0 });
        data
    }
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 20 {
            let mut pos = 0;
            self.command = data[pos]; pos += 1;
            self.chr_banks.copy_from_slice(&data[pos..pos+8]); pos += 8;
            self.prg_bank_6 = data[pos]; pos += 1;
            self.prg_bank_8 = data[pos]; pos += 1;
            self.prg_bank_a = data[pos]; pos += 1;
            self.prg_bank_c = data[pos]; pos += 1;
            self.prg_ram_enabled = data[pos] != 0; pos += 1;
            self.prg_ram_select = data[pos] != 0; pos += 1;
            self.mirror_mode = data[pos]; pos += 1;
            self.irq_enabled = data[pos] != 0; pos += 1;
            self.irq_counter_enabled = data[pos] != 0; pos += 1;
            self.irq_counter = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
            self.irq_pending_flag = data[pos] != 0;
        }
    }
}

pub struct Mapper010 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    prg_banks: usize,
    prg_bank: u8,
    chr_bank_0_fd: u8,
    chr_bank_0_fe: u8,
    chr_bank_1_fd: u8,
    chr_bank_1_fe: u8,
    latch_0: std::cell::Cell<bool>,
    latch_1: std::cell::Cell<bool>,
    mirror_mode: u8,
}

impl Mapper010 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x4000;
        Mapper010 {
            prg_rom, chr_rom, prg_ram: vec![0; 0x2000],
            mirroring, prg_banks, prg_bank: 0,
            chr_bank_0_fd: 0, chr_bank_0_fe: 0,
            chr_bank_1_fd: 0, chr_bank_1_fe: 0,
            latch_0: std::cell::Cell::new(true), latch_1: std::cell::Cell::new(true),
            mirror_mode: 0,
        }
    }
}

impl Mapper for Mapper010 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xBFFF => {
                let bank = (self.prg_bank as usize) % self.prg_banks;
                self.prg_rom[bank * 0x4000 + (addr - 0x8000) as usize]
            }
            0xC000..=0xFFFF => {
                let bank = self.prg_banks - 1;
                self.prg_rom[bank * 0x4000 + (addr - 0xC000) as usize]
            }
            _ => 0,
        }
    }
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            0xA000..=0xAFFF => self.prg_bank = data & 0x0F,
            0xB000..=0xBFFF => self.chr_bank_0_fd = data & 0x1F,
            0xC000..=0xCFFF => self.chr_bank_0_fe = data & 0x1F,
            0xD000..=0xDFFF => self.chr_bank_1_fd = data & 0x1F,
            0xE000..=0xEFFF => self.chr_bank_1_fe = data & 0x1F,
            0xF000..=0xFFFF => self.mirror_mode = data & 0x01,
            _ => {}
        }
    }
    fn read_chr(&self, addr: u16) -> u8 {
        let bank = match addr {
            0x0000..=0x0FFF => if self.latch_0.get() { self.chr_bank_0_fe } else { self.chr_bank_0_fd },
            0x1000..=0x1FFF => if self.latch_1.get() { self.chr_bank_1_fe } else { self.chr_bank_1_fd },
            _ => 0,
        } as usize;
        let chr_banks = self.chr_rom.len() / 0x1000;
        if chr_banks == 0 { return 0; }
        let bank = bank % chr_banks;
        let offset = bank * 0x1000 + (addr & 0x0FFF) as usize;
        let result = if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 };
        match addr {
            0x0FD8..=0x0FDF => self.latch_0.set(false),
            0x0FE8..=0x0FEF => self.latch_0.set(true),
            0x1FD8..=0x1FDF => self.latch_1.set(false),
            0x1FE8..=0x1FEF => self.latch_1.set(true),
            _ => {}
        }
        result
    }
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        if self.mirror_mode == 0 { crate::cartridge::Mirroring::Vertical } else { crate::cartridge::Mirroring::Horizontal }
    }
    fn get_sram(&self) -> Vec<u8> { self.prg_ram.clone() }
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
    }
    fn save_state(&self) -> Vec<u8> {
        vec![self.prg_bank, self.chr_bank_0_fd, self.chr_bank_0_fe, self.chr_bank_1_fd, self.chr_bank_1_fe,
             if self.latch_0.get() { 1 } else { 0 }, if self.latch_1.get() { 1 } else { 0 }, self.mirror_mode]
    }
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 8 {
            self.prg_bank = data[0]; self.chr_bank_0_fd = data[1]; self.chr_bank_0_fe = data[2];
            self.chr_bank_1_fd = data[3]; self.chr_bank_1_fe = data[4];
            self.latch_0.set(data[5] != 0); self.latch_1.set(data[6] != 0); self.mirror_mode = data[7];
        }
    }
}

pub struct Mapper079 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_bank: u8,
    chr_bank: u8,
    mirroring: crate::cartridge::Mirroring,
}

impl Mapper079 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        Mapper079 { prg_rom, chr_rom, prg_bank: 0, chr_bank: 0, mirroring }
    }
}

impl Mapper for Mapper079 {
    fn read_prg(&self, addr: u16) -> u8 {
        if addr >= 0x8000 {
            let prg_banks = self.prg_rom.len() / 0x8000;
            if prg_banks == 0 { return 0; }
            let bank = (self.prg_bank as usize) % prg_banks;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
        } else { 0 }
    }
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x4100 && addr <= 0x5FFF {
            self.chr_bank = data & 0x07;
            self.prg_bank = (data >> 3) & 0x01;
        }
    }
    fn read_chr(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() { return 0; }
        let chr_banks = self.chr_rom.len() / 0x2000;
        if chr_banks == 0 { return 0; }
        let bank = (self.chr_bank as usize) % chr_banks;
        let offset = bank * 0x2000 + (addr as usize & 0x1FFF);
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    fn mirroring(&self) -> crate::cartridge::Mirroring { self.mirroring }
    fn save_state(&self) -> Vec<u8> { vec![self.prg_bank, self.chr_bank] }
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 2 { self.prg_bank = data[0]; self.chr_bank = data[1]; }
    }
}

pub struct Mapper206 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    bank_select: u8,
    registers: [u8; 8],
    prg_banks: usize,
}

impl Mapper206 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x2000;
        let has_chr_ram = chr_rom.is_empty();
        Mapper206 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            mirroring, bank_select: 0, registers: [0; 8], prg_banks,
        }
    }
}

impl Mapper for Mapper206 {
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => {
                let bank = (self.registers[6] as usize) % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0x8000) as usize]
            }
            0xA000..=0xBFFF => {
                let bank = (self.registers[7] as usize) % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0xA000) as usize]
            }
            0xC000..=0xDFFF => {
                let bank = (self.prg_banks - 2) % self.prg_banks;
                self.prg_rom[bank * 0x2000 + (addr - 0xC000) as usize]
            }
            0xE000..=0xFFFF => {
                let bank = self.prg_banks - 1;
                self.prg_rom[bank * 0x2000 + (addr - 0xE000) as usize]
            }
            _ => 0,
        }
    }
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x8000..=0x9FFF => {
                if addr & 1 == 0 {
                    self.bank_select = data & 0x07;
                } else {
                    self.registers[self.bank_select as usize] = data & 0x3F;
                }
            }
            _ => {}
        }
    }
    fn read_chr(&self, addr: u16) -> u8 {
        let bank = match addr {
            0x0000..=0x03FF => (self.registers[0] & 0xFE) as usize,
            0x0400..=0x07FF => (self.registers[0] | 0x01) as usize,
            0x0800..=0x0BFF => (self.registers[1] & 0xFE) as usize,
            0x0C00..=0x0FFF => (self.registers[1] | 0x01) as usize,
            0x1000..=0x13FF => self.registers[2] as usize,
            0x1400..=0x17FF => self.registers[3] as usize,
            0x1800..=0x1BFF => self.registers[4] as usize,
            0x1C00..=0x1FFF => self.registers[5] as usize,
            _ => 0,
        };
        let chr_banks = self.chr_rom.len() / 0x0400;
        if chr_banks == 0 { return 0; }
        let bank = bank % chr_banks;
        let offset = bank * 0x0400 + (addr & 0x03FF) as usize;
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }
    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_rom.len() <= 0x2000 { self.chr_rom[addr as usize & 0x1FFF] = data; }
    }
    fn mirroring(&self) -> crate::cartridge::Mirroring { self.mirroring }
    fn save_state(&self) -> Vec<u8> {
        let mut data = vec![self.bank_select];
        data.extend_from_slice(&self.registers);
        data
    }
    fn load_state(&mut self, data: &[u8]) {
        if data.len() >= 9 {
            self.bank_select = data[0];
            self.registers.copy_from_slice(&data[1..9]);
        }
    }
}
