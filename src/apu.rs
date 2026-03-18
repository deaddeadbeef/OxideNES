const DUTY_TABLE: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0],  // 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0],  // 25%
    [0, 1, 1, 1, 1, 0, 0, 0],  // 50%
    [1, 0, 0, 1, 1, 1, 1, 1],  // 75% (inverted 25%)
];

const LENGTH_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
    12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
];

const TRIANGLE_TABLE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
];

const NOISE_PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

const DMC_RATE_TABLE: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214,
    190, 160, 142, 128, 106, 84, 72, 54,
];

struct Envelope {
    start: bool,
    loop_flag: bool,
    constant_volume: bool,
    volume: u8,
    decay_level: u8,
    divider: u8,
}

impl Envelope {
    fn new() -> Self {
        Envelope { start: false, loop_flag: false, constant_volume: false, volume: 0, decay_level: 0, divider: 0 }
    }
    
    fn clock(&mut self) {
        if self.start {
            self.start = false;
            self.decay_level = 15;
            self.divider = self.volume;
        } else if self.divider == 0 {
            self.divider = self.volume;
            if self.decay_level > 0 {
                self.decay_level -= 1;
            } else if self.loop_flag {
                self.decay_level = 15;
            }
        } else {
            self.divider -= 1;
        }
    }
    
    fn output(&self) -> u8 {
        if self.constant_volume { self.volume } else { self.decay_level }
    }
}

struct Sweep {
    enabled: bool,
    period: u8,
    negate: bool,
    shift: u8,
    reload: bool,
    divider: u8,
    is_pulse1: bool,
}

impl Sweep {
    fn new(is_pulse1: bool) -> Self {
        Sweep { enabled: false, period: 0, negate: false, shift: 0, reload: false, divider: 0, is_pulse1 }
    }
    
    fn target_period(&self, current: u16) -> u16 {
        let change = current >> self.shift;
        if self.negate {
            if self.is_pulse1 {
                current.wrapping_sub(change).wrapping_sub(1)
            } else {
                current.wrapping_sub(change)
            }
        } else {
            current.wrapping_add(change)
        }
    }
    
    fn muting(&self, current: u16) -> bool {
        current < 8 || (self.shift > 0 && self.target_period(current) > 0x7FF)
    }
    
    fn clock(&mut self, timer_period: &mut u16) {
        let target = self.target_period(*timer_period);
        if self.divider == 0 && self.enabled && !self.muting(*timer_period) && self.shift > 0 {
            *timer_period = target;
        }
        if self.divider == 0 || self.reload {
            self.divider = self.period;
            self.reload = false;
        } else {
            self.divider -= 1;
        }
    }
}

struct PulseChannel {
    enabled: bool,
    duty: u8,
    duty_pos: u8,
    length_counter: u8,
    length_halt: bool,
    timer: u16,
    timer_period: u16,
    envelope: Envelope,
    sweep: Sweep,
}

impl PulseChannel {
    fn new(is_pulse1: bool) -> Self {
        PulseChannel {
            enabled: false, duty: 0, duty_pos: 0, length_counter: 0,
            length_halt: false, timer: 0, timer_period: 0,
            envelope: Envelope::new(), sweep: Sweep::new(is_pulse1),
        }
    }
    
    fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            self.duty_pos = (self.duty_pos + 1) % 8;
        } else {
            self.timer -= 1;
        }
    }
    
    fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || self.sweep.muting(self.timer_period)
            || DUTY_TABLE[self.duty as usize][self.duty_pos as usize] == 0 {
            0
        } else {
            self.envelope.output()
        }
    }
    
    fn write_reg(&mut self, reg: u8, data: u8) {
        match reg {
            0 => {
                self.duty = (data >> 6) & 0x03;
                self.length_halt = data & 0x20 != 0;
                self.envelope.loop_flag = data & 0x20 != 0;
                self.envelope.constant_volume = data & 0x10 != 0;
                self.envelope.volume = data & 0x0F;
            }
            1 => {
                self.sweep.enabled = data & 0x80 != 0;
                self.sweep.period = (data >> 4) & 0x07;
                self.sweep.negate = data & 0x08 != 0;
                self.sweep.shift = data & 0x07;
                self.sweep.reload = true;
            }
            2 => {
                self.timer_period = (self.timer_period & 0xFF00) | data as u16;
            }
            3 => {
                self.timer_period = (self.timer_period & 0x00FF) | ((data as u16 & 0x07) << 8);
                if self.enabled {
                    self.length_counter = LENGTH_TABLE[(data >> 3) as usize];
                }
                self.duty_pos = 0;
                self.envelope.start = true;
            }
            _ => {}
        }
    }
}

