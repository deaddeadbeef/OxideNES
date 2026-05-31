use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use serde::Serialize;
use serde_json::Value;

use crate::bus::{Bus, DmcDmaService};
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::joypad::JoypadButton;

pub const DIAGNOSTIC_PROVENANCE: &str =
    "Generated OxideNES diagnostic iNES cartridge: synthetic 6502 program and CHR patterns only, no ROM content.";
pub const DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION: u16 = 28;
pub const DIAGNOSTIC_SUITE_NAME: &str = "oxidenes_headless_diagnostic_cartridge";
pub const DIAGNOSTIC_SUITE_VERSION: &str = "diagnostic-cartridge-v28";

const DIAGNOSTIC_AI_GOALS: &[&str] = &[
    "headless end-to-end emulator validation",
    "machine-readable subsystem coverage",
    "failure localization for automated debugging",
    "structured expected-vs-observed probes for AI triage",
];

const PROGRAM_BASE: u16 = 0xC000;
const PRG_BANK_SIZE: usize = 16 * 1024;
const DIAGNOSTIC_MAPPER: u8 = 2;
const PRG_BANKS: u8 = 4;
const CHR_BANKS: u8 = 1;
const PRG_SIZE: usize = PRG_BANKS as usize * PRG_BANK_SIZE;
const CHR_SIZE: usize = CHR_BANKS as usize * 8 * 1024;
const PROGRAM_PRG_BANK: usize = PRG_BANKS as usize - 1;
const PROGRAM_PRG_OFFSET: usize = PROGRAM_PRG_BANK * PRG_BANK_SIZE;
const MAPPER2_SWITCHABLE_ADDR: u16 = 0x8000;
const MAPPER2_FIXED_SENTINEL_ADDR: u16 = 0xFF00;
const MAPPER2_BANK_SENTINELS: &[(u8, u8)] = &[(0, 0xA0), (1, 0xB1), (2, 0xC2)];
const MAPPER2_FIXED_SENTINEL: u8 = 0xD3;
const MAPPER2_PRG_RAM_LOW_ADDR: u16 = 0x6000;
const MAPPER2_PRG_RAM_HIGH_ADDR: u16 = 0x7FFF;
const MAPPER2_PRG_RAM_LOW_SENTINEL: u8 = 0x5C;
const MAPPER2_PRG_RAM_HIGH_SENTINEL: u8 = 0xA7;

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
const EXPECTED_JOYPAD1_MASK: u8 = 0x81;
const EXPECTED_JOYPAD2_MASK: u8 = 0x28;
const OAM_DMA_EXPECTED_MIN_CYCLES: u64 = 513;
const OAM_DMA_EXPECTED_MAX_CYCLES: u64 = 514;
const DMC_DMA_EXPECTED_MIN_OAM_OVERLAP_FETCHES: u64 = 1;
const DMC_DMA_EXPECTED_MIN_STALL_CYCLES: u8 = 3;
const DMC_DMA_EXPECTED_MAX_STALL_CYCLES: u8 = 4;
const INSTRUCTION_TRACE_TAIL_LIMIT: usize = 64;
const APU_STATUS_FAULT_LABEL: &str = "apu_status_register_before_status_read";
const CPU_ZERO_PAGE_WRAP_FAULT_LABEL: &str = "cpu_zero_page_index_wrap_before_read";
const CPU_INDIRECT_JMP_FAULT_LABEL: &str = "cpu_indirect_jmp_page_wrap_before_jump";
const DMA_OAM_TRANSFER_FAULT_LABEL: &str = "oam_dma_transfer_before_dma";
const JOYPAD_STROBE_RESET_FAULT_LABEL: &str = "joypad_strobe_reset_before_reset_read";
const MAPPER2_BANK_SWITCH_FAULT_LABEL: &str = "mapper2_prg_bank_switch_before_read";
const MAPPER2_PRG_RAM_FAULT_LABEL: &str = "mapper2_prg_ram_roundtrip_before_high_read";
const PPU_NAMETABLE_MIRRORING_FAULT_LABEL: &str =
    "ppu_horizontal_nametable_mirroring_before_first_mirror_read";
