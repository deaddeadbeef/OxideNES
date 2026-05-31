use std::collections::{BTreeSet, HashMap};

use serde::Serialize;

use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::joypad::JoypadButton;

pub const DIAGNOSTIC_PROVENANCE: &str =
    "Generated OxideNES diagnostic iNES cartridge: synthetic 6502 program and CHR patterns only, no ROM content.";

const PROGRAM_BASE: u16 = 0x8000;
const PRG_BANKS: u8 = 2;
const CHR_BANKS: u8 = 1;
const PRG_SIZE: usize = PRG_BANKS as usize * 16 * 1024;
const CHR_SIZE: usize = CHR_BANKS as usize * 8 * 1024;

const STATUS_ADDR: u8 = 0xF0;
const CURRENT_TEST_ADDR: u8 = 0xF1;
const FAILURE_CODE_ADDR: u8 = 0xF2;
const SIGNATURE_ADDR: u8 = 0xF3;
const NMI_COUNT_ADDR: u8 = 0xF4;

const RESULT_BASE: u16 = 0x0200;
const STATUS_RUNNING: u8 = 0x01;
const STATUS_PASS: u8 = 0x80;
const STATUS_FAIL: u8 = 0xE0;
const RESULT_PASS: u8 = 0x01;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DiagnosticTestSpec {
    pub id: u8,
    pub name: &'static str,
}

pub const DIAGNOSTIC_TESTS: &[DiagnosticTestSpec] = &[
    DiagnosticTestSpec {
        id: 1,
        name: "cpu_arithmetic_flags",
    },
    DiagnosticTestSpec {
        id: 2,
        name: "stack_jsr_rts",
    },
    DiagnosticTestSpec {
        id: 3,
        name: "cpu_ram_mirroring",
    },
    DiagnosticTestSpec {
        id: 4,
        name: "ppu_palette_register_roundtrip",
    },
    DiagnosticTestSpec {
        id: 5,
        name: "oam_dma_transfer",
    },
    DiagnosticTestSpec {
        id: 6,
        name: "apu_status_register",
    },
    DiagnosticTestSpec {
        id: 7,
        name: "joypad_strobe_shift",
    },
    DiagnosticTestSpec {
        id: 8,
        name: "ppu_nmi_and_render_frame",
    },
];

#[derive(Debug, Clone)]
pub struct DiagnosticConfig {
    pub max_cpu_cycles: u64,
    pub joypad1_mask: u8,
}

