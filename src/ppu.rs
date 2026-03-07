use crate::cartridge::{Cartridge, Mirroring};

pub struct Ppu {
    pub oam_data: [u8; 256],
    vram: [u8; 2048],
    palette_table: [u8; 32],
    pub frame_data: Vec<u32>,
    // Internal registers
    ctrl: u8,
    mask: u8,
    status: u8,
    oam_addr: u8,
    scroll_x: u8,
    scroll_y: u8,
    addr_latch: bool,
    ppu_addr: u16,
    ppu_data_buffer: u8,
    // Rendering state
    scanline: i16,
    cycle: u16,
    frame_complete: bool,
    nmi_occurred: bool,
    nmi_output: bool,
    // Background rendering
    v: u16,    // current VRAM address
    t: u16,    // temporary VRAM address
    x: u8,     // fine X scroll
    w: bool,   // write toggle
    bg_next_tile_id: u8,
    bg_next_tile_attrib: u8,
    bg_next_tile_lsb: u8,
    bg_next_tile_msb: u8,
    bg_shifter_pattern_lo: u16,
    bg_shifter_pattern_hi: u16,
    bg_shifter_attrib_lo: u16,
    bg_shifter_attrib_hi: u16,
    // Sprite rendering
    sprite_scanline: [OamEntry; 8],
    sprite_count: u8,
    sprite_shifter_pattern_lo: [u8; 8],
    sprite_shifter_pattern_hi: [u8; 8],
    sprite_zero_hit_possible: bool,
    sprite_zero_being_rendered: bool,
    odd_frame: bool,
}

#[derive(Clone, Copy)]
struct OamEntry {
    y: u8,
    tile_id: u8,
    attribute: u8,
    x: u8,
}

impl Default for OamEntry {
    fn default() -> Self {
        OamEntry { y: 0xFF, tile_id: 0, attribute: 0, x: 0xFF }
    }
}