const PPU_NMI_TIMEOUT_FAULT_LABEL: &str = "ppu_nmi_render_frame_after_enable";
const PPU_READ_BUFFER_FAULT_LABEL: &str = "ppu_vram_read_buffer_before_first_read";
const PPU_VRAM_INCREMENT_32_FAULT_LABEL: &str = "ppu_vram_increment_32_before_stride_read";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSubsystem {
    Cpu,
    Bus,
    Ppu,
    Apu,
    Dma,
    Cartridge,
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

#[derive(Debug, Clone, Copy)]
struct DiagnosticCoverageGapSpec {
    id: &'static str,
    subsystem: &'static str,
    risk: &'static str,
    current_coverage: &'static str,
    missing_coverage: &'static str,
    suggested_next_test: &'static str,
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
        intent: "Verify CPU-page OAM DMA transfers a full 256-byte pattern while DMC sample DMA is active.",
        expected_observations: &[
            "OAM checksum matches ascending 0x00..0xFF pattern",
            "DMC DMA fetch overlaps the OAM DMA stall window",
        ],
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
    DiagnosticTestSpec {
        id: 11,
        name: "joypad2_strobe_shift",
        subsystem: DiagnosticSubsystem::Joypad,
        tier: DiagnosticTestTier::Integration,
        intent: "Verify the shared strobe latches an independent player-2 Start + Down mask through $4017.",
        expected_observations: &["player 2 read sequence is 0,0,0,1,0,1,0,0"],
    },
    DiagnosticTestSpec {
        id: 12,
        name: "cpu_zero_page_index_wrap",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify zero-page indexed addressing wraps inside page zero for reads and writes.",
        expected_observations: &[
            "LDA $FF,X with X=0x81 reads $0080",
            "STA $FF,X with X=0x81 writes $0080",
        ],
    },
    DiagnosticTestSpec {
        id: 13,
        name: "cpu_indirect_jmp_page_wrap",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify JMP ($xxFF) uses the original 6502 page-wrap high-byte read behavior.",
        expected_observations: &[
            "JMP ($04FF) reads target low byte at $04FF",
            "JMP ($04FF) reads target high byte from $0400 instead of $0500",
        ],
    },
    DiagnosticTestSpec {
        id: 14,
        name: "ppu_vram_read_buffer",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify non-palette PPUDATA reads are delayed through the PPU read buffer.",
        expected_observations: &[
            "first $2007 read from $2000 loads the internal read buffer",
            "second and third reads return the $2000 and $2001 VRAM sentinels",
        ],
    },
    DiagnosticTestSpec {
        id: 15,
        name: "mapper2_prg_bank_switch",
        subsystem: DiagnosticSubsystem::Cartridge,
        tier: DiagnosticTestTier::Integration,
        intent: "Verify Mapper 2/UXROM PRG bank switching through CPU-visible cartridge reads.",
        expected_observations: &[
            "$8000 exposes distinct sentinels after selecting switchable PRG banks 0, 1, and 2",
            "$FF00 remains mapped to the fixed final PRG bank after switchable bank writes",
        ],
    },
    DiagnosticTestSpec {
        id: 16,
        name: "mapper2_prg_ram_roundtrip",
        subsystem: DiagnosticSubsystem::Cartridge,
        tier: DiagnosticTestTier::Integration,
        intent: "Verify Mapper 2 PRG RAM reads and writes through the CPU $6000-$7FFF cartridge window.",
        expected_observations: &[
            "$6000 and $7FFF retain CPU-written PRG RAM sentinels",
            "PRG RAM remains visible after Mapper 2 switchable PRG bank writes",
        ],
    },
    DiagnosticTestSpec {
        id: 17,
        name: "ppu_horizontal_nametable_mirroring",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::Integration,
        intent: "Verify the Mapper 2 cartridge's horizontal nametable mirroring reaches CPU-driven PPU VRAM access.",
        expected_observations: &[
            "$2400 mirrors the sentinel written through $2000",
            "$2C00 mirrors the sentinel written through $2800",
            "$2000 and $2800 stay independent horizontal nametable pairs",
        ],
    },
    DiagnosticTestSpec {
        id: 18,
        name: "joypad_strobe_reset_midstream",
        subsystem: DiagnosticSubsystem::Joypad,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify a mid-stream $4016 strobe-high/strobe-low sequence resets the joypad serial index.",
        expected_observations: &[
            "first post-reset $4016 read returns the A button bit again",
            "second post-reset $4016 read advances to the B button bit",
        ],
    },
    DiagnosticTestSpec {
        id: 19,
        name: "ppu_vram_increment_32",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify PPUCTRL bit 2 makes PPUDATA writes and reads auto-increment VRAM by 32, then returns cleanly to increment-by-1 mode.",
        expected_observations: &[
            "second $2007 write with increment-by-32 lands at $2020",
            "clearing PPUCTRL bit 2 returns PPUDATA writes to increment-by-1 behavior",
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
        code: 0x78,
        test_id: 18,
        assertion: "Joypad strobe reset returns the A button bit again",
        expected: "first $4016 read after a second strobe sequence returns bit 0 == 1",
        observed: "$4016 did not restart at the A button bit after strobe reset",
        likely_domain: "joypad.strobe_reset",
        remediation_hint: "Inspect joypad $4016 writes; a high strobe write must reset the serial read index before returning to low strobe mode.",
    },
    DiagnosticFailureSpec {
        code: 0x79,
        test_id: 18,
        assertion: "Joypad strobe reset resumes serial advancement after the A bit",
        expected: "second $4016 read after reset returns the B button bit == 0",
        observed: "$4016 did not advance from A to B after the reset read",
        likely_domain: "joypad.strobe_reset",
        remediation_hint: "Inspect joypad read-index advancement after strobe is lowered; reads should resume serial shifting from the reset index.",
    },
    DiagnosticFailureSpec {
        code: 0x7A,
        test_id: 19,
        assertion: "PPUDATA write auto-increments by 32 when PPUCTRL bit 2 is set",
        expected: "second $2007 write after address $2000 is readable at $2020",
        observed: "$2020 did not contain the increment-by-32 sentinel",
        likely_domain: "ppu.registers.ppudata_increment_32",
        remediation_hint: "Inspect PPUCTRL bit 2 handling and PPUDATA write-side VRAM increment selection.",
    },
    DiagnosticFailureSpec {
        code: 0x7B,
        test_id: 19,
        assertion: "PPUDATA returns to increment-by-1 behavior after clearing PPUCTRL bit 2",
        expected: "second $2007 write after address $2100 is readable at $2101",
        observed: "$2101 did not contain the increment-by-1 sentinel",
        likely_domain: "ppu.registers.ppudata_increment",
        remediation_hint: "Inspect PPUCTRL writes and make sure clearing bit 2 restores the 1-byte PPUDATA increment.",
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
        code: 0xA0,
        test_id: 11,
        assertion: "Joypad 2 serial read 0 returns the latched A button bit",
        expected: "$4017 read bit 0 == 0",
        observed: "$4017 read bit 0 was not 0",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 bus dispatch and shared strobe latch behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xA1,
        test_id: 11,
        assertion: "Joypad 2 serial read 1 returns the latched B button bit",
        expected: "$4017 read bit 0 == 0",
        observed: "$4017 read bit 0 was not 0",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 shift index advancement after the first read.",
    },
    DiagnosticFailureSpec {
        code: 0xA2,
        test_id: 11,
        assertion: "Joypad 2 serial read 2 returns the latched Select button bit",
        expected: "$4017 read bit 0 == 0",
        observed: "$4017 read bit 0 was not 0",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 shift order and Select bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0xA3,
        test_id: 11,
        assertion: "Joypad 2 serial read 3 returns the latched Start button bit",
        expected: "$4017 read bit 0 == 1",
        observed: "$4017 read bit 0 was not 1",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 Start button mapping and $4017 reads.",
    },
    DiagnosticFailureSpec {
        code: 0xA4,
        test_id: 11,
        assertion: "Joypad 2 serial read 4 returns the latched Up button bit",
        expected: "$4017 read bit 0 == 0",
        observed: "$4017 read bit 0 was not 0",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 shift order and Up bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0xA5,
        test_id: 11,
        assertion: "Joypad 2 serial read 5 returns the latched Down button bit",
        expected: "$4017 read bit 0 == 1",
        observed: "$4017 read bit 0 was not 1",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 Down button mapping and $4017 reads.",
    },
    DiagnosticFailureSpec {
        code: 0xA6,
        test_id: 11,
        assertion: "Joypad 2 serial read 6 returns the latched Left button bit",
        expected: "$4017 read bit 0 == 0",
        observed: "$4017 read bit 0 was not 0",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 shift order and Left bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0xA7,
        test_id: 11,
        assertion: "Joypad 2 serial read 7 returns the latched Right button bit",
        expected: "$4017 read bit 0 == 0",
        observed: "$4017 read bit 0 was not 0",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 shift order and Right bit mapping.",
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
    DiagnosticFailureSpec {
        code: 0xB0,
        test_id: 12,
        assertion: "Zero-page indexed LDA wraps from $FF + X to $80",
        expected: "LDA $FF,X with X=0x81 reads the byte stored at $0080",
        observed: "A differed from the $0080 sentinel",
        likely_domain: "cpu.addressing.zero_page_x_wrap",
        remediation_hint: "Inspect zero-page indexed address calculation; the base plus index must wrap to 8 bits before the CPU RAM read.",
    },
    DiagnosticFailureSpec {
        code: 0xB1,
        test_id: 12,
        assertion: "Zero-page indexed STA wraps from $FF + X to $80",
        expected: "STA $FF,X with X=0x81 writes the byte at $0080",
        observed: "$0080 did not contain the store sentinel",
        likely_domain: "cpu.addressing.zero_page_x_wrap",
        remediation_hint: "Inspect zero-page indexed address calculation; the base plus index must wrap to 8 bits before the CPU RAM write.",
    },
    DiagnosticFailureSpec {
        code: 0xC0,
        test_id: 13,
        assertion: "Indirect JMP pointer at $04FF wraps the high-byte read to $0400",
        expected: "JMP ($04FF) reaches the page-wrap target",
        observed: "JMP ($04FF) read the non-wrapped high byte and reached the wrong target",
        likely_domain: "cpu.control_flow.indirect_jmp_page_wrap",
        remediation_hint: "Inspect JMP indirect addressing; when the pointer low byte is 0xFF, the high byte must be read from the same page at xx00.",
    },
    DiagnosticFailureSpec {
        code: 0xC1,
        test_id: 13,
        assertion: "Indirect JMP page-wrap target executes normally",
        expected: "A == 0x7B after reaching the wrapped target",
        observed: "A differed from the wrapped-target sentinel",
        likely_domain: "cpu.control_flow.indirect_jmp",
        remediation_hint: "Inspect JMP indirect target calculation and program-counter update after resolving the target address.",
    },
    DiagnosticFailureSpec {
        code: 0xD0,
        test_id: 14,
        assertion: "PPUDATA read from $2000 returns the buffered VRAM byte on the second read",
        expected: "second $2007 read after setting PPUADDR to $2000 returns the $2000 sentinel",
        observed: "$2007 readback differed from the $2000 sentinel",
        likely_domain: "ppu.registers.ppudata_buffer",
        remediation_hint: "Inspect PPUDATA non-palette read buffering; the first read should return the old buffer and load the addressed VRAM byte.",
    },
    DiagnosticFailureSpec {
        code: 0xD1,
        test_id: 14,
        assertion: "PPUDATA read auto-increments to $2001 after the buffered $2000 read",
        expected: "third $2007 read returns the $2001 sentinel",
        observed: "$2007 readback differed from the $2001 sentinel",
        likely_domain: "ppu.registers.ppudata_increment",
        remediation_hint: "Inspect PPUDATA read-side VRAM increment and buffer reload behavior after non-palette reads.",
    },
    DiagnosticFailureSpec {
        code: 0xE0,
        test_id: 17,
        assertion: "Horizontal nametable mirroring maps $2000 reads through $2400",
        expected: "$2400 reads the sentinel written to $2000",
        observed: "$2400 did not expose the $2000 horizontal-mirror sentinel",
        likely_domain: "ppu.nametables.horizontal_mirroring",
        remediation_hint: "Inspect cartridge mirroring metadata and PPU nametable VRAM index calculation for the $2000/$2400 horizontal mirror pair.",
    },
    DiagnosticFailureSpec {
        code: 0xE1,
        test_id: 17,
        assertion: "Horizontal nametable mirroring maps $2800 reads through $2C00",
        expected: "$2C00 reads the sentinel written to $2800",
        observed: "$2C00 did not expose the $2800 horizontal-mirror sentinel",
        likely_domain: "ppu.nametables.horizontal_mirroring",
        remediation_hint: "Inspect cartridge mirroring metadata and PPU nametable VRAM index calculation for the $2800/$2C00 horizontal mirror pair.",
    },
    DiagnosticFailureSpec {
        code: 0xE2,
        test_id: 17,
        assertion: "Horizontal nametable mirror pairs remain independent",
        expected: "$2000 still reads its sentinel after writing the $2800 mirror pair",
        observed: "$2000 changed after writing the $2800/$2C00 horizontal mirror pair",
        likely_domain: "ppu.nametables.horizontal_mirroring",
        remediation_hint: "Inspect horizontal mirroring pair isolation; $2000/$2400 should not alias $2800/$2C00.",
    },
    DiagnosticFailureSpec {
        code: 0xF0,
        test_id: 15,
        assertion: "Mapper 2 switchable PRG window starts on bank 0",
        expected: "$8000 reads the bank-0 sentinel after selecting PRG bank 0",
        observed: "$8000 did not expose the bank-0 sentinel",
        likely_domain: "mapper.uxrom.prg_bank_switch",
        remediation_hint: "Inspect Mapper 2 bank-select initialization and CPU $8000-$BFFF read mapping.",
    },
    DiagnosticFailureSpec {
        code: 0xF1,
        test_id: 15,
        assertion: "Mapper 2 switches the $8000-$BFFF PRG window to bank 1",
        expected: "$8000 reads the bank-1 sentinel after writing bank select 1",
        observed: "$8000 did not expose the bank-1 sentinel",
        likely_domain: "mapper.uxrom.prg_bank_switch",
        remediation_hint: "Inspect Mapper 2 PRG write handling and modulo selection for the switchable bank window.",
    },
    DiagnosticFailureSpec {
        code: 0xF2,
        test_id: 15,
        assertion: "Mapper 2 switches the $8000-$BFFF PRG window to bank 2",
        expected: "$8000 reads the bank-2 sentinel after writing bank select 2",
        observed: "$8000 did not expose the bank-2 sentinel",
        likely_domain: "mapper.uxrom.prg_bank_switch",
        remediation_hint: "Inspect Mapper 2 PRG bank indexing for nonzero switchable banks.",
    },
    DiagnosticFailureSpec {
        code: 0xF3,
        test_id: 15,
        assertion: "Mapper 2 keeps the final PRG bank fixed at $C000-$FFFF",
        expected: "$FF00 reads the fixed-bank sentinel after switchable bank writes",
        observed: "$FF00 did not expose the fixed final-bank sentinel",
        likely_domain: "mapper.uxrom.fixed_prg_bank",
        remediation_hint: "Inspect Mapper 2 fixed-bank mapping for CPU $C000-$FFFF reads.",
    },
    DiagnosticFailureSpec {
        code: 0xF4,
        test_id: 16,
        assertion: "Mapper 2 PRG RAM lower boundary round-trips through $6000",
        expected: "$6000 reads the low PRG RAM sentinel after a CPU write",
        observed: "$6000 did not expose the low PRG RAM sentinel",
        likely_domain: "mapper.uxrom.prg_ram",
        remediation_hint: "Inspect Mapper 2 CPU $6000-$7FFF PRG RAM read/write dispatch and address masking.",
    },
    DiagnosticFailureSpec {
        code: 0xF5,
        test_id: 16,
        assertion: "Mapper 2 PRG RAM upper boundary round-trips through $7FFF",
        expected: "$7FFF reads the high PRG RAM sentinel after a CPU write",
        observed: "$7FFF did not expose the high PRG RAM sentinel",
        likely_domain: "mapper.uxrom.prg_ram",
        remediation_hint: "Inspect Mapper 2 PRG RAM upper-bound indexing for CPU $7FFF reads and writes.",
    },
    DiagnosticFailureSpec {
        code: 0xF6,
        test_id: 16,
        assertion: "Mapper 2 PRG RAM persists across switchable PRG bank writes",
        expected: "$6000 still reads the low PRG RAM sentinel after changing the PRG bank select",
        observed: "$6000 changed after Mapper 2 bank-select writes",
        likely_domain: "mapper.uxrom.prg_ram",
        remediation_hint: "Inspect Mapper 2 bank-select writes; they must not mutate or remap the $6000-$7FFF PRG RAM window.",
    },
];

const DIAGNOSTIC_COVERAGE_GAPS: &[DiagnosticCoverageGapSpec] = &[
    DiagnosticCoverageGapSpec {
        id: "cpu_opcode_matrix",
        subsystem: "cpu",
        risk: "The cartridge proves selected CPU execution paths, not full 6502 opcode/addressing-mode compatibility.",
        current_coverage: "ADC/SBC arithmetic, flags, stack push/pop, JSR/RTS, a taken page-crossing branch, zero-page indexed wraparound, and indirect JMP page-wrap behavior.",
        missing_coverage: "Complete official opcode matrix, illegal opcodes, interrupt priority edge cases, broader addressing-mode combinations, and cycle-accurate addressing penalties.",
        suggested_next_test: "Generate an opcode/addressing-mode matrix cartridge that records accumulator, flags, memory side effects, and cycle buckets per case.",
    },
    DiagnosticCoverageGapSpec {
        id: "ppu_pixel_pipeline",
        subsystem: "ppu",
        risk: "The cartridge catches gross PPU progress and palette behavior but does not prove detailed scanline/pixel correctness.",
        current_coverage: "Palette register round-trip, non-palette PPUDATA read buffering, PPUDATA increment-by-32 register behavior, horizontal nametable mirroring, NMI delivery, completed frames, and host-visible multi-color background output.",
        missing_coverage: "Sprite evaluation, sprite/background priority, scrolling seams, vblank timing, and per-dot rendering behavior.",
        suggested_next_test: "Add deterministic background/sprite scenes with expected frame checksums and targeted sprite-priority probes.",
    },
    DiagnosticCoverageGapSpec {
        id: "mapper_banking_runtime",
        subsystem: "cartridge",
        risk: "The diagnostic cartridge now exercises one PRG bank-switching mapper, but broader mapper behavior can still regress outside this fixture.",
        current_coverage: "The generated Mapper 2/UXROM cartridge validates CPU-visible PRG bank switching, the fixed final-bank window, PRG RAM round-trips, and header-declared horizontal nametable mirroring end to end.",
        missing_coverage: "Runtime CHR bank switching, IRQ-generating mappers, other mirroring modes, MMC register edge cases, and battery-backed RAM persistence.",
        suggested_next_test: "Generate additional mapper-specific synthetic cartridges for supported mappers and assert bank-visible sentinels from CPU and PPU paths.",
    },
    DiagnosticCoverageGapSpec {
        id: "apu_audio_depth",
        subsystem: "apu",
        risk: "The cartridge proves APU status and sample production, not channel accuracy or mixer behavior.",
        current_coverage: "$4015 pulse enable status and nonzero drained audio samples at frame boundaries.",
        missing_coverage: "Envelope, sweep, triangle/noise/DMC behavior, frame counter timing, mixer levels, and IRQ edge cases.",
        suggested_next_test: "Add per-channel register programs with host-side waveform windows, sample-count ranges, and peak/RMS expectations.",
    },
    DiagnosticCoverageGapSpec {
        id: "dma_cycle_timing",
        subsystem: "dma",
        risk: "The cartridge validates OAM contents and host-observed OAM DMA stall length, but not all DMA interactions.",
        current_coverage: "A full-page OAM DMA transfer produces the expected OAM checksum, stalls CPU execution for a 513-514 cycle bucket, records first active-cycle parity, observes DMC sample DMA during the OAM stall window, and validates the phase-specific 3-4 cycle DMC stall bucket.",
        missing_coverage: "Alternate odd/even OAM start-phase fixtures, multiple DMC overlap positions inside one transfer, and deeper CPU/APU interleaving across repeated DMA bursts.",
        suggested_next_test: "Add paired OAM DMA fixtures that force both CPU start parities and compare DMC overlap placement near the beginning, middle, and end of the OAM transfer.",
    },
    DiagnosticCoverageGapSpec {
        id: "input_port_matrix",
        subsystem: "joypad",
        risk: "The cartridge proves fixed serial-read masks for both controller ports but not the full input state matrix.",
        current_coverage: "Joypad 1 strobe/shift sequence for A + Right, joypad 1 mid-stream strobe reset behavior, joypad 1 overreads after the eighth latched button, and joypad 2 strobe/shift sequence for Start + Down.",
        missing_coverage: "Multiple masks per port, simultaneous opposite directions, disconnected input defaults, and host input remapping.",
        suggested_next_test: "Run the serial-read program across a generated mask table for both ports, including mid-stream strobe toggles.",
    },
];

#[derive(Debug, Clone)]
pub struct DiagnosticConfig {
    pub max_cpu_cycles: u64,
    pub joypad1_mask: u8,
    pub joypad2_mask: u8,
    pub fault_injection: Option<DiagnosticFaultInjection>,
}

impl Default for DiagnosticConfig {
    fn default() -> Self {
        Self {
            max_cpu_cycles: 500_000,
            joypad1_mask: EXPECTED_JOYPAD1_MASK,
            joypad2_mask: EXPECTED_JOYPAD2_MASK,
            fault_injection: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticFaultInjection {
    ApuStatusRegister,
    CpuIndirectJmpPageWrap,
    CpuZeroPageIndexWrap,
    DmaOamTransfer,
    JoypadStrobeReset,
    Mapper2PrgBankSwitch,
    Mapper2PrgRam,
    PpuNametableMirroring,
    PpuNmiTimeout,
    PpuVramIncrement32,
    PpuVramReadBuffer,
}

impl DiagnosticFaultInjection {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticFaultInjection::ApuStatusRegister => "apu_status_register",
            DiagnosticFaultInjection::CpuIndirectJmpPageWrap => "cpu_indirect_jmp_page_wrap",
            DiagnosticFaultInjection::CpuZeroPageIndexWrap => "cpu_zero_page_index_wrap",
            DiagnosticFaultInjection::DmaOamTransfer => "dma_oam_transfer",
            DiagnosticFaultInjection::JoypadStrobeReset => "joypad_strobe_reset",
            DiagnosticFaultInjection::Mapper2PrgBankSwitch => "mapper2_prg_bank_switch",
            DiagnosticFaultInjection::Mapper2PrgRam => "mapper2_prg_ram",
            DiagnosticFaultInjection::PpuNametableMirroring => "ppu_nametable_mirroring",
            DiagnosticFaultInjection::PpuNmiTimeout => "ppu_nmi_timeout",
            DiagnosticFaultInjection::PpuVramIncrement32 => "ppu_vram_increment_32",
            DiagnosticFaultInjection::PpuVramReadBuffer => "ppu_vram_read_buffer",
        }
    }

    fn injection_label(self) -> &'static str {
        match self {
            DiagnosticFaultInjection::ApuStatusRegister => APU_STATUS_FAULT_LABEL,
            DiagnosticFaultInjection::CpuIndirectJmpPageWrap => CPU_INDIRECT_JMP_FAULT_LABEL,
            DiagnosticFaultInjection::CpuZeroPageIndexWrap => CPU_ZERO_PAGE_WRAP_FAULT_LABEL,
            DiagnosticFaultInjection::DmaOamTransfer => DMA_OAM_TRANSFER_FAULT_LABEL,
            DiagnosticFaultInjection::JoypadStrobeReset => JOYPAD_STROBE_RESET_FAULT_LABEL,
            DiagnosticFaultInjection::Mapper2PrgBankSwitch => MAPPER2_BANK_SWITCH_FAULT_LABEL,
            DiagnosticFaultInjection::Mapper2PrgRam => MAPPER2_PRG_RAM_FAULT_LABEL,
            DiagnosticFaultInjection::PpuNametableMirroring => PPU_NAMETABLE_MIRRORING_FAULT_LABEL,
            DiagnosticFaultInjection::PpuNmiTimeout => PPU_NMI_TIMEOUT_FAULT_LABEL,
            DiagnosticFaultInjection::PpuVramIncrement32 => PPU_VRAM_INCREMENT_32_FAULT_LABEL,
            DiagnosticFaultInjection::PpuVramReadBuffer => PPU_READ_BUFFER_FAULT_LABEL,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DiagnosticTelemetry {
    pub schema_version: u16,
    pub provenance: &'static str,
    pub suite: DiagnosticSuiteTelemetry,
    pub cartridge: CartridgeTelemetry,
    pub input: DiagnosticInputTelemetry,
    pub verdict: VerdictTelemetry,
    pub analysis: DiagnosticAnalysisTelemetry,
    pub cycles: u64,
    pub frames: u64,
    pub cpu: CpuTelemetry,
    pub ram: RamTelemetry,
    pub tests: Vec<TestTelemetry>,
    pub timeline: Vec<TestTimelineTelemetry>,
    pub probes: Vec<DiagnosticProbeTelemetry>,
    pub dma: DmaTelemetry,
    pub oam: OamTelemetry,
    pub frame: FrameTelemetry,
    pub audio: AudioTelemetry,
    pub instruction_trace: InstructionTraceTelemetry,
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
pub struct DiagnosticInputTelemetry {
    pub joypad1_mask: u8,
    pub joypad1_mask_hex: String,
    pub joypad1_expected_mask: u8,
    pub joypad1_expected_mask_hex: String,
    pub joypad2_mask: u8,
    pub joypad2_mask_hex: String,
    pub joypad2_expected_mask: u8,
    pub joypad2_expected_mask_hex: String,
    pub fault_injection: Option<DiagnosticFaultInjection>,
    pub fault_injection_label: Option<&'static str>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticHealth {
    Healthy,
    CartridgeAssertionFailed,
    TimedOut,
    HostValidationFailed,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticAnalysisTelemetry {
    pub health: DiagnosticHealth,
    pub summary: String,
    pub debug_focus: DiagnosticDebugFocusTelemetry,
    pub coverage: DiagnosticCoverageTelemetry,
    pub coverage_gaps: Vec<DiagnosticCoverageGapTelemetry>,
    pub timing: DiagnosticTimingSummaryTelemetry,
    pub probe_summary: DiagnosticProbeSummaryTelemetry,
    pub failing_subsystem: Option<DiagnosticSubsystem>,
    pub failing_test: Option<&'static str>,
    pub first_failure_domain: Option<String>,
    pub next_actions: Vec<String>,
    pub test_transition_count: usize,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticDebugFocusTelemetry {
    pub health: DiagnosticHealth,
    pub focus_test_id: u8,
    pub focus_test_name: Option<&'static str>,
    pub focus_subsystem: Option<DiagnosticSubsystem>,
    pub focus_domain: Option<String>,
    pub failure_kind: Option<DiagnosticFailureKind>,
    pub failure_code_hex: String,
    pub failed_probe_ids: Vec<String>,
    pub skipped_probe_count: usize,
    pub last_event: Option<DiagnosticDebugEventFocusTelemetry>,
    pub terminal_instruction: Option<DiagnosticDebugInstructionFocusTelemetry>,
    pub last_test_instruction: Option<DiagnosticDebugInstructionFocusTelemetry>,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticDebugEventFocusTelemetry {
    pub kind: DiagnosticEventKind,
    pub cycle: u64,
    pub frame: u64,
    pub status_hex: String,
    pub current_test: u8,
    pub current_test_name: Option<&'static str>,
    pub pc_hex: String,
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticDebugInstructionFocusTelemetry {
    pub sequence: u64,
    pub cycle: u64,
    pub frame: u64,
    pub current_test: u8,
    pub current_test_name: Option<&'static str>,
    pub pc_hex: String,
    pub instruction: Option<String>,
    pub symbol: Option<String>,
    pub status_hex: String,
    pub current_result_hex: Option<String>,
    pub failure_code_hex: String,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticCoverageGapTelemetry {
    pub id: &'static str,
    pub subsystem: &'static str,
    pub risk: &'static str,
    pub current_coverage: &'static str,
    pub missing_coverage: &'static str,
    pub suggested_next_test: &'static str,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticCoverageTelemetry {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub subsystem_summary: Vec<SubsystemCoverageTelemetry>,
    pub tier_summary: Vec<TierCoverageTelemetry>,
}

#[derive(Debug, Serialize)]
pub struct SubsystemCoverageTelemetry {
    pub subsystem: DiagnosticSubsystem,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Debug, Serialize)]
pub struct TierCoverageTelemetry {
    pub tier: DiagnosticTestTier,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticTimingSummaryTelemetry {
    pub started_tests: usize,
    pub ended_tests: usize,
    pub not_started_tests: usize,
    pub timed_out_tests: usize,
    pub slowest_test: Option<TestDurationTelemetry>,
}

#[derive(Debug, Serialize)]
pub struct TestDurationTelemetry {
    pub test_id: u8,
    pub test_name: &'static str,
    pub subsystem: DiagnosticSubsystem,
    pub tier: DiagnosticTestTier,
    pub duration_cycles: u64,
    pub duration_frames: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticProbeSource {
    CartridgeResult,
    HostObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticProbeStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticProbeTelemetry {
    pub id: String,
    pub source: DiagnosticProbeSource,
    pub subsystem: Option<DiagnosticSubsystem>,
    pub test_id: Option<u8>,
    pub test_name: Option<&'static str>,
    pub status: DiagnosticProbeStatus,
    pub description: String,
    pub expected: String,
    pub observed: String,
    pub likely_domain: String,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticProbeSummaryTelemetry {
    pub total_probes: usize,
    pub passed_probes: usize,
    pub failed_probes: usize,
    pub skipped_probes: usize,
    pub first_failed_probe: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct CpuTelemetry {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub status: u8,
    pub pending_cycles: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticRamWatchTelemetry {
    pub status: u8,
    pub status_hex: String,
    pub current_test: u8,
    pub current_test_name: Option<&'static str>,
    pub failure_code: u8,
    pub failure_code_hex: String,
    pub signature: u8,
    pub signature_hex: String,
    pub nmi_count: u8,
    pub current_result_addr: Option<u16>,
    pub current_result_addr_hex: Option<String>,
    pub current_result: Option<u8>,
    pub current_result_hex: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RamTelemetry {
    pub signature: u8,
    pub nmi_count: u8,
    pub checksum: u64,
    pub result_base: u16,
}

#[derive(Debug, Serialize)]
pub struct DmaTelemetry {
    pub oam_dma_observed: bool,
    pub oam_dma_completed: bool,
    pub oam_dma_active_cycles: u64,
    pub oam_dma_expected_min_cycles: u64,
    pub oam_dma_expected_max_cycles: u64,
    pub oam_dma_start_cycle: Option<u64>,
    pub oam_dma_end_cycle: Option<u64>,
    pub oam_dma_first_active_cycle: Option<u64>,
    pub oam_dma_first_active_cycle_parity: Option<&'static str>,
    pub oam_dma_start_test: Option<u8>,
    pub oam_dma_start_test_name: Option<&'static str>,
    pub oam_dma_end_test: Option<u8>,
    pub oam_dma_end_test_name: Option<&'static str>,
    pub dmc_dma_fetches_observed: u64,
    pub dmc_dma_fetches_during_oam_dma: u64,
    pub dmc_dma_expected_min_oam_overlap_fetches: u64,
    pub dmc_dma_oam_overlap_observed: bool,
    pub dmc_dma_first_fetch_cycle: Option<u64>,
    pub dmc_dma_first_fetch_address: Option<u16>,
    pub dmc_dma_first_fetch_cpu_cycle_parity: Option<&'static str>,
    pub dmc_dma_first_fetch_stall_cycles: Option<u8>,
    pub dmc_dma_first_oam_overlap_cycle: Option<u64>,
    pub dmc_dma_first_oam_overlap_test: Option<u8>,
    pub dmc_dma_first_oam_overlap_test_name: Option<&'static str>,
    pub dmc_dma_first_oam_overlap_cpu_cycle_parity: Option<&'static str>,
    pub dmc_dma_first_oam_overlap_stall_cycles: Option<u8>,
    pub dmc_dma_three_cycle_fetches: u64,
    pub dmc_dma_four_cycle_fetches: u64,
    pub dmc_dma_expected_min_stall_cycles: u8,
    pub dmc_dma_expected_max_stall_cycles: u8,
    pub dmc_dma_stall_cycles: u64,
    pub dmc_dma_stall_cycles_after_oam_dma: u64,
    pub dmc_dma_queued_during_oam_dma_cycles: u64,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TestTimelineOutcome {
    NotStarted,
    Passed,
    Failed,
    TimedOut,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TestTimelineEndReason {
    NextTestStarted,
    CartridgePassed,
    CartridgeFailed,
    Timeout,
}

#[derive(Debug, Serialize)]
pub struct TestTimelineTelemetry {
    pub test_id: u8,
    pub test_name: &'static str,
    pub subsystem: DiagnosticSubsystem,
    pub tier: DiagnosticTestTier,
    pub outcome: TestTimelineOutcome,
    pub started: bool,
    pub ended: bool,
    pub start_cycle: Option<u64>,
    pub end_cycle: Option<u64>,
    pub duration_cycles: Option<u64>,
    pub start_frame: Option<u64>,
    pub end_frame: Option<u64>,
    pub duration_frames: Option<u64>,
    pub end_reason: Option<TestTimelineEndReason>,
    pub terminal_status: Option<u8>,
    pub terminal_status_hex: Option<String>,
    pub terminal_pc: Option<u16>,
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
pub struct InstructionTraceTelemetry {
    pub captured_instruction_count: u64,
    pub retained_instruction_count: usize,
    pub retention_limit: usize,
    pub truncated: bool,
    pub tail: Vec<InstructionTraceEntryTelemetry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstructionTraceEntryTelemetry {
    pub sequence: u64,
    pub cycle: u64,
    pub frame: u64,
    pub pc: u16,
    pub pc_hex: String,
    pub opcode: Option<u8>,
    pub opcode_hex: Option<String>,
    pub instruction: Option<InstructionDecodeTelemetry>,
    pub symbol: Option<DiagnosticSymbolTelemetry>,
    pub cpu: CpuTelemetry,
    pub diagnostic_ram: DiagnosticRamWatchTelemetry,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstructionDecodeTelemetry {
    pub mnemonic: &'static str,
    pub addressing_mode: &'static str,
    pub byte_len: u8,
    pub operand_bytes: Vec<u8>,
    pub operand_hex: Vec<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagnosticSymbolTelemetry {
    pub name: String,
    pub address: u16,
    pub address_hex: String,
    pub offset: u16,
    pub offset_hex: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticEventKind {
    Reset,
    TestChanged,
    StatusChanged,
    OamDmaStarted,
    OamDmaCompleted,
    DmcDmaFetched,
    DmcDmaOamOverlap,
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
    pub cpu: CpuTelemetry,
    pub diagnostic_ram: DiagnosticRamWatchTelemetry,
    pub note: String,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticComparisonTelemetry {
    pub baseline_schema_version: Option<u64>,
    pub current_schema_version: u16,
    pub passed: bool,
    pub summary: String,
    pub difference_count: usize,
    pub failure_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub differences: Vec<DiagnosticComparisonDifferenceTelemetry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticComparisonSeverity {
    Failure,
    Warning,
    Info,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticComparisonDifferenceTelemetry {
    pub severity: DiagnosticComparisonSeverity,
    pub category: &'static str,
    pub path: String,
    pub baseline: Option<String>,
    pub current: Option<String>,
    pub note: String,
}

pub fn build_diagnostic_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_program_with_labels()?;
    build_diagnostic_cartridge_from_program(&program, &labels)
}

fn build_diagnostic_cartridge_from_program(
    program: &[u8],
    labels: &HashMap<String, u16>,
) -> Result<Vec<u8>, String> {
    if program.len() > PRG_BANK_SIZE {
        return Err(format!(
            "diagnostic program is too large: {} bytes > {} bytes",
            program.len(),
            PRG_BANK_SIZE
        ));
    }

    let mut rom = Vec::with_capacity(16 + PRG_SIZE + CHR_SIZE);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(PRG_BANKS);
    rom.push(CHR_BANKS);
    rom.push((DIAGNOSTIC_MAPPER & 0x0F) << 4); // Mapper 2, horizontal mirroring.
    rom.push(DIAGNOSTIC_MAPPER & 0xF0);
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; PRG_SIZE];
    for (bank, sentinel) in MAPPER2_BANK_SENTINELS {
        prg[*bank as usize * PRG_BANK_SIZE] = *sentinel;
    }
    write_prg_cpu_byte(
        &mut prg,
        MAPPER2_FIXED_SENTINEL_ADDR,
        MAPPER2_FIXED_SENTINEL,
    );
    prg[PROGRAM_PRG_OFFSET..PROGRAM_PRG_OFFSET + program.len()].copy_from_slice(program);
    write_vector(&mut prg, 0xFFFA, label_addr(labels, "nmi")?);
    write_vector(&mut prg, 0xFFFC, PROGRAM_BASE);
    write_vector(&mut prg, 0xFFFE, label_addr(labels, "irq")?);
    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&build_chr_rom());
    Ok(rom)
}

pub fn run_diagnostic(config: DiagnosticConfig) -> Result<DiagnosticTelemetry, String> {
    let (program, labels) = build_program_with_labels()?;
    let trace_context = DiagnosticTraceContext::from_labels(&labels);
    let fault_injection_pc = match config.fault_injection {
        Some(fault) => Some(label_addr(&labels, fault.injection_label())?),
        None => None,
    };
    let rom = build_diagnostic_cartridge_from_program(&program, &labels)?;
    let cartridge_info = cartridge_telemetry(&rom);
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    apply_joypad_mask(&mut bus, config.joypad1_mask);
    apply_joypad2_mask(&mut bus, config.joypad2_mask);

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
    let mut dma_observation = DmaObservation::default();
    let mut instruction_trace = InstructionTraceObservation::default();
    let mut fault_injected = false;

    let reset_cpu = cpu_telemetry(&cpu);
    let reset_ram = diagnostic_ram_watch_telemetry(&mut bus, last_status, last_current_test);
    events.push(event_telemetry(EventTelemetryInput {
        cycle: 0,
        frame: 0,
        status: last_status,
        current_test: last_current_test,
        pc: cpu.pc,
        cpu: reset_cpu,
        diagnostic_ram: reset_ram,
        kind: DiagnosticEventKind::Reset,
        note: "reset",
    }));

    while cycles < config.max_cpu_cycles {
        maybe_apply_diagnostic_fault_injection(
            &mut bus,
            config.fault_injection,
            fault_injection_pc,
            &mut fault_injected,
            cpu.pc,
        );
        observe_instruction_trace(
            &mut instruction_trace,
            &trace_context,
            &mut bus,
            &cpu,
            cycles,
            frames,
        );
        let dma_active_before = bus.dma_active();
        let dmc_stall_before = bus.dmc_stall_active();
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        let dmc_dma_service = bus.service_dmc_dma(cpu.is_odd_cycle());
        cycles += 1;

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        let current_test = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
        let cpu_snapshot = cpu_telemetry(&cpu);
        let diagnostic_ram = diagnostic_ram_watch_telemetry(&mut bus, status, current_test);
        let dma_active_after = bus.dma_active();
        dma_observation.observe_tick(DmaTickObservation {
            cycle: cycles,
            frame: frames,
            status,
            current_test,
            pc: cpu.pc,
            cpu: cpu_snapshot,
            diagnostic_ram: diagnostic_ram.clone(),
            active_before: dma_active_before,
            active_after: dma_active_after,
            dmc_stall_before,
            dmc_stall_after: bus.dmc_stall_active(),
            dmc_dma_service,
            events: &mut events,
        });

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let samples = bus.apu.drain_samples();
            audio_sample_count += samples.len();
            for sample in samples {
                audio_peak_abs = audio_peak_abs.max(sample.abs());
            }
            events.push(event_telemetry(EventTelemetryInput {
                cycle: cycles,
                frame: frames,
                status,
                current_test,
                pc: cpu.pc,
                cpu: cpu_snapshot,
                diagnostic_ram: diagnostic_ram.clone(),
                kind: DiagnosticEventKind::FrameComplete,
                note: "frame_complete",
            }));
        }

        if current_test != last_current_test {
            last_current_test = current_test;
            events.push(event_telemetry(EventTelemetryInput {
                cycle: cycles,
                frame: frames,
                status,
                current_test,
                pc: cpu.pc,
                cpu: cpu_snapshot,
                diagnostic_ram: diagnostic_ram.clone(),
                kind: DiagnosticEventKind::TestChanged,
                note: "test_changed",
            }));
        }
        if status != last_status {
            last_status = status;
            events.push(event_telemetry(EventTelemetryInput {
                cycle: cycles,
                frame: frames,
                status,
                current_test,
                pc: cpu.pc,
                cpu: cpu_snapshot,
                diagnostic_ram: diagnostic_ram.clone(),
                kind: DiagnosticEventKind::StatusChanged,
                note: "status_changed",
            }));
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
            maybe_apply_diagnostic_fault_injection(
                &mut bus,
                config.fault_injection,
                fault_injection_pc,
                &mut fault_injected,
                cpu.pc,
            );
            observe_instruction_trace(
                &mut instruction_trace,
                &trace_context,
                &mut bus,
                &cpu,
                cycles,
                frames,
            );
            let dma_active_before = bus.dma_active();
            let dmc_stall_before = bus.dmc_stall_active();
            cpu.clock(&mut bus);
            bus.tick(1);
            bus.tick_apu();
            let dmc_dma_service = bus.service_dmc_dma(cpu.is_odd_cycle());
            cycles += 1;

            let status = read_ram_byte(&mut bus, STATUS_ADDR);
            let current_test = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
            let cpu_snapshot = cpu_telemetry(&cpu);
            let diagnostic_ram = diagnostic_ram_watch_telemetry(&mut bus, status, current_test);
            let dma_active_after = bus.dma_active();
            dma_observation.observe_tick(DmaTickObservation {
                cycle: cycles,
                frame: frames,
                status,
                current_test,
                pc: cpu.pc,
                cpu: cpu_snapshot,
                diagnostic_ram: diagnostic_ram.clone(),
                active_before: dma_active_before,
                active_after: dma_active_after,
                dmc_stall_before,
                dmc_stall_after: bus.dmc_stall_active(),
                dmc_dma_service,
                events: &mut events,
            });

            if bus.ppu.frame_complete() {
                frames += 1;
                bus.apu.end_frame();
                let samples = bus.apu.drain_samples();
                audio_sample_count += samples.len();
                for sample in samples {
                    audio_peak_abs = audio_peak_abs.max(sample.abs());
                }
                events.push(event_telemetry(EventTelemetryInput {
                    cycle: cycles,
                    frame: frames,
                    status,
                    current_test,
                    pc: cpu.pc,
                    cpu: cpu_snapshot,
                    diagnostic_ram: diagnostic_ram.clone(),
                    kind: DiagnosticEventKind::PostPassFrameComplete,
                    note: "post_pass_frame_complete",
                }));
            }
        }
    }

    let ram = bus.ram_snapshot();
    let status = ram[STATUS_ADDR as usize];
    let current_test = ram[CURRENT_TEST_ADDR as usize];
    let failure_code = ram[FAILURE_CODE_ADDR as usize];
    let test_results = test_telemetry(&ram);
    let dma = dma_observation.telemetry();
    let oam = oam_telemetry(&bus.ppu.oam_data);
    let frame = frame_telemetry(&bus.ppu.frame_data);
    let mut host_failures = host_validate(HostValidationInput {
        status,
        timeout,
        tests: &test_results,
        ram: &ram,
        dma: &dma,
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

    let probes = probe_telemetry(ProbeTelemetryInput {
        status,
        timeout,
        current_test,
        failure_code,
        tests: &test_results,
        ram: &ram,
        dma: &dma,
        oam: &oam,
        frame: &frame,
        audio_sample_count,
        frames,
    });
    let passed = status == STATUS_PASS && !timeout && host_failures.is_empty();
    let failure = failure_telemetry(
        passed,
        status,
        timeout,
        current_test,
        failure_code,
        &host_failures,
        &probes,
    );
    let verdict = VerdictTelemetry {
        passed,
        status,
        timeout,
        current_test,
        current_test_name: test_name(current_test),
        failure_code,
        failure,
        host_failures,
    };
    let timeline = test_timeline(&test_results, &events, &verdict, cycles, frames, cpu.pc);
    let instruction_trace = instruction_trace.telemetry();
    let analysis = analysis_telemetry(AnalysisTelemetryInput {
        verdict: &verdict,
        tests: &test_results,
        timeline: &timeline,
        probes: &probes,
        instruction_trace: &instruction_trace,
        events: &events,
        cycles,
        frames,
    });

    Ok(DiagnosticTelemetry {
        schema_version: DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION,
        provenance: DIAGNOSTIC_PROVENANCE,
        suite: suite_telemetry(),
        cartridge: cartridge_info,
        input: diagnostic_input_telemetry(&config),
        verdict,
        analysis,
        cycles,
        frames,
        cpu: cpu_telemetry(&cpu),
        ram: RamTelemetry {
            signature: ram[SIGNATURE_ADDR as usize],
            nmi_count: ram[NMI_COUNT_ADDR as usize],
            checksum: hash_bytes(&ram),
            result_base: RESULT_BASE,
        },
        tests: test_results,
        timeline,
        probes,
        dma,
        oam,
        frame,
        audio: AudioTelemetry {
            sample_count: audio_sample_count,
            peak_abs: audio_peak_abs,
        },
        instruction_trace,
        events,
    })
}

pub fn compare_diagnostic_to_baseline(
    telemetry: &DiagnosticTelemetry,
    baseline_json: &str,
) -> Result<DiagnosticComparisonTelemetry, String> {
    let baseline: Value = serde_json::from_str(baseline_json)
        .map_err(|err| format!("failed to parse baseline diagnostic JSON: {err}"))?;
    let current = serde_json::to_value(telemetry)
        .map_err(|err| format!("failed to serialize current diagnostic telemetry: {err}"))?;
    let mut differences = Vec::new();

    compare_schema(&baseline, &current, &mut differences);
    compare_input(&baseline, &current, &mut differences);
    compare_dma(&baseline, &current, &mut differences);
    compare_verdict(&baseline, &current, &mut differences);
    compare_coverage(&baseline, &current, &mut differences);
    compare_probes(&baseline, &current, &mut differences);
    compare_observation_checksums(&baseline, &current, &mut differences);
    compare_timeline(&baseline, &current, &mut differences);
    compare_instruction_trace(&baseline, &current, &mut differences);

    let failure_count = differences
        .iter()
        .filter(|difference| difference.severity == DiagnosticComparisonSeverity::Failure)
        .count();
    let warning_count = differences
        .iter()
        .filter(|difference| difference.severity == DiagnosticComparisonSeverity::Warning)
        .count();
    let info_count = differences
        .iter()
        .filter(|difference| difference.severity == DiagnosticComparisonSeverity::Info)
        .count();
    let passed = failure_count == 0;
    let summary = comparison_summary(passed, failure_count, warning_count, info_count);

    Ok(DiagnosticComparisonTelemetry {
        baseline_schema_version: json_u64(&baseline, &["schema_version"]),
        current_schema_version: telemetry.schema_version,
        passed,
        summary,
        difference_count: differences.len(),
        failure_count,
        warning_count,
        info_count,
        differences,
    })
}

pub fn format_diagnostic_comparison_report(comparison: &DiagnosticComparisonTelemetry) -> String {
    let mut report = String::new();

    writeln!(report, "# OxideNES Diagnostic Baseline Comparison").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "## Verdict").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "{}", comparison.summary).expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| Result | {} |",
        if comparison.passed { "pass" } else { "fail" }
    )
    .expect("write report");
    writeln!(
        report,
        "| Schema versions | baseline {}, current {} |",
        comparison
            .baseline_schema_version
            .map(|version| version.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        comparison.current_schema_version
    )
    .expect("write report");
    writeln!(report, "| Differences | {} |", comparison.difference_count).expect("write report");
    writeln!(report, "| Failures | {} |", comparison.failure_count).expect("write report");
    writeln!(report, "| Warnings | {} |", comparison.warning_count).expect("write report");
    writeln!(report, "| Info | {} |", comparison.info_count).expect("write report");
    writeln!(report).expect("write report");

    writeln!(report, "## Differences").expect("write report");
    writeln!(report).expect("write report");
    if comparison.differences.is_empty() {
        writeln!(report, "No baseline differences detected.").expect("write report");
        writeln!(report).expect("write report");
        return report;
    }

    writeln!(
        report,
        "| Severity | Category | Path | Baseline | Current | Note |"
    )
    .expect("write report");
    writeln!(report, "| --- | --- | --- | --- | --- | --- |").expect("write report");
    for difference in &comparison.differences {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} |",
            diagnostic_comparison_severity_label(difference.severity),
            difference.category,
            difference.path,
            difference.baseline.as_deref().unwrap_or("missing"),
            difference.current.as_deref().unwrap_or("missing"),
            difference.note
        )
        .expect("write report");
    }
    writeln!(report).expect("write report");

    report
}

pub fn format_diagnostic_report(telemetry: &DiagnosticTelemetry) -> String {
    let mut report = String::new();

    writeln!(report, "# OxideNES Diagnostic Report").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "## Verdict").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| Result | {} |",
        if telemetry.verdict.passed {
            "pass"
        } else {
            "fail"
        }
    )
    .expect("write report");
    writeln!(
        report,
        "| Health | {} |",
        diagnostic_health_label(telemetry.analysis.health)
    )
    .expect("write report");
    writeln!(report, "| Schema version | {} |", telemetry.schema_version).expect("write report");
    writeln!(report, "| Suite | {} |", telemetry.suite.version).expect("write report");
    writeln!(
        report,
        "| Cycles / frames | {} / {} |",
        telemetry.cycles, telemetry.frames
    )
    .expect("write report");
    writeln!(
        report,
        "| Current test | {} |",
        telemetry
            .verdict
            .current_test_name
            .unwrap_or("unknown_test")
    )
    .expect("write report");
    writeln!(
        report,
        "| Status | {} |",
        hex_byte(telemetry.verdict.status)
    )
    .expect("write report");
    writeln!(report).expect("write report");

    writeln!(report, "## Analysis").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "{}", telemetry.analysis.summary).expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| Failing subsystem | {} |",
        telemetry
            .analysis
            .failing_subsystem
            .map(diagnostic_subsystem_label)
            .unwrap_or("none")
    )
    .expect("write report");
    writeln!(
        report,
        "| Failing test | {} |",
        telemetry.analysis.failing_test.unwrap_or("none")
    )
    .expect("write report");
    writeln!(
        report,
        "| First failure domain | {} |",
        telemetry
            .analysis
            .first_failure_domain
            .as_deref()
            .unwrap_or("none")
    )
    .expect("write report");
    writeln!(
        report,
        "| Test transitions | {} |",
        telemetry.analysis.test_transition_count
    )
    .expect("write report");
    writeln!(report).expect("write report");

    write_input_section(&mut report, telemetry);
    write_debug_focus_section(&mut report, telemetry);
    write_failure_section(&mut report, telemetry);
    write_coverage_section(&mut report, telemetry);
    write_coverage_gaps_section(&mut report, telemetry);
    write_dma_section(&mut report, telemetry);
    write_timing_section(&mut report, telemetry);
    write_instruction_trace_section(&mut report, telemetry);
    write_probe_section(&mut report, telemetry);
    write_next_actions_section(&mut report, telemetry);
    write_host_failures_section(&mut report, telemetry);
    write_event_tail_section(&mut report, telemetry);

    report
}

fn write_debug_focus_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    let focus = &telemetry.analysis.debug_focus;

    writeln!(report, "## Debug Focus").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "This derived focus is the recommended first stop for automated triage before drilling into the full event, probe, and instruction streams."
    )
    .expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| Health | {} |",
        diagnostic_health_label(focus.health)
    )
    .expect("write report");
    writeln!(
        report,
        "| Focus test | {} ({}) |",
        focus.focus_test_name.unwrap_or("unknown_test"),
        focus.focus_test_id
    )
    .expect("write report");
    writeln!(
        report,
        "| Focus subsystem | {} |",
        focus
            .focus_subsystem
            .map(diagnostic_subsystem_label)
            .unwrap_or("none")
    )
    .expect("write report");
    writeln!(
        report,
        "| Focus domain | {} |",
        focus.focus_domain.as_deref().unwrap_or("none")
    )
    .expect("write report");
    writeln!(
        report,
        "| Failure kind | {} |",
        focus
            .failure_kind
            .map(diagnostic_failure_kind_label)
            .unwrap_or("none")
    )
    .expect("write report");
    writeln!(report, "| Failure code | {} |", focus.failure_code_hex).expect("write report");
    writeln!(
        report,
        "| Failed probe ids | {} |",
        if focus.failed_probe_ids.is_empty() {
            "none".to_string()
        } else {
            focus.failed_probe_ids.join(", ")
        }
    )
    .expect("write report");
    writeln!(
        report,
        "| Skipped probe count | {} |",
        focus.skipped_probe_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Last event | {} |",
        format_debug_event_focus(focus.last_event.as_ref())
    )
    .expect("write report");
    writeln!(
        report,
        "| Terminal instruction | {} |",
        format_debug_instruction_focus(focus.terminal_instruction.as_ref())
    )
    .expect("write report");
    writeln!(
        report,
        "| Last focus-test instruction | {} |",
        format_debug_instruction_focus(focus.last_test_instruction.as_ref())
    )
    .expect("write report");
    writeln!(report).expect("write report");
}

fn write_failure_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    if let Some(failure) = &telemetry.verdict.failure {
        writeln!(report, "## Failure Localization").expect("write report");
        writeln!(report).expect("write report");
        writeln!(report, "| Field | Value |").expect("write report");
        writeln!(report, "| --- | --- |").expect("write report");
        writeln!(
            report,
            "| Kind | {} |",
            diagnostic_failure_kind_label(failure.kind)
        )
        .expect("write report");
        writeln!(
            report,
            "| Test | {} |",
            failure.test_name.unwrap_or("unknown_test")
        )
        .expect("write report");
        writeln!(report, "| Failure code | {} |", failure.failure_code_hex).expect("write report");
        writeln!(report, "| Assertion | {} |", failure.assertion).expect("write report");
        writeln!(report, "| Expected | {} |", failure.expected).expect("write report");
        writeln!(report, "| Observed | {} |", failure.observed).expect("write report");
        writeln!(report, "| Likely domain | {} |", failure.likely_domain).expect("write report");
        writeln!(
            report,
            "| Remediation hint | {} |",
            failure.remediation_hint
        )
        .expect("write report");
        writeln!(report).expect("write report");
    }
}

fn write_input_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## Input Configuration").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| Joypad 1 mask / expected | {} / {} |",
        telemetry.input.joypad1_mask_hex, telemetry.input.joypad1_expected_mask_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Joypad 2 mask / expected | {} / {} |",
        telemetry.input.joypad2_mask_hex, telemetry.input.joypad2_expected_mask_hex
    )
    .expect("write report");
    writeln!(report).expect("write report");
}

fn write_coverage_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## Coverage").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "{} / {} tests passed; {} failed.",
        telemetry.analysis.coverage.passed_tests,
        telemetry.analysis.coverage.total_tests,
        telemetry.analysis.coverage.failed_tests
    )
    .expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Subsystem | Passed | Total |").expect("write report");
    writeln!(report, "| --- | ---: | ---: |").expect("write report");
    for entry in &telemetry.analysis.coverage.subsystem_summary {
        writeln!(
            report,
            "| {} | {} | {} |",
            diagnostic_subsystem_label(entry.subsystem),
            entry.passed,
            entry.total
        )
        .expect("write report");
    }
    writeln!(report).expect("write report");
    writeln!(report, "| Tier | Passed | Total |").expect("write report");
    writeln!(report, "| --- | ---: | ---: |").expect("write report");
    for entry in &telemetry.analysis.coverage.tier_summary {
        writeln!(
            report,
            "| {} | {} | {} |",
            diagnostic_test_tier_label(entry.tier),
            entry.passed,
            entry.total
        )
        .expect("write report");
    }
    writeln!(report).expect("write report");
}

fn write_coverage_gaps_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## Known Coverage Gaps").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "These are explicit limits of the generated cartridge. Passing diagnostics should not be read as coverage for these areas."
    )
    .expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "| Gap | Subsystem | Risk | Current coverage | Missing coverage | Suggested next test |"
    )
    .expect("write report");
    writeln!(report, "| --- | --- | --- | --- | --- | --- |").expect("write report");
    for gap in &telemetry.analysis.coverage_gaps {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} |",
            gap.id,
            gap.subsystem,
            gap.risk,
            gap.current_coverage,
            gap.missing_coverage,
            gap.suggested_next_test
        )
        .expect("write report");
    }
    writeln!(report).expect("write report");
}

fn write_dma_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## DMA Timing").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| OAM DMA observed | {} |",
        telemetry.dma.oam_dma_observed
    )
    .expect("write report");
    writeln!(
        report,
        "| OAM DMA completed | {} |",
        telemetry.dma.oam_dma_completed
    )
    .expect("write report");
    writeln!(
        report,
        "| Active cycles / expected | {} / {}..={} |",
        telemetry.dma.oam_dma_active_cycles,
        telemetry.dma.oam_dma_expected_min_cycles,
        telemetry.dma.oam_dma_expected_max_cycles
    )
    .expect("write report");
    writeln!(
        report,
        "| Start cycle / end cycle | {} / {} |",
        optional_u64(telemetry.dma.oam_dma_start_cycle),
        optional_u64(telemetry.dma.oam_dma_end_cycle)
    )
    .expect("write report");
    writeln!(
        report,
        "| Start test / end test | {} / {} |",
        telemetry
            .dma
            .oam_dma_start_test_name
            .unwrap_or("unknown_test"),
        telemetry
            .dma
            .oam_dma_end_test_name
            .unwrap_or("unknown_test")
    )
    .expect("write report");
    writeln!(
        report,
        "| First active cycle / parity | {} / {} |",
        optional_u64(telemetry.dma.oam_dma_first_active_cycle),
        telemetry
            .dma
            .oam_dma_first_active_cycle_parity
            .unwrap_or("none")
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC fetches / overlapping fetches | {} / {} |",
        telemetry.dma.dmc_dma_fetches_observed, telemetry.dma.dmc_dma_fetches_during_oam_dma
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC first fetch / first overlap | {} / {} |",
        optional_u64(telemetry.dma.dmc_dma_first_fetch_cycle),
        optional_u64(telemetry.dma.dmc_dma_first_oam_overlap_cycle)
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC first fetch address | {} |",
        optional_pc(telemetry.dma.dmc_dma_first_fetch_address)
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC first fetch parity / stall bucket | {} / {} |",
        telemetry
            .dma
            .dmc_dma_first_fetch_cpu_cycle_parity
            .unwrap_or("none"),
        optional_u8(telemetry.dma.dmc_dma_first_fetch_stall_cycles)
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC overlap test | {} |",
        telemetry
            .dma
            .dmc_dma_first_oam_overlap_test_name
            .unwrap_or("unknown_test")
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC overlap parity / stall bucket | {} / {} |",
        telemetry
            .dma
            .dmc_dma_first_oam_overlap_cpu_cycle_parity
            .unwrap_or("none"),
        optional_u8(telemetry.dma.dmc_dma_first_oam_overlap_stall_cycles)
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC 3-cycle / 4-cycle fetches | {} / {} |",
        telemetry.dma.dmc_dma_three_cycle_fetches, telemetry.dma.dmc_dma_four_cycle_fetches
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC queued / post-OAM stall cycles | {} / {} |",
        telemetry.dma.dmc_dma_queued_during_oam_dma_cycles,
        telemetry.dma.dmc_dma_stall_cycles_after_oam_dma
    )
    .expect("write report");
    writeln!(report).expect("write report");
}

fn write_timing_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## Timing").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | ---: |").expect("write report");
    writeln!(
        report,
        "| Started tests | {} |",
        telemetry.analysis.timing.started_tests
    )
    .expect("write report");
    writeln!(
        report,
        "| Ended tests | {} |",
        telemetry.analysis.timing.ended_tests
    )
    .expect("write report");
    writeln!(
        report,
        "| Not started tests | {} |",
        telemetry.analysis.timing.not_started_tests
    )
    .expect("write report");
    writeln!(
        report,
        "| Timed out tests | {} |",
        telemetry.analysis.timing.timed_out_tests
    )
    .expect("write report");
    if let Some(slowest) = &telemetry.analysis.timing.slowest_test {
        writeln!(
            report,
            "| Slowest test | {} ({} cycles, {} frames) |",
            slowest.test_name, slowest.duration_cycles, slowest.duration_frames
        )
        .expect("write report");
    } else {
        writeln!(report, "| Slowest test | none |").expect("write report");
    }
    writeln!(report).expect("write report");

    writeln!(report, "| ID | Test | Subsystem | Tier | Outcome | Start | End | Duration | End reason | Terminal status | Terminal PC |").expect("write report");
    writeln!(
        report,
        "| ---: | --- | --- | --- | --- | ---: | ---: | ---: | --- | --- | --- |"
    )
    .expect("write report");
    for test in &telemetry.timeline {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            test.test_id,
            test.test_name,
            diagnostic_subsystem_label(test.subsystem),
            diagnostic_test_tier_label(test.tier),
            test_timeline_outcome_label(test.outcome),
            optional_u64(test.start_cycle),
            optional_u64(test.end_cycle),
            optional_u64(test.duration_cycles),
            test.end_reason
                .map(test_timeline_end_reason_label)
                .unwrap_or("none"),
            test.terminal_status_hex.as_deref().unwrap_or("none"),
            optional_pc(test.terminal_pc)
        )
        .expect("write report");
    }
    writeln!(report).expect("write report");
}

fn write_probe_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## Observation Probes").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | ---: |").expect("write report");
    writeln!(
        report,
        "| Total probes | {} |",
        telemetry.analysis.probe_summary.total_probes
    )
    .expect("write report");
    writeln!(
        report,
        "| Passed probes | {} |",
        telemetry.analysis.probe_summary.passed_probes
    )
    .expect("write report");
    writeln!(
        report,
        "| Failed probes | {} |",
        telemetry.analysis.probe_summary.failed_probes
    )
    .expect("write report");
    writeln!(
        report,
        "| Skipped probes | {} |",
        telemetry.analysis.probe_summary.skipped_probes
    )
    .expect("write report");
    writeln!(
        report,
        "| First failed probe | {} |",
        telemetry
            .analysis
            .probe_summary
            .first_failed_probe
            .as_deref()
            .unwrap_or("none")
    )
    .expect("write report");
    writeln!(report).expect("write report");

    writeln!(
        report,
        "| Status | Probe | Source | Subsystem | Test | Expected | Observed | Likely domain |"
    )
    .expect("write report");
    writeln!(report, "| --- | --- | --- | --- | --- | --- | --- | --- |").expect("write report");
    for probe in &telemetry.probes {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            diagnostic_probe_status_label(probe.status),
            probe.id,
            diagnostic_probe_source_label(probe.source),
            probe
                .subsystem
                .map(diagnostic_subsystem_label)
                .unwrap_or("none"),
            probe.test_name.unwrap_or("none"),
            probe.expected,
            probe.observed,
            probe.likely_domain
        )
        .expect("write report");
    }
    writeln!(report).expect("write report");
}