struct TriangleChannel {
    enabled: bool,
    length_counter: u8,
    length_halt: bool,
    linear_counter: u8,
    linear_counter_reload: u8,
    linear_counter_reload_flag: bool,
    timer: u16,
    timer_period: u16,
    sequence_pos: u8,
}

impl TriangleChannel {
    fn new() -> Self {
        TriangleChannel {
            enabled: false, length_counter: 0, length_halt: false,
            linear_counter: 0, linear_counter_reload: 0, linear_counter_reload_flag: false,
            timer: 0, timer_period: 0, sequence_pos: 0,
        }
    }
    
    fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            if self.length_counter > 0 && self.linear_counter > 0 {
                self.sequence_pos = (self.sequence_pos + 1) % 32;
            }
        } else {
            self.timer -= 1;
        }
    }
    
    fn clock_linear_counter(&mut self) {
        if self.linear_counter_reload_flag {
            self.linear_counter = self.linear_counter_reload;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }
        if !self.length_halt {
            self.linear_counter_reload_flag = false;
        }
    }
    
    fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || self.linear_counter == 0 || self.timer_period < 2 {
            0
        } else {
            TRIANGLE_TABLE[self.sequence_pos as usize]
        }
    }
    
    fn write_reg(&mut self, reg: u8, data: u8) {
        match reg {
            0 => {
                self.length_halt = data & 0x80 != 0;
                self.linear_counter_reload = data & 0x7F;
            }
            2 => {
                self.timer_period = (self.timer_period & 0xFF00) | data as u16;
            }
            3 => {
                self.timer_period = (self.timer_period & 0x00FF) | ((data as u16 & 0x07) << 8);
                if self.enabled {
                    self.length_counter = LENGTH_TABLE[(data >> 3) as usize];
                }
                self.linear_counter_reload_flag = true;
            }
            _ => {}
        }
    }
}

struct NoiseChannel {
    enabled: bool,
    length_counter: u8,
    length_halt: bool,
    envelope: Envelope,
    timer: u16,
    timer_period: u16,
    mode: bool,
    shift_register: u16,
}

impl NoiseChannel {
    fn new() -> Self {
        NoiseChannel {
            enabled: false, length_counter: 0, length_halt: false,
            envelope: Envelope::new(), timer: 0, timer_period: 0,
            mode: false, shift_register: 1,
        }
    }
    
    fn clock_timer(&mut self) {
        if self.timer == 0 {
            self.timer = self.timer_period;
            let bit = if self.mode { 6 } else { 1 };
            let feedback = (self.shift_register & 1) ^ ((self.shift_register >> bit) & 1);
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
        } else {
            self.timer -= 1;
        }
    }
    
    fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || self.shift_register & 1 != 0 {
            0
        } else {
            self.envelope.output()
        }
    }
    
    fn write_reg(&mut self, reg: u8, data: u8) {
        match reg {
            0 => {
                self.length_halt = data & 0x20 != 0;
                self.envelope.loop_flag = data & 0x20 != 0;
                self.envelope.constant_volume = data & 0x10 != 0;
                self.envelope.volume = data & 0x0F;
            }
            2 => {
                self.mode = data & 0x80 != 0;
                self.timer_period = NOISE_PERIOD_TABLE[(data & 0x0F) as usize];
            }
            3 => {
                if self.enabled {
                    self.length_counter = LENGTH_TABLE[(data >> 3) as usize];
                }
                self.envelope.start = true;
            }
            _ => {}
        }
    }
}