impl Default for DiagnosticConfig {
    fn default() -> Self {
        Self {
            max_cpu_cycles: 500_000,
            joypad1_mask: 0x81, // A + Right, matching the cartridge joypad test.
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DiagnosticTelemetry {
    pub provenance: &'static str,
    pub cartridge: CartridgeTelemetry,
    pub verdict: VerdictTelemetry,
    pub cycles: u64,
    pub frames: u64,
    pub cpu: CpuTelemetry,
    pub ram: RamTelemetry,
    pub tests: Vec<TestTelemetry>,
    pub oam: OamTelemetry,
    pub frame: FrameTelemetry,
    pub audio: AudioTelemetry,
    pub events: Vec<EventTelemetry>,
}

#[derive(Debug, Serialize)]
pub struct CartridgeTelemetry {
    pub mapper: u8,
    pub prg_banks: u8,
    pub chr_banks: u8,
    pub size_bytes: usize,
    pub reset_vector: u16,
    pub nmi_vector: u16,
    pub irq_vector: u16,
    pub rom_hash: u64,
}

#[derive(Debug, Serialize)]
pub struct VerdictTelemetry {
    pub passed: bool,
    pub status: u8,
    pub timeout: bool,
    pub current_test: u8,
    pub current_test_name: Option<&'static str>,
    pub failure_code: u8,
    pub host_failures: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CpuTelemetry {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub status: u8,
    pub pending_cycles: u8,
}

#[derive(Debug, Serialize)]
pub struct RamTelemetry {
    pub signature: u8,
    pub nmi_count: u8,
    pub checksum: u64,
    pub result_base: u16,
}

#[derive(Debug, Serialize)]
pub struct TestTelemetry {
    pub id: u8,
    pub name: &'static str,
    pub result: u8,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct OamTelemetry {
    pub checksum: u64,
    pub expected_checksum: u64,
    pub first_16: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct FrameTelemetry {
    pub checksum: u64,
    pub unique_colors: usize,
    pub nonzero_pixels: usize,
}

#[derive(Debug, Serialize)]
pub struct AudioTelemetry {
    pub sample_count: usize,
    pub peak_abs: f32,
}

#[derive(Debug, Serialize)]
pub struct EventTelemetry {
    pub cycle: u64,
    pub frame: u64,
    pub status: u8,
    pub current_test: u8,
    pub pc: u16,
    pub note: String,
}

pub fn build_diagnostic_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_program_with_labels()?;
    if program.len() > PRG_SIZE {
        return Err(format!(
            "diagnostic program is too large: {} bytes > {} bytes",
            program.len(),
            PRG_SIZE
        ));
    }

    let mut rom = Vec::with_capacity(16 + PRG_SIZE + CHR_SIZE);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(PRG_BANKS);
    rom.push(CHR_BANKS);
    rom.push(0x00); // mapper 0, horizontal mirroring
    rom.push(0x00);
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; PRG_SIZE];
    prg[..program.len()].copy_from_slice(&program);
    write_vector(&mut prg, 0xFFFA, label_addr(&labels, "nmi")?);
    write_vector(&mut prg, 0xFFFC, PROGRAM_BASE);
    write_vector(&mut prg, 0xFFFE, label_addr(&labels, "irq")?);
    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&build_chr_rom());
    Ok(rom)
}

pub fn run_diagnostic(config: DiagnosticConfig) -> Result<DiagnosticTelemetry, String> {
    let rom = build_diagnostic_cartridge()?;
    let cartridge_info = cartridge_telemetry(&rom);
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    apply_joypad_mask(&mut bus, config.joypad1_mask);

    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let mut audio_sample_count = 0usize;
    let mut audio_peak_abs = 0.0f32;
    let mut events = Vec::new();
    let mut last_status = read_ram_byte(&mut bus, STATUS_ADDR);
    let mut timeout = true;

    events.push(EventTelemetry {
        cycle: 0,
        frame: 0,
        status: last_status,
        current_test: read_ram_byte(&mut bus, CURRENT_TEST_ADDR),
        pc: cpu.pc,
        note: "reset".to_string(),
    });

    while cycles < config.max_cpu_cycles {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        bus.service_dmc_dma();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let samples = bus.apu.drain_samples();
            audio_sample_count += samples.len();
            for sample in samples {
                audio_peak_abs = audio_peak_abs.max(sample.abs());
            }
            events.push(EventTelemetry {
                cycle: cycles,
                frame: frames,
                status: read_ram_byte(&mut bus, STATUS_ADDR),
                current_test: read_ram_byte(&mut bus, CURRENT_TEST_ADDR),
                pc: cpu.pc,
                note: "frame_complete".to_string(),
            });
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if status != last_status {
            last_status = status;
            events.push(EventTelemetry {
                cycle: cycles,
                frame: frames,
                status,
                current_test: read_ram_byte(&mut bus, CURRENT_TEST_ADDR),
                pc: cpu.pc,
                note: "status_changed".to_string(),
            });
        }

        if status == STATUS_PASS || status == STATUS_FAIL {
            timeout = false;
            break;
        }
    }

    if !timeout && read_ram_byte(&mut bus, STATUS_ADDR) == STATUS_PASS {
        let target_frames = frames + 1;
        let cycle_limit = cycles.saturating_add(40_000);
        while cycles < cycle_limit && frames < target_frames {
            cpu.clock(&mut bus);
            bus.tick(1);
            bus.tick_apu();
            bus.service_dmc_dma();
            cycles += 1;

            if bus.ppu.frame_complete() {
                frames += 1;
                bus.apu.end_frame();
                let samples = bus.apu.drain_samples();
                audio_sample_count += samples.len();
                for sample in samples {
                    audio_peak_abs = audio_peak_abs.max(sample.abs());
                }
                events.push(EventTelemetry {
                    cycle: cycles,
                    frame: frames,
                    status: read_ram_byte(&mut bus, STATUS_ADDR),
                    current_test: read_ram_byte(&mut bus, CURRENT_TEST_ADDR),
                    pc: cpu.pc,
                    note: "post_pass_frame_complete".to_string(),
                });
            }
        }
    }

    let ram = bus.ram_snapshot();
    let status = ram[STATUS_ADDR as usize];
    let current_test = ram[CURRENT_TEST_ADDR as usize];
    let failure_code = ram[FAILURE_CODE_ADDR as usize];
    let test_results = test_telemetry(&ram);
    let oam = oam_telemetry(&bus.ppu.oam_data);
    let frame = frame_telemetry(&bus.ppu.frame_data);
    let mut host_failures = host_validate(HostValidationInput {
        status,
        timeout,
        tests: &test_results,
        ram: &ram,
        oam: &oam,
        frame: &frame,
        audio_sample_count,
        frames,
    });

    if status == STATUS_FAIL {
        host_failures.push(format!(
            "cartridge reported failure in test {} with code 0x{:02X}",
            current_test, failure_code
        ));
    }

    let passed = status == STATUS_PASS && !timeout && host_failures.is_empty();

    Ok(DiagnosticTelemetry {
        provenance: DIAGNOSTIC_PROVENANCE,
        cartridge: cartridge_info,
        verdict: VerdictTelemetry {
            passed,
            status,
            timeout,
            current_test,
            current_test_name: test_name(current_test),
            failure_code,
            host_failures,
        },
        cycles,
        frames,
        cpu: CpuTelemetry {
            pc: cpu.pc,
            a: cpu.a,
            x: cpu.x,
            y: cpu.y,
            sp: cpu.sp,
            status: cpu.status,
            pending_cycles: cpu.cycles,
        },
        ram: RamTelemetry {
            signature: ram[SIGNATURE_ADDR as usize],
            nmi_count: ram[NMI_COUNT_ADDR as usize],
            checksum: hash_bytes(&ram),
            result_base: RESULT_BASE,
        },
        tests: test_results,
        oam,
        frame,
        audio: AudioTelemetry {
            sample_count: audio_sample_count,
            peak_abs: audio_peak_abs,
        },
        events,
    })
}

struct HostValidationInput<'a> {
    status: u8,
    timeout: bool,
    tests: &'a [TestTelemetry],
    ram: &'a [u8],
    oam: &'a OamTelemetry,
    frame: &'a FrameTelemetry,
    audio_sample_count: usize,
    frames: u64,
}

fn host_validate(input: HostValidationInput<'_>) -> Vec<String> {
    let mut failures = Vec::new();

    if input.timeout {
        failures.push("diagnostic timed out before cartridge completion".to_string());
    }
    if input.status != STATUS_PASS {
        failures.push(format!(
            "cartridge status 0x{:02X} did not reach PASS",
            input.status
        ));
    }
    if input.ram[SIGNATURE_ADDR as usize] != 0xA5 {
        failures.push(format!(
            "signature byte mismatch: got 0x{:02X}",
            input.ram[SIGNATURE_ADDR as usize]
        ));
    }
    if input.ram[NMI_COUNT_ADDR as usize] < 2 {
        failures.push(format!(
            "expected at least two NMIs, got {}",
            input.ram[NMI_COUNT_ADDR as usize]
        ));
    }
    for test in input.tests {
        if !test.passed {
            failures.push(format!(
                "test {} ({}) result byte is 0x{:02X}",
                test.id, test.name, test.result
            ));
        }
    }
    if input.oam.checksum != input.oam.expected_checksum {
        failures.push(format!(
            "OAM DMA checksum mismatch: got 0x{:016X}, expected 0x{:016X}",
            input.oam.checksum, input.oam.expected_checksum
        ));
    }
    if input.frames < 2 {
        failures.push(format!(
            "expected at least two completed frames, got {}",
            input.frames
        ));
    }
    if input.frame.unique_colors < 2 {
        failures.push(format!(
            "expected rendered diagnostic frame to contain multiple colors, got {}",
            input.frame.unique_colors
        ));
    }
    if input.audio_sample_count == 0 {
        failures.push("APU did not produce any drained frame samples".to_string());
    }

    failures
}

fn build_program_with_labels() -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new();

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017); // Inhibit APU frame IRQs.
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);

    program.cpu_arithmetic_flags();
    program.stack_jsr_rts();
    program.cpu_ram_mirroring();
    program.ppu_palette_roundtrip();
    program.oam_dma_transfer();
    program.apu_status_register();
    program.joypad_strobe_shift();
    program.ppu_nmi_and_render_frame();

    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("fail")?;
    program.asm.lda_imm(STATUS_FAIL);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    program.asm.label("sub_stack_jsr")?;
    program.asm.lda_imm(0x77);
    program.asm.rts();

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();

    program.asm.label("irq")?;
    program.asm.rti();

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