// NES system palette (RGB values)
const NES_PALETTE: [u32; 64] = [
    0x666666, 0x002A88, 0x1412A7, 0x3B00A4, 0x5C007E, 0x6E0040, 0x6C0600, 0x561D00,
    0x333500, 0x0B4800, 0x005200, 0x004F08, 0x00404D, 0x000000, 0x000000, 0x000000,
    0xADADAD, 0x155FD9, 0x4240FF, 0x7527FE, 0xA01ACC, 0xB71E7B, 0xB53120, 0x994E00,
    0x6B6D00, 0x388700, 0x0C9300, 0x008F32, 0x007C8D, 0x000000, 0x000000, 0x000000,
    0xFFFEFF, 0x64B0FF, 0x9290FF, 0xC676FF, 0xF36AFF, 0xFE6ECC, 0xFE8170, 0xEA9E22,
    0xBCBE00, 0x88D800, 0x5CE430, 0x45E082, 0x48CDDE, 0x4F4F4F, 0x000000, 0x000000,
    0xFFFEFF, 0xC0DFFF, 0xD3D2FF, 0xE8C8FF, 0xFBC2FF, 0xFEC4EA, 0xFECCC5, 0xF7D8A5,
    0xE4E594, 0xCFEF96, 0xBDF4AB, 0xB3F3CC, 0xB5EBF2, 0xB8B8B8, 0x000000, 0x000000,
];

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            oam_data: [0; 256],
            vram: [0; 2048],
            palette_table: [0; 32],
            frame_data: vec![0; 256 * 240],
            ctrl: 0,
            mask: 0,
            status: 0,
            oam_addr: 0,
            scroll_x: 0,
            scroll_y: 0,
            addr_latch: false,
            ppu_addr: 0,
            ppu_data_buffer: 0,
            scanline: 0,
            cycle: 0,
            frame_complete: false,
            nmi_occurred: false,
            nmi_output: false,
            v: 0,
            t: 0,
            x: 0,
            w: false,
            bg_next_tile_id: 0,
            bg_next_tile_attrib: 0,
            bg_next_tile_lsb: 0,
            bg_next_tile_msb: 0,
            bg_shifter_pattern_lo: 0,
            bg_shifter_pattern_hi: 0,
            bg_shifter_attrib_lo: 0,
            bg_shifter_attrib_hi: 0,
            sprite_scanline: [OamEntry::default(); 8],
            sprite_count: 0,
            sprite_shifter_pattern_lo: [0; 8],
            sprite_shifter_pattern_hi: [0; 8],
            sprite_zero_hit_possible: false,
            sprite_zero_being_rendered: false,
            odd_frame: false,
        }
    }

    fn mirror_vram_addr(mirroring: &Mirroring, addr: u16) -> u16 {
        let mirrored = addr & 0x2FFF;
        let vram_index = mirrored - 0x2000;
        let nametable = vram_index / 0x0400;
        match (mirroring, nametable) {
            (Mirroring::Vertical, 2) | (Mirroring::Vertical, 3) => vram_index - 0x800,
            (Mirroring::Horizontal, 1) | (Mirroring::Horizontal, 2) => vram_index - 0x400,
            (Mirroring::Horizontal, 3) => vram_index - 0x800,
            _ => vram_index,
        }
    }

    fn mirror_palette_addr(addr: u16) -> u16 {
        let addr = addr & 0x1F;
        // Mirrors of background color
        match addr {
            0x10 | 0x14 | 0x18 | 0x1C => addr - 0x10,
            _ => addr,
        }
    }

    fn ppu_read(&self, addr: u16, cart: &Cartridge) -> u8 {
        match addr {
            0x0000..=0x1FFF => cart.mapper.read_chr(addr),
            0x2000..=0x3EFF => {
                let mirroring = cart.mapper.mirroring();
                let idx = Self::mirror_vram_addr(&mirroring, addr) as usize;
                self.vram[idx]
            }
            0x3F00..=0x3FFF => {
                let idx = Self::mirror_palette_addr(addr) as usize;
                self.palette_table[idx]
            }
            _ => 0,
        }
    }

    fn ppu_write(&mut self, addr: u16, data: u8, cart: &mut Cartridge) {
        match addr {
            0x0000..=0x1FFF => cart.mapper.write_chr(addr, data),
            0x2000..=0x3EFF => {
                let mirroring = cart.mapper.mirroring();
                let idx = Self::mirror_vram_addr(&mirroring, addr) as usize;
                self.vram[idx] = data;
            }
            0x3F00..=0x3FFF => {
                let idx = Self::mirror_palette_addr(addr) as usize;
                self.palette_table[idx] = data;
            }
            _ => {}
        }
    }

    pub fn cpu_read(&mut self, addr: u16, cart: &Cartridge) -> u8 {
        match addr {
            0x2002 => {
                let data = (self.status & 0xE0) | (self.ppu_data_buffer & 0x1F);
                self.status &= !0x80; // clear vblank
                self.nmi_occurred = false;
                self.w = false;
                data
            }
            0x2004 => self.oam_data[self.oam_addr as usize],
            0x2007 => {
                let mut data = self.ppu_data_buffer;
                self.ppu_data_buffer = self.ppu_read(self.v, cart);
                if self.v >= 0x3F00 {
                    data = self.ppu_data_buffer;
                    // Read from nametable "underneath" the palette
                    self.ppu_data_buffer = self.ppu_read(self.v - 0x1000, cart);
                }
                self.v = self.v.wrapping_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
                data
            }
            _ => 0,
        }
    }

    pub fn cpu_write(&mut self, addr: u16, data: u8, cart: &mut Cartridge) {
        match addr {
            0x2000 => {
                self.ctrl = data;
                self.nmi_output = data & 0x80 != 0;
                self.t = (self.t & 0xF3FF) | (((data as u16) & 0x03) << 10);
                if self.nmi_output && self.nmi_occurred {
                    // Trigger NMI if in vblank
                }
            }
            0x2001 => self.mask = data,
            0x2003 => self.oam_addr = data,
            0x2004 => {
                self.oam_data[self.oam_addr as usize] = data;
                self.oam_addr = self.oam_addr.wrapping_add(1);
            }
            0x2005 => {
                if !self.w {
                    self.x = data & 0x07;
                    self.t = (self.t & 0xFFE0) | ((data as u16) >> 3);
                    self.w = true;
                } else {
                    self.t = (self.t & 0x8C1F) | (((data as u16) & 0x07) << 12)
                        | (((data as u16) & 0xF8) << 2);
                    self.w = false;
                }
            }
            0x2006 => {
                if !self.w {
                    self.t = (self.t & 0x00FF) | (((data as u16) & 0x3F) << 8);
                    self.w = true;
                } else {
                    self.t = (self.t & 0xFF00) | data as u16;
                    self.v = self.t;
                    self.w = false;
                }
            }
            0x2007 => {
                let addr = self.v;
                self.ppu_write(addr, data, cart);
                self.v = self.v.wrapping_add(if self.ctrl & 0x04 != 0 { 32 } else { 1 });
            }
            _ => {}
        }
    }

    fn rendering_enabled(&self) -> bool {
        self.mask & 0x18 != 0
    }

    fn increment_scroll_x(&mut self) {
        if !self.rendering_enabled() { return; }
        if self.v & 0x001F == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    fn increment_scroll_y(&mut self) {
        if !self.rendering_enabled() { return; }
        if self.v & 0x7000 != 0x7000 {
            self.v += 0x1000;
        } else {
            self.v &= !0x7000;
            let mut y = (self.v & 0x03E0) >> 5;
            if y == 29 {
                y = 0;
                self.v ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }
            self.v = (self.v & !0x03E0) | (y << 5);
        }
    }

    fn transfer_address_x(&mut self) {
        if !self.rendering_enabled() { return; }
        self.v = (self.v & !0x041F) | (self.t & 0x041F);
    }

    fn transfer_address_y(&mut self) {
        if !self.rendering_enabled() { return; }
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
    }

    fn load_background_shifters(&mut self) {
        self.bg_shifter_pattern_lo = (self.bg_shifter_pattern_lo & 0xFF00) | self.bg_next_tile_lsb as u16;
        self.bg_shifter_pattern_hi = (self.bg_shifter_pattern_hi & 0xFF00) | self.bg_next_tile_msb as u16;
        self.bg_shifter_attrib_lo = (self.bg_shifter_attrib_lo & 0xFF00) | if self.bg_next_tile_attrib & 0x01 != 0 { 0xFF } else { 0x00 };
        self.bg_shifter_attrib_hi = (self.bg_shifter_attrib_hi & 0xFF00) | if self.bg_next_tile_attrib & 0x02 != 0 { 0xFF } else { 0x00 };
    }

    fn update_shifters(&mut self) {
        if self.mask & 0x08 != 0 {
            self.bg_shifter_pattern_lo <<= 1;
            self.bg_shifter_pattern_hi <<= 1;
            self.bg_shifter_attrib_lo <<= 1;
            self.bg_shifter_attrib_hi <<= 1;
        }
        if self.mask & 0x10 != 0 && self.cycle >= 1 && self.cycle < 258 {
            for i in 0..self.sprite_count as usize {
                if self.sprite_scanline[i].x > 0 {
                    self.sprite_scanline[i].x -= 1;
                } else {
                    self.sprite_shifter_pattern_lo[i] <<= 1;
                    self.sprite_shifter_pattern_hi[i] <<= 1;
                }
            }
        }
    }

    pub fn tick(&mut self, cart: &Cartridge) -> bool {
        let mut trigger_nmi = false;

        if self.scanline >= -1 && self.scanline < 240 {
            if self.scanline == 0 && self.cycle == 0 && self.odd_frame && self.rendering_enabled() {
                self.cycle = 1;
            }

            if self.scanline == -1 && self.cycle == 1 {
                self.status &= !0x80; // clear vblank
                self.nmi_occurred = false;
                self.status &= !0x40; // clear sprite overflow
                self.status &= !0x20; // clear sprite zero hit
                self.sprite_shifter_pattern_lo = [0; 8];
                self.sprite_shifter_pattern_hi = [0; 8];
            }

            if (self.cycle >= 2 && self.cycle < 258) || (self.cycle >= 321 && self.cycle < 338) {
                self.update_shifters();

                match (self.cycle - 1) % 8 {
                    0 => {
                        self.load_background_shifters();
                        let addr = 0x2000 | (self.v & 0x0FFF);
                        self.bg_next_tile_id = self.ppu_read(addr, cart);
                    }
                    2 => {
                        let addr = 0x23C0
                            | (self.v & 0x0C00)
                            | ((self.v >> 4) & 0x38)
                            | ((self.v >> 2) & 0x07);
                        self.bg_next_tile_attrib = self.ppu_read(addr, cart);
                        if self.v & 0x40 != 0 { self.bg_next_tile_attrib >>= 4; }
                        if self.v & 0x02 != 0 { self.bg_next_tile_attrib >>= 2; }
                        self.bg_next_tile_attrib &= 0x03;
                    }
                    4 => {
                        let bg_pattern = if self.ctrl & 0x10 != 0 { 0x1000u16 } else { 0 };
                        let addr = bg_pattern
                            + (self.bg_next_tile_id as u16) * 16
                            + ((self.v >> 12) & 0x07);
                        self.bg_next_tile_lsb = self.ppu_read(addr, cart);
                    }
                    6 => {
                        let bg_pattern = if self.ctrl & 0x10 != 0 { 0x1000u16 } else { 0 };
                        let addr = bg_pattern
                            + (self.bg_next_tile_id as u16) * 16
                            + ((self.v >> 12) & 0x07) + 8;
                        self.bg_next_tile_msb = self.ppu_read(addr, cart);
                    }
                    7 => {
                        self.increment_scroll_x();
                    }
                    _ => {}
                }
            }

            if self.cycle == 256 {
                self.increment_scroll_y();
            }

            if self.cycle == 257 {
                self.load_background_shifters();
                self.transfer_address_x();
            }

            if self.scanline == -1 && self.cycle >= 280 && self.cycle < 305 {
                self.transfer_address_y();
            }

            // Sprite evaluation
            if self.cycle == 257 && self.scanline >= 0 {
                self.sprite_scanline = [OamEntry::default(); 8];
                self.sprite_count = 0;
                self.sprite_zero_hit_possible = false;

                let sprite_size: i16 = if self.ctrl & 0x20 != 0 { 16 } else { 8 };

                for i in 0..64usize {
                    let oam_y = self.oam_data[i * 4] as i16;
                    let diff = self.scanline - oam_y;

                    if diff >= 0 && diff < sprite_size && self.sprite_count < 8 {
                        if i == 0 {
                            self.sprite_zero_hit_possible = true;
                        }
                        let entry = OamEntry {
                            y: self.oam_data[i * 4],
                            tile_id: self.oam_data[i * 4 + 1],
                            attribute: self.oam_data[i * 4 + 2],
                            x: self.oam_data[i * 4 + 3],
                        };
                        self.sprite_scanline[self.sprite_count as usize] = entry;
                        self.sprite_count += 1;
                    }
                }
                if self.sprite_count > 8 {
                    self.sprite_count = 8;
                    self.status |= 0x20; // sprite overflow
                }
            }

            if self.cycle == 340 && self.scanline >= 0 {
                for i in 0..self.sprite_count as usize {
                    let sprite = &self.sprite_scanline[i];
                    let sprite_pattern_addr: u16;

                    if self.ctrl & 0x20 == 0 {
                        // 8x8 sprites
                        let sprite_table = if self.ctrl & 0x08 != 0 { 0x1000u16 } else { 0 };
                        let row = if sprite.attribute & 0x80 != 0 {
                            7 - (self.scanline as u16 - sprite.y as u16)
                        } else {
                            self.scanline as u16 - sprite.y as u16
                        };
                        sprite_pattern_addr = sprite_table + (sprite.tile_id as u16) * 16 + row;
                    } else {
                        // 8x16 sprites
                        let row = if sprite.attribute & 0x80 != 0 {
                            15 - (self.scanline as u16 - sprite.y as u16)
                        } else {
                            self.scanline as u16 - sprite.y as u16
                        };
                        let table = (sprite.tile_id as u16 & 0x01) * 0x1000;
                        let tile = sprite.tile_id as u16 & 0xFE;
                        if row < 8 {
                            sprite_pattern_addr = table + tile * 16 + row;
                        } else {
                            sprite_pattern_addr = table + (tile + 1) * 16 + (row - 8);
                        }
                    };

                    let mut lo = self.ppu_read(sprite_pattern_addr, cart);
                    let mut hi = self.ppu_read(sprite_pattern_addr + 8, cart);

                    // Flip horizontally
                    if sprite.attribute & 0x40 != 0 {
                        lo = lo.reverse_bits();
                        hi = hi.reverse_bits();
                    }

                    self.sprite_shifter_pattern_lo[i] = lo;
                    self.sprite_shifter_pattern_hi[i] = hi;
                }
            }
        }

        // Visible pixel output
        if self.scanline >= 0 && self.scanline < 240 && self.cycle >= 1 && self.cycle <= 256 {
            // Background pixel
            let mut bg_pixel: u8 = 0;
            let mut bg_palette: u8 = 0;

            if self.mask & 0x08 != 0 {
                if self.mask & 0x02 != 0 || self.cycle > 8 {
                    let mux = 0x8000 >> self.x;
                    let p0 = if self.bg_shifter_pattern_lo & mux != 0 { 1 } else { 0 };
                    let p1 = if self.bg_shifter_pattern_hi & mux != 0 { 1 } else { 0 };
                    bg_pixel = (p1 << 1) | p0;

                    let a0 = if self.bg_shifter_attrib_lo & mux != 0 { 1 } else { 0 };
                    let a1 = if self.bg_shifter_attrib_hi & mux != 0 { 1 } else { 0 };
                    bg_palette = (a1 << 1) | a0;
                }
            }

            // Sprite pixel
            let mut fg_pixel: u8 = 0;
            let mut fg_palette: u8 = 0;
            let mut fg_priority: bool = false;
            self.sprite_zero_being_rendered = false;

            if self.mask & 0x10 != 0 {
                if self.mask & 0x04 != 0 || self.cycle > 8 {
                    for i in 0..self.sprite_count as usize {
                        if self.sprite_scanline[i].x == 0 {
                            let p0 = if self.sprite_shifter_pattern_lo[i] & 0x80 != 0 { 1 } else { 0 };
                            let p1 = if self.sprite_shifter_pattern_hi[i] & 0x80 != 0 { 1 } else { 0 };
                            fg_pixel = (p1 << 1) | p0;
                            fg_palette = (self.sprite_scanline[i].attribute & 0x03) + 4;
                            fg_priority = self.sprite_scanline[i].attribute & 0x20 == 0;

                            if fg_pixel != 0 {
                                if i == 0 {
                                    self.sprite_zero_being_rendered = true;
                                }
                                break;
                            }
                        }
                    }
                }
            }

            // Priority multiplexer
            let (pixel, palette) = match (bg_pixel, fg_pixel) {
                (0, 0) => (0, 0),
                (0, _) => (fg_pixel, fg_palette),
                (_, 0) => (bg_pixel, bg_palette),
                (_, _) => {
                    // Sprite zero hit detection
                    if self.sprite_zero_hit_possible && self.sprite_zero_being_rendered {
                        if self.mask & 0x18 == 0x18 {
                            if !(self.mask & 0x06 == 0x06) {
                                if self.cycle >= 9 && self.cycle <= 256 {
                                    self.status |= 0x40;
                                }
                            } else if self.cycle >= 1 && self.cycle <= 256 {
                                self.status |= 0x40;
                            }
                        }
                    }
                    if fg_priority { (fg_pixel, fg_palette) } else { (bg_pixel, bg_palette) }
                }
            };

            let color_addr = 0x3F00 + (palette as u16) * 4 + pixel as u16;
            let color_index = self.ppu_read(color_addr, cart) as usize & 0x3F;
            let color = NES_PALETTE[color_index];

            let x = (self.cycle - 1) as usize;
            let y = self.scanline as usize;
            if x < 256 && y < 240 {
                self.frame_data[y * 256 + x] = color;
            }
        }

        if self.scanline == 241 && self.cycle == 1 {
            self.status |= 0x80;
            self.nmi_occurred = true;
            if self.nmi_output {
                trigger_nmi = true;
            }
        }

        self.cycle += 1;
        if self.cycle >= 341 {
            self.cycle = 0;
            self.scanline += 1;
            if self.scanline >= 261 {
                self.scanline = -1;
                self.frame_complete = true;
                self.odd_frame = !self.odd_frame;
            }
        }

        trigger_nmi
    }

    pub fn poll_nmi(&mut self) -> bool {
        let nmi = self.nmi_occurred && self.nmi_output;
        if nmi {
            self.nmi_occurred = false;
        }
        nmi
    }

    pub fn frame_complete(&mut self) -> bool {
        let complete = self.frame_complete;
        self.frame_complete = false;
        complete
    }
}
