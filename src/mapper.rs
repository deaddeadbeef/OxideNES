use std::cell::Cell;

pub trait Mapper {
    fn read_prg(&self, addr: u16) -> u8;
    fn write_prg(&mut self, addr: u16, data: u8);
    fn read_chr(&self, addr: u16) -> u8;
    fn write_chr(&mut self, addr: u16, data: u8);
    fn mirroring(&self) -> crate::cartridge::Mirroring;
    fn clock_scanline(&mut self) {}  // NEW - default no-op
    fn irq_pending(&self) -> bool { false }  // NEW - default false
    fn irq_clear(&mut self) {}  // NEW - default no-op
    
    // Audio expansion (VRC6, etc.) - returns mixed audio sample
    fn audio_output(&self) -> f32 { 0.0 }
    
    // Save state support - SRAM/PRG RAM access
    fn get_sram(&self) -> Vec<u8> { Vec::new() }
    fn set_sram(&mut self, _data: &[u8]) {}
    
    // Save state support - mapper state
    fn save_state(&self) -> Vec<u8> { Vec::new() }
    fn load_state(&mut self, _data: &[u8]) {}
}

// Enum dispatch wrapper for performance - eliminates vtable indirection
pub enum MapperEnum {
    Mapper000(Mapper000),
    Mapper001(Mapper001),
    Mapper002(Mapper002),
    Mapper003(Mapper003),
    Mapper004(Mapper004),
    Mapper005(Mapper005),
    Mapper007(Mapper007),
    Mapper009(Mapper009),
    Mapper010(Mapper010),
    Mapper011(Mapper011),
    Mapper019(Mapper019),
    Mapper024(MapperVRC6), // VRC6a
    Mapper026(MapperVRC6), // VRC6b
    Mapper034(Mapper034),
    Mapper066(Mapper066),
    Mapper069(Mapper069),
    Mapper071(Mapper071),
    Mapper079(Mapper079),
    Mapper085(Mapper085),
    Mapper206(Mapper206),
}

impl MapperEnum {
    #[inline]
    pub fn read_prg(&self, addr: u16) -> u8 {
        match self {
            MapperEnum::Mapper000(m) => m.read_prg(addr),
            MapperEnum::Mapper001(m) => m.read_prg(addr),
            MapperEnum::Mapper002(m) => m.read_prg(addr),
            MapperEnum::Mapper003(m) => m.read_prg(addr),
            MapperEnum::Mapper004(m) => m.read_prg(addr),
            MapperEnum::Mapper005(m) => m.read_prg(addr),
            MapperEnum::Mapper007(m) => m.read_prg(addr),
            MapperEnum::Mapper009(m) => m.read_prg(addr),
            MapperEnum::Mapper010(m) => m.read_prg(addr),
            MapperEnum::Mapper011(m) => m.read_prg(addr),
            MapperEnum::Mapper019(m) => m.read_prg(addr),
            MapperEnum::Mapper024(m) => m.read_prg(addr),
            MapperEnum::Mapper026(m) => m.read_prg(addr),
            MapperEnum::Mapper034(m) => m.read_prg(addr),
            MapperEnum::Mapper066(m) => m.read_prg(addr),
            MapperEnum::Mapper069(m) => m.read_prg(addr),
            MapperEnum::Mapper071(m) => m.read_prg(addr),
            MapperEnum::Mapper079(m) => m.read_prg(addr),
            MapperEnum::Mapper085(m) => m.read_prg(addr),
            MapperEnum::Mapper206(m) => m.read_prg(addr),
        }
    }

    #[inline]
    pub fn write_prg(&mut self, addr: u16, data: u8) {
        match self {
            MapperEnum::Mapper000(m) => m.write_prg(addr, data),
            MapperEnum::Mapper001(m) => m.write_prg(addr, data),
            MapperEnum::Mapper002(m) => m.write_prg(addr, data),
            MapperEnum::Mapper003(m) => m.write_prg(addr, data),
            MapperEnum::Mapper004(m) => m.write_prg(addr, data),
            MapperEnum::Mapper005(m) => m.write_prg(addr, data),
            MapperEnum::Mapper007(m) => m.write_prg(addr, data),
            MapperEnum::Mapper009(m) => m.write_prg(addr, data),
            MapperEnum::Mapper010(m) => m.write_prg(addr, data),
            MapperEnum::Mapper011(m) => m.write_prg(addr, data),
            MapperEnum::Mapper019(m) => m.write_prg(addr, data),
            MapperEnum::Mapper024(m) => m.write_prg(addr, data),
            MapperEnum::Mapper026(m) => m.write_prg(addr, data),
            MapperEnum::Mapper034(m) => m.write_prg(addr, data),
            MapperEnum::Mapper066(m) => m.write_prg(addr, data),
            MapperEnum::Mapper069(m) => m.write_prg(addr, data),
            MapperEnum::Mapper071(m) => m.write_prg(addr, data),
            MapperEnum::Mapper079(m) => m.write_prg(addr, data),
            MapperEnum::Mapper085(m) => m.write_prg(addr, data),
            MapperEnum::Mapper206(m) => m.write_prg(addr, data),
        }
    }

    #[inline]
    pub fn read_chr(&self, addr: u16) -> u8 {
        match self {
            MapperEnum::Mapper000(m) => m.read_chr(addr),
            MapperEnum::Mapper001(m) => m.read_chr(addr),
            MapperEnum::Mapper002(m) => m.read_chr(addr),
            MapperEnum::Mapper003(m) => m.read_chr(addr),
            MapperEnum::Mapper004(m) => m.read_chr(addr),
            MapperEnum::Mapper005(m) => m.read_chr(addr),
            MapperEnum::Mapper007(m) => m.read_chr(addr),
            MapperEnum::Mapper009(m) => m.read_chr(addr),
            MapperEnum::Mapper010(m) => m.read_chr(addr),
            MapperEnum::Mapper011(m) => m.read_chr(addr),
            MapperEnum::Mapper019(m) => m.read_chr(addr),
            MapperEnum::Mapper024(m) => m.read_chr(addr),
            MapperEnum::Mapper026(m) => m.read_chr(addr),
            MapperEnum::Mapper034(m) => m.read_chr(addr),
            MapperEnum::Mapper066(m) => m.read_chr(addr),
            MapperEnum::Mapper069(m) => m.read_chr(addr),
            MapperEnum::Mapper071(m) => m.read_chr(addr),
            MapperEnum::Mapper079(m) => m.read_chr(addr),
            MapperEnum::Mapper085(m) => m.read_chr(addr),
            MapperEnum::Mapper206(m) => m.read_chr(addr),
        }
    }

    #[inline]
    pub fn write_chr(&mut self, addr: u16, data: u8) {
        match self {
            MapperEnum::Mapper000(m) => m.write_chr(addr, data),
            MapperEnum::Mapper001(m) => m.write_chr(addr, data),
            MapperEnum::Mapper002(m) => m.write_chr(addr, data),
            MapperEnum::Mapper003(m) => m.write_chr(addr, data),
            MapperEnum::Mapper004(m) => m.write_chr(addr, data),
            MapperEnum::Mapper005(m) => m.write_chr(addr, data),
            MapperEnum::Mapper007(m) => m.write_chr(addr, data),
            MapperEnum::Mapper009(m) => m.write_chr(addr, data),
            MapperEnum::Mapper010(m) => m.write_chr(addr, data),
            MapperEnum::Mapper011(m) => m.write_chr(addr, data),
            MapperEnum::Mapper019(m) => m.write_chr(addr, data),
            MapperEnum::Mapper024(m) => m.write_chr(addr, data),
            MapperEnum::Mapper026(m) => m.write_chr(addr, data),
            MapperEnum::Mapper034(m) => m.write_chr(addr, data),
            MapperEnum::Mapper066(m) => m.write_chr(addr, data),
            MapperEnum::Mapper069(m) => m.write_chr(addr, data),
            MapperEnum::Mapper071(m) => m.write_chr(addr, data),
            MapperEnum::Mapper079(m) => m.write_chr(addr, data),
            MapperEnum::Mapper085(m) => m.write_chr(addr, data),
            MapperEnum::Mapper206(m) => m.write_chr(addr, data),
        }
    }

    #[inline]
    pub fn mirroring(&self) -> crate::cartridge::Mirroring {
        match self {
            MapperEnum::Mapper000(m) => m.mirroring(),
            MapperEnum::Mapper001(m) => m.mirroring(),
            MapperEnum::Mapper002(m) => m.mirroring(),
            MapperEnum::Mapper003(m) => m.mirroring(),
            MapperEnum::Mapper004(m) => m.mirroring(),
            MapperEnum::Mapper005(m) => m.mirroring(),
            MapperEnum::Mapper007(m) => m.mirroring(),
            MapperEnum::Mapper009(m) => m.mirroring(),
            MapperEnum::Mapper010(m) => m.mirroring(),
            MapperEnum::Mapper011(m) => m.mirroring(),
            MapperEnum::Mapper019(m) => m.mirroring(),
            MapperEnum::Mapper024(m) => m.mirroring(),
            MapperEnum::Mapper026(m) => m.mirroring(),
            MapperEnum::Mapper034(m) => m.mirroring(),
            MapperEnum::Mapper066(m) => m.mirroring(),
            MapperEnum::Mapper069(m) => m.mirroring(),
            MapperEnum::Mapper071(m) => m.mirroring(),
            MapperEnum::Mapper079(m) => m.mirroring(),
            MapperEnum::Mapper085(m) => m.mirroring(),
            MapperEnum::Mapper206(m) => m.mirroring(),
        }
    }