struct DiagnosticProgram {
    asm: Assembler,
    next_label: usize,
}

impl DiagnosticProgram {
    fn new() -> Self {
        Self {
            asm: Assembler::new(PROGRAM_BASE),
            next_label: 0,
        }
    }

    fn begin_test(&mut self, id: u8) {
        self.asm.lda_imm(id);
        self.asm.sta_zp(CURRENT_TEST_ADDR);
    }

    fn pass_test(&mut self, id: u8) {
        self.asm.lda_imm(RESULT_PASS);
        self.asm.sta_abs(result_addr(id));
    }

    fn expect_a_eq(&mut self, expected: u8, fail_code: u8) {
        let ok = self.unique_label("ok");
        self.asm.cmp_imm(expected);
        self.asm.beq(&ok);
        self.asm.lda_imm(fail_code);
        self.asm.sta_zp(FAILURE_CODE_ADDR);
        self.asm.jmp_label("fail");
        self.asm
            .label(&ok)
            .expect("unique label should not collide");
    }

    fn unique_label(&mut self, prefix: &str) -> String {
        let label = format!("{prefix}_{}", self.next_label);
        self.next_label += 1;
        label
    }

    fn cpu_arithmetic_flags(&mut self) {
        self.begin_test(1);
        self.asm.lda_imm(0x10);
        self.asm.clc();
        self.asm.adc_imm(0x22);
        self.expect_a_eq(0x32, 0x11);
        self.asm.sec();
        self.asm.sbc_imm(0x12);
        self.expect_a_eq(0x20, 0x12);
        self.asm.lda_imm(0x7F);
        self.asm.clc();
        self.asm.adc_imm(0x01);
        self.expect_a_eq(0x80, 0x13);
        self.pass_test(1);
    }

