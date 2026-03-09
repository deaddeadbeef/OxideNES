use crate::cartridge::Cartridge;
use crate::ppu::Ppu;
use crate::joypad::Joypad;
use crate::apu::Apu;

pub struct Bus {
    cpu_ram: [u8; 2048],
    pub ppu: Ppu,
    pub cartridge: Cartridge,
    pub joypad1: Joypad,
    pub joypad2: Joypad,
    pub apu: Apu,
    cycles: usize,
    dma_page: u8,
    dma_addr: u8,
    dma_data: u8,
    dma_transfer: bool,
    dma_dummy: bool,
}

impl Bus {
    pub fn new(cartridge: Cartridge) -> Self {
        Bus {
            cpu_ram: [0; 2048],
            ppu: Ppu::new(),
            cartridge,
            joypad1: Joypad::new(),
            joypad2: Joypad::new(),
            apu: Apu::new(44100),
            cycles: 0,
            dma_page: 0,
            dma_addr: 0,
            dma_data: 0,
            dma_transfer: false,
            dma_dummy: true,
        }
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => self.ppu.cpu_read(addr & 0x2007, &self.cartridge),
            0x4016 => self.joypad1.read(),
            0x4017 => self.joypad2.read(),
            0x4000..=0x4015 => self.apu.read(addr), // APU
            0x4018..=0x401F => 0, // APU test mode
            0x4020..=0xFFFF => self.cartridge.mapper.read_prg(addr),
            _ => 0,
        }
    }

    // Save state support - access methods for private fields
    pub fn get_sram(&self) -> Vec<u8> {
        self.cartridge.mapper.get_sram()
    }

    pub fn set_sram(&mut self, data: &[u8]) {
        self.cartridge.mapper.set_sram(data);
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0x07FF) as usize] = data,
            0x2000..=0x3FFF => self.ppu.cpu_write(addr & 0x2007, data, &mut self.cartridge),
            0x4014 => {
                self.dma_page = data;
                self.dma_addr = 0;
                self.dma_transfer = true;
                self.dma_dummy = true;
            }
            0x4016 => {
                self.joypad1.write(data);
                self.joypad2.write(data);
            }
            0x4000..=0x4013 | 0x4015 | 0x4017 => self.apu.write(addr, data), // APU
            0x4018..=0x401F => {} // APU test mode
            0x4020..=0xFFFF => self.cartridge.mapper.write_prg(addr, data),
            _ => {}
        }
    }

    pub fn tick(&mut self, cpu_cycles: u8) -> bool {
        let mut nmi = false;
        for _ in 0..(cpu_cycles as usize * 3) {
            if self.ppu.tick(&mut self.cartridge) {
                nmi = true;
            }
        }
        nmi
    }

    pub fn poll_nmi(&mut self) -> bool {
        self.ppu.poll_nmi()
    }

    pub fn poll_apu_irq(&self) -> bool {
        self.apu.irq_pending || self.apu.dmc.irq_pending
    }

    pub fn poll_mapper_irq(&mut self) -> bool {
        let pending = self.cartridge.mapper.irq_pending();
        if pending {
            self.cartridge.mapper.irq_clear();
        }
        pending
    }

    // DMA transfer handling
    pub fn dma_active(&self) -> bool {
        self.dma_transfer
    }

    pub fn dma_tick(&mut self, odd_cycle: bool) {
        if self.dma_dummy {
            if odd_cycle {
                self.dma_dummy = false;
            }
        } else if !odd_cycle {
            // Read cycle
            let addr = (self.dma_page as u16) << 8 | self.dma_addr as u16;
            self.dma_data = self.cpu_read(addr);
        } else {
            // Write cycle
            self.ppu.oam_data[self.dma_addr as usize] = self.dma_data;
            self.dma_addr = self.dma_addr.wrapping_add(1);
            if self.dma_addr == 0 {
                self.dma_transfer = false;
                self.dma_dummy = true;
            }
        }
    }

    pub fn set_apu_sample_rate(&mut self, sample_rate: u32) {
        self.apu.set_sample_rate(sample_rate);
    }

    pub fn tick_apu(&mut self) {
        self.apu.tick();
    }
    
    pub fn service_dmc_dma(&mut self) {
        if self.apu.dmc.dma_request {
            let addr = self.apu.dmc.dma_address;
            let data = self.cpu_read(addr);
            self.apu.dmc.receive_sample(data);
        }
    }
    
    // ── Save state support ──────────────────────────────────────────
    pub fn save_state(&self) -> Vec<u8> {
        let mut data = Vec::new();
        
        // CPU RAM (2048 bytes)
        data.extend_from_slice(&self.cpu_ram);
        
        // PPU state
        let ppu_state = self.ppu.save_state();
        data.extend_from_slice(&(ppu_state.len() as u32).to_le_bytes());
        data.extend(ppu_state);
        
        // Mapper SRAM
        let sram = self.cartridge.mapper.get_sram();
        data.extend_from_slice(&(sram.len() as u32).to_le_bytes());
        data.extend(sram);
        
        // Mapper state
        let mapper_state = self.cartridge.mapper.save_state();
        data.extend_from_slice(&(mapper_state.len() as u32).to_le_bytes());
        data.extend(mapper_state);
        
        // Bus state
        data.extend_from_slice(&self.cycles.to_le_bytes());
        data.push(self.dma_page);
        data.push(self.dma_addr);
        data.push(self.dma_data);
        data.push(if self.dma_transfer { 1 } else { 0 });
        data.push(if self.dma_dummy { 1 } else { 0 });
        
        data
    }

    pub fn load_state(&mut self, data: &[u8]) -> bool {
        if data.len() < 2048 { return false; }
        let mut pos = 0;
        
        // CPU RAM
        self.cpu_ram.copy_from_slice(&data[pos..pos+2048]);
        pos += 2048;
        
        // PPU state
        if pos + 4 > data.len() { return false; }
        let ppu_len = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
        pos += 4;
        if pos + ppu_len > data.len() { return false; }
        if !self.ppu.load_state(&data[pos..pos+ppu_len]) { return false; }
        pos += ppu_len;
        
        // Mapper SRAM
        if pos + 4 > data.len() { return true; } // Optional sections
        let sram_len = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
        pos += 4;
        if pos + sram_len <= data.len() {
            self.cartridge.mapper.set_sram(&data[pos..pos+sram_len]);
            pos += sram_len;
        }
        
        // Mapper state
        if pos + 4 > data.len() { return true; }
        let mapper_len = u32::from_le_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
        pos += 4;
        if pos + mapper_len <= data.len() {
            self.cartridge.mapper.load_state(&data[pos..pos+mapper_len]);
            pos += mapper_len;
        }
        
        // Bus state  
        if pos + 13 <= data.len() {
            self.cycles = usize::from_le_bytes([
                data[pos], data[pos+1], data[pos+2], data[pos+3],
                data[pos+4], data[pos+5], data[pos+6], data[pos+7]
            ]);
            pos += 8;
            self.dma_page = data[pos]; pos += 1;
            self.dma_addr = data[pos]; pos += 1;
            self.dma_data = data[pos]; pos += 1;
            self.dma_transfer = data[pos] != 0; pos += 1;
            self.dma_dummy = data[pos] != 0;
        }
        
        true
    }
}