    #[inline]
    pub fn clock_scanline(&mut self) {
        match self {
            MapperEnum::Mapper000(m) => m.clock_scanline(),
            MapperEnum::Mapper001(m) => m.clock_scanline(),
            MapperEnum::Mapper002(m) => m.clock_scanline(),
            MapperEnum::Mapper003(m) => m.clock_scanline(),
            MapperEnum::Mapper004(m) => m.clock_scanline(),
            MapperEnum::Mapper005(m) => m.clock_scanline(),
            MapperEnum::Mapper007(m) => m.clock_scanline(),
            MapperEnum::Mapper009(m) => m.clock_scanline(),
            MapperEnum::Mapper010(m) => m.clock_scanline(),
            MapperEnum::Mapper011(m) => m.clock_scanline(),
            MapperEnum::Mapper019(m) => m.clock_scanline(),
            MapperEnum::Mapper024(m) => m.clock_scanline(),
            MapperEnum::Mapper026(m) => m.clock_scanline(),
            MapperEnum::Mapper034(m) => m.clock_scanline(),
            MapperEnum::Mapper066(m) => m.clock_scanline(),
            MapperEnum::Mapper069(m) => m.clock_scanline(),
            MapperEnum::Mapper071(m) => m.clock_scanline(),
            MapperEnum::Mapper079(m) => m.clock_scanline(),
            MapperEnum::Mapper085(m) => m.clock_scanline(),
            MapperEnum::Mapper206(m) => m.clock_scanline(),
        }
    }

    #[inline]
    pub fn irq_pending(&self) -> bool {
        match self {
            MapperEnum::Mapper000(m) => m.irq_pending(),
            MapperEnum::Mapper001(m) => m.irq_pending(),
            MapperEnum::Mapper002(m) => m.irq_pending(),
            MapperEnum::Mapper003(m) => m.irq_pending(),
            MapperEnum::Mapper004(m) => m.irq_pending(),
            MapperEnum::Mapper005(m) => m.irq_pending(),
            MapperEnum::Mapper007(m) => m.irq_pending(),
            MapperEnum::Mapper009(m) => m.irq_pending(),
            MapperEnum::Mapper010(m) => m.irq_pending(),
            MapperEnum::Mapper011(m) => m.irq_pending(),
            MapperEnum::Mapper019(m) => m.irq_pending(),
            MapperEnum::Mapper024(m) => m.irq_pending(),
            MapperEnum::Mapper026(m) => m.irq_pending(),
            MapperEnum::Mapper034(m) => m.irq_pending(),
            MapperEnum::Mapper066(m) => m.irq_pending(),
            MapperEnum::Mapper069(m) => m.irq_pending(),
            MapperEnum::Mapper071(m) => m.irq_pending(),
            MapperEnum::Mapper079(m) => m.irq_pending(),
            MapperEnum::Mapper085(m) => m.irq_pending(),
            MapperEnum::Mapper206(m) => m.irq_pending(),
        }
    }

    #[inline]
    pub fn irq_clear(&mut self) {
        match self {
            MapperEnum::Mapper000(m) => m.irq_clear(),
            MapperEnum::Mapper001(m) => m.irq_clear(),
            MapperEnum::Mapper002(m) => m.irq_clear(),
            MapperEnum::Mapper003(m) => m.irq_clear(),
            MapperEnum::Mapper004(m) => m.irq_clear(),
            MapperEnum::Mapper005(m) => m.irq_clear(),
            MapperEnum::Mapper007(m) => m.irq_clear(),
            MapperEnum::Mapper009(m) => m.irq_clear(),
            MapperEnum::Mapper010(m) => m.irq_clear(),
            MapperEnum::Mapper011(m) => m.irq_clear(),
            MapperEnum::Mapper019(m) => m.irq_clear(),
            MapperEnum::Mapper024(m) => m.irq_clear(),
            MapperEnum::Mapper026(m) => m.irq_clear(),
            MapperEnum::Mapper034(m) => m.irq_clear(),
            MapperEnum::Mapper066(m) => m.irq_clear(),
            MapperEnum::Mapper069(m) => m.irq_clear(),
            MapperEnum::Mapper071(m) => m.irq_clear(),
            MapperEnum::Mapper079(m) => m.irq_clear(),
            MapperEnum::Mapper085(m) => m.irq_clear(),
            MapperEnum::Mapper206(m) => m.irq_clear(),
        }
    }

    #[inline]
    pub fn audio_output(&self) -> f32 {
        match self {
            MapperEnum::Mapper000(m) => m.audio_output(),
            MapperEnum::Mapper001(m) => m.audio_output(),
            MapperEnum::Mapper002(m) => m.audio_output(),
            MapperEnum::Mapper003(m) => m.audio_output(),
            MapperEnum::Mapper004(m) => m.audio_output(),
            MapperEnum::Mapper005(m) => m.audio_output(),
            MapperEnum::Mapper007(m) => m.audio_output(),
            MapperEnum::Mapper009(m) => m.audio_output(),
            MapperEnum::Mapper010(m) => m.audio_output(),
            MapperEnum::Mapper011(m) => m.audio_output(),
            MapperEnum::Mapper019(m) => m.audio_output(),
            MapperEnum::Mapper024(m) => m.audio_output(),
            MapperEnum::Mapper026(m) => m.audio_output(),
            MapperEnum::Mapper034(m) => m.audio_output(),
            MapperEnum::Mapper066(m) => m.audio_output(),
            MapperEnum::Mapper069(m) => m.audio_output(),
            MapperEnum::Mapper071(m) => m.audio_output(),
            MapperEnum::Mapper079(m) => m.audio_output(),
            MapperEnum::Mapper085(m) => m.audio_output(),
            MapperEnum::Mapper206(m) => m.audio_output(),
        }
    }

    #[inline]
    pub fn get_sram(&self) -> Vec<u8> {
        match self {
            MapperEnum::Mapper000(m) => m.get_sram(),
            MapperEnum::Mapper001(m) => m.get_sram(),
            MapperEnum::Mapper002(m) => m.get_sram(),
            MapperEnum::Mapper003(m) => m.get_sram(),
            MapperEnum::Mapper004(m) => m.get_sram(),
            MapperEnum::Mapper005(m) => m.get_sram(),
            MapperEnum::Mapper007(m) => m.get_sram(),
            MapperEnum::Mapper009(m) => m.get_sram(),
            MapperEnum::Mapper010(m) => m.get_sram(),
            MapperEnum::Mapper011(m) => m.get_sram(),
            MapperEnum::Mapper019(m) => m.get_sram(),
            MapperEnum::Mapper024(m) => m.get_sram(),
            MapperEnum::Mapper026(m) => m.get_sram(),
            MapperEnum::Mapper034(m) => m.get_sram(),
            MapperEnum::Mapper066(m) => m.get_sram(),
            MapperEnum::Mapper069(m) => m.get_sram(),
            MapperEnum::Mapper071(m) => m.get_sram(),
            MapperEnum::Mapper079(m) => m.get_sram(),
            MapperEnum::Mapper085(m) => m.get_sram(),
            MapperEnum::Mapper206(m) => m.get_sram(),
        }
    }

    #[inline]
    pub fn set_sram(&mut self, data: &[u8]) {
        match self {
            MapperEnum::Mapper000(m) => m.set_sram(data),
            MapperEnum::Mapper001(m) => m.set_sram(data),
            MapperEnum::Mapper002(m) => m.set_sram(data),
            MapperEnum::Mapper003(m) => m.set_sram(data),
            MapperEnum::Mapper004(m) => m.set_sram(data),
            MapperEnum::Mapper005(m) => m.set_sram(data),
            MapperEnum::Mapper007(m) => m.set_sram(data),
            MapperEnum::Mapper009(m) => m.set_sram(data),
            MapperEnum::Mapper010(m) => m.set_sram(data),
            MapperEnum::Mapper011(m) => m.set_sram(data),
            MapperEnum::Mapper019(m) => m.set_sram(data),
            MapperEnum::Mapper024(m) => m.set_sram(data),
            MapperEnum::Mapper026(m) => m.set_sram(data),
            MapperEnum::Mapper034(m) => m.set_sram(data),
            MapperEnum::Mapper066(m) => m.set_sram(data),
            MapperEnum::Mapper069(m) => m.set_sram(data),
            MapperEnum::Mapper071(m) => m.set_sram(data),
            MapperEnum::Mapper079(m) => m.set_sram(data),
            MapperEnum::Mapper085(m) => m.set_sram(data),
            MapperEnum::Mapper206(m) => m.set_sram(data),
        }
    }

    #[inline]
    pub fn save_state(&self) -> Vec<u8> {
        match self {
            MapperEnum::Mapper000(m) => m.save_state(),
            MapperEnum::Mapper001(m) => m.save_state(),
            MapperEnum::Mapper002(m) => m.save_state(),
            MapperEnum::Mapper003(m) => m.save_state(),
            MapperEnum::Mapper004(m) => m.save_state(),
            MapperEnum::Mapper005(m) => m.save_state(),
            MapperEnum::Mapper007(m) => m.save_state(),
            MapperEnum::Mapper009(m) => m.save_state(),
            MapperEnum::Mapper010(m) => m.save_state(),
            MapperEnum::Mapper011(m) => m.save_state(),
            MapperEnum::Mapper019(m) => m.save_state(),
            MapperEnum::Mapper024(m) => m.save_state(),
            MapperEnum::Mapper026(m) => m.save_state(),
            MapperEnum::Mapper034(m) => m.save_state(),
            MapperEnum::Mapper066(m) => m.save_state(),
            MapperEnum::Mapper069(m) => m.save_state(),
            MapperEnum::Mapper071(m) => m.save_state(),
            MapperEnum::Mapper079(m) => m.save_state(),
            MapperEnum::Mapper085(m) => m.save_state(),
            MapperEnum::Mapper206(m) => m.save_state(),
        }
    }