fn write_instruction_trace_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## Instruction Trace Tail").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | ---: |").expect("write report");
    writeln!(
        report,
        "| Captured instructions | {} |",
        telemetry.instruction_trace.captured_instruction_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Retained instructions | {} |",
        telemetry.instruction_trace.retained_instruction_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Retention limit | {} |",
        telemetry.instruction_trace.retention_limit
    )
    .expect("write report");
    writeln!(
        report,
        "| Truncated | {} |",
        telemetry.instruction_trace.truncated
    )
    .expect("write report");
    writeln!(report).expect("write report");

    writeln!(
        report,
        "| Seq | Cycle | Frame | Test | PC | Instruction | Symbol | CPU A/X/Y | SP/P | Result |"
    )
    .expect("write report");
    writeln!(
        report,
        "| ---: | ---: | ---: | --- | --- | --- | --- | --- | --- | --- |"
    )
    .expect("write report");
    let start = telemetry.instruction_trace.tail.len().saturating_sub(16);
    for entry in &telemetry.instruction_trace.tail[start..] {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} | {} | {}/{}/{} | {}/{} | {} |",
            entry.sequence,
            entry.cycle,
            entry.frame,
            entry
                .diagnostic_ram
                .current_test_name
                .unwrap_or("unknown_test"),
            entry.pc_hex,
            entry
                .instruction
                .as_ref()
                .map(|instruction| instruction.text.as_str())
                .or(entry.opcode_hex.as_deref())
                .unwrap_or("none"),
            entry
                .symbol
                .as_ref()
                .map(format_symbol)
                .unwrap_or_else(|| "none".to_string()),
            hex_byte(entry.cpu.a),
            hex_byte(entry.cpu.x),
            hex_byte(entry.cpu.y),
            hex_byte(entry.cpu.sp),
            hex_byte(entry.cpu.status),
            entry
                .diagnostic_ram
                .current_result_hex
                .as_deref()
                .unwrap_or("none")
        )
        .expect("write report");
    }
    writeln!(report).expect("write report");
}

