use oxidenes::bus::Bus;
use oxidenes::cartridge::Cartridge;

fn make_minimal_rom() -> Vec<u8> {
    let mut rom = vec![0u8; 16 + 16384 + 8192];
    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = 1;
    rom[5] = 1;
    rom[6] = 0;
    rom[7] = 0;
    rom
}

fn make_test_bus() -> Bus {
    let cart = Cartridge::new(&make_minimal_rom()).unwrap();
    Bus::new(cart)
}

#[test]
fn bus_ram_write_read() {
    let mut bus = make_test_bus();
    bus.cpu_write(0x0000, 0x42);
    assert_eq!(bus.cpu_read(0x0000), 0x42);
}

#[test]
fn bus_ram_mirroring_read() {
    let mut bus = make_test_bus();
    bus.cpu_write(0x0000, 0xAB);
    assert_eq!(bus.cpu_read(0x0800), 0xAB);
    assert_eq!(bus.cpu_read(0x1000), 0xAB);
    assert_eq!(bus.cpu_read(0x1800), 0xAB);
}

#[test]
fn bus_ram_mirroring_write() {
    let mut bus = make_test_bus();
    bus.cpu_write(0x0800, 0xCD);
    assert_eq!(bus.cpu_read(0x0000), 0xCD);
}

#[test]
fn bus_ram_independence() {
    let mut bus = make_test_bus();
    for i in 0..=0xFF_u16 {
        bus.cpu_write(i, i as u8);
    }
    for i in 0..=0xFF_u16 {
        assert_eq!(bus.cpu_read(i), i as u8);
    }
}

#[test]
fn bus_oam_dma_trigger() {
    let mut bus = make_test_bus();
    // Write data to page 0x02 (RAM addresses 0x0200-0x02FF)
    for i in 0..=0xFF_u16 {
        bus.cpu_write(0x0200 + i, i as u8);
    }
    // Trigger OAM DMA by writing to $4014
    bus.cpu_write(0x4014, 0x02);
    assert!(bus.dma_active());
}

#[test]
fn bus_joypad_write_read() {
    let mut bus = make_test_bus();
    // Write strobe to joypad port
    bus.cpu_write(0x4016, 0x01);
    bus.cpu_write(0x4016, 0x00);
    // Read joypad (all buttons released by default)
    let val = bus.cpu_read(0x4016);
    assert_eq!(val & 0x1F, 0x00); // lower bits should be 0 for no buttons
}

#[test]
fn bus_ram_snapshot() {
    let mut bus = make_test_bus();
    bus.cpu_write(0x0000, 0x42);
    bus.cpu_write(0x07FF, 0xFF);
    let snapshot = bus.ram_snapshot();
    assert_eq!(snapshot.len(), 2048);
    assert_eq!(snapshot[0x0000], 0x42);
    assert_eq!(snapshot[0x07FF], 0xFF);
}

#[test]
fn bus_save_load_state() {
    let mut bus = make_test_bus();
    bus.cpu_write(0x0000, 0x42);
    bus.cpu_write(0x0100, 0xAB);

    let state = bus.save_state();

    let mut bus2 = make_test_bus();
    assert!(bus2.load_state(&state));
    assert_eq!(bus2.cpu_read(0x0000), 0x42);
    assert_eq!(bus2.cpu_read(0x0100), 0xAB);
}

#[test]
fn bus_cartridge_space_read() {
    let mut bus = make_test_bus();
    // Mapper 0 maps PRG ROM at 0x8000+
    // PRG ROM was initialized with zeros, so reading should return 0
    let val = bus.cpu_read(0x8000);
    assert_eq!(val, 0x00);
}

#[test]
fn bus_poll_nmi_initially_false() {
    let mut bus = make_test_bus();
    assert!(!bus.poll_nmi());
}