    #[inline]
    pub fn load_state(&mut self, data: &[u8]) {
        match self {
            MapperEnum::Mapper000(m) => m.load_state(data),
            MapperEnum::Mapper001(m) => m.load_state(data),
            MapperEnum::Mapper002(m) => m.load_state(data),
            MapperEnum::Mapper003(m) => m.load_state(data),
            MapperEnum::Mapper004(m) => m.load_state(data),
            MapperEnum::Mapper005(m) => m.load_state(data),
            MapperEnum::Mapper007(m) => m.load_state(data),
            MapperEnum::Mapper009(m) => m.load_state(data),
            MapperEnum::Mapper010(m) => m.load_state(data),
            MapperEnum::Mapper011(m) => m.load_state(data),
            MapperEnum::Mapper019(m) => m.load_state(data),
            MapperEnum::Mapper024(m) => m.load_state(data),
            MapperEnum::Mapper026(m) => m.load_state(data),
            MapperEnum::Mapper034(m) => m.load_state(data),
            MapperEnum::Mapper066(m) => m.load_state(data),
            MapperEnum::Mapper069(m) => m.load_state(data),
            MapperEnum::Mapper071(m) => m.load_state(data),
            MapperEnum::Mapper079(m) => m.load_state(data),
            MapperEnum::Mapper085(m) => m.load_state(data),
            MapperEnum::Mapper206(m) => m.load_state(data),
        }
    }
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
    #[inline]
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

    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            _ => {} // ROM, ignore writes
        }
    }

    #[inline]
    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_rom[addr as usize]
    }

    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        // CHR RAM (when no CHR ROM on cart)
        if self.chr_rom.len() == 0x2000 {
            self.chr_rom[addr as usize] = data;
        }
    }

    #[inline]
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        self.mirroring
    }
    
    #[inline]
    fn get_sram(&self) -> Vec<u8> {
        self.prg_ram.clone()
    }
    
    #[inline]
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
    }
}

#[allow(dead_code)]
pub struct Mapper004 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    
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
    
    #[inline]
    fn prg_bank_offset(&self, bank: usize) -> usize {
        let bank = bank % self.prg_banks;
        bank << 13  // 0x2000 = 8KB
    }
    
    #[inline]
    fn chr_bank_offset(&self, bank: usize) -> usize {
        let chr_banks = self.chr_rom.len() >> 10;  // / 0x0400 = / 1KB
        if chr_banks == 0 { return 0; }
        let bank = bank % chr_banks;
        bank << 10  // 0x0400 = 1KB
    }
}

impl Mapper for Mapper004 {
    #[inline]
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
    
    #[inline]
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
    
    #[inline]
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
    
    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        // Only write if CHR RAM
        if addr < 0x2000 && self.chr_rom.len() <= 0x2000 {
            self.chr_rom[addr as usize] = data;
        }
    }
    
    #[inline]
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
    #[inline]
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

    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            0x8000..=0xFFFF => {
                if self.prg_banks > 0 {
                    self.bank_select = (data as usize % self.prg_banks) as u8;
                }
            }
            _ => {}
        }
    }

    #[inline]
    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_rom[addr as usize]
    }

    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        self.chr_rom[addr as usize] = data;
    }

    #[inline]
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
            if self.prg_banks > 0 {
                self.bank_select = (data[0] as usize % self.prg_banks) as u8;
            } else {
                self.bank_select = 0;
            }
        }
    }
}

#[allow(dead_code)]
pub struct Mapper001 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    
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
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, _mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x4000;
        let chr_size = chr_rom.len();
        let has_chr_ram = chr_rom.is_empty();
        let chr_banks = if has_chr_ram { 0 } else { chr_size / 0x1000 };
        Mapper001 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x2000],
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
    #[inline]
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
    
    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            0x8000..=0xFFFF => self.load_register(addr, data),
            _ => {}
        }
    }
    
    #[inline]
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
    
    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_banks == 0 {
            self.chr_rom[addr as usize] = data;
        }
    }
    
    #[inline]
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
    #[inline]
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
    
    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.chr_bank = data & 0x03; // Usually 2 bits, supporting up to 4 banks
        }
    }
    
    #[inline]
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
    
    #[inline]
    fn write_chr(&mut self, _addr: u16, _data: u8) {
        // CHR ROM is read-only for CNROM
    }
    
    #[inline]
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
    #[inline]
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
    
    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.prg_bank = data & 0x07; // bits 0-2: PRG bank
            self.mirroring_bit = data & 0x10 != 0; // bit 4: mirroring
        }
    }
    
    #[inline]
    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_ram[addr as usize & 0x1FFF]
    }
    
    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        self.chr_ram[addr as usize & 0x1FFF] = data;
    }
    
    #[inline]
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

#[allow(dead_code)]
pub struct Mapper009 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
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
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, _mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x2000;
        Mapper009 {
            prg_rom, chr_rom,
            prg_ram: vec![0; 0x2000],
            prg_banks,
            prg_bank: 0,
            chr_bank_0_fd: 0, chr_bank_0_fe: 0,
            chr_bank_1_fd: 0, chr_bank_1_fe: 0,
            latch_0: std::cell::Cell::new(true), latch_1: std::cell::Cell::new(true),
            mirror_mode: 0,
        }
    }
}

impl Mapper for Mapper009 {
    #[inline]
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
    
    #[inline]
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
    
    #[inline]
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
    
    #[inline]
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    
    #[inline]
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
    #[inline]
    fn read_prg(&self, addr: u16) -> u8 {
        if addr >= 0x8000 {
            let prg_banks = self.prg_rom.len() / 0x8000;
            if prg_banks == 0 { return 0; }
            let bank = (self.prg_bank as usize) % prg_banks;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
        } else { 0 }
    }
    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.prg_bank = (data >> 4) & 0x03;
            self.chr_bank = data & 0x0F;
        }
    }
    #[inline]
    fn read_chr(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() { return 0; }
        let chr_banks = self.chr_rom.len() / 0x2000;
        if chr_banks == 0 { return 0; }
        let bank = (self.chr_bank as usize) % chr_banks;
        let offset = bank * 0x2000 + (addr as usize & 0x1FFF);
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }
    #[inline]
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    #[inline]
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
    #[inline]
    fn read_prg(&self, addr: u16) -> u8 {
        if addr >= 0x8000 {
            let prg_banks = self.prg_rom.len() / 0x8000;
            if prg_banks == 0 { return 0; }
            let bank = (self.prg_bank as usize) % prg_banks;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
        } else { 0 }
    }
    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.chr_bank = data & 0x03;
            self.prg_bank = (data >> 4) & 0x03;
        }
    }
    #[inline]
    fn read_chr(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() { return 0; }
        let chr_banks = self.chr_rom.len() / 0x2000;
        if chr_banks == 0 { return 0; }
        let bank = (self.chr_bank as usize) % chr_banks;
        let offset = bank * 0x2000 + (addr as usize & 0x1FFF);
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }
    #[inline]
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    fn read_chr(&self, addr: u16) -> u8 { self.chr_ram[addr as usize & 0x1FFF] }
    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) { self.chr_ram[addr as usize & 0x1FFF] = data; }
    #[inline]
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
    #[inline]
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
    
    #[inline]
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
    
    #[inline]
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
    
    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_rom.len() <= 0x2000 {
            self.chr_rom[addr as usize & 0x1FFF] = data;
        }
    }
    
    #[inline]
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

#[allow(dead_code)]
pub struct Mapper010 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
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
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, _mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x4000;
        Mapper010 {
            prg_rom, chr_rom, prg_ram: vec![0; 0x2000],
            prg_banks, prg_bank: 0,
            chr_bank_0_fd: 0, chr_bank_0_fe: 0,
            chr_bank_1_fd: 0, chr_bank_1_fe: 0,
            latch_0: std::cell::Cell::new(true), latch_1: std::cell::Cell::new(true),
            mirror_mode: 0,
        }
    }
}

impl Mapper for Mapper010 {
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    #[inline]
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
    #[inline]
    fn read_prg(&self, addr: u16) -> u8 {
        if addr >= 0x8000 {
            let prg_banks = self.prg_rom.len() / 0x8000;
            if prg_banks == 0 { return 0; }
            let bank = (self.prg_bank as usize) % prg_banks;
            let offset = bank * 0x8000 + (addr - 0x8000) as usize;
            if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
        } else { 0 }
    }
    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        if addr >= 0x4100 && addr <= 0x5FFF {
            self.chr_bank = data & 0x07;
            self.prg_bank = (data >> 3) & 0x01;
        }
    }
    #[inline]
    fn read_chr(&self, addr: u16) -> u8 {
        if self.chr_rom.is_empty() { return 0; }
        let chr_banks = self.chr_rom.len() / 0x2000;
        if chr_banks == 0 { return 0; }
        let bank = (self.chr_bank as usize) % chr_banks;
        let offset = bank * 0x2000 + (addr as usize & 0x1FFF);
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }
    #[inline]
    fn write_chr(&mut self, _addr: u16, _data: u8) {}
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_rom.len() <= 0x2000 { self.chr_rom[addr as usize & 0x1FFF] = data; }
    }
    #[inline]
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

// === Mapper 34: BNROM ===
// 32KB PRG bank switching, CHR RAM only. Similar to AxROM but simpler (no mirroring control).

pub struct Mapper034 {
    prg_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    prg_ram: Vec<u8>,
    mirroring: crate::cartridge::Mirroring,
    prg_banks: usize,
    bank_select: u8,
}

impl Mapper034 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks = prg_rom.len() / 0x8000;
        Mapper034 {
            prg_rom,
            chr_ram: if chr_rom.is_empty() { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x2000],
            mirroring,
            prg_banks: prg_banks.max(1),
            bank_select: 0,
        }
    }
}

impl Mapper for Mapper034 {
    #[inline]
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xFFFF => {
                let bank = (self.bank_select as usize) % self.prg_banks;
                let offset = bank * 0x8000 + (addr - 0x8000) as usize;
                if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
            }
            _ => 0,
        }
    }
    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,
            0x8000..=0xFFFF => {
                self.bank_select = (data as usize % self.prg_banks) as u8;
            }
            _ => {}
        }
    }
    #[inline]
    fn read_chr(&self, addr: u16) -> u8 {
        self.chr_ram[addr as usize & 0x1FFF]
    }
    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        self.chr_ram[addr as usize & 0x1FFF] = data;
    }
    #[inline]
    fn mirroring(&self) -> crate::cartridge::Mirroring { self.mirroring }
    fn get_sram(&self) -> Vec<u8> { self.prg_ram.clone() }
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
    }
    fn save_state(&self) -> Vec<u8> { vec![self.bank_select] }
    fn load_state(&mut self, data: &[u8]) {
        if !data.is_empty() {
            self.bank_select = (data[0] as usize % self.prg_banks) as u8;
        }
    }
}