fn write_next_actions_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## Next Actions").expect("write report");
    writeln!(report).expect("write report");
    for action in &telemetry.analysis.next_actions {
        writeln!(report, "- {action}").expect("write report");
    }
    writeln!(report).expect("write report");
}

fn write_host_failures_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    if telemetry.verdict.host_failures.is_empty() {
        return;
    }

    writeln!(report, "## Host Failures").expect("write report");
    writeln!(report).expect("write report");
    for failure in &telemetry.verdict.host_failures {
        writeln!(report, "- {failure}").expect("write report");
    }
    writeln!(report).expect("write report");
}

fn write_event_tail_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## Event Tail").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "| Kind | Cycle | Frame | Status | Test | PC | CPU A/X/Y | SP/P | Result | Failure |"
    )
    .expect("write report");
    writeln!(
        report,
        "| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- |"
    )
    .expect("write report");
    let start = telemetry.events.len().saturating_sub(8);
    for event in &telemetry.events[start..] {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} | {}/{}/{} | {}/{} | {} | {} |",
            diagnostic_event_kind_label(event.kind),
            event.cycle,
            event.frame,
            hex_byte(event.status),
            event.current_test_name.unwrap_or("unknown_test"),
            format_pc(event.pc),
            hex_byte(event.cpu.a),
            hex_byte(event.cpu.x),
            hex_byte(event.cpu.y),
            hex_byte(event.cpu.sp),
            hex_byte(event.cpu.status),
            event
                .diagnostic_ram
                .current_result_hex
                .as_deref()
                .unwrap_or("none"),
            event.diagnostic_ram.failure_code_hex
        )
        .expect("write report");
    }
    writeln!(report).expect("write report");
}

#[derive(Debug, Default)]
struct InstructionTraceObservation {
    captured_instruction_count: u64,
    tail: Vec<InstructionTraceEntryTelemetry>,
}

impl InstructionTraceObservation {
    fn observe(&mut self, mut entry: InstructionTraceEntryTelemetry) {
        self.captured_instruction_count += 1;
        entry.sequence = self.captured_instruction_count;

        if self.tail.len() == INSTRUCTION_TRACE_TAIL_LIMIT {
            self.tail.remove(0);
        }
        self.tail.push(entry);
    }

    fn telemetry(self) -> InstructionTraceTelemetry {
        let retained_instruction_count = self.tail.len();
        InstructionTraceTelemetry {
            captured_instruction_count: self.captured_instruction_count,
            retained_instruction_count,
            retention_limit: INSTRUCTION_TRACE_TAIL_LIMIT,
            truncated: self.captured_instruction_count > retained_instruction_count as u64,
            tail: self.tail,
        }
    }
}

#[derive(Debug, Clone)]
struct DiagnosticTraceContext {
    symbols: Vec<DiagnosticTraceSymbol>,
}

#[derive(Debug, Clone)]
struct DiagnosticTraceSymbol {
    name: String,
    address: u16,
}

impl DiagnosticTraceContext {
    fn from_labels(labels: &HashMap<String, u16>) -> Self {
        let mut symbols = labels
            .iter()
            .map(|(name, address)| DiagnosticTraceSymbol {
                name: name.clone(),
                address: *address,
            })
            .collect::<Vec<_>>();
        symbols.sort_by(|left, right| {
            left.address
                .cmp(&right.address)
                .then_with(|| left.name.cmp(&right.name))
        });
        Self { symbols }
    }

    fn symbol_for_pc(&self, pc: u16) -> Option<DiagnosticSymbolTelemetry> {
        let symbol = self
            .symbols
            .iter()
            .rev()
            .find(|symbol| symbol.address <= pc)?;
        let offset = pc.wrapping_sub(symbol.address);
        Some(DiagnosticSymbolTelemetry {
            name: symbol.name.clone(),
            address: symbol.address,
            address_hex: format_pc(symbol.address),
            offset,
            offset_hex: format!("0x{offset:04X}"),
        })
    }
}

fn observe_instruction_trace(
    trace: &mut InstructionTraceObservation,
    trace_context: &DiagnosticTraceContext,
    bus: &mut Bus,
    cpu: &Cpu,
    cycle: u64,
    frame: u64,
) {
    if cpu.cycles != 0 || bus.dma_active() || bus.dmc_stall_active() {
        return;
    }

    let status = read_ram_byte(bus, STATUS_ADDR);
    let current_test = read_ram_byte(bus, CURRENT_TEST_ADDR);
    let diagnostic_ram = diagnostic_ram_watch_telemetry(bus, status, current_test);
    let opcode = instruction_opcode(bus, cpu.pc);
    let instruction = instruction_decode_telemetry(bus, cpu.pc, opcode);
    trace.observe(InstructionTraceEntryTelemetry {
        sequence: 0,
        cycle,
        frame,
        pc: cpu.pc,
        pc_hex: format_pc(cpu.pc),
        opcode,
        opcode_hex: opcode.map(hex_byte),
        instruction,
        symbol: trace_context.symbol_for_pc(cpu.pc),
        cpu: cpu_telemetry(cpu),
        diagnostic_ram,
    });
}

fn instruction_opcode(bus: &Bus, pc: u16) -> Option<u8> {
    (pc >= 0x4020).then(|| bus.cartridge.mapper.read_prg(pc))
}

fn instruction_decode_telemetry(
    bus: &Bus,
    pc: u16,
    opcode: Option<u8>,
) -> Option<InstructionDecodeTelemetry> {
    let decode = decode_opcode(opcode?)?;
    let bytes = instruction_prg_bytes(bus, pc, decode.byte_len)?;
    let operand_bytes = bytes[1..].to_vec();
    let operand_hex = operand_bytes.iter().copied().map(hex_byte).collect();
    Some(InstructionDecodeTelemetry {
        mnemonic: decode.mnemonic,
        addressing_mode: decode.addressing_mode,
        byte_len: decode.byte_len,
        text: format_instruction_text(decode, pc, &operand_bytes),
        operand_bytes,
        operand_hex,
    })
}

fn instruction_prg_bytes(bus: &Bus, pc: u16, byte_len: u8) -> Option<Vec<u8>> {
    if pc < 0x4020 {
        return None;
    }

    Some(
        (0..byte_len)
            .map(|offset| {
                bus.cartridge
                    .mapper
                    .read_prg(pc.wrapping_add(offset as u16))
            })
            .collect(),
    )
}

#[derive(Debug, Clone, Copy)]
struct OpcodeDecode {
    mnemonic: &'static str,
    addressing_mode: &'static str,
    byte_len: u8,
}

fn decode_opcode(opcode: u8) -> Option<OpcodeDecode> {
    let decode = match opcode {
        0x18 => OpcodeDecode::implied("CLC"),
        0x20 => OpcodeDecode::absolute("JSR"),
        0x29 => OpcodeDecode::immediate("AND"),
        0x38 => OpcodeDecode::implied("SEC"),
        0x40 => OpcodeDecode::implied("RTI"),
        0x48 => OpcodeDecode::implied("PHA"),
        0x4C => OpcodeDecode::absolute("JMP"),
        0x6C => OpcodeDecode::indirect("JMP"),
        0x60 => OpcodeDecode::implied("RTS"),
        0x68 => OpcodeDecode::implied("PLA"),
        0x69 => OpcodeDecode::immediate("ADC"),
        0x78 => OpcodeDecode::implied("SEI"),
        0x85 => OpcodeDecode::zero_page("STA"),
        0x95 => OpcodeDecode::zero_page_x("STA"),
        0x8A => OpcodeDecode::implied("TXA"),
        0x8D => OpcodeDecode::absolute("STA"),
        0x9A => OpcodeDecode::implied("TXS"),
        0x9D => OpcodeDecode::absolute_x("STA"),
        0xA2 => OpcodeDecode::immediate("LDX"),
        0xA5 => OpcodeDecode::zero_page("LDA"),
        0xA9 => OpcodeDecode::immediate("LDA"),
        0xAD => OpcodeDecode::absolute("LDA"),
        0xB5 => OpcodeDecode::zero_page_x("LDA"),
        0xC9 => OpcodeDecode::immediate("CMP"),
        0xD0 => OpcodeDecode::relative("BNE"),
        0xD8 => OpcodeDecode::implied("CLD"),
        0xE6 => OpcodeDecode::zero_page("INC"),
        0xE8 => OpcodeDecode::implied("INX"),
        0xE9 => OpcodeDecode::immediate("SBC"),
        0xEA => OpcodeDecode::implied("NOP"),
        0xF0 => OpcodeDecode::relative("BEQ"),
        _ => return None,
    };
    Some(decode)
}

impl OpcodeDecode {
    fn implied(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            addressing_mode: "implied",
            byte_len: 1,
        }
    }

    fn immediate(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            addressing_mode: "immediate",
            byte_len: 2,
        }
    }

    fn zero_page(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            addressing_mode: "zero_page",
            byte_len: 2,
        }
    }

    fn zero_page_x(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            addressing_mode: "zero_page_x",
            byte_len: 2,
        }
    }

    fn absolute(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            addressing_mode: "absolute",
            byte_len: 3,
        }
    }

    fn absolute_x(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            addressing_mode: "absolute_x",
            byte_len: 3,
        }
    }

    fn indirect(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            addressing_mode: "indirect",
            byte_len: 3,
        }
    }

    fn relative(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            addressing_mode: "relative",
            byte_len: 2,
        }
    }
}

fn format_instruction_text(decode: OpcodeDecode, pc: u16, operand_bytes: &[u8]) -> String {
    match decode.addressing_mode {
        "immediate" => format!("{} #${:02X}", decode.mnemonic, operand_bytes[0]),
        "zero_page" => format!("{} ${:02X}", decode.mnemonic, operand_bytes[0]),
        "zero_page_x" => format!("{} ${:02X},X", decode.mnemonic, operand_bytes[0]),
        "absolute" => format!(
            "{} {}",
            decode.mnemonic,
            format_pc(u16::from_le_bytes([operand_bytes[0], operand_bytes[1]]))
        ),
        "absolute_x" => format!(
            "{} {},X",
            decode.mnemonic,
            format_pc(u16::from_le_bytes([operand_bytes[0], operand_bytes[1]]))
        ),
        "indirect" => format!(
            "{} ({})",
            decode.mnemonic,
            format_pc(u16::from_le_bytes([operand_bytes[0], operand_bytes[1]]))
        ),
        "relative" => {
            let target = (pc as i32 + 2 + operand_bytes[0] as i8 as i32) as u16;
            format!("{} {}", decode.mnemonic, format_pc(target))
        }
        _ => decode.mnemonic.to_string(),
    }
}

#[derive(Debug, Default)]
struct DmaObservation {
    oam_dma_start_cycle: Option<u64>,
    oam_dma_end_cycle: Option<u64>,
    oam_dma_first_active_cycle: Option<u64>,
    oam_dma_first_active_cycle_even: Option<bool>,
    oam_dma_active_cycles: u64,
    oam_dma_start_test: Option<u8>,
    oam_dma_end_test: Option<u8>,
    dmc_dma_fetches_observed: u64,
    dmc_dma_fetches_during_oam_dma: u64,
    dmc_dma_first_fetch_cycle: Option<u64>,
    dmc_dma_first_fetch_address: Option<u16>,
    dmc_dma_first_fetch_cpu_cycle_odd: Option<bool>,
    dmc_dma_first_fetch_stall_cycles: Option<u8>,
    dmc_dma_first_oam_overlap_cycle: Option<u64>,
    dmc_dma_first_oam_overlap_test: Option<u8>,
    dmc_dma_first_oam_overlap_cpu_cycle_odd: Option<bool>,
    dmc_dma_first_oam_overlap_stall_cycles: Option<u8>,
    dmc_dma_three_cycle_fetches: u64,
    dmc_dma_four_cycle_fetches: u64,
    dmc_dma_stall_cycles: u64,
    dmc_dma_stall_cycles_after_oam_dma: u64,
    dmc_dma_queued_during_oam_dma_cycles: u64,
}

struct DmaTickObservation<'a> {
    cycle: u64,
    frame: u64,
    status: u8,
    current_test: u8,
    pc: u16,
    cpu: CpuTelemetry,
    diagnostic_ram: DiagnosticRamWatchTelemetry,
    active_before: bool,
    active_after: bool,
    dmc_stall_before: bool,
    dmc_stall_after: bool,
    dmc_dma_service: Option<DmcDmaService>,
    events: &'a mut Vec<EventTelemetry>,
}