    fn stack_jsr_rts(&mut self) {
        self.begin_test(2);
        self.asm.lda_imm(0x42);
        self.asm.pha();
        self.asm.lda_imm(0x00);
        self.asm.pla();
        self.expect_a_eq(0x42, 0x21);
        self.asm.jsr_label("sub_stack_jsr");
        self.expect_a_eq(0x77, 0x22);
        self.pass_test(2);
    }

    fn cpu_ram_mirroring(&mut self) {
        self.begin_test(3);
        self.asm.lda_imm(0x5A);
        self.asm.sta_abs(0x0002);
        self.asm.lda_abs(0x0802);
        self.expect_a_eq(0x5A, 0x31);
        self.asm.lda_imm(0xA5);
        self.asm.sta_abs(0x07FF);
        self.asm.lda_abs(0x1FFF);
        self.expect_a_eq(0xA5, 0x32);
        self.pass_test(3);
    }

    fn ppu_palette_roundtrip(&mut self) {
        self.begin_test(4);
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x3F);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x25);
        self.asm.sta_abs(0x2007);
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x3F);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_abs(0x2007);
        self.asm.and_imm(0x3F);
        self.expect_a_eq(0x25, 0x41);
        self.pass_test(4);
    }

    fn oam_dma_transfer(&mut self) {
        self.begin_test(5);
        let loop_label = self.unique_label("fill_oam");
        self.asm.ldx_imm(0x00);
        self.asm
            .label(&loop_label)
            .expect("unique label should not collide");
        self.asm.txa();
        self.asm.sta_abs_x(0x0300);
        self.asm.inx();
        self.asm.bne(&loop_label);
        self.asm.lda_imm(0x03);
        self.asm.sta_abs(0x4014);
        self.pass_test(5);
    }

    fn apu_status_register(&mut self) {
        self.begin_test(6);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4015);
        self.asm.lda_imm(0x1F);
        self.asm.sta_abs(0x4000);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x4002);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x4003);
        self.asm.lda_abs(0x4015);
        self.asm.and_imm(0x01);
        self.expect_a_eq(0x01, 0x61);
        self.pass_test(6);
    }

    fn joypad_strobe_shift(&mut self) {
        self.begin_test(7);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4016);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4016);

        let expected = [1, 0, 0, 0, 0, 0, 0, 1];
        for (index, expected_bit) in expected.into_iter().enumerate() {
            self.asm.lda_abs(0x4016);
            self.asm.and_imm(0x01);
            self.expect_a_eq(expected_bit, 0x70 + index as u8);
        }
        self.pass_test(7);
    }

    fn ppu_nmi_and_render_frame(&mut self) {
        self.begin_test(8);
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        let fill_name = self.unique_label("fill_name");
        self.asm.ldx_imm(0x00);
        self.asm
            .label(&fill_name)
            .expect("unique label should not collide");
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x2007);
        self.asm.inx();
        self.asm.bne(&fill_name);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x3F);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2007);
        self.asm.lda_imm(0x21);
        self.asm.sta_abs(0x2007);

        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2005);
        self.asm.sta_abs(0x2005);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x2001);
        self.asm.lda_imm(0x80);
        self.asm.sta_abs(0x2000);

        let wait = self.unique_label("wait_nmi");
        self.asm
            .label(&wait)
            .expect("unique label should not collide");
        self.asm.lda_zp(NMI_COUNT_ADDR);
        self.asm.cmp_imm(0x02);
        self.asm.bne(&wait);
        self.pass_test(8);
    }
}

