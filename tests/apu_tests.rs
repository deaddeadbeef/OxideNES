use oxidenes::apu::Apu;

const SAMPLE_RATE: u32 = 44100;

#[test]
fn new_apu_initial_state() {
    let apu = Apu::new(SAMPLE_RATE);
    assert!(apu.sample_buffer.is_empty(), "sample_buffer should be empty on init");
    assert!(!apu.irq_pending, "frame IRQ should not be pending on init");
    assert!(!apu.dmc.irq_pending, "DMC IRQ should not be pending on init");
}

#[test]
fn channel_enable_disable() {
    let mut apu = Apu::new(SAMPLE_RATE);

    // All channels disabled initially — status should be 0 (no length counters active)
    let status = apu.read(0x4015);
    assert_eq!(status & 0x1F, 0, "No channels should report active initially");

    // Enable pulse 1 and write to its length counter register ($4003)
    // to load a non-zero length counter
    apu.write(0x4015, 0x01); // enable pulse 1
    apu.write(0x4003, 0x08); // load length counter (table lookup)
    let status = apu.read(0x4015);
    assert!(status & 0x01 != 0, "Pulse 1 should report active after length load");

    // Disable pulse 1
    apu.write(0x4015, 0x00);
    let status = apu.read(0x4015);
    assert_eq!(status & 0x01, 0, "Pulse 1 should be inactive after disable");
}

#[test]
fn read_status_clears_frame_irq() {
    let mut apu = Apu::new(SAMPLE_RATE);

    // Set 4-step mode, IRQ not inhibited → frame IRQ will fire at step 14915
    apu.write(0x4017, 0x00); // 4-step mode, IRQ enabled

    // Tick enough CPU cycles to trigger frame IRQ (frame_cycle 14915 × 2 CPU cycles)
    for _ in 0..29831 {
        apu.tick();
    }
    assert!(apu.irq_pending, "Frame IRQ should be pending after enough ticks");

    // Reading $4015 should report IRQ and clear it
    let status = apu.read(0x4015);
    assert!(status & 0x40 != 0, "Status should show frame IRQ bit");
    assert!(!apu.irq_pending, "Frame IRQ should be cleared after read");

    // Second read should not show IRQ
    let status2 = apu.read(0x4015);
    assert_eq!(status2 & 0x40, 0, "Frame IRQ bit should be clear on second read");
}

#[test]
fn pulse_channel_write() {
    let mut apu = Apu::new(SAMPLE_RATE);
    apu.write(0x4015, 0x01); // enable pulse 1

    // Write to all 4 pulse 1 registers
    apu.write(0x4000, 0xBF); // duty=10, length halt, constant vol, vol=15
    apu.write(0x4001, 0x00); // sweep disabled
    apu.write(0x4002, 0xFD); // timer low
    apu.write(0x4003, 0x08); // length counter load + timer high

    // Verify channel is alive — tick and no panic
    for _ in 0..1000 {
        apu.tick();
    }

    // Pulse 1 should be active (length counter loaded)
    let status = apu.read(0x4015);
    assert!(status & 0x01 != 0, "Pulse 1 should be active after register writes");
}

#[test]
fn triangle_channel_write() {
    let mut apu = Apu::new(SAMPLE_RATE);
    apu.write(0x4015, 0x04); // enable triangle

    apu.write(0x4008, 0xFF); // linear counter reload, length halt
    apu.write(0x400A, 0xF0); // timer low
    apu.write(0x400B, 0x08); // length counter load + timer high

    for _ in 0..1000 {
        apu.tick();
    }

    let status = apu.read(0x4015);
    assert!(status & 0x04 != 0, "Triangle should be active after register writes");
}

#[test]
fn noise_channel_write() {
    let mut apu = Apu::new(SAMPLE_RATE);
    apu.write(0x4015, 0x08); // enable noise

    apu.write(0x400C, 0x3F); // length halt, constant volume, vol=15
    apu.write(0x400E, 0x00); // mode=0, period=0
    apu.write(0x400F, 0x08); // length counter load

    for _ in 0..1000 {
        apu.tick();
    }

    let status = apu.read(0x4015);
    assert!(status & 0x08 != 0, "Noise should be active after register writes");
}