impl DmaObservation {
    fn observe_tick(&mut self, tick: DmaTickObservation<'_>) {
        if tick.active_before {
            if self.oam_dma_first_active_cycle.is_none() {
                self.oam_dma_first_active_cycle = Some(tick.cycle);
                self.oam_dma_first_active_cycle_even = Some(tick.cycle.is_multiple_of(2));
            }
            self.oam_dma_active_cycles += 1;
        }
        if tick.dmc_stall_before {
            if tick.active_before {
                self.dmc_dma_queued_during_oam_dma_cycles += 1;
            } else {
                self.dmc_dma_stall_cycles += 1;
                if self.oam_dma_end_cycle.is_some() {
                    self.dmc_dma_stall_cycles_after_oam_dma += 1;
                }
            }
        }

        if !tick.active_before && tick.active_after && self.oam_dma_start_cycle.is_none() {
            self.oam_dma_start_cycle = Some(tick.cycle);
            self.oam_dma_start_test = known_test_id(tick.current_test);
            tick.events.push(event_telemetry(EventTelemetryInput {
                cycle: tick.cycle,
                frame: tick.frame,
                status: tick.status,
                current_test: tick.current_test,
                pc: tick.pc,
                cpu: tick.cpu,
                diagnostic_ram: tick.diagnostic_ram.clone(),
                kind: DiagnosticEventKind::OamDmaStarted,
                note: "oam_dma_started",
            }));
        }

        if tick.active_before && !tick.active_after && self.oam_dma_end_cycle.is_none() {
            self.oam_dma_end_cycle = Some(tick.cycle);
            self.oam_dma_end_test = known_test_id(tick.current_test);
            tick.events.push(event_telemetry(EventTelemetryInput {
                cycle: tick.cycle,
                frame: tick.frame,
                status: tick.status,
                current_test: tick.current_test,
                pc: tick.pc,
                cpu: tick.cpu,
                diagnostic_ram: tick.diagnostic_ram.clone(),
                kind: DiagnosticEventKind::OamDmaCompleted,
                note: "oam_dma_completed",
            }));
        }

        if let Some(service) = tick.dmc_dma_service {
            self.dmc_dma_fetches_observed += 1;
            if service.stall_cycles == 3 {
                self.dmc_dma_three_cycle_fetches += 1;
            }
            if service.stall_cycles == 4 {
                self.dmc_dma_four_cycle_fetches += 1;
            }
            self.dmc_dma_first_fetch_cycle.get_or_insert(tick.cycle);
            self.dmc_dma_first_fetch_address
                .get_or_insert(service.address);
            self.dmc_dma_first_fetch_cpu_cycle_odd
                .get_or_insert(service.odd_cpu_cycle);
            self.dmc_dma_first_fetch_stall_cycles
                .get_or_insert(service.stall_cycles);
            tick.events.push(event_telemetry(EventTelemetryInput {
                cycle: tick.cycle,
                frame: tick.frame,
                status: tick.status,
                current_test: tick.current_test,
                pc: tick.pc,
                cpu: tick.cpu,
                diagnostic_ram: tick.diagnostic_ram.clone(),
                kind: DiagnosticEventKind::DmcDmaFetched,
                note: "dmc_dma_fetched",
            }));

            if tick.active_before || tick.active_after {
                self.dmc_dma_fetches_during_oam_dma += 1;
                self.dmc_dma_first_oam_overlap_cycle
                    .get_or_insert(tick.cycle);
                if self.dmc_dma_first_oam_overlap_test.is_none() {
                    self.dmc_dma_first_oam_overlap_test = known_test_id(tick.current_test);
                }
                self.dmc_dma_first_oam_overlap_cpu_cycle_odd
                    .get_or_insert(service.odd_cpu_cycle);
                self.dmc_dma_first_oam_overlap_stall_cycles
                    .get_or_insert(service.stall_cycles);
                tick.events.push(event_telemetry(EventTelemetryInput {
                    cycle: tick.cycle,
                    frame: tick.frame,
                    status: tick.status,
                    current_test: tick.current_test,
                    pc: tick.pc,
                    cpu: tick.cpu,
                    diagnostic_ram: tick.diagnostic_ram.clone(),
                    kind: DiagnosticEventKind::DmcDmaOamOverlap,
                    note: "dmc_dma_oam_overlap",
                }));
            }
        }

        if tick.dmc_stall_after && tick.active_after && !tick.dmc_stall_before {
            self.dmc_dma_queued_during_oam_dma_cycles += 1;
        }
    }

    fn telemetry(&self) -> DmaTelemetry {
        DmaTelemetry {
            oam_dma_observed: self.oam_dma_start_cycle.is_some(),
            oam_dma_completed: self.oam_dma_start_cycle.is_some()
                && self.oam_dma_end_cycle.is_some(),
            oam_dma_active_cycles: self.oam_dma_active_cycles,
            oam_dma_expected_min_cycles: OAM_DMA_EXPECTED_MIN_CYCLES,
            oam_dma_expected_max_cycles: OAM_DMA_EXPECTED_MAX_CYCLES,
            oam_dma_start_cycle: self.oam_dma_start_cycle,
            oam_dma_end_cycle: self.oam_dma_end_cycle,
            oam_dma_first_active_cycle: self.oam_dma_first_active_cycle,
            oam_dma_first_active_cycle_parity: self
                .oam_dma_first_active_cycle_even
                .map(cycle_parity_label),
            oam_dma_start_test: self.oam_dma_start_test,
            oam_dma_start_test_name: self.oam_dma_start_test.and_then(test_name),
            oam_dma_end_test: self.oam_dma_end_test,
            oam_dma_end_test_name: self.oam_dma_end_test.and_then(test_name),
            dmc_dma_fetches_observed: self.dmc_dma_fetches_observed,
            dmc_dma_fetches_during_oam_dma: self.dmc_dma_fetches_during_oam_dma,
            dmc_dma_expected_min_oam_overlap_fetches: DMC_DMA_EXPECTED_MIN_OAM_OVERLAP_FETCHES,
            dmc_dma_oam_overlap_observed: self.dmc_dma_fetches_during_oam_dma
                >= DMC_DMA_EXPECTED_MIN_OAM_OVERLAP_FETCHES,
            dmc_dma_first_fetch_cycle: self.dmc_dma_first_fetch_cycle,
            dmc_dma_first_fetch_address: self.dmc_dma_first_fetch_address,
            dmc_dma_first_fetch_cpu_cycle_parity: self
                .dmc_dma_first_fetch_cpu_cycle_odd
                .map(|odd| cycle_parity_label(!odd)),
            dmc_dma_first_fetch_stall_cycles: self.dmc_dma_first_fetch_stall_cycles,
            dmc_dma_first_oam_overlap_cycle: self.dmc_dma_first_oam_overlap_cycle,
            dmc_dma_first_oam_overlap_test: self.dmc_dma_first_oam_overlap_test,
            dmc_dma_first_oam_overlap_test_name: self
                .dmc_dma_first_oam_overlap_test
                .and_then(test_name),
            dmc_dma_first_oam_overlap_cpu_cycle_parity: self
                .dmc_dma_first_oam_overlap_cpu_cycle_odd
                .map(|odd| cycle_parity_label(!odd)),
            dmc_dma_first_oam_overlap_stall_cycles: self.dmc_dma_first_oam_overlap_stall_cycles,
            dmc_dma_three_cycle_fetches: self.dmc_dma_three_cycle_fetches,
            dmc_dma_four_cycle_fetches: self.dmc_dma_four_cycle_fetches,
            dmc_dma_expected_min_stall_cycles: DMC_DMA_EXPECTED_MIN_STALL_CYCLES,
            dmc_dma_expected_max_stall_cycles: DMC_DMA_EXPECTED_MAX_STALL_CYCLES,
            dmc_dma_stall_cycles: self.dmc_dma_stall_cycles,
            dmc_dma_stall_cycles_after_oam_dma: self.dmc_dma_stall_cycles_after_oam_dma,
            dmc_dma_queued_during_oam_dma_cycles: self.dmc_dma_queued_during_oam_dma_cycles,
        }
    }
}

fn cycle_parity_label(even: bool) -> &'static str {
    if even {
        "even"
    } else {
        "odd"
    }
}

struct HostValidationInput<'a> {
    status: u8,
    timeout: bool,
    tests: &'a [TestTelemetry],
    ram: &'a [u8],
    dma: &'a DmaTelemetry,
    oam: &'a OamTelemetry,
    frame: &'a FrameTelemetry,
    audio_sample_count: usize,
    frames: u64,
}

struct ProbeTelemetryInput<'a> {
    status: u8,
    timeout: bool,
    current_test: u8,
    failure_code: u8,
    tests: &'a [TestTelemetry],
    ram: &'a [u8],
    dma: &'a DmaTelemetry,
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
    if !input.dma.oam_dma_observed {
        failures.push("OAM DMA transfer was not observed by the host runner".to_string());
    }
    if !input.dma.oam_dma_completed {
        failures.push("OAM DMA transfer did not complete before diagnostic pass".to_string());
    }
    if input.dma.oam_dma_active_cycles < input.dma.oam_dma_expected_min_cycles
        || input.dma.oam_dma_active_cycles > input.dma.oam_dma_expected_max_cycles
    {
        failures.push(format!(
            "OAM DMA active cycle count {} outside expected {}..={}",
            input.dma.oam_dma_active_cycles,
            input.dma.oam_dma_expected_min_cycles,
            input.dma.oam_dma_expected_max_cycles
        ));
    }
    if input.dma.dmc_dma_fetches_during_oam_dma < input.dma.dmc_dma_expected_min_oam_overlap_fetches
    {
        failures.push(format!(
            "expected at least {} DMC DMA fetch during OAM DMA, observed {}",
            input.dma.dmc_dma_expected_min_oam_overlap_fetches,
            input.dma.dmc_dma_fetches_during_oam_dma
        ));
    }
    if let Some(stall_cycles) = input.dma.dmc_dma_first_oam_overlap_stall_cycles {
        if stall_cycles < input.dma.dmc_dma_expected_min_stall_cycles
            || stall_cycles > input.dma.dmc_dma_expected_max_stall_cycles
        {
            failures.push(format!(
                "DMC DMA overlap stall count {} outside expected {}..={}",
                stall_cycles,
                input.dma.dmc_dma_expected_min_stall_cycles,
                input.dma.dmc_dma_expected_max_stall_cycles
            ));
        }
        if input.dma.dmc_dma_stall_cycles_after_oam_dma != u64::from(stall_cycles) {
            failures.push(format!(
                "post-OAM DMC stall count {} differed from overlap service bucket {}",
                input.dma.dmc_dma_stall_cycles_after_oam_dma, stall_cycles
            ));
        }
    } else {
        failures.push("DMC DMA overlap stall bucket was not observed".to_string());
    }
    if input.dma.dmc_dma_three_cycle_fetches + input.dma.dmc_dma_four_cycle_fetches
        != input.dma.dmc_dma_fetches_observed
    {
        failures.push(format!(
            "DMC DMA phase bucket count {}+{} did not match observed fetches {}",
            input.dma.dmc_dma_three_cycle_fetches,
            input.dma.dmc_dma_four_cycle_fetches,
            input.dma.dmc_dma_fetches_observed
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

fn probe_telemetry(input: ProbeTelemetryInput<'_>) -> Vec<DiagnosticProbeTelemetry> {
    let mut probes = Vec::new();

    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "runtime.completed".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: None,
            test_id: None,
            test_name: None,
            status: if !input.timeout && matches!(input.status, STATUS_PASS | STATUS_FAIL) {
                DiagnosticProbeStatus::Passed
            } else {
                DiagnosticProbeStatus::Failed
            },
            description: "Host runner reached a terminal cartridge status before the cycle budget"
                .to_string(),
            expected: "timeout=false and status is PASS or FAIL".to_string(),
            observed: format!(
                "timeout={}, status={}",
                input.timeout,
                hex_byte(input.status)
            ),
            likely_domain: "emulator.progress_or_infinite_loop".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ram.signature".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Bus),
            test_id: None,
            test_name: None,
            status: passed_or_failed(input.ram[SIGNATURE_ADDR as usize] == 0xA5),
            description: "Diagnostic cartridge wrote its RAM signature byte".to_string(),
            expected: "signature byte 0xA5 at $00F3".to_string(),
            observed: format!(
                "signature byte {}",
                hex_byte(input.ram[SIGNATURE_ADDR as usize])
            ),
            likely_domain: "bus.cpu_ram_or_reset_vector".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "cartridge.status.pass".to_string(),
            source: DiagnosticProbeSource::CartridgeResult,
            subsystem: None,
            test_id: known_test_id(input.current_test),
            test_name: test_name(input.current_test),
            status: passed_or_failed(input.status == STATUS_PASS),
            description: "Diagnostic cartridge reported the full suite passed".to_string(),
            expected: "status byte 0x80".to_string(),
            observed: format!("status byte {}", hex_byte(input.status)),
            likely_domain: status_probe_domain(input.status, input.failure_code),
        },
    );

    for test in input.tests {
        push_probe(
            &mut probes,
            ProbeTelemetryRecord {
                id: format!("cartridge.test.{}.result", test.id),
                source: DiagnosticProbeSource::CartridgeResult,
                subsystem: Some(test.subsystem),
                test_id: Some(test.id),
                test_name: Some(test.name),
                status: test_probe_status(test, input.status, input.timeout, input.current_test),
                description: format!("{} result byte", test.name),
                expected: "result byte 0x01".to_string(),
                observed: format!("result byte {}", hex_byte(test.result)),
                likely_domain: test_probe_domain(test, input.current_test, input.failure_code),
            },
        );
    }

    let passed_suite = input.status == STATUS_PASS;
    let active_ppu_render_test = input.timeout && input.current_test == 10;
    let should_validate_ppu_render_observations = passed_suite || active_ppu_render_test;
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ppu.nmi_count".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(10),
            test_name: test_name(10),
            status: gated_probe_status(
                should_validate_ppu_render_observations,
                input.ram[NMI_COUNT_ADDR as usize] >= 2,
            ),
            description: "PPU generated repeated NMIs during the render-frame test".to_string(),
            expected: "NMI count >= 2".to_string(),
            observed: format!("NMI count {}", input.ram[NMI_COUNT_ADDR as usize]),
            likely_domain: "ppu.nmi".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "oam.dma_checksum".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Dma),
            test_id: Some(5),
            test_name: test_name(5),
            status: gated_probe_status(
                passed_suite,
                input.oam.checksum == input.oam.expected_checksum,
            ),
            description: "Host-observed PPU OAM contents match the diagnostic DMA pattern"
                .to_string(),
            expected: format!("OAM checksum 0x{:016X}", input.oam.expected_checksum),
            observed: format!("OAM checksum 0x{:016X}", input.oam.checksum),
            likely_domain: "dma.oam_transfer".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "dma.oam_active_cycles".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Dma),
            test_id: Some(5),
            test_name: test_name(5),
            status: gated_probe_status(
                passed_suite,
                input.dma.oam_dma_completed
                    && input.dma.oam_dma_active_cycles >= input.dma.oam_dma_expected_min_cycles
                    && input.dma.oam_dma_active_cycles <= input.dma.oam_dma_expected_max_cycles,
            ),
            description: "Host-observed OAM DMA stalls CPU execution for the expected cycle bucket"
                .to_string(),
            expected: format!(
                "OAM DMA active cycles {}..={}",
                input.dma.oam_dma_expected_min_cycles, input.dma.oam_dma_expected_max_cycles
            ),
            observed: format!(
                "OAM DMA observed={}, completed={}, active cycles {}",
                input.dma.oam_dma_observed,
                input.dma.oam_dma_completed,
                input.dma.oam_dma_active_cycles
            ),
            likely_domain: "dma.oam_stall_timing".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "dma.dmc_oam_overlap".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Dma),
            test_id: Some(5),
            test_name: test_name(5),
            status: gated_probe_status(
                passed_suite,
                input.dma.dmc_dma_fetches_during_oam_dma
                    >= input.dma.dmc_dma_expected_min_oam_overlap_fetches,
            ),
            description: "DMC sample DMA is serviced while OAM DMA is holding the CPU"
                .to_string(),
            expected: format!(
                "DMC DMA fetches during OAM DMA >= {}",
                input.dma.dmc_dma_expected_min_oam_overlap_fetches
            ),
            observed: format!(
                "DMC fetches {}, overlapping fetches {}, overlap stall bucket {}, queued-during-OAM cycles {}, post-OAM DMC stall cycles {}",
                input.dma.dmc_dma_fetches_observed,
                input.dma.dmc_dma_fetches_during_oam_dma,
                optional_u8(input.dma.dmc_dma_first_oam_overlap_stall_cycles),
                input.dma.dmc_dma_queued_during_oam_dma_cycles,
                input.dma.dmc_dma_stall_cycles_after_oam_dma
            ),
            likely_domain: "dma.dmc_oam_interleaving".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "dma.dmc_stall_phase".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Dma),
            test_id: Some(5),
            test_name: test_name(5),
            status: gated_probe_status(
                passed_suite,
                input
                    .dma
                    .dmc_dma_first_oam_overlap_stall_cycles
                    .is_some_and(|cycles| {
                        cycles >= input.dma.dmc_dma_expected_min_stall_cycles
                            && cycles <= input.dma.dmc_dma_expected_max_stall_cycles
                            && input.dma.dmc_dma_stall_cycles_after_oam_dma == u64::from(cycles)
                    })
                    && input.dma.dmc_dma_three_cycle_fetches
                        + input.dma.dmc_dma_four_cycle_fetches
                        == input.dma.dmc_dma_fetches_observed,
            ),
            description: "DMC DMA service records phase-specific CPU stall bucket".to_string(),
            expected: format!(
                "DMC DMA stall bucket {}..={} cycles and phase buckets sum to observed fetches",
                input.dma.dmc_dma_expected_min_stall_cycles,
                input.dma.dmc_dma_expected_max_stall_cycles
            ),
            observed: format!(
                "first fetch parity {}, first fetch bucket {}, first overlap parity {}, first overlap bucket {}, three-cycle fetches {}, four-cycle fetches {}",
                input
                    .dma
                    .dmc_dma_first_fetch_cpu_cycle_parity
                    .unwrap_or("none"),
                optional_u8(input.dma.dmc_dma_first_fetch_stall_cycles),
                input
                    .dma
                    .dmc_dma_first_oam_overlap_cpu_cycle_parity
                    .unwrap_or("none"),
                optional_u8(input.dma.dmc_dma_first_oam_overlap_stall_cycles),
                input.dma.dmc_dma_three_cycle_fetches,
                input.dma.dmc_dma_four_cycle_fetches
            ),
            likely_domain: "dma.dmc_stall_phase".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ppu.frame_count".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(10),
            test_name: test_name(10),
            status: gated_probe_status(should_validate_ppu_render_observations, input.frames >= 2),
            description: "Host observed completed frames during the diagnostic run".to_string(),
            expected: "completed frames >= 2".to_string(),
            observed: format!("completed frames {}", input.frames),
            likely_domain: "ppu.frame_progress".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ppu.frame_unique_colors".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(10),
            test_name: test_name(10),
            status: gated_probe_status(
                should_validate_ppu_render_observations,
                input.frame.unique_colors >= 2,
            ),
            description: "Rendered diagnostic frame contains multiple colors".to_string(),
            expected: "unique rendered colors >= 2".to_string(),
            observed: format!("unique rendered colors {}", input.frame.unique_colors),
            likely_domain: "ppu.rendering.background".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "apu.sample_count".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Apu),
            test_id: Some(6),
            test_name: test_name(6),
            status: gated_probe_status(passed_suite, input.audio_sample_count > 0),
            description: "APU produced samples that the host runner drained at frame boundaries"
                .to_string(),
            expected: "drained audio samples > 0".to_string(),
            observed: format!("drained audio samples {}", input.audio_sample_count),
            likely_domain: "apu.frame_output".to_string(),
        },
    );

    probes
}

struct ProbeTelemetryRecord {
    id: String,
    source: DiagnosticProbeSource,
    subsystem: Option<DiagnosticSubsystem>,
    test_id: Option<u8>,
    test_name: Option<&'static str>,
    status: DiagnosticProbeStatus,
    description: String,
    expected: String,
    observed: String,
    likely_domain: String,
}

fn push_probe(probes: &mut Vec<DiagnosticProbeTelemetry>, record: ProbeTelemetryRecord) {
    probes.push(DiagnosticProbeTelemetry {
        id: record.id,
        source: record.source,
        subsystem: record.subsystem,
        test_id: record.test_id,
        test_name: record.test_name,
        status: record.status,
        description: record.description,
        expected: record.expected,
        observed: record.observed,
        likely_domain: record.likely_domain,
    });
}

fn passed_or_failed(passed: bool) -> DiagnosticProbeStatus {
    if passed {
        DiagnosticProbeStatus::Passed
    } else {
        DiagnosticProbeStatus::Failed
    }
}

fn gated_probe_status(gate: bool, passed: bool) -> DiagnosticProbeStatus {
    if !gate {
        DiagnosticProbeStatus::Skipped
    } else {
        passed_or_failed(passed)
    }
}

fn test_probe_status(
    test: &TestTelemetry,
    status: u8,
    timeout: bool,
    current_test: u8,
) -> DiagnosticProbeStatus {
    if test.passed {
        return DiagnosticProbeStatus::Passed;
    }
    if status == STATUS_PASS {
        return DiagnosticProbeStatus::Failed;
    }
    if status == STATUS_FAIL && test.id <= current_test {
        return DiagnosticProbeStatus::Failed;
    }
    if timeout && current_test != 0 && test.id == current_test {
        return DiagnosticProbeStatus::Failed;
    }
    DiagnosticProbeStatus::Skipped
}

fn status_probe_domain(status: u8, failure_code: u8) -> String {
    if status == STATUS_PASS {
        return "diagnostic.suite_status".to_string();
    }
    failure_spec(failure_code)
        .map(|failure| failure.likely_domain.to_string())
        .unwrap_or_else(|| "diagnostic.suite_status".to_string())
}

fn test_probe_domain(test: &TestTelemetry, current_test: u8, failure_code: u8) -> String {
    if test.id == current_test {
        if let Some(failure) = failure_spec(failure_code) {
            return failure.likely_domain.to_string();
        }
    }
    diagnostic_subsystem_probe_domain(test.subsystem).to_string()
}