// === Mapper 24/26: Konami VRC6 ===
// PRG: 16KB switchable at $8000 + 8KB switchable at $C000 + 8KB fixed at $E000
// CHR: 8 × 1KB banks
// IRQ: scanline counter
// Audio: 2 pulse channels + 1 sawtooth channel

#[allow(dead_code)]
pub struct MapperVRC6 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    prg_banks: usize,

    // PRG bank registers
    prg_bank_16k: u8,  // $8000-$BFFF (16KB)
    prg_bank_8k: u8,   // $C000-$DFFF (8KB)

    // CHR bank registers (8 × 1KB)
    chr_banks: [u8; 8],

    // Mirroring
    mirror_mode: u8,

    // IRQ
    irq_latch: u8,
    irq_counter: u8,
    irq_prescaler: i16,
    irq_enabled: bool,
    irq_cycle_mode: bool, // true = cycle mode, false = scanline mode
    irq_pending_flag: bool,

    // Audio - Pulse channel 1
    pulse1_volume: u8,
    pulse1_duty: u8,
    pulse1_mode: bool,  // direct volume mode (duty ignored)
    pulse1_period: u16,
    pulse1_enabled: bool,
    pulse1_timer: u16,
    pulse1_step: u8,

    // Audio - Pulse channel 2
    pulse2_volume: u8,
    pulse2_duty: u8,
    pulse2_mode: bool,
    pulse2_period: u16,
    pulse2_enabled: bool,
    pulse2_timer: u16,
    pulse2_step: u8,

    // Audio - Sawtooth channel
    saw_accum_rate: u8,
    saw_period: u16,
    saw_enabled: bool,
    saw_timer: u16,
    saw_accum: u8,
    saw_step: u8,

    // Mapper variant: false = mapper 24 ($x000/$x001/$x002), true = mapper 26 (A0/A1 swapped)
    is_vrc6b: bool,
}

impl MapperVRC6 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring, is_vrc6b: bool) -> Self {
        let prg_banks = prg_rom.len() / 0x2000; // 8KB banks
        let has_chr_ram = chr_rom.is_empty();
        MapperVRC6 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x2000],
            prg_banks: prg_banks.max(1),
            prg_bank_16k: 0, prg_bank_8k: 0,
            chr_banks: [0; 8],
            mirror_mode: if mirroring == crate::cartridge::Mirroring::Vertical { 0 } else { 1 },
            irq_latch: 0, irq_counter: 0, irq_prescaler: 341,
            irq_enabled: false, irq_cycle_mode: false, irq_pending_flag: false,
            pulse1_volume: 0, pulse1_duty: 0, pulse1_mode: false,
            pulse1_period: 0, pulse1_enabled: false, pulse1_timer: 0, pulse1_step: 0,
            pulse2_volume: 0, pulse2_duty: 0, pulse2_mode: false,
            pulse2_period: 0, pulse2_enabled: false, pulse2_timer: 0, pulse2_step: 0,
            saw_accum_rate: 0, saw_period: 0, saw_enabled: false,
            saw_timer: 0, saw_accum: 0, saw_step: 0,
            is_vrc6b,
        }
    }

    fn translate_addr(&self, addr: u16) -> u16 {
        if addr < 0x8000 { return addr; }
        if self.is_vrc6b {
            // Mapper 26: swap A0 and A1
            let base = addr & 0xFFFC;
            let a0 = (addr >> 1) & 1;
            let a1 = (addr & 1) << 1;
            base | a0 | a1
        } else {
            addr
        }
    }

    fn clock_irq(&mut self) {
        if self.irq_counter == 0xFF {
            self.irq_counter = self.irq_latch;
            self.irq_pending_flag = true;
        } else {
            self.irq_counter += 1;
        }
    }

    fn pulse_output(&self, volume: u8, duty: u8, mode: bool, step: u8, enabled: bool) -> f32 {
        if !enabled { return 0.0; }
        if mode {
            // Direct volume mode
            volume as f32 / 15.0
        } else {
            // Duty cycle: step < (duty + 1) means output high
            if step <= duty { volume as f32 / 15.0 } else { 0.0 }
        }
    }

    fn saw_output(&self) -> f32 {
        if !self.saw_enabled { return 0.0; }
        // Output is top 5 bits of accumulator
        ((self.saw_accum >> 3) & 0x1F) as f32 / 31.0
    }
}

impl Mapper for MapperVRC6 {
    #[inline]
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0xBFFF => {
                // 16KB bank
                let bank = (self.prg_bank_16k as usize * 2) % self.prg_banks;
                let offset = bank * 0x2000 + (addr - 0x8000) as usize;
                if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
            }
            0xC000..=0xDFFF => {
                let bank = (self.prg_bank_8k as usize) % self.prg_banks;
                let offset = bank * 0x2000 + (addr - 0xC000) as usize;
                if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
            }
            0xE000..=0xFFFF => {
                let bank = self.prg_banks - 1;
                let offset = bank * 0x2000 + (addr - 0xE000) as usize;
                if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
            }
            _ => 0,
        }
    }

    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        let addr = self.translate_addr(addr);
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,

            // $8000-$8003: PRG 16KB bank select
            0x8000..=0x8003 => self.prg_bank_16k = data & 0x0F,

            // $9000-$9002: Pulse 1 audio
            0x9000 => {
                self.pulse1_volume = data & 0x0F;
                self.pulse1_duty = (data >> 4) & 0x07;
                self.pulse1_mode = data & 0x80 != 0;
            }
            0x9001 => {
                self.pulse1_period = (self.pulse1_period & 0x0F00) | data as u16;
            }
            0x9002 => {
                self.pulse1_period = (self.pulse1_period & 0x00FF) | ((data as u16 & 0x0F) << 8);
                self.pulse1_enabled = data & 0x80 != 0;
            }

            // $9003: frequency scaling (not commonly used, ignore for now)
            0x9003 => {}

            // $A000-$A002: Pulse 2 audio
            0xA000 => {
                self.pulse2_volume = data & 0x0F;
                self.pulse2_duty = (data >> 4) & 0x07;
                self.pulse2_mode = data & 0x80 != 0;
            }
            0xA001 => {
                self.pulse2_period = (self.pulse2_period & 0x0F00) | data as u16;
            }
            0xA002 => {
                self.pulse2_period = (self.pulse2_period & 0x00FF) | ((data as u16 & 0x0F) << 8);
                self.pulse2_enabled = data & 0x80 != 0;
            }

            // $B000-$B002: Sawtooth audio
            0xB000 => {
                self.saw_accum_rate = data & 0x3F;
            }
            0xB001 => {
                self.saw_period = (self.saw_period & 0x0F00) | data as u16;
            }
            0xB002 => {
                self.saw_period = (self.saw_period & 0x00FF) | ((data as u16 & 0x0F) << 8);
                self.saw_enabled = data & 0x80 != 0;
            }

            // $B003: Mirroring control
            0xB003 => {
                self.mirror_mode = (data >> 2) & 0x03;
            }

            // $C000-$C003: PRG 8KB bank select
            0xC000..=0xC003 => self.prg_bank_8k = data & 0x1F,

            // $D000-$D003: CHR banks 0-3
            0xD000 => self.chr_banks[0] = data,
            0xD001 => self.chr_banks[1] = data,
            0xD002 => self.chr_banks[2] = data,
            0xD003 => self.chr_banks[3] = data,

            // $E000-$E003: CHR banks 4-7
            0xE000 => self.chr_banks[4] = data,
            0xE001 => self.chr_banks[5] = data,
            0xE002 => self.chr_banks[6] = data,
            0xE003 => self.chr_banks[7] = data,

            // $F000: IRQ latch
            0xF000 => self.irq_latch = data,

            // $F001: IRQ control
            0xF001 => {
                self.irq_cycle_mode = data & 0x04 != 0;
                self.irq_enabled = data & 0x02 != 0;
                if self.irq_enabled {
                    self.irq_counter = self.irq_latch;
                    self.irq_prescaler = 341;
                }
                self.irq_pending_flag = false;
            }

            // $F002: IRQ acknowledge
            0xF002 => {
                self.irq_pending_flag = false;
            }

            _ => {}
        }
    }

    #[inline]
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

    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_rom.len() <= 0x2000 {
            self.chr_rom[addr as usize & 0x1FFF] = data;
        }
    }

    #[inline]
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        match self.mirror_mode {
            0 => crate::cartridge::Mirroring::Vertical,
            1 => crate::cartridge::Mirroring::Horizontal,
            2 => crate::cartridge::Mirroring::SingleScreenLower,
            3 => crate::cartridge::Mirroring::SingleScreenUpper,
            _ => crate::cartridge::Mirroring::Vertical,
        }
    }

    fn clock_scanline(&mut self) {
        if !self.irq_enabled { return; }

        if self.irq_cycle_mode {
            self.clock_irq();
        } else {
            // Scanline mode: use prescaler
            self.irq_prescaler -= 3; // ~3 PPU cycles per CPU cycle approximation
            if self.irq_prescaler <= 0 {
                self.irq_prescaler += 341;
                self.clock_irq();
            }
        }

        // Clock audio channels (~114 CPU cycles per scanline)
        for _ in 0..114 {
            // Pulse 1
            if self.pulse1_enabled && self.pulse1_period > 0 {
                if self.pulse1_timer == 0 {
                    self.pulse1_timer = self.pulse1_period;
                    self.pulse1_step = (self.pulse1_step + 1) % 16;
                } else {
                    self.pulse1_timer -= 1;
                }
            }
            // Pulse 2
            if self.pulse2_enabled && self.pulse2_period > 0 {
                if self.pulse2_timer == 0 {
                    self.pulse2_timer = self.pulse2_period;
                    self.pulse2_step = (self.pulse2_step + 1) % 16;
                } else {
                    self.pulse2_timer -= 1;
                }
            }
            // Sawtooth
            if self.saw_enabled && self.saw_period > 0 {
                if self.saw_timer == 0 {
                    self.saw_timer = self.saw_period;
                    self.saw_step += 1;
                    if self.saw_step >= 14 {
                        self.saw_step = 0;
                        self.saw_accum = 0;
                    } else if self.saw_step % 2 == 0 {
                        self.saw_accum = self.saw_accum.wrapping_add(self.saw_accum_rate);
                    }
                } else {
                    self.saw_timer -= 1;
                }
            }
        }
    }

    fn irq_pending(&self) -> bool { self.irq_pending_flag }
    fn irq_clear(&mut self) { self.irq_pending_flag = false; }

    fn audio_output(&self) -> f32 {
        let p1 = self.pulse_output(self.pulse1_volume, self.pulse1_duty, self.pulse1_mode, self.pulse1_step, self.pulse1_enabled);
        let p2 = self.pulse_output(self.pulse2_volume, self.pulse2_duty, self.pulse2_mode, self.pulse2_step, self.pulse2_enabled);
        let saw = self.saw_output();
        // Mix: VRC6 audio is roughly equal amplitude to APU
        (p1 + p2 + saw) / 3.0
    }

    fn get_sram(&self) -> Vec<u8> { self.prg_ram.clone() }
    fn set_sram(&mut self, data: &[u8]) {
        let len = data.len().min(self.prg_ram.len());
        self.prg_ram[..len].copy_from_slice(&data[..len]);
    }

    fn save_state(&self) -> Vec<u8> {
        let mut s = Vec::new();
        s.push(self.prg_bank_16k);
        s.push(self.prg_bank_8k);
        s.extend_from_slice(&self.chr_banks);
        s.push(self.mirror_mode);
        s.push(self.irq_latch);
        s.push(self.irq_counter);
        s.extend_from_slice(&self.irq_prescaler.to_le_bytes());
        s.push(if self.irq_enabled { 1 } else { 0 });
        s.push(if self.irq_cycle_mode { 1 } else { 0 });
        s.push(if self.irq_pending_flag { 1 } else { 0 });
        // Audio state
        s.push(self.pulse1_volume); s.push(self.pulse1_duty);
        s.push(if self.pulse1_mode { 1 } else { 0 });
        s.extend_from_slice(&self.pulse1_period.to_le_bytes());
        s.push(if self.pulse1_enabled { 1 } else { 0 });
        s.push(self.pulse1_step);
        s.push(self.pulse2_volume); s.push(self.pulse2_duty);
        s.push(if self.pulse2_mode { 1 } else { 0 });
        s.extend_from_slice(&self.pulse2_period.to_le_bytes());
        s.push(if self.pulse2_enabled { 1 } else { 0 });
        s.push(self.pulse2_step);
        s.push(self.saw_accum_rate);
        s.extend_from_slice(&self.saw_period.to_le_bytes());
        s.push(if self.saw_enabled { 1 } else { 0 });
        s.push(self.saw_accum);
        s.push(self.saw_step);
        s
    }

    fn load_state(&mut self, data: &[u8]) {
        if data.len() < 17 { return; }
        let mut p = 0;
        self.prg_bank_16k = data[p]; p += 1;
        self.prg_bank_8k = data[p]; p += 1;
        self.chr_banks.copy_from_slice(&data[p..p+8]); p += 8;
        self.mirror_mode = data[p]; p += 1;
        self.irq_latch = data[p]; p += 1;
        self.irq_counter = data[p]; p += 1;
        self.irq_prescaler = i16::from_le_bytes([data[p], data[p+1]]); p += 2;
        self.irq_enabled = data[p] != 0; p += 1;
        self.irq_cycle_mode = data[p] != 0; p += 1;
        self.irq_pending_flag = data[p] != 0; p += 1;
        // Audio state
        if data.len() >= p + 18 {
            self.pulse1_volume = data[p]; p += 1;
            self.pulse1_duty = data[p]; p += 1;
            self.pulse1_mode = data[p] != 0; p += 1;
            self.pulse1_period = u16::from_le_bytes([data[p], data[p+1]]); p += 2;
            self.pulse1_enabled = data[p] != 0; p += 1;
            self.pulse1_step = data[p]; p += 1;
            self.pulse2_volume = data[p]; p += 1;
            self.pulse2_duty = data[p]; p += 1;
            self.pulse2_mode = data[p] != 0; p += 1;
            self.pulse2_period = u16::from_le_bytes([data[p], data[p+1]]); p += 2;
            self.pulse2_enabled = data[p] != 0; p += 1;
            self.pulse2_step = data[p]; p += 1;
            self.saw_accum_rate = data[p]; p += 1;
            self.saw_period = u16::from_le_bytes([data[p], data[p+1]]); p += 2;
            self.saw_enabled = data[p] != 0; p += 1;
            self.saw_accum = data[p]; p += 1;
            self.saw_step = data[p];
        }
    }
}