pub struct DmcChannel {
    enabled: bool,
    irq_enabled: bool,
    loop_flag: bool,
    rate_index: u8,
    timer: u16,
    timer_period: u16,
    
    output_level: u8,       // 0-127, current DAC output
    
    // Sample buffer
    sample_buffer: u8,
    sample_buffer_empty: bool,
    bits_remaining: u8,     // bits left in current byte (0-8)
    shift_register: u8,
    silence_flag: bool,
    
    // Memory reader
    pub sample_address: u16,    // current read address
    pub sample_length: u16,     // bytes remaining
    start_address: u16,     // configured start ($C000 + val*64)
    start_length: u16,      // configured length (val*16 + 1)
    
    // IRQ
    pub irq_pending: bool,
    
    // DMA request
    pub dma_request: bool,
    pub dma_address: u16,
}

impl DmcChannel {
    fn new() -> Self {
        DmcChannel {
            enabled: false,
            irq_enabled: false,
            loop_flag: false,
            rate_index: 0,
            timer: 0,
            timer_period: DMC_RATE_TABLE[0],
            output_level: 0,
            sample_buffer: 0,
            sample_buffer_empty: true,
            bits_remaining: 0,
            shift_register: 0,
            silence_flag: true,
            sample_address: 0xC000,
            sample_length: 0,
            start_address: 0xC000,
            start_length: 1,
            irq_pending: false,
            dma_request: false,
            dma_address: 0,
        }
    }
    
    fn tick(&mut self) {
        if self.timer > 0 {
            self.timer -= 1;
            return;
        }
        self.timer = self.timer_period;
        
        // Output unit
        if !self.silence_flag {
            if self.shift_register & 1 != 0 {
                if self.output_level <= 125 {
                    self.output_level += 2;
                }
            } else {
                if self.output_level >= 2 {
                    self.output_level -= 2;
                }
            }
            self.shift_register >>= 1;
        }
        
        self.bits_remaining = self.bits_remaining.saturating_sub(1);
        if self.bits_remaining == 0 {
            self.bits_remaining = 8;
            if self.sample_buffer_empty {
                self.silence_flag = true;
            } else {
                self.silence_flag = false;
                self.shift_register = self.sample_buffer;
                self.sample_buffer_empty = true;
                // Request next sample byte
                self.start_dma();
            }
        }
    }
    
    fn start_dma(&mut self) {
        if self.sample_length > 0 {
            self.dma_request = true;
            self.dma_address = self.sample_address;
        }
    }
    
    pub fn receive_sample(&mut self, data: u8) {
        self.sample_buffer = data;
        self.sample_buffer_empty = false;
        self.dma_request = false;
        
        // Advance address (wraps from $FFFF to $8000)
        self.sample_address = if self.sample_address == 0xFFFF {
            0x8000
        } else {
            self.sample_address + 1
        };
        
        self.sample_length -= 1;
        if self.sample_length == 0 {
            if self.loop_flag {
                self.sample_address = self.start_address;
                self.sample_length = self.start_length;
            } else if self.irq_enabled {
                self.irq_pending = true;
            }
        }
    }
    
    fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            0x4010 => {
                self.irq_enabled = data & 0x80 != 0;
                self.loop_flag = data & 0x40 != 0;
                self.rate_index = data & 0x0F;
                self.timer_period = DMC_RATE_TABLE[self.rate_index as usize];
                if !self.irq_enabled {
                    self.irq_pending = false;
                }
            }
            0x4011 => {
                self.output_level = data & 0x7F;
            }
            0x4012 => {
                self.start_address = 0xC000 + (data as u16) * 64;
            }
            0x4013 => {
                self.start_length = (data as u16) * 16 + 1;
            }
            _ => {}
        }
    }
    
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.irq_pending = false;
        if !enabled {
            self.sample_length = 0;
        } else if self.sample_length == 0 {
            self.sample_address = self.start_address;
            self.sample_length = self.start_length;
            if self.sample_buffer_empty {
                self.start_dma();
            }
        }
    }
    
    fn output(&self) -> u8 {
        self.output_level
    }
}

use blip_buf::BlipBuf;