fn diagnostic_subsystem_probe_domain(subsystem: DiagnosticSubsystem) -> &'static str {
    match subsystem {
        DiagnosticSubsystem::Cpu => "cpu.execution",
        DiagnosticSubsystem::Bus => "bus.memory",
        DiagnosticSubsystem::Ppu => "ppu.rendering",
        DiagnosticSubsystem::Apu => "apu.audio",
        DiagnosticSubsystem::Dma => "dma.transfer",
        DiagnosticSubsystem::Cartridge => "cartridge.mapper",
        DiagnosticSubsystem::Joypad => "joypad.input",
    }
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
    program.joypad2_strobe_shift();
    program.cpu_zero_page_index_wrap();
    program.cpu_indirect_jmp_page_wrap();
    program.ppu_vram_read_buffer();
    program.mapper2_prg_bank_switch();
    program.mapper2_prg_ram_roundtrip();
    program.ppu_horizontal_nametable_mirroring();
    program.joypad_strobe_reset_midstream();
    program.ppu_vram_increment_32();

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
        if let Some(name) = test_name(id) {
            self.asm
                .label(&format!("test_{id:02}_{name}"))
                .expect("test label should not collide");
        }
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
        self.asm.lda_imm(0x0F);
        self.asm.sta_abs(0x4010); // Fastest DMC rate, IRQ/loop disabled.
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4011);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4012); // Sample starts at $C000 in the fixed Mapper 2 PRG bank.
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4013); // 17 bytes, enough to request again during OAM DMA.
        self.asm.lda_imm(0x10);
        self.asm.sta_abs(0x4015); // Prime an immediate DMC sample fetch.
        self.asm
            .label(DMA_OAM_TRANSFER_FAULT_LABEL)
            .expect("fault injection label should not collide");
        self.asm.lda_imm(0x03);
        self.asm.sta_abs(0x4014);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4015); // Contain the DMC setup before later APU tests.
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
        self.asm
            .label(APU_STATUS_FAULT_LABEL)
            .expect("fault injection label should not collide");
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
        self.expect_serial_bits(0x4016, &expected, 0x70);
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

        self.asm
            .label(PPU_NMI_TIMEOUT_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_zp(NMI_COUNT_ADDR);
        self.asm.cmp_imm(0x02);
        self.asm.bne(PPU_NMI_TIMEOUT_FAULT_LABEL);
        self.pass_test(10);
    }

    fn joypad2_strobe_shift(&mut self) {
        self.begin_test(11);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4016);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4016);

        let expected = [0, 0, 0, 1, 0, 1, 0, 0];
        self.expect_serial_bits(0x4017, &expected, 0xA0);
        self.pass_test(11);
    }

    fn cpu_zero_page_index_wrap(&mut self) {
        self.begin_test(12);
        self.asm.lda_imm(0x3C);
        self.asm.sta_zp(0x80);
        self.asm.ldx_imm(0x81);
        self.asm
            .label(CPU_ZERO_PAGE_WRAP_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_zp_x(0xFF);
        self.expect_a_eq(0x3C, 0xB0);
        self.asm.lda_imm(0x6D);
        self.asm.sta_zp_x(0xFF);
        self.asm.lda_zp(0x80);
        self.expect_a_eq(0x6D, 0xB1);
        self.pass_test(12);
    }

    fn cpu_indirect_jmp_page_wrap(&mut self) {
        self.begin_test(13);
        let wrong_target = self.unique_label("indirect_jmp_wrong_target");
        let correct_target = self.unique_label("indirect_jmp_correct_target");
        self.asm.lda_label_low(&correct_target);
        self.asm.sta_abs(0x04FF);
        self.asm.lda_label_high(&correct_target);
        self.asm.sta_abs(0x0400);
        self.asm.lda_label_high(&wrong_target);
        self.asm.sta_abs(0x0500);
        self.asm
            .label(CPU_INDIRECT_JMP_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.jmp_indirect(0x04FF);

        self.asm.pad_until_low_byte(0x00);
        self.asm
            .label(&wrong_target)
            .expect("unique label should not collide");
        self.asm.lda_imm(0xC0);
        self.asm.sta_zp(FAILURE_CODE_ADDR);
        self.asm.jmp_label("fail");

        self.asm.pad_until_low_byte(0x00);
        self.asm
            .label(&correct_target)
            .expect("unique label should not collide");
        self.asm.lda_imm(0x7B);
        self.expect_a_eq(0x7B, 0xC1);
        self.pass_test(13);
    }

    fn ppu_vram_read_buffer(&mut self) {
        self.begin_test(14);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2000);
        self.asm.sta_abs(0x2001);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x2A);
        self.asm.sta_abs(0x2007);
        self.asm.lda_imm(0x6B);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm
            .label(PPU_READ_BUFFER_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_abs(0x2007);
        self.asm.lda_abs(0x2007);
        self.expect_a_eq(0x2A, 0xD0);
        self.asm.lda_abs(0x2007);
        self.expect_a_eq(0x6B, 0xD1);
        self.pass_test(14);
    }

    fn mapper2_prg_bank_switch(&mut self) {
        self.begin_test(15);
        for (bank, sentinel) in MAPPER2_BANK_SENTINELS {
            self.asm.lda_imm(*bank);
            self.asm.sta_abs(MAPPER2_SWITCHABLE_ADDR);
            if *bank == 1 {
                self.asm
                    .label(MAPPER2_BANK_SWITCH_FAULT_LABEL)
                    .expect("diagnostic fault-injection label should not collide");
            }
            self.asm.lda_abs(MAPPER2_SWITCHABLE_ADDR);
            self.expect_a_eq(*sentinel, 0xF0 + *bank);
        }
        self.asm.lda_abs(MAPPER2_FIXED_SENTINEL_ADDR);
        self.expect_a_eq(MAPPER2_FIXED_SENTINEL, 0xF3);
        self.pass_test(15);
    }

    fn mapper2_prg_ram_roundtrip(&mut self) {
        self.begin_test(16);
        self.asm.lda_imm(MAPPER2_PRG_RAM_LOW_SENTINEL);
        self.asm.sta_abs(MAPPER2_PRG_RAM_LOW_ADDR);
        self.asm.lda_imm(MAPPER2_PRG_RAM_HIGH_SENTINEL);
        self.asm.sta_abs(MAPPER2_PRG_RAM_HIGH_ADDR);

        self.asm.lda_imm(0x02);
        self.asm.sta_abs(MAPPER2_SWITCHABLE_ADDR);
        self.asm.lda_abs(MAPPER2_PRG_RAM_LOW_ADDR);
        self.expect_a_eq(MAPPER2_PRG_RAM_LOW_SENTINEL, 0xF4);
        self.asm.nop();
        self.asm
            .label(MAPPER2_PRG_RAM_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_abs(MAPPER2_PRG_RAM_HIGH_ADDR);
        self.expect_a_eq(MAPPER2_PRG_RAM_HIGH_SENTINEL, 0xF5);

        self.asm.lda_imm(0x00);
        self.asm.sta_abs(MAPPER2_SWITCHABLE_ADDR);
        self.asm.lda_abs(MAPPER2_PRG_RAM_LOW_ADDR);
        self.expect_a_eq(MAPPER2_PRG_RAM_LOW_SENTINEL, 0xF6);
        self.pass_test(16);
    }

    fn ppu_horizontal_nametable_mirroring(&mut self) {
        self.begin_test(17);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2000);
        self.asm.sta_abs(0x2001);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x43);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm
            .label(PPU_NAMETABLE_MIRRORING_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_imm(0x24);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_abs(0x2007);
        self.asm.lda_abs(0x2007);
        self.expect_a_eq(0x43, 0xE0);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x28);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x76);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x2C);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_abs(0x2007);
        self.asm.lda_abs(0x2007);
        self.expect_a_eq(0x76, 0xE1);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_abs(0x2007);
        self.asm.lda_abs(0x2007);
        self.expect_a_eq(0x43, 0xE2);
        self.pass_test(17);
    }

    fn joypad_strobe_reset_midstream(&mut self) {
        self.begin_test(18);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4016);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4016);
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_eq(0x01, 0x78);
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_eq(0x00, 0x79);

        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4016);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4016);
        self.asm
            .label(JOYPAD_STROBE_RESET_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_eq(0x01, 0x78);
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_eq(0x00, 0x79);
        self.pass_test(18);
    }

    fn ppu_vram_increment_32(&mut self) {
        self.begin_test(19);
        self.asm.lda_imm(0x04);
        self.asm.sta_abs(0x2000);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2001);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x31);
        self.asm.sta_abs(0x2007);
        self.asm.lda_imm(0x62);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm
            .label(PPU_VRAM_INCREMENT_32_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_abs(0x2007);
        self.asm.lda_abs(0x2007);
        self.expect_a_eq(0x62, 0x7A);

        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2000);
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x21);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x11);
        self.asm.sta_abs(0x2007);
        self.asm.lda_imm(0x22);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x21);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x2006);
        self.asm.lda_abs(0x2007);
        self.asm.lda_abs(0x2007);
        self.expect_a_eq(0x22, 0x7B);
        self.pass_test(19);
    }

    fn expect_serial_bits(&mut self, addr: u16, expected: &[u8], fail_base: u8) {
        for (index, expected_bit) in expected.iter().copied().enumerate() {
            self.asm.lda_abs(addr);
            self.asm.and_imm(0x01);
            self.expect_a_eq(expected_bit, fail_base + index as u8);
        }
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
    LabelLowByte,
    LabelHighByte,
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
                PatchKind::LabelLowByte => {
                    self.bytes[patch.at] = target as u8;
                }
                PatchKind::LabelHighByte => {
                    self.bytes[patch.at] = (target >> 8) as u8;
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

    fn op_imm_label_byte(&mut self, op: u8, label: &str, kind: PatchKind) {
        self.emit(op);
        let at = self.bytes.len();
        self.emit(0);
        self.patches.push(Patch {
            at,
            label: label.to_string(),
            kind,
        });
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

    fn lda_label_low(&mut self, label: &str) {
        self.op_imm_label_byte(0xA9, label, PatchKind::LabelLowByte);
    }

    fn lda_label_high(&mut self, label: &str) {
        self.op_imm_label_byte(0xA9, label, PatchKind::LabelHighByte);
    }

    fn lda_zp(&mut self, addr: u8) {
        self.op_zp(0xA5, addr);
    }

    fn lda_zp_x(&mut self, addr: u8) {
        self.op_zp(0xB5, addr);
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

    fn sta_zp_x(&mut self, addr: u8) {
        self.op_zp(0x95, addr);
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

    fn jmp_indirect(&mut self, addr: u16) {
        self.op_abs(0x6C, addr);
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

fn diagnostic_prg_offset_for_cpu_addr(addr: u16) -> usize {
    match addr {
        0x8000..=0xBFFF => (addr - 0x8000) as usize,
        0xC000..=0xFFFF => PROGRAM_PRG_OFFSET + (addr - 0xC000) as usize,
        _ => panic!("diagnostic PRG CPU address out of cartridge range: 0x{addr:04X}"),
    }
}

fn write_prg_cpu_byte(prg: &mut [u8], addr: u16, value: u8) {
    let index = diagnostic_prg_offset_for_cpu_addr(addr);
    prg[index] = value;
}

fn write_vector(prg: &mut [u8], vector_addr: u16, value: u16) {
    let index = diagnostic_prg_offset_for_cpu_addr(vector_addr);
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
        mapper: DIAGNOSTIC_MAPPER,
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
    let index = diagnostic_prg_offset_for_cpu_addr(vector_addr);
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

fn apply_joypad2_mask(bus: &mut Bus, mask: u8) {
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
        bus.joypad2
            .set_button_pressed(button, mask & (1 << index) != 0);
    }
}

fn diagnostic_input_telemetry(config: &DiagnosticConfig) -> DiagnosticInputTelemetry {
    DiagnosticInputTelemetry {
        joypad1_mask: config.joypad1_mask,
        joypad1_mask_hex: hex_byte(config.joypad1_mask),
        joypad1_expected_mask: EXPECTED_JOYPAD1_MASK,
        joypad1_expected_mask_hex: hex_byte(EXPECTED_JOYPAD1_MASK),
        joypad2_mask: config.joypad2_mask,
        joypad2_mask_hex: hex_byte(config.joypad2_mask),
        joypad2_expected_mask: EXPECTED_JOYPAD2_MASK,
        joypad2_expected_mask_hex: hex_byte(EXPECTED_JOYPAD2_MASK),
        fault_injection: config.fault_injection,
        fault_injection_label: config.fault_injection.map(DiagnosticFaultInjection::as_str),
    }
}

fn maybe_apply_diagnostic_fault_injection(
    bus: &mut Bus,
    fault: Option<DiagnosticFaultInjection>,
    fault_pc: Option<u16>,
    fault_injected: &mut bool,
    pc: u16,
) {
    if *fault_injected || fault_pc != Some(pc) {
        return;
    }
    if let Some(fault) = fault {
        apply_diagnostic_fault_injection(bus, fault);
        *fault_injected = true;
    }
}

fn apply_diagnostic_fault_injection(bus: &mut Bus, fault: DiagnosticFaultInjection) {
    match fault {
        DiagnosticFaultInjection::ApuStatusRegister => {
            bus.cpu_write(0x4015, 0x00);
        }
        DiagnosticFaultInjection::CpuIndirectJmpPageWrap => {
            let wrong_target_high = bus.cpu_read(0x0500);
            bus.cpu_write(0x0400, wrong_target_high);
        }
        DiagnosticFaultInjection::CpuZeroPageIndexWrap => {
            bus.cpu_write(0x0080, 0x00);
        }
        DiagnosticFaultInjection::DmaOamTransfer => {
            bus.cpu_write(0x0300, 0xFF);
        }
        DiagnosticFaultInjection::JoypadStrobeReset => {
            let _ = bus.cpu_read(0x4016);
        }
        DiagnosticFaultInjection::Mapper2PrgBankSwitch => {
            bus.cpu_write(MAPPER2_SWITCHABLE_ADDR, 0x00);
        }
        DiagnosticFaultInjection::Mapper2PrgRam => {
            bus.cpu_write(MAPPER2_PRG_RAM_HIGH_ADDR, 0x00);
        }
        DiagnosticFaultInjection::PpuNametableMirroring => {
            bus.cpu_write(0x2006, 0x24);
            bus.cpu_write(0x2006, 0x00);
            bus.cpu_write(0x2007, 0x00);
            let _ = bus.cpu_read(0x2002);
        }
        DiagnosticFaultInjection::PpuNmiTimeout => {
            bus.cpu_write(0x2000, 0x00);
        }
        DiagnosticFaultInjection::PpuVramIncrement32 => {
            bus.cpu_write(0x2006, 0x20);
            bus.cpu_write(0x2006, 0x20);
            bus.cpu_write(0x2007, 0x00);
            let _ = bus.cpu_read(0x2002);
        }
        DiagnosticFaultInjection::PpuVramReadBuffer => {
            bus.cpu_write(0x2006, 0x20);
            bus.cpu_write(0x2006, 0x00);
            bus.cpu_write(0x2007, 0x00);
            bus.cpu_write(0x2006, 0x20);
            bus.cpu_write(0x2006, 0x00);
        }
    }
}

fn read_ram_byte(bus: &mut Bus, addr: u8) -> u8 {
    bus.cpu_read(addr as u16)
}

fn cpu_telemetry(cpu: &Cpu) -> CpuTelemetry {
    CpuTelemetry {
        pc: cpu.pc,
        a: cpu.a,
        x: cpu.x,
        y: cpu.y,
        sp: cpu.sp,
        status: cpu.status,
        pending_cycles: cpu.cycles,
    }
}

fn diagnostic_ram_watch_telemetry(
    bus: &mut Bus,
    status: u8,
    current_test: u8,
) -> DiagnosticRamWatchTelemetry {
    let failure_code = read_ram_byte(bus, FAILURE_CODE_ADDR);
    let signature = read_ram_byte(bus, SIGNATURE_ADDR);
    let nmi_count = read_ram_byte(bus, NMI_COUNT_ADDR);
    let current_result_addr = known_test_id(current_test).map(result_addr);
    let current_result = current_result_addr.map(|addr| bus.cpu_read(addr));

    DiagnosticRamWatchTelemetry {
        status,
        status_hex: hex_byte(status),
        current_test,
        current_test_name: test_name(current_test),
        failure_code,
        failure_code_hex: hex_byte(failure_code),
        signature,
        signature_hex: hex_byte(signature),
        nmi_count,
        current_result_addr,
        current_result_addr_hex: current_result_addr.map(format_pc),
        current_result,
        current_result_hex: current_result.map(hex_byte),
    }
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
    probes: &[DiagnosticProbeTelemetry],
) -> Option<DiagnosticFailureTelemetry> {
    if passed {
        return None;
    }

    let current_spec = test_spec(current_test);
    if timeout {
        let likely_domain = timeout_likely_domain(current_test);
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
            likely_domain: likely_domain.to_string(),
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

    let failed_probe = primary_failed_probe(probes);
    let failure_test_id = failed_probe
        .and_then(|probe| probe.test_id)
        .unwrap_or(current_test);
    let failure_spec = test_spec(failure_test_id);
    let failure_test_name = failed_probe
        .and_then(|probe| probe.test_name)
        .or_else(|| failure_spec.map(|spec| spec.name));
    let failure_subsystem = failed_probe
        .and_then(|probe| probe.subsystem)
        .or_else(|| failure_spec.map(|spec| spec.subsystem));

    Some(DiagnosticFailureTelemetry {
        kind: DiagnosticFailureKind::HostValidation,
        test_id: failure_test_id,
        test_name: failure_test_name,
        subsystem: failure_subsystem,
        tier: failure_spec.map(|spec| spec.tier),
        failure_code,
        failure_code_hex: hex_byte(failure_code),
        assertion: failed_probe.map_or_else(
            || "host-side diagnostic validation completed without failures".to_string(),
            |probe| probe.description.clone(),
        ),
        expected: failed_probe.map_or_else(
            || "host_failures is empty after cartridge completion".to_string(),
            |probe| probe.expected.clone(),
        ),
        observed: failed_probe.map_or_else(
            || {
                if host_failures.is_empty() {
                    "host validation failed without a detailed message".to_string()
                } else {
                    host_failures.join("; ")
                }
            },
            |probe| probe.observed.clone(),
        ),
        likely_domain: failed_probe.map_or_else(
            || "host.validation".to_string(),
            |probe| probe.likely_domain.clone(),
        ),
        remediation_hint: failed_probe.map_or_else(
            || {
                "Inspect host telemetry checks for OAM, frame, audio, RAM signature, and per-test result bytes."
                    .to_string()
            },
            |probe| {
                format!(
                    "Inspect failed probe {} plus its subsystem telemetry before broadening the search.",
                    probe.id
                )
            },
        ),
    })
}

fn timeout_likely_domain(current_test: u8) -> &'static str {
    match current_test {
        10 => "ppu.nmi",
        _ => "emulator.progress_or_infinite_loop",
    }
}

struct AnalysisTelemetryInput<'a> {
    verdict: &'a VerdictTelemetry,
    tests: &'a [TestTelemetry],
    timeline: &'a [TestTimelineTelemetry],
    probes: &'a [DiagnosticProbeTelemetry],
    instruction_trace: &'a InstructionTraceTelemetry,
    events: &'a [EventTelemetry],
    cycles: u64,
    frames: u64,
}

fn analysis_telemetry(input: AnalysisTelemetryInput<'_>) -> DiagnosticAnalysisTelemetry {
    let coverage = coverage_telemetry(input.tests);
    let timing = timing_summary(input.timeline);
    let probe_summary = probe_summary(input.probes);
    let health = diagnostic_health(input.verdict);
    let test_transition_count = input
        .events
        .iter()
        .filter(|event| event.kind == DiagnosticEventKind::TestChanged)
        .count();

    let failing_subsystem = input
        .verdict
        .failure
        .as_ref()
        .and_then(|failure| failure.subsystem)
        .or_else(|| first_failed_test(input.tests).map(|test| test.subsystem));
    let failing_test = input
        .verdict
        .failure
        .as_ref()
        .and_then(|failure| failure.test_name)
        .or_else(|| first_failed_test(input.tests).map(|test| test.name));
    let first_failure_domain = input
        .verdict
        .failure
        .as_ref()
        .map(|failure| failure.likely_domain.clone());

    let summary = analysis_summary(
        health,
        &coverage,
        input.verdict,
        failing_test,
        first_failure_domain.as_deref(),
        input.cycles,
        input.frames,
    );
    let next_actions = analysis_next_actions(health, input.verdict);
    let debug_focus = debug_focus_telemetry(
        health,
        input.verdict,
        input.probes,
        input.events,
        input.instruction_trace,
    );

    DiagnosticAnalysisTelemetry {
        health,
        summary,
        debug_focus,
        coverage,
        coverage_gaps: coverage_gap_telemetry(),
        timing,
        probe_summary,
        failing_subsystem,
        failing_test,
        first_failure_domain,
        next_actions,
        test_transition_count,
    }
}

fn debug_focus_telemetry(
    health: DiagnosticHealth,
    verdict: &VerdictTelemetry,
    probes: &[DiagnosticProbeTelemetry],
    events: &[EventTelemetry],
    instruction_trace: &InstructionTraceTelemetry,
) -> DiagnosticDebugFocusTelemetry {
    let focus_test_id = verdict
        .failure
        .as_ref()
        .map(|failure| failure.test_id)
        .unwrap_or(verdict.current_test);
    let focus_spec = test_spec(focus_test_id);
    let focus_test_name = verdict
        .failure
        .as_ref()
        .and_then(|failure| failure.test_name)
        .or_else(|| focus_spec.map(|spec| spec.name))
        .or(verdict.current_test_name);
    let focus_subsystem = verdict
        .failure
        .as_ref()
        .and_then(|failure| failure.subsystem)
        .or_else(|| focus_spec.map(|spec| spec.subsystem));
    let focus_domain = verdict
        .failure
        .as_ref()
        .map(|failure| failure.likely_domain.clone())
        .or_else(|| {
            probes
                .iter()
                .find(|probe| probe.status == DiagnosticProbeStatus::Failed)
                .map(|probe| probe.likely_domain.clone())
        });
    let failure_kind = verdict.failure.as_ref().map(|failure| failure.kind);
    let failed_probe_ids = probes
        .iter()
        .filter(|probe| probe.status == DiagnosticProbeStatus::Failed)
        .take(8)
        .map(|probe| probe.id.clone())
        .collect();
    let skipped_probe_count = probes
        .iter()
        .filter(|probe| probe.status == DiagnosticProbeStatus::Skipped)
        .count();
    let terminal_instruction = instruction_trace
        .tail
        .last()
        .map(debug_instruction_focus_telemetry);
    let last_test_instruction = instruction_trace
        .tail
        .iter()
        .rev()
        .find(|entry| {
            entry.diagnostic_ram.current_test == focus_test_id
                && entry
                    .symbol
                    .as_ref()
                    .is_none_or(|symbol| symbol.name != "hang")
        })
        .or_else(|| {
            instruction_trace
                .tail
                .iter()
                .rev()
                .find(|entry| entry.diagnostic_ram.current_test == focus_test_id)
        })
        .map(debug_instruction_focus_telemetry);

    DiagnosticDebugFocusTelemetry {
        health,
        focus_test_id,
        focus_test_name,
        focus_subsystem,
        focus_domain,
        failure_kind,
        failure_code_hex: hex_byte(verdict.failure_code),
        failed_probe_ids,
        skipped_probe_count,
        last_event: events.last().map(debug_event_focus_telemetry),
        terminal_instruction,
        last_test_instruction,
    }
}

fn debug_event_focus_telemetry(event: &EventTelemetry) -> DiagnosticDebugEventFocusTelemetry {
    DiagnosticDebugEventFocusTelemetry {
        kind: event.kind,
        cycle: event.cycle,
        frame: event.frame,
        status_hex: hex_byte(event.status),
        current_test: event.current_test,
        current_test_name: event.current_test_name,
        pc_hex: format_pc(event.pc),
        note: event.note.clone(),
    }
}

fn debug_instruction_focus_telemetry(
    entry: &InstructionTraceEntryTelemetry,
) -> DiagnosticDebugInstructionFocusTelemetry {
    DiagnosticDebugInstructionFocusTelemetry {
        sequence: entry.sequence,
        cycle: entry.cycle,
        frame: entry.frame,
        current_test: entry.diagnostic_ram.current_test,
        current_test_name: entry.diagnostic_ram.current_test_name,
        pc_hex: entry.pc_hex.clone(),
        instruction: entry
            .instruction
            .as_ref()
            .map(|instruction| instruction.text.clone()),
        symbol: entry.symbol.as_ref().map(format_symbol),
        status_hex: entry.diagnostic_ram.status_hex.clone(),
        current_result_hex: entry.diagnostic_ram.current_result_hex.clone(),
        failure_code_hex: entry.diagnostic_ram.failure_code_hex.clone(),
    }
}

fn coverage_gap_telemetry() -> Vec<DiagnosticCoverageGapTelemetry> {
    DIAGNOSTIC_COVERAGE_GAPS
        .iter()
        .map(|gap| DiagnosticCoverageGapTelemetry {
            id: gap.id,
            subsystem: gap.subsystem,
            risk: gap.risk,
            current_coverage: gap.current_coverage,
            missing_coverage: gap.missing_coverage,
            suggested_next_test: gap.suggested_next_test,
        })
        .collect()
}

fn probe_summary(probes: &[DiagnosticProbeTelemetry]) -> DiagnosticProbeSummaryTelemetry {
    let passed_probes = probes
        .iter()
        .filter(|probe| probe.status == DiagnosticProbeStatus::Passed)
        .count();
    let failed_probes = probes
        .iter()
        .filter(|probe| probe.status == DiagnosticProbeStatus::Failed)
        .count();
    let skipped_probes = probes
        .iter()
        .filter(|probe| probe.status == DiagnosticProbeStatus::Skipped)
        .count();
    let first_failed_probe = probes
        .iter()
        .find(|probe| probe.status == DiagnosticProbeStatus::Failed)
        .map(|probe| probe.id.clone());

    DiagnosticProbeSummaryTelemetry {
        total_probes: probes.len(),
        passed_probes,
        failed_probes,
        skipped_probes,
        first_failed_probe,
    }
}

fn coverage_telemetry(tests: &[TestTelemetry]) -> DiagnosticCoverageTelemetry {
    let passed_tests = tests.iter().filter(|test| test.passed).count();
    let failed_tests = tests.len().saturating_sub(passed_tests);

    DiagnosticCoverageTelemetry {
        total_tests: tests.len(),
        passed_tests,
        failed_tests,
        subsystem_summary: subsystem_coverage(tests),
        tier_summary: tier_coverage(tests),
    }
}

fn subsystem_coverage(tests: &[TestTelemetry]) -> Vec<SubsystemCoverageTelemetry> {
    let mut summary = Vec::new();
    for spec in DIAGNOSTIC_TESTS {
        if summary
            .iter()
            .any(|entry: &SubsystemCoverageTelemetry| entry.subsystem == spec.subsystem)
        {
            continue;
        }
        let mut entry = SubsystemCoverageTelemetry {
            subsystem: spec.subsystem,
            total: 0,
            passed: 0,
            failed: 0,
        };
        for test in tests.iter().filter(|test| test.subsystem == spec.subsystem) {
            entry.total += 1;
            if test.passed {
                entry.passed += 1;
            } else {
                entry.failed += 1;
            }
        }
        summary.push(entry);
    }
    summary
}

fn tier_coverage(tests: &[TestTelemetry]) -> Vec<TierCoverageTelemetry> {
    let mut summary = Vec::new();
    for spec in DIAGNOSTIC_TESTS {
        if summary
            .iter()
            .any(|entry: &TierCoverageTelemetry| entry.tier == spec.tier)
        {
            continue;
        }
        let mut entry = TierCoverageTelemetry {
            tier: spec.tier,
            total: 0,
            passed: 0,
            failed: 0,
        };
        for test in tests.iter().filter(|test| test.tier == spec.tier) {
            entry.total += 1;
            if test.passed {
                entry.passed += 1;
            } else {
                entry.failed += 1;
            }
        }
        summary.push(entry);
    }
    summary
}

fn diagnostic_health(verdict: &VerdictTelemetry) -> DiagnosticHealth {
    if verdict.passed {
        return DiagnosticHealth::Healthy;
    }

    match verdict.failure.as_ref().map(|failure| failure.kind) {
        Some(DiagnosticFailureKind::Timeout) => DiagnosticHealth::TimedOut,
        Some(DiagnosticFailureKind::CartridgeAssertion) => {
            DiagnosticHealth::CartridgeAssertionFailed
        }
        _ => DiagnosticHealth::HostValidationFailed,
    }
}

fn analysis_summary(
    health: DiagnosticHealth,
    coverage: &DiagnosticCoverageTelemetry,
    verdict: &VerdictTelemetry,
    failing_test: Option<&str>,
    first_failure_domain: Option<&str>,
    cycles: u64,
    frames: u64,
) -> String {
    match health {
        DiagnosticHealth::Healthy => format!(
            "diagnostic passed: {}/{} tests across {} subsystems in {} cycles and {} frames",
            coverage.passed_tests,
            coverage.total_tests,
            coverage.subsystem_summary.len(),
            cycles,
            frames
        ),
        DiagnosticHealth::CartridgeAssertionFailed => format!(
            "diagnostic failed at {} with failure {} in {}",
            failing_test.unwrap_or("unknown_test"),
            hex_byte(verdict.failure_code),
            first_failure_domain.unwrap_or("unknown_domain")
        ),
        DiagnosticHealth::TimedOut => format!(
            "diagnostic timed out while current_test={} ({}) after {} cycles",
            verdict.current_test,
            verdict.current_test_name.unwrap_or("unknown_test"),
            cycles
        ),
        DiagnosticHealth::HostValidationFailed => format!(
            "diagnostic reached cartridge status {} but host validation reported {} issue(s)",
            hex_byte(verdict.status),
            verdict.host_failures.len()
        ),
    }
}

fn analysis_next_actions(health: DiagnosticHealth, verdict: &VerdictTelemetry) -> Vec<String> {
    match health {
        DiagnosticHealth::Healthy => vec![
            "Use this telemetry as the current generated-cartridge baseline.".to_string(),
            "Diff future failing runs against the coverage summary, instruction trace tail, event transitions, and frame/audio checks.".to_string(),
        ],
        DiagnosticHealth::CartridgeAssertionFailed => {
            if let Some(failure) = &verdict.failure {
                vec![
                    failure.remediation_hint.clone(),
                    format!(
                        "Start with test {} ({}) and failure {} before investigating later unrun tests.",
                        failure.test_id,
                        failure.test_name.unwrap_or("unknown_test"),
                        failure.failure_code_hex
                    ),
                    "Use instruction_trace.tail and test_changed events to inspect the last executed instructions before failure.".to_string(),
                ]
            } else {
                vec![
                    "Inspect verdict.current_test and verdict.failure_code; no catalog entry was available.".to_string(),
                    "Use the cartridge failure catalog to add or correct the missing assertion mapping.".to_string(),
                ]
            }
        }
        DiagnosticHealth::TimedOut => vec![
            "Inspect instruction_trace.tail, the current test, CPU PC, and final test_changed event to locate the likely loop.".to_string(),
            "Rerun with a higher --max-cycles only after confirming progress is still being made.".to_string(),
        ],
        DiagnosticHealth::HostValidationFailed => vec![
            "Inspect host_failures first; they validate emulator-side state the cartridge cannot read.".to_string(),
            "Compare OAM, frame, audio, RAM signature, instruction trace, and per-test result telemetry against expected values.".to_string(),
        ],
    }
}

struct EventTelemetryInput<'a> {
    cycle: u64,
    frame: u64,
    status: u8,
    current_test: u8,
    pc: u16,
    cpu: CpuTelemetry,
    diagnostic_ram: DiagnosticRamWatchTelemetry,
    kind: DiagnosticEventKind,
    note: &'a str,
}

fn event_telemetry(input: EventTelemetryInput<'_>) -> EventTelemetry {
    EventTelemetry {
        kind: input.kind,
        cycle: input.cycle,
        frame: input.frame,
        status: input.status,
        current_test: input.current_test,
        current_test_name: test_name(input.current_test),
        pc: input.pc,
        cpu: input.cpu,
        diagnostic_ram: input.diagnostic_ram,
        note: input.note.to_string(),
    }
}

#[derive(Debug, Clone, Copy)]
struct TimelineMark {
    cycle: u64,
    frame: u64,
    status: u8,
    pc: u16,
}

#[derive(Debug, Clone, Copy, Default)]
struct TimelineSlot {
    start: Option<TimelineMark>,
    end: Option<TimelineMark>,
    end_reason: Option<TestTimelineEndReason>,
}

fn test_timeline(
    tests: &[TestTelemetry],
    events: &[EventTelemetry],
    verdict: &VerdictTelemetry,
    final_cycles: u64,
    final_frames: u64,
    final_pc: u16,
) -> Vec<TestTimelineTelemetry> {
    let mut slots = HashMap::<u8, TimelineSlot>::new();
    let mut active_test = None;

    for event in events {
        match event.kind {
            DiagnosticEventKind::Reset => {
                if test_spec(event.current_test).is_some() {
                    start_timeline_slot(&mut slots, event.current_test, mark_from_event(event));
                    active_test = Some(event.current_test);
                }
            }
            DiagnosticEventKind::TestChanged => {
                if let Some(previous_test) = active_test {
                    if previous_test != event.current_test {
                        end_timeline_slot(
                            &mut slots,
                            previous_test,
                            mark_from_event(event),
                            TestTimelineEndReason::NextTestStarted,
                        );
                    }
                }

                active_test = if test_spec(event.current_test).is_some() {
                    start_timeline_slot(&mut slots, event.current_test, mark_from_event(event));
                    Some(event.current_test)
                } else {
                    None
                };
            }
            DiagnosticEventKind::StatusChanged if event.status == STATUS_PASS => {
                if let Some(current_test) = active_test {
                    end_timeline_slot(
                        &mut slots,
                        current_test,
                        mark_from_event(event),
                        TestTimelineEndReason::CartridgePassed,
                    );
                }
            }
            DiagnosticEventKind::StatusChanged if event.status == STATUS_FAIL => {
                if let Some(current_test) = active_test {
                    end_timeline_slot(
                        &mut slots,
                        current_test,
                        mark_from_event(event),
                        TestTimelineEndReason::CartridgeFailed,
                    );
                }
            }
            _ => {}
        }
    }

    if verdict.timeout {
        if let Some(current_test) =
            active_test.or_else(|| test_spec(verdict.current_test).map(|_| verdict.current_test))
        {
            if slots
                .get(&current_test)
                .and_then(|slot| slot.start)
                .is_some()
            {
                end_timeline_slot(
                    &mut slots,
                    current_test,
                    TimelineMark {
                        cycle: final_cycles,
                        frame: final_frames,
                        status: verdict.status,
                        pc: final_pc,
                    },
                    TestTimelineEndReason::Timeout,
                );
            }
        }
    }

    tests
        .iter()
        .map(|test| {
            let slot = slots.get(&test.id).copied().unwrap_or_default();
            timeline_telemetry(test, slot, verdict)
        })
        .collect()
}

fn start_timeline_slot(slots: &mut HashMap<u8, TimelineSlot>, test_id: u8, mark: TimelineMark) {
    let slot = slots.entry(test_id).or_default();
    if slot.start.is_none() {
        slot.start = Some(mark);
    }
}

fn end_timeline_slot(
    slots: &mut HashMap<u8, TimelineSlot>,
    test_id: u8,
    mark: TimelineMark,
    reason: TestTimelineEndReason,
) {
    let slot = slots.entry(test_id).or_default();
    if slot.end.is_none() {
        slot.end = Some(mark);
        slot.end_reason = Some(reason);
    }
}

fn mark_from_event(event: &EventTelemetry) -> TimelineMark {
    TimelineMark {
        cycle: event.cycle,
        frame: event.frame,
        status: event.status,
        pc: event.pc,
    }
}

fn timeline_telemetry(
    test: &TestTelemetry,
    slot: TimelineSlot,
    verdict: &VerdictTelemetry,
) -> TestTimelineTelemetry {
    let duration_cycles = slot
        .start
        .zip(slot.end)
        .map(|(start, end)| end.cycle.saturating_sub(start.cycle));
    let duration_frames = slot
        .start
        .zip(slot.end)
        .map(|(start, end)| end.frame.saturating_sub(start.frame));
    let outcome = timeline_outcome(test, slot, verdict);

    TestTimelineTelemetry {
        test_id: test.id,
        test_name: test.name,
        subsystem: test.subsystem,
        tier: test.tier,
        outcome,
        started: slot.start.is_some(),
        ended: slot.end.is_some(),
        start_cycle: slot.start.map(|mark| mark.cycle),
        end_cycle: slot.end.map(|mark| mark.cycle),
        duration_cycles,
        start_frame: slot.start.map(|mark| mark.frame),
        end_frame: slot.end.map(|mark| mark.frame),
        duration_frames,
        end_reason: slot.end_reason,
        terminal_status: slot.end.map(|mark| mark.status),
        terminal_status_hex: slot.end.map(|mark| hex_byte(mark.status)),
        terminal_pc: slot.end.map(|mark| mark.pc),
    }
}

fn timeline_outcome(
    test: &TestTelemetry,
    slot: TimelineSlot,
    verdict: &VerdictTelemetry,
) -> TestTimelineOutcome {
    if slot.start.is_none() {
        return TestTimelineOutcome::NotStarted;
    }
    if slot.end_reason == Some(TestTimelineEndReason::Timeout) {
        return TestTimelineOutcome::TimedOut;
    }
    if test.passed {
        return TestTimelineOutcome::Passed;
    }
    if verdict.status == STATUS_FAIL && verdict.current_test == test.id {
        return TestTimelineOutcome::Failed;
    }
    TestTimelineOutcome::Incomplete
}

fn timing_summary(timeline: &[TestTimelineTelemetry]) -> DiagnosticTimingSummaryTelemetry {
    let started_tests = timeline.iter().filter(|test| test.started).count();
    let ended_tests = timeline.iter().filter(|test| test.ended).count();
    let timed_out_tests = timeline
        .iter()
        .filter(|test| test.outcome == TestTimelineOutcome::TimedOut)
        .count();
    let slowest_test = timeline
        .iter()
        .filter_map(|test| {
            test.duration_cycles
                .map(|duration_cycles| TestDurationTelemetry {
                    test_id: test.test_id,
                    test_name: test.test_name,
                    subsystem: test.subsystem,
                    tier: test.tier,
                    duration_cycles,
                    duration_frames: test.duration_frames.unwrap_or_default(),
                })
        })
        .max_by_key(|test| test.duration_cycles);

    DiagnosticTimingSummaryTelemetry {
        started_tests,
        ended_tests,
        not_started_tests: timeline.len().saturating_sub(started_tests),
        timed_out_tests,
        slowest_test,
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

fn known_test_id(id: u8) -> Option<u8> {
    test_spec(id).map(|spec| spec.id)
}

fn first_failed_test(tests: &[TestTelemetry]) -> Option<&TestTelemetry> {
    tests.iter().find(|test| !test.passed)
}

fn primary_failed_probe(probes: &[DiagnosticProbeTelemetry]) -> Option<&DiagnosticProbeTelemetry> {
    probes
        .iter()
        .find(|probe| probe.status == DiagnosticProbeStatus::Failed)
}

fn test_spec(id: u8) -> Option<&'static DiagnosticTestSpec> {
    DIAGNOSTIC_TESTS.iter().find(|spec| spec.id == id)
}

fn failure_spec(code: u8) -> Option<&'static DiagnosticFailureSpec> {
    DIAGNOSTIC_FAILURES
        .iter()
        .find(|failure| failure.code == code)
}

fn compare_schema(
    baseline: &Value,
    current: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    compare_optional_value(
        baseline,
        current,
        &["schema_version"],
        "schema",
        DiagnosticComparisonSeverity::Warning,
        "baseline and current telemetry use different schema versions",
        differences,
    );
    compare_optional_value(
        baseline,
        current,
        &["suite", "version"],
        "suite",
        DiagnosticComparisonSeverity::Warning,
        "diagnostic suite version changed from baseline",
        differences,
    );
}

fn compare_input(
    baseline: &Value,
    current: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    for path in [
        &["input", "joypad1_mask_hex"][..],
        &["input", "joypad1_expected_mask_hex"][..],
        &["input", "joypad2_mask_hex"][..],
        &["input", "joypad2_expected_mask_hex"][..],
    ] {
        compare_optional_value(
            baseline,
            current,
            path,
            "input",
            DiagnosticComparisonSeverity::Warning,
            "diagnostic input mask changed from baseline",
            differences,
        );
    }
}

fn compare_dma(
    baseline: &Value,
    current: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    for path in [
        &["dma", "oam_dma_observed"][..],
        &["dma", "oam_dma_completed"][..],
        &["dma", "oam_dma_active_cycles"][..],
        &["dma", "oam_dma_first_active_cycle_parity"][..],
        &["dma", "oam_dma_start_test_name"][..],
        &["dma", "oam_dma_end_test_name"][..],
        &["dma", "dmc_dma_fetches_during_oam_dma"][..],
        &["dma", "dmc_dma_oam_overlap_observed"][..],
        &["dma", "dmc_dma_first_oam_overlap_test_name"][..],
        &["dma", "dmc_dma_first_fetch_cpu_cycle_parity"][..],
        &["dma", "dmc_dma_first_fetch_stall_cycles"][..],
        &["dma", "dmc_dma_first_oam_overlap_cpu_cycle_parity"][..],
        &["dma", "dmc_dma_first_oam_overlap_stall_cycles"][..],
        &["dma", "dmc_dma_three_cycle_fetches"][..],
        &["dma", "dmc_dma_four_cycle_fetches"][..],
        &["dma", "dmc_dma_stall_cycles_after_oam_dma"][..],
    ] {
        compare_optional_value(
            baseline,
            current,
            path,
            "dma",
            DiagnosticComparisonSeverity::Warning,
            "DMA timing telemetry changed from baseline",
            differences,
        );
    }
}

fn compare_verdict(
    baseline: &Value,
    current: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    if json_bool(baseline, &["verdict", "passed"]) != json_bool(current, &["verdict", "passed"]) {
        let current_passed = json_bool(current, &["verdict", "passed"]).unwrap_or(false);
        push_difference(
            differences,
            if current_passed {
                DiagnosticComparisonSeverity::Info
            } else {
                DiagnosticComparisonSeverity::Failure
            },
            "verdict",
            "verdict.passed",
            json_path_display(baseline, &["verdict", "passed"]),
            json_path_display(current, &["verdict", "passed"]),
            if current_passed {
                "current run passes where baseline did not"
            } else {
                "current run fails where baseline passed or was missing"
            },
        );
    }

    if json_string(baseline, &["analysis", "health"])
        != json_string(current, &["analysis", "health"])
    {
        let current_health = json_string(current, &["analysis", "health"]);
        push_difference(
            differences,
            if current_health.as_deref() == Some("healthy") {
                DiagnosticComparisonSeverity::Info
            } else {
                DiagnosticComparisonSeverity::Failure
            },
            "verdict",
            "analysis.health",
            json_path_display(baseline, &["analysis", "health"]),
            json_path_display(current, &["analysis", "health"]),
            "diagnostic health changed from baseline",
        );
    }

    compare_optional_value(
        baseline,
        current,
        &["analysis", "first_failure_domain"],
        "verdict",
        DiagnosticComparisonSeverity::Warning,
        "first failure domain changed",
        differences,
    );
}

fn compare_coverage(
    baseline: &Value,
    current: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    compare_u64_regression(
        baseline,
        current,
        U64RegressionComparison {
            path: &["analysis", "coverage", "passed_tests"],
            category: "coverage",
            lower_is_regression: true,
            regression_note: "fewer diagnostic tests passed than in baseline",
            improvement_note: "more diagnostic tests passed than in baseline",
        },
        differences,
    );
    compare_u64_regression(
        baseline,
        current,
        U64RegressionComparison {
            path: &["analysis", "coverage", "failed_tests"],
            category: "coverage",
            lower_is_regression: false,
            regression_note: "more diagnostic tests failed than in baseline",
            improvement_note: "fewer diagnostic tests failed than in baseline",
        },
        differences,
    );
    compare_u64_regression(
        baseline,
        current,
        U64RegressionComparison {
            path: &["analysis", "timing", "not_started_tests"],
            category: "coverage",
            lower_is_regression: false,
            regression_note: "more diagnostic tests were skipped or not reached than in baseline",
            improvement_note: "fewer diagnostic tests were skipped or not reached than in baseline",
        },
        differences,
    );
}

fn compare_probes(
    baseline: &Value,
    current: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    compare_u64_regression(
        baseline,
        current,
        U64RegressionComparison {
            path: &["analysis", "probe_summary", "passed_probes"],
            category: "probes",
            lower_is_regression: true,
            regression_note: "fewer observation probes passed than in baseline",
            improvement_note: "more observation probes passed than in baseline",
        },
        differences,
    );
    compare_u64_regression(
        baseline,
        current,
        U64RegressionComparison {
            path: &["analysis", "probe_summary", "failed_probes"],
            category: "probes",
            lower_is_regression: false,
            regression_note: "more observation probes failed than in baseline",
            improvement_note: "fewer observation probes failed than in baseline",
        },
        differences,
    );

    let baseline_probes = probe_by_id(baseline);
    let current_probes = probe_by_id(current);
    if baseline_probes.is_empty() || current_probes.is_empty() {
        return;
    }

    for (probe_id, current_probe) in &current_probes {
        let path = format!("probes[{probe_id}]");
        let Some(baseline_probe) = baseline_probes.get(probe_id) else {
            push_difference(
                differences,
                DiagnosticComparisonSeverity::Info,
                "probes",
                &path,
                None,
                Some("present".to_string()),
                "current run includes an observation probe absent from baseline",
            );
            continue;
        };

        let baseline_status = json_string(baseline_probe, &["status"]);
        let current_status = json_string(current_probe, &["status"]);
        if baseline_status != current_status {
            push_difference(
                differences,
                probe_status_change_severity(current_status.as_deref()),
                "probes",
                &format!("{path}.status"),
                baseline_status,
                current_status.clone(),
                "observation probe status changed from baseline",
            );
        } else if current_status.as_deref() == Some("passed") {
            compare_probe_observed_value(probe_id, baseline_probe, current_probe, differences);
        }
    }

    for probe_id in baseline_probes.keys() {
        if !current_probes.contains_key(probe_id) {
            push_difference(
                differences,
                DiagnosticComparisonSeverity::Failure,
                "probes",
                &format!("probes[{probe_id}]"),
                Some("present".to_string()),
                None,
                "baseline observation probe is missing from current run",
            );
        }
    }
}

fn compare_probe_observed_value(
    probe_id: &str,
    baseline_probe: &Value,
    current_probe: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    let baseline_observed = json_string(baseline_probe, &["observed"]);
    let current_observed = json_string(current_probe, &["observed"]);
    if baseline_observed != current_observed {
        push_difference(
            differences,
            DiagnosticComparisonSeverity::Warning,
            "probes",
            &format!("probes[{probe_id}].observed"),
            baseline_observed,
            current_observed,
            "observation probe value changed from baseline while status still passed",
        );
    }
}

fn probe_status_change_severity(current_status: Option<&str>) -> DiagnosticComparisonSeverity {
    match current_status {
        Some("failed") => DiagnosticComparisonSeverity::Failure,
        Some("skipped") => DiagnosticComparisonSeverity::Warning,
        _ => DiagnosticComparisonSeverity::Info,
    }
}

fn compare_observation_checksums(
    baseline: &Value,
    current: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    compare_optional_value(
        baseline,
        current,
        &["ram", "signature"],
        "state",
        DiagnosticComparisonSeverity::Failure,
        "diagnostic RAM signature changed",
        differences,
    );
    for path in [
        &["cpu", "pc"][..],
        &["cpu", "a"][..],
        &["cpu", "x"][..],
        &["cpu", "y"][..],
        &["cpu", "sp"][..],
        &["cpu", "status"][..],
        &["cpu", "pending_cycles"][..],
        &["ram", "nmi_count"][..],
        &["ram", "checksum"][..],
    ] {
        compare_optional_value(
            baseline,
            current,
            path,
            "state",
            DiagnosticComparisonSeverity::Warning,
            "final diagnostic execution state changed from baseline",
            differences,
        );
    }
    for path in [
        &["cartridge", "rom_hash"][..],
        &["oam", "checksum"][..],
        &["frame", "checksum"][..],
        &["frame", "unique_colors"][..],
        &["audio", "sample_count"][..],
    ] {
        compare_optional_value(
            baseline,
            current,
            path,
            "observation",
            DiagnosticComparisonSeverity::Warning,
            "observable diagnostic artifact changed from baseline",
            differences,
        );
    }
}

fn compare_instruction_trace(
    baseline: &Value,
    current: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    for path in [
        &["instruction_trace", "captured_instruction_count"][..],
        &["instruction_trace", "retained_instruction_count"][..],
        &["instruction_trace", "retention_limit"][..],
        &["instruction_trace", "truncated"][..],
    ] {
        compare_optional_value(
            baseline,
            current,
            path,
            "trace",
            DiagnosticComparisonSeverity::Warning,
            "instruction trace telemetry changed from baseline",
            differences,
        );
    }
}

fn compare_timeline(
    baseline: &Value,
    current: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    let baseline_timeline = timeline_by_id(baseline);
    let current_timeline = timeline_by_id(current);

    for (test_id, current_test) in &current_timeline {
        let path = format!("timeline[{test_id}]");
        let Some(baseline_test) = baseline_timeline.get(test_id) else {
            push_difference(
                differences,
                DiagnosticComparisonSeverity::Info,
                "timeline",
                &path,
                None,
                Some("present".to_string()),
                "current run includes a test that was not present in baseline timeline",
            );
            continue;
        };

        let baseline_outcome = json_string(baseline_test, &["outcome"]);
        let current_outcome = json_string(current_test, &["outcome"]);
        if baseline_outcome != current_outcome {
            push_difference(
                differences,
                if current_outcome.as_deref() == Some("passed") {
                    DiagnosticComparisonSeverity::Info
                } else {
                    DiagnosticComparisonSeverity::Failure
                },
                "timeline",
                &format!("{path}.outcome"),
                baseline_outcome,
                current_outcome.clone(),
                "per-test outcome changed from baseline",
            );
        }

        compare_test_duration(*test_id, baseline_test, current_test, differences);
    }

    for test_id in baseline_timeline.keys() {
        if !current_timeline.contains_key(test_id) {
            push_difference(
                differences,
                DiagnosticComparisonSeverity::Failure,
                "timeline",
                &format!("timeline[{test_id}]"),
                Some("present".to_string()),
                None,
                "baseline test is missing from current timeline",
            );
        }
    }
}

fn compare_test_duration(
    test_id: u64,
    baseline_test: &Value,
    current_test: &Value,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    let baseline_duration = json_u64(baseline_test, &["duration_cycles"]);
    let current_duration = json_u64(current_test, &["duration_cycles"]);
    match (baseline_duration, current_duration) {
        (Some(baseline_duration), Some(current_duration)) => {
            let tolerance = 1_000u64.max(baseline_duration / 4);
            if current_duration > baseline_duration.saturating_add(tolerance) {
                push_difference(
                    differences,
                    DiagnosticComparisonSeverity::Warning,
                    "timing",
                    &format!("timeline[{test_id}].duration_cycles"),
                    Some(baseline_duration.to_string()),
                    Some(current_duration.to_string()),
                    "test duration exceeded baseline by more than 25 percent or 1000 cycles",
                );
            }
        }
        (Some(baseline_duration), None) => push_difference(
            differences,
            DiagnosticComparisonSeverity::Warning,
            "timing",
            &format!("timeline[{test_id}].duration_cycles"),
            Some(baseline_duration.to_string()),
            None,
            "baseline had a duration but current run did not",
        ),
        _ => {}
    }
}

fn compare_optional_value(
    baseline: &Value,
    current: &Value,
    path: &[&str],
    category: &'static str,
    severity: DiagnosticComparisonSeverity,
    note: &'static str,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    let baseline_value = json_path_display(baseline, path);
    let current_value = json_path_display(current, path);
    if baseline_value != current_value {
        push_difference(
            differences,
            severity,
            category,
            &path.join("."),
            baseline_value,
            current_value,
            note,
        );
    }
}

struct U64RegressionComparison<'a> {
    path: &'a [&'a str],
    category: &'static str,
    lower_is_regression: bool,
    regression_note: &'static str,
    improvement_note: &'static str,
}

fn compare_u64_regression(
    baseline: &Value,
    current: &Value,
    comparison: U64RegressionComparison<'_>,
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
) {
    let baseline_value = json_u64(baseline, comparison.path);
    let current_value = json_u64(current, comparison.path);
    if baseline_value == current_value {
        return;
    }

    let severity = match (baseline_value, current_value) {
        (Some(baseline_value), Some(current_value))
            if comparison.lower_is_regression && current_value < baseline_value =>
        {
            DiagnosticComparisonSeverity::Failure
        }
        (Some(baseline_value), Some(current_value))
            if !comparison.lower_is_regression && current_value > baseline_value =>
        {
            DiagnosticComparisonSeverity::Failure
        }
        _ => DiagnosticComparisonSeverity::Info,
    };
    let note = match severity {
        DiagnosticComparisonSeverity::Failure => comparison.regression_note,
        _ => comparison.improvement_note,
    };

    push_difference(
        differences,
        severity,
        comparison.category,
        &comparison.path.join("."),
        baseline_value.map(|value| value.to_string()),
        current_value.map(|value| value.to_string()),
        note,
    );
}

fn push_difference(
    differences: &mut Vec<DiagnosticComparisonDifferenceTelemetry>,
    severity: DiagnosticComparisonSeverity,
    category: &'static str,
    path: &str,
    baseline: Option<String>,
    current: Option<String>,
    note: &'static str,
) {
    differences.push(DiagnosticComparisonDifferenceTelemetry {
        severity,
        category,
        path: path.to_string(),
        baseline,
        current,
        note: note.to_string(),
    });
}

fn comparison_summary(
    passed: bool,
    failure_count: usize,
    warning_count: usize,
    info_count: usize,
) -> String {
    if passed && warning_count == 0 && info_count == 0 {
        return "diagnostic comparison passed: current run matches baseline".to_string();
    }
    if passed {
        return format!(
            "diagnostic comparison passed with {warning_count} warning(s) and {info_count} informational difference(s)"
        );
    }
    format!(
        "diagnostic comparison failed: {failure_count} regression(s), {warning_count} warning(s), {info_count} informational difference(s)"
    )
}

fn timeline_by_id(value: &Value) -> HashMap<u64, &Value> {
    let mut timeline = HashMap::new();
    if let Some(entries) = json_at(value, &["timeline"]).and_then(Value::as_array) {
        for entry in entries {
            if let Some(test_id) = json_u64(entry, &["test_id"]) {
                timeline.insert(test_id, entry);
            }
        }
    }
    timeline
}

fn probe_by_id(value: &Value) -> HashMap<String, &Value> {
    let mut probes = HashMap::new();
    if let Some(entries) = json_at(value, &["probes"]).and_then(Value::as_array) {
        for entry in entries {
            if let Some(probe_id) = json_string(entry, &["id"]) {
                probes.insert(probe_id, entry);
            }
        }
    }
    probes
}

fn json_path_display(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path).and_then(display_json_value)
}

fn display_json_value(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) => Some(value.clone()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        _ => Some(value.to_string()),
    }
}