// === Mapper 5: MMC5 (ExROM) ===
// Used by Castlevania III (US), Just Breed, etc.
// Complex mapper with multiple PRG/CHR banking modes, ExRAM, hardware multiplier, scanline IRQ.

#[allow(dead_code)]
pub struct Mapper005 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,        // 8KB at $6000-$7FFF (can be up to 64KB total)
    ex_ram: Vec<u8>,         // 1KB ExRAM at $5C00-$5FFF

    prg_banks_8k: usize,

    // PRG mode (0-3)
    prg_mode: u8,
    // CHR mode (0-3)
    chr_mode: u8,

    // PRG bank registers
    prg_bank: [u8; 5],      // banks for different modes
    prg_ram_protect1: u8,
    prg_ram_protect2: u8,

    // CHR bank registers
    chr_bank: [u16; 12],    // up to 12 registers for various CHR modes
    chr_upper: u8,           // upper CHR bits ($5130)

    // Mirroring
    nametable_mapping: u8,

    // Multiplier
    multiplicand: u8,
    multiplier: u8,

    // Fill mode
    fill_tile: u8,
    fill_attr: u8,

    // IRQ
    irq_target: u8,
    irq_enabled: bool,
    irq_pending_flag: bool,
    scanline_counter: u8,
    in_frame: bool,

    // ExRAM mode
    ex_ram_mode: u8,
}

impl Mapper005 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, _mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks_8k = prg_rom.len() / 0x2000;
        let has_chr_ram = chr_rom.is_empty();
        Mapper005 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x10000], // 64KB PRG RAM
            ex_ram: vec![0; 0x0400],

            prg_banks_8k: prg_banks_8k.max(1),

            prg_mode: 3,  // default to 4×8KB mode
            chr_mode: 3,  // default to 8×1KB mode

            prg_bank: [0, 0, 0, 0, 0xFF], // last bank defaults to last page
            prg_ram_protect1: 0,
            prg_ram_protect2: 0,

            chr_bank: [0; 12],
            chr_upper: 0,

            nametable_mapping: 0,

            multiplicand: 0,
            multiplier: 0,

            fill_tile: 0,
            fill_attr: 0,

            irq_target: 0,
            irq_enabled: false,
            irq_pending_flag: false,
            scanline_counter: 0,
            in_frame: false,

            ex_ram_mode: 0,
        }
    }

    fn prg_bank_offset(&self, bank: u8, is_ram: bool) -> usize {
        if is_ram {
            // PRG RAM bank
            ((bank & 0x07) as usize * 0x2000) % self.prg_ram.len()
        } else {
            // PRG ROM bank
            ((bank & 0x7F) as usize % self.prg_banks_8k) * 0x2000
        }
    }
}

impl Mapper for Mapper005 {
    #[inline]
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            // $5000-$5BFF: MMC5 internal registers
            0x5000..=0x5BFF => {
                match addr {
                    // Multiplier result
                    0x5205 => {
                        let result = self.multiplicand as u16 * self.multiplier as u16;
                        (result & 0xFF) as u8
                    }
                    0x5206 => {
                        let result = self.multiplicand as u16 * self.multiplier as u16;
                        (result >> 8) as u8
                    }
                    _ => 0,
                }
            }

            // $5C00-$5FFF: ExRAM
            0x5C00..=0x5FFF => {
                if self.ex_ram_mode >= 2 {
                    self.ex_ram[(addr - 0x5C00) as usize]
                } else {
                    0
                }
            }

            // $6000-$7FFF: PRG RAM
            0x6000..=0x7FFF => {
                let bank_offset = self.prg_bank_offset(self.prg_bank[0], true);
                let local = (addr - 0x6000) as usize;
                if bank_offset + local < self.prg_ram.len() {
                    self.prg_ram[bank_offset + local]
                } else { 0 }
            }

