use std::collections::{BTreeSet, HashMap};

use serde::Serialize;

use crate::bus::Bus;
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::joypad::JoypadButton;

pub const DIAGNOSTIC_PROVENANCE: &str =
    "Generated OxideNES diagnostic iNES cartridge: synthetic 6502 program and CHR patterns only, no ROM content.";
pub const DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION: u16 = 2;
pub const DIAGNOSTIC_SUITE_NAME: &str = "oxidenes_headless_diagnostic_cartridge";
pub const DIAGNOSTIC_SUITE_VERSION: &str = "diagnostic-cartridge-v2";

const DIAGNOSTIC_AI_GOALS: &[&str] = &[
    "headless end-to-end emulator validation",
    "machine-readable subsystem coverage",
    "failure localization for automated debugging",
];

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSubsystem {
    Cpu,
    Bus,
    Ppu,
    Apu,
    Dma,
    Joypad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticTestTier {
    Smoke,
    EdgeCase,
    Integration,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DiagnosticTestSpec {
    pub id: u8,
    pub name: &'static str,
    pub subsystem: DiagnosticSubsystem,
    pub tier: DiagnosticTestTier,
    pub intent: &'static str,
    pub expected_observations: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
struct DiagnosticFailureSpec {
    code: u8,
    test_id: u8,
    assertion: &'static str,
    expected: &'static str,
    observed: &'static str,
    likely_domain: &'static str,
    remediation_hint: &'static str,
}

pub const DIAGNOSTIC_TESTS: &[DiagnosticTestSpec] = &[
    DiagnosticTestSpec {
        id: 1,
        name: "cpu_arithmetic_flags",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::Smoke,
        intent: "Verify ADC/SBC arithmetic results and flag-driven carry/overflow behavior.",
        expected_observations: &["A register reaches 0x32, 0x20, and 0x80"],
    },
    DiagnosticTestSpec {
        id: 2,
        name: "stack_jsr_rts",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::Smoke,
        intent: "Verify stack push/pop and subroutine return through the CPU execution path.",
        expected_observations: &["PLA restores 0x42", "JSR/RTS returns 0x77"],
    },
    DiagnosticTestSpec {
        id: 3,
        name: "cpu_ram_mirroring",
        subsystem: DiagnosticSubsystem::Bus,
        tier: DiagnosticTestTier::Smoke,
        intent: "Verify 2 KiB CPU RAM mirrors across the $0000-$1FFF bus window.",
        expected_observations: &["$0002 mirrors to $0802", "$07FF mirrors to $1FFF"],
    },
    DiagnosticTestSpec {
        id: 4,
        name: "ppu_palette_register_roundtrip",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::Smoke,
        intent: "Verify palette writes can be read back through PPUADDR/PPUDATA.",
        expected_observations: &["palette byte 0x25 is read back masked to 6 bits"],
    },
    DiagnosticTestSpec {
        id: 5,
        name: "oam_dma_transfer",
        subsystem: DiagnosticSubsystem::Dma,
        tier: DiagnosticTestTier::Integration,
        intent: "Verify CPU-page OAM DMA transfers a full 256-byte pattern into PPU OAM.",
        expected_observations: &["OAM checksum matches ascending 0x00..0xFF pattern"],
    },
    DiagnosticTestSpec {
        id: 6,
        name: "apu_status_register",
        subsystem: DiagnosticSubsystem::Apu,
        tier: DiagnosticTestTier::Smoke,
        intent: "Verify enabling pulse channel 1 is reflected through the APU status register.",
        expected_observations: &["$4015 bit 0 remains set after pulse setup"],
    },
    DiagnosticTestSpec {
        id: 7,
        name: "joypad_strobe_shift",
        subsystem: DiagnosticSubsystem::Joypad,
        tier: DiagnosticTestTier::Smoke,
        intent: "Verify joypad strobe latches the configured A + Right button mask in read order.",
        expected_observations: &["read sequence is 1,0,0,0,0,0,0,1"],
    },
    DiagnosticTestSpec {
        id: 8,
        name: "cpu_branch_page_crossing",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify a taken relative branch can cross a CPU page boundary.",
        expected_observations: &[
            "BEQ target is reached from a branch placed at page low byte 0xFC",
        ],
    },
    DiagnosticTestSpec {
        id: 9,
        name: "joypad_overread_returns_one",
        subsystem: DiagnosticSubsystem::Joypad,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify joypad reads after the eighth latched button return 1.",
        expected_observations: &["ninth and tenth serial reads both return 1"],
    },
    DiagnosticTestSpec {
        id: 10,
        name: "ppu_nmi_and_render_frame",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::Integration,
        intent:
            "Verify PPU background rendering advances through NMI and produces host-visible frames.",
        expected_observations: &[
            "at least two NMIs",
            "rendered frame contains multiple colors",
        ],
    },
];

const DIAGNOSTIC_FAILURES: &[DiagnosticFailureSpec] = &[
    DiagnosticFailureSpec {
        code: 0x11,
        test_id: 1,
        assertion: "ADC without carry produces the expected accumulator value",
        expected: "A == 0x32 after 0x10 + 0x22",
        observed: "A differed from 0x32",
        likely_domain: "cpu.alu.adc",
        remediation_hint: "Inspect ADC immediate execution, carry input handling, and zero/negative flag side effects.",
    },
    DiagnosticFailureSpec {
        code: 0x12,
        test_id: 1,
        assertion: "SBC with carry set subtracts without borrow",
        expected: "A == 0x20 after 0x32 - 0x12",
        observed: "A differed from 0x20",
        likely_domain: "cpu.alu.sbc",
        remediation_hint: "Inspect SBC carry-as-not-borrow semantics and accumulator writeback.",
    },
    DiagnosticFailureSpec {
        code: 0x13,
        test_id: 1,
        assertion: "ADC crosses signed overflow into the negative range",
        expected: "A == 0x80 after 0x7F + 0x01",
        observed: "A differed from 0x80",
        likely_domain: "cpu.alu.adc_flags",
        remediation_hint: "Inspect ADC overflow/negative flag calculation and accumulator wrapping.",
    },
    DiagnosticFailureSpec {
        code: 0x21,
        test_id: 2,
        assertion: "PLA restores the byte pushed by PHA",
        expected: "A == 0x42 after PHA/PLA",
        observed: "A differed from 0x42",
        likely_domain: "cpu.stack",
        remediation_hint: "Inspect stack pointer pre/post increment behavior and stack page addressing.",
    },
    DiagnosticFailureSpec {
        code: 0x22,
        test_id: 2,
        assertion: "JSR/RTS returns from a subroutine with accumulator state intact",
        expected: "A == 0x77 after subroutine return",
        observed: "A differed from 0x77",
        likely_domain: "cpu.control_flow.stack",
        remediation_hint: "Inspect JSR return-address push order and RTS pull/increment behavior.",
    },
    DiagnosticFailureSpec {
        code: 0x31,
        test_id: 3,
        assertion: "CPU RAM mirror at $0802 reflects $0002",
        expected: "$0802 reads 0x5A after writing $0002",
        observed: "$0802 did not read 0x5A",
        likely_domain: "bus.cpu_ram_mirroring",
        remediation_hint: "Inspect CPU RAM address masking for the $0000-$1FFF range.",
    },
    DiagnosticFailureSpec {
        code: 0x32,
        test_id: 3,
        assertion: "CPU RAM mirror at $1FFF reflects $07FF",
        expected: "$1FFF reads 0xA5 after writing $07FF",
        observed: "$1FFF did not read 0xA5",
        likely_domain: "bus.cpu_ram_mirroring",
        remediation_hint: "Inspect high-end CPU RAM mirror masking and bus dispatch ordering.",
    },
    DiagnosticFailureSpec {
        code: 0x41,
        test_id: 4,
        assertion: "PPU palette byte round-trips through PPUADDR/PPUDATA",
        expected: "$3F00 reads back 0x25 masked to six bits",
        observed: "$2007 readback differed from 0x25",
        likely_domain: "ppu.registers.palette",
        remediation_hint: "Inspect PPU address latch handling, palette mirroring, and PPUDATA read/write paths.",
    },
    DiagnosticFailureSpec {
        code: 0x61,
        test_id: 6,
        assertion: "APU status reports pulse channel 1 enabled",
        expected: "$4015 bit 0 is set",
        observed: "$4015 bit 0 was clear",
        likely_domain: "apu.status",
        remediation_hint: "Inspect APU $4015 channel enable state and pulse channel length counter setup.",
    },
    DiagnosticFailureSpec {
        code: 0x70,
        test_id: 7,
        assertion: "Joypad serial read 0 returns the latched A button bit",
        expected: "$4016 read bit 0 == 1",
        observed: "$4016 read bit 0 was not 1",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad strobe latch behavior and button-bit mapping for A.",
    },
    DiagnosticFailureSpec {
        code: 0x71,
        test_id: 7,
        assertion: "Joypad serial read 1 returns the latched B button bit",
        expected: "$4016 read bit 0 == 0",
        observed: "$4016 read bit 0 was not 0",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift index advancement after the first read.",
    },
    DiagnosticFailureSpec {
        code: 0x72,
        test_id: 7,
        assertion: "Joypad serial read 2 returns the latched Select button bit",
        expected: "$4016 read bit 0 == 0",
        observed: "$4016 read bit 0 was not 0",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Select bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x73,
        test_id: 7,
        assertion: "Joypad serial read 3 returns the latched Start button bit",
        expected: "$4016 read bit 0 == 0",
        observed: "$4016 read bit 0 was not 0",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Start bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x74,
        test_id: 7,
        assertion: "Joypad serial read 4 returns the latched Up button bit",
        expected: "$4016 read bit 0 == 0",
        observed: "$4016 read bit 0 was not 0",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Up bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x75,
        test_id: 7,
        assertion: "Joypad serial read 5 returns the latched Down button bit",
        expected: "$4016 read bit 0 == 0",
        observed: "$4016 read bit 0 was not 0",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Down bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x76,
        test_id: 7,
        assertion: "Joypad serial read 6 returns the latched Left button bit",
        expected: "$4016 read bit 0 == 0",
        observed: "$4016 read bit 0 was not 0",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Left bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x77,
        test_id: 7,
        assertion: "Joypad serial read 7 returns the latched Right button bit",
        expected: "$4016 read bit 0 == 1",
        observed: "$4016 read bit 0 was not 1",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Right bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x81,
        test_id: 8,
        assertion: "BEQ is taken after CMP sets zero",
        expected: "branch target is reached",
        observed: "fallthrough path executed after BEQ",
        likely_domain: "cpu.branch",
        remediation_hint: "Inspect relative branch condition evaluation and program counter updates.",
    },
    DiagnosticFailureSpec {
        code: 0x82,
        test_id: 8,
        assertion: "Page-crossing branch target executes normally",
        expected: "A == 0x5C after reaching the branch target",
        observed: "A differed from 0x5C",
        likely_domain: "cpu.branch.page_cross",
        remediation_hint: "Inspect relative offset sign extension and branch target address calculation.",
    },
    DiagnosticFailureSpec {
        code: 0x90,
        test_id: 9,
        assertion: "Joypad serial read 8 returns one after all buttons are shifted",
        expected: "$4016 read bit 0 == 1",
        observed: "$4016 read bit 0 was not 1",
        likely_domain: "joypad.overread",
        remediation_hint: "Inspect joypad reads after index 7; NES-compatible overreads should return 1.",
    },
    DiagnosticFailureSpec {
        code: 0x91,
        test_id: 9,
        assertion: "Joypad serial read 9 keeps returning one after all buttons are shifted",
        expected: "$4016 read bit 0 == 1",
        observed: "$4016 read bit 0 was not 1",
        likely_domain: "joypad.overread",
        remediation_hint: "Inspect joypad overread behavior and make sure the shift index saturates or returns 1 after button 7.",
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
    pub schema_version: u16,
    pub provenance: &'static str,
    pub suite: DiagnosticSuiteTelemetry,
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
pub struct DiagnosticSuiteTelemetry {
    pub name: &'static str,
    pub version: &'static str,
    pub test_count: usize,
    pub goals: &'static [&'static str],
    pub failure_catalog: Vec<FailureCatalogTelemetry>,
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
    pub failure: Option<DiagnosticFailureTelemetry>,
    pub host_failures: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticFailureKind {
    CartridgeAssertion,
    Timeout,
    HostValidation,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticFailureTelemetry {
    pub kind: DiagnosticFailureKind,
    pub test_id: u8,
    pub test_name: Option<&'static str>,
    pub subsystem: Option<DiagnosticSubsystem>,
    pub tier: Option<DiagnosticTestTier>,
    pub failure_code: u8,
    pub failure_code_hex: String,
    pub assertion: String,
    pub expected: String,
    pub observed: String,
    pub likely_domain: String,
    pub remediation_hint: String,
}

#[derive(Debug, Serialize)]
pub struct FailureCatalogTelemetry {
    pub code: u8,
    pub code_hex: String,
    pub test_id: u8,
    pub test_name: Option<&'static str>,
    pub subsystem: Option<DiagnosticSubsystem>,
    pub assertion: &'static str,
    pub expected: &'static str,
    pub likely_domain: &'static str,
    pub remediation_hint: &'static str,
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
    pub subsystem: DiagnosticSubsystem,
    pub tier: DiagnosticTestTier,
    pub intent: &'static str,
    pub expected_observations: &'static [&'static str],
    pub result_addr: u16,
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

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventKind {
    Reset,
    TestChanged,
    StatusChanged,
    FrameComplete,
    PostPassFrameComplete,
}

#[derive(Debug, Serialize)]
pub struct EventTelemetry {
    pub kind: DiagnosticEventKind,
    pub cycle: u64,
    pub frame: u64,
    pub status: u8,
    pub current_test: u8,
    pub current_test_name: Option<&'static str>,
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
    let mut last_current_test = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
    let mut timeout = true;

    events.push(event_telemetry(
        0,
        0,
        last_status,
        last_current_test,
        cpu.pc,
        DiagnosticEventKind::Reset,
        "reset",
    ));

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
            let status = read_ram_byte(&mut bus, STATUS_ADDR);
            let current_test = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
            events.push(event_telemetry(
                cycles,
                frames,
                status,
                current_test,
                cpu.pc,
                DiagnosticEventKind::FrameComplete,
                "frame_complete",
            ));
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        let current_test = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
        if current_test != last_current_test {
            last_current_test = current_test;
            events.push(event_telemetry(
                cycles,
                frames,
                status,
                current_test,
                cpu.pc,
                DiagnosticEventKind::TestChanged,
                "test_changed",
            ));
        }
        if status != last_status {
            last_status = status;
            events.push(event_telemetry(
                cycles,
                frames,
                status,
                current_test,
                cpu.pc,
                DiagnosticEventKind::StatusChanged,
                "status_changed",
            ));
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
                let status = read_ram_byte(&mut bus, STATUS_ADDR);
                let current_test = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
                events.push(event_telemetry(
                    cycles,
                    frames,
                    status,
                    current_test,
                    cpu.pc,
                    DiagnosticEventKind::PostPassFrameComplete,
                    "post_pass_frame_complete",
                ));
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
    let failure = failure_telemetry(
        passed,
        status,
        timeout,
        current_test,
        failure_code,
        &host_failures,
    );

    Ok(DiagnosticTelemetry {
        schema_version: DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION,
        provenance: DIAGNOSTIC_PROVENANCE,
        suite: suite_telemetry(),
        cartridge: cartridge_info,
        verdict: VerdictTelemetry {
            passed,
            status,
            timeout,
            current_test,
            current_test_name: test_name(current_test),
            failure_code,
            failure,
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
    if input.ram[SIGNATURE_ADDR as usize] != 0xA5 {
        failures.push(format!(
            "signature byte mismatch: got 0x{:02X}",
            input.ram[SIGNATURE_ADDR as usize]
        ));
    }
    if input.status == STATUS_FAIL {
        return failures;
    }
    if input.status != STATUS_PASS {
        failures.push(format!(
            "cartridge status 0x{:02X} did not reach PASS",
            input.status
        ));
        return failures;
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
    program.cpu_branch_page_crossing();
    program.joypad_overread_returns_one();
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

    fn cpu_branch_page_crossing(&mut self) {
        self.begin_test(8);
        self.asm.lda_imm(0x00);
        self.asm.cmp_imm(0x00);
        self.asm.pad_until_low_byte(0xFC);
        let target = self.unique_label("branch_page_target");
        self.asm.beq(&target);
        self.asm.lda_imm(0x81);
        self.asm.sta_zp(FAILURE_CODE_ADDR);
        self.asm.jmp_label("fail");
        self.asm
            .label(&target)
            .expect("unique label should not collide");
        self.asm.lda_imm(0x5C);
        self.expect_a_eq(0x5C, 0x82);
        self.pass_test(8);
    }

    fn joypad_overread_returns_one(&mut self) {
        self.begin_test(9);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4016);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4016);

        for _ in 0..8 {
            self.asm.lda_abs(0x4016);
        }
        for index in 0..2 {
            self.asm.lda_abs(0x4016);
            self.asm.and_imm(0x01);
            self.expect_a_eq(0x01, 0x90 + index);
        }
        self.pass_test(9);
    }

    fn ppu_nmi_and_render_frame(&mut self) {
        self.begin_test(10);
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
        self.pass_test(10);
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

    fn current_addr(&self) -> u16 {
        self.base + self.bytes.len() as u16
    }

    fn pad_until_low_byte(&mut self, low_byte: u8) {
        while self.current_addr() as u8 != low_byte {
            self.nop();
        }
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

    fn nop(&mut self) {
        self.emit(0xEA);
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

fn suite_telemetry() -> DiagnosticSuiteTelemetry {
    DiagnosticSuiteTelemetry {
        name: DIAGNOSTIC_SUITE_NAME,
        version: DIAGNOSTIC_SUITE_VERSION,
        test_count: DIAGNOSTIC_TESTS.len(),
        goals: DIAGNOSTIC_AI_GOALS,
        failure_catalog: failure_catalog_telemetry(),
    }
}

fn failure_catalog_telemetry() -> Vec<FailureCatalogTelemetry> {
    DIAGNOSTIC_FAILURES
        .iter()
        .map(|failure| {
            let spec = test_spec(failure.test_id);
            FailureCatalogTelemetry {
                code: failure.code,
                code_hex: hex_byte(failure.code),
                test_id: failure.test_id,
                test_name: spec.map(|spec| spec.name),
                subsystem: spec.map(|spec| spec.subsystem),
                assertion: failure.assertion,
                expected: failure.expected,
                likely_domain: failure.likely_domain,
                remediation_hint: failure.remediation_hint,
            }
        })
        .collect()
}

fn failure_telemetry(
    passed: bool,
    status: u8,
    timeout: bool,
    current_test: u8,
    failure_code: u8,
    host_failures: &[String],
) -> Option<DiagnosticFailureTelemetry> {
    if passed {
        return None;
    }

    let current_spec = test_spec(current_test);
    if timeout {
        return Some(DiagnosticFailureTelemetry {
            kind: DiagnosticFailureKind::Timeout,
            test_id: current_test,
            test_name: current_spec.map(|spec| spec.name),
            subsystem: current_spec.map(|spec| spec.subsystem),
            tier: current_spec.map(|spec| spec.tier),
            failure_code,
            failure_code_hex: hex_byte(failure_code),
            assertion: "diagnostic cartridge completed before the cycle limit".to_string(),
            expected: "STATUS_PASS or STATUS_FAIL before max_cpu_cycles".to_string(),
            observed: format!(
                "status was {} while current_test was {}",
                hex_byte(status),
                current_test
            ),
            likely_domain: "emulator.progress_or_infinite_loop".to_string(),
            remediation_hint:
                "Inspect the current test transition events, CPU PC, and the subsystem under the active test."
                    .to_string(),
        });
    }

    if status == STATUS_FAIL {
        if let Some(failure) = failure_spec(failure_code) {
            let spec = test_spec(failure.test_id);
            return Some(DiagnosticFailureTelemetry {
                kind: DiagnosticFailureKind::CartridgeAssertion,
                test_id: failure.test_id,
                test_name: spec.map(|spec| spec.name),
                subsystem: spec.map(|spec| spec.subsystem),
                tier: spec.map(|spec| spec.tier),
                failure_code,
                failure_code_hex: hex_byte(failure_code),
                assertion: failure.assertion.to_string(),
                expected: failure.expected.to_string(),
                observed: failure.observed.to_string(),
                likely_domain: failure.likely_domain.to_string(),
                remediation_hint: failure.remediation_hint.to_string(),
            });
        }

        return Some(DiagnosticFailureTelemetry {
            kind: DiagnosticFailureKind::CartridgeAssertion,
            test_id: current_test,
            test_name: current_spec.map(|spec| spec.name),
            subsystem: current_spec.map(|spec| spec.subsystem),
            tier: current_spec.map(|spec| spec.tier),
            failure_code,
            failure_code_hex: hex_byte(failure_code),
            assertion: "diagnostic cartridge reported an unknown assertion failure".to_string(),
            expected: "failure code is present in the diagnostic failure catalog".to_string(),
            observed: format!("unknown failure code {}", hex_byte(failure_code)),
            likely_domain: "diagnostic.cartridge.failure_catalog".to_string(),
            remediation_hint:
                "Add this cartridge failure code to DIAGNOSTIC_FAILURES or inspect the failing test assembly."
                    .to_string(),
        });
    }

    Some(DiagnosticFailureTelemetry {
        kind: DiagnosticFailureKind::HostValidation,
        test_id: current_test,
        test_name: current_spec.map(|spec| spec.name),
        subsystem: current_spec.map(|spec| spec.subsystem),
        tier: current_spec.map(|spec| spec.tier),
        failure_code,
        failure_code_hex: hex_byte(failure_code),
        assertion: "host-side diagnostic validation completed without failures".to_string(),
        expected: "host_failures is empty after cartridge completion".to_string(),
        observed: if host_failures.is_empty() {
            "host validation failed without a detailed message".to_string()
        } else {
            host_failures.join("; ")
        },
        likely_domain: "host.validation".to_string(),
        remediation_hint:
            "Inspect host telemetry checks for OAM, frame, audio, RAM signature, and per-test result bytes."
                .to_string(),
    })
}

fn event_telemetry(
    cycle: u64,
    frame: u64,
    status: u8,
    current_test: u8,
    pc: u16,
    kind: DiagnosticEventKind,
    note: &str,
) -> EventTelemetry {
    EventTelemetry {
        kind,
        cycle,
        frame,
        status,
        current_test,
        current_test_name: test_name(current_test),
        pc,
        note: note.to_string(),
    }
}

fn test_telemetry(ram: &[u8]) -> Vec<TestTelemetry> {
    DIAGNOSTIC_TESTS
        .iter()
        .map(|spec| {
            let result_addr = result_addr(spec.id);
            let result = ram[(result_addr & 0x07FF) as usize];
            TestTelemetry {
                id: spec.id,
                name: spec.name,
                subsystem: spec.subsystem,
                tier: spec.tier,
                intent: spec.intent,
                expected_observations: spec.expected_observations,
                result_addr,
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
    test_spec(id).map(|spec| spec.name)
}

fn test_spec(id: u8) -> Option<&'static DiagnosticTestSpec> {
    DIAGNOSTIC_TESTS.iter().find(|spec| spec.id == id)
}

fn failure_spec(code: u8) -> Option<&'static DiagnosticFailureSpec> {
    DIAGNOSTIC_FAILURES
        .iter()
        .find(|failure| failure.code == code)
}

fn hex_byte(value: u8) -> String {
    format!("0x{value:02X}")
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
    fn diagnostic_test_metadata_is_unique_and_ai_readable() {
        let mut ids = BTreeSet::new();
        let mut has_edge_case = false;

        for spec in DIAGNOSTIC_TESTS {
            assert!(ids.insert(spec.id), "duplicate diagnostic id {}", spec.id);
            assert!(!spec.name.is_empty());
            assert!(!spec.intent.is_empty());
            assert!(!spec.expected_observations.is_empty());
            has_edge_case |= spec.tier == DiagnosticTestTier::EdgeCase;
        }

        assert!(has_edge_case, "diagnostic suite should include edge cases");
    }

    #[test]
    fn diagnostic_failure_catalog_is_unique_and_test_scoped() {
        let mut codes = BTreeSet::new();

        for failure in DIAGNOSTIC_FAILURES {
            assert!(
                codes.insert(failure.code),
                "duplicate failure code 0x{:02X}",
                failure.code
            );
            assert!(
                test_spec(failure.test_id).is_some(),
                "failure code 0x{:02X} references unknown test {}",
                failure.code,
                failure.test_id
            );
            assert!(!failure.assertion.is_empty());
            assert!(!failure.expected.is_empty());
            assert!(!failure.observed.is_empty());
            assert!(!failure.likely_domain.is_empty());
            assert!(!failure.remediation_hint.is_empty());
        }
    }

    #[test]
    fn headless_diagnostic_passes_and_collects_telemetry() {
        let telemetry = run_diagnostic(DiagnosticConfig::default()).expect("diagnostic runs");

        assert_eq!(
            telemetry.schema_version,
            DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION
        );
        assert_eq!(telemetry.suite.test_count, DIAGNOSTIC_TESTS.len());
        assert!(!telemetry.suite.failure_catalog.is_empty());
        assert!(
            telemetry.verdict.passed,
            "diagnostic should pass: {:?}",
            telemetry.verdict.host_failures
        );
        assert!(telemetry.verdict.failure.is_none());
        assert_eq!(telemetry.ram.signature, 0xA5);
        assert!(telemetry.frames >= 2);
        assert!(telemetry.audio.sample_count > 0);
        assert!(telemetry.frame.unique_colors >= 2);
        assert!(telemetry.tests.iter().all(|test| test.passed));
        assert!(telemetry.events.iter().any(|event| {
            matches!(event.kind, DiagnosticEventKind::TestChanged)
                && event.current_test_name == Some("cpu_branch_page_crossing")
        }));
    }
}