#[derive(Debug, Clone)]
struct Assembler {
    base: u16,
    bytes: Vec<u8>,
    labels: HashMap<String, u16>,
    patches: Vec<Patch>,
}

#[derive(Debug, Clone)]
struct Patch {
    at: usize,
    label: String,
    kind: PatchKind,
}

#[derive(Debug, Clone, Copy)]
enum PatchKind {
    Absolute,
    Relative,
}

impl Assembler {
    fn new(base: u16) -> Self {
        Self {
            base,
            bytes: Vec::new(),
            labels: HashMap::new(),
            patches: Vec::new(),
        }
    }

    fn label(&mut self, name: &str) -> Result<(), String> {
        let addr = self
            .base
            .checked_add(self.bytes.len() as u16)
            .ok_or_else(|| format!("label address overflow for {name}"))?;
        if self.labels.insert(name.to_string(), addr).is_some() {
            return Err(format!("duplicate diagnostic label: {name}"));
        }
        Ok(())
    }

    fn finalize(mut self) -> Result<Vec<u8>, String> {
        for patch in self.patches {
            let target = *self
                .labels
                .get(&patch.label)
                .ok_or_else(|| format!("missing diagnostic label: {}", patch.label))?;
            match patch.kind {
                PatchKind::Absolute => {
                    self.bytes[patch.at] = target as u8;
                    self.bytes[patch.at + 1] = (target >> 8) as u8;
                }
                PatchKind::Relative => {
                    let next_pc = self
                        .base
                        .checked_add(patch.at as u16 + 1)
                        .ok_or_else(|| "relative patch address overflow".to_string())?;
                    let offset = target as i32 - next_pc as i32;
                    if !(-128..=127).contains(&offset) {
                        return Err(format!(
                            "branch to {} out of range: offset {}",
                            patch.label, offset
                        ));
                    }
                    self.bytes[patch.at] = offset as i8 as u8;
                }
            }
        }
        Ok(self.bytes)
    }

    fn emit(&mut self, byte: u8) {
        self.bytes.push(byte);
    }

    fn emit_u16(&mut self, value: u16) {
        self.bytes.push(value as u8);
        self.bytes.push((value >> 8) as u8);
    }

    fn op_imm(&mut self, op: u8, value: u8) {
        self.emit(op);
        self.emit(value);
    }

    fn op_zp(&mut self, op: u8, addr: u8) {
        self.emit(op);
        self.emit(addr);
    }