            // $8000-$FFFF: PRG ROM/RAM based on mode
            0x8000..=0xFFFF => {
                match self.prg_mode {
                    0 => {
                        // Mode 0: one 32KB bank at $8000
                        let bank = (self.prg_bank[4] & 0x7C) as usize; // 32KB aligned
                        let offset = (bank % self.prg_banks_8k) * 0x2000 + (addr - 0x8000) as usize;
                        if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                    }
                    1 => {
                        // Mode 1: 16KB + 16KB
                        match addr {
                            0x8000..=0xBFFF => {
                                let bank_reg = self.prg_bank[1];
                                let is_ram = bank_reg & 0x80 == 0;
                                if is_ram {
                                    let offset = self.prg_bank_offset(bank_reg & 0x06, true) + (addr - 0x8000) as usize;
                                    if offset < self.prg_ram.len() { self.prg_ram[offset] } else { 0 }
                                } else {
                                    let bank = ((bank_reg & 0x7E) as usize) % self.prg_banks_8k;
                                    let offset = bank * 0x2000 + (addr - 0x8000) as usize;
                                    if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                                }
                            }
                            0xC000..=0xFFFF => {
                                let bank = ((self.prg_bank[4] & 0x7E) as usize) % self.prg_banks_8k;
                                let offset = bank * 0x2000 + (addr - 0xC000) as usize;
                                if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                            }
                            _ => 0,
                        }
                    }
                    2 => {
                        // Mode 2: 16KB + 8KB + 8KB
                        match addr {
                            0x8000..=0xBFFF => {
                                let bank_reg = self.prg_bank[1];
                                let is_ram = bank_reg & 0x80 == 0;
                                if is_ram {
                                    let offset = self.prg_bank_offset(bank_reg & 0x06, true) + (addr - 0x8000) as usize;
                                    if offset < self.prg_ram.len() { self.prg_ram[offset] } else { 0 }
                                } else {
                                    let bank = ((bank_reg & 0x7E) as usize) % self.prg_banks_8k;
                                    let offset = bank * 0x2000 + (addr - 0x8000) as usize;
                                    if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                                }
                            }
                            0xC000..=0xDFFF => {
                                let bank_reg = self.prg_bank[3];
                                let is_ram = bank_reg & 0x80 == 0;
                                if is_ram {
                                    let offset = self.prg_bank_offset(bank_reg, true) + (addr - 0xC000) as usize;
                                    if offset < self.prg_ram.len() { self.prg_ram[offset] } else { 0 }
                                } else {
                                    let bank = ((bank_reg & 0x7F) as usize) % self.prg_banks_8k;
                                    let offset = bank * 0x2000 + (addr - 0xC000) as usize;
                                    if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                                }
                            }
                            0xE000..=0xFFFF => {
                                let bank = ((self.prg_bank[4] & 0x7F) as usize) % self.prg_banks_8k;
                                let offset = bank * 0x2000 + (addr - 0xE000) as usize;
                                if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                            }
                            _ => 0,
                        }
                    }
                    3 | _ => {
                        // Mode 3: 4 × 8KB banks
                        match addr {
                            0x8000..=0x9FFF => {
                                let bank_reg = self.prg_bank[1];
                                let is_ram = bank_reg & 0x80 == 0;
                                if is_ram {
                                    let offset = self.prg_bank_offset(bank_reg, true) + (addr - 0x8000) as usize;
                                    if offset < self.prg_ram.len() { self.prg_ram[offset] } else { 0 }
                                } else {
                                    let bank = ((bank_reg & 0x7F) as usize) % self.prg_banks_8k;
                                    let offset = bank * 0x2000 + (addr - 0x8000) as usize;
                                    if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                                }
                            }
                            0xA000..=0xBFFF => {
                                let bank_reg = self.prg_bank[2];
                                let is_ram = bank_reg & 0x80 == 0;
                                if is_ram {
                                    let offset = self.prg_bank_offset(bank_reg, true) + (addr - 0xA000) as usize;
                                    if offset < self.prg_ram.len() { self.prg_ram[offset] } else { 0 }
                                } else {
                                    let bank = ((bank_reg & 0x7F) as usize) % self.prg_banks_8k;
                                    let offset = bank * 0x2000 + (addr - 0xA000) as usize;
                                    if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                                }
                            }
                            0xC000..=0xDFFF => {
                                let bank_reg = self.prg_bank[3];
                                let is_ram = bank_reg & 0x80 == 0;
                                if is_ram {
                                    let offset = self.prg_bank_offset(bank_reg, true) + (addr - 0xC000) as usize;
                                    if offset < self.prg_ram.len() { self.prg_ram[offset] } else { 0 }
                                } else {
                                    let bank = ((bank_reg & 0x7F) as usize) % self.prg_banks_8k;
                                    let offset = bank * 0x2000 + (addr - 0xC000) as usize;
                                    if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                                }
                            }
                            0xE000..=0xFFFF => {
                                // Always ROM in mode 3
                                let bank = ((self.prg_bank[4] & 0x7F) as usize) % self.prg_banks_8k;
                                let offset = bank * 0x2000 + (addr - 0xE000) as usize;
                                if offset < self.prg_rom.len() { self.prg_rom[offset] } else { 0 }
                            }
                            _ => 0,
                        }
                    }
                }
            }
            _ => 0,
        }
    }

    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            // $5000-$5015: Audio (skip for now)
            0x5000..=0x5015 => {}

            0x5100 => self.prg_mode = data & 0x03,
            0x5101 => self.chr_mode = data & 0x03,
            0x5102 => self.prg_ram_protect1 = data & 0x03,
            0x5103 => self.prg_ram_protect2 = data & 0x03,

            0x5104 => self.ex_ram_mode = data & 0x03,

            0x5105 => self.nametable_mapping = data,

            0x5106 => self.fill_tile = data,
            0x5107 => self.fill_attr = data & 0x03,

            // PRG bank registers
            0x5113 => self.prg_bank[0] = data, // $6000-$7FFF RAM bank
            0x5114 => self.prg_bank[1] = data, // $8000-$9FFF
            0x5115 => self.prg_bank[2] = data, // $A000-$BFFF
            0x5116 => self.prg_bank[3] = data, // $C000-$DFFF
            0x5117 => self.prg_bank[4] = data | 0x80, // $E000-$FFFF (always ROM)

            // CHR bank registers
            0x5120 => self.chr_bank[0] = data as u16 | ((self.chr_upper as u16) << 8),
            0x5121 => self.chr_bank[1] = data as u16 | ((self.chr_upper as u16) << 8),
            0x5122 => self.chr_bank[2] = data as u16 | ((self.chr_upper as u16) << 8),
            0x5123 => self.chr_bank[3] = data as u16 | ((self.chr_upper as u16) << 8),
            0x5124 => self.chr_bank[4] = data as u16 | ((self.chr_upper as u16) << 8),
            0x5125 => self.chr_bank[5] = data as u16 | ((self.chr_upper as u16) << 8),
            0x5126 => self.chr_bank[6] = data as u16 | ((self.chr_upper as u16) << 8),
            0x5127 => self.chr_bank[7] = data as u16 | ((self.chr_upper as u16) << 8),
            0x5128 => self.chr_bank[8] = data as u16 | ((self.chr_upper as u16) << 8),
            0x5129 => self.chr_bank[9] = data as u16 | ((self.chr_upper as u16) << 8),
            0x512A => self.chr_bank[10] = data as u16 | ((self.chr_upper as u16) << 8),
            0x512B => self.chr_bank[11] = data as u16 | ((self.chr_upper as u16) << 8),

            0x5130 => self.chr_upper = data & 0x03,

            // Multiplier
            0x5205 => self.multiplicand = data,
            0x5206 => self.multiplier = data,

            // IRQ
            0x5203 => self.irq_target = data,
            0x5204 => {
                self.irq_enabled = data & 0x80 != 0;
            }

            // ExRAM writes
            0x5C00..=0x5FFF => {
                if self.ex_ram_mode <= 1 || self.ex_ram_mode == 2 {
                    self.ex_ram[(addr - 0x5C00) as usize] = data;
                }
            }

            // PRG RAM writes
            0x6000..=0x7FFF => {
                if self.prg_ram_protect1 == 0x02 && self.prg_ram_protect2 == 0x01 {
                    let bank_offset = self.prg_bank_offset(self.prg_bank[0], true);
                    let local = (addr - 0x6000) as usize;
                    if bank_offset + local < self.prg_ram.len() {
                        self.prg_ram[bank_offset + local] = data;
                    }
                }
            }

            // ROM area writes - only to RAM banks in certain modes
            0x8000..=0xDFFF => {
                if self.prg_ram_protect1 == 0x02 && self.prg_ram_protect2 == 0x01 {
                    let bank_reg = match (self.prg_mode, addr) {
                        (3, 0x8000..=0x9FFF) => self.prg_bank[1],
                        (3, 0xA000..=0xBFFF) => self.prg_bank[2],
                        (3, 0xC000..=0xDFFF) | (2, 0xC000..=0xDFFF) => self.prg_bank[3],
                        (1, 0x8000..=0xBFFF) | (2, 0x8000..=0xBFFF) => self.prg_bank[1],
                        _ => return,
                    };
                    if bank_reg & 0x80 == 0 {
                        // Is RAM
                        let base = match addr {
                            0x8000..=0x9FFF => 0x8000,
                            0xA000..=0xBFFF => 0xA000,
                            0xC000..=0xDFFF => 0xC000,
                            _ => return,
                        };
                        let offset = self.prg_bank_offset(bank_reg, true) + (addr - base) as usize;
                        if offset < self.prg_ram.len() {
                            self.prg_ram[offset] = data;
                        }
                    }
                }
            }

            _ => {}
        }
    }

    #[inline]
    fn read_chr(&self, addr: u16) -> u8 {
        let chr_banks_1k = self.chr_rom.len() / 0x0400;
        if chr_banks_1k == 0 {
            // CHR RAM
            return self.chr_rom[addr as usize & 0x1FFF];
        }

        // Use chr_mode to determine banking
        let bank = match self.chr_mode {
            0 => {
                // 8KB mode: one bank for all
                let b = self.chr_bank[7] as usize * 8;
                b + (addr as usize / 0x0400)
            }
            1 => {
                // 4KB mode
                match addr {
                    0x0000..=0x0FFF => (self.chr_bank[3] as usize * 4) + (addr as usize / 0x0400),
                    _ => (self.chr_bank[7] as usize * 4) + ((addr as usize - 0x1000) / 0x0400),
                }
            }
            2 => {
                // 2KB mode
                match addr {
                    0x0000..=0x07FF => (self.chr_bank[1] as usize * 2) + (addr as usize / 0x0400),
                    0x0800..=0x0FFF => (self.chr_bank[3] as usize * 2) + ((addr as usize - 0x0800) / 0x0400),
                    0x1000..=0x17FF => (self.chr_bank[5] as usize * 2) + ((addr as usize - 0x1000) / 0x0400),
                    _ => (self.chr_bank[7] as usize * 2) + ((addr as usize - 0x1800) / 0x0400),
                }
            }
            3 | _ => {
                // 1KB mode
                let idx = (addr / 0x0400) as usize;
                self.chr_bank[idx.min(7)] as usize
            }
        };

        let bank = bank % chr_banks_1k;
        let offset = bank * 0x0400 + (addr & 0x03FF) as usize;
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }

    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        // CHR RAM writes
        if self.chr_rom.len() <= 0x2000 {
            self.chr_rom[addr as usize & 0x1FFF] = data;
        }
    }

    #[inline]
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        // MMC5 nametable mapping is complex; simplify to basic modes
        match self.nametable_mapping & 0x03 {
            0 => crate::cartridge::Mirroring::SingleScreenLower,
            1 => crate::cartridge::Mirroring::SingleScreenUpper,
            2 => crate::cartridge::Mirroring::Vertical,
            3 => crate::cartridge::Mirroring::Horizontal,
            _ => crate::cartridge::Mirroring::Vertical,
        }
    }

    fn clock_scanline(&mut self) {
        if !self.in_frame {
            self.in_frame = true;
            self.scanline_counter = 0;
        }
        self.scanline_counter += 1;
        if self.scanline_counter == self.irq_target && self.irq_enabled {
            self.irq_pending_flag = true;
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
        let mut s = Vec::new();
        s.push(self.prg_mode);
        s.push(self.chr_mode);
        s.extend_from_slice(&self.prg_bank);
        s.push(self.prg_ram_protect1);
        s.push(self.prg_ram_protect2);
        for &b in &self.chr_bank { s.extend_from_slice(&b.to_le_bytes()); }
        s.push(self.chr_upper);
        s.push(self.nametable_mapping);
        s.push(self.multiplicand);
        s.push(self.multiplier);
        s.push(self.fill_tile);
        s.push(self.fill_attr);
        s.push(self.irq_target);
        s.push(if self.irq_enabled { 1 } else { 0 });
        s.push(if self.irq_pending_flag { 1 } else { 0 });
        s.push(self.scanline_counter);
        s.push(if self.in_frame { 1 } else { 0 });
        s.push(self.ex_ram_mode);
        s.extend_from_slice(&self.ex_ram);
        s
    }

    fn load_state(&mut self, data: &[u8]) {
        if data.len() < 45 { return; }
        let mut p = 0;
        self.prg_mode = data[p]; p += 1;
        self.chr_mode = data[p]; p += 1;
        self.prg_bank.copy_from_slice(&data[p..p+5]); p += 5;
        self.prg_ram_protect1 = data[p]; p += 1;
        self.prg_ram_protect2 = data[p]; p += 1;
        for i in 0..12 {
            self.chr_bank[i] = u16::from_le_bytes([data[p], data[p+1]]); p += 2;
        }
        self.chr_upper = data[p]; p += 1;
        self.nametable_mapping = data[p]; p += 1;
        self.multiplicand = data[p]; p += 1;
        self.multiplier = data[p]; p += 1;
        self.fill_tile = data[p]; p += 1;
        self.fill_attr = data[p]; p += 1;
        self.irq_target = data[p]; p += 1;
        self.irq_enabled = data[p] != 0; p += 1;
        self.irq_pending_flag = data[p] != 0; p += 1;
        self.scanline_counter = data[p]; p += 1;
        self.in_frame = data[p] != 0; p += 1;
        self.ex_ram_mode = data[p]; p += 1;
        if data.len() >= p + 0x0400 {
            self.ex_ram.copy_from_slice(&data[p..p+0x0400]);
        }
    }
}

