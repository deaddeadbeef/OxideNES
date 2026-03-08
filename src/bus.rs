use crate::cartridge::Cartridge;
use crate::ppu::Ppu;
use crate::joypad::Joypad;
use crate::apu::Apu;

pub struct Bus {
    cpu_ram: [u8; 2048],
    pub ppu: Ppu,
    pub cartridge: Cartridge,
    pub joypad1: Joypad,
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
            0x4017 => 0, // joypad 2 stub
            0x4000..=0x4015 => self.apu.read(addr), // APU
            0x4018..=0x401F => 0, // APU test mode
            0x4020..=0xFFFF => self.cartridge.mapper.read_prg(addr),
            _ => 0,
        }
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
            0x4016 => self.joypad1.write(data),
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
        self.apu.irq_pending
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
}