    fn op_abs(&mut self, op: u8, addr: u16) {
        self.emit(op);
        self.emit_u16(addr);
    }

    fn op_abs_label(&mut self, op: u8, label: &str) {
        self.emit(op);
        let at = self.bytes.len();
        self.emit_u16(0);
        self.patches.push(Patch {
            at,
            label: label.to_string(),
            kind: PatchKind::Absolute,
        });
    }

    fn op_rel(&mut self, op: u8, label: &str) {
        self.emit(op);
        let at = self.bytes.len();
        self.emit(0);
        self.patches.push(Patch {
            at,
            label: label.to_string(),
            kind: PatchKind::Relative,
        });
    }

    fn lda_imm(&mut self, value: u8) {
        self.op_imm(0xA9, value);
    }

    fn lda_zp(&mut self, addr: u8) {
        self.op_zp(0xA5, addr);
    }

    fn lda_abs(&mut self, addr: u16) {
        self.op_abs(0xAD, addr);
    }

    fn ldx_imm(&mut self, value: u8) {
        self.op_imm(0xA2, value);
    }

    fn sta_zp(&mut self, addr: u8) {
        self.op_zp(0x85, addr);
    }

    fn sta_abs(&mut self, addr: u16) {
        self.op_abs(0x8D, addr);
    }

    fn sta_abs_x(&mut self, addr: u16) {
        self.op_abs(0x9D, addr);
    }

    fn adc_imm(&mut self, value: u8) {
        self.op_imm(0x69, value);
    }

    fn sbc_imm(&mut self, value: u8) {
        self.op_imm(0xE9, value);
    }

    fn and_imm(&mut self, value: u8) {
        self.op_imm(0x29, value);
    }

    fn cmp_imm(&mut self, value: u8) {
        self.op_imm(0xC9, value);
    }

    fn beq(&mut self, label: &str) {
        self.op_rel(0xF0, label);
    }

    fn bne(&mut self, label: &str) {
        self.op_rel(0xD0, label);
    }

    fn jmp_label(&mut self, label: &str) {
        self.op_abs_label(0x4C, label);
    }

    fn jsr_label(&mut self, label: &str) {
        self.op_abs_label(0x20, label);
    }

    fn inc_zp(&mut self, addr: u8) {
        self.op_zp(0xE6, addr);
    }

    fn inx(&mut self) {
        self.emit(0xE8);
    }

    fn txa(&mut self) {
        self.emit(0x8A);
    }

    fn txs(&mut self) {
        self.emit(0x9A);
    }

    fn pha(&mut self) {
        self.emit(0x48);
    }

    fn pla(&mut self) {
        self.emit(0x68);
    }

    fn rts(&mut self) {
        self.emit(0x60);
    }

    fn rti(&mut self) {
        self.emit(0x40);
    }

    fn sei(&mut self) {
        self.emit(0x78);
    }

    fn cld(&mut self) {
        self.emit(0xD8);
    }

    fn clc(&mut self) {
        self.emit(0x18);
    }

    fn sec(&mut self) {
        self.emit(0x38);
    }
}

fn build_chr_rom() -> Vec<u8> {
    let mut chr = vec![0; CHR_SIZE];

    for row in 0..8 {
        chr[16 + row] = if row % 2 == 0 {
            0b1010_1010
        } else {
            0b0101_0101
        };
        chr[16 + 8 + row] = 0;
    }

    chr
}

fn write_vector(prg: &mut [u8], vector_addr: u16, value: u16) {
    let index = (vector_addr - 0x8000) as usize;
    prg[index] = value as u8;
    prg[index + 1] = (value >> 8) as u8;
}

fn result_addr(test_id: u8) -> u16 {
    RESULT_BASE + test_id as u16 - 1
}

fn label_addr(labels: &HashMap<String, u16>, label: &str) -> Result<u16, String> {
    labels
        .get(label)
        .copied()
        .ok_or_else(|| format!("missing diagnostic label: {label}"))
}