pub struct Apu {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    pub dmc: DmcChannel,
    
    frame_counter_mode: u8,  // 0 = 4-step, 1 = 5-step
    frame_irq_inhibit: bool,
    frame_step: u8,
    frame_cycle: usize,
    
    pub irq_pending: bool,
    
    cycle: usize,
    
    // blip_buf for band-limited resampling
    blip: BlipBuf,
    
    // Previous mixed output (to detect transitions)
    prev_output: i32,
    
    // Clock cycle counter (reset each frame)
    clock_cycle: u32,
    
    // Output samples ready for audio callback
    pub sample_buffer: Vec<f32>,

    // External audio from mapper expansion chips
    pub external_audio: f32,

    // Pre-computed mixer lookup tables
    pulse_table: [i32; 31],


    // Reusable read buffer for end_frame
    read_buf: Vec<i16>,
}

impl Apu {
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.blip.set_rates(1_789_773.0, sample_rate as f64);
    }

    pub fn new(sample_rate: u32) -> Self {
        let mut blip = BlipBuf::new(sample_rate / 15); // buffer ~1/15 sec
        blip.set_rates(1_789_773.0, sample_rate as f64);

        // Pre-compute mixer lookup tables
        let mut pulse_table = [0i32; 31];
        for i in 1..31u32 {
            let v = 95.88 / ((8128.0 / i as f64) + 100.0);
            pulse_table[i as usize] = (v * 32000.0) as i32;
        }

        Apu {
            pulse1: PulseChannel::new(true),
            pulse2: PulseChannel::new(false),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            frame_counter_mode: 0,
            frame_irq_inhibit: true,
            frame_step: 0,
            frame_cycle: 0,
            irq_pending: false,
            cycle: 0,
            blip,
            prev_output: 0,
            clock_cycle: 0,
            sample_buffer: Vec::with_capacity(2048),
            external_audio: 0.0,
            pulse_table,
            read_buf: vec![0i16; 4096],  // pre-allocate
        }
    }
    
    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4000..=0x4003 => self.pulse1.write_reg((addr - 0x4000) as u8, data),
            0x4004..=0x4007 => self.pulse2.write_reg((addr - 0x4004) as u8, data),
            0x4008 | 0x400A | 0x400B => {
                let reg = match addr { 0x4008 => 0, 0x400A => 2, 0x400B => 3, _ => 0 };
                self.triangle.write_reg(reg, data);
            }
            0x400C | 0x400E | 0x400F => {
                let reg = match addr { 0x400C => 0, 0x400E => 2, 0x400F => 3, _ => 0 };
                self.noise.write_reg(reg, data);
            }
            0x4010..=0x4013 => self.dmc.write_register(addr, data),
            0x4015 => {
                self.pulse1.enabled = data & 0x01 != 0;
                self.pulse2.enabled = data & 0x02 != 0;
                self.triangle.enabled = data & 0x04 != 0;
                self.noise.enabled = data & 0x08 != 0;
                self.dmc.set_enabled(data & 0x10 != 0);
                if !self.pulse1.enabled { self.pulse1.length_counter = 0; }
                if !self.pulse2.enabled { self.pulse2.length_counter = 0; }
                if !self.triangle.enabled { self.triangle.length_counter = 0; }
                if !self.noise.enabled { self.noise.length_counter = 0; }
            }
            0x4017 => {
                self.frame_counter_mode = (data >> 7) & 1;
                self.frame_irq_inhibit = data & 0x40 != 0;
                if self.frame_irq_inhibit {
                    self.irq_pending = false;
                }
                self.frame_cycle = 0;
                if self.frame_counter_mode == 1 {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
            }
            _ => {}
        }
    }
    
    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                let mut status = 0u8;
                if self.pulse1.length_counter > 0 { status |= 0x01; }
                if self.pulse2.length_counter > 0 { status |= 0x02; }
                if self.triangle.length_counter > 0 { status |= 0x04; }
                if self.noise.length_counter > 0 { status |= 0x08; }
                if self.dmc.sample_length > 0 { status |= 0x10; }
                if self.irq_pending { status |= 0x40; }
                if self.dmc.irq_pending { status |= 0x80; }
                self.irq_pending = false; // reading clears frame counter IRQ, but NOT DMC IRQ
                status
            }
            _ => 0,
        }
    }
    
    fn clock_quarter_frame(&mut self) {
        self.pulse1.envelope.clock();
        self.pulse2.envelope.clock();
        self.triangle.clock_linear_counter();
        self.noise.envelope.clock();
    }
    
    fn clock_half_frame(&mut self) {
        // Length counters
        if self.pulse1.length_counter > 0 && !self.pulse1.length_halt {
            self.pulse1.length_counter -= 1;
        }
        if self.pulse2.length_counter > 0 && !self.pulse2.length_halt {
            self.pulse2.length_counter -= 1;
        }
        if self.triangle.length_counter > 0 && !self.triangle.length_halt {
            self.triangle.length_counter -= 1;
        }
        if self.noise.length_counter > 0 && !self.noise.length_halt {
            self.noise.length_counter -= 1;
        }
        // Sweep units
        self.pulse1.sweep.clock(&mut self.pulse1.timer_period);
        self.pulse2.sweep.clock(&mut self.pulse2.timer_period);
    }
    
    pub fn tick(&mut self) {
        // Triangle clocks every CPU cycle
        self.triangle.clock_timer();
        
        // DMC clocks every CPU cycle
        self.dmc.tick();
        
        // Other channels clock every other CPU cycle
        if self.cycle % 2 == 0 {
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
            self.noise.clock_timer();
        }
        
        // Frame counter runs at APU rate (half CPU rate)
        if self.cycle % 2 == 0 {
            self.frame_cycle += 1;
            match self.frame_counter_mode {
                0 => { // 4-step
                    match self.frame_cycle {
                        3729 => self.clock_quarter_frame(),
                        7457 => { self.clock_quarter_frame(); self.clock_half_frame(); }
                        11186 => self.clock_quarter_frame(),
                        14915 => {
                            self.clock_quarter_frame();
                            self.clock_half_frame();
                            if !self.frame_irq_inhibit {
                                self.irq_pending = true;
                            }
                            self.frame_cycle = 0;
                        }
                        _ => {}
                    }
                }
                1 => { // 5-step
                    match self.frame_cycle {
                        3729 => self.clock_quarter_frame(),
                        7457 => { self.clock_quarter_frame(); self.clock_half_frame(); }
                        11186 => self.clock_quarter_frame(),
                        18641 => {
                            self.clock_quarter_frame();
                            self.clock_half_frame();
                            self.frame_cycle = 0;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        
        // Record transition if output changed
        let output = self.mix_output();
        if output != self.prev_output {
            self.blip.add_delta(self.clock_cycle, output - self.prev_output);
            self.prev_output = output;
        }
        
        self.clock_cycle += 1;

        self.cycle += 1;
    }
    
    // Mix channels to an integer amplitude value for blip_buf
    // blip_buf works with i32 deltas
    fn mix_output(&self) -> i32 {
        let p1 = self.pulse1.output() as usize;
        let p2 = self.pulse2.output() as usize;
        let tri = self.triangle.output() as usize;
        let noise = self.noise.output() as usize;
        let dmc = self.dmc.output() as f64;

        // Pulse channels use pre-computed table
        let pulse_out = self.pulse_table[(p1 + p2).min(30)];
        
        // TND channels using safer formula to prevent NaN
        let tnd_out = if tri > 0 || noise > 0 || dmc > 0.0 {
            let tnd_sum = tri as f64 / 8227.0 + noise as f64 / 12241.0 + dmc / 22638.0;
            if tnd_sum > 0.0 {
                let tnd = 159.79 / (1.0 / tnd_sum + 100.0);
                (tnd * 32000.0) as i32
            } else {
                0
            }
        } else {
            0
        };

        let ext_out = self.ext_output();

        pulse_out + tnd_out + ext_out
    }

    // External/mapper expansion audio contribution, scaled to match internal amplitude
    fn ext_output(&self) -> i32 {
        (self.external_audio * 32000.0) as i32
    }
    
    // Called at end of each emulated frame (~29780 CPU cycles)
    pub fn end_frame(&mut self) {
        self.blip.end_frame(self.clock_cycle);
        self.clock_cycle = 0;

        let count = self.blip.samples_avail() as usize;
        if count > 0 {
            if count > self.read_buf.len() {
                self.read_buf.resize(count, 0);
            }
            self.blip.read_samples(&mut self.read_buf[..count], false);

            for &s in &self.read_buf[..count] {
                self.sample_buffer.push(s as f32 / 32768.0);
            }
        }
    }
    
    pub fn drain_samples(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.sample_buffer)
    }

    // ── Save state support ──────────────────────────────────────────
    pub fn save_state(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // Pulse 1
        data.push(self.pulse1.enabled as u8);
        data.push(self.pulse1.duty);
        data.push(self.pulse1.duty_pos);
        data.push(self.pulse1.length_counter);
        data.push(self.pulse1.length_halt as u8);
        data.extend_from_slice(&self.pulse1.timer.to_le_bytes());
        data.extend_from_slice(&self.pulse1.timer_period.to_le_bytes());
        data.push(self.pulse1.envelope.start as u8);
        data.push(self.pulse1.envelope.loop_flag as u8);
        data.push(self.pulse1.envelope.constant_volume as u8);
        data.push(self.pulse1.envelope.volume);
        data.push(self.pulse1.envelope.decay_level);
        data.push(self.pulse1.envelope.divider);
        data.push(self.pulse1.sweep.enabled as u8);
        data.push(self.pulse1.sweep.period);
        data.push(self.pulse1.sweep.negate as u8);
        data.push(self.pulse1.sweep.shift);
        data.push(self.pulse1.sweep.reload as u8);
        data.push(self.pulse1.sweep.divider);
        data.push(self.pulse1.sweep.is_pulse1 as u8);

        // Pulse 2
        data.push(self.pulse2.enabled as u8);
        data.push(self.pulse2.duty);
        data.push(self.pulse2.duty_pos);
        data.push(self.pulse2.length_counter);
        data.push(self.pulse2.length_halt as u8);
        data.extend_from_slice(&self.pulse2.timer.to_le_bytes());
        data.extend_from_slice(&self.pulse2.timer_period.to_le_bytes());
        data.push(self.pulse2.envelope.start as u8);
        data.push(self.pulse2.envelope.loop_flag as u8);
        data.push(self.pulse2.envelope.constant_volume as u8);
        data.push(self.pulse2.envelope.volume);
        data.push(self.pulse2.envelope.decay_level);
        data.push(self.pulse2.envelope.divider);
        data.push(self.pulse2.sweep.enabled as u8);
        data.push(self.pulse2.sweep.period);
        data.push(self.pulse2.sweep.negate as u8);
        data.push(self.pulse2.sweep.shift);
        data.push(self.pulse2.sweep.reload as u8);
        data.push(self.pulse2.sweep.divider);
        data.push(self.pulse2.sweep.is_pulse1 as u8);

        // Triangle
        data.push(self.triangle.enabled as u8);
        data.push(self.triangle.length_counter);
        data.push(self.triangle.length_halt as u8);
        data.push(self.triangle.linear_counter);
        data.push(self.triangle.linear_counter_reload);
        data.push(self.triangle.linear_counter_reload_flag as u8);
        data.extend_from_slice(&self.triangle.timer.to_le_bytes());
        data.extend_from_slice(&self.triangle.timer_period.to_le_bytes());
        data.push(self.triangle.sequence_pos);

        // Noise
        data.push(self.noise.enabled as u8);
        data.push(self.noise.length_counter);
        data.push(self.noise.length_halt as u8);
        data.push(self.noise.envelope.start as u8);
        data.push(self.noise.envelope.loop_flag as u8);
        data.push(self.noise.envelope.constant_volume as u8);
        data.push(self.noise.envelope.volume);
        data.push(self.noise.envelope.decay_level);
        data.push(self.noise.envelope.divider);
        data.extend_from_slice(&self.noise.timer.to_le_bytes());
        data.extend_from_slice(&self.noise.timer_period.to_le_bytes());
        data.push(self.noise.mode as u8);
        data.extend_from_slice(&self.noise.shift_register.to_le_bytes());

        // DMC
        data.push(self.dmc.enabled as u8);
        data.push(self.dmc.irq_enabled as u8);
        data.push(self.dmc.loop_flag as u8);
        data.push(self.dmc.rate_index);
        data.extend_from_slice(&self.dmc.timer.to_le_bytes());
        data.extend_from_slice(&self.dmc.timer_period.to_le_bytes());
        data.push(self.dmc.output_level);
        data.push(self.dmc.sample_buffer);
        data.push(self.dmc.sample_buffer_empty as u8);
        data.push(self.dmc.bits_remaining);
        data.push(self.dmc.shift_register);
        data.push(self.dmc.silence_flag as u8);
        data.extend_from_slice(&self.dmc.sample_address.to_le_bytes());
        data.extend_from_slice(&self.dmc.sample_length.to_le_bytes());
        data.extend_from_slice(&self.dmc.start_address.to_le_bytes());
        data.extend_from_slice(&self.dmc.start_length.to_le_bytes());
        data.push(self.dmc.irq_pending as u8);
        data.push(self.dmc.dma_request as u8);
        data.extend_from_slice(&self.dmc.dma_address.to_le_bytes());

        // Frame counter
        data.push(self.frame_counter_mode);
        data.push(self.frame_irq_inhibit as u8);
        data.push(self.frame_step);
        data.extend_from_slice(&self.frame_cycle.to_le_bytes());
        data.push(self.irq_pending as u8);
        data.extend_from_slice(&self.cycle.to_le_bytes());

        data
    }

    pub fn load_state(&mut self, data: &[u8]) -> bool {
        let mut pos = 0;

        // Helper macro for bounds checking
        macro_rules! need {
            ($n:expr) => {
                if pos + $n > data.len() { return false; }
            };
        }

        // Pulse 1
        need!(21);
        self.pulse1.enabled = data[pos] != 0; pos += 1;
        self.pulse1.duty = data[pos]; pos += 1;
        self.pulse1.duty_pos = data[pos]; pos += 1;
        self.pulse1.length_counter = data[pos]; pos += 1;
        self.pulse1.length_halt = data[pos] != 0; pos += 1;
        self.pulse1.timer = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.pulse1.timer_period = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.pulse1.envelope.start = data[pos] != 0; pos += 1;
        self.pulse1.envelope.loop_flag = data[pos] != 0; pos += 1;
        self.pulse1.envelope.constant_volume = data[pos] != 0; pos += 1;
        self.pulse1.envelope.volume = data[pos]; pos += 1;
        self.pulse1.envelope.decay_level = data[pos]; pos += 1;
        self.pulse1.envelope.divider = data[pos]; pos += 1;
        self.pulse1.sweep.enabled = data[pos] != 0; pos += 1;
        self.pulse1.sweep.period = data[pos]; pos += 1;
        self.pulse1.sweep.negate = data[pos] != 0; pos += 1;
        self.pulse1.sweep.shift = data[pos]; pos += 1;
        self.pulse1.sweep.reload = data[pos] != 0; pos += 1;
        self.pulse1.sweep.divider = data[pos]; pos += 1;
        self.pulse1.sweep.is_pulse1 = data[pos] != 0; pos += 1;

        // Pulse 2
        need!(21);
        self.pulse2.enabled = data[pos] != 0; pos += 1;
        self.pulse2.duty = data[pos]; pos += 1;
        self.pulse2.duty_pos = data[pos]; pos += 1;
        self.pulse2.length_counter = data[pos]; pos += 1;
        self.pulse2.length_halt = data[pos] != 0; pos += 1;
        self.pulse2.timer = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.pulse2.timer_period = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.pulse2.envelope.start = data[pos] != 0; pos += 1;
        self.pulse2.envelope.loop_flag = data[pos] != 0; pos += 1;
        self.pulse2.envelope.constant_volume = data[pos] != 0; pos += 1;
        self.pulse2.envelope.volume = data[pos]; pos += 1;
        self.pulse2.envelope.decay_level = data[pos]; pos += 1;
        self.pulse2.envelope.divider = data[pos]; pos += 1;
        self.pulse2.sweep.enabled = data[pos] != 0; pos += 1;
        self.pulse2.sweep.period = data[pos]; pos += 1;
        self.pulse2.sweep.negate = data[pos] != 0; pos += 1;
        self.pulse2.sweep.shift = data[pos]; pos += 1;
        self.pulse2.sweep.reload = data[pos] != 0; pos += 1;
        self.pulse2.sweep.divider = data[pos]; pos += 1;
        self.pulse2.sweep.is_pulse1 = data[pos] != 0; pos += 1;

        // Triangle
        need!(11);
        self.triangle.enabled = data[pos] != 0; pos += 1;
        self.triangle.length_counter = data[pos]; pos += 1;
        self.triangle.length_halt = data[pos] != 0; pos += 1;
        self.triangle.linear_counter = data[pos]; pos += 1;
        self.triangle.linear_counter_reload = data[pos]; pos += 1;
        self.triangle.linear_counter_reload_flag = data[pos] != 0; pos += 1;
        self.triangle.timer = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.triangle.timer_period = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.triangle.sequence_pos = data[pos]; pos += 1;

        // Noise
        need!(16);
        self.noise.enabled = data[pos] != 0; pos += 1;
        self.noise.length_counter = data[pos]; pos += 1;
        self.noise.length_halt = data[pos] != 0; pos += 1;
        self.noise.envelope.start = data[pos] != 0; pos += 1;
        self.noise.envelope.loop_flag = data[pos] != 0; pos += 1;
        self.noise.envelope.constant_volume = data[pos] != 0; pos += 1;
        self.noise.envelope.volume = data[pos]; pos += 1;
        self.noise.envelope.decay_level = data[pos]; pos += 1;
        self.noise.envelope.divider = data[pos]; pos += 1;
        self.noise.timer = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.noise.timer_period = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.noise.mode = data[pos] != 0; pos += 1;
        self.noise.shift_register = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;

        // DMC
        need!(24);
        self.dmc.enabled = data[pos] != 0; pos += 1;
        self.dmc.irq_enabled = data[pos] != 0; pos += 1;
        self.dmc.loop_flag = data[pos] != 0; pos += 1;
        self.dmc.rate_index = data[pos]; pos += 1;
        self.dmc.timer = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.dmc.timer_period = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.dmc.output_level = data[pos]; pos += 1;
        self.dmc.sample_buffer = data[pos]; pos += 1;
        self.dmc.sample_buffer_empty = data[pos] != 0; pos += 1;
        self.dmc.bits_remaining = data[pos]; pos += 1;
        self.dmc.shift_register = data[pos]; pos += 1;
        self.dmc.silence_flag = data[pos] != 0; pos += 1;
        self.dmc.sample_address = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.dmc.sample_length = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.dmc.start_address = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.dmc.start_length = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;
        self.dmc.irq_pending = data[pos] != 0; pos += 1;
        self.dmc.dma_request = data[pos] != 0; pos += 1;
        self.dmc.dma_address = u16::from_le_bytes([data[pos], data[pos+1]]); pos += 2;

        // Frame counter
        let usize_bytes = std::mem::size_of::<usize>();
        need!(3 + usize_bytes + 1 + usize_bytes);
        self.frame_counter_mode = data[pos]; pos += 1;
        self.frame_irq_inhibit = data[pos] != 0; pos += 1;
        self.frame_step = data[pos]; pos += 1;
        let mut fc_bytes = [0u8; 8];
        fc_bytes[..usize_bytes].copy_from_slice(&data[pos..pos+usize_bytes]);
        self.frame_cycle = usize::from_le_bytes(fc_bytes);
        pos += usize_bytes;
        self.irq_pending = data[pos] != 0; pos += 1;
        let mut c_bytes = [0u8; 8];
        c_bytes[..usize_bytes].copy_from_slice(&data[pos..pos+usize_bytes]);
        self.cycle = usize::from_le_bytes(c_bytes);

        true
    }
}
