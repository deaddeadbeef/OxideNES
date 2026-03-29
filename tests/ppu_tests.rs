use oxidenes::cartridge::Cartridge;
use oxidenes::ppu::Ppu;

fn make_rom(prg_banks: u8, chr_banks: u8, mapper: u8, flags: u8) -> Vec<u8> {
    let prg_size = prg_banks as usize * 16384;
    let chr_size = chr_banks as usize * 8192;
    let mut rom = vec![0u8; 16 + prg_size + chr_size];
    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = prg_banks;
    rom[5] = chr_banks;
    rom[6] = ((mapper & 0x0F) << 4) | (flags & 0x0F);
    rom[7] = mapper & 0xF0;
    for i in 0..prg_size {
        rom[16 + i] = (i & 0xFF) as u8;
    }
    for i in 0..chr_size {
        rom[16 + prg_size + i] = ((i >> 2) & 0xFF) as u8;
    }
    rom
}

fn make_cart() -> Cartridge {
    Cartridge::new(&make_rom(1, 1, 0, 0)).unwrap()
}

#[test]
fn ppu_initial_state() {
    let ppu = Ppu::new();
    assert_eq!(ppu.frame_data.len(), 256 * 240, "Frame buffer should be 256x240 pixels");
    assert_eq!(ppu.oam_data.len(), 256, "OAM should be 256 bytes");
}

#[test]
fn ppu_oam_write_read() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    // Set OAM address to 0x10, write 0xAB
    ppu.cpu_write(0x2003, 0x10, &mut cart);
    ppu.cpu_write(0x2004, 0xAB, &mut cart);
    // Verify via pub field
    assert_eq!(ppu.oam_data[0x10], 0xAB, "OAM byte should be written via register");
    // Read back through register: reset address then read
    ppu.cpu_write(0x2003, 0x10, &mut cart);
    let readback = ppu.cpu_read(0x2004, &cart);
    assert_eq!(readback, 0xAB, "OAM byte should read back via 0x2004");
}

#[test]
fn ppu_status_read_clears_vblank() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    // Enable NMI to detect vblank entry
    ppu.cpu_write(0x2000, 0x80, &mut cart);
    // Tick until NMI fires (rising edge of vblank)
    let mut entered_vblank = false;
    for _ in 0..100_000 {
        ppu.tick(&mut cart);
        if ppu.poll_nmi() {
            entered_vblank = true;
            break;
        }
    }
    assert!(entered_vblank, "PPU should reach vblank within 100K ticks");
    // STATUS bit 7 (vblank) should be set
    let status = ppu.cpu_read(0x2002, &cart);
    assert_ne!(status & 0x80, 0, "Vblank flag (bit 7) should be set at vblank");
    // Reading STATUS clears the vblank flag
    let status_after = ppu.cpu_read(0x2002, &cart);
    assert_eq!(status_after & 0x80, 0, "Vblank flag should be cleared after STATUS read");
}

#[test]
fn ppu_scroll_double_write() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    ppu.cpu_write(0x2005, 100, &mut cart); // X scroll
    ppu.cpu_write(0x2005, 50, &mut cart);  // Y scroll
}

#[test]
fn ppu_addr_double_write() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    ppu.cpu_write(0x2006, 0x20, &mut cart); // high byte
    ppu.cpu_write(0x2006, 0x00, &mut cart); // low byte
    ppu.cpu_write(0x2007, 0x42, &mut cart); // write data
}

#[test]
fn ppu_ctrl_write() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    ppu.cpu_write(0x2000, 0xA0, &mut cart);
}

#[test]
fn ppu_mask_write() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    ppu.cpu_write(0x2001, 0x1E, &mut cart);
}

#[test]
fn ppu_frame_complete_after_full_frame() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    let mut completed = false;
    for _ in 0..(341 * 262 + 500) {
        ppu.tick(&mut cart);
        if ppu.frame_complete() {
            completed = true;
            break;
        }
    }
    assert!(completed, "PPU should signal frame completion within ~89342 ticks");
}

#[test]
fn ppu_nmi_at_vblank() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    // Enable NMI via CTRL (bit 7)
    ppu.cpu_write(0x2000, 0x80, &mut cart);
    let mut nmi_fired = false;
    for _ in 0..100_000 {
        ppu.tick(&mut cart);
        if ppu.poll_nmi() {
            nmi_fired = true;
            break;
        }
    }
    assert!(nmi_fired, "NMI should fire at vblank when enabled via CTRL bit 7");
}

#[test]
fn ppu_save_load_state() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    // Write recognizable OAM data
    ppu.cpu_write(0x2003, 0x00, &mut cart);
    ppu.cpu_write(0x2004, 0xAA, &mut cart);
    assert_eq!(ppu.oam_data[0x00], 0xAA);
    // Save state
    let state = ppu.save_state();
    assert!(!state.is_empty(), "save_state should produce non-empty data");
    // Modify OAM directly
    ppu.oam_data[0x00] = 0x55;
    assert_eq!(ppu.oam_data[0x00], 0x55);
    // Load state
    let success = ppu.load_state(&state);
    assert!(success, "load_state should succeed");
    assert_eq!(ppu.oam_data[0x00], 0xAA, "OAM should be restored after load_state");
}

#[test]
fn ppu_data_read_write_roundtrip() {
    let mut ppu = Ppu::new();
    let mut cart = make_cart();
    // Ensure VRAM increment = 1
    ppu.cpu_write(0x2000, 0x00, &mut cart);
    // Set PPU address to palette RAM $3F00
    ppu.cpu_write(0x2006, 0x3F, &mut cart);
    ppu.cpu_write(0x2006, 0x00, &mut cart);
    // Write value to palette
    ppu.cpu_write(0x2007, 0x15, &mut cart);
    // Reset address to $3F00
    ppu.cpu_write(0x2006, 0x3F, &mut cart);
    ppu.cpu_write(0x2006, 0x00, &mut cart);
    // Read back (palette reads are immediate, not buffered)
    let readback = ppu.cpu_read(0x2007, &cart);
    assert_eq!(readback & 0x3F, 0x15, "Palette data should round-trip through PPU registers");
}