// === Mapper 19: Namco 163 ===
// PRG: 4 × 8KB switchable banks
// CHR: 8 × 1KB banks (values >= $E0 can map to internal RAM)
// IRQ: 15-bit counter at $5000-$5FFF
// Internal sound RAM: 128 bytes at $4800 (data) / $F800 (address)

#[allow(dead_code)]
pub struct Mapper019 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    prg_banks_8k: usize,

    // CHR bank registers (8 × 1KB)
    chr_banks: [u8; 8],

    // PRG bank registers (4 × 8KB)
    prg_bank: [u8; 4],

    // Mirroring
    mirror_mode: u8,

    // IRQ
    irq_counter: u16,
    irq_enabled: bool,
    irq_pending_flag: bool,

    // Internal sound RAM (128 bytes)
    sound_ram: Vec<u8>,
    sound_addr: Cell<u8>,
    sound_auto_inc: bool,

    // CHR RAM for nametable replacement
    chr_ram: Vec<u8>,
}

impl Mapper019 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks_8k = prg_rom.len() / 0x2000;
        let has_chr_ram = chr_rom.is_empty();
        Mapper019 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x2000],
            prg_banks_8k: prg_banks_8k.max(1),
            chr_banks: [0; 8],
            prg_bank: [0, 0, 0, (prg_banks_8k - 1) as u8],
            mirror_mode: if mirroring == crate::cartridge::Mirroring::Vertical { 0 } else { 1 },
            irq_counter: 0,
            irq_enabled: false,
            irq_pending_flag: false,
            sound_ram: vec![0; 128],
            sound_addr: Cell::new(0),
            sound_auto_inc: false,
            chr_ram: vec![0; 0x2000],
        }
    }
}

impl Mapper for Mapper019 {
    #[inline]
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            // $4800-$4FFF: Sound RAM data port
            0x4800..=0x4FFF => {
                let addr = self.sound_addr.get();
                let val = self.sound_ram[(addr & 0x7F) as usize];
                if self.sound_auto_inc {
                    self.sound_addr.set((addr.wrapping_add(1)) & 0x7F);
                }
                val
            }

