use crate::apu::Apu;
use crate::cartridge::Cartridge;
use crate::joypad::Joypad;
use crate::ppu::Ppu;
use crate::state_io::StateReader;

#[derive(Clone)]
pub struct GameGenieCode {
    pub address: u16,
    pub replace: u8,
    pub compare: Option<u8>,
    pub enabled: bool,
    pub code_str: String,
}

impl GameGenieCode {
    pub fn decode(code: &str) -> Option<Self> {
        let letters = "APZLGITYEOXUKSVN";
        let code = code.to_uppercase();
        let vals: Vec<u8> = code
            .chars()
            .filter_map(|c| letters.find(c).map(|i| i as u8))
            .collect();

        if vals.len() == 6 {
            let address = 0x8000
                | ((vals[3] as u16 & 0x07) << 12)
                | ((vals[5] as u16 & 0x07) << 8)
                | ((vals[4] as u16 & 0x08) << 8)
                | ((vals[2] as u16 & 0x07) << 4)
                | ((vals[1] as u16 & 0x08) << 4)
                | (vals[4] as u16 & 0x07)
                | (vals[3] as u16 & 0x08);
            let replace = ((vals[1] & 0x07) << 4)
                | ((vals[0] & 0x08) << 4)
                | (vals[0] & 0x07)
                | (vals[5] & 0x08);
            Some(GameGenieCode {
                address,
                replace,
                compare: None,
                enabled: true,
                code_str: code.clone(),
            })
        } else if vals.len() == 8 {
            let address = 0x8000
                | ((vals[3] as u16 & 0x07) << 12)
                | ((vals[5] as u16 & 0x07) << 8)
                | ((vals[4] as u16 & 0x08) << 8)
                | ((vals[2] as u16 & 0x07) << 4)
                | ((vals[1] as u16 & 0x08) << 4)
                | (vals[4] as u16 & 0x07)
                | (vals[3] as u16 & 0x08);
            let replace = ((vals[1] & 0x07) << 4)
                | ((vals[0] & 0x08) << 4)
                | (vals[0] & 0x07)
                | (vals[7] & 0x08);
            let compare = ((vals[7] & 0x07) << 4)
                | ((vals[6] & 0x08) << 4)
                | (vals[6] & 0x07)
                | (vals[5] & 0x08);
            Some(GameGenieCode {
                address,
                replace,
                compare: Some(compare),
                enabled: true,
                code_str: code.clone(),
            })
        } else {
            None
        }
    }
}

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
    dmc_stall_cycles: u8,
    pub cheats: Vec<GameGenieCode>,
    has_enabled_cheats: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmcDmaService {
    pub address: u16,
    pub odd_cpu_cycle: bool,
    pub stall_cycles: u8,
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
            dmc_stall_cycles: 0,
            cheats: Vec::new(),
            has_enabled_cheats: false,
        }
    }

    /// Snapshot the first 2KB of CPU RAM (0x0000-0x07FF) for scripting.
    pub fn ram_snapshot(&self) -> Vec<u8> {
        self.cpu_ram.to_vec()
    }

    /// Recalculate the `has_enabled_cheats` cache after any cheat modification.
    pub fn update_cheats_cache(&mut self) {
        self.has_enabled_cheats = self.cheats.iter().any(|c| c.enabled);
    }

    #[inline(always)]
    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        // Game Genie interception - fast path when no cheats active
        if addr >= 0x8000 && self.has_enabled_cheats {
            for cheat in &self.cheats {
                if cheat.enabled && cheat.address == addr {
                    let original = self.cartridge.mapper.read_prg(addr);
                    match cheat.compare {
                        None => return cheat.replace,
                        Some(cmp) if cmp == original => return cheat.replace,
                        _ => {}
                    }
                }
            }
        }

        match addr {
            0x0000..=0x1FFF => self.cpu_ram[(addr & 0x07FF) as usize],
            0x2000..=0x3FFF => self.ppu.cpu_read(addr & 0x2007, &self.cartridge),
            0x4016 => self.joypad1.read(),
            0x4017 => self.joypad2.read(),
            0x4000..=0x4015 => self.apu.read(addr), // APU
            0x4018..=0x401F => 0,                   // APU test mode
            0x4020..=0xFFFF => self.cartridge.mapper.read_prg(addr),
        }
    }

    // Save state support - access methods for private fields
    pub fn get_sram(&self) -> Vec<u8> {
        self.cartridge.mapper.get_sram()
    }

    pub fn set_sram(&mut self, data: &[u8]) {
        self.cartridge.mapper.set_sram(data);
    }

    #[inline(always)]
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
            0x4018..=0x401F => {}                                            // APU test mode
            0x4020..=0xFFFF => self.cartridge.mapper.write_prg(addr, data),
        }
    }

    pub fn tick(&mut self, cpu_cycles: u8) -> bool {
        let mut nmi = false;
        for _ in 0..(cpu_cycles as usize * 3) {
            if self.tick_ppu_once() {
                nmi = true;
            }
        }
        nmi
    }

    pub fn tick_ppu_once(&mut self) -> bool {
        self.ppu.tick(&mut self.cartridge)
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
    #[inline]
    pub fn dma_active(&self) -> bool {
        self.dma_transfer
    }

    #[inline]
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
                self.ppu.mark_oam_dirty();
            }
        }
    }

    pub fn set_apu_sample_rate(&mut self, sample_rate: u32) {
        self.apu.set_sample_rate(sample_rate);
    }

    pub fn tick_apu(&mut self) {
        // Mix in mapper expansion audio (audio_output added by mapper.rs agent)
        self.apu.external_audio = self.cartridge.mapper.audio_output();
        self.apu.tick();
    }

    #[inline]
    pub fn service_dmc_dma(&mut self, odd_cpu_cycle: bool) -> Option<DmcDmaService> {
        if self.apu.dmc.dma_request {
            let addr = self.apu.dmc.dma_address;
            let data = self.cpu_read(addr);
            self.apu.dmc.receive_sample(data);
            let stall_cycles = if odd_cpu_cycle { 3 } else { 4 };
            self.dmc_stall_cycles = stall_cycles;
            Some(DmcDmaService {
                address: addr,
                odd_cpu_cycle,
                stall_cycles,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn dmc_stall_active(&self) -> bool {
        self.dmc_stall_cycles > 0
    }

    #[inline]
    pub fn dmc_stall_tick(&mut self) {
        self.dmc_stall_cycles -= 1;
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

        // APU state
        let apu_state = self.apu.save_state();
        data.extend_from_slice(&(apu_state.len() as u32).to_le_bytes());
        data.extend(apu_state);

        data
    }

    pub fn load_state(&mut self, data: &[u8]) -> bool {
        let mut reader = StateReader::new(data);

        let Some(cpu_ram) = reader.read_bytes(2048) else {
            return false;
        };

        // PPU state
        let Some(ppu_state) = reader.read_len_prefixed_u32() else {
            return false;
        };
        if !self.ppu.load_state(ppu_state) {
            return false;
        }

        self.cpu_ram.copy_from_slice(cpu_ram);

        // Mapper SRAM
        if reader.remaining() == 0 {
            return true;
        }
        let Some(sram) = reader.read_len_prefixed_u32() else {
            return false;
        };
        self.cartridge.mapper.set_sram(sram);

        // Mapper state
        if reader.remaining() == 0 {
            return true;
        }
        let Some(mapper_state) = reader.read_len_prefixed_u32() else {
            return false;
        };
        self.cartridge.mapper.load_state(mapper_state);

        // Bus state
        if reader.remaining() == 0 {
            return true;
        }
        let Some(cycles) = reader.read_usize_le() else {
            return false;
        };
        let Some(dma_page) = reader.read_u8() else {
            return false;
        };
        let Some(dma_addr) = reader.read_u8() else {
            return false;
        };
        let Some(dma_data) = reader.read_u8() else {
            return false;
        };
        let Some(dma_transfer) = reader.read_bool() else {
            return false;
        };
        let Some(dma_dummy) = reader.read_bool() else {
            return false;
        };

        self.cycles = cycles;
        self.dma_page = dma_page;
        self.dma_addr = dma_addr;
        self.dma_data = dma_data;
        self.dma_transfer = dma_transfer;
        self.dma_dummy = dma_dummy;

        // APU state (optional - backwards compatible with old saves)
        if reader.remaining() == 0 {
            return true;
        }
        let Some(apu_state) = reader.read_len_prefixed_u32() else {
            return false;
        };
        if !self.apu.load_state(apu_state) {
            return false;
        }

        true
    }
}