fn json_bool(value: &Value, path: &[&str]) -> Option<bool> {
    json_at(value, path).and_then(Value::as_bool)
}

fn json_u64(value: &Value, path: &[&str]) -> Option<u64> {
    json_at(value, path).and_then(Value::as_u64)
}

fn json_string(value: &Value, path: &[&str]) -> Option<String> {
    json_at(value, path)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn json_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    Some(current)
}

fn diagnostic_subsystem_label(subsystem: DiagnosticSubsystem) -> &'static str {
    match subsystem {
        DiagnosticSubsystem::Cpu => "cpu",
        DiagnosticSubsystem::Bus => "bus",
        DiagnosticSubsystem::Ppu => "ppu",
        DiagnosticSubsystem::Apu => "apu",
        DiagnosticSubsystem::Dma => "dma",
        DiagnosticSubsystem::Cartridge => "cartridge",
        DiagnosticSubsystem::Joypad => "joypad",
    }
}

fn diagnostic_test_tier_label(tier: DiagnosticTestTier) -> &'static str {
    match tier {
        DiagnosticTestTier::Smoke => "smoke",
        DiagnosticTestTier::EdgeCase => "edge_case",
        DiagnosticTestTier::Integration => "integration",
    }
}

fn diagnostic_health_label(health: DiagnosticHealth) -> &'static str {
    match health {
        DiagnosticHealth::Healthy => "healthy",
        DiagnosticHealth::CartridgeAssertionFailed => "cartridge_assertion_failed",
        DiagnosticHealth::TimedOut => "timed_out",
        DiagnosticHealth::HostValidationFailed => "host_validation_failed",
    }
}