            // $5000-$5FFF: IRQ counter reads
            0x5000..=0x57FF => {
                (self.irq_counter & 0xFF) as u8
            }
            0x5800..=0x5FFF => {
                ((self.irq_counter >> 8) & 0x7F) as u8 | if self.irq_enabled { 0x80 } else { 0 }
            }

            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],

            0x8000..=0x9FFF => {
                let bank = (self.prg_bank[0] as usize & 0x3F) % self.prg_banks_8k;
                self.prg_rom[bank * 0x2000 + (addr - 0x8000) as usize]
            }
            0xA000..=0xBFFF => {
                let bank = (self.prg_bank[1] as usize & 0x3F) % self.prg_banks_8k;
                self.prg_rom[bank * 0x2000 + (addr - 0xA000) as usize]
            }
            0xC000..=0xDFFF => {
                let bank = (self.prg_bank[2] as usize & 0x3F) % self.prg_banks_8k;
                self.prg_rom[bank * 0x2000 + (addr - 0xC000) as usize]
            }
            0xE000..=0xFFFF => {
                let bank = (self.prg_bank[3] as usize & 0x3F) % self.prg_banks_8k;
                self.prg_rom[bank * 0x2000 + (addr - 0xE000) as usize]
            }
            _ => 0,
        }
    }

    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        match addr {
            // $4800-$4FFF: Sound RAM data port
            0x4800..=0x4FFF => {
                self.sound_ram[(self.sound_addr.get() & 0x7F) as usize] = data;
                if self.sound_auto_inc {
                    self.sound_addr.set((self.sound_addr.get().wrapping_add(1)) & 0x7F);
                }
            }

            // $5000-$57FF: IRQ counter low
            0x5000..=0x57FF => {
                self.irq_counter = (self.irq_counter & 0xFF00) | data as u16;
                self.irq_pending_flag = false;
            }
            // $5800-$5FFF: IRQ counter high + enable
            0x5800..=0x5FFF => {
                self.irq_counter = (self.irq_counter & 0x00FF) | ((data as u16 & 0x7F) << 8);
                self.irq_enabled = data & 0x80 != 0;
                self.irq_pending_flag = false;
            }

            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,

            // CHR bank registers ($8000-$BFFF)
            0x8000..=0x87FF => self.chr_banks[0] = data,
            0x8800..=0x8FFF => self.chr_banks[1] = data,
            0x9000..=0x97FF => self.chr_banks[2] = data,
            0x9800..=0x9FFF => self.chr_banks[3] = data,
            0xA000..=0xA7FF => self.chr_banks[4] = data,
            0xA800..=0xAFFF => self.chr_banks[5] = data,
            0xB000..=0xB7FF => self.chr_banks[6] = data,
            0xB800..=0xBFFF => self.chr_banks[7] = data,

            // Nametable mirroring ($C000-$DFFF) - simplified
            0xC000..=0xC7FF => {} // NT 0
            0xC800..=0xCFFF => {} // NT 1
            0xD000..=0xD7FF => {} // NT 2
            0xD800..=0xDFFF => {} // NT 3

            // PRG bank registers ($E000-$FFFF)
            0xE000..=0xE7FF => self.prg_bank[0] = data & 0x3F,
            0xE800..=0xEFFF => self.prg_bank[1] = data & 0x3F,
            0xF000..=0xF7FF => self.prg_bank[2] = data & 0x3F,

            // $F800: Sound address port
            0xF800..=0xFFFF => {
                // Sound address port at $F800
                self.sound_addr.set(data & 0x7F);
                self.sound_auto_inc = data & 0x80 != 0;
                // Last PRG bank is fixed to last
                self.prg_bank[3] = (self.prg_banks_8k - 1) as u8;
            }

            _ => {}
        }
    }

    #[inline]
    fn read_chr(&self, addr: u16) -> u8 {
        let bank_idx = (addr / 0x0400) as usize;
        if bank_idx >= 8 { return 0; }
        let bank_val = self.chr_banks[bank_idx];

        // If bank value >= 0xE0, use internal CHR RAM (nametable area)
        if bank_val >= 0xE0 {
            let ram_offset = ((bank_val as usize - 0xE0) * 0x0400 + (addr & 0x03FF) as usize) % self.chr_ram.len();
            return self.chr_ram[ram_offset];
        }

        let chr_banks_1k = self.chr_rom.len() / 0x0400;
        if chr_banks_1k == 0 { return 0; }
        let bank = (bank_val as usize) % chr_banks_1k;
        let offset = bank * 0x0400 + (addr & 0x03FF) as usize;
        if offset < self.chr_rom.len() { self.chr_rom[offset] } else { 0 }
    }

    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        let bank_idx = (addr / 0x0400) as usize;
        if bank_idx >= 8 { return; }
        let bank_val = self.chr_banks[bank_idx];

        if bank_val >= 0xE0 {
            let ram_offset = ((bank_val as usize - 0xE0) * 0x0400 + (addr & 0x03FF) as usize) % self.chr_ram.len();
            self.chr_ram[ram_offset] = data;
        } else if self.chr_rom.len() <= 0x2000 {
            self.chr_rom[addr as usize & 0x1FFF] = data;
        }
    }

    #[inline]
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        match self.mirror_mode {
            0 => crate::cartridge::Mirroring::Vertical,
            1 => crate::cartridge::Mirroring::Horizontal,
            2 => crate::cartridge::Mirroring::SingleScreenLower,
            3 => crate::cartridge::Mirroring::SingleScreenUpper,
            _ => crate::cartridge::Mirroring::Vertical,
        }
    }

    fn clock_scanline(&mut self) {
        if self.irq_enabled {
            if self.irq_counter >= 0x7FFF {
                self.irq_pending_flag = true;
            } else {
                // Increment by ~113 CPU cycles per scanline (approximate)
                self.irq_counter = self.irq_counter.saturating_add(113);
                if self.irq_counter >= 0x7FFF {
                    self.irq_counter = 0x7FFF;
                    self.irq_pending_flag = true;
                }
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
        let mut s = Vec::new();
        s.extend_from_slice(&self.chr_banks);
        s.extend_from_slice(&self.prg_bank);
        s.push(self.mirror_mode);
        s.extend_from_slice(&self.irq_counter.to_le_bytes());
        s.push(if self.irq_enabled { 1 } else { 0 });
        s.push(if self.irq_pending_flag { 1 } else { 0 });
        s.push(self.sound_addr.get());
        s.push(if self.sound_auto_inc { 1 } else { 0 });
        s.extend_from_slice(&self.sound_ram);
        s
    }

    fn load_state(&mut self, data: &[u8]) {
        if data.len() < 18 { return; }
        let mut p = 0;
        self.chr_banks.copy_from_slice(&data[p..p+8]); p += 8;
        self.prg_bank.copy_from_slice(&data[p..p+4]); p += 4;
        self.mirror_mode = data[p]; p += 1;
        self.irq_counter = u16::from_le_bytes([data[p], data[p+1]]); p += 2;
        self.irq_enabled = data[p] != 0; p += 1;
        self.irq_pending_flag = data[p] != 0; p += 1;
        self.sound_addr.set(data[p]); p += 1;
        self.sound_auto_inc = data[p] != 0; p += 1;
        if data.len() >= p + 128 {
            self.sound_ram.copy_from_slice(&data[p..p+128]);
        }
    }
}

// === Mapper 85: Konami VRC7 ===
// PRG: 3 × 8KB switchable + 8KB fixed last
// CHR: 8 × 1KB banks
// IRQ: similar to VRC6
// Audio: FM synthesis (OPLL) - skipped for now

#[allow(dead_code)]
pub struct Mapper085 {
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    prg_banks_8k: usize,

    // PRG banks
    prg_bank: [u8; 3],

    // CHR banks (8 × 1KB)
    chr_banks: [u8; 8],

    // Mirroring
    mirror_mode: u8,

    // IRQ
    irq_latch: u8,
    irq_counter: u8,
    irq_prescaler: i16,
    irq_enabled: bool,
    irq_cycle_mode: bool,
    irq_pending_flag: bool,

    // Variant: VRC7a vs VRC7b address line differences
    // VRC7 uses $x010 and $x018 (or $x008 and $x010)
    // We support both common wiring variants
    addr_mask: u16,
}

impl Mapper085 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: crate::cartridge::Mirroring) -> Self {
        let prg_banks_8k = prg_rom.len() / 0x2000;
        let has_chr_ram = chr_rom.is_empty();
        Mapper085 {
            prg_rom,
            chr_rom: if has_chr_ram { vec![0; 0x2000] } else { chr_rom },
            prg_ram: vec![0; 0x2000],
            prg_banks_8k: prg_banks_8k.max(1),
            prg_bank: [0; 3],
            chr_banks: [0; 8],
            mirror_mode: if mirroring == crate::cartridge::Mirroring::Vertical { 0 } else { 1 },
            irq_latch: 0, irq_counter: 0, irq_prescaler: 341,
            irq_enabled: false, irq_cycle_mode: false, irq_pending_flag: false,
            addr_mask: 0x0018, // Common VRC7 wiring: A3 and A4
        }
    }

    fn translate_addr(&self, addr: u16) -> u16 {
        if addr < 0x8000 { return addr; }
        // Normalize to $x000/$x008/$x010/$x018 pattern
        let base = addr & 0xF000;
        let low = addr & self.addr_mask;
        base | low
    }

    fn clock_irq(&mut self) {
        if self.irq_counter == 0xFF {
            self.irq_counter = self.irq_latch;
            self.irq_pending_flag = true;
        } else {
            self.irq_counter += 1;
        }
    }
}

impl Mapper for Mapper085 {
    #[inline]
    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize],
            0x8000..=0x9FFF => {
                let bank = (self.prg_bank[0] as usize) % self.prg_banks_8k;
                self.prg_rom[bank * 0x2000 + (addr - 0x8000) as usize]
            }
            0xA000..=0xBFFF => {
                let bank = (self.prg_bank[1] as usize) % self.prg_banks_8k;
                self.prg_rom[bank * 0x2000 + (addr - 0xA000) as usize]
            }
            0xC000..=0xDFFF => {
                let bank = (self.prg_bank[2] as usize) % self.prg_banks_8k;
                self.prg_rom[bank * 0x2000 + (addr - 0xC000) as usize]
            }
            0xE000..=0xFFFF => {
                let bank = self.prg_banks_8k - 1;
                self.prg_rom[bank * 0x2000 + (addr - 0xE000) as usize]
            }
            _ => 0,
        }
    }

    #[inline]
    fn write_prg(&mut self, addr: u16, data: u8) {
        let addr = self.translate_addr(addr);
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr - 0x6000) as usize] = data,

            // $8000: PRG bank 0
            0x8000 => self.prg_bank[0] = data & 0x3F,
            // $8008/$8010: PRG bank 1
            0x8008 | 0x8010 => self.prg_bank[1] = data & 0x3F,
            // $9000: PRG bank 2
            0x9000 => self.prg_bank[2] = data & 0x3F,
            // $9008/$9010: Audio register select (FM - skip)
            0x9008 | 0x9010 => {}
            // $9018: Audio data (FM - skip)
            0x9018 => {}

            // CHR banks
            0xA000 => self.chr_banks[0] = data,
            0xA008 | 0xA010 => self.chr_banks[1] = data,
            0xB000 => self.chr_banks[2] = data,
            0xB008 | 0xB010 => self.chr_banks[3] = data,
            0xC000 => self.chr_banks[4] = data,
            0xC008 | 0xC010 => self.chr_banks[5] = data,
            0xD000 => self.chr_banks[6] = data,
            0xD008 | 0xD010 => self.chr_banks[7] = data,

            // $E000: Mirroring
            0xE000 => self.mirror_mode = data & 0x03,

            // $E008/$E010: IRQ latch
            0xE008 | 0xE010 => self.irq_latch = data,

            // $F000: IRQ control
            0xF000 => {
                self.irq_cycle_mode = data & 0x04 != 0;
                self.irq_enabled = data & 0x02 != 0;
                if self.irq_enabled {
                    self.irq_counter = self.irq_latch;
                    self.irq_prescaler = 341;
                }
                self.irq_pending_flag = false;
            }

            // $F008/$F010: IRQ acknowledge
            0xF008 | 0xF010 => {
                self.irq_pending_flag = false;
            }

            _ => {}
        }
    }

    #[inline]
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

    #[inline]
    fn write_chr(&mut self, addr: u16, data: u8) {
        if self.chr_rom.len() <= 0x2000 {
            self.chr_rom[addr as usize & 0x1FFF] = data;
        }
    }

    #[inline]
    fn mirroring(&self) -> crate::cartridge::Mirroring {
        match self.mirror_mode {
            0 => crate::cartridge::Mirroring::Vertical,
            1 => crate::cartridge::Mirroring::Horizontal,
            2 => crate::cartridge::Mirroring::SingleScreenLower,
            3 => crate::cartridge::Mirroring::SingleScreenUpper,
            _ => crate::cartridge::Mirroring::Vertical,
        }
    }

    fn clock_scanline(&mut self) {
        if !self.irq_enabled { return; }

        if self.irq_cycle_mode {
            self.clock_irq();
        } else {
            self.irq_prescaler -= 3;
            if self.irq_prescaler <= 0 {
                self.irq_prescaler += 341;
                self.clock_irq();
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
        let mut s = Vec::new();
        s.extend_from_slice(&self.prg_bank);
        s.extend_from_slice(&self.chr_banks);
        s.push(self.mirror_mode);
        s.push(self.irq_latch);
        s.push(self.irq_counter);
        s.extend_from_slice(&self.irq_prescaler.to_le_bytes());
        s.push(if self.irq_enabled { 1 } else { 0 });
        s.push(if self.irq_cycle_mode { 1 } else { 0 });
        s.push(if self.irq_pending_flag { 1 } else { 0 });
        s
    }

    fn load_state(&mut self, data: &[u8]) {
        if data.len() < 18 { return; }
        let mut p = 0;
        self.prg_bank.copy_from_slice(&data[p..p+3]); p += 3;
        self.chr_banks.copy_from_slice(&data[p..p+8]); p += 8;
        self.mirror_mode = data[p]; p += 1;
        self.irq_latch = data[p]; p += 1;
        self.irq_counter = data[p]; p += 1;
        self.irq_prescaler = i16::from_le_bytes([data[p], data[p+1]]); p += 2;
        self.irq_enabled = data[p] != 0; p += 1;
        self.irq_cycle_mode = data[p] != 0; p += 1;
        self.irq_pending_flag = data[p] != 0;
    }
}