fn cartridge_telemetry(rom: &[u8]) -> CartridgeTelemetry {
    let prg_start = 16;
    let prg = &rom[prg_start..prg_start + PRG_SIZE];
    CartridgeTelemetry {
        mapper: 0,
        prg_banks: PRG_BANKS,
        chr_banks: CHR_BANKS,
        size_bytes: rom.len(),
        reset_vector: read_vector(prg, 0xFFFC),
        nmi_vector: read_vector(prg, 0xFFFA),
        irq_vector: read_vector(prg, 0xFFFE),
        rom_hash: hash_bytes(rom),
    }
}

fn read_vector(prg: &[u8], vector_addr: u16) -> u16 {
    let index = (vector_addr - 0x8000) as usize;
    prg[index] as u16 | ((prg[index + 1] as u16) << 8)
}

fn apply_joypad_mask(bus: &mut Bus, mask: u8) {
    const BUTTONS: [JoypadButton; 8] = [
        JoypadButton::A,
        JoypadButton::B,
        JoypadButton::Select,
        JoypadButton::Start,
        JoypadButton::Up,
        JoypadButton::Down,
        JoypadButton::Left,
        JoypadButton::Right,
    ];

    for (index, button) in BUTTONS.into_iter().enumerate() {
        bus.joypad1
            .set_button_pressed(button, mask & (1 << index) != 0);
    }
}

fn read_ram_byte(bus: &mut Bus, addr: u8) -> u8 {
    bus.cpu_read(addr as u16)
}

fn test_telemetry(ram: &[u8]) -> Vec<TestTelemetry> {
    DIAGNOSTIC_TESTS
        .iter()
        .map(|spec| {
            let result = ram[((result_addr(spec.id)) & 0x07FF) as usize];
            TestTelemetry {
                id: spec.id,
                name: spec.name,
                result,
                passed: result == RESULT_PASS,
            }
        })
        .collect()
}

fn oam_telemetry(oam: &[u8; 256]) -> OamTelemetry {
    let expected: Vec<u8> = (0..=255).collect();
    OamTelemetry {
        checksum: hash_bytes(oam),
        expected_checksum: hash_bytes(&expected),
        first_16: oam[..16].to_vec(),
    }
}

fn frame_telemetry(frame: &[u32]) -> FrameTelemetry {
    let mut bytes = Vec::with_capacity(frame.len() * 4);
    let mut colors = BTreeSet::new();
    let mut nonzero_pixels = 0;
    for &pixel in frame {
        bytes.extend_from_slice(&pixel.to_le_bytes());
        colors.insert(pixel);
        if pixel != 0 {
            nonzero_pixels += 1;
        }
    }
    FrameTelemetry {
        checksum: hash_bytes(&bytes),
        unique_colors: colors.len(),
        nonzero_pixels,
    }
}

fn test_name(id: u8) -> Option<&'static str> {
    DIAGNOSTIC_TESTS
        .iter()
        .find(|spec| spec.id == id)
        .map(|spec| spec.name)
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;

    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_cartridge_is_valid_ines_and_ip_safe() {
        let rom = build_diagnostic_cartridge().expect("diagnostic cartridge builds");

        assert_eq!(&rom[0..4], b"NES\x1A");
        assert_eq!(rom[4], PRG_BANKS);
        assert_eq!(rom[5], CHR_BANKS);
        assert!(DIAGNOSTIC_PROVENANCE.contains("no ROM content"));
        Cartridge::new(&rom).expect("diagnostic cartridge should load");

        let info = cartridge_telemetry(&rom);
        assert_eq!(info.reset_vector, PROGRAM_BASE);
        assert!(info.nmi_vector >= PROGRAM_BASE);
        assert!(info.irq_vector >= PROGRAM_BASE);
    }

    #[test]
    fn headless_diagnostic_passes_and_collects_telemetry() {
        let telemetry = run_diagnostic(DiagnosticConfig::default()).expect("diagnostic runs");

        assert!(
            telemetry.verdict.passed,
            "diagnostic should pass: {:?}",
            telemetry.verdict.host_failures
        );
        assert_eq!(telemetry.ram.signature, 0xA5);
        assert!(telemetry.frames >= 2);
        assert!(telemetry.audio.sample_count > 0);
        assert!(telemetry.frame.unique_colors >= 2);
        assert!(telemetry.tests.iter().all(|test| test.passed));
    }
}