fn diagnostic_failure_kind_label(kind: DiagnosticFailureKind) -> &'static str {
    match kind {
        DiagnosticFailureKind::CartridgeAssertion => "cartridge_assertion",
        DiagnosticFailureKind::Timeout => "timeout",
        DiagnosticFailureKind::HostValidation => "host_validation",
    }
}

fn test_timeline_outcome_label(outcome: TestTimelineOutcome) -> &'static str {
    match outcome {
        TestTimelineOutcome::NotStarted => "not_started",
        TestTimelineOutcome::Passed => "passed",
        TestTimelineOutcome::Failed => "failed",
        TestTimelineOutcome::TimedOut => "timed_out",
        TestTimelineOutcome::Incomplete => "incomplete",
    }
}

fn test_timeline_end_reason_label(reason: TestTimelineEndReason) -> &'static str {
    match reason {
        TestTimelineEndReason::NextTestStarted => "next_test_started",
        TestTimelineEndReason::CartridgePassed => "cartridge_passed",
        TestTimelineEndReason::CartridgeFailed => "cartridge_failed",
        TestTimelineEndReason::Timeout => "timeout",
    }
}

fn diagnostic_probe_source_label(source: DiagnosticProbeSource) -> &'static str {
    match source {
        DiagnosticProbeSource::CartridgeResult => "cartridge_result",
        DiagnosticProbeSource::HostObservation => "host_observation",
    }
}

fn diagnostic_probe_status_label(status: DiagnosticProbeStatus) -> &'static str {
    match status {
        DiagnosticProbeStatus::Passed => "passed",
        DiagnosticProbeStatus::Failed => "failed",
        DiagnosticProbeStatus::Skipped => "skipped",
    }
}

fn diagnostic_event_kind_label(kind: DiagnosticEventKind) -> &'static str {
    match kind {
        DiagnosticEventKind::Reset => "reset",
        DiagnosticEventKind::TestChanged => "test_changed",
        DiagnosticEventKind::StatusChanged => "status_changed",
        DiagnosticEventKind::OamDmaStarted => "oam_dma_started",
        DiagnosticEventKind::OamDmaCompleted => "oam_dma_completed",
        DiagnosticEventKind::DmcDmaFetched => "dmc_dma_fetched",
        DiagnosticEventKind::DmcDmaOamOverlap => "dmc_dma_oam_overlap",
        DiagnosticEventKind::FrameComplete => "frame_complete",
        DiagnosticEventKind::PostPassFrameComplete => "post_pass_frame_complete",
    }
}

fn diagnostic_comparison_severity_label(severity: DiagnosticComparisonSeverity) -> &'static str {
    match severity {
        DiagnosticComparisonSeverity::Failure => "failure",
        DiagnosticComparisonSeverity::Warning => "warning",
        DiagnosticComparisonSeverity::Info => "info",
    }
}

fn optional_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn optional_u8(value: Option<u8>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn optional_pc(value: Option<u16>) -> String {
    value.map(format_pc).unwrap_or_else(|| "none".to_string())
}

fn format_debug_event_focus(event: Option<&DiagnosticDebugEventFocusTelemetry>) -> String {
    let Some(event) = event else {
        return "none".to_string();
    };

    format!(
        "{} at cycle {} pc {} status {} test {} ({}) note {}",
        diagnostic_event_kind_label(event.kind),
        event.cycle,
        event.pc_hex,
        event.status_hex,
        event.current_test,
        event.current_test_name.unwrap_or("unknown_test"),
        event.note
    )
}

fn format_debug_instruction_focus(
    instruction: Option<&DiagnosticDebugInstructionFocusTelemetry>,
) -> String {
    let Some(instruction) = instruction else {
        return "none".to_string();
    };

    format!(
        "seq {} cycle {} pc {} {} symbol {} status {} failure {}",
        instruction.sequence,
        instruction.cycle,
        instruction.pc_hex,
        instruction
            .instruction
            .as_deref()
            .unwrap_or("unknown_instruction"),
        instruction.symbol.as_deref().unwrap_or("none"),
        instruction.status_hex,
        instruction.failure_code_hex
    )
}

fn format_symbol(symbol: &DiagnosticSymbolTelemetry) -> String {
    if symbol.offset == 0 {
        symbol.name.clone()
    } else {
        format!("{}+{}", symbol.name, symbol.offset_hex)
    }
}

fn format_pc(value: u16) -> String {
    format!("0x{value:04X}")
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
        assert_eq!(info.mapper, DIAGNOSTIC_MAPPER);
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
    fn diagnostic_coverage_gaps_are_unique_and_actionable() {
        let mut ids = BTreeSet::new();

        for gap in DIAGNOSTIC_COVERAGE_GAPS {
            assert!(ids.insert(gap.id), "duplicate coverage gap {}", gap.id);
            assert!(!gap.subsystem.is_empty());
            assert!(!gap.risk.is_empty());
            assert!(!gap.current_coverage.is_empty());
            assert!(!gap.missing_coverage.is_empty());
            assert!(!gap.suggested_next_test.is_empty());
        }

        assert!(
            DIAGNOSTIC_COVERAGE_GAPS.len() >= 5,
            "diagnostic suite should declare major known coverage gaps"
        );
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
        assert_eq!(telemetry.analysis.health, DiagnosticHealth::Healthy);
        assert_eq!(
            telemetry.analysis.coverage.total_tests,
            DIAGNOSTIC_TESTS.len()
        );
        assert_eq!(
            telemetry.analysis.coverage.passed_tests,
            DIAGNOSTIC_TESTS.len()
        );
        assert_eq!(telemetry.analysis.coverage.failed_tests, 0);
        assert_eq!(
            telemetry.analysis.coverage_gaps.len(),
            DIAGNOSTIC_COVERAGE_GAPS.len()
        );
        assert!(telemetry
            .analysis
            .coverage_gaps
            .iter()
            .any(|gap| gap.id == "ppu_pixel_pipeline" && gap.missing_coverage.contains("Sprite")));
        assert!(telemetry.analysis.summary.contains("diagnostic passed"));
        assert!(!telemetry.analysis.next_actions.is_empty());
        assert_eq!(
            telemetry.analysis.timing.started_tests,
            DIAGNOSTIC_TESTS.len()
        );
        assert_eq!(telemetry.analysis.timing.not_started_tests, 0);
        assert!(telemetry.analysis.timing.slowest_test.is_some());
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
        assert!(telemetry
            .timeline
            .iter()
            .all(|test| test.outcome == TestTimelineOutcome::Passed));
        assert!(telemetry.events.iter().any(|event| {
            matches!(event.kind, DiagnosticEventKind::TestChanged)
                && event.current_test_name == Some("cpu_branch_page_crossing")
        }));
    }
}