#[test]
fn dmc_channel_enable_disable() {
    let mut apu = Apu::new(SAMPLE_RATE);

    // Enable DMC via $4015 bit 4
    apu.write(0x4015, 0x10);
    // DMC enabled → sample_length reloaded from start_length (default 1)
    assert!(apu.dmc.sample_length > 0, "DMC sample_length should be non-zero when enabled");

    // Disable DMC
    apu.write(0x4015, 0x00);
    assert_eq!(apu.dmc.sample_length, 0, "DMC sample_length should be 0 when disabled");
}

#[test]
fn frame_counter_mode_set() {
    // 4-step mode (bit 7 clear) — frame IRQ can fire
    let mut apu4 = Apu::new(SAMPLE_RATE);
    apu4.write(0x4017, 0x00); // 4-step, IRQ not inhibited

    for _ in 0..29831 {
        apu4.tick();
    }
    assert!(apu4.irq_pending, "4-step mode should generate frame IRQ");

    // 5-step mode (bit 7 set) — frame IRQ never fires
    let mut apu5 = Apu::new(SAMPLE_RATE);
    apu5.write(0x4017, 0x80); // 5-step mode

    for _ in 0..40000 {
        apu5.tick();
    }
    assert!(!apu5.irq_pending, "5-step mode should never generate frame IRQ");
}

#[test]
fn tick_produces_samples() {
    let mut apu = Apu::new(SAMPLE_RATE);

    // Tick a full frame worth of CPU cycles (~29780)
    for _ in 0..29780 {
        apu.tick();
    }
    apu.end_frame();

    assert!(!apu.sample_buffer.is_empty(), "sample_buffer should have data after a frame");
}

#[test]
fn drain_samples_clears_buffer() {
    let mut apu = Apu::new(SAMPLE_RATE);

    for _ in 0..29780 {
        apu.tick();
    }
    apu.end_frame();
    assert!(!apu.sample_buffer.is_empty(), "Should have samples before drain");

    let samples = apu.drain_samples();
    assert!(!samples.is_empty(), "Drained samples should be non-empty");
    assert!(apu.sample_buffer.is_empty(), "Buffer should be empty after drain");
}

#[test]
fn save_load_state_roundtrip() {
    let mut apu = Apu::new(SAMPLE_RATE);

    // Set up some state
    apu.write(0x4015, 0x0F); // enable pulse1, pulse2, triangle, noise
    apu.write(0x4000, 0xBF);
    apu.write(0x4003, 0x08);
    apu.write(0x4017, 0x00); // 4-step mode

    for _ in 0..5000 {
        apu.tick();
    }

    let saved = apu.save_state();

    // Read status to capture current state
    // Note: read clears irq, so use a fresh APU for comparison
    let irq_before = apu.irq_pending;
    let dmc_irq_before = apu.dmc.irq_pending;

    // Mutate APU significantly
    apu.write(0x4015, 0x00); // disable everything
    for _ in 0..10000 {
        apu.tick();
    }

    // Restore state
    let loaded = apu.load_state(&saved);
    assert!(loaded, "load_state should return true for valid data");
    assert_eq!(apu.irq_pending, irq_before, "IRQ pending should be restored");
    assert_eq!(apu.dmc.irq_pending, dmc_irq_before, "DMC IRQ should be restored");
}

#[test]
fn sample_rate_change() {
    let mut apu = Apu::new(SAMPLE_RATE);

    // Change to a different sample rate
    apu.set_sample_rate(48000);

    // Tick and produce samples at new rate
    for _ in 0..29780 {
        apu.tick();
    }
    apu.end_frame();

    assert!(!apu.sample_buffer.is_empty(), "Should produce samples after sample rate change");
}
