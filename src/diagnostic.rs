use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use serde::Serialize;
use serde_json::Value;

use crate::bus::{Bus, DmcDmaService};
use crate::cartridge::Cartridge;
use crate::cpu::Cpu;
use crate::joypad::JoypadButton;
use crate::ppu::PpuTimingState;

pub const DIAGNOSTIC_PROVENANCE: &str =
    "Generated OxideNES diagnostic iNES cartridge: synthetic 6502 program and CHR patterns only, no ROM content.";
pub const DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION: u16 = 62;
pub const DIAGNOSTIC_SUITE_NAME: &str = "oxidenes_headless_diagnostic_cartridge";
pub const DIAGNOSTIC_SUITE_VERSION: &str = "diagnostic-cartridge-v62";

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
const CHR_BANK_SIZE: usize = 8 * 1024;
const PRG_SIZE: usize = PRG_BANKS as usize * PRG_BANK_SIZE;
const CHR_SIZE: usize = CHR_BANKS as usize * CHR_BANK_SIZE;
const MAPPER2_SWITCHABLE_ADDR: u16 = 0x8000;
const MAPPER2_FIXED_SENTINEL_ADDR: u16 = 0xFF00;
const MAPPER2_BANK_SENTINELS: &[(u8, u8)] = &[(0, 0xA0), (1, 0xB1), (2, 0xC2)];
const MAPPER2_FIXED_SENTINEL: u8 = 0xD3;
const MAPPER2_PRG_RAM_LOW_ADDR: u16 = 0x6000;
const MAPPER2_PRG_RAM_HIGH_ADDR: u16 = 0x7FFF;
const MAPPER2_PRG_RAM_LOW_SENTINEL: u8 = 0x5C;
const MAPPER2_PRG_RAM_HIGH_SENTINEL: u8 = 0xA7;
const MAPPER3_CHR_BANK_TEST_ID: u8 = 29;
const INPUT_MASK_SWEEP_TEST_ID: u8 = 30;
const MAPPER7_AXROM_TEST_ID: u8 = 31;
const MAPPER1_MMC1_TEST_ID: u8 = 32;
const MAPPER4_MMC3_TEST_ID: u8 = 33;
const MAPPER4_MMC3_EDGE_TEST_ID: u8 = 34;
const MAPPER1_MMC1_32K_PRG_TEST_ID: u8 = 35;
const MAPPER4_MMC3_PRG_RAM_TEST_ID: u8 = 36;
const CPU_RMW_MATRIX_TEST_ID: u8 = 37;
const CPU_RMW_ADDRESSING_MATRIX_TEST_ID: u8 = 38;
const CPU_BRANCH_MATRIX_TEST_ID: u8 = 39;
const CPU_STACK_MATRIX_TEST_ID: u8 = 40;
const MAPPER1_MAPPER: u8 = 1;
const MAPPER1_PRG_BANKS: u8 = 4;
const MAPPER1_CHR_8K_BANKS: u8 = 2;
const MAPPER1_CHR_4K_BANKS: usize = 4;
const MAPPER1_PRG_SWITCH_ADDR: u16 = 0x8000;
const MAPPER1_PRG_FIXED_ADDR: u16 = 0xFFE0;
const MAPPER1_CHR_LOW_READ_ADDR: u16 = 0x0010;
const MAPPER1_CHR_HIGH_READ_ADDR: u16 = 0x1010;
const MAPPER1_EXPECTED_CASE_COUNT: u8 = 12;
const MAPPER1_PRG_BANK_WRITES: [u8; 3] = [0x00, 0x02, 0x01];
const MAPPER1_PRG_EXPECTED_VALUES: [u8; 5] = [0xA0, 0xA0, 0xC2, 0xB1, 0xD3];
const MAPPER1_PRG_BANK_SENTINELS: [u8; 3] = [0xA0, 0xB1, 0xC2];
const MAPPER1_CHR_BANK_WRITES: [u8; 4] = [0x02, 0x03, 0x00, 0x01];
const MAPPER1_CHR_BANK_SENTINELS: [u8; 4] = [0x51, 0x62, 0x73, 0x84];
const MAPPER1_CHR_EXPECTED_VALUES: [u8; 4] = [0x73, 0x84, 0x51, 0x62];
const MAPPER1_MIRROR_EXPECTED_VALUES: [u8; 3] = [0x5A, 0xA5, 0x5A];
const MAPPER1_32K_LOW_READ_ADDR: u16 = 0x8000;
const MAPPER1_32K_HIGH_READ_ADDR: u16 = 0xE000;
const MAPPER1_32K_EXPECTED_CASE_COUNT: u8 = 10;
const MAPPER1_32K_CONTROL_WRITES: [u8; 2] = [0x00, 0x04];
const MAPPER1_32K_PRG_BANK_WRITES: [u8; 5] = [0x00, 0x01, 0x02, 0x03, 0x03];
const MAPPER1_32K_BANK_SENTINELS: [u8; 4] = [0xA0, 0xB1, 0xC2, 0xD3];
const MAPPER1_32K_EXPECTED_VALUES: [u8; 10] =
    [0xA0, 0xB1, 0xA0, 0xB1, 0xC2, 0xD3, 0xC2, 0xD3, 0xC2, 0xD3];
const MAPPER3_MAPPER: u8 = 3;
const MAPPER3_PRG_BANKS: u8 = 2;
const MAPPER3_CHR_BANKS: u8 = 4;
const MAPPER3_CHR_READ_ADDR: u16 = 0x0010;
const MAPPER3_CHR_BANK_EXPECTED_CASE_COUNT: u8 = 4;
const MAPPER3_CHR_BANK_EXPECTED_BANKS: [u8; 4] = [0, 1, 2, 3];
const MAPPER3_CHR_BANK_EXPECTED_VALUES: [u8; 4] = [0x11, 0x22, 0x33, 0x44];
const MAPPER4_MAPPER: u8 = 4;
const MAPPER4_PRG_16K_BANKS: u8 = 4;
const MAPPER4_PRG_8K_BANKS: usize = 8;
const MAPPER4_CHR_8K_BANKS: u8 = 1;
const MAPPER4_CHR_1K_BANKS: usize = 8;
const MAPPER4_PRG_R6_READ_ADDR: u16 = 0x8000;
const MAPPER4_PRG_R7_READ_ADDR: u16 = 0xA000;
const MAPPER4_PRG_FIXED_READ_ADDR: u16 = 0xE100;
const MAPPER4_CHR_READ_ADDRS: [u16; 5] = [0x0010, 0x0410, 0x0810, 0x1010, 0x1410];
const MAPPER4_EXPECTED_CASE_COUNT: u8 = 11;
const MAPPER4_PRG_REGISTER_WRITES: [(u8, u8); 2] = [(0x06, 0x02), (0x07, 0x03)];
const MAPPER4_PRG_EXPECTED_VALUES: [u8; 3] = [0xC2, 0xD3, 0xF7];
const MAPPER4_PRG_BANK_SENTINELS: [(usize, u16, u8); 3] = [
    (2, MAPPER4_PRG_R6_READ_ADDR, MAPPER4_PRG_EXPECTED_VALUES[0]),
    (3, MAPPER4_PRG_R7_READ_ADDR, MAPPER4_PRG_EXPECTED_VALUES[1]),
    (
        MAPPER4_PRG_8K_BANKS - 1,
        MAPPER4_PRG_FIXED_READ_ADDR,
        MAPPER4_PRG_EXPECTED_VALUES[2],
    ),
];
const MAPPER4_CHR_REGISTER_WRITES: [(u8, u8); 4] =
    [(0x00, 0x02), (0x01, 0x04), (0x02, 0x06), (0x03, 0x07)];
const MAPPER4_CHR_BANK_SENTINELS: [u8; 8] = [0x10, 0x21, 0x32, 0x43, 0x54, 0x65, 0x76, 0x87];
const MAPPER4_CHR_EXPECTED_VALUES: [u8; 5] = [0x32, 0x43, 0x54, 0x76, 0x87];
const MAPPER4_MIRROR_EXPECTED_VALUES: [u8; 2] = [0x5A, 0xA5];
const MAPPER4_IRQ_LATCH: u8 = 0x02;
const MAPPER4_EXPECTED_IRQ_COUNT: u8 = 1;
const MAPPER4_EDGE_PROGRAM_BASE: u16 = 0xE000;
const MAPPER4_EDGE_PRG_READ_ADDRS: [u16; 3] = [0x8000, 0xA000, 0xC000];
const MAPPER4_EDGE_CHR_READ_ADDRS: [u16; 8] = [
    0x0010, 0x0410, 0x0810, 0x0C10, 0x1010, 0x1410, 0x1810, 0x1C10,
];
const MAPPER4_EDGE_EXPECTED_CASE_COUNT: u8 = 13;
const MAPPER4_EDGE_PRG_SELECT_WRITES: [(u8, u8); 2] = [(0x46, 0x02), (0x47, 0x03)];
const MAPPER4_EDGE_CHR_SELECT_WRITES: [(u8, u8); 6] = [
    (0x80, 0x02),
    (0x81, 0x04),
    (0x82, 0x06),
    (0x83, 0x07),
    (0x84, 0x00),
    (0x85, 0x01),
];
const MAPPER4_EDGE_PRG_EXPECTED_VALUES: [u8; 3] = [0xE6, 0xD3, 0xC2];
const MAPPER4_EDGE_PRG_BANK_SENTINELS: [(usize, u16, u8); 3] = [
    (
        MAPPER4_PRG_8K_BANKS - 2,
        MAPPER4_EDGE_PRG_READ_ADDRS[0],
        MAPPER4_EDGE_PRG_EXPECTED_VALUES[0],
    ),
    (
        3,
        MAPPER4_EDGE_PRG_READ_ADDRS[1],
        MAPPER4_EDGE_PRG_EXPECTED_VALUES[1],
    ),
    (
        2,
        MAPPER4_EDGE_PRG_READ_ADDRS[2],
        MAPPER4_EDGE_PRG_EXPECTED_VALUES[2],
    ),
];
const MAPPER4_EDGE_CHR_EXPECTED_VALUES: [u8; 8] = [0x76, 0x87, 0x10, 0x21, 0x32, 0x43, 0x54, 0x65];
const MAPPER4_EDGE_IRQ_LATCHES: [u8; 2] = [0x03, 0x00];
const MAPPER4_EDGE_EXPECTED_IRQ_COUNTS: [u8; 2] = [0x01, 0x02];
const MAPPER4_PRG_RAM_SIZE: usize = 0x2000;
const MAPPER4_PRG_RAM_READ_ADDRS: [u16; 4] = [0x6000, 0x67FF, 0x7FFF, 0x6000];
const MAPPER4_PRG_RAM_EXPECTED_VALUES: [u8; 4] = [0x5A, 0xC3, 0xA7, 0x3C];
const MAPPER4_PRG_RAM_RESTORED_ADDRS: [u16; 3] = [0x6000, 0x67FF, 0x7FFF];
const MAPPER4_PRG_RAM_RESTORED_VALUES: [u8; 3] = [0x3C, 0xC3, 0xA7];
const MAPPER4_PRG_RAM_EXPECTED_CASE_COUNT: u8 = 4;
const MAPPER7_MAPPER: u8 = 7;
const MAPPER7_PRG_BANKS: u8 = 8;
const MAPPER7_CHR_BANKS: u8 = 0;
const MAPPER7_32K_BANKS: usize = 4;
const MAPPER7_PRG_SENTINEL_ADDR: u16 = 0x8000;
const MAPPER7_EXPECTED_CASE_COUNT: u8 = 7;
const MAPPER7_PRG_BANK_WRITES: [u8; 4] = [0x00, 0x01, 0x02, 0x03];
const MAPPER7_PRG_EXPECTED_VALUES: [u8; 4] = [0xA0, 0xB1, 0xC2, 0xD3];
const MAPPER7_MIRROR_EXPECTED_VALUES: [u8; 3] = [0x5A, 0xA5, 0x5A];
const INPUT_MASK_SWEEP_CASES: [(u8, u8); 16] = [
    (0x00, 0x00),
    (0xFF, 0xFF),
    (0xAA, 0x55),
    (0x55, 0xAA),
    (0x81, 0x28),
    (0x18, 0x42),
    (0x24, 0x81),
    (0xC3, 0x3C),
    (0x01, 0x00),
    (0x00, 0x80),
    (0x7E, 0xE7),
    (0x99, 0x66),
    (0x10, 0x08),
    (0xEF, 0xF7),
    (0xA5, 0x5A),
    (0x3C, 0xC3),
];
const INPUT_MASK_SWEEP_EXPECTED_CASE_COUNT: u8 = INPUT_MASK_SWEEP_CASES.len() as u8;

const STATUS_ADDR: u8 = 0xF0;
const CURRENT_TEST_ADDR: u8 = 0xF1;
const FAILURE_CODE_ADDR: u8 = 0xF2;
const SIGNATURE_ADDR: u8 = 0xF3;
const NMI_COUNT_ADDR: u8 = 0xF4;
const JOYPAD1_EXPECTED_MASK_ADDR: u8 = 0xF5;
const JOYPAD2_EXPECTED_MASK_ADDR: u8 = 0xF6;

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
const DMC_DMA_OAM_OVERLAP_EXPECTED_MIN_POSITION_BUCKETS: usize = 3;
const DMC_DMA_OAM_OVERLAP_EXPECTED_MIN_PHASE_MATRIX_TRANSFERS: usize = 3;
const DMC_DMA_OAM_OVERLAP_POSITION_BUCKETS: [&str; 3] = ["beginning", "middle", "end"];
const DMC_DMA_OAM_MIDDLE_TRANSFER_ALIGNMENT_NOPS: usize = 80;
const INSTRUCTION_TRACE_TAIL_LIMIT: usize = 64;
const PPU_VBLANK_TIMING_TEST_ID: u8 = 10;
const PPU_VBLANK_FIRST_NMI_MIN_CYCLES: u64 = 1;
const PPU_VBLANK_FIRST_NMI_MAX_CYCLES: u64 = 30_000;
const PPU_VBLANK_INTER_NMI_MIN_CYCLES: u64 = 29_700;
const PPU_VBLANK_INTER_NMI_MAX_CYCLES: u64 = 29_900;
const PPU_VBLANK_EDGE_SET_SCANLINE: i16 = 241;
const PPU_VBLANK_EDGE_SET_DOT: u16 = 1;
const PPU_VBLANK_EDGE_CLEAR_SCANLINE: i16 = -1;
const PPU_VBLANK_EDGE_CLEAR_DOT: u16 = 1;
const PPU_VBLANK_EDGE_EXPECTED_SET_COUNT: u8 = 2;
const PPU_VBLANK_EDGE_EXPECTED_CLEAR_COUNT: u8 = 1;
pub const DIAGNOSTIC_RENDER_FRAME_EXPECTED_CHECKSUM: u64 = 0x00BDFEF60DAB16A5;
pub const DIAGNOSTIC_RENDER_FRAME_EXPECTED_UNIQUE_COLORS: usize = 3;
pub const DIAGNOSTIC_RENDER_FRAME_EXPECTED_NONZERO_PIXELS: usize = 256 * 240;
const DIAGNOSTIC_RENDER_FRAME_SIGNATURE_ENABLED_REASON: &str =
    "enabled: canonical default diagnostic fixture";
const DIAGNOSTIC_RENDER_FRAME_SIGNATURE_INPUT_REASON: &str =
    "disabled: non-default input timing fixture";
const DIAGNOSTIC_RENDER_FRAME_SIGNATURE_FAULT_REASON: &str =
    "disabled: intentional fault-injection fixture";
const APU_AUDIO_EXPECTED_MIN_SAMPLE_COUNT: usize = 12_000;
const APU_AUDIO_EXPECTED_MAX_SAMPLE_COUNT: usize = 13_000;
const APU_AUDIO_EXPECTED_MIN_PEAK_ABS: f32 = 0.05;
const APU_AUDIO_EXPECTED_MAX_PEAK_ABS: f32 = 0.20;
const APU_AUDIO_EXPECTED_MIN_RMS_ABS: f32 = 0.005;
const APU_AUDIO_EXPECTED_MAX_RMS_ABS: f32 = 0.10;
const APU_AUDIO_EXPECTED_MIN_MEAN_ABS: f32 = 0.001;
const APU_AUDIO_EXPECTED_MAX_MEAN_ABS: f32 = 0.10;
const APU_STATUS_MATRIX_EXPECTED_MASK: u8 = 0x0F;
const APU_STATUS_MATRIX_EXPECTED_CASE_COUNT: u8 = 4;
const APU_STATUS_MATRIX_OBSERVED_MASK_ADDR: u16 = 0x0262;
const APU_STATUS_MATRIX_CASE_COUNT_ADDR: u16 = 0x0263;
const APU_DMC_STATUS_EXPECTED_BIT: u8 = 0x10;
const APU_DMC_STATUS_EXPECTED_CASE_COUNT: u8 = 1;
const APU_DMC_STATUS_OBSERVED_BIT_ADDR: u16 = 0x0264;
const APU_DMC_STATUS_CASE_COUNT_ADDR: u16 = 0x0265;
const MAPPER3_CHR_BANK_CASE_COUNT_ADDR: u16 = 0x0266;
const MAPPER3_CHR_BANK_OBSERVED_BASE_ADDR: u16 = 0x0267;
const INPUT_MASK_SWEEP_JOYPAD1_OBSERVED_ADDR: u16 = 0x026B;
const INPUT_MASK_SWEEP_JOYPAD2_OBSERVED_ADDR: u16 = 0x026C;
const INPUT_MASK_SWEEP_CASE_COUNT_ADDR: u16 = 0x026D;
const MAPPER7_AXROM_CASE_COUNT_ADDR: u16 = 0x026E;
const MAPPER7_AXROM_PRG_OBSERVED_BASE_ADDR: u16 = 0x026F;
const MAPPER7_AXROM_MIRROR_OBSERVED_BASE_ADDR: u16 = 0x0273;
const MAPPER1_MMC1_CASE_COUNT_ADDR: u16 = 0x0276;
const MAPPER1_MMC1_PRG_OBSERVED_BASE_ADDR: u16 = 0x0277;
const MAPPER1_MMC1_CHR_OBSERVED_BASE_ADDR: u16 = 0x027C;
const MAPPER1_MMC1_MIRROR_OBSERVED_BASE_ADDR: u16 = 0x0280;
const MAPPER4_MMC3_CASE_COUNT_ADDR: u16 = 0x0283;
const MAPPER4_MMC3_PRG_OBSERVED_BASE_ADDR: u16 = 0x0284;
const MAPPER4_MMC3_CHR_OBSERVED_BASE_ADDR: u16 = 0x0287;
const MAPPER4_MMC3_MIRROR_OBSERVED_BASE_ADDR: u16 = 0x028C;
const MAPPER4_MMC3_IRQ_COUNT_ADDR: u16 = 0x028E;
const MAPPER4_MMC3_IRQ_OBSERVED_ADDR: u16 = 0x028F;
const MAPPER4_MMC3_EDGE_CASE_COUNT_ADDR: u16 = 0x0290;
const MAPPER4_MMC3_EDGE_PRG_OBSERVED_BASE_ADDR: u16 = 0x0291;
const MAPPER4_MMC3_EDGE_CHR_OBSERVED_BASE_ADDR: u16 = 0x0294;
const MAPPER4_MMC3_EDGE_IRQ_COUNT_ADDR: u16 = 0x029C;
const MAPPER4_MMC3_EDGE_IRQ_OBSERVED_BASE_ADDR: u16 = 0x029D;
const MAPPER1_MMC1_32K_PRG_CASE_COUNT_ADDR: u16 = 0x02A0;
const MAPPER1_MMC1_32K_PRG_OBSERVED_BASE_ADDR: u16 = 0x02A1;
const MAPPER4_MMC3_PRG_RAM_CASE_COUNT_ADDR: u16 = 0x02B0;
const MAPPER4_MMC3_PRG_RAM_OBSERVED_BASE_ADDR: u16 = 0x02B1;
// Keep the canonical render-frame signature phase stable after earlier tests grow.
const PPU_RENDER_FRAME_PHASE_ALIGNMENT_NOPS: usize = 31;
const APU_STATUS_FAULT_LABEL: &str = "apu_status_register_before_status_read";
const CPU_RAM_MIRRORING_FAULT_LABEL: &str = "cpu_ram_mirroring_before_first_mirror_read";
const CPU_ZERO_PAGE_WRAP_FAULT_LABEL: &str = "cpu_zero_page_index_wrap_before_read";
const CPU_INDIRECT_JMP_FAULT_LABEL: &str = "cpu_indirect_jmp_page_wrap_before_jump";
const DMA_OAM_TRANSFER_FAULT_LABEL: &str = "oam_dma_transfer_before_dma";
const JOYPAD_STROBE_HIGH_HOLD_FAULT_LABEL: &str = "joypad_strobe_high_hold_before_reads";
const JOYPAD_STROBE_RESET_FAULT_LABEL: &str = "joypad_strobe_reset_before_reset_read";
const MAPPER2_BANK_SWITCH_FAULT_LABEL: &str = "mapper2_prg_bank_switch_before_read";
const MAPPER2_PRG_RAM_FAULT_LABEL: &str = "mapper2_prg_ram_roundtrip_before_high_read";
const PPU_NAMETABLE_MIRRORING_FAULT_LABEL: &str =
    "ppu_horizontal_nametable_mirroring_before_first_mirror_read";
const PPU_NMI_TIMEOUT_FAULT_LABEL: &str = "ppu_nmi_render_frame_after_enable";
const PPU_READ_BUFFER_FAULT_LABEL: &str = "ppu_vram_read_buffer_before_first_read";
const PPU_SCROLL_SEAM_FAULT_LABEL: &str = "ppu_scroll_seam_before_render_enable";
const PPU_SPRITE_OVERFLOW_FAULT_LABEL: &str = "ppu_sprite_overflow_before_render_enable";
const PPU_SPRITE_PRIORITY_FAULT_LABEL: &str = "ppu_sprite_priority_before_render_enable";
const PPU_SPRITE_ZERO_HIT_FAULT_LABEL: &str = "ppu_sprite_zero_hit_before_render_enable";
const PPU_STATUS_LATCH_RESET_FAULT_LABEL: &str = "ppu_status_latch_reset_before_address_write";
const PPU_VRAM_INCREMENT_32_FAULT_LABEL: &str = "ppu_vram_increment_32_before_stride_read";
const CPU_ADDRESSING_MATRIX_FAULT_LABEL: &str = "cpu_addressing_matrix_before_page_cross_read";
const CPU_RMW_MATRIX_FAULT_LABEL: &str = "cpu_rmw_matrix_before_asl";
const CPU_RMW_ADDRESSING_MATRIX_FAULT_LABEL: &str = "cpu_rmw_addressing_matrix_before_absolute_asl";
const CPU_BRANCH_MATRIX_FAULT_LABEL: &str = "cpu_branch_condition_matrix_before_cases";
const CPU_STACK_MATRIX_FAULT_LABEL: &str = "cpu_stack_status_matrix_before_cases";
const INPUT_PORT_MATRIX_FAULT_LABEL: &str = "input_port_matrix_before_serial_reads";
const DMA_PHASE_MATRIX_FAULT_LABEL: &str = "oam_dma_phase_matrix_before_second_dma";

const CPU_ADDRESSING_MATRIX_ABS_X_NO_CROSS_ADDR: u16 = 0x0240;
const CPU_ADDRESSING_MATRIX_ABS_X_PAGE_CROSS_ADDR: u16 = 0x0241;
const CPU_ADDRESSING_MATRIX_INDIRECT_Y_PAGE_CROSS_ADDR: u16 = 0x0242;
const CPU_ADDRESSING_MATRIX_CASE_COUNT_ADDR: u16 = 0x0243;
const CPU_ADDRESSING_MATRIX_EXPECTED_CASE_COUNT: u8 = 3;
const INPUT_PORT_MATRIX_JOYPAD1_HIGH_FIRST_ADDR: u16 = 0x0244;
const INPUT_PORT_MATRIX_JOYPAD1_HIGH_SECOND_ADDR: u16 = 0x0245;
const INPUT_PORT_MATRIX_JOYPAD2_HIGH_FIRST_ADDR: u16 = 0x0246;
const INPUT_PORT_MATRIX_JOYPAD2_HIGH_SECOND_ADDR: u16 = 0x0247;
const INPUT_PORT_MATRIX_JOYPAD1_OVERREAD_FIRST_ADDR: u16 = 0x0248;
const INPUT_PORT_MATRIX_JOYPAD1_OVERREAD_SECOND_ADDR: u16 = 0x0249;
const INPUT_PORT_MATRIX_JOYPAD2_OVERREAD_FIRST_ADDR: u16 = 0x024A;
const INPUT_PORT_MATRIX_JOYPAD2_OVERREAD_SECOND_ADDR: u16 = 0x024B;
const INPUT_PORT_MATRIX_CASE_COUNT_ADDR: u16 = 0x024C;
const INPUT_PORT_MATRIX_EXPECTED_CASE_COUNT: u8 = 24;
const DMA_PHASE_MATRIX_CASE_COUNT_ADDR: u16 = 0x024D;
const DMA_PHASE_MATRIX_CONTROL_ADDR: u16 = 0x024E;
const DMA_PHASE_MATRIX_TEST_ID: u8 = 24;
const DMA_PHASE_MATRIX_EXPECTED_TEST_TRANSFERS: usize = 2;
const DMA_PHASE_MATRIX_EXPECTED_TOTAL_TRANSFERS: usize = 3;
const PPU_SPRITE_ZERO_HIT_STATUS_ADDR: u16 = 0x024F;
const PPU_SPRITE_ZERO_HIT_CASE_COUNT_ADDR: u16 = 0x0250;
const PPU_SPRITE_ZERO_HIT_EXPECTED_STATUS_BIT: u8 = 0x40;
const PPU_SPRITE_ZERO_HIT_EXPECTED_CASE_COUNT: u8 = 1;
const PPU_SPRITE_ZERO_HIT_TEST_ID: u8 = 25;
const PPU_SPRITE_OVERFLOW_STATUS_ADDR: u16 = 0x0251;
const PPU_SPRITE_OVERFLOW_CASE_COUNT_ADDR: u16 = 0x0252;
const PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT: u8 = 0x20;
const PPU_SPRITE_OVERFLOW_EXPECTED_CLEAR_STATUS_BIT: u8 = 0x00;
const PPU_SPRITE_OVERFLOW_EXPECTED_CASE_COUNT: u8 = 3;
const PPU_SPRITE_OVERFLOW_TEST_ID: u8 = 26;
const PPU_SPRITE_OVERFLOW_RESTORE_BYTES: usize = 256;
const PPU_SPRITE_OVERFLOW_FALSE_POSITIVE_STATUS_ADDR: u16 = 0x0260;
const PPU_SPRITE_OVERFLOW_FALSE_NEGATIVE_STATUS_ADDR: u16 = 0x0261;
const PPU_SPRITE_PRIORITY_TEST_ID: u8 = 27;
const PPU_SPRITE_PRIORITY_CASE_COUNT_ADDR: u16 = 0x0253;
const PPU_SPRITE_PRIORITY_EXPECTED_CASE_COUNT: u8 = 2;
const PPU_SPRITE_PRIORITY_FRONT_SAMPLE_X: usize = 18;
const PPU_SPRITE_PRIORITY_FRONT_SAMPLE_Y: usize = 18;
const PPU_SPRITE_PRIORITY_BEHIND_SAMPLE_X: usize = 42;
const PPU_SPRITE_PRIORITY_BEHIND_SAMPLE_Y: usize = 18;
const PPU_SPRITE_PRIORITY_EXPECTED_FRONT_COLOR: u32 = 0xB53120;
const PPU_SPRITE_PRIORITY_EXPECTED_BEHIND_COLOR: u32 = 0x64B0FF;
const PPU_SCROLL_SEAM_TEST_ID: u8 = 28;
const PPU_SCROLL_SEAM_CASE_COUNT_ADDR: u16 = 0x0254;
const PPU_SCROLL_SEAM_EXPECTED_CASE_COUNT: u8 = 6;
const PPU_SCROLL_SEAM_LEFT_SAMPLE_X: usize = 2;
const PPU_SCROLL_SEAM_LEFT_SAMPLE_Y: usize = 18;
const PPU_SCROLL_SEAM_RIGHT_SAMPLE_X: usize = 10;
const PPU_SCROLL_SEAM_RIGHT_SAMPLE_Y: usize = 18;
const PPU_SCROLL_SEAM_EXPECTED_LEFT_COLOR: u32 = 0x64B0FF;
const PPU_SCROLL_SEAM_EXPECTED_RIGHT_COLOR: u32 = 0xB53120;
const PPU_SCROLL_SEAM_COARSE_LEFT_SAMPLE_X: usize = 2;
const PPU_SCROLL_SEAM_COARSE_LEFT_SAMPLE_Y: usize = 18;
const PPU_SCROLL_SEAM_COARSE_RIGHT_SAMPLE_X: usize = 10;
const PPU_SCROLL_SEAM_COARSE_RIGHT_SAMPLE_Y: usize = 18;
const PPU_SCROLL_SEAM_EXPECTED_COARSE_LEFT_COLOR: u32 = 0xB53120;
const PPU_SCROLL_SEAM_EXPECTED_COARSE_RIGHT_COLOR: u32 = 0x64B0FF;
const PPU_SCROLL_SEAM_NAMETABLE_WRAP_CASE_COUNT: u8 = 8;
const PPU_SCROLL_SEAM_NAMETABLE_WRAP_SCROLL_X: u8 = 0xF8;
const PPU_SCROLL_SEAM_NAMETABLE_WRAP_SCROLL_Y: u8 = 0x00;
const PPU_SCROLL_SEAM_NAMETABLE_WRAP_LEFT_SAMPLE_X: usize = 2;
const PPU_SCROLL_SEAM_NAMETABLE_WRAP_LEFT_SAMPLE_Y: usize = 18;
const PPU_SCROLL_SEAM_NAMETABLE_WRAP_RIGHT_SAMPLE_X: usize = 10;
const PPU_SCROLL_SEAM_NAMETABLE_WRAP_RIGHT_SAMPLE_Y: usize = 18;
const PPU_SCROLL_SEAM_EXPECTED_NAMETABLE_WRAP_LEFT_COLOR: u32 = 0xB53120;
const PPU_SCROLL_SEAM_EXPECTED_NAMETABLE_WRAP_RIGHT_COLOR: u32 = 0x64B0FF;
const PPU_SCROLL_SEAM_TOP_SAMPLE_X: usize = 2;
const PPU_SCROLL_SEAM_TOP_SAMPLE_Y: usize = 12;
const PPU_SCROLL_SEAM_BOTTOM_SAMPLE_X: usize = 2;
const PPU_SCROLL_SEAM_BOTTOM_SAMPLE_Y: usize = 20;
const PPU_SCROLL_SEAM_EXPECTED_TOP_COLOR: u32 = 0x64B0FF;
const PPU_SCROLL_SEAM_EXPECTED_BOTTOM_COLOR: u32 = 0xB53120;
const CPU_RMW_MATRIX_ASL_RESULT_ADDR: u16 = 0x02B5;
const CPU_RMW_MATRIX_ROL_RESULT_ADDR: u16 = 0x02B6;
const CPU_RMW_MATRIX_LSR_RESULT_ADDR: u16 = 0x02B7;
const CPU_RMW_MATRIX_ROR_RESULT_ADDR: u16 = 0x02B8;
const CPU_RMW_MATRIX_INC_RESULT_ADDR: u16 = 0x02B9;
const CPU_RMW_MATRIX_DEC_RESULT_ADDR: u16 = 0x02BA;
const CPU_RMW_MATRIX_CASE_COUNT_ADDR: u16 = 0x02BB;
const CPU_RMW_MATRIX_EXPECTED_CASE_COUNT: u8 = 6;
const CPU_RMW_ADDRESSING_ASL_ABS_RESULT_ADDR: u16 = 0x02BC;
const CPU_RMW_ADDRESSING_ROL_ABS_X_RESULT_ADDR: u16 = 0x02BD;
const CPU_RMW_ADDRESSING_LSR_ABS_RESULT_ADDR: u16 = 0x02BE;
const CPU_RMW_ADDRESSING_ROR_ABS_X_RESULT_ADDR: u16 = 0x02BF;
const CPU_RMW_ADDRESSING_INC_ABS_RESULT_ADDR: u16 = 0x02C0;
const CPU_RMW_ADDRESSING_DEC_ABS_X_RESULT_ADDR: u16 = 0x02C1;
const CPU_RMW_ADDRESSING_CASE_COUNT_ADDR: u16 = 0x02C2;
const CPU_RMW_ADDRESSING_EXPECTED_CASE_COUNT: u8 = 6;
const CPU_BRANCH_MATRIX_TAKEN_MASK_ADDR: u16 = 0x02C3;
const CPU_BRANCH_MATRIX_NOT_TAKEN_MASK_ADDR: u16 = 0x02C4;
const CPU_BRANCH_MATRIX_PAGE_CROSS_RESULT_ADDR: u16 = 0x02C5;
const CPU_BRANCH_MATRIX_CASE_COUNT_ADDR: u16 = 0x02C6;
const CPU_BRANCH_MATRIX_EXPECTED_MASK: u8 = 0xFF;
const CPU_BRANCH_MATRIX_EXPECTED_PAGE_CROSS_RESULT: u8 = 0x5C;
const CPU_BRANCH_MATRIX_EXPECTED_CASE_COUNT: u8 = 17;
const CPU_STACK_MATRIX_TSX_RESULT_ADDR: u16 = 0x02C7;
const CPU_STACK_MATRIX_PULL_RESULT_ADDR: u16 = 0x02C8;
const CPU_STACK_MATRIX_STATUS_RESULT_ADDR: u16 = 0x02C9;
const CPU_STACK_MATRIX_JSR_RESULT_ADDR: u16 = 0x02CA;
const CPU_STACK_MATRIX_FINAL_SP_ADDR: u16 = 0x02CB;
const CPU_STACK_MATRIX_CASE_COUNT_ADDR: u16 = 0x02CC;
const CPU_STACK_MATRIX_EXPECTED_STACK_POINTER: u8 = 0xF0;
const CPU_STACK_MATRIX_EXPECTED_PULL_RESULT: u8 = 0xA6;
const CPU_STACK_MATRIX_EXPECTED_STATUS_RESULT: u8 = 0xA9;
const CPU_STACK_MATRIX_EXPECTED_JSR_RESULT: u8 = 0x77;
const CPU_STACK_MATRIX_EXPECTED_CASE_COUNT: u8 = 5;

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
        intent: "Verify non-DMC channel enables are reflected through the APU status register.",
        expected_observations: &["$4015 bits 0-3 remain set after pulse 1, pulse 2, triangle, and noise setup"],
    },
    DiagnosticTestSpec {
        id: 7,
        name: "joypad_strobe_shift",
        subsystem: DiagnosticSubsystem::Joypad,
        tier: DiagnosticTestTier::Smoke,
        intent: "Verify joypad strobe latches the configured joypad-1 expected mask in read order.",
        expected_observations: &["read sequence matches the configured joypad-1 expected mask"],
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
        intent: "Verify the shared strobe latches the configured independent joypad-2 expected mask through $4017.",
        expected_observations: &["player 2 read sequence matches the configured joypad-2 expected mask"],
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
            "first post-reset $4016 read returns the configured joypad-1 A bit again",
            "second post-reset $4016 read advances to the configured joypad-1 B bit",
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
    DiagnosticTestSpec {
        id: 20,
        name: "ppu_status_latch_reset",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify reading PPUSTATUS resets the shared PPUADDR/PPUSCROLL write latch before subsequent PPUADDR writes.",
        expected_observations: &[
            "a half-written PPUADDR latch is reset by reading $2002",
            "the next $2006 high/low pair writes PPUDATA to the intended $2100 address",
        ],
    },
    DiagnosticTestSpec {
        id: 21,
        name: "joypad_strobe_high_hold",
        subsystem: DiagnosticSubsystem::Joypad,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify reads while $4016 strobe is high repeatedly return the configured A-button bit and do not advance the serial index.",
        expected_observations: &[
            "two strobe-high reads both return the configured joypad-1 A bit",
            "the first post-strobe-low read still starts at the configured joypad-1 A bit",
        ],
    },
    DiagnosticTestSpec {
        id: 22,
        name: "cpu_addressing_mode_matrix",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify CPU load-addressing matrix cases that include absolute,X and (zero-page),Y page-crossing loads.",
        expected_observations: &[
            "LDA $0440,X with X=0x02 reads a non-crossing absolute,X sentinel",
            "LDA $04FF,X with X=0x01 reads the $0500 page-crossing sentinel",
            "LDA ($42),Y with pointer $04FF and Y=0x01 reads the same page-crossing sentinel",
        ],
    },
    DiagnosticTestSpec {
        id: 23,
        name: "input_port_serial_matrix",
        subsystem: DiagnosticSubsystem::Joypad,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify both input ports under strobe-high hold, serial shift, and post-eight-read overread behavior.",
        expected_observations: &[
            "$4016 and $4017 strobe-high reads repeatedly expose each port's configured A bit",
            "$4016 and $4017 serial reads match the configured expected masks for all eight buttons",
            "$4016 and $4017 reads after the eighth button return 1",
        ],
    },
    DiagnosticTestSpec {
        id: 24,
        name: "oam_dma_phase_matrix",
        subsystem: DiagnosticSubsystem::Dma,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Trigger paired OAM DMA transfers so host telemetry proves both odd and even start-phase cycle buckets, then schedule DMC playback across a repeated OAM DMA burst train with a middle-position overlap.",
        expected_observations: &[
            "multiple additional OAM DMA transfers are started by the diagnostic cartridge",
            "host telemetry observes both 513-cycle and 514-cycle OAM DMA buckets across odd/even start phases",
            "DMC/OAM overlap placement covers beginning, middle, and end buckets across the cartridge run",
            "DMC/OAM overlap is observed across at least three distinct phase-matrix OAM transfers",
        ],
    },
    DiagnosticTestSpec {
        id: 25,
        name: "ppu_sprite_zero_hit",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify deterministic sprite/background overlap sets PPUSTATUS sprite-zero-hit.",
        expected_observations: &[
            "solid background tile 2 and sprite 0 tile 2 overlap at a visible pixel",
            "PPUSTATUS bit 6 remains set after the next completed frame",
        ],
    },
    DiagnosticTestSpec {
        id: 26,
        name: "ppu_sprite_overflow",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify nine in-range sprites on one scanline set PPUSTATUS sprite-overflow.",
        expected_observations: &[
            "nine synthetic sprite entries share one visible scanline",
            "PPUSTATUS bit 5 remains set after sprite evaluation completes",
        ],
    },
    DiagnosticTestSpec {
        id: 27,
        name: "ppu_sprite_priority_mux",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify sprite/background priority selects sprite pixels in front and background pixels when the sprite priority bit sends a sprite behind background.",
        expected_observations: &[
            "front-priority sprite sample uses the sprite palette color over a non-transparent background pixel",
            "behind-priority sprite sample uses the background palette color over a non-transparent sprite pixel",
        ],
    },
    DiagnosticTestSpec {
        id: 28,
        name: "ppu_scroll_seam_matrix",
        subsystem: DiagnosticSubsystem::Ppu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify fine-X, coarse-X, coarse-X nametable-wrap, and vertical scrolling render expected pixels across deterministic background tile seams.",
        expected_observations: &[
            "left and right samples straddle the fine-X horizontal tile seam",
            "coarse-X samples prove an 8-pixel scroll shifts the viewport into the next background tile",
            "nametable-wrap samples prove a 248-pixel scroll crosses from nametable $2000 into vertical-mirrored nametable $2400",
            "top and bottom samples straddle a vertical scroll tile seam",
        ],
    },
    DiagnosticTestSpec {
        id: CPU_RMW_MATRIX_TEST_ID,
        name: "cpu_read_modify_write_matrix",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify zero-page read-modify-write CPU opcodes update memory through the CPU execution path.",
        expected_observations: &[
            "ASL and LSR write shifted zero-page memory results",
            "ROL and ROR consume carry-in while writing zero-page memory results",
            "INC and DEC wrap memory and expose zero/negative-result cases",
        ],
    },
    DiagnosticTestSpec {
        id: CPU_RMW_ADDRESSING_MATRIX_TEST_ID,
        name: "cpu_rmw_addressing_matrix",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify non-zero-page read-modify-write CPU opcodes update memory through absolute and absolute,X addressing paths.",
        expected_observations: &[
            "ASL, LSR, and INC write expected absolute-address memory results",
            "ROL, ROR, and DEC write expected absolute,X memory results",
            "Indexed RMW cases prove write-back targets use the indexed effective address",
        ],
    },
    DiagnosticTestSpec {
        id: CPU_BRANCH_MATRIX_TEST_ID,
        name: "cpu_branch_condition_matrix",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify all official conditional branch opcodes take and skip based on their status flag conditions, including a page-crossing relative target.",
        expected_observations: &[
            "BPL, BMI, BVC, BVS, BCC, BCS, BNE, and BEQ all reach their taken targets when their condition is true",
            "the same branch opcodes all fall through when their condition is false",
            "a taken relative branch placed at page low byte 0xFC reaches a target on the next CPU page",
        ],
    },
    DiagnosticTestSpec {
        id: CPU_STACK_MATRIX_TEST_ID,
        name: "cpu_stack_status_matrix",
        subsystem: DiagnosticSubsystem::Cpu,
        tier: DiagnosticTestTier::EdgeCase,
        intent: "Verify stack pointer, push/pop, status push/pull, and JSR/RTS stack behavior through observable CPU RAM sentinels.",
        expected_observations: &[
            "TXS followed by TSX preserves the selected stack pointer",
            "PHA/PLA restores a pushed accumulator byte and preserves the stack depth",
            "PHP/PLP restores zero/carry status flags after intervening flag mutations",
            "JSR/RTS returns with the expected accumulator sentinel and final stack pointer",
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
        expected: "$4016 read bit 0 matches configured joypad-1 mask bit 0",
        observed: "$4016 read bit 0 did not match configured joypad-1 mask bit 0",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad strobe latch behavior and button-bit mapping for A.",
    },
    DiagnosticFailureSpec {
        code: 0x71,
        test_id: 7,
        assertion: "Joypad serial read 1 returns the latched B button bit",
        expected: "$4016 read bit 0 matches configured joypad-1 mask bit 1",
        observed: "$4016 read bit 0 did not match configured joypad-1 mask bit 1",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift index advancement after the first read.",
    },
    DiagnosticFailureSpec {
        code: 0x72,
        test_id: 7,
        assertion: "Joypad serial read 2 returns the latched Select button bit",
        expected: "$4016 read bit 0 matches configured joypad-1 mask bit 2",
        observed: "$4016 read bit 0 did not match configured joypad-1 mask bit 2",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Select bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x73,
        test_id: 7,
        assertion: "Joypad serial read 3 returns the latched Start button bit",
        expected: "$4016 read bit 0 matches configured joypad-1 mask bit 3",
        observed: "$4016 read bit 0 did not match configured joypad-1 mask bit 3",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Start bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x74,
        test_id: 7,
        assertion: "Joypad serial read 4 returns the latched Up button bit",
        expected: "$4016 read bit 0 matches configured joypad-1 mask bit 4",
        observed: "$4016 read bit 0 did not match configured joypad-1 mask bit 4",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Up bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x75,
        test_id: 7,
        assertion: "Joypad serial read 5 returns the latched Down button bit",
        expected: "$4016 read bit 0 matches configured joypad-1 mask bit 5",
        observed: "$4016 read bit 0 did not match configured joypad-1 mask bit 5",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Down bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x76,
        test_id: 7,
        assertion: "Joypad serial read 6 returns the latched Left button bit",
        expected: "$4016 read bit 0 matches configured joypad-1 mask bit 6",
        observed: "$4016 read bit 0 did not match configured joypad-1 mask bit 6",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Left bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x77,
        test_id: 7,
        assertion: "Joypad serial read 7 returns the latched Right button bit",
        expected: "$4016 read bit 0 matches configured joypad-1 mask bit 7",
        observed: "$4016 read bit 0 did not match configured joypad-1 mask bit 7",
        likely_domain: "joypad.strobe_shift",
        remediation_hint: "Inspect joypad shift order and Right bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0x78,
        test_id: 18,
        assertion: "Joypad strobe reset returns the A button bit again",
        expected: "first $4016 read after a second strobe sequence matches configured joypad-1 mask bit 0",
        observed: "$4016 did not restart at the configured A button bit after strobe reset",
        likely_domain: "joypad.strobe_reset",
        remediation_hint: "Inspect joypad $4016 writes; a high strobe write must reset the serial read index before returning to low strobe mode.",
    },
    DiagnosticFailureSpec {
        code: 0x79,
        test_id: 18,
        assertion: "Joypad strobe reset resumes serial advancement after the A bit",
        expected: "second $4016 read after reset matches configured joypad-1 mask bit 1",
        observed: "$4016 did not advance from A to the configured B bit after the reset read",
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
        code: 0x7C,
        test_id: 20,
        assertion: "PPUSTATUS read resets the PPUADDR write latch before the next address pair",
        expected: "after reading $2002, $2006 writes 0x21 then 0x00 and $2007 stores a sentinel at $2100",
        observed: "$2100 did not contain the sentinel written after the PPUSTATUS latch reset",
        likely_domain: "ppu.registers.status_latch_reset",
        remediation_hint: "Inspect PPUSTATUS reads and the shared PPUADDR/PPUSCROLL write latch; reading $2002 must reset the latch to expect the high byte.",
    },
    DiagnosticFailureSpec {
        code: 0x7D,
        test_id: 21,
        assertion: "Joypad strobe-high read returns the configured A button bit",
        expected: "first $4016 read while strobe is high matches the configured expected A bit",
        observed: "$4016 strobe-high read did not match the configured A bit",
        likely_domain: "joypad.strobe_high_hold",
        remediation_hint: "Inspect joypad $4016 read behavior while strobe is high; reads should return the A button state without advancing the serial index.",
    },
    DiagnosticFailureSpec {
        code: 0x7E,
        test_id: 21,
        assertion: "Repeated joypad strobe-high reads keep returning the configured A button bit",
        expected: "second $4016 read while strobe is high still matches the configured expected A bit",
        observed: "$4016 strobe-high read advanced away from the A bit or returned the wrong value",
        likely_domain: "joypad.strobe_high_hold",
        remediation_hint: "Inspect joypad strobe-high handling; the serial read index must stay pinned while strobe remains high.",
    },
    DiagnosticFailureSpec {
        code: 0x7F,
        test_id: 21,
        assertion: "Dropping joypad strobe low starts serial reads at the configured A button bit",
        expected: "first $4016 read after strobe falls low matches the configured expected A bit",
        observed: "$4016 serial reads did not restart at A after the strobe-high phase",
        likely_domain: "joypad.strobe_high_hold",
        remediation_hint: "Inspect joypad strobe transitions; high-strobe reads must not advance the index used after strobe is lowered.",
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
        expected: "$4017 read bit 0 matches configured joypad-2 mask bit 0",
        observed: "$4017 read bit 0 did not match configured joypad-2 mask bit 0",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 bus dispatch and shared strobe latch behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xA1,
        test_id: 11,
        assertion: "Joypad 2 serial read 1 returns the latched B button bit",
        expected: "$4017 read bit 0 matches configured joypad-2 mask bit 1",
        observed: "$4017 read bit 0 did not match configured joypad-2 mask bit 1",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 shift index advancement after the first read.",
    },
    DiagnosticFailureSpec {
        code: 0xA2,
        test_id: 11,
        assertion: "Joypad 2 serial read 2 returns the latched Select button bit",
        expected: "$4017 read bit 0 matches configured joypad-2 mask bit 2",
        observed: "$4017 read bit 0 did not match configured joypad-2 mask bit 2",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 shift order and Select bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0xA3,
        test_id: 11,
        assertion: "Joypad 2 serial read 3 returns the latched Start button bit",
        expected: "$4017 read bit 0 matches configured joypad-2 mask bit 3",
        observed: "$4017 read bit 0 did not match configured joypad-2 mask bit 3",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 Start button mapping and $4017 reads.",
    },
    DiagnosticFailureSpec {
        code: 0xA4,
        test_id: 11,
        assertion: "Joypad 2 serial read 4 returns the latched Up button bit",
        expected: "$4017 read bit 0 matches configured joypad-2 mask bit 4",
        observed: "$4017 read bit 0 did not match configured joypad-2 mask bit 4",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 shift order and Up bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0xA5,
        test_id: 11,
        assertion: "Joypad 2 serial read 5 returns the latched Down button bit",
        expected: "$4017 read bit 0 matches configured joypad-2 mask bit 5",
        observed: "$4017 read bit 0 did not match configured joypad-2 mask bit 5",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 Down button mapping and $4017 reads.",
    },
    DiagnosticFailureSpec {
        code: 0xA6,
        test_id: 11,
        assertion: "Joypad 2 serial read 6 returns the latched Left button bit",
        expected: "$4017 read bit 0 matches configured joypad-2 mask bit 6",
        observed: "$4017 read bit 0 did not match configured joypad-2 mask bit 6",
        likely_domain: "joypad2.strobe_shift",
        remediation_hint: "Inspect joypad 2 shift order and Left bit mapping.",
    },
    DiagnosticFailureSpec {
        code: 0xA7,
        test_id: 11,
        assertion: "Joypad 2 serial read 7 returns the latched Right button bit",
        expected: "$4017 read bit 0 matches configured joypad-2 mask bit 7",
        observed: "$4017 read bit 0 did not match configured joypad-2 mask bit 7",
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
        code: 0x92,
        test_id: 23,
        assertion: "Joypad 1 input-port matrix first strobe-high read matches the configured A bit",
        expected: "$4016 read bit 0 matches configured joypad-1 mask bit 0 while strobe is high",
        observed: "$4016 first strobe-high matrix read did not match the configured joypad-1 A bit",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect $4016 strobe-high reads for joypad 1 before broadening the serial-shift search.",
    },
    DiagnosticFailureSpec {
        code: 0x93,
        test_id: 23,
        assertion: "Joypad 1 input-port matrix repeated strobe-high read still matches the configured A bit",
        expected: "$4016 read bit 0 stays on configured joypad-1 mask bit 0 while strobe remains high",
        observed: "$4016 repeated strobe-high matrix read advanced or changed unexpectedly",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect joypad 1 strobe-high index handling; reads should not advance until strobe falls low.",
    },
    DiagnosticFailureSpec {
        code: 0x94,
        test_id: 23,
        assertion: "Joypad 2 input-port matrix first strobe-high read matches the configured A bit",
        expected: "$4017 read bit 0 matches configured joypad-2 mask bit 0 while strobe is high",
        observed: "$4017 first strobe-high matrix read did not match the configured joypad-2 A bit",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect $4017 strobe-high reads and shared strobe dispatch for joypad 2.",
    },
    DiagnosticFailureSpec {
        code: 0x95,
        test_id: 23,
        assertion: "Joypad 2 input-port matrix repeated strobe-high read still matches the configured A bit",
        expected: "$4017 read bit 0 stays on configured joypad-2 mask bit 0 while strobe remains high",
        observed: "$4017 repeated strobe-high matrix read advanced or changed unexpectedly",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect joypad 2 strobe-high index handling and shared $4016 strobe writes.",
    },
    DiagnosticFailureSpec {
        code: 0x96,
        test_id: 23,
        assertion: "Joypad 1 input-port matrix overread 8 returns one",
        expected: "$4016 read bit 0 == 1 after eight serial reads",
        observed: "$4016 first matrix overread after the eighth button was not 1",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect joypad 1 read behavior once the serial index moves past button 7.",
    },
    DiagnosticFailureSpec {
        code: 0x97,
        test_id: 23,
        assertion: "Joypad 1 input-port matrix overread 9 keeps returning one",
        expected: "$4016 read bit 0 == 1 on repeated overread",
        observed: "$4016 second matrix overread after the eighth button was not 1",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect joypad 1 overread saturation behavior after the eighth button.",
    },
    DiagnosticFailureSpec {
        code: 0x98,
        test_id: 23,
        assertion: "Joypad 2 input-port matrix overread 8 returns one",
        expected: "$4017 read bit 0 == 1 after eight serial reads",
        observed: "$4017 first matrix overread after the eighth button was not 1",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect joypad 2 read behavior once the serial index moves past button 7.",
    },
    DiagnosticFailureSpec {
        code: 0x99,
        test_id: 23,
        assertion: "Joypad 2 input-port matrix overread 9 keeps returning one",
        expected: "$4017 read bit 0 == 1 on repeated overread",
        observed: "$4017 second matrix overread after the eighth button was not 1",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect joypad 2 overread saturation behavior after the eighth button.",
    },
    DiagnosticFailureSpec {
        code: 0x9A,
        test_id: 23,
        assertion: "Joypad 1 input-port matrix serial reads match the configured expected mask",
        expected: "$4016 serial bits 0..7 match the configured joypad-1 mask",
        observed: "$4016 matrix serial reads diverged from the configured joypad-1 mask",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect joypad 1 bit order, index advancement, and button-state reads across a complete serial sequence.",
    },
    DiagnosticFailureSpec {
        code: 0x9B,
        test_id: 23,
        assertion: "Joypad 2 input-port matrix serial reads match the configured expected mask",
        expected: "$4017 serial bits 0..7 match the configured joypad-2 mask",
        observed: "$4017 matrix serial reads diverged from the configured joypad-2 mask",
        likely_domain: "joypad.input_port_matrix",
        remediation_hint: "Inspect joypad 2 bit order, index advancement, and shared strobe behavior across a complete serial sequence.",
    },
    DiagnosticFailureSpec {
        code: 0x83,
        test_id: 24,
        assertion: "OAM DMA phase matrix reaches the second cartridge-triggered transfer",
        expected: "two OAM DMA transfers are triggered by the phase-matrix test",
        observed: "the phase-matrix test stopped before the second OAM DMA transfer",
        likely_domain: "dma.oam_phase_matrix",
        remediation_hint: "Inspect OAM DMA scheduling around consecutive $4014 writes and the host phase-matrix telemetry.",
    },
    DiagnosticFailureSpec {
        code: 0x84,
        test_id: 24,
        assertion: "OAM DMA phase matrix records its expected case count",
        expected: "phase-matrix case count reaches two transfers",
        observed: "the phase-matrix case count did not reach the expected transfer count",
        likely_domain: "dma.oam_phase_matrix",
        remediation_hint: "Inspect the diagnostic cartridge's consecutive DMA trigger path before debugging host telemetry.",
    },
    DiagnosticFailureSpec {
        code: 0x85,
        test_id: 25,
        assertion: "PPUSTATUS reports sprite-zero-hit after sprite/background overlap",
        expected: "PPUSTATUS bit 6 is set after sprite 0 overlaps a non-transparent background pixel",
        observed: "PPUSTATUS sprite-zero-hit bit was not set after the deterministic overlap scene",
        likely_domain: "ppu.sprite_zero_hit",
        remediation_hint: "Inspect sprite evaluation, background/sprite pixel muxing, and PPUSTATUS bit-6 lifetime across vblank.",
    },
    DiagnosticFailureSpec {
        code: 0x86,
        test_id: 26,
        assertion: "PPUSTATUS reports sprite overflow after nine sprites share one scanline",
        expected: "PPUSTATUS bit 5 is set after sprite evaluation sees more than eight in-range sprites",
        observed: "PPUSTATUS sprite-overflow bit was not set after the deterministic overflow scene",
        likely_domain: "ppu.sprite_overflow",
        remediation_hint: "Inspect secondary OAM sprite evaluation, overflow-bit setting, and PPUSTATUS bit-5 lifetime across vblank.",
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
        code: 0xB2,
        test_id: 22,
        assertion: "Absolute,X non-crossing load reads the expected sentinel",
        expected: "LDA $0440,X with X=0x02 reads the byte stored at $0442",
        observed: "A differed from the $0442 absolute,X sentinel",
        likely_domain: "cpu.addressing.absolute_x_load",
        remediation_hint: "Inspect absolute,X effective-address calculation and LDA readback before broadening the opcode search.",
    },
    DiagnosticFailureSpec {
        code: 0xB3,
        test_id: 22,
        assertion: "Absolute,X page-crossing load reads the expected sentinel",
        expected: "LDA $04FF,X with X=0x01 reads the byte stored at $0500",
        observed: "A differed from the $0500 absolute,X page-crossing sentinel",
        likely_domain: "cpu.addressing.page_cross_load",
        remediation_hint: "Inspect absolute,X page-crossing effective-address calculation and page-cross cycle penalty handling.",
    },
    DiagnosticFailureSpec {
        code: 0xB4,
        test_id: 22,
        assertion: "Indirect,Y page-crossing load reads the expected sentinel",
        expected: "LDA ($42),Y with pointer $04FF and Y=0x01 reads the byte stored at $0500",
        observed: "A differed from the $0500 indirect,Y page-crossing sentinel",
        likely_domain: "cpu.addressing.indirect_y_page_cross_load",
        remediation_hint: "Inspect indirect,Y zero-page pointer wrapping, Y indexing, and page-cross cycle penalty handling.",
    },
    DiagnosticFailureSpec {
        code: 0xB5,
        test_id: CPU_RMW_MATRIX_TEST_ID,
        assertion: "ASL zero-page shifts memory left and writes the result back",
        expected: "ASL $30 turns 0x40 into 0x80 in zero-page RAM",
        observed: "$0030 did not contain the ASL write-back sentinel",
        likely_domain: "cpu.rmw.asl",
        remediation_hint: "Inspect ASL zero-page read/modify/write sequencing, carry-out, and memory write-back behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xB6,
        test_id: CPU_RMW_MATRIX_TEST_ID,
        assertion: "ROL zero-page consumes carry-in and writes the rotated result back",
        expected: "SEC; ROL $31 turns 0x80 into 0x01 in zero-page RAM",
        observed: "$0031 did not contain the ROL write-back sentinel",
        likely_domain: "cpu.rmw.rol",
        remediation_hint: "Inspect ROL zero-page carry-in/carry-out handling and memory write-back behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xB7,
        test_id: CPU_RMW_MATRIX_TEST_ID,
        assertion: "LSR zero-page shifts memory right and writes the result back",
        expected: "LSR $32 turns 0x81 into 0x40 in zero-page RAM",
        observed: "$0032 did not contain the LSR write-back sentinel",
        likely_domain: "cpu.rmw.lsr",
        remediation_hint: "Inspect LSR zero-page read/modify/write sequencing, carry-out, and memory write-back behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xB8,
        test_id: CPU_RMW_MATRIX_TEST_ID,
        assertion: "ROR zero-page consumes carry-in and writes the rotated result back",
        expected: "SEC; ROR $33 turns 0x01 into 0x80 in zero-page RAM",
        observed: "$0033 did not contain the ROR write-back sentinel",
        likely_domain: "cpu.rmw.ror",
        remediation_hint: "Inspect ROR zero-page carry-in/carry-out handling and memory write-back behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xB9,
        test_id: CPU_RMW_MATRIX_TEST_ID,
        assertion: "INC zero-page wraps 0xFF to 0x00 and writes the result back",
        expected: "INC $34 turns 0xFF into 0x00 in zero-page RAM",
        observed: "$0034 did not contain the INC write-back sentinel",
        likely_domain: "cpu.rmw.inc",
        remediation_hint: "Inspect INC zero-page memory write-back and zero/negative flag updates.",
    },
    DiagnosticFailureSpec {
        code: 0xBA,
        test_id: CPU_RMW_MATRIX_TEST_ID,
        assertion: "DEC zero-page wraps 0x00 to 0xFF and writes the result back",
        expected: "DEC $35 turns 0x00 into 0xFF in zero-page RAM",
        observed: "$0035 did not contain the DEC write-back sentinel",
        likely_domain: "cpu.rmw.dec",
        remediation_hint: "Inspect DEC zero-page memory write-back and zero/negative flag updates.",
    },
    DiagnosticFailureSpec {
        code: 0xCA,
        test_id: CPU_RMW_ADDRESSING_MATRIX_TEST_ID,
        assertion: "ASL absolute shifts memory left and writes the result back",
        expected: "ASL $0450 turns 0x40 into 0x80 in CPU RAM",
        observed: "$0450 did not contain the ASL absolute write-back sentinel",
        likely_domain: "cpu.rmw.absolute_asl",
        remediation_hint: "Inspect ASL absolute effective-address resolution, read/modify/write sequencing, and memory write-back behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xCB,
        test_id: CPU_RMW_ADDRESSING_MATRIX_TEST_ID,
        assertion: "ROL absolute,X consumes carry-in and writes the indexed result back",
        expected: "SEC; ROL $04FD,X with X=0x03 turns $0500 from 0x80 into 0x01",
        observed: "$0500 did not contain the ROL absolute,X write-back sentinel",
        likely_domain: "cpu.rmw.absolute_x_rol",
        remediation_hint: "Inspect ROL absolute,X indexed effective-address resolution, carry-in/carry-out handling, and memory write-back behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xCC,
        test_id: CPU_RMW_ADDRESSING_MATRIX_TEST_ID,
        assertion: "LSR absolute shifts memory right and writes the result back",
        expected: "LSR $0470 turns 0x81 into 0x40 in CPU RAM",
        observed: "$0470 did not contain the LSR absolute write-back sentinel",
        likely_domain: "cpu.rmw.absolute_lsr",
        remediation_hint: "Inspect LSR absolute read/modify/write sequencing, carry-out, and memory write-back behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xCD,
        test_id: CPU_RMW_ADDRESSING_MATRIX_TEST_ID,
        assertion: "ROR absolute,X consumes carry-in and writes the indexed result back",
        expected: "SEC; ROR $04FE,X with X=0x04 turns $0502 from 0x01 into 0x80",
        observed: "$0502 did not contain the ROR absolute,X write-back sentinel",
        likely_domain: "cpu.rmw.absolute_x_ror",
        remediation_hint: "Inspect ROR absolute,X indexed effective-address resolution, carry-in/carry-out handling, and memory write-back behavior.",
    },
    DiagnosticFailureSpec {
        code: 0xCE,
        test_id: CPU_RMW_ADDRESSING_MATRIX_TEST_ID,
        assertion: "INC absolute wraps 0xFF to 0x00 and writes the result back",
        expected: "INC $0490 turns 0xFF into 0x00 in CPU RAM",
        observed: "$0490 did not contain the INC absolute write-back sentinel",
        likely_domain: "cpu.rmw.absolute_inc",
        remediation_hint: "Inspect INC absolute memory write-back and zero/negative flag updates.",
    },
    DiagnosticFailureSpec {
        code: 0xCF,
        test_id: CPU_RMW_ADDRESSING_MATRIX_TEST_ID,
        assertion: "DEC absolute,X wraps 0x00 to 0xFF and writes the indexed result back",
        expected: "DEC $04FA,X with X=0x05 turns $04FF from 0x00 into 0xFF",
        observed: "$04FF did not contain the DEC absolute,X write-back sentinel",
        likely_domain: "cpu.rmw.absolute_x_dec",
        remediation_hint: "Inspect DEC absolute,X indexed effective-address resolution, memory write-back, and zero/negative flag updates.",
    },
    DiagnosticFailureSpec {
        code: 0xD2,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "BPL branches only when the negative flag is clear",
        expected: "BPL reaches its taken target with N=0 and falls through with N=1",
        observed: "BPL took the wrong path in the branch condition matrix",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect BPL condition evaluation, status negative-flag updates, and relative branch PC updates.",
    },
    DiagnosticFailureSpec {
        code: 0xD3,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "BMI branches only when the negative flag is set",
        expected: "BMI reaches its taken target with N=1 and falls through with N=0",
        observed: "BMI took the wrong path in the branch condition matrix",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect BMI condition evaluation, status negative-flag updates, and relative branch PC updates.",
    },
    DiagnosticFailureSpec {
        code: 0xD4,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "BVC branches only when the overflow flag is clear",
        expected: "BVC reaches its taken target with V=0 and falls through with V=1",
        observed: "BVC took the wrong path in the branch condition matrix",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect BVC condition evaluation, overflow-flag updates, and relative branch PC updates.",
    },
    DiagnosticFailureSpec {
        code: 0xD5,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "BVS branches only when the overflow flag is set",
        expected: "BVS reaches its taken target with V=1 and falls through with V=0",
        observed: "BVS took the wrong path in the branch condition matrix",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect BVS condition evaluation, overflow-flag updates, and relative branch PC updates.",
    },
    DiagnosticFailureSpec {
        code: 0xD6,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "BCC branches only when the carry flag is clear",
        expected: "BCC reaches its taken target with C=0 and falls through with C=1",
        observed: "BCC took the wrong path in the branch condition matrix",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect BCC condition evaluation, carry-flag updates, and relative branch PC updates.",
    },
    DiagnosticFailureSpec {
        code: 0xD7,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "BCS branches only when the carry flag is set",
        expected: "BCS reaches its taken target with C=1 and falls through with C=0",
        observed: "BCS took the wrong path in the branch condition matrix",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect BCS condition evaluation, carry-flag updates, and relative branch PC updates.",
    },
    DiagnosticFailureSpec {
        code: 0xD8,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "BNE branches only when the zero flag is clear",
        expected: "BNE reaches its taken target with Z=0 and falls through with Z=1",
        observed: "BNE took the wrong path in the branch condition matrix",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect BNE condition evaluation, zero-flag updates, and relative branch PC updates.",
    },
    DiagnosticFailureSpec {
        code: 0xD9,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "BEQ branches only when the zero flag is set",
        expected: "BEQ reaches its taken target with Z=1 and falls through with Z=0",
        observed: "BEQ took the wrong path in the branch condition matrix",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect BEQ condition evaluation, zero-flag updates, and relative branch PC updates.",
    },
    DiagnosticFailureSpec {
        code: 0xDA,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "Page-crossing branch target executes normally inside the branch matrix",
        expected: "A taken BNE placed at page low byte 0xFC stores the page-cross sentinel at the next-page target",
        observed: "the page-cross branch did not reach the expected target",
        likely_domain: "cpu.branch.page_cross",
        remediation_hint: "Inspect relative branch signed-offset calculation and page-cross PC updates.",
    },
    DiagnosticFailureSpec {
        code: 0xDB,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "Branch condition matrix records every taken branch case",
        expected: "taken mask == 0xFF",
        observed: "one or more true branch-condition cases did not set the taken mask bit",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect conditional branch opcode dispatch and status-flag inputs before checking cycle accounting.",
    },
    DiagnosticFailureSpec {
        code: 0xDC,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "Branch condition matrix records every not-taken branch case",
        expected: "not-taken mask == 0xFF",
        observed: "one or more false branch-condition cases incorrectly branched",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect fallthrough PC advancement for branch opcodes when their status-flag condition is false.",
    },
    DiagnosticFailureSpec {
        code: 0xDD,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "Branch condition matrix records the page-cross sentinel",
        expected: "page-cross result == 0x5C",
        observed: "the page-cross branch result byte did not match the expected sentinel",
        likely_domain: "cpu.branch.page_cross",
        remediation_hint: "Inspect relative branch target calculation when a taken branch crosses a CPU page boundary.",
    },
    DiagnosticFailureSpec {
        code: 0xDE,
        test_id: CPU_BRANCH_MATRIX_TEST_ID,
        assertion: "Branch condition matrix records its expected case count",
        expected: "case count == 17",
        observed: "the branch matrix case count did not match the expected taken, not-taken, and page-cross cases",
        likely_domain: "cpu.branch.condition_matrix",
        remediation_hint: "Inspect branch matrix execution flow and any early exits before broadening the CPU opcode search.",
    },
    DiagnosticFailureSpec {
        code: 0xA8,
        test_id: CPU_STACK_MATRIX_TEST_ID,
        assertion: "TXS and TSX preserve the selected stack pointer",
        expected: "TSX reads back stack pointer 0xF0 after TXS",
        observed: "TSX did not return the stack pointer written by TXS",
        likely_domain: "cpu.stack.status_matrix",
        remediation_hint: "Inspect TXS/TSX transfer semantics and stack-pointer storage independent of accumulator flags.",
    },
    DiagnosticFailureSpec {
        code: 0xA9,
        test_id: CPU_STACK_MATRIX_TEST_ID,
        assertion: "PHA and PLA preserve the pushed accumulator byte",
        expected: "PLA restores accumulator sentinel 0xA6",
        observed: "PLA returned a different byte from the stack",
        likely_domain: "cpu.stack.status_matrix",
        remediation_hint: "Inspect stack page addressing plus pre/post stack-pointer increment and decrement behavior for PHA/PLA.",
    },
    DiagnosticFailureSpec {
        code: 0xAA,
        test_id: CPU_STACK_MATRIX_TEST_ID,
        assertion: "PLP restores the zero flag saved by PHP",
        expected: "BEQ is taken after PHP saves Z=1 and PLP restores it",
        observed: "the restored status did not preserve the zero flag",
        likely_domain: "cpu.stack.status_matrix",
        remediation_hint: "Inspect PHP pushed status bits and PLP restoration of the zero flag after intervening accumulator changes.",
    },
    DiagnosticFailureSpec {
        code: 0xAB,
        test_id: CPU_STACK_MATRIX_TEST_ID,
        assertion: "PLP restores the carry flag saved by PHP",
        expected: "BCC is taken after PHP saves C=0 and PLP restores it",
        observed: "the restored status did not preserve the carry flag",
        likely_domain: "cpu.stack.status_matrix",
        remediation_hint: "Inspect PHP/PLP status-mask handling, especially carry, break, and unused-bit normalization.",
    },
    DiagnosticFailureSpec {
        code: 0xAC,
        test_id: CPU_STACK_MATRIX_TEST_ID,
        assertion: "JSR and RTS round-trip through the stack",
        expected: "RTS returns to the caller with accumulator sentinel 0x77",
        observed: "JSR/RTS did not return to the expected continuation with the expected accumulator",
        likely_domain: "cpu.stack.status_matrix",
        remediation_hint: "Inspect JSR return-address push order, RTS pull order, and the post-return PC increment.",
    },
    DiagnosticFailureSpec {
        code: 0xAD,
        test_id: CPU_STACK_MATRIX_TEST_ID,
        assertion: "Stack matrix restores final stack depth",
        expected: "final TSX reads stack pointer 0xF0 after push/pop and JSR/RTS cases",
        observed: "the final stack pointer did not match the selected stack depth",
        likely_domain: "cpu.stack.status_matrix",
        remediation_hint: "Inspect whether push/pull, PHP/PLP, or JSR/RTS leave the stack pointer unbalanced.",
    },
    DiagnosticFailureSpec {
        code: 0xAE,
        test_id: CPU_STACK_MATRIX_TEST_ID,
        assertion: "Stack matrix records its expected case count",
        expected: "case count == 5",
        observed: "the stack matrix case count did not match the expected TXS/TSX, PHA/PLA, PHP/PLP, JSR/RTS, and final-SP cases",
        likely_domain: "cpu.stack.status_matrix",
        remediation_hint: "Inspect stack matrix execution flow and any early exits before broadening the CPU opcode matrix further.",
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
        current_coverage: "ADC/SBC arithmetic, flags, stack push/pop, JSR/RTS, a taken page-crossing branch, a conditional branch matrix covering all official branch opcodes across taken and not-taken flag states plus a page-crossing branch target, zero-page indexed wraparound, indirect JMP page-wrap behavior, a telemetry-backed load-addressing matrix covering absolute,X plus indirect,Y page-crossing cases, a zero-page read-modify-write matrix covering ASL, ROL, LSR, ROR, INC, and DEC memory write-back sentinels, and a non-zero-page RMW addressing matrix covering absolute plus page-crossing absolute,X write-back sentinels.",
        missing_coverage: "Complete official opcode matrix, illegal opcodes, interrupt priority edge cases, indirect read/modify/write addressing is not applicable to official 6502 opcodes but accumulator and broader addressing/flag combinations remain incomplete, and broader cycle-accurate addressing penalties beyond targeted branch and load page-crossing cases.",
        suggested_next_test: "Generate an opcode/addressing-mode matrix cartridge that records accumulator, flags, memory side effects, and cycle buckets per case across all official opcodes.",
    },
    DiagnosticCoverageGapSpec {
        id: "ppu_pixel_pipeline",
        subsystem: "ppu",
        risk: "The cartridge catches gross PPU progress and selected pixel behavior but does not prove detailed scanline/dot correctness.",
        current_coverage: "Palette register round-trip, non-palette PPUDATA read buffering, PPUDATA increment-by-32 register behavior, PPUSTATUS write-latch reset behavior, horizontal nametable mirroring, sprite-zero-hit collision signaling, sprite-overflow evaluation including hardware-bug false-positive and false-negative subcases, sprite/background priority pixel sampling, fine-X horizontal scroll seam sampling, coarse-X tile-shift sampling, coarse-X nametable-wrap sampling through a vertical-mirroring variant cartridge, vertical scroll seam sampling, NMI delivery, host-observed first/inter-NMI vblank timing windows, PPUSTATUS vblank set/clear dot-edge timing, completed frames, host-visible multi-color background output, and an expected full-frame render checksum for the deterministic background frame.",
        missing_coverage: "Per-dot rendering behavior beyond targeted sprite-priority, scroll-seam, sprite-overflow, and deterministic full-frame signature samples.",
        suggested_next_test: "Add broader per-dot renderer checks with scanline/window-local expected signatures and tile fetch phase assertions.",
    },
    DiagnosticCoverageGapSpec {
        id: "mapper_banking_runtime",
        subsystem: "cartridge",
        risk: "The diagnostic cartridges now exercise several simple bank-switching mappers, but broader mapper behavior can still regress outside these fixtures.",
        current_coverage: "The generated Mapper 1/MMC1 variants validate serial shift-register commits, delayed PRG bank commit after four writes, fixed-last PRG mode, MMC1 32 KiB PRG modes 0/1 with ignored low PRG bank bit, 4 KiB CHR bank switching, and single-screen lower/upper mirroring end to end; the generated Mapper 2/UXROM cartridge validates CPU-visible PRG bank switching, the fixed final-bank window, PRG RAM round-trips, and header-declared horizontal nametable mirroring end to end; a generated Mapper 3/CNROM variant validates CPU bank-select writes and PPU-visible CHR bank reads across four CHR banks; generated Mapper 4/MMC3 variants validate R6/R7 PRG bank writes, fixed-last PRG reads, 2 KiB and 1 KiB CHR bank reads, mirroring control, scanline IRQ delivery, PRG-mode inversion, CHR inversion across all eight 1 KiB windows, IRQ reload phases including a zero-latch reload, and battery-backed PRG RAM write/read plus host SRAM restore behavior; a generated Mapper 7/AxROM variant validates 32 KiB PRG bank switching plus single-screen lower/upper mirroring through CPU and PPU bus paths.",
        missing_coverage: "Active-render CHR/PRG switches, deeper MMC3 IRQ A12 filtering behavior, and broader mapper-family coverage beyond the generated variants.",
        suggested_next_test: "Generate MMC-style synthetic cartridges that switch CHR/PRG banks during rendering and assert selected pattern/table data through PPU-visible pixels and mapper IRQ timing.",
    },
    DiagnosticCoverageGapSpec {
        id: "apu_audio_depth",
        subsystem: "apu",
        risk: "The cartridge proves APU status and sample production, not channel accuracy or mixer behavior.",
        current_coverage: "$4015 non-DMC channel status matrix for pulse 1, pulse 2, triangle, and noise, DMC active-bit observation during sample-DMA setup, plus host-observed drained sample-count, peak, RMS, and mean absolute audio envelope windows at frame boundaries.",
        missing_coverage: "Per-channel envelope, sweep, triangle/noise/DMC waveform behavior, frame counter timing, mixer levels, and IRQ edge cases.",
        suggested_next_test: "Add per-channel register programs with channel-specific waveform windows and mixer-level expectations.",
    },
    DiagnosticCoverageGapSpec {
        id: "dma_cycle_timing",
        subsystem: "dma",
        risk: "The cartridge validates OAM contents and both host-observed OAM DMA stall-length phases, but not all DMA interactions.",
        current_coverage: "A full-page OAM DMA transfer produces the expected OAM checksum, a phase-matrix test forces multiple OAM DMA transfers across both 513-cycle and 514-cycle start-phase buckets, DMC sample DMA overlaps the OAM stall window, the phase-specific 3-4 cycle DMC stall bucket is validated, host telemetry records DMC/OAM overlap offsets covering beginning, middle, and end placement buckets, and a DMC-active phase-matrix burst train spans at least three distinct OAM transfers.",
        missing_coverage: "Deeper CPU/APU interleaving across longer-running DMA sequences, repeated bursts with mixed DMC reload rates, and interrupt-adjacent DMA service ordering.",
        suggested_next_test: "Extend the DMA matrix with longer CPU/APU interleaving sequences, varied DMC rates/sample lengths, and interrupt-boundary DMA service ordering checks.",
    },
    DiagnosticCoverageGapSpec {
        id: "input_port_matrix",
        subsystem: "joypad",
        risk: "The cartridge proves fixed serial-read masks for both controller ports but not the full input state matrix.",
        current_coverage: "Joypad 1 and joypad 2 strobe/shift sequences use explicit expected masks, the scenario suite includes generated default, alternating, all-released, all-pressed, joypad-1-only pressed, joypad-2-only pressed, sparse-bits, and nibble-split input-mask pass fixtures, joypad 1 verifies mid-stream strobe reset behavior, a combined input-port matrix verifies both $4016 and $4017 strobe-high reads, full eight-bit serial masks, and overreads, and a generated input-mask sweep variant reconstructs both serial bytes across 16 host-applied mask pairs.",
        missing_coverage: "Exhaustive 65,536 two-port mask sweeps, disconnected-controller electrical defaults beyond all-released masks, and host input remapping.",
        suggested_next_test: "Add an optional exhaustive input-port sweep mode and host-remapping fixtures.",
    },
];

#[derive(Debug, Clone)]
pub struct DiagnosticConfig {
    pub max_cpu_cycles: u64,
    pub joypad1_mask: u8,
    pub expected_joypad1_mask: u8,
    pub joypad2_mask: u8,
    pub expected_joypad2_mask: u8,
    pub fault_injection: Option<DiagnosticFaultInjection>,
}

impl Default for DiagnosticConfig {
    fn default() -> Self {
        Self {
            max_cpu_cycles: 500_000,
            joypad1_mask: EXPECTED_JOYPAD1_MASK,
            expected_joypad1_mask: EXPECTED_JOYPAD1_MASK,
            joypad2_mask: EXPECTED_JOYPAD2_MASK,
            expected_joypad2_mask: EXPECTED_JOYPAD2_MASK,
            fault_injection: None,
        }
    }
}

fn diagnostic_render_frame_signature_validation(config: &DiagnosticConfig) -> (bool, &'static str) {
    if config.fault_injection.is_some() {
        return (false, DIAGNOSTIC_RENDER_FRAME_SIGNATURE_FAULT_REASON);
    }
    if config.joypad1_mask != EXPECTED_JOYPAD1_MASK
        || config.expected_joypad1_mask != EXPECTED_JOYPAD1_MASK
        || config.joypad2_mask != EXPECTED_JOYPAD2_MASK
        || config.expected_joypad2_mask != EXPECTED_JOYPAD2_MASK
    {
        return (false, DIAGNOSTIC_RENDER_FRAME_SIGNATURE_INPUT_REASON);
    }
    (true, DIAGNOSTIC_RENDER_FRAME_SIGNATURE_ENABLED_REASON)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticFaultInjection {
    ApuStatusRegister,
    CpuAddressingModeMatrix,
    CpuBranchConditionMatrix,
    CpuIndirectJmpPageWrap,
    CpuRamMirroring,
    CpuReadModifyWriteAddressingMatrix,
    CpuReadModifyWriteMatrix,
    CpuStackStatusMatrix,
    CpuZeroPageIndexWrap,
    DmaOamTransfer,
    DmaPhaseMatrix,
    InputPortMatrix,
    JoypadStrobeHighHold,
    JoypadStrobeReset,
    Mapper2PrgBankSwitch,
    Mapper2PrgRam,
    PpuNametableMirroring,
    PpuNmiTimeout,
    PpuScrollSeam,
    PpuSpriteOverflow,
    PpuSpritePriority,
    PpuSpriteZeroHit,
    PpuStatusLatchReset,
    PpuVramIncrement32,
    PpuVramReadBuffer,
}

impl DiagnosticFaultInjection {
    pub const ALL: [DiagnosticFaultInjection; 25] = [
        DiagnosticFaultInjection::ApuStatusRegister,
        DiagnosticFaultInjection::CpuAddressingModeMatrix,
        DiagnosticFaultInjection::CpuBranchConditionMatrix,
        DiagnosticFaultInjection::CpuIndirectJmpPageWrap,
        DiagnosticFaultInjection::CpuRamMirroring,
        DiagnosticFaultInjection::CpuReadModifyWriteAddressingMatrix,
        DiagnosticFaultInjection::CpuReadModifyWriteMatrix,
        DiagnosticFaultInjection::CpuStackStatusMatrix,
        DiagnosticFaultInjection::CpuZeroPageIndexWrap,
        DiagnosticFaultInjection::DmaOamTransfer,
        DiagnosticFaultInjection::DmaPhaseMatrix,
        DiagnosticFaultInjection::InputPortMatrix,
        DiagnosticFaultInjection::JoypadStrobeHighHold,
        DiagnosticFaultInjection::JoypadStrobeReset,
        DiagnosticFaultInjection::Mapper2PrgBankSwitch,
        DiagnosticFaultInjection::Mapper2PrgRam,
        DiagnosticFaultInjection::PpuNametableMirroring,
        DiagnosticFaultInjection::PpuNmiTimeout,
        DiagnosticFaultInjection::PpuScrollSeam,
        DiagnosticFaultInjection::PpuSpriteOverflow,
        DiagnosticFaultInjection::PpuSpritePriority,
        DiagnosticFaultInjection::PpuSpriteZeroHit,
        DiagnosticFaultInjection::PpuStatusLatchReset,
        DiagnosticFaultInjection::PpuVramIncrement32,
        DiagnosticFaultInjection::PpuVramReadBuffer,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticFaultInjection::ApuStatusRegister => "apu_status_register",
            DiagnosticFaultInjection::CpuAddressingModeMatrix => "cpu_addressing_mode_matrix",
            DiagnosticFaultInjection::CpuBranchConditionMatrix => "cpu_branch_condition_matrix",
            DiagnosticFaultInjection::CpuIndirectJmpPageWrap => "cpu_indirect_jmp_page_wrap",
            DiagnosticFaultInjection::CpuRamMirroring => "cpu_ram_mirroring",
            DiagnosticFaultInjection::CpuReadModifyWriteAddressingMatrix => {
                "cpu_rmw_addressing_matrix"
            }
            DiagnosticFaultInjection::CpuReadModifyWriteMatrix => "cpu_rmw_matrix",
            DiagnosticFaultInjection::CpuStackStatusMatrix => "cpu_stack_status_matrix",
            DiagnosticFaultInjection::CpuZeroPageIndexWrap => "cpu_zero_page_index_wrap",
            DiagnosticFaultInjection::DmaOamTransfer => "dma_oam_transfer",
            DiagnosticFaultInjection::DmaPhaseMatrix => "dma_phase_matrix",
            DiagnosticFaultInjection::InputPortMatrix => "input_port_matrix",
            DiagnosticFaultInjection::JoypadStrobeHighHold => "joypad_strobe_high_hold",
            DiagnosticFaultInjection::JoypadStrobeReset => "joypad_strobe_reset",
            DiagnosticFaultInjection::Mapper2PrgBankSwitch => "mapper2_prg_bank_switch",
            DiagnosticFaultInjection::Mapper2PrgRam => "mapper2_prg_ram",
            DiagnosticFaultInjection::PpuNametableMirroring => "ppu_nametable_mirroring",
            DiagnosticFaultInjection::PpuNmiTimeout => "ppu_nmi_timeout",
            DiagnosticFaultInjection::PpuScrollSeam => "ppu_scroll_seam",
            DiagnosticFaultInjection::PpuSpriteOverflow => "ppu_sprite_overflow",
            DiagnosticFaultInjection::PpuSpritePriority => "ppu_sprite_priority",
            DiagnosticFaultInjection::PpuSpriteZeroHit => "ppu_sprite_zero_hit",
            DiagnosticFaultInjection::PpuStatusLatchReset => "ppu_status_latch_reset",
            DiagnosticFaultInjection::PpuVramIncrement32 => "ppu_vram_increment_32",
            DiagnosticFaultInjection::PpuVramReadBuffer => "ppu_vram_read_buffer",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|fault| fault.as_str() == value)
    }

    fn injection_label(self) -> &'static str {
        match self {
            DiagnosticFaultInjection::ApuStatusRegister => APU_STATUS_FAULT_LABEL,
            DiagnosticFaultInjection::CpuAddressingModeMatrix => CPU_ADDRESSING_MATRIX_FAULT_LABEL,
            DiagnosticFaultInjection::CpuBranchConditionMatrix => CPU_BRANCH_MATRIX_FAULT_LABEL,
            DiagnosticFaultInjection::CpuIndirectJmpPageWrap => CPU_INDIRECT_JMP_FAULT_LABEL,
            DiagnosticFaultInjection::CpuRamMirroring => CPU_RAM_MIRRORING_FAULT_LABEL,
            DiagnosticFaultInjection::CpuReadModifyWriteAddressingMatrix => {
                CPU_RMW_ADDRESSING_MATRIX_FAULT_LABEL
            }
            DiagnosticFaultInjection::CpuReadModifyWriteMatrix => CPU_RMW_MATRIX_FAULT_LABEL,
            DiagnosticFaultInjection::CpuStackStatusMatrix => CPU_STACK_MATRIX_FAULT_LABEL,
            DiagnosticFaultInjection::CpuZeroPageIndexWrap => CPU_ZERO_PAGE_WRAP_FAULT_LABEL,
            DiagnosticFaultInjection::DmaOamTransfer => DMA_OAM_TRANSFER_FAULT_LABEL,
            DiagnosticFaultInjection::DmaPhaseMatrix => DMA_PHASE_MATRIX_FAULT_LABEL,
            DiagnosticFaultInjection::InputPortMatrix => INPUT_PORT_MATRIX_FAULT_LABEL,
            DiagnosticFaultInjection::JoypadStrobeHighHold => JOYPAD_STROBE_HIGH_HOLD_FAULT_LABEL,
            DiagnosticFaultInjection::JoypadStrobeReset => JOYPAD_STROBE_RESET_FAULT_LABEL,
            DiagnosticFaultInjection::Mapper2PrgBankSwitch => MAPPER2_BANK_SWITCH_FAULT_LABEL,
            DiagnosticFaultInjection::Mapper2PrgRam => MAPPER2_PRG_RAM_FAULT_LABEL,
            DiagnosticFaultInjection::PpuNametableMirroring => PPU_NAMETABLE_MIRRORING_FAULT_LABEL,
            DiagnosticFaultInjection::PpuNmiTimeout => PPU_NMI_TIMEOUT_FAULT_LABEL,
            DiagnosticFaultInjection::PpuScrollSeam => PPU_SCROLL_SEAM_FAULT_LABEL,
            DiagnosticFaultInjection::PpuSpriteOverflow => PPU_SPRITE_OVERFLOW_FAULT_LABEL,
            DiagnosticFaultInjection::PpuSpritePriority => PPU_SPRITE_PRIORITY_FAULT_LABEL,
            DiagnosticFaultInjection::PpuSpriteZeroHit => PPU_SPRITE_ZERO_HIT_FAULT_LABEL,
            DiagnosticFaultInjection::PpuStatusLatchReset => PPU_STATUS_LATCH_RESET_FAULT_LABEL,
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
    pub mapper1_mmc1: Mapper1Mmc1Telemetry,
    pub mapper1_mmc1_32k_prg: Mapper1Mmc1Prg32kTelemetry,
    pub mapper3_chr_bank: Mapper3ChrBankTelemetry,
    pub mapper4_mmc3: Mapper4Mmc3Telemetry,
    pub mapper4_mmc3_edge: Mapper4Mmc3EdgeTelemetry,
    pub mapper4_mmc3_prg_ram: Mapper4Mmc3PrgRamTelemetry,
    pub mapper7_axrom: Mapper7AxromTelemetry,
    pub input_mask_sweep: InputMaskSweepTelemetry,
    pub input: DiagnosticInputTelemetry,
    pub verdict: VerdictTelemetry,
    pub analysis: DiagnosticAnalysisTelemetry,
    pub cycles: u64,
    pub frames: u64,
    pub cpu: CpuTelemetry,
    pub cpu_addressing_matrix: CpuAddressingMatrixTelemetry,
    pub cpu_branch_matrix: CpuBranchMatrixTelemetry,
    pub cpu_rmw_addressing_matrix: CpuRmwAddressingMatrixTelemetry,
    pub cpu_rmw_matrix: CpuRmwMatrixTelemetry,
    pub cpu_stack_matrix: CpuStackMatrixTelemetry,
    pub input_port_matrix: InputPortMatrixTelemetry,
    pub apu_status_matrix: ApuStatusMatrixTelemetry,
    pub apu_dmc_status: ApuDmcStatusTelemetry,
    pub ppu_vblank_timing: PpuVblankTimingTelemetry,
    pub ppu_scroll_seam: PpuScrollSeamTelemetry,
    pub ppu_sprite_overflow: PpuSpriteOverflowTelemetry,
    pub ppu_sprite_priority: PpuSpritePriorityTelemetry,
    pub ppu_sprite_zero_hit: PpuSpriteZeroHitTelemetry,
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
pub struct Mapper1Mmc1Telemetry {
    pub mapper: u8,
    pub prg_banks: u8,
    pub chr_8k_banks: u8,
    pub chr_4k_banks: usize,
    pub prg_switch_addr: u16,
    pub prg_switch_addr_hex: String,
    pub prg_fixed_addr: u16,
    pub prg_fixed_addr_hex: String,
    pub chr_low_read_addr: u16,
    pub chr_low_read_addr_hex: String,
    pub chr_high_read_addr: u16,
    pub chr_high_read_addr_hex: String,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub prg_bank_writes: Vec<u8>,
    pub prg_bank_writes_hex: Vec<String>,
    pub chr_bank_writes: Vec<u8>,
    pub chr_bank_writes_hex: Vec<String>,
    pub expected_prg_values: Vec<u8>,
    pub expected_prg_values_hex: Vec<String>,
    pub observed_prg_values: Vec<u8>,
    pub observed_prg_values_hex: Vec<String>,
    pub expected_chr_values: Vec<u8>,
    pub expected_chr_values_hex: Vec<String>,
    pub observed_chr_values: Vec<u8>,
    pub observed_chr_values_hex: Vec<String>,
    pub expected_mirror_values: Vec<u8>,
    pub expected_mirror_values_hex: Vec<String>,
    pub observed_mirror_values: Vec<u8>,
    pub observed_mirror_values_hex: Vec<String>,
    pub cycles: u64,
    pub frames: u64,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Mapper1Mmc1Prg32kTelemetry {
    pub mapper: u8,
    pub prg_banks: u8,
    pub chr_8k_banks: u8,
    pub low_read_addr: u16,
    pub low_read_addr_hex: String,
    pub high_read_addr: u16,
    pub high_read_addr_hex: String,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub control_writes: Vec<u8>,
    pub control_writes_hex: Vec<String>,
    pub prg_bank_writes: Vec<u8>,
    pub prg_bank_writes_hex: Vec<String>,
    pub expected_values: Vec<u8>,
    pub expected_values_hex: Vec<String>,
    pub observed_values: Vec<u8>,
    pub observed_values_hex: Vec<String>,
    pub cycles: u64,
    pub frames: u64,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Mapper3ChrBankTelemetry {
    pub mapper: u8,
    pub prg_banks: u8,
    pub chr_banks: u8,
    pub read_addr: u16,
    pub read_addr_hex: String,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub expected_banks: Vec<u8>,
    pub expected_values: Vec<u8>,
    pub expected_values_hex: Vec<String>,
    pub observed_values: Vec<u8>,
    pub observed_values_hex: Vec<String>,
    pub cycles: u64,
    pub frames: u64,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Mapper4Mmc3Telemetry {
    pub mapper: u8,
    pub prg_16k_banks: u8,
    pub prg_8k_banks: usize,
    pub chr_8k_banks: u8,
    pub chr_1k_banks: usize,
    pub prg_read_addrs: Vec<u16>,
    pub prg_read_addrs_hex: Vec<String>,
    pub chr_read_addrs: Vec<u16>,
    pub chr_read_addrs_hex: Vec<String>,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub prg_register_writes: Vec<(u8, u8)>,
    pub prg_register_writes_hex: Vec<String>,
    pub chr_register_writes: Vec<(u8, u8)>,
    pub chr_register_writes_hex: Vec<String>,
    pub irq_latch: u8,
    pub irq_latch_hex: String,
    pub expected_irq_count: u8,
    pub observed_irq_count: u8,
    pub expected_prg_values: Vec<u8>,
    pub expected_prg_values_hex: Vec<String>,
    pub observed_prg_values: Vec<u8>,
    pub observed_prg_values_hex: Vec<String>,
    pub expected_chr_values: Vec<u8>,
    pub expected_chr_values_hex: Vec<String>,
    pub observed_chr_values: Vec<u8>,
    pub observed_chr_values_hex: Vec<String>,
    pub expected_mirror_values: Vec<u8>,
    pub expected_mirror_values_hex: Vec<String>,
    pub observed_mirror_values: Vec<u8>,
    pub observed_mirror_values_hex: Vec<String>,
    pub cycles: u64,
    pub frames: u64,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Mapper4Mmc3EdgeTelemetry {
    pub mapper: u8,
    pub prg_16k_banks: u8,
    pub prg_8k_banks: usize,
    pub chr_8k_banks: u8,
    pub chr_1k_banks: usize,
    pub program_base: u16,
    pub program_base_hex: String,
    pub prg_read_addrs: Vec<u16>,
    pub prg_read_addrs_hex: Vec<String>,
    pub chr_read_addrs: Vec<u16>,
    pub chr_read_addrs_hex: Vec<String>,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub prg_select_writes: Vec<(u8, u8)>,
    pub prg_select_writes_hex: Vec<String>,
    pub chr_select_writes: Vec<(u8, u8)>,
    pub chr_select_writes_hex: Vec<String>,
    pub irq_latches: Vec<u8>,
    pub irq_latches_hex: Vec<String>,
    pub expected_irq_counts: Vec<u8>,
    pub observed_irq_counts: Vec<u8>,
    pub expected_prg_values: Vec<u8>,
    pub expected_prg_values_hex: Vec<String>,
    pub observed_prg_values: Vec<u8>,
    pub observed_prg_values_hex: Vec<String>,
    pub expected_chr_values: Vec<u8>,
    pub expected_chr_values_hex: Vec<String>,
    pub observed_chr_values: Vec<u8>,
    pub observed_chr_values_hex: Vec<String>,
    pub cycles: u64,
    pub frames: u64,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Mapper4Mmc3PrgRamTelemetry {
    pub mapper: u8,
    pub prg_16k_banks: u8,
    pub prg_8k_banks: usize,
    pub chr_8k_banks: u8,
    pub battery_backed: bool,
    pub prg_ram_size: usize,
    pub read_addrs: Vec<u16>,
    pub read_addrs_hex: Vec<String>,
    pub restored_addrs: Vec<u16>,
    pub restored_addrs_hex: Vec<String>,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub expected_values: Vec<u8>,
    pub expected_values_hex: Vec<String>,
    pub observed_values: Vec<u8>,
    pub observed_values_hex: Vec<String>,
    pub sram_snapshot_values: Vec<u8>,
    pub sram_snapshot_values_hex: Vec<String>,
    pub restored_values: Vec<u8>,
    pub restored_values_hex: Vec<String>,
    pub cycles: u64,
    pub frames: u64,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Mapper7AxromTelemetry {
    pub mapper: u8,
    pub prg_banks: u8,
    pub chr_banks: u8,
    pub prg_read_addr: u16,
    pub prg_read_addr_hex: String,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub bank_writes: Vec<u8>,
    pub bank_writes_hex: Vec<String>,
    pub expected_prg_values: Vec<u8>,
    pub expected_prg_values_hex: Vec<String>,
    pub observed_prg_values: Vec<u8>,
    pub observed_prg_values_hex: Vec<String>,
    pub expected_mirror_values: Vec<u8>,
    pub expected_mirror_values_hex: Vec<String>,
    pub observed_mirror_values: Vec<u8>,
    pub observed_mirror_values_hex: Vec<String>,
    pub cycles: u64,
    pub frames: u64,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InputMaskSweepTelemetry {
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub passed_case_count: usize,
    pub failed_case_count: usize,
    pub cases: Vec<InputMaskSweepCaseTelemetry>,
    pub passed: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InputMaskSweepCaseTelemetry {
    pub index: usize,
    pub joypad1_expected_mask: u8,
    pub joypad1_expected_mask_hex: String,
    pub joypad1_observed_mask: u8,
    pub joypad1_observed_mask_hex: String,
    pub joypad2_expected_mask: u8,
    pub joypad2_expected_mask_hex: String,
    pub joypad2_observed_mask: u8,
    pub joypad2_observed_mask_hex: String,
    pub observed_case_count: u8,
    pub cycles: u64,
    pub frames: u64,
    pub passed: bool,
    pub error: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct CpuAddressingMatrixTelemetry {
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub passed: bool,
    pub abs_x_no_cross_result: u8,
    pub abs_x_no_cross_result_hex: String,
    pub abs_x_page_cross_result: u8,
    pub abs_x_page_cross_result_hex: String,
    pub indirect_y_page_cross_result: u8,
    pub indirect_y_page_cross_result_hex: String,
}

#[derive(Debug, Serialize)]
pub struct CpuBranchMatrixTelemetry {
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub expected_mask: u8,
    pub expected_mask_hex: String,
    pub taken_mask: u8,
    pub taken_mask_hex: String,
    pub not_taken_mask: u8,
    pub not_taken_mask_hex: String,
    pub expected_page_cross_result: u8,
    pub expected_page_cross_result_hex: String,
    pub page_cross_result: u8,
    pub page_cross_result_hex: String,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct CpuStackMatrixTelemetry {
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub expected_stack_pointer: u8,
    pub expected_stack_pointer_hex: String,
    pub tsx_result: u8,
    pub tsx_result_hex: String,
    pub pull_result: u8,
    pub pull_result_hex: String,
    pub status_result: u8,
    pub status_result_hex: String,
    pub jsr_result: u8,
    pub jsr_result_hex: String,
    pub final_stack_pointer: u8,
    pub final_stack_pointer_hex: String,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct CpuRmwMatrixTelemetry {
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub passed: bool,
    pub asl_result: u8,
    pub asl_result_hex: String,
    pub rol_result: u8,
    pub rol_result_hex: String,
    pub lsr_result: u8,
    pub lsr_result_hex: String,
    pub ror_result: u8,
    pub ror_result_hex: String,
    pub inc_result: u8,
    pub inc_result_hex: String,
    pub dec_result: u8,
    pub dec_result_hex: String,
}

#[derive(Debug, Serialize)]
pub struct CpuRmwAddressingMatrixTelemetry {
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub passed: bool,
    pub asl_abs_result: u8,
    pub asl_abs_result_hex: String,
    pub rol_abs_x_result: u8,
    pub rol_abs_x_result_hex: String,
    pub lsr_abs_result: u8,
    pub lsr_abs_result_hex: String,
    pub ror_abs_x_result: u8,
    pub ror_abs_x_result_hex: String,
    pub inc_abs_result: u8,
    pub inc_abs_result_hex: String,
    pub dec_abs_x_result: u8,
    pub dec_abs_x_result_hex: String,
}

#[derive(Debug, Serialize)]
pub struct InputPortMatrixTelemetry {
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub passed: bool,
    pub joypad1_high_first: u8,
    pub joypad1_high_first_hex: String,
    pub joypad1_high_second: u8,
    pub joypad1_high_second_hex: String,
    pub joypad2_high_first: u8,
    pub joypad2_high_first_hex: String,
    pub joypad2_high_second: u8,
    pub joypad2_high_second_hex: String,
    pub joypad1_overread_first: u8,
    pub joypad1_overread_first_hex: String,
    pub joypad1_overread_second: u8,
    pub joypad1_overread_second_hex: String,
    pub joypad2_overread_first: u8,
    pub joypad2_overread_first_hex: String,
    pub joypad2_overread_second: u8,
    pub joypad2_overread_second_hex: String,
}

#[derive(Debug, Serialize)]
pub struct ApuStatusMatrixTelemetry {
    pub expected_mask: u8,
    pub expected_mask_hex: String,
    pub observed_mask: u8,
    pub observed_mask_hex: String,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub pulse1_status_bit: bool,
    pub pulse2_status_bit: bool,
    pub triangle_status_bit: bool,
    pub noise_status_bit: bool,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct ApuDmcStatusTelemetry {
    pub expected_bit: u8,
    pub expected_bit_hex: String,
    pub observed_bit: u8,
    pub observed_bit_hex: String,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub dmc_status_bit: bool,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct PpuSpriteZeroHitTelemetry {
    pub expected_status_bit: u8,
    pub expected_status_bit_hex: String,
    pub observed_status_bit: u8,
    pub observed_status_bit_hex: String,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct PpuSpriteOverflowTelemetry {
    pub expected_status_bit: u8,
    pub expected_status_bit_hex: String,
    pub observed_status_bit: u8,
    pub observed_status_bit_hex: String,
    pub false_positive_expected_status_bit: u8,
    pub false_positive_expected_status_bit_hex: String,
    pub false_positive_observed_status_bit: u8,
    pub false_positive_observed_status_bit_hex: String,
    pub false_negative_expected_status_bit: u8,
    pub false_negative_expected_status_bit_hex: String,
    pub false_negative_observed_status_bit: u8,
    pub false_negative_observed_status_bit_hex: String,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub hardware_bug_matrix_passed: bool,
    pub restored_oam_byte_count: u16,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct PpuSpritePriorityTelemetry {
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub front_sample_x: usize,
    pub front_sample_y: usize,
    pub front_expected_color: u32,
    pub front_expected_color_hex: String,
    pub front_observed_color: u32,
    pub front_observed_color_hex: String,
    pub behind_sample_x: usize,
    pub behind_sample_y: usize,
    pub behind_expected_color: u32,
    pub behind_expected_color_hex: String,
    pub behind_observed_color: u32,
    pub behind_observed_color_hex: String,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct PpuScrollSeamTelemetry {
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub scroll_x: u8,
    pub coarse_scroll_x: u8,
    pub scroll_y: u8,
    pub left_sample_x: usize,
    pub left_sample_y: usize,
    pub left_expected_color: u32,
    pub left_expected_color_hex: String,
    pub left_observed_color: u32,
    pub left_observed_color_hex: String,
    pub right_sample_x: usize,
    pub right_sample_y: usize,
    pub right_expected_color: u32,
    pub right_expected_color_hex: String,
    pub right_observed_color: u32,
    pub right_observed_color_hex: String,
    pub coarse_left_sample_x: usize,
    pub coarse_left_sample_y: usize,
    pub coarse_left_expected_color: u32,
    pub coarse_left_expected_color_hex: String,
    pub coarse_left_observed_color: u32,
    pub coarse_left_observed_color_hex: String,
    pub coarse_right_sample_x: usize,
    pub coarse_right_sample_y: usize,
    pub coarse_right_expected_color: u32,
    pub coarse_right_expected_color_hex: String,
    pub coarse_right_observed_color: u32,
    pub coarse_right_observed_color_hex: String,
    pub nametable_wrap_mirroring: String,
    pub nametable_wrap_scroll_x: u8,
    pub nametable_wrap_scroll_y: u8,
    pub nametable_wrap_left_sample_x: usize,
    pub nametable_wrap_left_sample_y: usize,
    pub nametable_wrap_left_expected_color: u32,
    pub nametable_wrap_left_expected_color_hex: String,
    pub nametable_wrap_left_observed_color: u32,
    pub nametable_wrap_left_observed_color_hex: String,
    pub nametable_wrap_right_sample_x: usize,
    pub nametable_wrap_right_sample_y: usize,
    pub nametable_wrap_right_expected_color: u32,
    pub nametable_wrap_right_expected_color_hex: String,
    pub nametable_wrap_right_observed_color: u32,
    pub nametable_wrap_right_observed_color_hex: String,
    pub nametable_wrap_frames: u64,
    pub nametable_wrap_cycles: u64,
    pub nametable_wrap_passed: bool,
    pub nametable_wrap_error: Option<String>,
    pub top_sample_x: usize,
    pub top_sample_y: usize,
    pub top_expected_color: u32,
    pub top_expected_color_hex: String,
    pub top_observed_color: u32,
    pub top_observed_color_hex: String,
    pub bottom_sample_x: usize,
    pub bottom_sample_y: usize,
    pub bottom_expected_color: u32,
    pub bottom_expected_color_hex: String,
    pub bottom_observed_color: u32,
    pub bottom_observed_color_hex: String,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct PpuVblankTimingTelemetry {
    pub test_id: u8,
    pub test_name: Option<&'static str>,
    pub wait_loop_start_cycle: Option<u64>,
    pub wait_loop_start_frame: Option<u64>,
    pub first_nmi_cycle: Option<u64>,
    pub first_nmi_frame: Option<u64>,
    pub first_nmi_latency_cycles: Option<u64>,
    pub first_nmi_expected_min_cycles: u64,
    pub first_nmi_expected_max_cycles: u64,
    pub second_nmi_cycle: Option<u64>,
    pub second_nmi_frame: Option<u64>,
    pub inter_nmi_cycles: Option<u64>,
    pub inter_nmi_expected_min_cycles: u64,
    pub inter_nmi_expected_max_cycles: u64,
    pub observed_nmi_count: u8,
    pub nmi_window_passed: bool,
    pub edge_expected_set_scanline: i16,
    pub edge_expected_set_dot: u16,
    pub edge_expected_clear_scanline: i16,
    pub edge_expected_clear_dot: u16,
    pub edge_expected_set_count: u8,
    pub edge_expected_clear_count: u8,
    pub edge_set_count: u8,
    pub edge_clear_count: u8,
    pub edge_nmi_trigger_count: u8,
    pub edge_first_set_cpu_cycle: Option<u64>,
    pub edge_first_set_frame: Option<u64>,
    pub edge_first_set_ppu_scanline: Option<i16>,
    pub edge_first_set_ppu_dot: Option<u16>,
    pub edge_first_set_ppu_phase: Option<u8>,
    pub edge_first_clear_cpu_cycle: Option<u64>,
    pub edge_first_clear_frame: Option<u64>,
    pub edge_first_clear_ppu_scanline: Option<i16>,
    pub edge_first_clear_ppu_dot: Option<u16>,
    pub edge_first_clear_ppu_phase: Option<u8>,
    pub edge_second_set_cpu_cycle: Option<u64>,
    pub edge_second_set_frame: Option<u64>,
    pub edge_second_set_ppu_scanline: Option<i16>,
    pub edge_second_set_ppu_dot: Option<u16>,
    pub edge_second_set_ppu_phase: Option<u8>,
    pub edge_passed: bool,
    pub passed: bool,
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
    pub oam_dma_transfer_count: usize,
    pub oam_dma_total_active_cycles: u64,
    pub oam_dma_active_cycle_buckets: Vec<u64>,
    pub oam_dma_active_cycle_parities: Vec<&'static str>,
    pub oam_dma_phase_matrix_expected_total_transfers: usize,
    pub oam_dma_phase_matrix_expected_test_transfers: usize,
    pub oam_dma_phase_matrix_test_transfer_count: usize,
    pub oam_dma_phase_matrix_has_even_start: bool,
    pub oam_dma_phase_matrix_has_odd_start: bool,
    pub oam_dma_phase_matrix_passed: bool,
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
    pub dmc_dma_oam_overlap_offsets: Vec<u64>,
    pub dmc_dma_oam_overlap_transfer_indices: Vec<usize>,
    pub dmc_dma_oam_overlap_phase_matrix_transfer_indices: Vec<usize>,
    pub dmc_dma_oam_overlap_phase_matrix_distinct_transfer_count: usize,
    pub dmc_dma_oam_overlap_expected_min_phase_matrix_transfers: usize,
    pub dmc_dma_oam_overlap_burst_train_passed: bool,
    pub dmc_dma_oam_overlap_position_buckets: Vec<&'static str>,
    pub dmc_dma_oam_overlap_covered_position_buckets: Vec<&'static str>,
    pub dmc_dma_oam_overlap_expected_position_buckets: Vec<&'static str>,
    pub dmc_dma_oam_overlap_missing_position_buckets: Vec<&'static str>,
    pub dmc_dma_oam_overlap_expected_min_position_buckets: usize,
    pub dmc_dma_oam_overlap_position_matrix_passed: bool,
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
    pub checksum_hex: String,
    pub expected_checksum: u64,
    pub expected_checksum_hex: String,
    pub checksum_matches_expected: bool,
    pub checksum_validation_enabled: bool,
    pub checksum_validation_reason: String,
    pub unique_colors: usize,
    pub expected_unique_colors: usize,
    pub unique_colors_match_expected: bool,
    pub nonzero_pixels: usize,
    pub expected_nonzero_pixels: usize,
    pub nonzero_pixels_match_expected: bool,
}

#[derive(Debug, Serialize)]
pub struct AudioTelemetry {
    pub sample_count: usize,
    pub expected_min_sample_count: usize,
    pub expected_max_sample_count: usize,
    pub sample_count_passed: bool,
    pub peak_abs: f32,
    pub expected_min_peak_abs: f32,
    pub expected_max_peak_abs: f32,
    pub peak_abs_passed: bool,
    pub rms_abs: f32,
    pub expected_min_rms_abs: f32,
    pub expected_max_rms_abs: f32,
    pub rms_abs_passed: bool,
    pub mean_abs: f32,
    pub expected_min_mean_abs: f32,
    pub expected_max_mean_abs: f32,
    pub mean_abs_passed: bool,
    pub passed: bool,
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
    build_diagnostic_cartridge_from_program_with_flags6(program, labels, 0)
}

fn build_diagnostic_cartridge_from_program_with_flags6(
    program: &[u8],
    labels: &HashMap<String, u16>,
    flags6_mirroring_bits: u8,
) -> Result<Vec<u8>, String> {
    build_diagnostic_cartridge_rom(
        program,
        labels,
        DiagnosticCartridgeRomConfig {
            mapper: DIAGNOSTIC_MAPPER,
            prg_banks: PRG_BANKS,
            flags6_mirroring_bits,
            chr_rom: build_chr_rom(),
            seed_mapper2_prg_sentinels: true,
        },
    )
}

struct DiagnosticCartridgeRomConfig {
    mapper: u8,
    prg_banks: u8,
    flags6_mirroring_bits: u8,
    chr_rom: Vec<u8>,
    seed_mapper2_prg_sentinels: bool,
}

fn build_diagnostic_cartridge_rom(
    program: &[u8],
    labels: &HashMap<String, u16>,
    config: DiagnosticCartridgeRomConfig,
) -> Result<Vec<u8>, String> {
    if program.len() > PRG_BANK_SIZE {
        return Err(format!(
            "diagnostic program is too large: {} bytes > {} bytes",
            program.len(),
            PRG_BANK_SIZE
        ));
    }
    if config.prg_banks == 0 {
        return Err("diagnostic cartridge requires at least one PRG bank".to_string());
    }
    if config.chr_rom.is_empty() || !config.chr_rom.len().is_multiple_of(CHR_BANK_SIZE) {
        return Err(format!(
            "diagnostic CHR ROM size must be a nonzero multiple of {} bytes, got {} bytes",
            CHR_BANK_SIZE,
            config.chr_rom.len()
        ));
    }

    let chr_banks = config.chr_rom.len() / CHR_BANK_SIZE;
    let chr_banks_u8 = u8::try_from(chr_banks)
        .map_err(|_| format!("diagnostic CHR bank count {chr_banks} exceeds iNES limit"))?;
    let prg_size = config.prg_banks as usize * PRG_BANK_SIZE;
    let program_prg_offset = (config.prg_banks as usize - 1) * PRG_BANK_SIZE;

    let mut rom = Vec::with_capacity(16 + prg_size + config.chr_rom.len());
    rom.extend_from_slice(b"NES\x1A");
    rom.push(config.prg_banks);
    rom.push(chr_banks_u8);
    rom.push(((config.mapper & 0x0F) << 4) | (config.flags6_mirroring_bits & 0x09));
    rom.push(config.mapper & 0xF0);
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; prg_size];
    if config.seed_mapper2_prg_sentinels {
        for (bank, sentinel) in MAPPER2_BANK_SENTINELS {
            if *bank < config.prg_banks {
                prg[*bank as usize * PRG_BANK_SIZE] = *sentinel;
            }
        }
        write_prg_cpu_byte_for_banks(
            &mut prg,
            config.prg_banks,
            MAPPER2_FIXED_SENTINEL_ADDR,
            MAPPER2_FIXED_SENTINEL,
        );
    }
    prg[program_prg_offset..program_prg_offset + program.len()].copy_from_slice(program);
    write_vector_for_banks(
        &mut prg,
        config.prg_banks,
        0xFFFA,
        label_addr(labels, "nmi")?,
    );
    write_vector_for_banks(&mut prg, config.prg_banks, 0xFFFC, PROGRAM_BASE);
    write_vector_for_banks(
        &mut prg,
        config.prg_banks,
        0xFFFE,
        label_addr(labels, "irq")?,
    );
    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&config.chr_rom);
    Ok(rom)
}

fn build_ppu_scroll_wrap_variant_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_ppu_scroll_wrap_variant_program_with_labels()?;
    build_diagnostic_cartridge_from_program_with_flags6(&program, &labels, 0x01)
}

fn build_mapper3_chr_bank_variant_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_mapper3_chr_bank_variant_program_with_labels()?;
    build_diagnostic_cartridge_rom(
        &program,
        &labels,
        DiagnosticCartridgeRomConfig {
            mapper: MAPPER3_MAPPER,
            prg_banks: MAPPER3_PRG_BANKS,
            flags6_mirroring_bits: 0,
            chr_rom: build_mapper3_chr_bank_variant_chr_rom(),
            seed_mapper2_prg_sentinels: false,
        },
    )
}

fn build_mapper1_mmc1_variant_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_mapper1_mmc1_variant_program_with_labels()?;
    if program.len() > PRG_BANK_SIZE {
        return Err(format!(
            "Mapper 1 diagnostic program is too large: {} bytes > {} bytes",
            program.len(),
            PRG_BANK_SIZE
        ));
    }

    let prg_size = MAPPER1_PRG_BANKS as usize * PRG_BANK_SIZE;
    let mut rom = Vec::with_capacity(16 + prg_size + MAPPER1_CHR_8K_BANKS as usize * CHR_BANK_SIZE);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(MAPPER1_PRG_BANKS);
    rom.push(MAPPER1_CHR_8K_BANKS);
    rom.push((MAPPER1_MAPPER & 0x0F) << 4);
    rom.push(MAPPER1_MAPPER & 0xF0);
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; prg_size];
    for (bank, sentinel) in MAPPER1_PRG_BANK_SENTINELS.iter().enumerate() {
        prg[bank * PRG_BANK_SIZE + (MAPPER1_PRG_SWITCH_ADDR - 0x8000) as usize] = *sentinel;
    }
    write_prg_cpu_byte_for_banks(
        &mut prg,
        MAPPER1_PRG_BANKS,
        MAPPER1_PRG_FIXED_ADDR,
        MAPPER1_PRG_EXPECTED_VALUES[4],
    );
    let program_offset = (MAPPER1_PRG_BANKS as usize - 1) * PRG_BANK_SIZE;
    prg[program_offset..program_offset + program.len()].copy_from_slice(&program);
    write_vector_for_banks(
        &mut prg,
        MAPPER1_PRG_BANKS,
        0xFFFA,
        label_addr(&labels, "nmi")?,
    );
    write_vector_for_banks(&mut prg, MAPPER1_PRG_BANKS, 0xFFFC, PROGRAM_BASE);
    write_vector_for_banks(
        &mut prg,
        MAPPER1_PRG_BANKS,
        0xFFFE,
        label_addr(&labels, "irq")?,
    );

    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&build_mapper1_mmc1_chr_rom());
    Ok(rom)
}

fn build_mapper1_mmc1_chr_rom() -> Vec<u8> {
    let mut chr = vec![0; MAPPER1_CHR_8K_BANKS as usize * CHR_BANK_SIZE];
    for (bank, sentinel) in MAPPER1_CHR_BANK_SENTINELS.iter().enumerate() {
        chr[bank * 0x1000 + (MAPPER1_CHR_LOW_READ_ADDR & 0x0FFF) as usize] = *sentinel;
    }
    chr
}

fn build_mapper1_mmc1_32k_prg_variant_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_mapper1_mmc1_32k_prg_variant_program_with_labels()?;
    if program.len() > PRG_BANK_SIZE {
        return Err(format!(
            "Mapper 1 32 KiB PRG diagnostic program is too large: {} bytes > {} bytes",
            program.len(),
            PRG_BANK_SIZE
        ));
    }

    let prg_size = MAPPER1_PRG_BANKS as usize * PRG_BANK_SIZE;
    let mut rom = Vec::with_capacity(16 + prg_size + MAPPER1_CHR_8K_BANKS as usize * CHR_BANK_SIZE);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(MAPPER1_PRG_BANKS);
    rom.push(MAPPER1_CHR_8K_BANKS);
    rom.push((MAPPER1_MAPPER & 0x0F) << 4);
    rom.push(MAPPER1_MAPPER & 0xF0);
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; prg_size];
    let low_offset = (MAPPER1_32K_LOW_READ_ADDR - 0x8000) as usize;
    let high_offset = (MAPPER1_32K_HIGH_READ_ADDR - 0xC000) as usize;
    prg[low_offset] = MAPPER1_32K_BANK_SENTINELS[0];
    prg[PRG_BANK_SIZE + high_offset] = MAPPER1_32K_BANK_SENTINELS[1];
    prg[2 * PRG_BANK_SIZE + low_offset] = MAPPER1_32K_BANK_SENTINELS[2];
    prg[3 * PRG_BANK_SIZE + high_offset] = MAPPER1_32K_BANK_SENTINELS[3];

    for bank in [1usize, 3usize] {
        let program_offset = bank * PRG_BANK_SIZE + (PROGRAM_BASE - 0xC000) as usize;
        prg[program_offset..program_offset + program.len()].copy_from_slice(&program);
    }
    write_vector_for_banks(
        &mut prg,
        MAPPER1_PRG_BANKS,
        0xFFFA,
        label_addr(&labels, "nmi")?,
    );
    write_vector_for_banks(&mut prg, MAPPER1_PRG_BANKS, 0xFFFC, PROGRAM_BASE);
    write_vector_for_banks(
        &mut prg,
        MAPPER1_PRG_BANKS,
        0xFFFE,
        label_addr(&labels, "irq")?,
    );

    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&build_mapper1_mmc1_chr_rom());
    Ok(rom)
}

fn build_mapper4_mmc3_variant_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_mapper4_mmc3_variant_program_with_labels()?;
    if program.len() > 0x2000 {
        return Err(format!(
            "Mapper 4 diagnostic program is too large for fixed $C000-$DFFF execution: {} bytes > {} bytes",
            program.len(),
            0x2000
        ));
    }

    let prg_size = MAPPER4_PRG_16K_BANKS as usize * PRG_BANK_SIZE;
    let mut rom = Vec::with_capacity(16 + prg_size + MAPPER4_CHR_8K_BANKS as usize * CHR_BANK_SIZE);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(MAPPER4_PRG_16K_BANKS);
    rom.push(MAPPER4_CHR_8K_BANKS);
    rom.push((MAPPER4_MAPPER & 0x0F) << 4);
    rom.push(MAPPER4_MAPPER & 0xF0);
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; prg_size];
    for (bank, addr, value) in MAPPER4_PRG_BANK_SENTINELS {
        write_mapper4_8k_cpu_byte(&mut prg, bank, addr, value);
    }
    let program_offset = (MAPPER4_PRG_16K_BANKS as usize - 1) * PRG_BANK_SIZE;
    prg[program_offset..program_offset + program.len()].copy_from_slice(&program);
    write_vector_for_banks(
        &mut prg,
        MAPPER4_PRG_16K_BANKS,
        0xFFFA,
        label_addr(&labels, "nmi")?,
    );
    write_vector_for_banks(&mut prg, MAPPER4_PRG_16K_BANKS, 0xFFFC, PROGRAM_BASE);
    write_vector_for_banks(
        &mut prg,
        MAPPER4_PRG_16K_BANKS,
        0xFFFE,
        label_addr(&labels, "irq")?,
    );

    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&build_mapper4_mmc3_chr_rom());
    Ok(rom)
}

fn build_mapper4_mmc3_chr_rom() -> Vec<u8> {
    let mut chr = vec![0; MAPPER4_CHR_8K_BANKS as usize * CHR_BANK_SIZE];
    for (bank, sentinel) in MAPPER4_CHR_BANK_SENTINELS.iter().enumerate() {
        chr[bank * 0x0400 + (MAPPER4_CHR_READ_ADDRS[0] & 0x03FF) as usize] = *sentinel;
    }
    chr
}

fn build_mapper4_mmc3_edge_variant_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_mapper4_mmc3_edge_variant_program_with_labels()?;
    if program.len() > 0x1FFA {
        return Err(format!(
            "Mapper 4 edge diagnostic program is too large for fixed $E000-$FFF9 execution: {} bytes > {} bytes",
            program.len(),
            0x1FFA
        ));
    }

    let prg_size = MAPPER4_PRG_16K_BANKS as usize * PRG_BANK_SIZE;
    let mut rom = Vec::with_capacity(16 + prg_size + MAPPER4_CHR_8K_BANKS as usize * CHR_BANK_SIZE);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(MAPPER4_PRG_16K_BANKS);
    rom.push(MAPPER4_CHR_8K_BANKS);
    rom.push((MAPPER4_MAPPER & 0x0F) << 4);
    rom.push(MAPPER4_MAPPER & 0xF0);
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; prg_size];
    for (bank, addr, value) in MAPPER4_EDGE_PRG_BANK_SENTINELS {
        write_mapper4_8k_cpu_byte(&mut prg, bank, addr, value);
    }
    let program_bank = MAPPER4_PRG_8K_BANKS - 1;
    let program_offset = program_bank * 0x2000 + (MAPPER4_EDGE_PROGRAM_BASE & 0x1FFF) as usize;
    prg[program_offset..program_offset + program.len()].copy_from_slice(&program);
    write_mapper4_8k_cpu_vector(&mut prg, program_bank, 0xFFFA, label_addr(&labels, "nmi")?);
    write_mapper4_8k_cpu_vector(
        &mut prg,
        program_bank,
        0xFFFC,
        label_addr(&labels, "reset")?,
    );
    write_mapper4_8k_cpu_vector(&mut prg, program_bank, 0xFFFE, label_addr(&labels, "irq")?);

    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&build_mapper4_mmc3_chr_rom());
    Ok(rom)
}

fn build_mapper4_mmc3_prg_ram_variant_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_mapper4_mmc3_prg_ram_variant_program_with_labels()?;
    if program.len() > 0x2000 {
        return Err(format!(
            "Mapper 4 PRG RAM diagnostic program is too large for fixed $C000-$DFFF execution: {} bytes > {} bytes",
            program.len(),
            0x2000
        ));
    }

    let prg_size = MAPPER4_PRG_16K_BANKS as usize * PRG_BANK_SIZE;
    let mut rom = Vec::with_capacity(16 + prg_size + MAPPER4_CHR_8K_BANKS as usize * CHR_BANK_SIZE);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(MAPPER4_PRG_16K_BANKS);
    rom.push(MAPPER4_CHR_8K_BANKS);
    rom.push(((MAPPER4_MAPPER & 0x0F) << 4) | 0x02);
    rom.push(MAPPER4_MAPPER & 0xF0);
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; prg_size];
    let program_offset = (MAPPER4_PRG_16K_BANKS as usize - 1) * PRG_BANK_SIZE;
    prg[program_offset..program_offset + program.len()].copy_from_slice(&program);
    write_vector_for_banks(
        &mut prg,
        MAPPER4_PRG_16K_BANKS,
        0xFFFA,
        label_addr(&labels, "nmi")?,
    );
    write_vector_for_banks(&mut prg, MAPPER4_PRG_16K_BANKS, 0xFFFC, PROGRAM_BASE);
    write_vector_for_banks(
        &mut prg,
        MAPPER4_PRG_16K_BANKS,
        0xFFFE,
        label_addr(&labels, "irq")?,
    );

    rom.extend_from_slice(&prg);
    rom.extend_from_slice(&build_mapper4_mmc3_chr_rom());
    Ok(rom)
}

fn build_mapper7_axrom_variant_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_mapper7_axrom_variant_program_with_labels()?;
    if program.len() > PRG_BANK_SIZE {
        return Err(format!(
            "Mapper 7 diagnostic program is too large: {} bytes > {} bytes",
            program.len(),
            PRG_BANK_SIZE
        ));
    }

    let prg_size = MAPPER7_PRG_BANKS as usize * PRG_BANK_SIZE;
    let mut rom = Vec::with_capacity(16 + prg_size);
    rom.extend_from_slice(b"NES\x1A");
    rom.push(MAPPER7_PRG_BANKS);
    rom.push(MAPPER7_CHR_BANKS);
    rom.push((MAPPER7_MAPPER & 0x0F) << 4);
    rom.push(MAPPER7_MAPPER & 0xF0);
    rom.extend_from_slice(&[0; 8]);

    let mut prg = vec![0xEA; prg_size];
    for (bank, expected_value) in MAPPER7_PRG_EXPECTED_VALUES
        .iter()
        .enumerate()
        .take(MAPPER7_32K_BANKS)
    {
        let bank_base = bank * 0x8000;
        prg[bank_base + (MAPPER7_PRG_SENTINEL_ADDR - 0x8000) as usize] = *expected_value;
        let program_offset = bank_base + (PROGRAM_BASE - 0x8000) as usize;
        prg[program_offset..program_offset + program.len()].copy_from_slice(&program);
        write_mapper7_32k_vector(&mut prg, bank, 0xFFFA, label_addr(&labels, "nmi")?);
        write_mapper7_32k_vector(&mut prg, bank, 0xFFFC, PROGRAM_BASE);
        write_mapper7_32k_vector(&mut prg, bank, 0xFFFE, label_addr(&labels, "irq")?);
    }

    rom.extend_from_slice(&prg);
    Ok(rom)
}

fn build_input_mask_sweep_variant_cartridge() -> Result<Vec<u8>, String> {
    let (program, labels) = build_input_mask_sweep_variant_program_with_labels()?;
    build_diagnostic_cartridge_from_program_with_flags6(&program, &labels, 0)
}

fn build_mapper1_mmc1_variant_program_with_labels(
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new();

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017);
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(MAPPER1_MMC1_TEST_ID);
    program.asm.sta_zp(CURRENT_TEST_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);
    for offset in 0..MAPPER1_PRG_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER1_MMC1_PRG_OBSERVED_BASE_ADDR + offset as u16);
    }
    for offset in 0..MAPPER1_CHR_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER1_MMC1_CHR_OBSERVED_BASE_ADDR + offset as u16);
    }
    for offset in 0..MAPPER1_MIRROR_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER1_MMC1_MIRROR_OBSERVED_BASE_ADDR + offset as u16);
    }
    program.asm.sta_abs(0x2000);
    program.asm.sta_abs(0x2001);

    program.asm.lda_imm(0x80);
    program.asm.sta_abs(0x8000);
    program.write_mmc1_register(0x8000, 0x1F);
    program.write_mmc1_register(0xE000, MAPPER1_PRG_BANK_WRITES[0]);
    program.asm.lda_abs(MAPPER1_PRG_SWITCH_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_PRG_OBSERVED_BASE_ADDR);
    program.expect_a_eq(MAPPER1_PRG_EXPECTED_VALUES[0], 0xC0);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);

    program.write_mmc1_register_bits(0xE000, MAPPER1_PRG_BANK_WRITES[1], 4);
    program.asm.lda_abs(MAPPER1_PRG_SWITCH_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_PRG_OBSERVED_BASE_ADDR + 1);
    program.expect_a_eq(MAPPER1_PRG_EXPECTED_VALUES[1], 0xC1);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);
    program.write_mmc1_register_bits(0xE000, MAPPER1_PRG_BANK_WRITES[1] >> 4, 1);
    program.asm.lda_abs(MAPPER1_PRG_SWITCH_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_PRG_OBSERVED_BASE_ADDR + 2);
    program.expect_a_eq(MAPPER1_PRG_EXPECTED_VALUES[2], 0xC2);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);

    program.write_mmc1_register(0xE000, MAPPER1_PRG_BANK_WRITES[2]);
    program.asm.lda_abs(MAPPER1_PRG_SWITCH_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_PRG_OBSERVED_BASE_ADDR + 3);
    program.expect_a_eq(MAPPER1_PRG_EXPECTED_VALUES[3], 0xC3);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);
    program.asm.lda_abs(MAPPER1_PRG_FIXED_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_PRG_OBSERVED_BASE_ADDR + 4);
    program.expect_a_eq(MAPPER1_PRG_EXPECTED_VALUES[4], 0xC4);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);

    program.write_mmc1_register(0xA000, MAPPER1_CHR_BANK_WRITES[0]);
    program.write_mmc1_register(0xC000, MAPPER1_CHR_BANK_WRITES[1]);
    program.read_ppu_data_into_a(MAPPER1_CHR_LOW_READ_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_CHR_OBSERVED_BASE_ADDR);
    program.expect_a_eq(MAPPER1_CHR_EXPECTED_VALUES[0], 0xC5);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);
    program.read_ppu_data_into_a(MAPPER1_CHR_HIGH_READ_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_CHR_OBSERVED_BASE_ADDR + 1);
    program.expect_a_eq(MAPPER1_CHR_EXPECTED_VALUES[1], 0xC6);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);

    program.write_mmc1_register(0xA000, MAPPER1_CHR_BANK_WRITES[2]);
    program.write_mmc1_register(0xC000, MAPPER1_CHR_BANK_WRITES[3]);
    program.read_ppu_data_into_a(MAPPER1_CHR_LOW_READ_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_CHR_OBSERVED_BASE_ADDR + 2);
    program.expect_a_eq(MAPPER1_CHR_EXPECTED_VALUES[2], 0xC7);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);
    program.read_ppu_data_into_a(MAPPER1_CHR_HIGH_READ_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_CHR_OBSERVED_BASE_ADDR + 3);
    program.expect_a_eq(MAPPER1_CHR_EXPECTED_VALUES[3], 0xC8);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);

    program.write_mmc1_register(0x8000, 0x1C);
    program.write_ppu_data(0x2000, MAPPER1_MIRROR_EXPECTED_VALUES[0]);
    program.read_ppu_data_into_a(0x2400);
    program.asm.sta_abs(MAPPER1_MMC1_MIRROR_OBSERVED_BASE_ADDR);
    program.expect_a_eq(MAPPER1_MIRROR_EXPECTED_VALUES[0], 0xC9);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);

    program.write_mmc1_register(0x8000, 0x1D);
    program.write_ppu_data(0x2000, MAPPER1_MIRROR_EXPECTED_VALUES[1]);
    program.read_ppu_data_into_a(0x2400);
    program
        .asm
        .sta_abs(MAPPER1_MMC1_MIRROR_OBSERVED_BASE_ADDR + 1);
    program.expect_a_eq(MAPPER1_MIRROR_EXPECTED_VALUES[1], 0xCA);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);

    program.write_mmc1_register(0x8000, 0x1C);
    program.read_ppu_data_into_a(0x2400);
    program
        .asm
        .sta_abs(MAPPER1_MMC1_MIRROR_OBSERVED_BASE_ADDR + 2);
    program.expect_a_eq(MAPPER1_MIRROR_EXPECTED_VALUES[2], 0xCB);
    program.increment_abs(MAPPER1_MMC1_CASE_COUNT_ADDR);

    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("fail")?;
    program.asm.lda_imm(STATUS_FAIL);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("irq")?;
    program.asm.rti();
    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

fn build_mapper1_mmc1_32k_prg_variant_program_with_labels(
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new();

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017);
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(MAPPER1_MMC1_32K_PRG_TEST_ID);
    program.asm.sta_zp(CURRENT_TEST_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);
    program.asm.sta_abs(MAPPER1_MMC1_32K_PRG_CASE_COUNT_ADDR);
    for offset in 0..MAPPER1_32K_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER1_MMC1_32K_PRG_OBSERVED_BASE_ADDR + offset as u16);
    }
    program.asm.sta_abs(0x2000);
    program.asm.sta_abs(0x2001);

    program.asm.lda_imm(0x80);
    program.asm.sta_abs(0x8000);
    program.write_mmc1_register(0x8000, MAPPER1_32K_CONTROL_WRITES[0]);

    let mut observed_index = 0usize;
    for bank_write in MAPPER1_32K_PRG_BANK_WRITES.iter().copied().take(4) {
        program.write_mmc1_register(0xE000, bank_write);
        program.asm.lda_abs(MAPPER1_32K_LOW_READ_ADDR);
        program
            .asm
            .sta_abs(MAPPER1_MMC1_32K_PRG_OBSERVED_BASE_ADDR + observed_index as u16);
        program.expect_a_eq(
            MAPPER1_32K_EXPECTED_VALUES[observed_index],
            0xD0 + observed_index as u8,
        );
        program.increment_abs(MAPPER1_MMC1_32K_PRG_CASE_COUNT_ADDR);
        observed_index += 1;

        program.asm.lda_abs(MAPPER1_32K_HIGH_READ_ADDR);
        program
            .asm
            .sta_abs(MAPPER1_MMC1_32K_PRG_OBSERVED_BASE_ADDR + observed_index as u16);
        program.expect_a_eq(
            MAPPER1_32K_EXPECTED_VALUES[observed_index],
            0xD0 + observed_index as u8,
        );
        program.increment_abs(MAPPER1_MMC1_32K_PRG_CASE_COUNT_ADDR);
        observed_index += 1;
    }

    program.write_mmc1_register(0x8000, MAPPER1_32K_CONTROL_WRITES[1]);
    program.write_mmc1_register(0xE000, MAPPER1_32K_PRG_BANK_WRITES[4]);
    program.asm.lda_abs(MAPPER1_32K_LOW_READ_ADDR);
    program
        .asm
        .sta_abs(MAPPER1_MMC1_32K_PRG_OBSERVED_BASE_ADDR + observed_index as u16);
    program.expect_a_eq(
        MAPPER1_32K_EXPECTED_VALUES[observed_index],
        0xD0 + observed_index as u8,
    );
    program.increment_abs(MAPPER1_MMC1_32K_PRG_CASE_COUNT_ADDR);
    observed_index += 1;

    program.asm.lda_abs(MAPPER1_32K_HIGH_READ_ADDR);
    program
        .asm
        .sta_abs(MAPPER1_MMC1_32K_PRG_OBSERVED_BASE_ADDR + observed_index as u16);
    program.expect_a_eq(
        MAPPER1_32K_EXPECTED_VALUES[observed_index],
        0xD0 + observed_index as u8,
    );
    program.increment_abs(MAPPER1_MMC1_32K_PRG_CASE_COUNT_ADDR);

    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("fail")?;
    program.asm.lda_imm(STATUS_FAIL);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("irq")?;
    program.asm.rti();
    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

fn build_mapper3_chr_bank_variant_program_with_labels(
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new();

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017);
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(MAPPER3_CHR_BANK_TEST_ID);
    program.asm.sta_zp(CURRENT_TEST_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);
    program.asm.sta_abs(MAPPER3_CHR_BANK_CASE_COUNT_ADDR);
    for offset in 0..MAPPER3_CHR_BANK_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER3_CHR_BANK_OBSERVED_BASE_ADDR + offset as u16);
    }
    program.asm.sta_abs(0x2000);
    program.asm.sta_abs(0x2001);

    for (case_index, (&bank, &expected)) in MAPPER3_CHR_BANK_EXPECTED_BANKS
        .iter()
        .zip(MAPPER3_CHR_BANK_EXPECTED_VALUES.iter())
        .enumerate()
    {
        program.asm.lda_imm(bank);
        program.asm.sta_abs(0x8000);
        program.read_ppu_data_into_a(MAPPER3_CHR_READ_ADDR);
        program
            .asm
            .sta_abs(MAPPER3_CHR_BANK_OBSERVED_BASE_ADDR + case_index as u16);
        program.expect_a_eq(expected, 0x90 + case_index as u8);
        program.asm.lda_abs(MAPPER3_CHR_BANK_CASE_COUNT_ADDR);
        program.asm.clc();
        program.asm.adc_imm(0x01);
        program.asm.sta_abs(MAPPER3_CHR_BANK_CASE_COUNT_ADDR);
    }

    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("fail")?;
    program.asm.lda_imm(STATUS_FAIL);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("irq")?;
    program.asm.rti();
    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

fn build_mapper4_mmc3_variant_program_with_labels(
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new();

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017);
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(MAPPER4_MMC3_TEST_ID);
    program.asm.sta_zp(CURRENT_TEST_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_CASE_COUNT_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_IRQ_COUNT_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_IRQ_OBSERVED_ADDR);
    for offset in 0..MAPPER4_PRG_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER4_MMC3_PRG_OBSERVED_BASE_ADDR + offset as u16);
    }
    for offset in 0..MAPPER4_CHR_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER4_MMC3_CHR_OBSERVED_BASE_ADDR + offset as u16);
    }
    for offset in 0..MAPPER4_MIRROR_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER4_MMC3_MIRROR_OBSERVED_BASE_ADDR + offset as u16);
    }
    program.asm.sta_abs(0x2000);
    program.asm.sta_abs(0x2001);
    program.asm.sta_abs(0xE000);

    for &(register, value) in &MAPPER4_PRG_REGISTER_WRITES {
        program.write_mmc3_bank_register(register, value);
    }
    program.asm.lda_abs(MAPPER4_PRG_R6_READ_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_PRG_OBSERVED_BASE_ADDR);
    program.expect_a_eq(MAPPER4_PRG_EXPECTED_VALUES[0], 0xD0);
    program.increment_abs(MAPPER4_MMC3_CASE_COUNT_ADDR);
    program.asm.lda_abs(MAPPER4_PRG_R7_READ_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_PRG_OBSERVED_BASE_ADDR + 1);
    program.expect_a_eq(MAPPER4_PRG_EXPECTED_VALUES[1], 0xD1);
    program.increment_abs(MAPPER4_MMC3_CASE_COUNT_ADDR);
    program.asm.lda_abs(MAPPER4_PRG_FIXED_READ_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_PRG_OBSERVED_BASE_ADDR + 2);
    program.expect_a_eq(MAPPER4_PRG_EXPECTED_VALUES[2], 0xD2);
    program.increment_abs(MAPPER4_MMC3_CASE_COUNT_ADDR);

    for &(register, value) in &MAPPER4_CHR_REGISTER_WRITES {
        program.write_mmc3_bank_register(register, value);
    }
    for (index, (&addr, &expected)) in MAPPER4_CHR_READ_ADDRS
        .iter()
        .zip(MAPPER4_CHR_EXPECTED_VALUES.iter())
        .enumerate()
    {
        program.read_ppu_data_into_a(addr);
        program
            .asm
            .sta_abs(MAPPER4_MMC3_CHR_OBSERVED_BASE_ADDR + index as u16);
        program.expect_a_eq(expected, 0xD3 + index as u8);
        program.increment_abs(MAPPER4_MMC3_CASE_COUNT_ADDR);
    }

    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0xA000);
    program.write_ppu_data(0x2000, MAPPER4_MIRROR_EXPECTED_VALUES[0]);
    program.read_ppu_data_into_a(0x2800);
    program.asm.sta_abs(MAPPER4_MMC3_MIRROR_OBSERVED_BASE_ADDR);
    program.expect_a_eq(MAPPER4_MIRROR_EXPECTED_VALUES[0], 0xD8);
    program.increment_abs(MAPPER4_MMC3_CASE_COUNT_ADDR);

    program.asm.lda_imm(0x01);
    program.asm.sta_abs(0xA000);
    program.write_ppu_data(0x2000, MAPPER4_MIRROR_EXPECTED_VALUES[1]);
    program.read_ppu_data_into_a(0x2400);
    program
        .asm
        .sta_abs(MAPPER4_MMC3_MIRROR_OBSERVED_BASE_ADDR + 1);
    program.expect_a_eq(MAPPER4_MIRROR_EXPECTED_VALUES[1], 0xD9);
    program.increment_abs(MAPPER4_MMC3_CASE_COUNT_ADDR);

    program.asm.lda_imm(MAPPER4_IRQ_LATCH);
    program.asm.sta_abs(0xC000);
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0xC001);
    program.asm.sta_abs(0xE001);
    program.asm.lda_imm(0x08);
    program.asm.sta_abs(0x2001);
    program.asm.cli();
    program.asm.label("wait_irq")?;
    program.asm.lda_abs(MAPPER4_MMC3_IRQ_COUNT_ADDR);
    program.asm.cmp_imm(MAPPER4_EXPECTED_IRQ_COUNT);
    program.asm.bne("wait_irq");
    program.asm.sei();
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0x2001);
    program.asm.lda_abs(MAPPER4_MMC3_IRQ_COUNT_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_IRQ_OBSERVED_ADDR);
    program.expect_a_eq(MAPPER4_EXPECTED_IRQ_COUNT, 0xDA);
    program.increment_abs(MAPPER4_MMC3_CASE_COUNT_ADDR);

    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("fail")?;
    program.asm.lda_imm(STATUS_FAIL);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("irq")?;
    program.asm.sei();
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0xE000);
    program.asm.lda_abs(MAPPER4_MMC3_IRQ_COUNT_ADDR);
    program.asm.clc();
    program.asm.adc_imm(0x01);
    program.asm.sta_abs(MAPPER4_MMC3_IRQ_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

fn build_mapper4_mmc3_edge_variant_program_with_labels(
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new_at(MAPPER4_EDGE_PROGRAM_BASE);

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017);
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(MAPPER4_MMC3_EDGE_TEST_ID);
    program.asm.sta_zp(CURRENT_TEST_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_EDGE_CASE_COUNT_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_EDGE_IRQ_COUNT_ADDR);
    for offset in 0..MAPPER4_EDGE_PRG_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER4_MMC3_EDGE_PRG_OBSERVED_BASE_ADDR + offset as u16);
    }
    for offset in 0..MAPPER4_EDGE_CHR_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER4_MMC3_EDGE_CHR_OBSERVED_BASE_ADDR + offset as u16);
    }
    for offset in 0..MAPPER4_EDGE_EXPECTED_IRQ_COUNTS.len() {
        program
            .asm
            .sta_abs(MAPPER4_MMC3_EDGE_IRQ_OBSERVED_BASE_ADDR + offset as u16);
    }
    program.asm.sta_abs(0x2000);
    program.asm.sta_abs(0x2001);
    program.asm.sta_abs(0xE000);

    for &(select, value) in &MAPPER4_EDGE_PRG_SELECT_WRITES {
        program.write_mmc3_select_register(select, value);
    }
    for (index, (&addr, &expected)) in MAPPER4_EDGE_PRG_READ_ADDRS
        .iter()
        .zip(MAPPER4_EDGE_PRG_EXPECTED_VALUES.iter())
        .enumerate()
    {
        program.asm.lda_abs(addr);
        program
            .asm
            .sta_abs(MAPPER4_MMC3_EDGE_PRG_OBSERVED_BASE_ADDR + index as u16);
        program.expect_a_eq(expected, 0xDB + index as u8);
        program.increment_abs(MAPPER4_MMC3_EDGE_CASE_COUNT_ADDR);
    }

    for &(select, value) in &MAPPER4_EDGE_CHR_SELECT_WRITES {
        program.write_mmc3_select_register(select, value);
    }
    for (index, (&addr, &expected)) in MAPPER4_EDGE_CHR_READ_ADDRS
        .iter()
        .zip(MAPPER4_EDGE_CHR_EXPECTED_VALUES.iter())
        .enumerate()
    {
        program.read_ppu_data_into_a(addr);
        program
            .asm
            .sta_abs(MAPPER4_MMC3_EDGE_CHR_OBSERVED_BASE_ADDR + index as u16);
        program.expect_a_eq(expected, 0xE0 + index as u8);
        program.increment_abs(MAPPER4_MMC3_EDGE_CASE_COUNT_ADDR);
    }

    program.asm.lda_imm(MAPPER4_EDGE_IRQ_LATCHES[0]);
    program.asm.sta_abs(0xC000);
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0xC001);
    program.asm.sta_abs(0xE001);
    program.asm.lda_imm(0x08);
    program.asm.sta_abs(0x2001);
    program.asm.cli();
    program.asm.label("wait_irq_edge_first")?;
    program.asm.lda_abs(MAPPER4_MMC3_EDGE_IRQ_COUNT_ADDR);
    program.asm.cmp_imm(MAPPER4_EDGE_EXPECTED_IRQ_COUNTS[0]);
    program.asm.bne("wait_irq_edge_first");
    program.asm.sei();
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0x2001);
    program.asm.lda_abs(MAPPER4_MMC3_EDGE_IRQ_COUNT_ADDR);
    program
        .asm
        .sta_abs(MAPPER4_MMC3_EDGE_IRQ_OBSERVED_BASE_ADDR);
    program.expect_a_eq(MAPPER4_EDGE_EXPECTED_IRQ_COUNTS[0], 0xE8);
    program.increment_abs(MAPPER4_MMC3_EDGE_CASE_COUNT_ADDR);

    program.asm.lda_imm(MAPPER4_EDGE_IRQ_LATCHES[1]);
    program.asm.sta_abs(0xC000);
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0xC001);
    program.asm.sta_abs(0xE001);
    program.asm.lda_imm(0x08);
    program.asm.sta_abs(0x2001);
    program.asm.cli();
    program.asm.label("wait_irq_edge_zero_latch")?;
    program.asm.lda_abs(MAPPER4_MMC3_EDGE_IRQ_COUNT_ADDR);
    program.asm.cmp_imm(MAPPER4_EDGE_EXPECTED_IRQ_COUNTS[1]);
    program.asm.bne("wait_irq_edge_zero_latch");
    program.asm.sei();
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0x2001);
    program.asm.lda_abs(MAPPER4_MMC3_EDGE_IRQ_COUNT_ADDR);
    program
        .asm
        .sta_abs(MAPPER4_MMC3_EDGE_IRQ_OBSERVED_BASE_ADDR + 1);
    program.expect_a_eq(MAPPER4_EDGE_EXPECTED_IRQ_COUNTS[1], 0xE9);
    program.increment_abs(MAPPER4_MMC3_EDGE_CASE_COUNT_ADDR);

    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("fail")?;
    program.asm.lda_imm(STATUS_FAIL);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("irq")?;
    program.asm.sei();
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0xE000);
    program.asm.lda_abs(MAPPER4_MMC3_EDGE_IRQ_COUNT_ADDR);
    program.asm.clc();
    program.asm.adc_imm(0x01);
    program.asm.sta_abs(MAPPER4_MMC3_EDGE_IRQ_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

fn build_mapper4_mmc3_prg_ram_variant_program_with_labels(
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new();

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017);
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(MAPPER4_MMC3_PRG_RAM_TEST_ID);
    program.asm.sta_zp(CURRENT_TEST_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);
    program.asm.sta_abs(MAPPER4_MMC3_PRG_RAM_CASE_COUNT_ADDR);
    for offset in 0..MAPPER4_PRG_RAM_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER4_MMC3_PRG_RAM_OBSERVED_BASE_ADDR + offset as u16);
    }
    program.asm.sta_abs(0x2000);
    program.asm.sta_abs(0x2001);
    program.asm.sta_abs(0xE000);

    for (index, (&addr, &expected)) in MAPPER4_PRG_RAM_READ_ADDRS
        .iter()
        .zip(MAPPER4_PRG_RAM_EXPECTED_VALUES.iter())
        .enumerate()
    {
        program.asm.lda_imm(expected);
        program.asm.sta_abs(addr);
        program.asm.lda_abs(addr);
        program
            .asm
            .sta_abs(MAPPER4_MMC3_PRG_RAM_OBSERVED_BASE_ADDR + index as u16);
        program.expect_a_eq(expected, 0xEA + index as u8);
        program.increment_abs(MAPPER4_MMC3_PRG_RAM_CASE_COUNT_ADDR);
    }

    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("fail")?;
    program.asm.lda_imm(STATUS_FAIL);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("irq")?;
    program.asm.rti();
    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

fn build_mapper7_axrom_variant_program_with_labels(
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new();

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017);
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(MAPPER7_AXROM_TEST_ID);
    program.asm.sta_zp(CURRENT_TEST_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);
    program.asm.sta_abs(MAPPER7_AXROM_CASE_COUNT_ADDR);
    for offset in 0..MAPPER7_PRG_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER7_AXROM_PRG_OBSERVED_BASE_ADDR + offset as u16);
    }
    for offset in 0..MAPPER7_MIRROR_EXPECTED_VALUES.len() {
        program
            .asm
            .sta_abs(MAPPER7_AXROM_MIRROR_OBSERVED_BASE_ADDR + offset as u16);
    }
    program.asm.sta_abs(0x2000);
    program.asm.sta_abs(0x2001);

    for (case_index, (&bank_write, &expected)) in MAPPER7_PRG_BANK_WRITES
        .iter()
        .zip(MAPPER7_PRG_EXPECTED_VALUES.iter())
        .enumerate()
    {
        program.asm.lda_imm(bank_write);
        program.asm.sta_abs(0x8000);
        program.asm.lda_abs(MAPPER7_PRG_SENTINEL_ADDR);
        program
            .asm
            .sta_abs(MAPPER7_AXROM_PRG_OBSERVED_BASE_ADDR + case_index as u16);
        program.expect_a_eq(expected, 0xB0 + case_index as u8);
        program.increment_abs(MAPPER7_AXROM_CASE_COUNT_ADDR);
    }

    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0x8000);
    program.write_ppu_data(0x2000, MAPPER7_MIRROR_EXPECTED_VALUES[0]);
    program.read_ppu_data_into_a(0x2400);
    program.asm.sta_abs(MAPPER7_AXROM_MIRROR_OBSERVED_BASE_ADDR);
    program.expect_a_eq(MAPPER7_MIRROR_EXPECTED_VALUES[0], 0xB4);
    program.increment_abs(MAPPER7_AXROM_CASE_COUNT_ADDR);

    program.asm.lda_imm(0x10);
    program.asm.sta_abs(0x8000);
    program.write_ppu_data(0x2000, MAPPER7_MIRROR_EXPECTED_VALUES[1]);
    program.read_ppu_data_into_a(0x2400);
    program
        .asm
        .sta_abs(MAPPER7_AXROM_MIRROR_OBSERVED_BASE_ADDR + 1);
    program.expect_a_eq(MAPPER7_MIRROR_EXPECTED_VALUES[1], 0xB5);
    program.increment_abs(MAPPER7_AXROM_CASE_COUNT_ADDR);

    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0x8000);
    program.read_ppu_data_into_a(0x2400);
    program
        .asm
        .sta_abs(MAPPER7_AXROM_MIRROR_OBSERVED_BASE_ADDR + 2);
    program.expect_a_eq(MAPPER7_MIRROR_EXPECTED_VALUES[2], 0xB6);
    program.increment_abs(MAPPER7_AXROM_CASE_COUNT_ADDR);

    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("fail")?;
    program.asm.lda_imm(STATUS_FAIL);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("irq")?;
    program.asm.rti();
    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

fn build_input_mask_sweep_variant_program_with_labels(
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new();

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017);
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(INPUT_MASK_SWEEP_TEST_ID);
    program.asm.sta_zp(CURRENT_TEST_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);
    program.asm.sta_abs(INPUT_MASK_SWEEP_JOYPAD1_OBSERVED_ADDR);
    program.asm.sta_abs(INPUT_MASK_SWEEP_JOYPAD2_OBSERVED_ADDR);
    program.asm.sta_abs(INPUT_MASK_SWEEP_CASE_COUNT_ADDR);
    program.asm.sta_abs(0x2000);
    program.asm.sta_abs(0x2001);

    program.asm.lda_imm(0x01);
    program.asm.sta_abs(0x4016);
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0x4016);
    program.read_joypad_port_mask_into(0x4016, INPUT_MASK_SWEEP_JOYPAD1_OBSERVED_ADDR);
    program.read_joypad_port_mask_into(0x4017, INPUT_MASK_SWEEP_JOYPAD2_OBSERVED_ADDR);

    program.asm.lda_abs(INPUT_MASK_SWEEP_JOYPAD1_OBSERVED_ADDR);
    program.expect_a_eq_zp(JOYPAD1_EXPECTED_MASK_ADDR, 0xA0);
    program.asm.lda_abs(INPUT_MASK_SWEEP_JOYPAD2_OBSERVED_ADDR);
    program.expect_a_eq_zp(JOYPAD2_EXPECTED_MASK_ADDR, 0xA1);
    program.asm.lda_imm(0x01);
    program.asm.sta_abs(INPUT_MASK_SWEEP_CASE_COUNT_ADDR);
    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("fail")?;
    program.asm.lda_imm(STATUS_FAIL);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("irq")?;
    program.asm.rti();
    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

fn build_ppu_scroll_wrap_variant_program_with_labels(
) -> Result<(Vec<u8>, HashMap<String, u16>), String> {
    let mut program = DiagnosticProgram::new();

    program.asm.label("reset")?;
    program.asm.sei();
    program.asm.cld();
    program.asm.ldx_imm(0xFF);
    program.asm.txs();
    program.asm.lda_imm(0x40);
    program.asm.sta_abs(0x4017);
    program.asm.lda_imm(STATUS_RUNNING);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.lda_imm(0xA5);
    program.asm.sta_zp(SIGNATURE_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_zp(FAILURE_CODE_ADDR);
    program.asm.sta_zp(NMI_COUNT_ADDR);

    program.begin_test(PPU_SCROLL_SEAM_TEST_ID);
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0x2000);
    program.asm.sta_abs(0x2001);
    program.write_ppu_data(0x205F, 0x03);
    program.write_ppu_data(0x2440, 0x02);
    program.write_ppu_data(0x23C7, 0x00);
    program.write_ppu_data(0x27C0, 0x00);
    program.write_ppu_data(0x3F00, 0x0F);
    program.asm.lda_imm(0x21);
    program.asm.sta_abs(0x2007);
    program.asm.lda_imm(0x16);
    program.asm.sta_abs(0x2007);

    program
        .asm
        .lda_imm(PPU_SCROLL_SEAM_NAMETABLE_WRAP_CASE_COUNT);
    program.asm.sta_abs(PPU_SCROLL_SEAM_CASE_COUNT_ADDR);
    program.asm.lda_imm(0x00);
    program.asm.sta_abs(0x2000);
    program.asm.lda_abs(0x2002);
    program.asm.lda_imm(PPU_SCROLL_SEAM_NAMETABLE_WRAP_SCROLL_X);
    program.asm.sta_abs(0x2005);
    program.asm.lda_imm(PPU_SCROLL_SEAM_NAMETABLE_WRAP_SCROLL_Y);
    program.asm.sta_abs(0x2005);
    program.asm.lda_imm(0x0A);
    program.asm.sta_abs(0x2001);
    program.wait_for_vblank("scroll_wrap_first_vblank");
    program.wait_for_vblank("scroll_wrap_second_vblank");
    program.delay_host_frame_capture();
    program.pass_test(PPU_SCROLL_SEAM_TEST_ID);
    program.asm.lda_imm(STATUS_PASS);
    program.asm.sta_zp(STATUS_ADDR);
    program.asm.jmp_label("hang");

    program.asm.label("nmi")?;
    program.asm.inc_zp(NMI_COUNT_ADDR);
    program.asm.rti();
    program.asm.label("irq")?;
    program.asm.rti();
    program.asm.label("hang")?;
    program.asm.jmp_label("hang");

    let labels = program.asm.labels.clone();
    let bytes = program.asm.finalize()?;
    Ok((bytes, labels))
}

pub fn run_diagnostic(config: DiagnosticConfig) -> Result<DiagnosticTelemetry, String> {
    let (program, labels) = build_program_with_labels()?;
    let trace_context = DiagnosticTraceContext::from_labels(&labels);
    let fault_injection_pc = match config.fault_injection {
        Some(fault) => Some(label_addr(&labels, fault.injection_label())?),
        None => None,
    };
    let ppu_nmi_wait_pc = label_addr(&labels, PPU_NMI_TIMEOUT_FAULT_LABEL)?;
    let rom = build_diagnostic_cartridge_from_program(&program, &labels)?;
    let cartridge_info = cartridge_telemetry(&rom);
    let (validate_render_frame_signature, render_frame_signature_validation_reason) =
        diagnostic_render_frame_signature_validation(&config);
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    apply_joypad_mask(&mut bus, config.joypad1_mask);
    apply_joypad2_mask(&mut bus, config.joypad2_mask);
    bus.cpu_write(
        JOYPAD1_EXPECTED_MASK_ADDR as u16,
        config.expected_joypad1_mask,
    );
    bus.cpu_write(
        JOYPAD2_EXPECTED_MASK_ADDR as u16,
        config.expected_joypad2_mask,
    );

    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let mut audio_sample_count = 0usize;
    let mut audio_peak_abs = 0.0f32;
    let mut audio_sum_abs = 0.0f64;
    let mut audio_sum_squares = 0.0f64;
    let mut diagnostic_render_frame = None;
    let mut ppu_scroll_seam_frame = None;
    let mut ppu_sprite_priority_frame = None;
    let mut events = Vec::new();
    let mut last_status = read_ram_byte(&mut bus, STATUS_ADDR);
    let mut last_current_test = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
    let mut timeout = true;
    let mut dma_observation = DmaObservation::default();
    let mut instruction_trace = InstructionTraceObservation::default();
    let mut ppu_vblank_timing =
        PpuVblankTimingObservation::new(read_ram_byte(&mut bus, NMI_COUNT_ADDR));
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
        ppu_vblank_timing.observe_wait_loop(
            cpu.pc,
            ppu_nmi_wait_pc,
            cycles,
            frames,
            read_ram_byte(&mut bus, CURRENT_TEST_ADDR),
        );
        let dma_active_before = bus.dma_active();
        let dmc_stall_before = bus.dmc_stall_active();
        cpu.clock(&mut bus);
        let current_test_after_cpu = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
        tick_ppu_for_diagnostic_cpu_cycle(
            &mut bus,
            &mut ppu_vblank_timing,
            current_test_after_cpu,
            cycles,
            frames,
        );
        bus.tick_apu();
        let dmc_dma_service = bus.service_dmc_dma(cpu.is_odd_cycle());
        cycles += 1;

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        let current_test = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
        let cpu_snapshot = cpu_telemetry(&cpu);
        let diagnostic_ram = diagnostic_ram_watch_telemetry(&mut bus, status, current_test);
        ppu_vblank_timing.observe_nmi_count(cycles, frames, current_test, diagnostic_ram.nmi_count);
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
            maybe_capture_diagnostic_render_frame(
                &mut diagnostic_render_frame,
                current_test,
                &bus.ppu.frame_data,
                validate_render_frame_signature,
                render_frame_signature_validation_reason,
            );
            maybe_capture_ppu_sprite_priority_frame(
                &mut ppu_sprite_priority_frame,
                current_test,
                &bus.ppu.frame_data,
            );
            let ppu_scroll_seam_case_count = bus.cpu_read(PPU_SCROLL_SEAM_CASE_COUNT_ADDR);
            maybe_capture_ppu_scroll_seam_frame(
                &mut ppu_scroll_seam_frame,
                current_test,
                ppu_scroll_seam_case_count,
                &bus.ppu.frame_data,
            );
            bus.apu.end_frame();
            let samples = bus.apu.drain_samples();
            audio_sample_count += samples.len();
            for sample in samples {
                let abs = sample.abs();
                audio_peak_abs = audio_peak_abs.max(abs);
                audio_sum_abs += f64::from(abs);
                audio_sum_squares += f64::from(sample) * f64::from(sample);
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
            ppu_vblank_timing.observe_wait_loop(
                cpu.pc,
                ppu_nmi_wait_pc,
                cycles,
                frames,
                read_ram_byte(&mut bus, CURRENT_TEST_ADDR),
            );
            let dma_active_before = bus.dma_active();
            let dmc_stall_before = bus.dmc_stall_active();
            cpu.clock(&mut bus);
            let current_test_after_cpu = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
            tick_ppu_for_diagnostic_cpu_cycle(
                &mut bus,
                &mut ppu_vblank_timing,
                current_test_after_cpu,
                cycles,
                frames,
            );
            bus.tick_apu();
            let dmc_dma_service = bus.service_dmc_dma(cpu.is_odd_cycle());
            cycles += 1;

            let status = read_ram_byte(&mut bus, STATUS_ADDR);
            let current_test = read_ram_byte(&mut bus, CURRENT_TEST_ADDR);
            let cpu_snapshot = cpu_telemetry(&cpu);
            let diagnostic_ram = diagnostic_ram_watch_telemetry(&mut bus, status, current_test);
            ppu_vblank_timing.observe_nmi_count(
                cycles,
                frames,
                current_test,
                diagnostic_ram.nmi_count,
            );
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
                maybe_capture_diagnostic_render_frame(
                    &mut diagnostic_render_frame,
                    current_test,
                    &bus.ppu.frame_data,
                    validate_render_frame_signature,
                    render_frame_signature_validation_reason,
                );
                maybe_capture_ppu_sprite_priority_frame(
                    &mut ppu_sprite_priority_frame,
                    current_test,
                    &bus.ppu.frame_data,
                );
                let ppu_scroll_seam_case_count = bus.cpu_read(PPU_SCROLL_SEAM_CASE_COUNT_ADDR);
                maybe_capture_ppu_scroll_seam_frame(
                    &mut ppu_scroll_seam_frame,
                    current_test,
                    ppu_scroll_seam_case_count,
                    &bus.ppu.frame_data,
                );
                bus.apu.end_frame();
                let samples = bus.apu.drain_samples();
                audio_sample_count += samples.len();
                for sample in samples {
                    let abs = sample.abs();
                    audio_peak_abs = audio_peak_abs.max(abs);
                    audio_sum_abs += f64::from(abs);
                    audio_sum_squares += f64::from(sample) * f64::from(sample);
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
    let frame = diagnostic_render_frame.unwrap_or_else(|| {
        frame_telemetry(
            &bus.ppu.frame_data,
            validate_render_frame_signature,
            render_frame_signature_validation_reason,
        )
    });
    let cpu_addressing_matrix = cpu_addressing_matrix_telemetry(&ram);
    let cpu_branch_matrix = cpu_branch_matrix_telemetry(&ram);
    let cpu_stack_matrix = cpu_stack_matrix_telemetry(&ram);
    let input_port_matrix = input_port_matrix_telemetry(&ram, &config);
    let apu_status_matrix = apu_status_matrix_telemetry(&ram);
    let apu_dmc_status = apu_dmc_status_telemetry(&ram);
    let ppu_vblank_timing = ppu_vblank_timing.telemetry(ram[NMI_COUNT_ADDR as usize]);
    let ppu_scroll_wrap = run_ppu_scroll_wrap_variant();
    let ppu_scroll_seam = ppu_scroll_seam_telemetry(
        &ram,
        ppu_scroll_seam_frame.as_ref(),
        &bus.ppu.frame_data,
        &ppu_scroll_wrap,
    );
    let mapper1_mmc1 = mapper1_mmc1_telemetry(&run_mapper1_mmc1_variant());
    let mapper1_mmc1_32k_prg = mapper1_mmc1_32k_prg_telemetry(&run_mapper1_mmc1_32k_prg_variant());
    let mapper3_chr_bank = mapper3_chr_bank_telemetry(&run_mapper3_chr_bank_variant());
    let mapper4_mmc3 = mapper4_mmc3_telemetry(&run_mapper4_mmc3_variant());
    let mapper4_mmc3_edge = mapper4_mmc3_edge_telemetry(&run_mapper4_mmc3_edge_variant());
    let mapper4_mmc3_prg_ram = mapper4_mmc3_prg_ram_telemetry(&run_mapper4_mmc3_prg_ram_variant());
    let mapper7_axrom = mapper7_axrom_telemetry(&run_mapper7_axrom_variant());
    let input_mask_sweep = input_mask_sweep_telemetry(&run_input_mask_sweep_variant());
    let ppu_sprite_overflow = ppu_sprite_overflow_telemetry(&ram);
    let ppu_sprite_priority = ppu_sprite_priority_telemetry(
        &ram,
        ppu_sprite_priority_frame.as_ref(),
        &bus.ppu.frame_data,
    );
    let ppu_sprite_zero_hit = ppu_sprite_zero_hit_telemetry(&ram);
    let audio = audio_telemetry(
        audio_sample_count,
        audio_peak_abs,
        audio_sum_abs,
        audio_sum_squares,
    );
    let cpu_rmw_matrix = cpu_rmw_matrix_telemetry(&ram);
    let cpu_rmw_addressing_matrix = cpu_rmw_addressing_matrix_telemetry(&ram);
    let mut host_failures = host_validate(HostValidationInput {
        status,
        timeout,
        tests: &test_results,
        ram: &ram,
        cpu_addressing_matrix: &cpu_addressing_matrix,
        cpu_branch_matrix: &cpu_branch_matrix,
        cpu_stack_matrix: &cpu_stack_matrix,
        cpu_rmw_addressing_matrix: &cpu_rmw_addressing_matrix,
        cpu_rmw_matrix: &cpu_rmw_matrix,
        input_port_matrix: &input_port_matrix,
        input_mask_sweep: &input_mask_sweep,
        apu_status_matrix: &apu_status_matrix,
        apu_dmc_status: &apu_dmc_status,
        ppu_vblank_timing: &ppu_vblank_timing,
        ppu_scroll_seam: &ppu_scroll_seam,
        ppu_sprite_overflow: &ppu_sprite_overflow,
        ppu_sprite_priority: &ppu_sprite_priority,
        ppu_sprite_zero_hit: &ppu_sprite_zero_hit,
        mapper1_mmc1: &mapper1_mmc1,
        mapper1_mmc1_32k_prg: &mapper1_mmc1_32k_prg,
        mapper3_chr_bank: &mapper3_chr_bank,
        mapper4_mmc3: &mapper4_mmc3,
        mapper4_mmc3_edge: &mapper4_mmc3_edge,
        mapper4_mmc3_prg_ram: &mapper4_mmc3_prg_ram,
        mapper7_axrom: &mapper7_axrom,
        dma: &dma,
        oam: &oam,
        frame: &frame,
        audio: &audio,
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
        cpu_addressing_matrix: &cpu_addressing_matrix,
        cpu_branch_matrix: &cpu_branch_matrix,
        cpu_stack_matrix: &cpu_stack_matrix,
        cpu_rmw_addressing_matrix: &cpu_rmw_addressing_matrix,
        cpu_rmw_matrix: &cpu_rmw_matrix,
        input_port_matrix: &input_port_matrix,
        input_mask_sweep: &input_mask_sweep,
        apu_status_matrix: &apu_status_matrix,
        apu_dmc_status: &apu_dmc_status,
        ppu_vblank_timing: &ppu_vblank_timing,
        ppu_scroll_seam: &ppu_scroll_seam,
        ppu_sprite_overflow: &ppu_sprite_overflow,
        ppu_sprite_priority: &ppu_sprite_priority,
        ppu_sprite_zero_hit: &ppu_sprite_zero_hit,
        mapper1_mmc1: &mapper1_mmc1,
        mapper1_mmc1_32k_prg: &mapper1_mmc1_32k_prg,
        mapper3_chr_bank: &mapper3_chr_bank,
        mapper4_mmc3: &mapper4_mmc3,
        mapper4_mmc3_edge: &mapper4_mmc3_edge,
        mapper4_mmc3_prg_ram: &mapper4_mmc3_prg_ram,
        mapper7_axrom: &mapper7_axrom,
        dma: &dma,
        oam: &oam,
        frame: &frame,
        audio: &audio,
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
        mapper1_mmc1,
        mapper1_mmc1_32k_prg,
        mapper3_chr_bank,
        mapper4_mmc3,
        mapper4_mmc3_edge,
        mapper4_mmc3_prg_ram,
        mapper7_axrom,
        input_mask_sweep,
        input: diagnostic_input_telemetry(&config),
        verdict,
        analysis,
        cycles,
        frames,
        cpu: cpu_telemetry(&cpu),
        cpu_addressing_matrix,
        cpu_branch_matrix,
        cpu_stack_matrix,
        cpu_rmw_addressing_matrix,
        cpu_rmw_matrix,
        input_port_matrix,
        apu_status_matrix,
        apu_dmc_status,
        ppu_vblank_timing,
        ppu_scroll_seam,
        ppu_sprite_overflow,
        ppu_sprite_priority,
        ppu_sprite_zero_hit,
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
        audio,
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
    let current_json = serde_json::to_string(telemetry)
        .map_err(|err| format!("failed to serialize current diagnostic telemetry: {err}"))?;
    let current: Value = serde_json::from_str(&current_json)
        .map_err(|err| format!("failed to parse current diagnostic JSON: {err}"))?;
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
    write_mapper_section(&mut report, telemetry);
    write_ppu_section(&mut report, telemetry);
    write_dma_section(&mut report, telemetry);
    write_audio_section(&mut report, telemetry);
    write_cpu_branch_section(&mut report, telemetry);
    write_cpu_stack_section(&mut report, telemetry);
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
    writeln!(
        report,
        "| Input mask sweep cases / expected | {} / {} |",
        telemetry.input_mask_sweep.observed_case_count,
        telemetry.input_mask_sweep.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Input mask sweep passed / failed | {} / {} |",
        telemetry.input_mask_sweep.passed_case_count, telemetry.input_mask_sweep.failed_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Input mask sweep passed | {} |",
        telemetry.input_mask_sweep.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Input mask sweep error | {} |",
        optional_string(telemetry.input_mask_sweep.error.as_deref())
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

fn write_mapper_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## Cartridge Mapper Variants").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| Main cartridge mapper / PRG banks / CHR banks | {} / {} / {} |",
        telemetry.cartridge.mapper, telemetry.cartridge.prg_banks, telemetry.cartridge.chr_banks
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 variant mapper / PRG banks / CHR banks | {} / {} / {} |",
        telemetry.mapper1_mmc1.mapper,
        telemetry.mapper1_mmc1.prg_banks,
        telemetry.mapper1_mmc1.chr_8k_banks
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 PRG switch / fixed addresses | {} / {} |",
        telemetry.mapper1_mmc1.prg_switch_addr_hex, telemetry.mapper1_mmc1.prg_fixed_addr_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 CHR low / high read addresses | {} / {} |",
        telemetry.mapper1_mmc1.chr_low_read_addr_hex, telemetry.mapper1_mmc1.chr_high_read_addr_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 PRG bank writes | {:?} |",
        telemetry.mapper1_mmc1.prg_bank_writes_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 CHR bank writes | {:?} |",
        telemetry.mapper1_mmc1.chr_bank_writes_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 PRG observed / expected | {:?} / {:?} |",
        telemetry.mapper1_mmc1.observed_prg_values_hex,
        telemetry.mapper1_mmc1.expected_prg_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 CHR observed / expected | {:?} / {:?} |",
        telemetry.mapper1_mmc1.observed_chr_values_hex,
        telemetry.mapper1_mmc1.expected_chr_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 mirror observed / expected | {:?} / {:?} |",
        telemetry.mapper1_mmc1.observed_mirror_values_hex,
        telemetry.mapper1_mmc1.expected_mirror_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 cases / expected | {} / {} |",
        telemetry.mapper1_mmc1.observed_case_count, telemetry.mapper1_mmc1.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 cycles / frames / passed | {} / {} / {} |",
        telemetry.mapper1_mmc1.cycles, telemetry.mapper1_mmc1.frames, telemetry.mapper1_mmc1.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 error | {} |",
        optional_string(telemetry.mapper1_mmc1.error.as_deref())
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 32 KiB PRG read addresses | {} / {} |",
        telemetry.mapper1_mmc1_32k_prg.low_read_addr_hex,
        telemetry.mapper1_mmc1_32k_prg.high_read_addr_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 32 KiB PRG control writes | {:?} |",
        telemetry.mapper1_mmc1_32k_prg.control_writes_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 32 KiB PRG bank writes | {:?} |",
        telemetry.mapper1_mmc1_32k_prg.prg_bank_writes_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 32 KiB PRG observed / expected | {:?} / {:?} |",
        telemetry.mapper1_mmc1_32k_prg.observed_values_hex,
        telemetry.mapper1_mmc1_32k_prg.expected_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 32 KiB PRG cases / expected | {} / {} |",
        telemetry.mapper1_mmc1_32k_prg.observed_case_count,
        telemetry.mapper1_mmc1_32k_prg.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 32 KiB PRG cycles / frames / passed | {} / {} / {} |",
        telemetry.mapper1_mmc1_32k_prg.cycles,
        telemetry.mapper1_mmc1_32k_prg.frames,
        telemetry.mapper1_mmc1_32k_prg.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 1 32 KiB PRG error | {} |",
        optional_string(telemetry.mapper1_mmc1_32k_prg.error.as_deref())
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 3 variant mapper / PRG banks / CHR banks | {} / {} / {} |",
        telemetry.mapper3_chr_bank.mapper,
        telemetry.mapper3_chr_bank.prg_banks,
        telemetry.mapper3_chr_bank.chr_banks
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 3 CHR read address | {} |",
        telemetry.mapper3_chr_bank.read_addr_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 3 CHR banks | {:?} |",
        telemetry.mapper3_chr_bank.expected_banks
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 3 CHR observed / expected | {:?} / {:?} |",
        telemetry.mapper3_chr_bank.observed_values_hex,
        telemetry.mapper3_chr_bank.expected_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 3 CHR cases / expected | {} / {} |",
        telemetry.mapper3_chr_bank.observed_case_count,
        telemetry.mapper3_chr_bank.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 3 CHR cycles / frames / passed | {} / {} / {} |",
        telemetry.mapper3_chr_bank.cycles,
        telemetry.mapper3_chr_bank.frames,
        telemetry.mapper3_chr_bank.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 3 CHR error | {} |",
        optional_string(telemetry.mapper3_chr_bank.error.as_deref())
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 variant mapper / PRG 16K banks / CHR 8K banks | {} / {} / {} |",
        telemetry.mapper4_mmc3.mapper,
        telemetry.mapper4_mmc3.prg_16k_banks,
        telemetry.mapper4_mmc3.chr_8k_banks
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG read addresses | {:?} |",
        telemetry.mapper4_mmc3.prg_read_addrs_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 CHR read addresses | {:?} |",
        telemetry.mapper4_mmc3.chr_read_addrs_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG register writes | {:?} |",
        telemetry.mapper4_mmc3.prg_register_writes_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 CHR register writes | {:?} |",
        telemetry.mapper4_mmc3.chr_register_writes_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG observed / expected | {:?} / {:?} |",
        telemetry.mapper4_mmc3.observed_prg_values_hex,
        telemetry.mapper4_mmc3.expected_prg_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 CHR observed / expected | {:?} / {:?} |",
        telemetry.mapper4_mmc3.observed_chr_values_hex,
        telemetry.mapper4_mmc3.expected_chr_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 mirror observed / expected | {:?} / {:?} |",
        telemetry.mapper4_mmc3.observed_mirror_values_hex,
        telemetry.mapper4_mmc3.expected_mirror_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 IRQ observed / expected | {} / {} |",
        telemetry.mapper4_mmc3.observed_irq_count, telemetry.mapper4_mmc3.expected_irq_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 cases / expected | {} / {} |",
        telemetry.mapper4_mmc3.observed_case_count, telemetry.mapper4_mmc3.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 cycles / frames / passed | {} / {} / {} |",
        telemetry.mapper4_mmc3.cycles, telemetry.mapper4_mmc3.frames, telemetry.mapper4_mmc3.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 error | {} |",
        optional_string(telemetry.mapper4_mmc3.error.as_deref())
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge program base | {} |",
        telemetry.mapper4_mmc3_edge.program_base_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge PRG read addresses | {:?} |",
        telemetry.mapper4_mmc3_edge.prg_read_addrs_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge CHR read addresses | {:?} |",
        telemetry.mapper4_mmc3_edge.chr_read_addrs_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge PRG select writes | {:?} |",
        telemetry.mapper4_mmc3_edge.prg_select_writes_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge CHR select writes | {:?} |",
        telemetry.mapper4_mmc3_edge.chr_select_writes_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge PRG observed / expected | {:?} / {:?} |",
        telemetry.mapper4_mmc3_edge.observed_prg_values_hex,
        telemetry.mapper4_mmc3_edge.expected_prg_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge CHR observed / expected | {:?} / {:?} |",
        telemetry.mapper4_mmc3_edge.observed_chr_values_hex,
        telemetry.mapper4_mmc3_edge.expected_chr_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge IRQ latches / observed / expected | {:?} / {:?} / {:?} |",
        telemetry.mapper4_mmc3_edge.irq_latches_hex,
        telemetry.mapper4_mmc3_edge.observed_irq_counts,
        telemetry.mapper4_mmc3_edge.expected_irq_counts
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge cases / expected | {} / {} |",
        telemetry.mapper4_mmc3_edge.observed_case_count,
        telemetry.mapper4_mmc3_edge.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge cycles / frames / passed | {} / {} / {} |",
        telemetry.mapper4_mmc3_edge.cycles,
        telemetry.mapper4_mmc3_edge.frames,
        telemetry.mapper4_mmc3_edge.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 edge error | {} |",
        optional_string(telemetry.mapper4_mmc3_edge.error.as_deref())
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG RAM mapper / PRG 16K banks / battery | {} / {} / {} |",
        telemetry.mapper4_mmc3_prg_ram.mapper,
        telemetry.mapper4_mmc3_prg_ram.prg_16k_banks,
        telemetry.mapper4_mmc3_prg_ram.battery_backed
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG RAM read addresses | {:?} |",
        telemetry.mapper4_mmc3_prg_ram.read_addrs_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG RAM observed / expected | {:?} / {:?} |",
        telemetry.mapper4_mmc3_prg_ram.observed_values_hex,
        telemetry.mapper4_mmc3_prg_ram.expected_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG RAM SRAM snapshot / restored | {:?} / {:?} |",
        telemetry.mapper4_mmc3_prg_ram.sram_snapshot_values_hex,
        telemetry.mapper4_mmc3_prg_ram.restored_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG RAM restore addresses | {:?} |",
        telemetry.mapper4_mmc3_prg_ram.restored_addrs_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG RAM cases / expected | {} / {} |",
        telemetry.mapper4_mmc3_prg_ram.observed_case_count,
        telemetry.mapper4_mmc3_prg_ram.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG RAM cycles / frames / passed | {} / {} / {} |",
        telemetry.mapper4_mmc3_prg_ram.cycles,
        telemetry.mapper4_mmc3_prg_ram.frames,
        telemetry.mapper4_mmc3_prg_ram.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 4 PRG RAM error | {} |",
        optional_string(telemetry.mapper4_mmc3_prg_ram.error.as_deref())
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 7 variant mapper / PRG banks / CHR banks | {} / {} / {} |",
        telemetry.mapper7_axrom.mapper,
        telemetry.mapper7_axrom.prg_banks,
        telemetry.mapper7_axrom.chr_banks
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 7 PRG read address | {} |",
        telemetry.mapper7_axrom.prg_read_addr_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 7 bank writes | {:?} |",
        telemetry.mapper7_axrom.bank_writes_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 7 PRG observed / expected | {:?} / {:?} |",
        telemetry.mapper7_axrom.observed_prg_values_hex,
        telemetry.mapper7_axrom.expected_prg_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 7 mirror observed / expected | {:?} / {:?} |",
        telemetry.mapper7_axrom.observed_mirror_values_hex,
        telemetry.mapper7_axrom.expected_mirror_values_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 7 cases / expected | {} / {} |",
        telemetry.mapper7_axrom.observed_case_count, telemetry.mapper7_axrom.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 7 cycles / frames / passed | {} / {} / {} |",
        telemetry.mapper7_axrom.cycles,
        telemetry.mapper7_axrom.frames,
        telemetry.mapper7_axrom.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Mapper 7 error | {} |",
        optional_string(telemetry.mapper7_axrom.error.as_deref())
    )
    .expect("write report");
    writeln!(report).expect("write report");
}

fn write_ppu_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## PPU Pixel Pipeline").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| Vblank wait-loop start cycle / frame | {} / {} |",
        optional_u64(telemetry.ppu_vblank_timing.wait_loop_start_cycle),
        optional_u64(telemetry.ppu_vblank_timing.wait_loop_start_frame)
    )
    .expect("write report");
    writeln!(
        report,
        "| First NMI cycle / latency | {} / {} |",
        optional_u64(telemetry.ppu_vblank_timing.first_nmi_cycle),
        optional_u64(telemetry.ppu_vblank_timing.first_nmi_latency_cycles)
    )
    .expect("write report");
    writeln!(
        report,
        "| Second NMI cycle / inter-NMI cycles | {} / {} |",
        optional_u64(telemetry.ppu_vblank_timing.second_nmi_cycle),
        optional_u64(telemetry.ppu_vblank_timing.inter_nmi_cycles)
    )
    .expect("write report");
    writeln!(
        report,
        "| Vblank timing windows | first {}..={}, inter {}..={} |",
        telemetry.ppu_vblank_timing.first_nmi_expected_min_cycles,
        telemetry.ppu_vblank_timing.first_nmi_expected_max_cycles,
        telemetry.ppu_vblank_timing.inter_nmi_expected_min_cycles,
        telemetry.ppu_vblank_timing.inter_nmi_expected_max_cycles
    )
    .expect("write report");
    writeln!(
        report,
        "| Vblank edge expected set/clear dots | set scanline {} dot {}, clear scanline {} dot {} |",
        telemetry.ppu_vblank_timing.edge_expected_set_scanline,
        telemetry.ppu_vblank_timing.edge_expected_set_dot,
        telemetry.ppu_vblank_timing.edge_expected_clear_scanline,
        telemetry.ppu_vblank_timing.edge_expected_clear_dot
    )
    .expect("write report");
    writeln!(
        report,
        "| Vblank edge observed counts | set {}, clear {}, NMI triggers {} |",
        telemetry.ppu_vblank_timing.edge_set_count,
        telemetry.ppu_vblank_timing.edge_clear_count,
        telemetry.ppu_vblank_timing.edge_nmi_trigger_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Vblank first set edge CPU/frame/PPU/phase | {} / {} / {}:{} / {} |",
        optional_u64(telemetry.ppu_vblank_timing.edge_first_set_cpu_cycle),
        optional_u64(telemetry.ppu_vblank_timing.edge_first_set_frame),
        optional_i16(telemetry.ppu_vblank_timing.edge_first_set_ppu_scanline),
        optional_u16(telemetry.ppu_vblank_timing.edge_first_set_ppu_dot),
        optional_u8(telemetry.ppu_vblank_timing.edge_first_set_ppu_phase)
    )
    .expect("write report");
    writeln!(
        report,
        "| Vblank first clear edge CPU/frame/PPU/phase | {} / {} / {}:{} / {} |",
        optional_u64(telemetry.ppu_vblank_timing.edge_first_clear_cpu_cycle),
        optional_u64(telemetry.ppu_vblank_timing.edge_first_clear_frame),
        optional_i16(telemetry.ppu_vblank_timing.edge_first_clear_ppu_scanline),
        optional_u16(telemetry.ppu_vblank_timing.edge_first_clear_ppu_dot),
        optional_u8(telemetry.ppu_vblank_timing.edge_first_clear_ppu_phase)
    )
    .expect("write report");
    writeln!(
        report,
        "| Vblank second set edge CPU/frame/PPU/phase | {} / {} / {}:{} / {} |",
        optional_u64(telemetry.ppu_vblank_timing.edge_second_set_cpu_cycle),
        optional_u64(telemetry.ppu_vblank_timing.edge_second_set_frame),
        optional_i16(telemetry.ppu_vblank_timing.edge_second_set_ppu_scanline),
        optional_u16(telemetry.ppu_vblank_timing.edge_second_set_ppu_dot),
        optional_u8(telemetry.ppu_vblank_timing.edge_second_set_ppu_phase)
    )
    .expect("write report");
    writeln!(
        report,
        "| Vblank timing passed | {} |",
        telemetry.ppu_vblank_timing.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Render-frame checksum / expected | {} / {} |",
        telemetry.frame.checksum_hex, telemetry.frame.expected_checksum_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Render-frame checksum passed | {} |",
        telemetry.frame.checksum_matches_expected
    )
    .expect("write report");
    writeln!(
        report,
        "| Render-frame checksum validation | {} |",
        telemetry.frame.checksum_validation_reason
    )
    .expect("write report");
    writeln!(
        report,
        "| Render-frame colors / expected | {} / {} |",
        telemetry.frame.unique_colors, telemetry.frame.expected_unique_colors
    )
    .expect("write report");
    writeln!(
        report,
        "| Render-frame nonzero pixels / expected | {} / {} |",
        telemetry.frame.nonzero_pixels, telemetry.frame.expected_nonzero_pixels
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-zero-hit status bit / expected | {} / {} |",
        telemetry.ppu_sprite_zero_hit.observed_status_bit_hex,
        telemetry.ppu_sprite_zero_hit.expected_status_bit_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-zero-hit cases / expected | {} / {} |",
        telemetry.ppu_sprite_zero_hit.observed_case_count,
        telemetry.ppu_sprite_zero_hit.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-zero-hit passed | {} |",
        telemetry.ppu_sprite_zero_hit.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-overflow status bit / expected | {} / {} |",
        telemetry.ppu_sprite_overflow.observed_status_bit_hex,
        telemetry.ppu_sprite_overflow.expected_status_bit_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-overflow false-positive bit / expected | {} / {} |",
        telemetry
            .ppu_sprite_overflow
            .false_positive_observed_status_bit_hex,
        telemetry
            .ppu_sprite_overflow
            .false_positive_expected_status_bit_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-overflow false-negative bit / expected | {} / {} |",
        telemetry
            .ppu_sprite_overflow
            .false_negative_observed_status_bit_hex,
        telemetry
            .ppu_sprite_overflow
            .false_negative_expected_status_bit_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-overflow cases / expected | {} / {} |",
        telemetry.ppu_sprite_overflow.observed_case_count,
        telemetry.ppu_sprite_overflow.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-overflow restored OAM bytes | {} |",
        telemetry.ppu_sprite_overflow.restored_oam_byte_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-overflow hardware-bug matrix passed | {} |",
        telemetry.ppu_sprite_overflow.hardware_bug_matrix_passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-overflow passed | {} |",
        telemetry.ppu_sprite_overflow.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-priority front sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_sprite_priority.front_sample_x,
        telemetry.ppu_sprite_priority.front_sample_y,
        telemetry.ppu_sprite_priority.front_observed_color_hex,
        telemetry.ppu_sprite_priority.front_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-priority behind sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_sprite_priority.behind_sample_x,
        telemetry.ppu_sprite_priority.behind_sample_y,
        telemetry.ppu_sprite_priority.behind_observed_color_hex,
        telemetry.ppu_sprite_priority.behind_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-priority cases / expected | {} / {} |",
        telemetry.ppu_sprite_priority.observed_case_count,
        telemetry.ppu_sprite_priority.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Sprite-priority passed | {} |",
        telemetry.ppu_sprite_priority.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam left sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_scroll_seam.left_sample_x,
        telemetry.ppu_scroll_seam.left_sample_y,
        telemetry.ppu_scroll_seam.left_observed_color_hex,
        telemetry.ppu_scroll_seam.left_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam right sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_scroll_seam.right_sample_x,
        telemetry.ppu_scroll_seam.right_sample_y,
        telemetry.ppu_scroll_seam.right_observed_color_hex,
        telemetry.ppu_scroll_seam.right_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam coarse-left sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_scroll_seam.coarse_left_sample_x,
        telemetry.ppu_scroll_seam.coarse_left_sample_y,
        telemetry.ppu_scroll_seam.coarse_left_observed_color_hex,
        telemetry.ppu_scroll_seam.coarse_left_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam coarse-right sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_scroll_seam.coarse_right_sample_x,
        telemetry.ppu_scroll_seam.coarse_right_sample_y,
        telemetry.ppu_scroll_seam.coarse_right_observed_color_hex,
        telemetry.ppu_scroll_seam.coarse_right_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam nametable-wrap left sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_scroll_seam.nametable_wrap_left_sample_x,
        telemetry.ppu_scroll_seam.nametable_wrap_left_sample_y,
        telemetry
            .ppu_scroll_seam
            .nametable_wrap_left_observed_color_hex,
        telemetry
            .ppu_scroll_seam
            .nametable_wrap_left_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam nametable-wrap right sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_scroll_seam.nametable_wrap_right_sample_x,
        telemetry.ppu_scroll_seam.nametable_wrap_right_sample_y,
        telemetry
            .ppu_scroll_seam
            .nametable_wrap_right_observed_color_hex,
        telemetry
            .ppu_scroll_seam
            .nametable_wrap_right_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam top sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_scroll_seam.top_sample_x,
        telemetry.ppu_scroll_seam.top_sample_y,
        telemetry.ppu_scroll_seam.top_observed_color_hex,
        telemetry.ppu_scroll_seam.top_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam bottom sample / expected | ({}, {}) {} / {} |",
        telemetry.ppu_scroll_seam.bottom_sample_x,
        telemetry.ppu_scroll_seam.bottom_sample_y,
        telemetry.ppu_scroll_seam.bottom_observed_color_hex,
        telemetry.ppu_scroll_seam.bottom_expected_color_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam scroll X/coarse-X/Y | {} / {} / {} |",
        telemetry.ppu_scroll_seam.scroll_x,
        telemetry.ppu_scroll_seam.coarse_scroll_x,
        telemetry.ppu_scroll_seam.scroll_y
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam nametable-wrap scroll X/Y/mirroring | {} / {} / {} |",
        telemetry.ppu_scroll_seam.nametable_wrap_scroll_x,
        telemetry.ppu_scroll_seam.nametable_wrap_scroll_y,
        telemetry.ppu_scroll_seam.nametable_wrap_mirroring
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam nametable-wrap frames/cycles/passed | {} / {} / {} |",
        telemetry.ppu_scroll_seam.nametable_wrap_frames,
        telemetry.ppu_scroll_seam.nametable_wrap_cycles,
        telemetry.ppu_scroll_seam.nametable_wrap_passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam nametable-wrap error | {} |",
        optional_string(telemetry.ppu_scroll_seam.nametable_wrap_error.as_deref())
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam cases / expected | {} / {} |",
        telemetry.ppu_scroll_seam.observed_case_count,
        telemetry.ppu_scroll_seam.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Scroll-seam passed | {} |",
        telemetry.ppu_scroll_seam.passed
    )
    .expect("write report");
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
        "| Transfer count / total active cycles | {} / {} |",
        telemetry.dma.oam_dma_transfer_count, telemetry.dma.oam_dma_total_active_cycles
    )
    .expect("write report");
    writeln!(
        report,
        "| Phase matrix transfers / expected | {} / {} |",
        telemetry.dma.oam_dma_phase_matrix_test_transfer_count,
        telemetry.dma.oam_dma_phase_matrix_expected_test_transfers
    )
    .expect("write report");
    writeln!(
        report,
        "| Phase matrix parity coverage | even={} odd={} buckets={:?} |",
        telemetry.dma.oam_dma_phase_matrix_has_even_start,
        telemetry.dma.oam_dma_phase_matrix_has_odd_start,
        telemetry.dma.oam_dma_active_cycle_buckets
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
        "| DMC overlap transfer indices / offsets | {:?} / {:?} |",
        telemetry.dma.dmc_dma_oam_overlap_transfer_indices,
        telemetry.dma.dmc_dma_oam_overlap_offsets
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC overlap phase-matrix burst train | transfers {:?}, distinct {} / expected-min {}, passed={} |",
        telemetry
            .dma
            .dmc_dma_oam_overlap_phase_matrix_transfer_indices,
        telemetry
            .dma
            .dmc_dma_oam_overlap_phase_matrix_distinct_transfer_count,
        telemetry
            .dma
            .dmc_dma_oam_overlap_expected_min_phase_matrix_transfers,
        telemetry.dma.dmc_dma_oam_overlap_burst_train_passed
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC overlap placement buckets | observed {:?}, covered {:?}, missing {:?}, expected-min {} |",
        telemetry.dma.dmc_dma_oam_overlap_position_buckets,
        telemetry.dma.dmc_dma_oam_overlap_covered_position_buckets,
        telemetry.dma.dmc_dma_oam_overlap_missing_position_buckets,
        telemetry.dma.dmc_dma_oam_overlap_expected_min_position_buckets
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

fn write_audio_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## APU Audio Output").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| Sample count / expected | {} / {}..={} |",
        telemetry.audio.sample_count,
        telemetry.audio.expected_min_sample_count,
        telemetry.audio.expected_max_sample_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Peak abs / expected | {} / {}..={} |",
        format_audio_level(telemetry.audio.peak_abs),
        format_audio_level(telemetry.audio.expected_min_peak_abs),
        format_audio_level(telemetry.audio.expected_max_peak_abs)
    )
    .expect("write report");
    writeln!(
        report,
        "| RMS abs / expected | {} / {}..={} |",
        format_audio_level(telemetry.audio.rms_abs),
        format_audio_level(telemetry.audio.expected_min_rms_abs),
        format_audio_level(telemetry.audio.expected_max_rms_abs)
    )
    .expect("write report");
    writeln!(
        report,
        "| Mean abs / expected | {} / {}..={} |",
        format_audio_level(telemetry.audio.mean_abs),
        format_audio_level(telemetry.audio.expected_min_mean_abs),
        format_audio_level(telemetry.audio.expected_max_mean_abs)
    )
    .expect("write report");
    writeln!(
        report,
        "| Audio envelope passed | sample_count={} peak={} rms={} mean={} overall={} |",
        telemetry.audio.sample_count_passed,
        telemetry.audio.peak_abs_passed,
        telemetry.audio.rms_abs_passed,
        telemetry.audio.mean_abs_passed,
        telemetry.audio.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| Status matrix mask / expected | {} / {} |",
        telemetry.apu_status_matrix.observed_mask_hex,
        telemetry.apu_status_matrix.expected_mask_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Status matrix channels | pulse1={} pulse2={} triangle={} noise={} |",
        telemetry.apu_status_matrix.pulse1_status_bit,
        telemetry.apu_status_matrix.pulse2_status_bit,
        telemetry.apu_status_matrix.triangle_status_bit,
        telemetry.apu_status_matrix.noise_status_bit
    )
    .expect("write report");
    writeln!(
        report,
        "| Status matrix cases / expected | {} / {} |",
        telemetry.apu_status_matrix.observed_case_count,
        telemetry.apu_status_matrix.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Status matrix passed | {} |",
        telemetry.apu_status_matrix.passed
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC status bit / expected | {} / {} |",
        telemetry.apu_dmc_status.observed_bit_hex, telemetry.apu_dmc_status.expected_bit_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| DMC status active / cases / passed | {} / {}/{} / {} |",
        telemetry.apu_dmc_status.dmc_status_bit,
        telemetry.apu_dmc_status.observed_case_count,
        telemetry.apu_dmc_status.expected_case_count,
        telemetry.apu_dmc_status.passed
    )
    .expect("write report");
    writeln!(report).expect("write report");
}

fn write_cpu_branch_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## CPU Branch Matrix").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| Branch taken mask / expected | {} / {} |",
        telemetry.cpu_branch_matrix.taken_mask_hex, telemetry.cpu_branch_matrix.expected_mask_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Branch not-taken mask / expected | {} / {} |",
        telemetry.cpu_branch_matrix.not_taken_mask_hex,
        telemetry.cpu_branch_matrix.expected_mask_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Branch page-cross result / expected | {} / {} |",
        telemetry.cpu_branch_matrix.page_cross_result_hex,
        telemetry.cpu_branch_matrix.expected_page_cross_result_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Branch matrix cases / expected | {} / {} |",
        telemetry.cpu_branch_matrix.observed_case_count,
        telemetry.cpu_branch_matrix.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Branch matrix passed | {} |",
        telemetry.cpu_branch_matrix.passed
    )
    .expect("write report");
    writeln!(report).expect("write report");
}

fn write_cpu_stack_section(report: &mut String, telemetry: &DiagnosticTelemetry) {
    writeln!(report, "## CPU Stack Matrix").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(
        report,
        "| TXS/TSX stack pointer / expected | {} / {} |",
        telemetry.cpu_stack_matrix.tsx_result_hex,
        telemetry.cpu_stack_matrix.expected_stack_pointer_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| PHA/PLA pull result / expected | {} / {} |",
        telemetry.cpu_stack_matrix.pull_result_hex,
        hex_byte(CPU_STACK_MATRIX_EXPECTED_PULL_RESULT)
    )
    .expect("write report");
    writeln!(
        report,
        "| PHP/PLP status result / expected | {} / {} |",
        telemetry.cpu_stack_matrix.status_result_hex,
        hex_byte(CPU_STACK_MATRIX_EXPECTED_STATUS_RESULT)
    )
    .expect("write report");
    writeln!(
        report,
        "| JSR/RTS result / expected | {} / {} |",
        telemetry.cpu_stack_matrix.jsr_result_hex,
        hex_byte(CPU_STACK_MATRIX_EXPECTED_JSR_RESULT)
    )
    .expect("write report");
    writeln!(
        report,
        "| Final stack pointer / expected | {} / {} |",
        telemetry.cpu_stack_matrix.final_stack_pointer_hex,
        telemetry.cpu_stack_matrix.expected_stack_pointer_hex
    )
    .expect("write report");
    writeln!(
        report,
        "| Stack matrix cases / expected | {} / {} |",
        telemetry.cpu_stack_matrix.observed_case_count,
        telemetry.cpu_stack_matrix.expected_case_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Stack matrix passed | {} |",
        telemetry.cpu_stack_matrix.passed
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
        0x08 => OpcodeDecode::implied("PHP"),
        0x18 => OpcodeDecode::implied("CLC"),
        0x20 => OpcodeDecode::absolute("JSR"),
        0x28 => OpcodeDecode::implied("PLP"),
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
        0x8A => OpcodeDecode::implied("TXA"),
        0x8D => OpcodeDecode::absolute("STA"),
        0x8E => OpcodeDecode::absolute("STX"),
        0x90 => OpcodeDecode::relative("BCC"),
        0x95 => OpcodeDecode::zero_page_x("STA"),
        0x9A => OpcodeDecode::implied("TXS"),
        0x9D => OpcodeDecode::absolute_x("STA"),
        0xA0 => OpcodeDecode::immediate("LDY"),
        0xA2 => OpcodeDecode::immediate("LDX"),
        0xA5 => OpcodeDecode::zero_page("LDA"),
        0xA9 => OpcodeDecode::immediate("LDA"),
        0xAD => OpcodeDecode::absolute("LDA"),
        0xB1 => OpcodeDecode::indirect_y("LDA"),
        0xB5 => OpcodeDecode::zero_page_x("LDA"),
        0xBA => OpcodeDecode::implied("TSX"),
        0xBD => OpcodeDecode::absolute_x("LDA"),
        0xC9 => OpcodeDecode::immediate("CMP"),
        0xD0 => OpcodeDecode::relative("BNE"),
        0xD8 => OpcodeDecode::implied("CLD"),
        0xE0 => OpcodeDecode::immediate("CPX"),
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

    fn indirect_y(mnemonic: &'static str) -> Self {
        Self {
            mnemonic,
            addressing_mode: "indirect_y",
            byte_len: 2,
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
        "indirect_y" => format!("{} (${:02X}),Y", decode.mnemonic, operand_bytes[0]),
        "relative" => {
            let target = (pc as i32 + 2 + operand_bytes[0] as i8 as i32) as u16;
            format!("{} {}", decode.mnemonic, format_pc(target))
        }
        _ => decode.mnemonic.to_string(),
    }
}

#[derive(Debug, Default)]
struct OamDmaTransferObservation {
    oam_dma_start_cycle: Option<u64>,
    oam_dma_end_cycle: Option<u64>,
    oam_dma_first_active_cycle: Option<u64>,
    oam_dma_first_active_cycle_even: Option<bool>,
    oam_dma_active_cycles: u64,
    oam_dma_start_test: Option<u8>,
    oam_dma_end_test: Option<u8>,
}

impl OamDmaTransferObservation {
    fn start(cycle: u64, current_test: u8) -> Self {
        Self {
            oam_dma_start_cycle: Some(cycle),
            oam_dma_start_test: known_test_id(current_test),
            ..Self::default()
        }
    }

    fn observe_active_cycle(&mut self, cycle: u64) {
        if self.oam_dma_first_active_cycle.is_none() {
            self.oam_dma_first_active_cycle = Some(cycle);
            self.oam_dma_first_active_cycle_even = Some(cycle.is_multiple_of(2));
        }
        self.oam_dma_active_cycles += 1;
    }

    fn complete(&mut self, cycle: u64, current_test: u8) {
        self.oam_dma_end_cycle = Some(cycle);
        self.oam_dma_end_test = known_test_id(current_test);
    }
}

#[derive(Debug)]
struct DmcOamOverlapObservation {
    transfer_index: usize,
    cycle: u64,
}

#[derive(Debug, Default)]
struct DmaObservation {
    oam_dma_transfers: Vec<OamDmaTransferObservation>,
    current_oam_dma_transfer: Option<OamDmaTransferObservation>,
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
    dmc_dma_oam_overlap_observations: Vec<DmcOamOverlapObservation>,
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
        if !tick.active_before && tick.active_after {
            self.current_oam_dma_transfer = Some(OamDmaTransferObservation::start(
                tick.cycle,
                tick.current_test,
            ));
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

        if tick.active_before {
            if self.current_oam_dma_transfer.is_none() {
                self.current_oam_dma_transfer = Some(OamDmaTransferObservation::default());
            }
            if let Some(transfer) = self.current_oam_dma_transfer.as_mut() {
                transfer.observe_active_cycle(tick.cycle);
            }
        }
        if tick.dmc_stall_before {
            if tick.active_before {
                self.dmc_dma_queued_during_oam_dma_cycles += 1;
            } else {
                self.dmc_dma_stall_cycles += 1;
                if !self.oam_dma_transfers.is_empty() {
                    self.dmc_dma_stall_cycles_after_oam_dma += 1;
                }
            }
        }

        if tick.active_before && !tick.active_after {
            if let Some(mut transfer) = self.current_oam_dma_transfer.take() {
                transfer.complete(tick.cycle, tick.current_test);
                self.oam_dma_transfers.push(transfer);
            }
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
                if self
                    .current_oam_dma_transfer
                    .as_ref()
                    .is_some_and(|transfer| transfer.oam_dma_first_active_cycle.is_some())
                {
                    self.dmc_dma_oam_overlap_observations
                        .push(DmcOamOverlapObservation {
                            transfer_index: self.oam_dma_transfers.len() + 1,
                            cycle: tick.cycle,
                        });
                }
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

    fn oam_transfer_by_index(&self, index: usize) -> Option<&OamDmaTransferObservation> {
        if index == 0 {
            return None;
        }
        self.oam_dma_transfers.get(index - 1).or_else(|| {
            (index == self.oam_dma_transfers.len() + 1)
                .then_some(self.current_oam_dma_transfer.as_ref())
                .flatten()
        })
    }

    fn telemetry(&self) -> DmaTelemetry {
        let first_transfer = self
            .oam_dma_transfers
            .first()
            .or(self.current_oam_dma_transfer.as_ref());
        let oam_dma_transfer_count = self.oam_dma_transfers.len();
        let oam_dma_total_active_cycles = self
            .oam_dma_transfers
            .iter()
            .map(|transfer| transfer.oam_dma_active_cycles)
            .sum();
        let oam_dma_active_cycle_buckets: Vec<u64> = self
            .oam_dma_transfers
            .iter()
            .map(|transfer| transfer.oam_dma_active_cycles)
            .collect();
        let oam_dma_active_cycle_parities: Vec<&'static str> = self
            .oam_dma_transfers
            .iter()
            .filter_map(|transfer| transfer.oam_dma_first_active_cycle_even)
            .map(cycle_parity_label)
            .collect();
        let oam_dma_phase_matrix_test_transfer_count = self
            .oam_dma_transfers
            .iter()
            .filter(|transfer| transfer.oam_dma_start_test == Some(DMA_PHASE_MATRIX_TEST_ID))
            .count();
        let oam_dma_phase_matrix_has_even_start = oam_dma_active_cycle_parities.contains(&"even");
        let oam_dma_phase_matrix_has_odd_start = oam_dma_active_cycle_parities.contains(&"odd");
        let oam_dma_phase_matrix_buckets_in_range = self.oam_dma_transfers.iter().all(|transfer| {
            transfer.oam_dma_active_cycles >= OAM_DMA_EXPECTED_MIN_CYCLES
                && transfer.oam_dma_active_cycles <= OAM_DMA_EXPECTED_MAX_CYCLES
        });
        let dmc_dma_oam_overlap_offsets: Vec<u64> = self
            .dmc_dma_oam_overlap_observations
            .iter()
            .filter_map(|overlap| {
                self.oam_transfer_by_index(overlap.transfer_index)
                    .and_then(|transfer| transfer.oam_dma_first_active_cycle)
                    .map(|first_active_cycle| overlap.cycle.saturating_sub(first_active_cycle))
            })
            .collect();
        let dmc_dma_oam_overlap_transfer_indices: Vec<usize> = self
            .dmc_dma_oam_overlap_observations
            .iter()
            .filter_map(|overlap| {
                self.oam_transfer_by_index(overlap.transfer_index)
                    .map(|_| overlap.transfer_index)
            })
            .collect();
        let dmc_dma_oam_overlap_phase_matrix_transfer_indices: Vec<usize> = self
            .dmc_dma_oam_overlap_observations
            .iter()
            .filter_map(|overlap| {
                let transfer = self.oam_transfer_by_index(overlap.transfer_index)?;
                (transfer.oam_dma_start_test == Some(DMA_PHASE_MATRIX_TEST_ID))
                    .then_some(overlap.transfer_index)
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let dmc_dma_oam_overlap_phase_matrix_distinct_transfer_count =
            dmc_dma_oam_overlap_phase_matrix_transfer_indices.len();
        let dmc_dma_oam_overlap_burst_train_passed =
            dmc_dma_oam_overlap_phase_matrix_distinct_transfer_count
                >= DMC_DMA_OAM_OVERLAP_EXPECTED_MIN_PHASE_MATRIX_TRANSFERS;
        let dmc_dma_oam_overlap_position_buckets: Vec<&'static str> = self
            .dmc_dma_oam_overlap_observations
            .iter()
            .filter_map(|overlap| {
                let transfer = self.oam_transfer_by_index(overlap.transfer_index)?;
                let first_active_cycle = transfer.oam_dma_first_active_cycle?;
                let offset = overlap.cycle.saturating_sub(first_active_cycle);
                Some(dmc_oam_overlap_position_bucket(
                    offset,
                    transfer.oam_dma_active_cycles,
                ))
            })
            .collect();
        let dmc_dma_oam_overlap_covered_position_buckets =
            dmc_oam_overlap_covered_position_buckets(&dmc_dma_oam_overlap_position_buckets);
        let dmc_dma_oam_overlap_expected_position_buckets =
            DMC_DMA_OAM_OVERLAP_POSITION_BUCKETS.to_vec();
        let dmc_dma_oam_overlap_missing_position_buckets: Vec<&'static str> =
            dmc_dma_oam_overlap_expected_position_buckets
                .iter()
                .copied()
                .filter(|expected| {
                    !dmc_dma_oam_overlap_covered_position_buckets
                        .iter()
                        .any(|covered| covered == expected)
                })
                .collect();
        let dmc_dma_oam_overlap_position_matrix_passed =
            dmc_dma_oam_overlap_covered_position_buckets.len()
                >= DMC_DMA_OAM_OVERLAP_EXPECTED_MIN_POSITION_BUCKETS;

        DmaTelemetry {
            oam_dma_observed: first_transfer.is_some(),
            oam_dma_completed: !self.oam_dma_transfers.is_empty(),
            oam_dma_active_cycles: first_transfer
                .map(|transfer| transfer.oam_dma_active_cycles)
                .unwrap_or_default(),
            oam_dma_expected_min_cycles: OAM_DMA_EXPECTED_MIN_CYCLES,
            oam_dma_expected_max_cycles: OAM_DMA_EXPECTED_MAX_CYCLES,
            oam_dma_start_cycle: first_transfer.and_then(|transfer| transfer.oam_dma_start_cycle),
            oam_dma_end_cycle: first_transfer.and_then(|transfer| transfer.oam_dma_end_cycle),
            oam_dma_first_active_cycle: first_transfer
                .and_then(|transfer| transfer.oam_dma_first_active_cycle),
            oam_dma_first_active_cycle_parity: first_transfer
                .and_then(|transfer| transfer.oam_dma_first_active_cycle_even)
                .map(cycle_parity_label),
            oam_dma_start_test: first_transfer.and_then(|transfer| transfer.oam_dma_start_test),
            oam_dma_start_test_name: first_transfer
                .and_then(|transfer| transfer.oam_dma_start_test)
                .and_then(test_name),
            oam_dma_end_test: first_transfer.and_then(|transfer| transfer.oam_dma_end_test),
            oam_dma_end_test_name: first_transfer
                .and_then(|transfer| transfer.oam_dma_end_test)
                .and_then(test_name),
            oam_dma_transfer_count,
            oam_dma_total_active_cycles,
            oam_dma_active_cycle_buckets,
            oam_dma_active_cycle_parities,
            oam_dma_phase_matrix_expected_total_transfers:
                DMA_PHASE_MATRIX_EXPECTED_TOTAL_TRANSFERS,
            oam_dma_phase_matrix_expected_test_transfers: DMA_PHASE_MATRIX_EXPECTED_TEST_TRANSFERS,
            oam_dma_phase_matrix_test_transfer_count,
            oam_dma_phase_matrix_has_even_start,
            oam_dma_phase_matrix_has_odd_start,
            oam_dma_phase_matrix_passed: oam_dma_transfer_count
                >= DMA_PHASE_MATRIX_EXPECTED_TOTAL_TRANSFERS
                && oam_dma_phase_matrix_test_transfer_count
                    >= DMA_PHASE_MATRIX_EXPECTED_TEST_TRANSFERS
                && oam_dma_phase_matrix_has_even_start
                && oam_dma_phase_matrix_has_odd_start
                && oam_dma_phase_matrix_buckets_in_range,
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
            dmc_dma_oam_overlap_offsets,
            dmc_dma_oam_overlap_transfer_indices,
            dmc_dma_oam_overlap_phase_matrix_transfer_indices,
            dmc_dma_oam_overlap_phase_matrix_distinct_transfer_count,
            dmc_dma_oam_overlap_expected_min_phase_matrix_transfers:
                DMC_DMA_OAM_OVERLAP_EXPECTED_MIN_PHASE_MATRIX_TRANSFERS,
            dmc_dma_oam_overlap_burst_train_passed,
            dmc_dma_oam_overlap_position_buckets,
            dmc_dma_oam_overlap_covered_position_buckets,
            dmc_dma_oam_overlap_expected_position_buckets,
            dmc_dma_oam_overlap_missing_position_buckets,
            dmc_dma_oam_overlap_expected_min_position_buckets:
                DMC_DMA_OAM_OVERLAP_EXPECTED_MIN_POSITION_BUCKETS,
            dmc_dma_oam_overlap_position_matrix_passed,
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

fn dmc_oam_overlap_position_bucket(offset: u64, active_cycles: u64) -> &'static str {
    if active_cycles == 0 {
        return "unknown";
    }
    if offset.saturating_mul(3) < active_cycles {
        "beginning"
    } else if offset.saturating_mul(3) < active_cycles.saturating_mul(2) {
        "middle"
    } else {
        "end"
    }
}

fn dmc_oam_overlap_covered_position_buckets(
    position_buckets: &[&'static str],
) -> Vec<&'static str> {
    DMC_DMA_OAM_OVERLAP_POSITION_BUCKETS
        .iter()
        .copied()
        .filter(|expected| position_buckets.iter().any(|bucket| bucket == expected))
        .collect()
}

#[derive(Debug, Default)]
struct PpuVblankTimingObservation {
    wait_loop_start_cycle: Option<u64>,
    wait_loop_start_frame: Option<u64>,
    first_nmi_cycle: Option<u64>,
    first_nmi_frame: Option<u64>,
    second_nmi_cycle: Option<u64>,
    second_nmi_frame: Option<u64>,
    last_nmi_count: u8,
    edge: PpuVblankEdgeObservation,
}

#[derive(Debug, Clone, Copy)]
struct PpuVblankEdgeSample {
    cpu_cycle: u64,
    frame: u64,
    ppu_scanline: i16,
    ppu_dot: u16,
    ppu_phase: u8,
}

#[derive(Debug, Clone, Copy)]
struct PpuVblankTickObservation {
    cpu_cycle: u64,
    frame: u64,
    ppu_phase: u8,
    before: PpuTimingState,
    after: PpuTimingState,
    nmi_triggered: bool,
}

#[derive(Debug, Default)]
struct PpuVblankEdgeObservation {
    first_set: Option<PpuVblankEdgeSample>,
    first_clear: Option<PpuVblankEdgeSample>,
    second_set: Option<PpuVblankEdgeSample>,
    set_count: u8,
    clear_count: u8,
    nmi_trigger_count: u8,
}

impl PpuVblankEdgeObservation {
    fn observe_ppu_tick(&mut self, tick: PpuVblankTickObservation) {
        let before_vblank = tick.before.status & 0x80 != 0;
        let after_vblank = tick.after.status & 0x80 != 0;
        let sample = PpuVblankEdgeSample {
            cpu_cycle: tick.cpu_cycle,
            frame: tick.frame,
            ppu_scanline: tick.before.scanline,
            ppu_dot: tick.before.dot,
            ppu_phase: tick.ppu_phase,
        };

        if !before_vblank && after_vblank {
            self.set_count = self.set_count.saturating_add(1);
            if self.first_set.is_none() {
                self.first_set = Some(sample);
            } else if self.second_set.is_none() {
                self.second_set = Some(sample);
            }
        } else if before_vblank && !after_vblank {
            self.clear_count = self.clear_count.saturating_add(1);
            if self.first_clear.is_none() {
                self.first_clear = Some(sample);
            }
        }

        if tick.nmi_triggered {
            self.nmi_trigger_count = self.nmi_trigger_count.saturating_add(1);
        }
    }

    fn edge_matches(sample: Option<PpuVblankEdgeSample>, scanline: i16, dot: u16) -> bool {
        sample.is_some_and(|sample| sample.ppu_scanline == scanline && sample.ppu_dot == dot)
    }

    fn passed(&self) -> bool {
        self.set_count >= PPU_VBLANK_EDGE_EXPECTED_SET_COUNT
            && self.clear_count >= PPU_VBLANK_EDGE_EXPECTED_CLEAR_COUNT
            && Self::edge_matches(
                self.first_set,
                PPU_VBLANK_EDGE_SET_SCANLINE,
                PPU_VBLANK_EDGE_SET_DOT,
            )
            && Self::edge_matches(
                self.second_set,
                PPU_VBLANK_EDGE_SET_SCANLINE,
                PPU_VBLANK_EDGE_SET_DOT,
            )
            && Self::edge_matches(
                self.first_clear,
                PPU_VBLANK_EDGE_CLEAR_SCANLINE,
                PPU_VBLANK_EDGE_CLEAR_DOT,
            )
    }
}

impl PpuVblankTimingObservation {
    fn new(initial_nmi_count: u8) -> Self {
        Self {
            last_nmi_count: initial_nmi_count,
            ..Self::default()
        }
    }

    fn observe_wait_loop(
        &mut self,
        pc: u16,
        wait_loop_pc: u16,
        cycle: u64,
        frame: u64,
        current_test: u8,
    ) {
        if current_test == PPU_VBLANK_TIMING_TEST_ID
            && pc == wait_loop_pc
            && self.wait_loop_start_cycle.is_none()
        {
            self.wait_loop_start_cycle = Some(cycle);
            self.wait_loop_start_frame = Some(frame);
        }
    }

    fn observe_nmi_count(&mut self, cycle: u64, frame: u64, current_test: u8, nmi_count: u8) {
        if current_test == PPU_VBLANK_TIMING_TEST_ID && nmi_count > self.last_nmi_count {
            for _ in self.last_nmi_count..nmi_count {
                if self.first_nmi_cycle.is_none() {
                    self.first_nmi_cycle = Some(cycle);
                    self.first_nmi_frame = Some(frame);
                } else if self.second_nmi_cycle.is_none() {
                    self.second_nmi_cycle = Some(cycle);
                    self.second_nmi_frame = Some(frame);
                }
            }
        }
        self.last_nmi_count = nmi_count;
    }

    fn observe_ppu_tick(&mut self, current_test: u8, tick: PpuVblankTickObservation) {
        if current_test == PPU_VBLANK_TIMING_TEST_ID {
            self.edge.observe_ppu_tick(tick);
        }
    }

    fn telemetry(&self, observed_nmi_count: u8) -> PpuVblankTimingTelemetry {
        let first_nmi_latency_cycles = self
            .wait_loop_start_cycle
            .zip(self.first_nmi_cycle)
            .map(|(start, first)| first.saturating_sub(start));
        let inter_nmi_cycles = self
            .first_nmi_cycle
            .zip(self.second_nmi_cycle)
            .map(|(first, second)| second.saturating_sub(first));
        let first_nmi_in_window = first_nmi_latency_cycles.is_some_and(|cycles| {
            (PPU_VBLANK_FIRST_NMI_MIN_CYCLES..=PPU_VBLANK_FIRST_NMI_MAX_CYCLES).contains(&cycles)
        });
        let inter_nmi_in_window = inter_nmi_cycles.is_some_and(|cycles| {
            (PPU_VBLANK_INTER_NMI_MIN_CYCLES..=PPU_VBLANK_INTER_NMI_MAX_CYCLES).contains(&cycles)
        });
        let nmi_window_passed =
            observed_nmi_count >= 2 && first_nmi_in_window && inter_nmi_in_window;
        let edge_passed = self.edge.passed();

        PpuVblankTimingTelemetry {
            test_id: PPU_VBLANK_TIMING_TEST_ID,
            test_name: test_name(PPU_VBLANK_TIMING_TEST_ID),
            wait_loop_start_cycle: self.wait_loop_start_cycle,
            wait_loop_start_frame: self.wait_loop_start_frame,
            first_nmi_cycle: self.first_nmi_cycle,
            first_nmi_frame: self.first_nmi_frame,
            first_nmi_latency_cycles,
            first_nmi_expected_min_cycles: PPU_VBLANK_FIRST_NMI_MIN_CYCLES,
            first_nmi_expected_max_cycles: PPU_VBLANK_FIRST_NMI_MAX_CYCLES,
            second_nmi_cycle: self.second_nmi_cycle,
            second_nmi_frame: self.second_nmi_frame,
            inter_nmi_cycles,
            inter_nmi_expected_min_cycles: PPU_VBLANK_INTER_NMI_MIN_CYCLES,
            inter_nmi_expected_max_cycles: PPU_VBLANK_INTER_NMI_MAX_CYCLES,
            observed_nmi_count,
            nmi_window_passed,
            edge_expected_set_scanline: PPU_VBLANK_EDGE_SET_SCANLINE,
            edge_expected_set_dot: PPU_VBLANK_EDGE_SET_DOT,
            edge_expected_clear_scanline: PPU_VBLANK_EDGE_CLEAR_SCANLINE,
            edge_expected_clear_dot: PPU_VBLANK_EDGE_CLEAR_DOT,
            edge_expected_set_count: PPU_VBLANK_EDGE_EXPECTED_SET_COUNT,
            edge_expected_clear_count: PPU_VBLANK_EDGE_EXPECTED_CLEAR_COUNT,
            edge_set_count: self.edge.set_count,
            edge_clear_count: self.edge.clear_count,
            edge_nmi_trigger_count: self.edge.nmi_trigger_count,
            edge_first_set_cpu_cycle: self.edge.first_set.map(|sample| sample.cpu_cycle),
            edge_first_set_frame: self.edge.first_set.map(|sample| sample.frame),
            edge_first_set_ppu_scanline: self.edge.first_set.map(|sample| sample.ppu_scanline),
            edge_first_set_ppu_dot: self.edge.first_set.map(|sample| sample.ppu_dot),
            edge_first_set_ppu_phase: self.edge.first_set.map(|sample| sample.ppu_phase),
            edge_first_clear_cpu_cycle: self.edge.first_clear.map(|sample| sample.cpu_cycle),
            edge_first_clear_frame: self.edge.first_clear.map(|sample| sample.frame),
            edge_first_clear_ppu_scanline: self.edge.first_clear.map(|sample| sample.ppu_scanline),
            edge_first_clear_ppu_dot: self.edge.first_clear.map(|sample| sample.ppu_dot),
            edge_first_clear_ppu_phase: self.edge.first_clear.map(|sample| sample.ppu_phase),
            edge_second_set_cpu_cycle: self.edge.second_set.map(|sample| sample.cpu_cycle),
            edge_second_set_frame: self.edge.second_set.map(|sample| sample.frame),
            edge_second_set_ppu_scanline: self.edge.second_set.map(|sample| sample.ppu_scanline),
            edge_second_set_ppu_dot: self.edge.second_set.map(|sample| sample.ppu_dot),
            edge_second_set_ppu_phase: self.edge.second_set.map(|sample| sample.ppu_phase),
            edge_passed,
            passed: nmi_window_passed && edge_passed,
        }
    }
}

fn tick_ppu_for_diagnostic_cpu_cycle(
    bus: &mut Bus,
    ppu_vblank_timing: &mut PpuVblankTimingObservation,
    current_test: u8,
    cycles: u64,
    frames: u64,
) {
    for ppu_phase in 0..3 {
        let before = bus.ppu.timing_state();
        let nmi_triggered = bus.tick_ppu_once();
        let after = bus.ppu.timing_state();
        let tick = PpuVblankTickObservation {
            cpu_cycle: cycles,
            frame: frames,
            ppu_phase,
            before,
            after,
            nmi_triggered,
        };
        ppu_vblank_timing.observe_ppu_tick(current_test, tick);
    }
}

struct HostValidationInput<'a> {
    status: u8,
    timeout: bool,
    tests: &'a [TestTelemetry],
    ram: &'a [u8],
    cpu_addressing_matrix: &'a CpuAddressingMatrixTelemetry,
    cpu_branch_matrix: &'a CpuBranchMatrixTelemetry,
    cpu_stack_matrix: &'a CpuStackMatrixTelemetry,
    cpu_rmw_addressing_matrix: &'a CpuRmwAddressingMatrixTelemetry,
    cpu_rmw_matrix: &'a CpuRmwMatrixTelemetry,
    input_port_matrix: &'a InputPortMatrixTelemetry,
    input_mask_sweep: &'a InputMaskSweepTelemetry,
    apu_status_matrix: &'a ApuStatusMatrixTelemetry,
    apu_dmc_status: &'a ApuDmcStatusTelemetry,
    ppu_vblank_timing: &'a PpuVblankTimingTelemetry,
    ppu_scroll_seam: &'a PpuScrollSeamTelemetry,
    ppu_sprite_overflow: &'a PpuSpriteOverflowTelemetry,
    ppu_sprite_priority: &'a PpuSpritePriorityTelemetry,
    ppu_sprite_zero_hit: &'a PpuSpriteZeroHitTelemetry,
    mapper1_mmc1: &'a Mapper1Mmc1Telemetry,
    mapper1_mmc1_32k_prg: &'a Mapper1Mmc1Prg32kTelemetry,
    mapper3_chr_bank: &'a Mapper3ChrBankTelemetry,
    mapper4_mmc3: &'a Mapper4Mmc3Telemetry,
    mapper4_mmc3_edge: &'a Mapper4Mmc3EdgeTelemetry,
    mapper4_mmc3_prg_ram: &'a Mapper4Mmc3PrgRamTelemetry,
    mapper7_axrom: &'a Mapper7AxromTelemetry,
    dma: &'a DmaTelemetry,
    oam: &'a OamTelemetry,
    frame: &'a FrameTelemetry,
    audio: &'a AudioTelemetry,
    frames: u64,
}

struct ProbeTelemetryInput<'a> {
    status: u8,
    timeout: bool,
    current_test: u8,
    failure_code: u8,
    tests: &'a [TestTelemetry],
    ram: &'a [u8],
    cpu_addressing_matrix: &'a CpuAddressingMatrixTelemetry,
    cpu_branch_matrix: &'a CpuBranchMatrixTelemetry,
    cpu_stack_matrix: &'a CpuStackMatrixTelemetry,
    cpu_rmw_addressing_matrix: &'a CpuRmwAddressingMatrixTelemetry,
    cpu_rmw_matrix: &'a CpuRmwMatrixTelemetry,
    input_port_matrix: &'a InputPortMatrixTelemetry,
    input_mask_sweep: &'a InputMaskSweepTelemetry,
    apu_status_matrix: &'a ApuStatusMatrixTelemetry,
    apu_dmc_status: &'a ApuDmcStatusTelemetry,
    ppu_vblank_timing: &'a PpuVblankTimingTelemetry,
    ppu_scroll_seam: &'a PpuScrollSeamTelemetry,
    ppu_sprite_overflow: &'a PpuSpriteOverflowTelemetry,
    ppu_sprite_priority: &'a PpuSpritePriorityTelemetry,
    ppu_sprite_zero_hit: &'a PpuSpriteZeroHitTelemetry,
    mapper1_mmc1: &'a Mapper1Mmc1Telemetry,
    mapper1_mmc1_32k_prg: &'a Mapper1Mmc1Prg32kTelemetry,
    mapper3_chr_bank: &'a Mapper3ChrBankTelemetry,
    mapper4_mmc3: &'a Mapper4Mmc3Telemetry,
    mapper4_mmc3_edge: &'a Mapper4Mmc3EdgeTelemetry,
    mapper4_mmc3_prg_ram: &'a Mapper4Mmc3PrgRamTelemetry,
    mapper7_axrom: &'a Mapper7AxromTelemetry,
    dma: &'a DmaTelemetry,
    oam: &'a OamTelemetry,
    frame: &'a FrameTelemetry,
    audio: &'a AudioTelemetry,
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
    if !input.cpu_addressing_matrix.passed {
        failures.push(format!(
            "CPU addressing matrix mismatch: abs_x_no_cross={}, abs_x_page_cross={}, indirect_y_page_cross={}, cases {}/{}",
            input.cpu_addressing_matrix.abs_x_no_cross_result_hex,
            input.cpu_addressing_matrix.abs_x_page_cross_result_hex,
            input.cpu_addressing_matrix.indirect_y_page_cross_result_hex,
            input.cpu_addressing_matrix.observed_case_count,
            input.cpu_addressing_matrix.expected_case_count
        ));
    }
    if !input.cpu_branch_matrix.passed {
        failures.push(format!(
            "CPU branch matrix mismatch: taken={}, not_taken={}, page_cross={}, cases {}/{}",
            input.cpu_branch_matrix.taken_mask_hex,
            input.cpu_branch_matrix.not_taken_mask_hex,
            input.cpu_branch_matrix.page_cross_result_hex,
            input.cpu_branch_matrix.observed_case_count,
            input.cpu_branch_matrix.expected_case_count
        ));
    }
    if !input.cpu_stack_matrix.passed {
        failures.push(format!(
            "CPU stack matrix mismatch: tsx={}, pull={}, status={}, jsr={}, final_sp={}, cases {}/{}",
            input.cpu_stack_matrix.tsx_result_hex,
            input.cpu_stack_matrix.pull_result_hex,
            input.cpu_stack_matrix.status_result_hex,
            input.cpu_stack_matrix.jsr_result_hex,
            input.cpu_stack_matrix.final_stack_pointer_hex,
            input.cpu_stack_matrix.observed_case_count,
            input.cpu_stack_matrix.expected_case_count
        ));
    }
    if !input.cpu_rmw_matrix.passed {
        failures.push(format!(
            "CPU read-modify-write matrix mismatch: asl={}, rol={}, lsr={}, ror={}, inc={}, dec={}, cases {}/{}",
            input.cpu_rmw_matrix.asl_result_hex,
            input.cpu_rmw_matrix.rol_result_hex,
            input.cpu_rmw_matrix.lsr_result_hex,
            input.cpu_rmw_matrix.ror_result_hex,
            input.cpu_rmw_matrix.inc_result_hex,
            input.cpu_rmw_matrix.dec_result_hex,
            input.cpu_rmw_matrix.observed_case_count,
            input.cpu_rmw_matrix.expected_case_count
        ));
    }
    if !input.cpu_rmw_addressing_matrix.passed {
        failures.push(format!(
            "CPU RMW addressing matrix mismatch: asl_abs={}, rol_abs_x={}, lsr_abs={}, ror_abs_x={}, inc_abs={}, dec_abs_x={}, cases {}/{}",
            input.cpu_rmw_addressing_matrix.asl_abs_result_hex,
            input.cpu_rmw_addressing_matrix.rol_abs_x_result_hex,
            input.cpu_rmw_addressing_matrix.lsr_abs_result_hex,
            input.cpu_rmw_addressing_matrix.ror_abs_x_result_hex,
            input.cpu_rmw_addressing_matrix.inc_abs_result_hex,
            input.cpu_rmw_addressing_matrix.dec_abs_x_result_hex,
            input.cpu_rmw_addressing_matrix.observed_case_count,
            input.cpu_rmw_addressing_matrix.expected_case_count
        ));
    }
    if !input.input_port_matrix.passed {
        failures.push(format!(
            "input port matrix mismatch: j1_high={}/{}, j2_high={}/{}, j1_overread={}/{}, j2_overread={}/{}, cases {}/{}",
            input.input_port_matrix.joypad1_high_first_hex,
            input.input_port_matrix.joypad1_high_second_hex,
            input.input_port_matrix.joypad2_high_first_hex,
            input.input_port_matrix.joypad2_high_second_hex,
            input.input_port_matrix.joypad1_overread_first_hex,
            input.input_port_matrix.joypad1_overread_second_hex,
            input.input_port_matrix.joypad2_overread_first_hex,
            input.input_port_matrix.joypad2_overread_second_hex,
            input.input_port_matrix.observed_case_count,
            input.input_port_matrix.expected_case_count
        ));
    }
    if !input.input_mask_sweep.passed {
        failures.push(format!(
            "input mask sweep mismatch: cases {}/{}, passed {}, failed {}, error {}",
            input.input_mask_sweep.observed_case_count,
            input.input_mask_sweep.expected_case_count,
            input.input_mask_sweep.passed_case_count,
            input.input_mask_sweep.failed_case_count,
            optional_string(input.input_mask_sweep.error.as_deref())
        ));
    }
    if !input.apu_status_matrix.passed {
        failures.push(format!(
            "APU status matrix mismatch: observed mask {} expected {}, cases {}/{}, pulse1={}, pulse2={}, triangle={}, noise={}",
            input.apu_status_matrix.observed_mask_hex,
            input.apu_status_matrix.expected_mask_hex,
            input.apu_status_matrix.observed_case_count,
            input.apu_status_matrix.expected_case_count,
            input.apu_status_matrix.pulse1_status_bit,
            input.apu_status_matrix.pulse2_status_bit,
            input.apu_status_matrix.triangle_status_bit,
            input.apu_status_matrix.noise_status_bit
        ));
    }
    if !input.apu_dmc_status.passed {
        failures.push(format!(
            "APU DMC status mismatch: observed bit {} expected {}, active={}, cases {}/{}",
            input.apu_dmc_status.observed_bit_hex,
            input.apu_dmc_status.expected_bit_hex,
            input.apu_dmc_status.dmc_status_bit,
            input.apu_dmc_status.observed_case_count,
            input.apu_dmc_status.expected_case_count
        ));
    }
    if !input.ppu_vblank_timing.passed {
        failures.push(format!(
            "PPU vblank timing mismatch: wait_start={}, first_nmi={}, first_latency={} expected {}..={}, second_nmi={}, inter_nmi={} expected {}..={}, nmi_count={}, nmi_window_passed={}, edge_set_count={}/{}, edge_clear_count={}/{}, edge_nmi_triggers={}, first_set={}:{}, first_clear={}:{}, second_set={}:{}, edge_passed={}",
            optional_u64(input.ppu_vblank_timing.wait_loop_start_cycle),
            optional_u64(input.ppu_vblank_timing.first_nmi_cycle),
            optional_u64(input.ppu_vblank_timing.first_nmi_latency_cycles),
            input.ppu_vblank_timing.first_nmi_expected_min_cycles,
            input.ppu_vblank_timing.first_nmi_expected_max_cycles,
            optional_u64(input.ppu_vblank_timing.second_nmi_cycle),
            optional_u64(input.ppu_vblank_timing.inter_nmi_cycles),
            input.ppu_vblank_timing.inter_nmi_expected_min_cycles,
            input.ppu_vblank_timing.inter_nmi_expected_max_cycles,
            input.ppu_vblank_timing.observed_nmi_count,
            input.ppu_vblank_timing.nmi_window_passed,
            input.ppu_vblank_timing.edge_set_count,
            input.ppu_vblank_timing.edge_expected_set_count,
            input.ppu_vblank_timing.edge_clear_count,
            input.ppu_vblank_timing.edge_expected_clear_count,
            input.ppu_vblank_timing.edge_nmi_trigger_count,
            optional_i16(input.ppu_vblank_timing.edge_first_set_ppu_scanline),
            optional_u16(input.ppu_vblank_timing.edge_first_set_ppu_dot),
            optional_i16(input.ppu_vblank_timing.edge_first_clear_ppu_scanline),
            optional_u16(input.ppu_vblank_timing.edge_first_clear_ppu_dot),
            optional_i16(input.ppu_vblank_timing.edge_second_set_ppu_scanline),
            optional_u16(input.ppu_vblank_timing.edge_second_set_ppu_dot),
            input.ppu_vblank_timing.edge_passed
        ));
    }
    if !input.ppu_sprite_zero_hit.passed {
        failures.push(format!(
            "PPU sprite-zero-hit mismatch: status bit {}, cases {}/{}",
            input.ppu_sprite_zero_hit.observed_status_bit_hex,
            input.ppu_sprite_zero_hit.observed_case_count,
            input.ppu_sprite_zero_hit.expected_case_count
        ));
    }
    if !input.ppu_sprite_overflow.passed {
        failures.push(format!(
            "PPU sprite-overflow mismatch: true_positive={}, false_positive={}, false_negative={}, cases {}/{}, hardware_bug_matrix_passed={}",
            input.ppu_sprite_overflow.observed_status_bit_hex,
            input.ppu_sprite_overflow
                .false_positive_observed_status_bit_hex,
            input.ppu_sprite_overflow
                .false_negative_observed_status_bit_hex,
            input.ppu_sprite_overflow.observed_case_count,
            input.ppu_sprite_overflow.expected_case_count,
            input.ppu_sprite_overflow.hardware_bug_matrix_passed
        ));
    }
    if !input.ppu_sprite_priority.passed {
        failures.push(format!(
            "PPU sprite-priority mismatch: front sample ({}, {}) {} expected {}, behind sample ({}, {}) {} expected {}, cases {}/{}",
            input.ppu_sprite_priority.front_sample_x,
            input.ppu_sprite_priority.front_sample_y,
            input.ppu_sprite_priority.front_observed_color_hex,
            input.ppu_sprite_priority.front_expected_color_hex,
            input.ppu_sprite_priority.behind_sample_x,
            input.ppu_sprite_priority.behind_sample_y,
            input.ppu_sprite_priority.behind_observed_color_hex,
            input.ppu_sprite_priority.behind_expected_color_hex,
            input.ppu_sprite_priority.observed_case_count,
            input.ppu_sprite_priority.expected_case_count
        ));
    }
    if !input.ppu_scroll_seam.passed {
        failures.push(format!(
            "PPU scroll-seam mismatch: left sample ({}, {}) {} expected {}, right sample ({}, {}) {} expected {}, coarse-left sample ({}, {}) {} expected {}, coarse-right sample ({}, {}) {} expected {}, nametable-wrap-left sample ({}, {}) {} expected {}, nametable-wrap-right sample ({}, {}) {} expected {}, top sample ({}, {}) {} expected {}, bottom sample ({}, {}) {} expected {}, scroll {}/{}, coarse scroll {}, nametable-wrap scroll {}/{}, nametable-wrap mirroring {}, cases {}/{}, nametable-wrap error {}",
            input.ppu_scroll_seam.left_sample_x,
            input.ppu_scroll_seam.left_sample_y,
            input.ppu_scroll_seam.left_observed_color_hex,
            input.ppu_scroll_seam.left_expected_color_hex,
            input.ppu_scroll_seam.right_sample_x,
            input.ppu_scroll_seam.right_sample_y,
            input.ppu_scroll_seam.right_observed_color_hex,
            input.ppu_scroll_seam.right_expected_color_hex,
            input.ppu_scroll_seam.coarse_left_sample_x,
            input.ppu_scroll_seam.coarse_left_sample_y,
            input.ppu_scroll_seam.coarse_left_observed_color_hex,
            input.ppu_scroll_seam.coarse_left_expected_color_hex,
            input.ppu_scroll_seam.coarse_right_sample_x,
            input.ppu_scroll_seam.coarse_right_sample_y,
            input.ppu_scroll_seam.coarse_right_observed_color_hex,
            input.ppu_scroll_seam.coarse_right_expected_color_hex,
            input.ppu_scroll_seam.nametable_wrap_left_sample_x,
            input.ppu_scroll_seam.nametable_wrap_left_sample_y,
            input.ppu_scroll_seam.nametable_wrap_left_observed_color_hex,
            input.ppu_scroll_seam.nametable_wrap_left_expected_color_hex,
            input.ppu_scroll_seam.nametable_wrap_right_sample_x,
            input.ppu_scroll_seam.nametable_wrap_right_sample_y,
            input.ppu_scroll_seam.nametable_wrap_right_observed_color_hex,
            input.ppu_scroll_seam.nametable_wrap_right_expected_color_hex,
            input.ppu_scroll_seam.top_sample_x,
            input.ppu_scroll_seam.top_sample_y,
            input.ppu_scroll_seam.top_observed_color_hex,
            input.ppu_scroll_seam.top_expected_color_hex,
            input.ppu_scroll_seam.bottom_sample_x,
            input.ppu_scroll_seam.bottom_sample_y,
            input.ppu_scroll_seam.bottom_observed_color_hex,
            input.ppu_scroll_seam.bottom_expected_color_hex,
            input.ppu_scroll_seam.scroll_x,
            input.ppu_scroll_seam.scroll_y,
            input.ppu_scroll_seam.coarse_scroll_x,
            input.ppu_scroll_seam.nametable_wrap_scroll_x,
            input.ppu_scroll_seam.nametable_wrap_scroll_y,
            input.ppu_scroll_seam.nametable_wrap_mirroring,
            input.ppu_scroll_seam.observed_case_count,
            input.ppu_scroll_seam.expected_case_count,
            optional_string(input.ppu_scroll_seam.nametable_wrap_error.as_deref())
        ));
    }
    if !input.mapper1_mmc1.passed {
        failures.push(format!(
            "Mapper 1 MMC1 variant mismatch: PRG observed {:?} expected {:?}, CHR observed {:?} expected {:?}, mirror observed {:?} expected {:?}, cases {}/{}, cycles={}, error {}",
            input.mapper1_mmc1.observed_prg_values_hex,
            input.mapper1_mmc1.expected_prg_values_hex,
            input.mapper1_mmc1.observed_chr_values_hex,
            input.mapper1_mmc1.expected_chr_values_hex,
            input.mapper1_mmc1.observed_mirror_values_hex,
            input.mapper1_mmc1.expected_mirror_values_hex,
            input.mapper1_mmc1.observed_case_count,
            input.mapper1_mmc1.expected_case_count,
            input.mapper1_mmc1.cycles,
            optional_string(input.mapper1_mmc1.error.as_deref())
        ));
    }
    if !input.mapper1_mmc1_32k_prg.passed {
        failures.push(format!(
            "Mapper 1 MMC1 32 KiB PRG variant mismatch: control writes {:?}, PRG writes {:?}, observed {:?} expected {:?}, cases {}/{}, cycles={}, error {}",
            input.mapper1_mmc1_32k_prg.control_writes_hex,
            input.mapper1_mmc1_32k_prg.prg_bank_writes_hex,
            input.mapper1_mmc1_32k_prg.observed_values_hex,
            input.mapper1_mmc1_32k_prg.expected_values_hex,
            input.mapper1_mmc1_32k_prg.observed_case_count,
            input.mapper1_mmc1_32k_prg.expected_case_count,
            input.mapper1_mmc1_32k_prg.cycles,
            optional_string(input.mapper1_mmc1_32k_prg.error.as_deref())
        ));
    }
    if !input.mapper3_chr_bank.passed {
        failures.push(format!(
            "Mapper 3 CHR-bank variant mismatch: read {}, observed {:?} expected {:?}, cases {}/{}, cycles={}, error {}",
            input.mapper3_chr_bank.read_addr_hex,
            input.mapper3_chr_bank.observed_values_hex,
            input.mapper3_chr_bank.expected_values_hex,
            input.mapper3_chr_bank.observed_case_count,
            input.mapper3_chr_bank.expected_case_count,
            input.mapper3_chr_bank.cycles,
            optional_string(input.mapper3_chr_bank.error.as_deref())
        ));
    }
    if !input.mapper4_mmc3.passed {
        failures.push(format!(
            "Mapper 4 MMC3 variant mismatch: PRG observed {:?} expected {:?}, CHR observed {:?} expected {:?}, mirror observed {:?} expected {:?}, IRQ {}/{}, cases {}/{}, cycles={}, error {}",
            input.mapper4_mmc3.observed_prg_values_hex,
            input.mapper4_mmc3.expected_prg_values_hex,
            input.mapper4_mmc3.observed_chr_values_hex,
            input.mapper4_mmc3.expected_chr_values_hex,
            input.mapper4_mmc3.observed_mirror_values_hex,
            input.mapper4_mmc3.expected_mirror_values_hex,
            input.mapper4_mmc3.observed_irq_count,
            input.mapper4_mmc3.expected_irq_count,
            input.mapper4_mmc3.observed_case_count,
            input.mapper4_mmc3.expected_case_count,
            input.mapper4_mmc3.cycles,
            optional_string(input.mapper4_mmc3.error.as_deref())
        ));
    }
    if !input.mapper4_mmc3_edge.passed {
        failures.push(format!(
            "Mapper 4 MMC3 edge variant mismatch: PRG observed {:?} expected {:?}, CHR observed {:?} expected {:?}, IRQ observed {:?} expected {:?}, cases {}/{}, cycles={}, error {}",
            input.mapper4_mmc3_edge.observed_prg_values_hex,
            input.mapper4_mmc3_edge.expected_prg_values_hex,
            input.mapper4_mmc3_edge.observed_chr_values_hex,
            input.mapper4_mmc3_edge.expected_chr_values_hex,
            input.mapper4_mmc3_edge.observed_irq_counts,
            input.mapper4_mmc3_edge.expected_irq_counts,
            input.mapper4_mmc3_edge.observed_case_count,
            input.mapper4_mmc3_edge.expected_case_count,
            input.mapper4_mmc3_edge.cycles,
            optional_string(input.mapper4_mmc3_edge.error.as_deref())
        ));
    }
    if !input.mapper4_mmc3_prg_ram.passed {
        failures.push(format!(
            "Mapper 4 MMC3 PRG RAM variant mismatch: battery={}, read addrs {:?}, observed {:?} expected {:?}, SRAM snapshot {:?}, restored {:?} expected {:?}, cases {}/{}, cycles={}, error {}",
            input.mapper4_mmc3_prg_ram.battery_backed,
            input.mapper4_mmc3_prg_ram.read_addrs_hex,
            input.mapper4_mmc3_prg_ram.observed_values_hex,
            input.mapper4_mmc3_prg_ram.expected_values_hex,
            input.mapper4_mmc3_prg_ram.sram_snapshot_values_hex,
            input.mapper4_mmc3_prg_ram.restored_values_hex,
            MAPPER4_PRG_RAM_RESTORED_VALUES
                .iter()
                .map(|value| hex_byte(*value))
                .collect::<Vec<_>>(),
            input.mapper4_mmc3_prg_ram.observed_case_count,
            input.mapper4_mmc3_prg_ram.expected_case_count,
            input.mapper4_mmc3_prg_ram.cycles,
            optional_string(input.mapper4_mmc3_prg_ram.error.as_deref())
        ));
    }
    if !input.mapper7_axrom.passed {
        failures.push(format!(
            "Mapper 7 AxROM variant mismatch: PRG observed {:?} expected {:?}, mirror observed {:?} expected {:?}, cases {}/{}, cycles={}, error {}",
            input.mapper7_axrom.observed_prg_values_hex,
            input.mapper7_axrom.expected_prg_values_hex,
            input.mapper7_axrom.observed_mirror_values_hex,
            input.mapper7_axrom.expected_mirror_values_hex,
            input.mapper7_axrom.observed_case_count,
            input.mapper7_axrom.expected_case_count,
            input.mapper7_axrom.cycles,
            optional_string(input.mapper7_axrom.error.as_deref())
        ));
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
    if !input.dma.oam_dma_phase_matrix_passed {
        failures.push(format!(
            "OAM DMA phase matrix incomplete: transfers {}/{}, test transfers {}/{}, even={}, odd={}, buckets={:?}",
            input.dma.oam_dma_transfer_count,
            input.dma.oam_dma_phase_matrix_expected_total_transfers,
            input.dma.oam_dma_phase_matrix_test_transfer_count,
            input.dma.oam_dma_phase_matrix_expected_test_transfers,
            input.dma.oam_dma_phase_matrix_has_even_start,
            input.dma.oam_dma_phase_matrix_has_odd_start,
            input.dma.oam_dma_active_cycle_buckets
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
    if !input.dma.dmc_dma_oam_overlap_position_matrix_passed {
        failures.push(format!(
            "DMC/OAM overlap placement coverage incomplete: covered {:?}, missing {:?}, offsets {:?}, expected at least {} distinct buckets",
            input.dma.dmc_dma_oam_overlap_covered_position_buckets,
            input.dma.dmc_dma_oam_overlap_missing_position_buckets,
            input.dma.dmc_dma_oam_overlap_offsets,
            input.dma.dmc_dma_oam_overlap_expected_min_position_buckets
        ));
    }
    if !input.dma.dmc_dma_oam_overlap_burst_train_passed {
        failures.push(format!(
            "DMC/OAM burst-train coverage incomplete: phase-matrix overlap transfers {:?}, distinct count {}, expected at least {}",
            input.dma.dmc_dma_oam_overlap_phase_matrix_transfer_indices,
            input.dma.dmc_dma_oam_overlap_phase_matrix_distinct_transfer_count,
            input
                .dma
                .dmc_dma_oam_overlap_expected_min_phase_matrix_transfers
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
        if input.dma.dmc_dma_stall_cycles_after_oam_dma < u64::from(stall_cycles) {
            failures.push(format!(
                "post-OAM DMC stall count {} did not include overlap service bucket {}",
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
    if input.frame.checksum_validation_enabled && !input.frame.checksum_matches_expected {
        failures.push(format!(
            "PPU render-frame checksum mismatch: got {}, expected {}",
            input.frame.checksum_hex, input.frame.expected_checksum_hex
        ));
    }
    if !input.audio.passed {
        failures.push(format!(
            "APU audio envelope outside expected windows: samples {} expected {}..={}, peak {} expected {}..={}, rms {} expected {}..={}, mean {} expected {}..={}",
            input.audio.sample_count,
            input.audio.expected_min_sample_count,
            input.audio.expected_max_sample_count,
            format_audio_level(input.audio.peak_abs),
            format_audio_level(input.audio.expected_min_peak_abs),
            format_audio_level(input.audio.expected_max_peak_abs),
            format_audio_level(input.audio.rms_abs),
            format_audio_level(input.audio.expected_min_rms_abs),
            format_audio_level(input.audio.expected_max_rms_abs),
            format_audio_level(input.audio.mean_abs),
            format_audio_level(input.audio.expected_min_mean_abs),
            format_audio_level(input.audio.expected_max_mean_abs)
        ));
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
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "cpu.addressing_matrix.results".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cpu),
            test_id: Some(22),
            test_name: test_name(22),
            status: gated_probe_status(passed_suite, input.cpu_addressing_matrix.passed),
            description:
                "CPU addressing matrix retained expected load sentinels for non-crossing and page-crossing cases"
                    .to_string(),
            expected:
                "absolute,X no-cross=0x34, absolute,X page-cross=0x56, indirect,Y page-cross=0x56, cases=3"
                    .to_string(),
            observed: format!(
                "absolute,X no-cross {}, absolute,X page-cross {}, indirect,Y page-cross {}, cases {}/{}",
                input.cpu_addressing_matrix.abs_x_no_cross_result_hex,
                input.cpu_addressing_matrix.abs_x_page_cross_result_hex,
                input.cpu_addressing_matrix.indirect_y_page_cross_result_hex,
                input.cpu_addressing_matrix.observed_case_count,
                input.cpu_addressing_matrix.expected_case_count
            ),
            likely_domain: "cpu.addressing.page_cross_load".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "cpu.branch_matrix.results".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cpu),
            test_id: Some(CPU_BRANCH_MATRIX_TEST_ID),
            test_name: test_name(CPU_BRANCH_MATRIX_TEST_ID),
            status: gated_probe_status(passed_suite, input.cpu_branch_matrix.passed),
            description:
                "CPU branch condition matrix retained expected taken, not-taken, and page-crossing branch observations"
                    .to_string(),
            expected: "taken=0xFF, not_taken=0xFF, page_cross=0x5C, cases=17".to_string(),
            observed: format!(
                "taken {}, not_taken {}, page_cross {}, cases {}/{}",
                input.cpu_branch_matrix.taken_mask_hex,
                input.cpu_branch_matrix.not_taken_mask_hex,
                input.cpu_branch_matrix.page_cross_result_hex,
                input.cpu_branch_matrix.observed_case_count,
                input.cpu_branch_matrix.expected_case_count
            ),
            likely_domain: "cpu.branch.condition_matrix".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "cpu.stack_matrix.results".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cpu),
            test_id: Some(CPU_STACK_MATRIX_TEST_ID),
            test_name: test_name(CPU_STACK_MATRIX_TEST_ID),
            status: gated_probe_status(passed_suite, input.cpu_stack_matrix.passed),
            description:
                "CPU stack status matrix retained expected stack pointer, push/pop, status, and JSR/RTS observations"
                    .to_string(),
            expected: "tsx=0xF0, pull=0xA6, status=0xA9, jsr=0x77, final_sp=0xF0, cases=5"
                .to_string(),
            observed: format!(
                "tsx {}, pull {}, status {}, jsr {}, final_sp {}, cases {}/{}",
                input.cpu_stack_matrix.tsx_result_hex,
                input.cpu_stack_matrix.pull_result_hex,
                input.cpu_stack_matrix.status_result_hex,
                input.cpu_stack_matrix.jsr_result_hex,
                input.cpu_stack_matrix.final_stack_pointer_hex,
                input.cpu_stack_matrix.observed_case_count,
                input.cpu_stack_matrix.expected_case_count
            ),
            likely_domain: "cpu.stack.status_matrix".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "cpu.rmw_matrix.results".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cpu),
            test_id: Some(CPU_RMW_MATRIX_TEST_ID),
            test_name: test_name(CPU_RMW_MATRIX_TEST_ID),
            status: gated_probe_status(passed_suite, input.cpu_rmw_matrix.passed),
            description:
                "CPU read-modify-write matrix retained expected zero-page memory write-back results"
                    .to_string(),
            expected: "ASL=0x80, ROL=0x01, LSR=0x40, ROR=0x80, INC=0x00, DEC=0xFF, cases=6"
                .to_string(),
            observed: format!(
                "ASL {}, ROL {}, LSR {}, ROR {}, INC {}, DEC {}, cases {}/{}",
                input.cpu_rmw_matrix.asl_result_hex,
                input.cpu_rmw_matrix.rol_result_hex,
                input.cpu_rmw_matrix.lsr_result_hex,
                input.cpu_rmw_matrix.ror_result_hex,
                input.cpu_rmw_matrix.inc_result_hex,
                input.cpu_rmw_matrix.dec_result_hex,
                input.cpu_rmw_matrix.observed_case_count,
                input.cpu_rmw_matrix.expected_case_count
            ),
            likely_domain: "cpu.rmw.asl".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "cpu.rmw_addressing_matrix.results".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cpu),
            test_id: Some(CPU_RMW_ADDRESSING_MATRIX_TEST_ID),
            test_name: test_name(CPU_RMW_ADDRESSING_MATRIX_TEST_ID),
            status: gated_probe_status(passed_suite, input.cpu_rmw_addressing_matrix.passed),
            description:
                "CPU RMW addressing matrix retained expected absolute and absolute,X memory write-back results"
                    .to_string(),
            expected: "ASL abs=0x80, ROL abs,X=0x01, LSR abs=0x40, ROR abs,X=0x80, INC abs=0x00, DEC abs,X=0xFF, cases=6"
                .to_string(),
            observed: format!(
                "ASL abs {}, ROL abs,X {}, LSR abs {}, ROR abs,X {}, INC abs {}, DEC abs,X {}, cases {}/{}",
                input.cpu_rmw_addressing_matrix.asl_abs_result_hex,
                input.cpu_rmw_addressing_matrix.rol_abs_x_result_hex,
                input.cpu_rmw_addressing_matrix.lsr_abs_result_hex,
                input.cpu_rmw_addressing_matrix.ror_abs_x_result_hex,
                input.cpu_rmw_addressing_matrix.inc_abs_result_hex,
                input.cpu_rmw_addressing_matrix.dec_abs_x_result_hex,
                input.cpu_rmw_addressing_matrix.observed_case_count,
                input.cpu_rmw_addressing_matrix.expected_case_count
            ),
            likely_domain: "cpu.rmw.absolute_asl".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "joypad.input_port_matrix.results".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Joypad),
            test_id: Some(23),
            test_name: test_name(23),
            status: gated_probe_status(passed_suite, input.input_port_matrix.passed),
            description:
                "Input-port matrix retained expected strobe-high and overread observations for $4016 and $4017"
                    .to_string(),
            expected:
                "both ports hold the configured A bit while strobe is high, shift eight configured mask bits, overread=1, cases=24"
                    .to_string(),
            observed: format!(
                "j1_high {}/{}, j2_high {}/{}, j1_overread {}/{}, j2_overread {}/{}, cases {}/{}",
                input.input_port_matrix.joypad1_high_first_hex,
                input.input_port_matrix.joypad1_high_second_hex,
                input.input_port_matrix.joypad2_high_first_hex,
                input.input_port_matrix.joypad2_high_second_hex,
                input.input_port_matrix.joypad1_overread_first_hex,
                input.input_port_matrix.joypad1_overread_second_hex,
                input.input_port_matrix.joypad2_overread_first_hex,
                input.input_port_matrix.joypad2_overread_second_hex,
                input.input_port_matrix.observed_case_count,
                input.input_port_matrix.expected_case_count
            ),
            likely_domain: "joypad.input_port_matrix".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "joypad.input_mask_sweep.results".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Joypad),
            test_id: Some(INPUT_MASK_SWEEP_TEST_ID),
            test_name: None,
            status: gated_probe_status(passed_suite, input.input_mask_sweep.passed),
            description:
                "Generated input-mask sweep variant reconstructed serial bytes for both input ports across host-applied mask pairs"
                    .to_string(),
            expected: format!(
                "{} mask pairs each reconstruct matching $4016 and $4017 serial bytes",
                input.input_mask_sweep.expected_case_count
            ),
            observed: format!(
                "cases {}/{}, passed {}, failed {}, error {}",
                input.input_mask_sweep.observed_case_count,
                input.input_mask_sweep.expected_case_count,
                input.input_mask_sweep.passed_case_count,
                input.input_mask_sweep.failed_case_count,
                optional_string(input.input_mask_sweep.error.as_deref())
            ),
            likely_domain: "joypad.input_mask_sweep".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "apu.status_matrix".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Apu),
            test_id: Some(6),
            test_name: test_name(6),
            status: gated_probe_status(passed_suite, input.apu_status_matrix.passed),
            description:
                "Cartridge-observed non-DMC APU channel status bits are retained in host telemetry"
                    .to_string(),
            expected: format!(
                "$4015 bits 0-3 mask {}, cases {}",
                input.apu_status_matrix.expected_mask_hex,
                input.apu_status_matrix.expected_case_count
            ),
            observed: format!(
                "mask {}, pulse1={}, pulse2={}, triangle={}, noise={}, cases {}/{}",
                input.apu_status_matrix.observed_mask_hex,
                input.apu_status_matrix.pulse1_status_bit,
                input.apu_status_matrix.pulse2_status_bit,
                input.apu_status_matrix.triangle_status_bit,
                input.apu_status_matrix.noise_status_bit,
                input.apu_status_matrix.observed_case_count,
                input.apu_status_matrix.expected_case_count
            ),
            likely_domain: "apu.status_matrix".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "apu.dmc_status".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Apu),
            test_id: Some(5),
            test_name: test_name(5),
            status: gated_probe_status(passed_suite, input.apu_dmc_status.passed),
            description: "Cartridge-observed DMC active status bit is retained in host telemetry"
                .to_string(),
            expected: format!(
                "$4015 bit 4 {} during DMC setup before OAM DMA, cases {}",
                input.apu_dmc_status.expected_bit_hex, input.apu_dmc_status.expected_case_count
            ),
            observed: format!(
                "bit {}, dmc_active={}, cases {}/{}",
                input.apu_dmc_status.observed_bit_hex,
                input.apu_dmc_status.dmc_status_bit,
                input.apu_dmc_status.observed_case_count,
                input.apu_dmc_status.expected_case_count
            ),
            likely_domain: "apu.dmc_status".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ppu.sprite_zero_hit.status".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(PPU_SPRITE_ZERO_HIT_TEST_ID),
            test_name: test_name(PPU_SPRITE_ZERO_HIT_TEST_ID),
            status: gated_probe_status(passed_suite, input.ppu_sprite_zero_hit.passed),
            description:
                "Cartridge-observed PPUSTATUS sprite-zero-hit bit is retained in host telemetry"
                    .to_string(),
            expected: format!(
                "sprite-zero-hit status bit {} and cases {}",
                input.ppu_sprite_zero_hit.expected_status_bit_hex,
                input.ppu_sprite_zero_hit.expected_case_count
            ),
            observed: format!(
                "sprite-zero-hit status bit {}, cases {}/{}",
                input.ppu_sprite_zero_hit.observed_status_bit_hex,
                input.ppu_sprite_zero_hit.observed_case_count,
                input.ppu_sprite_zero_hit.expected_case_count
            ),
            likely_domain: "ppu.sprite_zero_hit".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ppu.sprite_overflow.status".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(PPU_SPRITE_OVERFLOW_TEST_ID),
            test_name: test_name(PPU_SPRITE_OVERFLOW_TEST_ID),
            status: gated_probe_status(passed_suite, input.ppu_sprite_overflow.passed),
            description:
                "Cartridge-observed PPUSTATUS sprite-overflow bit is retained in host telemetry"
                    .to_string(),
            expected: format!(
                "sprite-overflow status bit {}, false-positive bit {}, false-negative bit {}, cases {}",
                input.ppu_sprite_overflow.expected_status_bit_hex,
                input.ppu_sprite_overflow
                    .false_positive_expected_status_bit_hex,
                input.ppu_sprite_overflow
                    .false_negative_expected_status_bit_hex,
                input.ppu_sprite_overflow.expected_case_count
            ),
            observed: format!(
                "sprite-overflow status bit {}, false_positive {}, false_negative {}, cases {}/{}, restored OAM bytes {}",
                input.ppu_sprite_overflow.observed_status_bit_hex,
                input.ppu_sprite_overflow
                    .false_positive_observed_status_bit_hex,
                input.ppu_sprite_overflow
                    .false_negative_observed_status_bit_hex,
                input.ppu_sprite_overflow.observed_case_count,
                input.ppu_sprite_overflow.expected_case_count,
                input.ppu_sprite_overflow.restored_oam_byte_count
            ),
            likely_domain: "ppu.sprite_overflow".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ppu.sprite_overflow.hardware_bug_matrix".to_string(),
            source: DiagnosticProbeSource::CartridgeResult,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(PPU_SPRITE_OVERFLOW_TEST_ID),
            test_name: test_name(PPU_SPRITE_OVERFLOW_TEST_ID),
            status: gated_probe_status(
                passed_suite,
                input.ppu_sprite_overflow.hardware_bug_matrix_passed,
            ),
            description:
                "Cartridge-observed sprite-overflow hardware-bug false-positive and false-negative subcases match expected status bits"
                    .to_string(),
            expected: format!(
                "true_positive={}, false_positive={}, false_negative={}",
                input.ppu_sprite_overflow.expected_status_bit_hex,
                input.ppu_sprite_overflow
                    .false_positive_expected_status_bit_hex,
                input.ppu_sprite_overflow
                    .false_negative_expected_status_bit_hex
            ),
            observed: format!(
                "true_positive={}, false_positive={}, false_negative={}",
                input.ppu_sprite_overflow.observed_status_bit_hex,
                input.ppu_sprite_overflow
                    .false_positive_observed_status_bit_hex,
                input.ppu_sprite_overflow
                    .false_negative_observed_status_bit_hex
            ),
            likely_domain: "ppu.sprite_overflow.hardware_bug".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ppu.sprite_priority.samples".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(PPU_SPRITE_PRIORITY_TEST_ID),
            test_name: test_name(PPU_SPRITE_PRIORITY_TEST_ID),
            status: gated_probe_status(passed_suite, input.ppu_sprite_priority.passed),
            description:
                "Host-sampled frame pixels prove front-priority and behind-background sprite mux behavior"
                    .to_string(),
            expected: format!(
                "front sample ({}, {}) {}, behind sample ({}, {}) {}, cases {}",
                input.ppu_sprite_priority.front_sample_x,
                input.ppu_sprite_priority.front_sample_y,
                input.ppu_sprite_priority.front_expected_color_hex,
                input.ppu_sprite_priority.behind_sample_x,
                input.ppu_sprite_priority.behind_sample_y,
                input.ppu_sprite_priority.behind_expected_color_hex,
                input.ppu_sprite_priority.expected_case_count
            ),
            observed: format!(
                "front sample {}, behind sample {}, cases {}/{}",
                input.ppu_sprite_priority.front_observed_color_hex,
                input.ppu_sprite_priority.behind_observed_color_hex,
                input.ppu_sprite_priority.observed_case_count,
                input.ppu_sprite_priority.expected_case_count
            ),
            likely_domain: "ppu.sprite_priority".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ppu.scroll_seam.samples".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(PPU_SCROLL_SEAM_TEST_ID),
            test_name: test_name(PPU_SCROLL_SEAM_TEST_ID),
            status: gated_probe_status(passed_suite, input.ppu_scroll_seam.passed),
            description:
                "Host-sampled frame pixels prove fine-X, coarse-X, coarse-X nametable-wrap, and vertical scroll seams"
                    .to_string(),
            expected: format!(
                "left sample ({}, {}) {}, right sample ({}, {}) {}, coarse-left sample ({}, {}) {}, coarse-right sample ({}, {}) {}, nametable-wrap-left sample ({}, {}) {}, nametable-wrap-right sample ({}, {}) {}, top sample ({}, {}) {}, bottom sample ({}, {}) {}, scroll {}/{}, coarse scroll {}, nametable-wrap scroll {}/{}, nametable-wrap mirroring {}, cases {}",
                input.ppu_scroll_seam.left_sample_x,
                input.ppu_scroll_seam.left_sample_y,
                input.ppu_scroll_seam.left_expected_color_hex,
                input.ppu_scroll_seam.right_sample_x,
                input.ppu_scroll_seam.right_sample_y,
                input.ppu_scroll_seam.right_expected_color_hex,
                input.ppu_scroll_seam.coarse_left_sample_x,
                input.ppu_scroll_seam.coarse_left_sample_y,
                input.ppu_scroll_seam.coarse_left_expected_color_hex,
                input.ppu_scroll_seam.coarse_right_sample_x,
                input.ppu_scroll_seam.coarse_right_sample_y,
                input.ppu_scroll_seam.coarse_right_expected_color_hex,
                input.ppu_scroll_seam.nametable_wrap_left_sample_x,
                input.ppu_scroll_seam.nametable_wrap_left_sample_y,
                input.ppu_scroll_seam.nametable_wrap_left_expected_color_hex,
                input.ppu_scroll_seam.nametable_wrap_right_sample_x,
                input.ppu_scroll_seam.nametable_wrap_right_sample_y,
                input.ppu_scroll_seam.nametable_wrap_right_expected_color_hex,
                input.ppu_scroll_seam.top_sample_x,
                input.ppu_scroll_seam.top_sample_y,
                input.ppu_scroll_seam.top_expected_color_hex,
                input.ppu_scroll_seam.bottom_sample_x,
                input.ppu_scroll_seam.bottom_sample_y,
                input.ppu_scroll_seam.bottom_expected_color_hex,
                input.ppu_scroll_seam.scroll_x,
                input.ppu_scroll_seam.scroll_y,
                input.ppu_scroll_seam.coarse_scroll_x,
                input.ppu_scroll_seam.nametable_wrap_scroll_x,
                input.ppu_scroll_seam.nametable_wrap_scroll_y,
                input.ppu_scroll_seam.nametable_wrap_mirroring,
                input.ppu_scroll_seam.expected_case_count
            ),
            observed: format!(
                "left sample {}, right sample {}, coarse-left sample {}, coarse-right sample {}, nametable-wrap-left sample {}, nametable-wrap-right sample {}, top sample {}, bottom sample {}, nametable-wrap frames/cycles/passed {}/{}/{}, cases {}/{}",
                input.ppu_scroll_seam.left_observed_color_hex,
                input.ppu_scroll_seam.right_observed_color_hex,
                input.ppu_scroll_seam.coarse_left_observed_color_hex,
                input.ppu_scroll_seam.coarse_right_observed_color_hex,
                input.ppu_scroll_seam.nametable_wrap_left_observed_color_hex,
                input.ppu_scroll_seam.nametable_wrap_right_observed_color_hex,
                input.ppu_scroll_seam.top_observed_color_hex,
                input.ppu_scroll_seam.bottom_observed_color_hex,
                input.ppu_scroll_seam.nametable_wrap_frames,
                input.ppu_scroll_seam.nametable_wrap_cycles,
                input.ppu_scroll_seam.nametable_wrap_passed,
                input.ppu_scroll_seam.observed_case_count,
                input.ppu_scroll_seam.expected_case_count
            ),
            likely_domain: "ppu.scroll_seam".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "mapper1.mmc1_shift_register".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cartridge),
            test_id: Some(MAPPER1_MMC1_TEST_ID),
            test_name: None,
            status: gated_probe_status(passed_suite, input.mapper1_mmc1.passed),
            description:
                "Generated Mapper 1 variant commits MMC1 serial register writes and validates PRG, CHR, and mirroring paths"
                    .to_string(),
            expected: format!(
                "mapper {}, PRG writes {:?}, CHR writes {:?}, PRG values {:?}, CHR values {:?}, mirror values {:?}, cases {}",
                input.mapper1_mmc1.mapper,
                input.mapper1_mmc1.prg_bank_writes_hex,
                input.mapper1_mmc1.chr_bank_writes_hex,
                input.mapper1_mmc1.expected_prg_values_hex,
                input.mapper1_mmc1.expected_chr_values_hex,
                input.mapper1_mmc1.expected_mirror_values_hex,
                input.mapper1_mmc1.expected_case_count
            ),
            observed: format!(
                "PRG values {:?}, CHR values {:?}, mirror values {:?}, cases {}/{}, cycles={}, frames={}, error {}",
                input.mapper1_mmc1.observed_prg_values_hex,
                input.mapper1_mmc1.observed_chr_values_hex,
                input.mapper1_mmc1.observed_mirror_values_hex,
                input.mapper1_mmc1.observed_case_count,
                input.mapper1_mmc1.expected_case_count,
                input.mapper1_mmc1.cycles,
                input.mapper1_mmc1.frames,
                optional_string(input.mapper1_mmc1.error.as_deref())
            ),
            likely_domain: "cartridge.mapper1_mmc1".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "mapper1.mmc1_32k_prg_mode".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cartridge),
            test_id: Some(MAPPER1_MMC1_32K_PRG_TEST_ID),
            test_name: None,
            status: gated_probe_status(passed_suite, input.mapper1_mmc1_32k_prg.passed),
            description:
                "Generated Mapper 1 variant validates MMC1 32 KiB PRG modes and ignored low PRG bank bit"
                    .to_string(),
            expected: format!(
                "mapper {}, control writes {:?}, PRG writes {:?}, read addrs {}/{}, values {:?}, cases {}",
                input.mapper1_mmc1_32k_prg.mapper,
                input.mapper1_mmc1_32k_prg.control_writes_hex,
                input.mapper1_mmc1_32k_prg.prg_bank_writes_hex,
                input.mapper1_mmc1_32k_prg.low_read_addr_hex,
                input.mapper1_mmc1_32k_prg.high_read_addr_hex,
                input.mapper1_mmc1_32k_prg.expected_values_hex,
                input.mapper1_mmc1_32k_prg.expected_case_count
            ),
            observed: format!(
                "values {:?}, cases {}/{}, cycles={}, frames={}, error {}",
                input.mapper1_mmc1_32k_prg.observed_values_hex,
                input.mapper1_mmc1_32k_prg.observed_case_count,
                input.mapper1_mmc1_32k_prg.expected_case_count,
                input.mapper1_mmc1_32k_prg.cycles,
                input.mapper1_mmc1_32k_prg.frames,
                optional_string(input.mapper1_mmc1_32k_prg.error.as_deref())
            ),
            likely_domain: "cartridge.mapper1_mmc1".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "mapper3.chr_bank_switch".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cartridge),
            test_id: Some(MAPPER3_CHR_BANK_TEST_ID),
            test_name: None,
            status: gated_probe_status(passed_suite, input.mapper3_chr_bank.passed),
            description:
                "Generated Mapper 3 variant switches CNROM CHR banks and reads distinct pattern bytes through PPUDATA"
                    .to_string(),
            expected: format!(
                "mapper {}, CHR banks {:?}, read {}, values {:?}, cases {}",
                input.mapper3_chr_bank.mapper,
                input.mapper3_chr_bank.expected_banks,
                input.mapper3_chr_bank.read_addr_hex,
                input.mapper3_chr_bank.expected_values_hex,
                input.mapper3_chr_bank.expected_case_count
            ),
            observed: format!(
                "values {:?}, cases {}/{}, cycles={}, frames={}, error {}",
                input.mapper3_chr_bank.observed_values_hex,
                input.mapper3_chr_bank.observed_case_count,
                input.mapper3_chr_bank.expected_case_count,
                input.mapper3_chr_bank.cycles,
                input.mapper3_chr_bank.frames,
                optional_string(input.mapper3_chr_bank.error.as_deref())
            ),
            likely_domain: "cartridge.mapper3_chr_bank".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "mapper4.mmc3_banks_irq".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cartridge),
            test_id: Some(MAPPER4_MMC3_TEST_ID),
            test_name: None,
            status: gated_probe_status(passed_suite, input.mapper4_mmc3.passed),
            description:
                "Generated Mapper 4 variant switches MMC3 PRG/CHR banks, toggles mirroring, and observes a scanline IRQ"
                    .to_string(),
            expected: format!(
                "mapper {}, PRG writes {:?}, CHR writes {:?}, PRG values {:?}, CHR values {:?}, mirror values {:?}, IRQ {}, cases {}",
                input.mapper4_mmc3.mapper,
                input.mapper4_mmc3.prg_register_writes_hex,
                input.mapper4_mmc3.chr_register_writes_hex,
                input.mapper4_mmc3.expected_prg_values_hex,
                input.mapper4_mmc3.expected_chr_values_hex,
                input.mapper4_mmc3.expected_mirror_values_hex,
                input.mapper4_mmc3.expected_irq_count,
                input.mapper4_mmc3.expected_case_count
            ),
            observed: format!(
                "PRG values {:?}, CHR values {:?}, mirror values {:?}, IRQ {}, cases {}/{}, cycles={}, frames={}, error {}",
                input.mapper4_mmc3.observed_prg_values_hex,
                input.mapper4_mmc3.observed_chr_values_hex,
                input.mapper4_mmc3.observed_mirror_values_hex,
                input.mapper4_mmc3.observed_irq_count,
                input.mapper4_mmc3.observed_case_count,
                input.mapper4_mmc3.expected_case_count,
                input.mapper4_mmc3.cycles,
                input.mapper4_mmc3.frames,
                optional_string(input.mapper4_mmc3.error.as_deref())
            ),
            likely_domain: "cartridge.mapper4_mmc3".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "mapper4.mmc3_inversion_irq_reload".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cartridge),
            test_id: Some(MAPPER4_MMC3_EDGE_TEST_ID),
            test_name: None,
            status: gated_probe_status(passed_suite, input.mapper4_mmc3_edge.passed),
            description:
                "Generated Mapper 4 edge variant runs from fixed $E000 while exercising MMC3 PRG inversion, CHR inversion, and IRQ reload phases"
                    .to_string(),
            expected: format!(
                "program {}, PRG selects {:?}, CHR selects {:?}, PRG values {:?}, CHR values {:?}, IRQ latches {:?} counts {:?}, cases {}",
                input.mapper4_mmc3_edge.program_base_hex,
                input.mapper4_mmc3_edge.prg_select_writes_hex,
                input.mapper4_mmc3_edge.chr_select_writes_hex,
                input.mapper4_mmc3_edge.expected_prg_values_hex,
                input.mapper4_mmc3_edge.expected_chr_values_hex,
                input.mapper4_mmc3_edge.irq_latches_hex,
                input.mapper4_mmc3_edge.expected_irq_counts,
                input.mapper4_mmc3_edge.expected_case_count
            ),
            observed: format!(
                "PRG values {:?}, CHR values {:?}, IRQ counts {:?}, cases {}/{}, cycles={}, frames={}, error {}",
                input.mapper4_mmc3_edge.observed_prg_values_hex,
                input.mapper4_mmc3_edge.observed_chr_values_hex,
                input.mapper4_mmc3_edge.observed_irq_counts,
                input.mapper4_mmc3_edge.observed_case_count,
                input.mapper4_mmc3_edge.expected_case_count,
                input.mapper4_mmc3_edge.cycles,
                input.mapper4_mmc3_edge.frames,
                optional_string(input.mapper4_mmc3_edge.error.as_deref())
            ),
            likely_domain: "cartridge.mapper4_mmc3".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "mapper4.mmc3_prg_ram_persistence".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cartridge),
            test_id: Some(MAPPER4_MMC3_PRG_RAM_TEST_ID),
            test_name: None,
            status: gated_probe_status(passed_suite, input.mapper4_mmc3_prg_ram.passed),
            description:
                "Generated Mapper 4 variant validates battery-backed PRG RAM writes and host SRAM restore into a fresh cartridge"
                    .to_string(),
            expected: format!(
                "mapper {}, battery true, PRG RAM {} bytes, read addrs {:?}, values {:?}, restored {:?} at {:?}, cases {}",
                input.mapper4_mmc3_prg_ram.mapper,
                input.mapper4_mmc3_prg_ram.prg_ram_size,
                input.mapper4_mmc3_prg_ram.read_addrs_hex,
                input.mapper4_mmc3_prg_ram.expected_values_hex,
                MAPPER4_PRG_RAM_RESTORED_VALUES
                    .iter()
                    .map(|value| hex_byte(*value))
                    .collect::<Vec<_>>(),
                input.mapper4_mmc3_prg_ram.restored_addrs_hex,
                input.mapper4_mmc3_prg_ram.expected_case_count
            ),
            observed: format!(
                "battery {}, values {:?}, SRAM snapshot {:?}, restored {:?}, cases {}/{}, cycles={}, frames={}, error {}",
                input.mapper4_mmc3_prg_ram.battery_backed,
                input.mapper4_mmc3_prg_ram.observed_values_hex,
                input.mapper4_mmc3_prg_ram.sram_snapshot_values_hex,
                input.mapper4_mmc3_prg_ram.restored_values_hex,
                input.mapper4_mmc3_prg_ram.observed_case_count,
                input.mapper4_mmc3_prg_ram.expected_case_count,
                input.mapper4_mmc3_prg_ram.cycles,
                input.mapper4_mmc3_prg_ram.frames,
                optional_string(input.mapper4_mmc3_prg_ram.error.as_deref())
            ),
            likely_domain: "cartridge.mapper4_mmc3".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "mapper7.axrom_switching".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Cartridge),
            test_id: Some(MAPPER7_AXROM_TEST_ID),
            test_name: None,
            status: gated_probe_status(passed_suite, input.mapper7_axrom.passed),
            description:
                "Generated Mapper 7 variant switches 32 KiB AxROM PRG banks and single-screen mirroring through CPU and PPU bus paths"
                    .to_string(),
            expected: format!(
                "mapper {}, writes {:?}, PRG values {:?}, mirror values {:?}, cases {}",
                input.mapper7_axrom.mapper,
                input.mapper7_axrom.bank_writes_hex,
                input.mapper7_axrom.expected_prg_values_hex,
                input.mapper7_axrom.expected_mirror_values_hex,
                input.mapper7_axrom.expected_case_count
            ),
            observed: format!(
                "PRG values {:?}, mirror values {:?}, cases {}/{}, cycles={}, frames={}, error {}",
                input.mapper7_axrom.observed_prg_values_hex,
                input.mapper7_axrom.observed_mirror_values_hex,
                input.mapper7_axrom.observed_case_count,
                input.mapper7_axrom.expected_case_count,
                input.mapper7_axrom.cycles,
                input.mapper7_axrom.frames,
                optional_string(input.mapper7_axrom.error.as_deref())
            ),
            likely_domain: "cartridge.mapper7_axrom".to_string(),
        },
    );
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
            id: "ppu.vblank_timing.nmi_window".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(PPU_VBLANK_TIMING_TEST_ID),
            test_name: test_name(PPU_VBLANK_TIMING_TEST_ID),
            status: gated_probe_status(
                should_validate_ppu_render_observations,
                input.ppu_vblank_timing.nmi_window_passed,
            ),
            description:
                "Host-observed NMI timing stays inside the expected NTSC vblank cadence window"
                    .to_string(),
            expected: format!(
                "first NMI latency {}..={} CPU cycles, inter-NMI interval {}..={} CPU cycles",
                input.ppu_vblank_timing.first_nmi_expected_min_cycles,
                input.ppu_vblank_timing.first_nmi_expected_max_cycles,
                input.ppu_vblank_timing.inter_nmi_expected_min_cycles,
                input.ppu_vblank_timing.inter_nmi_expected_max_cycles
            ),
            observed: format!(
                "wait_start={}, first_nmi={}, first_latency={}, second_nmi={}, inter_nmi={}, nmi_count={}",
                optional_u64(input.ppu_vblank_timing.wait_loop_start_cycle),
                optional_u64(input.ppu_vblank_timing.first_nmi_cycle),
                optional_u64(input.ppu_vblank_timing.first_nmi_latency_cycles),
                optional_u64(input.ppu_vblank_timing.second_nmi_cycle),
                optional_u64(input.ppu_vblank_timing.inter_nmi_cycles),
                input.ppu_vblank_timing.observed_nmi_count
            ),
            likely_domain: "ppu.vblank_timing".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "ppu.vblank_timing.edge_dots".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Ppu),
            test_id: Some(PPU_VBLANK_TIMING_TEST_ID),
            test_name: test_name(PPU_VBLANK_TIMING_TEST_ID),
            status: gated_probe_status(
                should_validate_ppu_render_observations,
                input.ppu_vblank_timing.edge_passed,
            ),
            description:
                "Host-observed PPUSTATUS vblank set/clear transitions occur on exact PPU dots"
                    .to_string(),
            expected: format!(
                "at least {} set edges at scanline {} dot {}, at least {} clear edge at scanline {} dot {}",
                input.ppu_vblank_timing.edge_expected_set_count,
                input.ppu_vblank_timing.edge_expected_set_scanline,
                input.ppu_vblank_timing.edge_expected_set_dot,
                input.ppu_vblank_timing.edge_expected_clear_count,
                input.ppu_vblank_timing.edge_expected_clear_scanline,
                input.ppu_vblank_timing.edge_expected_clear_dot
            ),
            observed: format!(
                "set_count={}, clear_count={}, nmi_triggers={}, first_set={}:{}, first_clear={}:{}, second_set={}:{}",
                input.ppu_vblank_timing.edge_set_count,
                input.ppu_vblank_timing.edge_clear_count,
                input.ppu_vblank_timing.edge_nmi_trigger_count,
                optional_i16(input.ppu_vblank_timing.edge_first_set_ppu_scanline),
                optional_u16(input.ppu_vblank_timing.edge_first_set_ppu_dot),
                optional_i16(input.ppu_vblank_timing.edge_first_clear_ppu_scanline),
                optional_u16(input.ppu_vblank_timing.edge_first_clear_ppu_dot),
                optional_i16(input.ppu_vblank_timing.edge_second_set_ppu_scanline),
                optional_u16(input.ppu_vblank_timing.edge_second_set_ppu_dot)
            ),
            likely_domain: "ppu.vblank_timing".to_string(),
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
            id: "dma.oam_phase_matrix".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Dma),
            test_id: Some(DMA_PHASE_MATRIX_TEST_ID),
            test_name: test_name(DMA_PHASE_MATRIX_TEST_ID),
            status: gated_probe_status(passed_suite, input.dma.oam_dma_phase_matrix_passed),
            description:
                "Host-observed OAM DMA phase matrix covers both odd and even start-phase cycle buckets"
                    .to_string(),
            expected: format!(
                "total OAM DMA transfers >= {}, phase-matrix transfers >= {}, both even and odd starts, each bucket {}..={}",
                input.dma.oam_dma_phase_matrix_expected_total_transfers,
                input.dma.oam_dma_phase_matrix_expected_test_transfers,
                input.dma.oam_dma_expected_min_cycles,
                input.dma.oam_dma_expected_max_cycles
            ),
            observed: format!(
                "transfers {}, phase transfers {}, parities {:?}, buckets {:?}",
                input.dma.oam_dma_transfer_count,
                input.dma.oam_dma_phase_matrix_test_transfer_count,
                input.dma.oam_dma_active_cycle_parities,
                input.dma.oam_dma_active_cycle_buckets
            ),
            likely_domain: "dma.oam_phase_matrix".to_string(),
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
            id: "dma.dmc_overlap_placement".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Dma),
            test_id: Some(5),
            test_name: test_name(5),
            status: gated_probe_status(
                passed_suite,
                input.dma.dmc_dma_oam_overlap_position_matrix_passed,
            ),
            description:
                "DMC/OAM overlap telemetry records placement offsets inside the OAM DMA stall window"
                    .to_string(),
            expected: format!(
                "covered DMC/OAM overlap positions >= {} distinct buckets from {:?}",
                input.dma.dmc_dma_oam_overlap_expected_min_position_buckets,
                input.dma.dmc_dma_oam_overlap_expected_position_buckets
            ),
            observed: format!(
                "transfer indices {:?}, offsets {:?}, buckets {:?}, covered {:?}, missing {:?}",
                input.dma.dmc_dma_oam_overlap_transfer_indices,
                input.dma.dmc_dma_oam_overlap_offsets,
                input.dma.dmc_dma_oam_overlap_position_buckets,
                input.dma.dmc_dma_oam_overlap_covered_position_buckets,
                input.dma.dmc_dma_oam_overlap_missing_position_buckets
            ),
            likely_domain: "dma.dmc_overlap_placement".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "dma.dmc_overlap_burst_train".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Dma),
            test_id: Some(DMA_PHASE_MATRIX_TEST_ID),
            test_name: test_name(DMA_PHASE_MATRIX_TEST_ID),
            status: gated_probe_status(
                passed_suite,
                input.dma.dmc_dma_oam_overlap_burst_train_passed,
            ),
            description: "DMC/OAM overlap telemetry covers a repeated phase-matrix DMA burst train"
                .to_string(),
            expected: format!(
                "DMC/OAM overlaps across >= {} distinct phase-matrix OAM DMA transfers",
                input
                    .dma
                    .dmc_dma_oam_overlap_expected_min_phase_matrix_transfers
            ),
            observed: format!(
                "phase-matrix overlap transfer indices {:?}, distinct count {}",
                input.dma.dmc_dma_oam_overlap_phase_matrix_transfer_indices,
                input
                    .dma
                    .dmc_dma_oam_overlap_phase_matrix_distinct_transfer_count
            ),
            likely_domain: "dma.dmc_oam_burst_train".to_string(),
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
                            && input.dma.dmc_dma_stall_cycles_after_oam_dma >= u64::from(cycles)
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
        frame_checksum_probe_record(
            passed_suite,
            should_validate_ppu_render_observations,
            input.frame,
        ),
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "apu.sample_count".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Apu),
            test_id: Some(6),
            test_name: test_name(6),
            status: gated_probe_status(passed_suite, input.audio.sample_count_passed),
            description: "APU produced samples that the host runner drained at frame boundaries"
                .to_string(),
            expected: format!(
                "drained audio samples {}..={}",
                input.audio.expected_min_sample_count, input.audio.expected_max_sample_count
            ),
            observed: format!("drained audio samples {}", input.audio.sample_count),
            likely_domain: "apu.frame_output".to_string(),
        },
    );
    push_probe(
        &mut probes,
        ProbeTelemetryRecord {
            id: "apu.output_envelope".to_string(),
            source: DiagnosticProbeSource::HostObservation,
            subsystem: Some(DiagnosticSubsystem::Apu),
            test_id: Some(6),
            test_name: test_name(6),
            status: gated_probe_status(passed_suite, input.audio.passed),
            description:
                "APU host output envelope stays within expected peak, RMS, and mean windows"
                    .to_string(),
            expected: format!(
                "peak {}..={}, rms {}..={}, mean {}..={}",
                format_audio_level(input.audio.expected_min_peak_abs),
                format_audio_level(input.audio.expected_max_peak_abs),
                format_audio_level(input.audio.expected_min_rms_abs),
                format_audio_level(input.audio.expected_max_rms_abs),
                format_audio_level(input.audio.expected_min_mean_abs),
                format_audio_level(input.audio.expected_max_mean_abs)
            ),
            observed: format!(
                "samples {}, peak {}, rms {}, mean {}",
                input.audio.sample_count,
                format_audio_level(input.audio.peak_abs),
                format_audio_level(input.audio.rms_abs),
                format_audio_level(input.audio.mean_abs)
            ),
            likely_domain: "apu.output_envelope".to_string(),
        },
    );

    probes
}

fn frame_checksum_probe_record(
    passed_suite: bool,
    should_validate_ppu_render_observations: bool,
    frame: &FrameTelemetry,
) -> ProbeTelemetryRecord {
    let status = if !should_validate_ppu_render_observations {
        DiagnosticProbeStatus::Skipped
    } else if !frame.checksum_validation_enabled {
        if passed_suite {
            DiagnosticProbeStatus::Passed
        } else {
            DiagnosticProbeStatus::Skipped
        }
    } else {
        passed_or_failed(frame.checksum_matches_expected)
    };

    let (description, expected) = if frame.checksum_validation_enabled {
        (
            "Rendered diagnostic frame matches the expected full-frame signature".to_string(),
            format!(
                "checksum {}, unique colors {}, nonzero pixels {}, validation {}",
                frame.expected_checksum_hex,
                frame.expected_unique_colors,
                frame.expected_nonzero_pixels,
                frame.checksum_validation_reason
            ),
        )
    } else {
        (
            "Canonical render-frame signature is recorded but not required for this fixture"
                .to_string(),
            format!(
                "canonical checksum validation disabled ({})",
                frame.checksum_validation_reason
            ),
        )
    };

    ProbeTelemetryRecord {
        id: "ppu.frame_checksum".to_string(),
        source: DiagnosticProbeSource::HostObservation,
        subsystem: Some(DiagnosticSubsystem::Ppu),
        test_id: Some(10),
        test_name: test_name(10),
        status,
        description,
        expected,
        observed: format!(
            "checksum {}, unique colors {}, nonzero pixels {}, validation {}",
            frame.checksum_hex,
            frame.unique_colors,
            frame.nonzero_pixels,
            frame.checksum_validation_reason
        ),
        likely_domain: "ppu.rendering.frame_signature".to_string(),
    }
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
    program.ppu_status_latch_reset();
    program.joypad_strobe_high_hold();
    program.cpu_addressing_mode_matrix();
    program.input_port_serial_matrix();
    program.oam_dma_phase_matrix();
    program.ppu_sprite_zero_hit();
    program.ppu_sprite_overflow();
    program.ppu_sprite_priority();
    program.ppu_scroll_seam();
    program.cpu_read_modify_write_matrix();
    program.cpu_rmw_addressing_matrix();
    program.cpu_branch_condition_matrix();
    program.cpu_stack_status_matrix();

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
        Self::new_at(PROGRAM_BASE)
    }

    fn new_at(base: u16) -> Self {
        Self {
            asm: Assembler::new(base),
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
        self.fail_with_code(fail_code);
        self.asm
            .label(&ok)
            .expect("unique label should not collide");
    }

    fn expect_a_eq_zp(&mut self, expected_addr: u8, fail_code: u8) {
        let ok = self.unique_label("ok");
        self.asm.cmp_zp(expected_addr);
        self.asm.beq(&ok);
        self.fail_with_code(fail_code);
        self.asm
            .label(&ok)
            .expect("unique label should not collide");
    }

    fn expect_z_set(&mut self, fail_code: u8) {
        let ok = self.unique_label("zero_set_ok");
        self.asm.beq(&ok);
        self.fail_with_code(fail_code);
        self.asm
            .label(&ok)
            .expect("unique label should not collide");
    }

    fn expect_c_clear(&mut self, fail_code: u8) {
        let ok = self.unique_label("carry_clear_ok");
        self.asm.bcc(&ok);
        self.fail_with_code(fail_code);
        self.asm
            .label(&ok)
            .expect("unique label should not collide");
    }

    fn fail_with_code(&mut self, fail_code: u8) {
        self.asm.lda_imm(fail_code);
        self.asm.sta_zp(FAILURE_CODE_ADDR);
        self.asm.jmp_label("fail");
    }

    fn expect_branch_taken(&mut self, opcode: u8, label_prefix: &str, mask_bit: u8, fail_code: u8) {
        let target = self.unique_label(label_prefix);
        self.asm.op_rel(opcode, &target);
        self.fail_with_code(fail_code);
        self.asm
            .label(&target)
            .expect("unique label should not collide");
        self.mark_branch_matrix_case(CPU_BRANCH_MATRIX_TAKEN_MASK_ADDR, mask_bit);
    }

    fn expect_branch_not_taken(
        &mut self,
        opcode: u8,
        label_prefix: &str,
        mask_bit: u8,
        fail_code: u8,
    ) {
        let wrong_taken = self.unique_label(label_prefix);
        let ok = self.unique_label("branch_not_taken_ok");
        self.asm.op_rel(opcode, &wrong_taken);
        self.mark_branch_matrix_case(CPU_BRANCH_MATRIX_NOT_TAKEN_MASK_ADDR, mask_bit);
        self.asm.jmp_label(&ok);
        self.asm
            .label(&wrong_taken)
            .expect("unique label should not collide");
        self.fail_with_code(fail_code);
        self.asm
            .label(&ok)
            .expect("unique label should not collide");
    }

    fn mark_branch_matrix_case(&mut self, mask_addr: u16, mask_bit: u8) {
        self.asm.lda_abs(mask_addr);
        self.asm.ora_imm(mask_bit);
        self.asm.sta_abs(mask_addr);
        self.asm.inc_abs(CPU_BRANCH_MATRIX_CASE_COUNT_ADDR);
    }

    fn set_overflow_flag(&mut self) {
        self.asm.clc();
        self.asm.lda_imm(0x40);
        self.asm.adc_imm(0x40);
    }

    fn write_ppu_data(&mut self, addr: u16, value: u8) {
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm((addr >> 8) as u8);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(addr as u8);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(value);
        self.asm.sta_abs(0x2007);
    }

    fn read_ppu_data_into_a(&mut self, addr: u16) {
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm((addr >> 8) as u8);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(addr as u8);
        self.asm.sta_abs(0x2006);
        self.asm.lda_abs(0x2007);
        self.asm.lda_abs(0x2007);
    }

    fn write_mmc1_register(&mut self, addr: u16, value: u8) {
        self.write_mmc1_register_bits(addr, value, 5);
    }

    fn write_mmc1_register_bits(&mut self, addr: u16, value: u8, bit_count: u8) {
        for bit in 0..bit_count {
            self.asm.lda_imm((value >> bit) & 0x01);
            self.asm.sta_abs(addr);
        }
    }

    fn write_mmc3_bank_register(&mut self, register: u8, value: u8) {
        self.write_mmc3_select_register(register, value);
    }

    fn write_mmc3_select_register(&mut self, select: u8, value: u8) {
        self.asm.lda_imm(select);
        self.asm.sta_abs(0x8000);
        self.asm.lda_imm(value);
        self.asm.sta_abs(0x8001);
    }

    fn increment_abs(&mut self, addr: u16) {
        self.asm.lda_abs(addr);
        self.asm.clc();
        self.asm.adc_imm(0x01);
        self.asm.sta_abs(addr);
    }

    fn wait_for_vblank(&mut self, label_prefix: &str) {
        let label = self.unique_label(label_prefix);
        self.asm
            .label(&label)
            .expect("unique label should not collide");
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(0x80);
        self.asm.cmp_imm(0x80);
        self.asm.bne(&label);
    }

    fn expect_a_matches_mask_bit(&mut self, expected_mask_addr: u8, bit_mask: u8, fail_code: u8) {
        let actual_zero = self.unique_label("actual_zero");
        let ok = self.unique_label("mask_bit_ok");

        self.asm.cmp_imm(0x00);
        self.asm.beq(&actual_zero);
        self.asm.lda_zp(expected_mask_addr);
        self.asm.and_imm(bit_mask);
        self.asm.cmp_imm(0x00);
        self.asm.bne(&ok);
        self.fail_with_code(fail_code);

        self.asm
            .label(&actual_zero)
            .expect("unique label should not collide");
        self.asm.lda_zp(expected_mask_addr);
        self.asm.and_imm(bit_mask);
        self.asm.cmp_imm(0x00);
        self.asm.beq(&ok);
        self.fail_with_code(fail_code);

        self.asm
            .label(&ok)
            .expect("unique label should not collide");
    }

    fn read_joypad_port_mask_into(&mut self, port_addr: u16, observed_addr: u16) {
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(observed_addr);
        for bit in 0..8 {
            let clear_bit = self.unique_label("joypad_bit_clear");
            self.asm.lda_abs(port_addr);
            self.asm.and_imm(0x01);
            self.asm.cmp_imm(0x00);
            self.asm.beq(&clear_bit);
            self.asm.lda_abs(observed_addr);
            self.asm.ora_imm(1 << bit);
            self.asm.sta_abs(observed_addr);
            self.asm
                .label(&clear_bit)
                .expect("unique label should not collide");
        }
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
        self.asm
            .label(CPU_RAM_MIRRORING_FAULT_LABEL)
            .expect("CPU RAM mirroring fault label should be unique");
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
        self.asm.lda_abs(0x4015);
        self.asm.and_imm(APU_DMC_STATUS_EXPECTED_BIT);
        self.asm.sta_abs(APU_DMC_STATUS_OBSERVED_BIT_ADDR);
        self.asm.lda_imm(APU_DMC_STATUS_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(APU_DMC_STATUS_CASE_COUNT_ADDR);
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
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(APU_STATUS_MATRIX_OBSERVED_MASK_ADDR);
        self.asm.sta_abs(APU_STATUS_MATRIX_CASE_COUNT_ADDR);

        self.asm.lda_imm(APU_STATUS_MATRIX_EXPECTED_MASK);
        self.asm.sta_abs(0x4015);

        self.asm.lda_imm(0x1F);
        self.asm.sta_abs(0x4000);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x4002);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x4003);

        self.asm.lda_imm(0x1F);
        self.asm.sta_abs(0x4004);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x4006);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x4007);

        self.asm.lda_imm(0xFF);
        self.asm.sta_abs(0x4008);
        self.asm.lda_imm(0xF0);
        self.asm.sta_abs(0x400A);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x400B);

        self.asm.lda_imm(0x3F);
        self.asm.sta_abs(0x400C);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x400E);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x400F);

        self.asm
            .label(APU_STATUS_FAULT_LABEL)
            .expect("fault injection label should not collide");
        self.asm.lda_abs(0x4015);
        self.asm.and_imm(APU_STATUS_MATRIX_EXPECTED_MASK);
        self.asm.sta_abs(APU_STATUS_MATRIX_OBSERVED_MASK_ADDR);
        self.expect_a_eq(APU_STATUS_MATRIX_EXPECTED_MASK, 0x61);
        self.asm.lda_imm(APU_STATUS_MATRIX_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(APU_STATUS_MATRIX_CASE_COUNT_ADDR);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4015);
        self.pass_test(6);
    }

    fn joypad_strobe_shift(&mut self) {
        self.begin_test(7);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4016);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4016);

        self.expect_serial_bits_from_mask(0x4016, JOYPAD1_EXPECTED_MASK_ADDR, 0x70);
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

    fn cpu_branch_condition_matrix(&mut self) {
        self.begin_test(CPU_BRANCH_MATRIX_TEST_ID);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(CPU_BRANCH_MATRIX_TAKEN_MASK_ADDR);
        self.asm.sta_abs(CPU_BRANCH_MATRIX_NOT_TAKEN_MASK_ADDR);
        self.asm.sta_abs(CPU_BRANCH_MATRIX_PAGE_CROSS_RESULT_ADDR);
        self.asm.sta_abs(CPU_BRANCH_MATRIX_CASE_COUNT_ADDR);

        self.asm
            .label(CPU_BRANCH_MATRIX_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");

        self.asm.lda_imm(0x01);
        self.expect_branch_taken(0x10, "bpl_taken", 0x01, 0xD2);
        self.asm.lda_imm(0x80);
        self.expect_branch_not_taken(0x10, "bpl_not_taken", 0x01, 0xD2);

        self.asm.lda_imm(0x80);
        self.expect_branch_taken(0x30, "bmi_taken", 0x02, 0xD3);
        self.asm.lda_imm(0x01);
        self.expect_branch_not_taken(0x30, "bmi_not_taken", 0x02, 0xD3);

        self.asm.clv();
        self.expect_branch_taken(0x50, "bvc_taken", 0x04, 0xD4);
        self.set_overflow_flag();
        self.expect_branch_not_taken(0x50, "bvc_not_taken", 0x04, 0xD4);

        self.set_overflow_flag();
        self.expect_branch_taken(0x70, "bvs_taken", 0x08, 0xD5);
        self.asm.clv();
        self.expect_branch_not_taken(0x70, "bvs_not_taken", 0x08, 0xD5);

        self.asm.clc();
        self.expect_branch_taken(0x90, "bcc_taken", 0x10, 0xD6);
        self.asm.sec();
        self.expect_branch_not_taken(0x90, "bcc_not_taken", 0x10, 0xD6);

        self.asm.sec();
        self.expect_branch_taken(0xB0, "bcs_taken", 0x20, 0xD7);
        self.asm.clc();
        self.expect_branch_not_taken(0xB0, "bcs_not_taken", 0x20, 0xD7);

        self.asm.lda_imm(0x01);
        self.expect_branch_taken(0xD0, "bne_taken", 0x40, 0xD8);
        self.asm.lda_imm(0x00);
        self.expect_branch_not_taken(0xD0, "bne_not_taken", 0x40, 0xD8);

        self.asm.lda_imm(0x00);
        self.expect_branch_taken(0xF0, "beq_taken", 0x80, 0xD9);
        self.asm.lda_imm(0x01);
        self.expect_branch_not_taken(0xF0, "beq_not_taken", 0x80, 0xD9);

        self.asm.lda_imm(0x01);
        self.asm.pad_until_low_byte(0xFC);
        let target = self.unique_label("branch_matrix_page_cross_target");
        self.asm.bne(&target);
        self.fail_with_code(0xDA);
        self.asm
            .label(&target)
            .expect("unique label should not collide");
        self.asm
            .lda_imm(CPU_BRANCH_MATRIX_EXPECTED_PAGE_CROSS_RESULT);
        self.asm.sta_abs(CPU_BRANCH_MATRIX_PAGE_CROSS_RESULT_ADDR);
        self.asm.inc_abs(CPU_BRANCH_MATRIX_CASE_COUNT_ADDR);

        self.asm.lda_abs(CPU_BRANCH_MATRIX_TAKEN_MASK_ADDR);
        self.expect_a_eq(CPU_BRANCH_MATRIX_EXPECTED_MASK, 0xDB);
        self.asm.lda_abs(CPU_BRANCH_MATRIX_NOT_TAKEN_MASK_ADDR);
        self.expect_a_eq(CPU_BRANCH_MATRIX_EXPECTED_MASK, 0xDC);
        self.asm.lda_abs(CPU_BRANCH_MATRIX_PAGE_CROSS_RESULT_ADDR);
        self.expect_a_eq(CPU_BRANCH_MATRIX_EXPECTED_PAGE_CROSS_RESULT, 0xDD);
        self.asm.lda_abs(CPU_BRANCH_MATRIX_CASE_COUNT_ADDR);
        self.expect_a_eq(CPU_BRANCH_MATRIX_EXPECTED_CASE_COUNT, 0xDE);
        self.pass_test(CPU_BRANCH_MATRIX_TEST_ID);
    }

    fn cpu_stack_status_matrix(&mut self) {
        self.begin_test(CPU_STACK_MATRIX_TEST_ID);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(CPU_STACK_MATRIX_TSX_RESULT_ADDR);
        self.asm.sta_abs(CPU_STACK_MATRIX_PULL_RESULT_ADDR);
        self.asm.sta_abs(CPU_STACK_MATRIX_STATUS_RESULT_ADDR);
        self.asm.sta_abs(CPU_STACK_MATRIX_JSR_RESULT_ADDR);
        self.asm.sta_abs(CPU_STACK_MATRIX_FINAL_SP_ADDR);
        self.asm.sta_abs(CPU_STACK_MATRIX_CASE_COUNT_ADDR);

        self.asm
            .label(CPU_STACK_MATRIX_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");

        self.asm.ldx_imm(CPU_STACK_MATRIX_EXPECTED_STACK_POINTER);
        self.asm.txs();
        self.asm.tsx();
        self.asm.stx_abs(CPU_STACK_MATRIX_TSX_RESULT_ADDR);
        self.asm.cpx_imm(CPU_STACK_MATRIX_EXPECTED_STACK_POINTER);
        self.expect_z_set(0xA8);
        self.asm.inc_abs(CPU_STACK_MATRIX_CASE_COUNT_ADDR);

        self.asm.lda_imm(CPU_STACK_MATRIX_EXPECTED_PULL_RESULT);
        self.asm.pha();
        self.asm.lda_imm(0x00);
        self.asm.pla();
        self.asm.sta_abs(CPU_STACK_MATRIX_PULL_RESULT_ADDR);
        self.expect_a_eq(CPU_STACK_MATRIX_EXPECTED_PULL_RESULT, 0xA9);
        self.asm.inc_abs(CPU_STACK_MATRIX_CASE_COUNT_ADDR);

        self.asm.lda_imm(0x00);
        self.asm.clc();
        self.asm.php();
        self.asm.lda_imm(0x80);
        self.asm.sec();
        self.asm.plp();
        self.expect_z_set(0xAA);
        self.expect_c_clear(0xAB);
        self.asm.lda_imm(CPU_STACK_MATRIX_EXPECTED_STATUS_RESULT);
        self.asm.sta_abs(CPU_STACK_MATRIX_STATUS_RESULT_ADDR);
        self.asm.inc_abs(CPU_STACK_MATRIX_CASE_COUNT_ADDR);

        self.asm.jsr_label("sub_stack_jsr");
        self.asm.sta_abs(CPU_STACK_MATRIX_JSR_RESULT_ADDR);
        self.expect_a_eq(CPU_STACK_MATRIX_EXPECTED_JSR_RESULT, 0xAC);
        self.asm.inc_abs(CPU_STACK_MATRIX_CASE_COUNT_ADDR);

        self.asm.tsx();
        self.asm.stx_abs(CPU_STACK_MATRIX_FINAL_SP_ADDR);
        self.asm.cpx_imm(CPU_STACK_MATRIX_EXPECTED_STACK_POINTER);
        self.expect_z_set(0xAD);
        self.asm.inc_abs(CPU_STACK_MATRIX_CASE_COUNT_ADDR);

        self.asm.lda_abs(CPU_STACK_MATRIX_CASE_COUNT_ADDR);
        self.expect_a_eq(CPU_STACK_MATRIX_EXPECTED_CASE_COUNT, 0xAE);
        self.pass_test(CPU_STACK_MATRIX_TEST_ID);
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
        for _ in 0..PPU_RENDER_FRAME_PHASE_ALIGNMENT_NOPS {
            self.asm.nop();
        }
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

        self.expect_serial_bits_from_mask(0x4017, JOYPAD2_EXPECTED_MASK_ADDR, 0xA0);
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
        self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 0x01, 0x78);
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 0x02, 0x79);

        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4016);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4016);
        self.asm
            .label(JOYPAD_STROBE_RESET_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 0x01, 0x78);
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 0x02, 0x79);
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

    fn ppu_status_latch_reset(&mut self) {
        self.begin_test(20);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2000);
        self.asm.sta_abs(0x2001);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_abs(0x2002);
        self.asm
            .label(PPU_STATUS_LATCH_RESET_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_imm(0x21);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x5D);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x21);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_abs(0x2007);
        self.asm.lda_abs(0x2007);
        self.expect_a_eq(0x5D, 0x7C);
        self.pass_test(20);
    }

    fn joypad_strobe_high_hold(&mut self) {
        self.begin_test(21);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4016);
        self.asm
            .label(JOYPAD_STROBE_HIGH_HOLD_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 0x01, 0x7D);
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 0x01, 0x7E);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4016);
        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 0x01, 0x7F);
        self.pass_test(21);
    }

    fn cpu_addressing_mode_matrix(&mut self) {
        self.begin_test(22);
        self.asm.lda_imm(0x34);
        self.asm.sta_abs(0x0442);
        self.asm.lda_imm(0x56);
        self.asm.sta_abs(0x0500);

        self.asm.ldx_imm(0x02);
        self.asm.lda_abs_x(0x0440);
        self.asm.sta_abs(CPU_ADDRESSING_MATRIX_ABS_X_NO_CROSS_ADDR);
        self.expect_a_eq(0x34, 0xB2);

        self.asm.ldx_imm(0x01);
        self.asm
            .label(CPU_ADDRESSING_MATRIX_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_abs_x(0x04FF);
        self.asm
            .sta_abs(CPU_ADDRESSING_MATRIX_ABS_X_PAGE_CROSS_ADDR);
        self.expect_a_eq(0x56, 0xB3);

        self.asm.lda_imm(0xFF);
        self.asm.sta_zp(0x42);
        self.asm.lda_imm(0x04);
        self.asm.sta_zp(0x43);
        self.asm.ldy_imm(0x01);
        self.asm.lda_indirect_y(0x42);
        self.asm
            .sta_abs(CPU_ADDRESSING_MATRIX_INDIRECT_Y_PAGE_CROSS_ADDR);
        self.expect_a_eq(0x56, 0xB4);

        self.asm.lda_imm(CPU_ADDRESSING_MATRIX_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(CPU_ADDRESSING_MATRIX_CASE_COUNT_ADDR);
        self.pass_test(22);
    }

    fn cpu_read_modify_write_matrix(&mut self) {
        self.begin_test(CPU_RMW_MATRIX_TEST_ID);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(CPU_RMW_MATRIX_CASE_COUNT_ADDR);

        self.asm.lda_imm(0x40);
        self.asm.sta_zp(0x30);
        self.asm
            .label(CPU_RMW_MATRIX_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.asl_zp(0x30);
        self.asm.lda_zp(0x30);
        self.asm.sta_abs(CPU_RMW_MATRIX_ASL_RESULT_ADDR);
        self.expect_a_eq(0x80, 0xB5);

        self.asm.lda_imm(0x80);
        self.asm.sta_zp(0x31);
        self.asm.sec();
        self.asm.rol_zp(0x31);
        self.asm.lda_zp(0x31);
        self.asm.sta_abs(CPU_RMW_MATRIX_ROL_RESULT_ADDR);
        self.expect_a_eq(0x01, 0xB6);

        self.asm.lda_imm(0x81);
        self.asm.sta_zp(0x32);
        self.asm.lsr_zp(0x32);
        self.asm.lda_zp(0x32);
        self.asm.sta_abs(CPU_RMW_MATRIX_LSR_RESULT_ADDR);
        self.expect_a_eq(0x40, 0xB7);

        self.asm.lda_imm(0x01);
        self.asm.sta_zp(0x33);
        self.asm.sec();
        self.asm.ror_zp(0x33);
        self.asm.lda_zp(0x33);
        self.asm.sta_abs(CPU_RMW_MATRIX_ROR_RESULT_ADDR);
        self.expect_a_eq(0x80, 0xB8);

        self.asm.lda_imm(0xFF);
        self.asm.sta_zp(0x34);
        self.asm.inc_zp(0x34);
        self.asm.lda_zp(0x34);
        self.asm.sta_abs(CPU_RMW_MATRIX_INC_RESULT_ADDR);
        self.expect_a_eq(0x00, 0xB9);

        self.asm.lda_imm(0x00);
        self.asm.sta_zp(0x35);
        self.asm.dec_zp(0x35);
        self.asm.lda_zp(0x35);
        self.asm.sta_abs(CPU_RMW_MATRIX_DEC_RESULT_ADDR);
        self.expect_a_eq(0xFF, 0xBA);

        self.asm.lda_imm(CPU_RMW_MATRIX_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(CPU_RMW_MATRIX_CASE_COUNT_ADDR);
        self.pass_test(CPU_RMW_MATRIX_TEST_ID);
    }

    fn cpu_rmw_addressing_matrix(&mut self) {
        self.begin_test(CPU_RMW_ADDRESSING_MATRIX_TEST_ID);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(CPU_RMW_ADDRESSING_CASE_COUNT_ADDR);

        self.asm.lda_imm(0x40);
        self.asm.sta_abs(0x0450);
        self.asm
            .label(CPU_RMW_ADDRESSING_MATRIX_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.asl_abs(0x0450);
        self.asm.lda_abs(0x0450);
        self.asm.sta_abs(CPU_RMW_ADDRESSING_ASL_ABS_RESULT_ADDR);
        self.expect_a_eq(0x80, 0xCA);

        self.asm.lda_imm(0x80);
        self.asm.sta_abs(0x0500);
        self.asm.ldx_imm(0x03);
        self.asm.sec();
        self.asm.rol_abs_x(0x04FD);
        self.asm.lda_abs(0x0500);
        self.asm.sta_abs(CPU_RMW_ADDRESSING_ROL_ABS_X_RESULT_ADDR);
        self.expect_a_eq(0x01, 0xCB);

        self.asm.lda_imm(0x81);
        self.asm.sta_abs(0x0470);
        self.asm.lsr_abs(0x0470);
        self.asm.lda_abs(0x0470);
        self.asm.sta_abs(CPU_RMW_ADDRESSING_LSR_ABS_RESULT_ADDR);
        self.expect_a_eq(0x40, 0xCC);

        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x0502);
        self.asm.ldx_imm(0x04);
        self.asm.sec();
        self.asm.ror_abs_x(0x04FE);
        self.asm.lda_abs(0x0502);
        self.asm.sta_abs(CPU_RMW_ADDRESSING_ROR_ABS_X_RESULT_ADDR);
        self.expect_a_eq(0x80, 0xCD);

        self.asm.lda_imm(0xFF);
        self.asm.sta_abs(0x0490);
        self.asm.inc_abs(0x0490);
        self.asm.lda_abs(0x0490);
        self.asm.sta_abs(CPU_RMW_ADDRESSING_INC_ABS_RESULT_ADDR);
        self.expect_a_eq(0x00, 0xCE);

        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x04FF);
        self.asm.ldx_imm(0x05);
        self.asm.dec_abs_x(0x04FA);
        self.asm.lda_abs(0x04FF);
        self.asm.sta_abs(CPU_RMW_ADDRESSING_DEC_ABS_X_RESULT_ADDR);
        self.expect_a_eq(0xFF, 0xCF);

        self.asm.lda_imm(CPU_RMW_ADDRESSING_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(CPU_RMW_ADDRESSING_CASE_COUNT_ADDR);
        self.pass_test(CPU_RMW_ADDRESSING_MATRIX_TEST_ID);
    }

    fn input_port_serial_matrix(&mut self) {
        self.begin_test(23);

        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4016);

        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.asm.sta_abs(INPUT_PORT_MATRIX_JOYPAD1_HIGH_FIRST_ADDR);
        self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 0x01, 0x92);

        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.asm.sta_abs(INPUT_PORT_MATRIX_JOYPAD1_HIGH_SECOND_ADDR);
        self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 0x01, 0x93);

        self.asm.lda_abs(0x4017);
        self.asm.and_imm(0x01);
        self.asm.sta_abs(INPUT_PORT_MATRIX_JOYPAD2_HIGH_FIRST_ADDR);
        self.expect_a_matches_mask_bit(JOYPAD2_EXPECTED_MASK_ADDR, 0x01, 0x94);

        self.asm.lda_abs(0x4017);
        self.asm.and_imm(0x01);
        self.asm.sta_abs(INPUT_PORT_MATRIX_JOYPAD2_HIGH_SECOND_ADDR);
        self.expect_a_matches_mask_bit(JOYPAD2_EXPECTED_MASK_ADDR, 0x01, 0x95);

        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4016);
        self.asm
            .label(INPUT_PORT_MATRIX_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");

        for index in 0..8 {
            self.asm.lda_abs(0x4016);
            self.asm.and_imm(0x01);
            self.expect_a_matches_mask_bit(JOYPAD1_EXPECTED_MASK_ADDR, 1 << index, 0x9A);

            self.asm.lda_abs(0x4017);
            self.asm.and_imm(0x01);
            self.expect_a_matches_mask_bit(JOYPAD2_EXPECTED_MASK_ADDR, 1 << index, 0x9B);
        }

        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.asm
            .sta_abs(INPUT_PORT_MATRIX_JOYPAD1_OVERREAD_FIRST_ADDR);
        self.expect_a_eq(0x01, 0x96);

        self.asm.lda_abs(0x4016);
        self.asm.and_imm(0x01);
        self.asm
            .sta_abs(INPUT_PORT_MATRIX_JOYPAD1_OVERREAD_SECOND_ADDR);
        self.expect_a_eq(0x01, 0x97);

        self.asm.lda_abs(0x4017);
        self.asm.and_imm(0x01);
        self.asm
            .sta_abs(INPUT_PORT_MATRIX_JOYPAD2_OVERREAD_FIRST_ADDR);
        self.expect_a_eq(0x01, 0x98);

        self.asm.lda_abs(0x4017);
        self.asm.and_imm(0x01);
        self.asm
            .sta_abs(INPUT_PORT_MATRIX_JOYPAD2_OVERREAD_SECOND_ADDR);
        self.expect_a_eq(0x01, 0x99);

        self.asm.lda_imm(INPUT_PORT_MATRIX_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(INPUT_PORT_MATRIX_CASE_COUNT_ADDR);
        self.pass_test(23);
    }

    fn oam_dma_phase_matrix(&mut self) {
        self.begin_test(DMA_PHASE_MATRIX_TEST_ID);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(DMA_PHASE_MATRIX_CASE_COUNT_ADDR);
        self.asm.sta_abs(DMA_PHASE_MATRIX_CONTROL_ADDR);

        let first_dma = self.unique_label("dma_phase_first_transfer");
        self.asm.cmp_imm(0x00);
        self.asm.beq(&first_dma);
        self.asm
            .label(&first_dma)
            .expect("unique label should not collide");
        self.asm.lda_imm(0x03);
        self.asm.sta_abs(0x4014);
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(DMA_PHASE_MATRIX_CASE_COUNT_ADDR);

        self.asm
            .label(DMA_PHASE_MATRIX_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        let second_dma = self.unique_label("dma_phase_second_transfer");
        self.asm.lda_abs(DMA_PHASE_MATRIX_CONTROL_ADDR);
        self.asm.cmp_imm(0x00);
        self.asm.beq(&second_dma);
        self.asm.lda_imm(0x83);
        self.asm.sta_zp(FAILURE_CODE_ADDR);
        self.asm.jmp_label("fail");
        self.asm
            .label(&second_dma)
            .expect("unique label should not collide");
        self.asm.lda_imm(0x03);
        self.asm.sta_abs(0x4014);
        self.asm
            .lda_imm(DMA_PHASE_MATRIX_EXPECTED_TEST_TRANSFERS as u8);
        self.asm.sta_abs(DMA_PHASE_MATRIX_CASE_COUNT_ADDR);
        self.asm.lda_abs(DMA_PHASE_MATRIX_CASE_COUNT_ADDR);
        self.expect_a_eq(DMA_PHASE_MATRIX_EXPECTED_TEST_TRANSFERS as u8, 0x84);
        self.asm.lda_imm(0x0F);
        self.asm.sta_abs(0x4010); // Fastest DMC rate, IRQ/loop disabled.
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4011);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4012); // Sample starts at $C000 in the fixed Mapper 2 PRG bank.
        self.asm.lda_imm(0x01);
        self.asm.sta_abs(0x4013); // 17 bytes, enough to request across the OAM DMA burst.
        self.asm.lda_imm(0x10);
        self.asm.sta_abs(0x4015);
        self.asm.lda_imm(0x03);
        self.asm.sta_abs(0x4014);
        for _ in 0..DMC_DMA_OAM_MIDDLE_TRANSFER_ALIGNMENT_NOPS {
            self.asm.nop();
        }
        self.asm.lda_imm(0x03);
        self.asm.sta_abs(0x4014);
        self.asm.lda_imm(0x03);
        self.asm.sta_abs(0x4014);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x4015);
        self.pass_test(DMA_PHASE_MATRIX_TEST_ID);
    }

    fn ppu_sprite_zero_hit(&mut self) {
        self.begin_test(PPU_SPRITE_ZERO_HIT_TEST_ID);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(PPU_SPRITE_ZERO_HIT_STATUS_ADDR);
        self.asm.sta_abs(PPU_SPRITE_ZERO_HIT_CASE_COUNT_ADDR);
        self.asm.sta_abs(0x2000);
        self.asm.sta_abs(0x2001);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x42);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x02);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x3F);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x0F);
        self.asm.sta_abs(0x2007);
        self.asm.lda_imm(0x21);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x3F);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x11);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x16);
        self.asm.sta_abs(0x2007);

        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2003);
        self.asm.lda_imm(0x10);
        self.asm.sta_abs(0x2004);
        self.asm.lda_imm(0x02);
        self.asm.sta_abs(0x2004);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2004);
        self.asm.lda_imm(0x10);
        self.asm.sta_abs(0x2004);

        self.asm
            .label(PPU_SPRITE_ZERO_HIT_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2005);
        self.asm.sta_abs(0x2005);
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2000);
        self.asm.lda_imm(0x18);
        self.asm.sta_abs(0x2001);

        let first_vblank = self.unique_label("sprite_zero_first_vblank");
        self.asm
            .label(&first_vblank)
            .expect("unique label should not collide");
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(0x80);
        self.asm.cmp_imm(0x80);
        self.asm.bne(&first_vblank);

        let second_vblank = self.unique_label("sprite_zero_second_vblank");
        self.asm
            .label(&second_vblank)
            .expect("unique label should not collide");
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(0x80);
        self.asm.cmp_imm(0x80);
        self.asm.bne(&second_vblank);

        self.asm.lda_abs(0x2002);
        self.asm.and_imm(PPU_SPRITE_ZERO_HIT_EXPECTED_STATUS_BIT);
        self.asm.sta_abs(PPU_SPRITE_ZERO_HIT_STATUS_ADDR);
        self.expect_a_eq(PPU_SPRITE_ZERO_HIT_EXPECTED_STATUS_BIT, 0x85);
        self.asm.lda_imm(PPU_SPRITE_ZERO_HIT_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(PPU_SPRITE_ZERO_HIT_CASE_COUNT_ADDR);
        self.restore_oam_prefix_from_dma_source(4);
        self.pass_test(PPU_SPRITE_ZERO_HIT_TEST_ID);
    }

    fn ppu_sprite_overflow(&mut self) {
        self.begin_test(PPU_SPRITE_OVERFLOW_TEST_ID);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(PPU_SPRITE_OVERFLOW_STATUS_ADDR);
        self.asm.sta_abs(PPU_SPRITE_OVERFLOW_CASE_COUNT_ADDR);
        self.asm
            .sta_abs(PPU_SPRITE_OVERFLOW_FALSE_POSITIVE_STATUS_ADDR);
        self.asm
            .sta_abs(PPU_SPRITE_OVERFLOW_FALSE_NEGATIVE_STATUS_ADDR);
        self.asm.sta_abs(0x2000);
        self.asm.sta_abs(0x2001);

        self.write_sprite_overflow_true_positive_scene();
        self.asm
            .label(PPU_SPRITE_OVERFLOW_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.render_sprite_overflow_scene(
            "sprite_overflow_true_positive",
            PPU_SPRITE_OVERFLOW_STATUS_ADDR,
        );
        self.write_sprite_overflow_false_positive_scene();
        self.render_sprite_overflow_scene(
            "sprite_overflow_false_positive",
            PPU_SPRITE_OVERFLOW_FALSE_POSITIVE_STATUS_ADDR,
        );
        self.write_sprite_overflow_false_negative_scene();
        self.render_sprite_overflow_scene(
            "sprite_overflow_false_negative",
            PPU_SPRITE_OVERFLOW_FALSE_NEGATIVE_STATUS_ADDR,
        );

        let overflow_fail = self.unique_label("sprite_overflow_fail");
        let overflow_ok = self.unique_label("sprite_overflow_ok");
        self.asm.lda_abs(0x2002);
        self.asm.lda_abs(PPU_SPRITE_OVERFLOW_STATUS_ADDR);
        self.asm.cmp_imm(PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT);
        self.asm.bne(&overflow_fail);
        self.asm
            .lda_abs(PPU_SPRITE_OVERFLOW_FALSE_POSITIVE_STATUS_ADDR);
        self.asm.cmp_imm(PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT);
        self.asm.bne(&overflow_fail);
        self.asm
            .lda_abs(PPU_SPRITE_OVERFLOW_FALSE_NEGATIVE_STATUS_ADDR);
        self.asm
            .cmp_imm(PPU_SPRITE_OVERFLOW_EXPECTED_CLEAR_STATUS_BIT);
        self.asm.bne(&overflow_fail);
        self.asm.jmp_label(&overflow_ok);

        self.asm
            .label(&overflow_fail)
            .expect("unique label should not collide");
        self.asm.lda_imm(0x86);
        self.asm.sta_zp(FAILURE_CODE_ADDR);
        self.restore_oam_prefix_from_dma_source(PPU_SPRITE_OVERFLOW_RESTORE_BYTES);
        self.asm.jmp_label("fail");

        self.asm
            .label(&overflow_ok)
            .expect("unique label should not collide");
        self.asm.lda_imm(PPU_SPRITE_OVERFLOW_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(PPU_SPRITE_OVERFLOW_CASE_COUNT_ADDR);
        self.restore_oam_prefix_from_dma_source(PPU_SPRITE_OVERFLOW_RESTORE_BYTES);
        self.pass_test(PPU_SPRITE_OVERFLOW_TEST_ID);
    }

    fn write_sprite_overflow_true_positive_scene(&mut self) {
        self.fill_oam(0xF0, "sprite_overflow_true_positive_clear_oam");
        for sprite_index in 0..9u8 {
            let x = 0x20 + sprite_index * 8;
            self.write_oam_entry_at(sprite_index, [0x30, 0x02, 0x00, x]);
        }
    }

    fn write_sprite_overflow_false_positive_scene(&mut self) {
        self.fill_oam(0xF0, "sprite_overflow_false_positive_clear_oam");
        for sprite_index in 0..8u8 {
            let x = 0x20 + sprite_index * 8;
            self.write_oam_entry_at(sprite_index, [0x30, 0x02, 0x00, x]);
        }
        self.write_oam_entry_at(9, [0xF0, 0x30, 0xF0, 0xF0]);
    }

    fn write_sprite_overflow_false_negative_scene(&mut self) {
        self.fill_oam(0xF0, "sprite_overflow_false_negative_clear_oam");
        for sprite_index in 0..8u8 {
            let x = 0x20 + sprite_index * 8;
            self.write_oam_entry_at(sprite_index, [0x30, 0x02, 0x00, x]);
        }
        self.write_oam_entry_at(9, [0x30, 0xF0, 0xF0, 0xF0]);
    }

    fn render_sprite_overflow_scene(&mut self, label_prefix: &str, status_addr: u16) {
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2005);
        self.asm.sta_abs(0x2005);
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2000);
        self.asm.lda_imm(0x18);
        self.asm.sta_abs(0x2001);
        self.wait_for_vblank(&format!("{label_prefix}_first_vblank"));
        self.wait_for_vblank(&format!("{label_prefix}_second_vblank"));
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT);
        self.asm.sta_abs(status_addr);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2001);
    }

    fn ppu_sprite_priority(&mut self) {
        self.begin_test(PPU_SPRITE_PRIORITY_TEST_ID);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(PPU_SPRITE_PRIORITY_CASE_COUNT_ADDR);
        self.asm.sta_abs(0x2000);
        self.asm.sta_abs(0x2001);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x42);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x02);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x45);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x02);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x23);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0xC0);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x3F);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x0F);
        self.asm.sta_abs(0x2007);
        self.asm.lda_imm(0x21);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x3F);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x11);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x16);
        self.asm.sta_abs(0x2007);

        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2003);
        let clear_oam = self.unique_label("sprite_priority_clear_oam");
        self.asm.ldx_imm(0x00);
        self.asm
            .label(&clear_oam)
            .expect("unique label should not collide");
        self.asm.lda_imm(0xF0);
        self.asm.sta_abs(0x2004);
        self.asm.inx();
        self.asm.bne(&clear_oam);

        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2003);
        self.write_oam_bytes(&[
            0x10, 0x02, 0x00, 0x10, // sprite palette wins in front of background
            0x10, 0x02, 0x20, 0x28, // priority bit sends sprite behind background
        ]);

        self.asm
            .label(PPU_SPRITE_PRIORITY_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_imm(PPU_SPRITE_PRIORITY_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(PPU_SPRITE_PRIORITY_CASE_COUNT_ADDR);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2005);
        self.asm.sta_abs(0x2005);
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2000);
        self.asm.lda_imm(0x18);
        self.asm.sta_abs(0x2001);

        let first_vblank = self.unique_label("sprite_priority_first_vblank");
        self.asm
            .label(&first_vblank)
            .expect("unique label should not collide");
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(0x80);
        self.asm.cmp_imm(0x80);
        self.asm.bne(&first_vblank);

        let second_vblank = self.unique_label("sprite_priority_second_vblank");
        self.asm
            .label(&second_vblank)
            .expect("unique label should not collide");
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(0x80);
        self.asm.cmp_imm(0x80);
        self.asm.bne(&second_vblank);

        self.delay_host_frame_capture();
        self.restore_oam_prefix_from_dma_source(PPU_SPRITE_OVERFLOW_RESTORE_BYTES);
        self.pass_test(PPU_SPRITE_PRIORITY_TEST_ID);
    }

    fn ppu_scroll_seam(&mut self) {
        self.begin_test(PPU_SCROLL_SEAM_TEST_ID);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(PPU_SCROLL_SEAM_CASE_COUNT_ADDR);
        self.asm.sta_abs(0x2000);
        self.asm.sta_abs(0x2001);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x40);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x02);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x41);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x03);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x42);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x02);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x20);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x60);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x03);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x23);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0xC0);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x23);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0xC7);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2007);

        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x3F);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2006);
        self.asm.lda_imm(0x0F);
        self.asm.sta_abs(0x2007);
        self.asm.lda_imm(0x21);
        self.asm.sta_abs(0x2007);
        self.asm.lda_imm(0x16);
        self.asm.sta_abs(0x2007);

        self.asm
            .label(PPU_SCROLL_SEAM_FAULT_LABEL)
            .expect("diagnostic fault-injection label should not collide");
        self.asm.lda_imm(0x04);
        self.asm.sta_abs(PPU_SCROLL_SEAM_CASE_COUNT_ADDR);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2000);
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x04);
        self.asm.sta_abs(0x2005);
        self.asm.lda_imm(0x04);
        self.asm.sta_abs(0x2005);
        self.asm.lda_imm(0x0A);
        self.asm.sta_abs(0x2001);

        let first_vblank = self.unique_label("scroll_seam_first_vblank");
        self.asm
            .label(&first_vblank)
            .expect("unique label should not collide");
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(0x80);
        self.asm.cmp_imm(0x80);
        self.asm.bne(&first_vblank);

        let second_vblank = self.unique_label("scroll_seam_second_vblank");
        self.asm
            .label(&second_vblank)
            .expect("unique label should not collide");
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(0x80);
        self.asm.cmp_imm(0x80);
        self.asm.bne(&second_vblank);

        self.delay_host_frame_capture();

        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2001);
        self.asm.lda_imm(PPU_SCROLL_SEAM_EXPECTED_CASE_COUNT);
        self.asm.sta_abs(PPU_SCROLL_SEAM_CASE_COUNT_ADDR);
        self.asm.lda_abs(0x2002);
        self.asm.lda_imm(0x08);
        self.asm.sta_abs(0x2005);
        self.asm.lda_imm(0x04);
        self.asm.sta_abs(0x2005);
        self.asm.lda_imm(0x0A);
        self.asm.sta_abs(0x2001);

        let coarse_first_vblank = self.unique_label("scroll_seam_coarse_first_vblank");
        self.asm
            .label(&coarse_first_vblank)
            .expect("unique label should not collide");
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(0x80);
        self.asm.cmp_imm(0x80);
        self.asm.bne(&coarse_first_vblank);

        let coarse_second_vblank = self.unique_label("scroll_seam_coarse_second_vblank");
        self.asm
            .label(&coarse_second_vblank)
            .expect("unique label should not collide");
        self.asm.lda_abs(0x2002);
        self.asm.and_imm(0x80);
        self.asm.cmp_imm(0x80);
        self.asm.bne(&coarse_second_vblank);

        self.delay_host_frame_capture();
        self.pass_test(PPU_SCROLL_SEAM_TEST_ID);
    }

    fn delay_host_frame_capture(&mut self) {
        for _ in 0..3 {
            let delay = self.unique_label("host_frame_capture_delay");
            self.asm.ldx_imm(0x00);
            self.asm
                .label(&delay)
                .expect("unique label should not collide");
            self.asm.inx();
            self.asm.bne(&delay);
        }
    }

    fn write_oam_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.asm.lda_imm(byte);
            self.asm.sta_abs(0x2004);
        }
    }

    fn write_oam_entry_at(&mut self, sprite_index: u8, bytes: [u8; 4]) {
        self.asm.lda_imm(sprite_index.saturating_mul(4));
        self.asm.sta_abs(0x2003);
        self.write_oam_bytes(&bytes);
    }

    fn fill_oam(&mut self, value: u8, label_prefix: &str) {
        let loop_label = self.unique_label(label_prefix);
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2003);
        self.asm.ldx_imm(0x00);
        self.asm
            .label(&loop_label)
            .expect("unique label should not collide");
        self.asm.lda_imm(value);
        self.asm.sta_abs(0x2004);
        self.asm.inx();
        self.asm.txa();
        self.asm.bne(&loop_label);
    }

    fn restore_oam_prefix_from_dma_source(&mut self, byte_count: usize) {
        self.asm.lda_imm(0x00);
        self.asm.sta_abs(0x2003);
        for offset in 0..byte_count {
            self.asm.lda_abs(0x0300 + offset as u16);
            self.asm.sta_abs(0x2004);
        }
    }

    fn expect_serial_bits_from_mask(&mut self, addr: u16, expected_mask_addr: u8, fail_base: u8) {
        for index in 0..8 {
            self.asm.lda_abs(addr);
            self.asm.and_imm(0x01);
            self.expect_a_matches_mask_bit(expected_mask_addr, 1 << index, fail_base + index);
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

    fn lda_abs_x(&mut self, addr: u16) {
        self.op_abs(0xBD, addr);
    }

    fn lda_indirect_y(&mut self, addr: u8) {
        self.op_zp(0xB1, addr);
    }

    fn ldx_imm(&mut self, value: u8) {
        self.op_imm(0xA2, value);
    }

    fn ldy_imm(&mut self, value: u8) {
        self.op_imm(0xA0, value);
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

    fn stx_abs(&mut self, addr: u16) {
        self.op_abs(0x8E, addr);
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

    fn ora_imm(&mut self, value: u8) {
        self.op_imm(0x09, value);
    }

    fn cmp_imm(&mut self, value: u8) {
        self.op_imm(0xC9, value);
    }

    fn cmp_zp(&mut self, addr: u8) {
        self.op_zp(0xC5, addr);
    }

    fn cpx_imm(&mut self, value: u8) {
        self.op_imm(0xE0, value);
    }

    fn beq(&mut self, label: &str) {
        self.op_rel(0xF0, label);
    }

    fn bcc(&mut self, label: &str) {
        self.op_rel(0x90, label);
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

    fn asl_zp(&mut self, addr: u8) {
        self.op_zp(0x06, addr);
    }

    fn asl_abs(&mut self, addr: u16) {
        self.op_abs(0x0E, addr);
    }

    fn rol_zp(&mut self, addr: u8) {
        self.op_zp(0x26, addr);
    }

    fn rol_abs_x(&mut self, addr: u16) {
        self.op_abs(0x3E, addr);
    }

    fn lsr_zp(&mut self, addr: u8) {
        self.op_zp(0x46, addr);
    }

    fn lsr_abs(&mut self, addr: u16) {
        self.op_abs(0x4E, addr);
    }

    fn ror_zp(&mut self, addr: u8) {
        self.op_zp(0x66, addr);
    }

    fn ror_abs_x(&mut self, addr: u16) {
        self.op_abs(0x7E, addr);
    }

    fn inc_zp(&mut self, addr: u8) {
        self.op_zp(0xE6, addr);
    }

    fn inc_abs(&mut self, addr: u16) {
        self.op_abs(0xEE, addr);
    }

    fn dec_zp(&mut self, addr: u8) {
        self.op_zp(0xC6, addr);
    }

    fn dec_abs_x(&mut self, addr: u16) {
        self.op_abs(0xDE, addr);
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

    fn tsx(&mut self) {
        self.emit(0xBA);
    }

    fn pha(&mut self) {
        self.emit(0x48);
    }

    fn php(&mut self) {
        self.emit(0x08);
    }

    fn pla(&mut self) {
        self.emit(0x68);
    }

    fn plp(&mut self) {
        self.emit(0x28);
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

    fn cli(&mut self) {
        self.emit(0x58);
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

    fn clv(&mut self) {
        self.emit(0xB8);
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
    for row in 0..8 {
        chr[32 + row] = 0xFF;
        chr[32 + 8 + row] = 0;
    }
    for row in 0..8 {
        chr[48 + row] = 0;
        chr[48 + 8 + row] = 0xFF;
    }

    chr
}

fn build_mapper3_chr_bank_variant_chr_rom() -> Vec<u8> {
    let mut chr = vec![0; MAPPER3_CHR_BANKS as usize * CHR_BANK_SIZE];
    for (bank, value) in MAPPER3_CHR_BANK_EXPECTED_VALUES.iter().enumerate() {
        let index = bank * CHR_BANK_SIZE + MAPPER3_CHR_READ_ADDR as usize;
        chr[index] = *value;
    }
    chr
}

fn diagnostic_prg_offset_for_cpu_addr(addr: u16) -> usize {
    diagnostic_prg_offset_for_cpu_addr_with_banks(addr, PRG_BANKS)
}

fn diagnostic_prg_offset_for_cpu_addr_with_banks(addr: u16, prg_banks: u8) -> usize {
    match addr {
        0x8000..=0xBFFF => (addr - 0x8000) as usize,
        0xC000..=0xFFFF => {
            let final_bank_offset = (prg_banks as usize - 1) * PRG_BANK_SIZE;
            final_bank_offset + (addr - 0xC000) as usize
        }
        _ => panic!("diagnostic PRG CPU address out of cartridge range: 0x{addr:04X}"),
    }
}

fn write_prg_cpu_byte_for_banks(prg: &mut [u8], prg_banks: u8, addr: u16, value: u8) {
    let index = diagnostic_prg_offset_for_cpu_addr_with_banks(addr, prg_banks);
    prg[index] = value;
}

fn write_mapper4_8k_cpu_byte(prg: &mut [u8], bank: usize, addr: u16, value: u8) {
    let index = bank * 0x2000 + (addr & 0x1FFF) as usize;
    prg[index] = value;
}

fn write_mapper4_8k_cpu_vector(prg: &mut [u8], bank: usize, vector_addr: u16, value: u16) {
    write_mapper4_8k_cpu_byte(prg, bank, vector_addr, value as u8);
    write_mapper4_8k_cpu_byte(prg, bank, vector_addr + 1, (value >> 8) as u8);
}

fn write_vector_for_banks(prg: &mut [u8], prg_banks: u8, vector_addr: u16, value: u16) {
    let index = diagnostic_prg_offset_for_cpu_addr_with_banks(vector_addr, prg_banks);
    prg[index] = value as u8;
    prg[index + 1] = (value >> 8) as u8;
}

fn write_mapper7_32k_vector(prg: &mut [u8], bank: usize, vector_addr: u16, value: u16) {
    let index = bank * 0x8000 + (vector_addr - 0x8000) as usize;
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
        joypad1_expected_mask: config.expected_joypad1_mask,
        joypad1_expected_mask_hex: hex_byte(config.expected_joypad1_mask),
        joypad2_mask: config.joypad2_mask,
        joypad2_mask_hex: hex_byte(config.joypad2_mask),
        joypad2_expected_mask: config.expected_joypad2_mask,
        joypad2_expected_mask_hex: hex_byte(config.expected_joypad2_mask),
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
        DiagnosticFaultInjection::CpuAddressingModeMatrix => {
            bus.cpu_write(0x0500, 0x00);
        }
        DiagnosticFaultInjection::CpuBranchConditionMatrix => {
            bus.cpu_write(CPU_BRANCH_MATRIX_CASE_COUNT_ADDR, 0x80);
        }
        DiagnosticFaultInjection::CpuStackStatusMatrix => {
            bus.cpu_write(CPU_STACK_MATRIX_CASE_COUNT_ADDR, 0x80);
        }
        DiagnosticFaultInjection::CpuIndirectJmpPageWrap => {
            let wrong_target_high = bus.cpu_read(0x0500);
            bus.cpu_write(0x0400, wrong_target_high);
        }
        DiagnosticFaultInjection::CpuRamMirroring => {
            bus.cpu_write(0x0002, 0x00);
        }
        DiagnosticFaultInjection::CpuReadModifyWriteAddressingMatrix => {
            bus.cpu_write(0x0450, 0x01);
        }
        DiagnosticFaultInjection::CpuReadModifyWriteMatrix => {
            bus.cpu_write(0x0030, 0x01);
        }
        DiagnosticFaultInjection::CpuZeroPageIndexWrap => {
            bus.cpu_write(0x0080, 0x00);
        }
        DiagnosticFaultInjection::DmaOamTransfer => {
            bus.cpu_write(0x0300, 0xFF);
        }
        DiagnosticFaultInjection::DmaPhaseMatrix => {
            bus.cpu_write(DMA_PHASE_MATRIX_CONTROL_ADDR, 0x01);
        }
        DiagnosticFaultInjection::InputPortMatrix => {
            bus.joypad2.set_button_pressed(JoypadButton::Start, false);
        }
        DiagnosticFaultInjection::JoypadStrobeHighHold => {
            bus.joypad1.set_button_pressed(JoypadButton::A, false);
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
        DiagnosticFaultInjection::PpuScrollSeam => {
            bus.cpu_write(0x2006, 0x20);
            bus.cpu_write(0x2006, 0x41);
            bus.cpu_write(0x2007, 0x02);
            let _ = bus.cpu_read(0x2002);
        }
        DiagnosticFaultInjection::PpuSpriteOverflow => {
            bus.ppu.oam_data[32..].fill(0xF0);
        }
        DiagnosticFaultInjection::PpuSpritePriority => {
            bus.ppu.oam_data[2] = 0x20;
            bus.ppu.oam_data[6] = 0x00;
        }
        DiagnosticFaultInjection::PpuSpriteZeroHit => {
            bus.cpu_write(0x2003, 0x01);
            bus.cpu_write(0x2004, 0x00);
        }
        DiagnosticFaultInjection::PpuStatusLatchReset => {
            bus.cpu_write(0x2006, 0x20);
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

fn cpu_addressing_matrix_telemetry(ram: &[u8]) -> CpuAddressingMatrixTelemetry {
    let abs_x_no_cross_result = ram[(CPU_ADDRESSING_MATRIX_ABS_X_NO_CROSS_ADDR & 0x07FF) as usize];
    let abs_x_page_cross_result =
        ram[(CPU_ADDRESSING_MATRIX_ABS_X_PAGE_CROSS_ADDR & 0x07FF) as usize];
    let indirect_y_page_cross_result =
        ram[(CPU_ADDRESSING_MATRIX_INDIRECT_Y_PAGE_CROSS_ADDR & 0x07FF) as usize];
    let observed_case_count = ram[(CPU_ADDRESSING_MATRIX_CASE_COUNT_ADDR & 0x07FF) as usize];
    CpuAddressingMatrixTelemetry {
        expected_case_count: CPU_ADDRESSING_MATRIX_EXPECTED_CASE_COUNT,
        observed_case_count,
        passed: observed_case_count == CPU_ADDRESSING_MATRIX_EXPECTED_CASE_COUNT
            && abs_x_no_cross_result == 0x34
            && abs_x_page_cross_result == 0x56
            && indirect_y_page_cross_result == 0x56,
        abs_x_no_cross_result,
        abs_x_no_cross_result_hex: hex_byte(abs_x_no_cross_result),
        abs_x_page_cross_result,
        abs_x_page_cross_result_hex: hex_byte(abs_x_page_cross_result),
        indirect_y_page_cross_result,
        indirect_y_page_cross_result_hex: hex_byte(indirect_y_page_cross_result),
    }
}

fn cpu_branch_matrix_telemetry(ram: &[u8]) -> CpuBranchMatrixTelemetry {
    let taken_mask = ram[(CPU_BRANCH_MATRIX_TAKEN_MASK_ADDR & 0x07FF) as usize];
    let not_taken_mask = ram[(CPU_BRANCH_MATRIX_NOT_TAKEN_MASK_ADDR & 0x07FF) as usize];
    let page_cross_result = ram[(CPU_BRANCH_MATRIX_PAGE_CROSS_RESULT_ADDR & 0x07FF) as usize];
    let observed_case_count = ram[(CPU_BRANCH_MATRIX_CASE_COUNT_ADDR & 0x07FF) as usize];
    CpuBranchMatrixTelemetry {
        expected_case_count: CPU_BRANCH_MATRIX_EXPECTED_CASE_COUNT,
        observed_case_count,
        expected_mask: CPU_BRANCH_MATRIX_EXPECTED_MASK,
        expected_mask_hex: hex_byte(CPU_BRANCH_MATRIX_EXPECTED_MASK),
        taken_mask,
        taken_mask_hex: hex_byte(taken_mask),
        not_taken_mask,
        not_taken_mask_hex: hex_byte(not_taken_mask),
        expected_page_cross_result: CPU_BRANCH_MATRIX_EXPECTED_PAGE_CROSS_RESULT,
        expected_page_cross_result_hex: hex_byte(CPU_BRANCH_MATRIX_EXPECTED_PAGE_CROSS_RESULT),
        page_cross_result,
        page_cross_result_hex: hex_byte(page_cross_result),
        passed: observed_case_count == CPU_BRANCH_MATRIX_EXPECTED_CASE_COUNT
            && taken_mask == CPU_BRANCH_MATRIX_EXPECTED_MASK
            && not_taken_mask == CPU_BRANCH_MATRIX_EXPECTED_MASK
            && page_cross_result == CPU_BRANCH_MATRIX_EXPECTED_PAGE_CROSS_RESULT,
    }
}

fn cpu_stack_matrix_telemetry(ram: &[u8]) -> CpuStackMatrixTelemetry {
    let tsx_result = ram[(CPU_STACK_MATRIX_TSX_RESULT_ADDR & 0x07FF) as usize];
    let pull_result = ram[(CPU_STACK_MATRIX_PULL_RESULT_ADDR & 0x07FF) as usize];
    let status_result = ram[(CPU_STACK_MATRIX_STATUS_RESULT_ADDR & 0x07FF) as usize];
    let jsr_result = ram[(CPU_STACK_MATRIX_JSR_RESULT_ADDR & 0x07FF) as usize];
    let final_stack_pointer = ram[(CPU_STACK_MATRIX_FINAL_SP_ADDR & 0x07FF) as usize];
    let observed_case_count = ram[(CPU_STACK_MATRIX_CASE_COUNT_ADDR & 0x07FF) as usize];
    CpuStackMatrixTelemetry {
        expected_case_count: CPU_STACK_MATRIX_EXPECTED_CASE_COUNT,
        observed_case_count,
        expected_stack_pointer: CPU_STACK_MATRIX_EXPECTED_STACK_POINTER,
        expected_stack_pointer_hex: hex_byte(CPU_STACK_MATRIX_EXPECTED_STACK_POINTER),
        tsx_result,
        tsx_result_hex: hex_byte(tsx_result),
        pull_result,
        pull_result_hex: hex_byte(pull_result),
        status_result,
        status_result_hex: hex_byte(status_result),
        jsr_result,
        jsr_result_hex: hex_byte(jsr_result),
        final_stack_pointer,
        final_stack_pointer_hex: hex_byte(final_stack_pointer),
        passed: observed_case_count == CPU_STACK_MATRIX_EXPECTED_CASE_COUNT
            && tsx_result == CPU_STACK_MATRIX_EXPECTED_STACK_POINTER
            && pull_result == CPU_STACK_MATRIX_EXPECTED_PULL_RESULT
            && status_result == CPU_STACK_MATRIX_EXPECTED_STATUS_RESULT
            && jsr_result == CPU_STACK_MATRIX_EXPECTED_JSR_RESULT
            && final_stack_pointer == CPU_STACK_MATRIX_EXPECTED_STACK_POINTER,
    }
}

fn cpu_rmw_matrix_telemetry(ram: &[u8]) -> CpuRmwMatrixTelemetry {
    let asl_result = ram[(CPU_RMW_MATRIX_ASL_RESULT_ADDR & 0x07FF) as usize];
    let rol_result = ram[(CPU_RMW_MATRIX_ROL_RESULT_ADDR & 0x07FF) as usize];
    let lsr_result = ram[(CPU_RMW_MATRIX_LSR_RESULT_ADDR & 0x07FF) as usize];
    let ror_result = ram[(CPU_RMW_MATRIX_ROR_RESULT_ADDR & 0x07FF) as usize];
    let inc_result = ram[(CPU_RMW_MATRIX_INC_RESULT_ADDR & 0x07FF) as usize];
    let dec_result = ram[(CPU_RMW_MATRIX_DEC_RESULT_ADDR & 0x07FF) as usize];
    let observed_case_count = ram[(CPU_RMW_MATRIX_CASE_COUNT_ADDR & 0x07FF) as usize];
    CpuRmwMatrixTelemetry {
        expected_case_count: CPU_RMW_MATRIX_EXPECTED_CASE_COUNT,
        observed_case_count,
        passed: observed_case_count == CPU_RMW_MATRIX_EXPECTED_CASE_COUNT
            && asl_result == 0x80
            && rol_result == 0x01
            && lsr_result == 0x40
            && ror_result == 0x80
            && inc_result == 0x00
            && dec_result == 0xFF,
        asl_result,
        asl_result_hex: hex_byte(asl_result),
        rol_result,
        rol_result_hex: hex_byte(rol_result),
        lsr_result,
        lsr_result_hex: hex_byte(lsr_result),
        ror_result,
        ror_result_hex: hex_byte(ror_result),
        inc_result,
        inc_result_hex: hex_byte(inc_result),
        dec_result,
        dec_result_hex: hex_byte(dec_result),
    }
}

fn cpu_rmw_addressing_matrix_telemetry(ram: &[u8]) -> CpuRmwAddressingMatrixTelemetry {
    let asl_abs_result = ram[(CPU_RMW_ADDRESSING_ASL_ABS_RESULT_ADDR & 0x07FF) as usize];
    let rol_abs_x_result = ram[(CPU_RMW_ADDRESSING_ROL_ABS_X_RESULT_ADDR & 0x07FF) as usize];
    let lsr_abs_result = ram[(CPU_RMW_ADDRESSING_LSR_ABS_RESULT_ADDR & 0x07FF) as usize];
    let ror_abs_x_result = ram[(CPU_RMW_ADDRESSING_ROR_ABS_X_RESULT_ADDR & 0x07FF) as usize];
    let inc_abs_result = ram[(CPU_RMW_ADDRESSING_INC_ABS_RESULT_ADDR & 0x07FF) as usize];
    let dec_abs_x_result = ram[(CPU_RMW_ADDRESSING_DEC_ABS_X_RESULT_ADDR & 0x07FF) as usize];
    let observed_case_count = ram[(CPU_RMW_ADDRESSING_CASE_COUNT_ADDR & 0x07FF) as usize];
    CpuRmwAddressingMatrixTelemetry {
        expected_case_count: CPU_RMW_ADDRESSING_EXPECTED_CASE_COUNT,
        observed_case_count,
        passed: observed_case_count == CPU_RMW_ADDRESSING_EXPECTED_CASE_COUNT
            && asl_abs_result == 0x80
            && rol_abs_x_result == 0x01
            && lsr_abs_result == 0x40
            && ror_abs_x_result == 0x80
            && inc_abs_result == 0x00
            && dec_abs_x_result == 0xFF,
        asl_abs_result,
        asl_abs_result_hex: hex_byte(asl_abs_result),
        rol_abs_x_result,
        rol_abs_x_result_hex: hex_byte(rol_abs_x_result),
        lsr_abs_result,
        lsr_abs_result_hex: hex_byte(lsr_abs_result),
        ror_abs_x_result,
        ror_abs_x_result_hex: hex_byte(ror_abs_x_result),
        inc_abs_result,
        inc_abs_result_hex: hex_byte(inc_abs_result),
        dec_abs_x_result,
        dec_abs_x_result_hex: hex_byte(dec_abs_x_result),
    }
}

fn input_port_matrix_telemetry(ram: &[u8], config: &DiagnosticConfig) -> InputPortMatrixTelemetry {
    let joypad1_high_first = ram[(INPUT_PORT_MATRIX_JOYPAD1_HIGH_FIRST_ADDR & 0x07FF) as usize];
    let joypad1_high_second = ram[(INPUT_PORT_MATRIX_JOYPAD1_HIGH_SECOND_ADDR & 0x07FF) as usize];
    let joypad2_high_first = ram[(INPUT_PORT_MATRIX_JOYPAD2_HIGH_FIRST_ADDR & 0x07FF) as usize];
    let joypad2_high_second = ram[(INPUT_PORT_MATRIX_JOYPAD2_HIGH_SECOND_ADDR & 0x07FF) as usize];
    let joypad1_overread_first =
        ram[(INPUT_PORT_MATRIX_JOYPAD1_OVERREAD_FIRST_ADDR & 0x07FF) as usize];
    let joypad1_overread_second =
        ram[(INPUT_PORT_MATRIX_JOYPAD1_OVERREAD_SECOND_ADDR & 0x07FF) as usize];
    let joypad2_overread_first =
        ram[(INPUT_PORT_MATRIX_JOYPAD2_OVERREAD_FIRST_ADDR & 0x07FF) as usize];
    let joypad2_overread_second =
        ram[(INPUT_PORT_MATRIX_JOYPAD2_OVERREAD_SECOND_ADDR & 0x07FF) as usize];
    let observed_case_count = ram[(INPUT_PORT_MATRIX_CASE_COUNT_ADDR & 0x07FF) as usize];
    let joypad1_a_bit = config.expected_joypad1_mask & 0x01;
    let joypad2_a_bit = config.expected_joypad2_mask & 0x01;
    InputPortMatrixTelemetry {
        expected_case_count: INPUT_PORT_MATRIX_EXPECTED_CASE_COUNT,
        observed_case_count,
        passed: observed_case_count == INPUT_PORT_MATRIX_EXPECTED_CASE_COUNT
            && joypad1_high_first == joypad1_a_bit
            && joypad1_high_second == joypad1_a_bit
            && joypad2_high_first == joypad2_a_bit
            && joypad2_high_second == joypad2_a_bit
            && joypad1_overread_first == 0x01
            && joypad1_overread_second == 0x01
            && joypad2_overread_first == 0x01
            && joypad2_overread_second == 0x01,
        joypad1_high_first,
        joypad1_high_first_hex: hex_byte(joypad1_high_first),
        joypad1_high_second,
        joypad1_high_second_hex: hex_byte(joypad1_high_second),
        joypad2_high_first,
        joypad2_high_first_hex: hex_byte(joypad2_high_first),
        joypad2_high_second,
        joypad2_high_second_hex: hex_byte(joypad2_high_second),
        joypad1_overread_first,
        joypad1_overread_first_hex: hex_byte(joypad1_overread_first),
        joypad1_overread_second,
        joypad1_overread_second_hex: hex_byte(joypad1_overread_second),
        joypad2_overread_first,
        joypad2_overread_first_hex: hex_byte(joypad2_overread_first),
        joypad2_overread_second,
        joypad2_overread_second_hex: hex_byte(joypad2_overread_second),
    }
}

fn apu_status_matrix_telemetry(ram: &[u8]) -> ApuStatusMatrixTelemetry {
    let observed_mask = ram[(APU_STATUS_MATRIX_OBSERVED_MASK_ADDR & 0x07FF) as usize]
        & APU_STATUS_MATRIX_EXPECTED_MASK;
    let observed_case_count = ram[(APU_STATUS_MATRIX_CASE_COUNT_ADDR & 0x07FF) as usize];
    ApuStatusMatrixTelemetry {
        expected_mask: APU_STATUS_MATRIX_EXPECTED_MASK,
        expected_mask_hex: hex_byte(APU_STATUS_MATRIX_EXPECTED_MASK),
        observed_mask,
        observed_mask_hex: hex_byte(observed_mask),
        expected_case_count: APU_STATUS_MATRIX_EXPECTED_CASE_COUNT,
        observed_case_count,
        pulse1_status_bit: observed_mask & 0x01 != 0,
        pulse2_status_bit: observed_mask & 0x02 != 0,
        triangle_status_bit: observed_mask & 0x04 != 0,
        noise_status_bit: observed_mask & 0x08 != 0,
        passed: observed_mask == APU_STATUS_MATRIX_EXPECTED_MASK
            && observed_case_count == APU_STATUS_MATRIX_EXPECTED_CASE_COUNT,
    }
}

fn apu_dmc_status_telemetry(ram: &[u8]) -> ApuDmcStatusTelemetry {
    let observed_bit =
        ram[(APU_DMC_STATUS_OBSERVED_BIT_ADDR & 0x07FF) as usize] & APU_DMC_STATUS_EXPECTED_BIT;
    let observed_case_count = ram[(APU_DMC_STATUS_CASE_COUNT_ADDR & 0x07FF) as usize];
    ApuDmcStatusTelemetry {
        expected_bit: APU_DMC_STATUS_EXPECTED_BIT,
        expected_bit_hex: hex_byte(APU_DMC_STATUS_EXPECTED_BIT),
        observed_bit,
        observed_bit_hex: hex_byte(observed_bit),
        expected_case_count: APU_DMC_STATUS_EXPECTED_CASE_COUNT,
        observed_case_count,
        dmc_status_bit: observed_bit & APU_DMC_STATUS_EXPECTED_BIT != 0,
        passed: observed_bit == APU_DMC_STATUS_EXPECTED_BIT
            && observed_case_count == APU_DMC_STATUS_EXPECTED_CASE_COUNT,
    }
}

fn mapper1_mmc1_telemetry(observation: &Mapper1Mmc1Observation) -> Mapper1Mmc1Telemetry {
    let prg_bank_writes = MAPPER1_PRG_BANK_WRITES.to_vec();
    let chr_bank_writes = MAPPER1_CHR_BANK_WRITES.to_vec();
    let expected_prg_values = MAPPER1_PRG_EXPECTED_VALUES.to_vec();
    let observed_prg_values = observation.observed_prg_values.to_vec();
    let expected_chr_values = MAPPER1_CHR_EXPECTED_VALUES.to_vec();
    let observed_chr_values = observation.observed_chr_values.to_vec();
    let expected_mirror_values = MAPPER1_MIRROR_EXPECTED_VALUES.to_vec();
    let observed_mirror_values = observation.observed_mirror_values.to_vec();

    Mapper1Mmc1Telemetry {
        mapper: MAPPER1_MAPPER,
        prg_banks: MAPPER1_PRG_BANKS,
        chr_8k_banks: MAPPER1_CHR_8K_BANKS,
        chr_4k_banks: MAPPER1_CHR_4K_BANKS,
        prg_switch_addr: MAPPER1_PRG_SWITCH_ADDR,
        prg_switch_addr_hex: format!("0x{:04X}", MAPPER1_PRG_SWITCH_ADDR),
        prg_fixed_addr: MAPPER1_PRG_FIXED_ADDR,
        prg_fixed_addr_hex: format!("0x{:04X}", MAPPER1_PRG_FIXED_ADDR),
        chr_low_read_addr: MAPPER1_CHR_LOW_READ_ADDR,
        chr_low_read_addr_hex: format!("0x{:04X}", MAPPER1_CHR_LOW_READ_ADDR),
        chr_high_read_addr: MAPPER1_CHR_HIGH_READ_ADDR,
        chr_high_read_addr_hex: format!("0x{:04X}", MAPPER1_CHR_HIGH_READ_ADDR),
        expected_case_count: MAPPER1_EXPECTED_CASE_COUNT,
        observed_case_count: observation.observed_case_count,
        prg_bank_writes: prg_bank_writes.clone(),
        prg_bank_writes_hex: prg_bank_writes
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        chr_bank_writes: chr_bank_writes.clone(),
        chr_bank_writes_hex: chr_bank_writes
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        expected_prg_values: expected_prg_values.clone(),
        expected_prg_values_hex: expected_prg_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_prg_values: observed_prg_values.clone(),
        observed_prg_values_hex: observed_prg_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        expected_chr_values: expected_chr_values.clone(),
        expected_chr_values_hex: expected_chr_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_chr_values: observed_chr_values.clone(),
        observed_chr_values_hex: observed_chr_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        expected_mirror_values: expected_mirror_values.clone(),
        expected_mirror_values_hex: expected_mirror_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_mirror_values: observed_mirror_values.clone(),
        observed_mirror_values_hex: observed_mirror_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        cycles: observation.cycles,
        frames: observation.frames,
        passed: observation.passed,
        error: observation.error.clone(),
    }
}

fn mapper1_mmc1_32k_prg_telemetry(
    observation: &Mapper1Mmc1Prg32kObservation,
) -> Mapper1Mmc1Prg32kTelemetry {
    let control_writes = MAPPER1_32K_CONTROL_WRITES.to_vec();
    let prg_bank_writes = MAPPER1_32K_PRG_BANK_WRITES.to_vec();
    let expected_values = MAPPER1_32K_EXPECTED_VALUES.to_vec();
    let observed_values = observation.observed_values.to_vec();

    Mapper1Mmc1Prg32kTelemetry {
        mapper: MAPPER1_MAPPER,
        prg_banks: MAPPER1_PRG_BANKS,
        chr_8k_banks: MAPPER1_CHR_8K_BANKS,
        low_read_addr: MAPPER1_32K_LOW_READ_ADDR,
        low_read_addr_hex: format!("0x{:04X}", MAPPER1_32K_LOW_READ_ADDR),
        high_read_addr: MAPPER1_32K_HIGH_READ_ADDR,
        high_read_addr_hex: format!("0x{:04X}", MAPPER1_32K_HIGH_READ_ADDR),
        expected_case_count: MAPPER1_32K_EXPECTED_CASE_COUNT,
        observed_case_count: observation.observed_case_count,
        control_writes: control_writes.clone(),
        control_writes_hex: control_writes
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        prg_bank_writes: prg_bank_writes.clone(),
        prg_bank_writes_hex: prg_bank_writes
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        expected_values: expected_values.clone(),
        expected_values_hex: expected_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_values: observed_values.clone(),
        observed_values_hex: observed_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        cycles: observation.cycles,
        frames: observation.frames,
        passed: observation.passed,
        error: observation.error.clone(),
    }
}

fn mapper3_chr_bank_telemetry(observation: &Mapper3ChrBankObservation) -> Mapper3ChrBankTelemetry {
    let expected_values = MAPPER3_CHR_BANK_EXPECTED_VALUES.to_vec();
    let observed_values = observation.observed_values.to_vec();
    Mapper3ChrBankTelemetry {
        mapper: MAPPER3_MAPPER,
        prg_banks: MAPPER3_PRG_BANKS,
        chr_banks: MAPPER3_CHR_BANKS,
        read_addr: MAPPER3_CHR_READ_ADDR,
        read_addr_hex: format!("0x{:04X}", MAPPER3_CHR_READ_ADDR),
        expected_case_count: MAPPER3_CHR_BANK_EXPECTED_CASE_COUNT,
        observed_case_count: observation.observed_case_count,
        expected_banks: MAPPER3_CHR_BANK_EXPECTED_BANKS.to_vec(),
        expected_values: expected_values.clone(),
        expected_values_hex: expected_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_values: observed_values.clone(),
        observed_values_hex: observed_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        cycles: observation.cycles,
        frames: observation.frames,
        passed: observation.passed,
        error: observation.error.clone(),
    }
}

fn mapper4_mmc3_telemetry(observation: &Mapper4Mmc3Observation) -> Mapper4Mmc3Telemetry {
    let prg_read_addrs = vec![
        MAPPER4_PRG_R6_READ_ADDR,
        MAPPER4_PRG_R7_READ_ADDR,
        MAPPER4_PRG_FIXED_READ_ADDR,
    ];
    let chr_read_addrs = MAPPER4_CHR_READ_ADDRS.to_vec();
    let prg_register_writes = MAPPER4_PRG_REGISTER_WRITES.to_vec();
    let chr_register_writes = MAPPER4_CHR_REGISTER_WRITES.to_vec();
    let expected_prg_values = MAPPER4_PRG_EXPECTED_VALUES.to_vec();
    let observed_prg_values = observation.observed_prg_values.to_vec();
    let expected_chr_values = MAPPER4_CHR_EXPECTED_VALUES.to_vec();
    let observed_chr_values = observation.observed_chr_values.to_vec();
    let expected_mirror_values = MAPPER4_MIRROR_EXPECTED_VALUES.to_vec();
    let observed_mirror_values = observation.observed_mirror_values.to_vec();

    Mapper4Mmc3Telemetry {
        mapper: MAPPER4_MAPPER,
        prg_16k_banks: MAPPER4_PRG_16K_BANKS,
        prg_8k_banks: MAPPER4_PRG_8K_BANKS,
        chr_8k_banks: MAPPER4_CHR_8K_BANKS,
        chr_1k_banks: MAPPER4_CHR_1K_BANKS,
        prg_read_addrs: prg_read_addrs.clone(),
        prg_read_addrs_hex: prg_read_addrs
            .iter()
            .map(|value| format!("0x{value:04X}"))
            .collect(),
        chr_read_addrs: chr_read_addrs.clone(),
        chr_read_addrs_hex: chr_read_addrs
            .iter()
            .map(|value| format!("0x{value:04X}"))
            .collect(),
        expected_case_count: MAPPER4_EXPECTED_CASE_COUNT,
        observed_case_count: observation.observed_case_count,
        prg_register_writes: prg_register_writes.clone(),
        prg_register_writes_hex: prg_register_writes
            .iter()
            .map(|(register, value)| format!("R{register}:{}", hex_byte(*value)))
            .collect(),
        chr_register_writes: chr_register_writes.clone(),
        chr_register_writes_hex: chr_register_writes
            .iter()
            .map(|(register, value)| format!("R{register}:{}", hex_byte(*value)))
            .collect(),
        irq_latch: MAPPER4_IRQ_LATCH,
        irq_latch_hex: hex_byte(MAPPER4_IRQ_LATCH),
        expected_irq_count: MAPPER4_EXPECTED_IRQ_COUNT,
        observed_irq_count: observation.observed_irq_count,
        expected_prg_values: expected_prg_values.clone(),
        expected_prg_values_hex: expected_prg_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_prg_values: observed_prg_values.clone(),
        observed_prg_values_hex: observed_prg_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        expected_chr_values: expected_chr_values.clone(),
        expected_chr_values_hex: expected_chr_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_chr_values: observed_chr_values.clone(),
        observed_chr_values_hex: observed_chr_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        expected_mirror_values: expected_mirror_values.clone(),
        expected_mirror_values_hex: expected_mirror_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_mirror_values: observed_mirror_values.clone(),
        observed_mirror_values_hex: observed_mirror_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        cycles: observation.cycles,
        frames: observation.frames,
        passed: observation.passed,
        error: observation.error.clone(),
    }
}

fn mapper4_mmc3_edge_telemetry(
    observation: &Mapper4Mmc3EdgeObservation,
) -> Mapper4Mmc3EdgeTelemetry {
    let prg_read_addrs = MAPPER4_EDGE_PRG_READ_ADDRS.to_vec();
    let chr_read_addrs = MAPPER4_EDGE_CHR_READ_ADDRS.to_vec();
    let prg_select_writes = MAPPER4_EDGE_PRG_SELECT_WRITES.to_vec();
    let chr_select_writes = MAPPER4_EDGE_CHR_SELECT_WRITES.to_vec();
    let irq_latches = MAPPER4_EDGE_IRQ_LATCHES.to_vec();
    let expected_irq_counts = MAPPER4_EDGE_EXPECTED_IRQ_COUNTS.to_vec();
    let observed_irq_counts = observation.observed_irq_counts.to_vec();
    let expected_prg_values = MAPPER4_EDGE_PRG_EXPECTED_VALUES.to_vec();
    let observed_prg_values = observation.observed_prg_values.to_vec();
    let expected_chr_values = MAPPER4_EDGE_CHR_EXPECTED_VALUES.to_vec();
    let observed_chr_values = observation.observed_chr_values.to_vec();

    Mapper4Mmc3EdgeTelemetry {
        mapper: MAPPER4_MAPPER,
        prg_16k_banks: MAPPER4_PRG_16K_BANKS,
        prg_8k_banks: MAPPER4_PRG_8K_BANKS,
        chr_8k_banks: MAPPER4_CHR_8K_BANKS,
        chr_1k_banks: MAPPER4_CHR_1K_BANKS,
        program_base: MAPPER4_EDGE_PROGRAM_BASE,
        program_base_hex: format!("0x{MAPPER4_EDGE_PROGRAM_BASE:04X}"),
        prg_read_addrs: prg_read_addrs.clone(),
        prg_read_addrs_hex: prg_read_addrs
            .iter()
            .map(|value| format!("0x{value:04X}"))
            .collect(),
        chr_read_addrs: chr_read_addrs.clone(),
        chr_read_addrs_hex: chr_read_addrs
            .iter()
            .map(|value| format!("0x{value:04X}"))
            .collect(),
        expected_case_count: MAPPER4_EDGE_EXPECTED_CASE_COUNT,
        observed_case_count: observation.observed_case_count,
        prg_select_writes: prg_select_writes.clone(),
        prg_select_writes_hex: prg_select_writes
            .iter()
            .map(|(select, value)| format!("select {}:{}", hex_byte(*select), hex_byte(*value)))
            .collect(),
        chr_select_writes: chr_select_writes.clone(),
        chr_select_writes_hex: chr_select_writes
            .iter()
            .map(|(select, value)| format!("select {}:{}", hex_byte(*select), hex_byte(*value)))
            .collect(),
        irq_latches: irq_latches.clone(),
        irq_latches_hex: irq_latches.iter().map(|value| hex_byte(*value)).collect(),
        expected_irq_counts: expected_irq_counts.clone(),
        observed_irq_counts: observed_irq_counts.clone(),
        expected_prg_values: expected_prg_values.clone(),
        expected_prg_values_hex: expected_prg_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_prg_values: observed_prg_values.clone(),
        observed_prg_values_hex: observed_prg_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        expected_chr_values: expected_chr_values.clone(),
        expected_chr_values_hex: expected_chr_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_chr_values: observed_chr_values.clone(),
        observed_chr_values_hex: observed_chr_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        cycles: observation.cycles,
        frames: observation.frames,
        passed: observation.passed,
        error: observation.error.clone(),
    }
}

fn mapper4_mmc3_prg_ram_telemetry(
    observation: &Mapper4Mmc3PrgRamObservation,
) -> Mapper4Mmc3PrgRamTelemetry {
    let read_addrs = MAPPER4_PRG_RAM_READ_ADDRS.to_vec();
    let restored_addrs = MAPPER4_PRG_RAM_RESTORED_ADDRS.to_vec();
    let expected_values = MAPPER4_PRG_RAM_EXPECTED_VALUES.to_vec();
    let observed_values = observation.observed_values.to_vec();
    let sram_snapshot_values = observation.sram_snapshot_values.to_vec();
    let restored_values = observation.restored_values.to_vec();

    Mapper4Mmc3PrgRamTelemetry {
        mapper: MAPPER4_MAPPER,
        prg_16k_banks: MAPPER4_PRG_16K_BANKS,
        prg_8k_banks: MAPPER4_PRG_8K_BANKS,
        chr_8k_banks: MAPPER4_CHR_8K_BANKS,
        battery_backed: observation.battery_backed,
        prg_ram_size: MAPPER4_PRG_RAM_SIZE,
        read_addrs: read_addrs.clone(),
        read_addrs_hex: read_addrs
            .iter()
            .map(|value| format!("0x{value:04X}"))
            .collect(),
        restored_addrs: restored_addrs.clone(),
        restored_addrs_hex: restored_addrs
            .iter()
            .map(|value| format!("0x{value:04X}"))
            .collect(),
        expected_case_count: MAPPER4_PRG_RAM_EXPECTED_CASE_COUNT,
        observed_case_count: observation.observed_case_count,
        expected_values: expected_values.clone(),
        expected_values_hex: expected_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_values: observed_values.clone(),
        observed_values_hex: observed_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        sram_snapshot_values: sram_snapshot_values.clone(),
        sram_snapshot_values_hex: sram_snapshot_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        restored_values: restored_values.clone(),
        restored_values_hex: restored_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        cycles: observation.cycles,
        frames: observation.frames,
        passed: observation.passed,
        error: observation.error.clone(),
    }
}

fn mapper7_axrom_telemetry(observation: &Mapper7AxromObservation) -> Mapper7AxromTelemetry {
    let bank_writes = MAPPER7_PRG_BANK_WRITES.to_vec();
    let expected_prg_values = MAPPER7_PRG_EXPECTED_VALUES.to_vec();
    let observed_prg_values = observation.observed_prg_values.to_vec();
    let expected_mirror_values = MAPPER7_MIRROR_EXPECTED_VALUES.to_vec();
    let observed_mirror_values = observation.observed_mirror_values.to_vec();
    Mapper7AxromTelemetry {
        mapper: MAPPER7_MAPPER,
        prg_banks: MAPPER7_PRG_BANKS,
        chr_banks: MAPPER7_CHR_BANKS,
        prg_read_addr: MAPPER7_PRG_SENTINEL_ADDR,
        prg_read_addr_hex: format!("0x{:04X}", MAPPER7_PRG_SENTINEL_ADDR),
        expected_case_count: MAPPER7_EXPECTED_CASE_COUNT,
        observed_case_count: observation.observed_case_count,
        bank_writes: bank_writes.clone(),
        bank_writes_hex: bank_writes.iter().map(|value| hex_byte(*value)).collect(),
        expected_prg_values: expected_prg_values.clone(),
        expected_prg_values_hex: expected_prg_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_prg_values: observed_prg_values.clone(),
        observed_prg_values_hex: observed_prg_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        expected_mirror_values: expected_mirror_values.clone(),
        expected_mirror_values_hex: expected_mirror_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        observed_mirror_values: observed_mirror_values.clone(),
        observed_mirror_values_hex: observed_mirror_values
            .iter()
            .map(|value| hex_byte(*value))
            .collect(),
        cycles: observation.cycles,
        frames: observation.frames,
        passed: observation.passed,
        error: observation.error.clone(),
    }
}

fn input_mask_sweep_telemetry(observation: &InputMaskSweepObservation) -> InputMaskSweepTelemetry {
    let cases: Vec<InputMaskSweepCaseTelemetry> = observation
        .cases
        .iter()
        .map(|case| InputMaskSweepCaseTelemetry {
            index: case.index,
            joypad1_expected_mask: case.joypad1_expected_mask,
            joypad1_expected_mask_hex: hex_byte(case.joypad1_expected_mask),
            joypad1_observed_mask: case.joypad1_observed_mask,
            joypad1_observed_mask_hex: hex_byte(case.joypad1_observed_mask),
            joypad2_expected_mask: case.joypad2_expected_mask,
            joypad2_expected_mask_hex: hex_byte(case.joypad2_expected_mask),
            joypad2_observed_mask: case.joypad2_observed_mask,
            joypad2_observed_mask_hex: hex_byte(case.joypad2_observed_mask),
            observed_case_count: case.observed_case_count,
            cycles: case.cycles,
            frames: case.frames,
            passed: case.passed,
            error: case.error.clone(),
        })
        .collect();
    let observed_case_count = cases
        .iter()
        .filter(|case| case.observed_case_count == 1)
        .count() as u8;
    let passed_case_count = cases.iter().filter(|case| case.passed).count();
    let failed_case_count = cases.len().saturating_sub(passed_case_count);
    let passed = observation.error.is_none()
        && observed_case_count == INPUT_MASK_SWEEP_EXPECTED_CASE_COUNT
        && passed_case_count == INPUT_MASK_SWEEP_CASES.len();
    let error = if passed {
        None
    } else {
        observation.error.clone().or_else(|| {
            cases
                .iter()
                .find_map(|case| case.error.clone())
                .or_else(|| {
                    Some("input mask sweep retained mismatched host observations".to_string())
                })
        })
    };

    InputMaskSweepTelemetry {
        expected_case_count: INPUT_MASK_SWEEP_EXPECTED_CASE_COUNT,
        observed_case_count,
        passed_case_count,
        failed_case_count,
        cases,
        passed,
        error,
    }
}

fn ppu_sprite_zero_hit_telemetry(ram: &[u8]) -> PpuSpriteZeroHitTelemetry {
    let observed_status_bit = ram[(PPU_SPRITE_ZERO_HIT_STATUS_ADDR & 0x07FF) as usize];
    let observed_case_count = ram[(PPU_SPRITE_ZERO_HIT_CASE_COUNT_ADDR & 0x07FF) as usize];
    PpuSpriteZeroHitTelemetry {
        expected_status_bit: PPU_SPRITE_ZERO_HIT_EXPECTED_STATUS_BIT,
        expected_status_bit_hex: hex_byte(PPU_SPRITE_ZERO_HIT_EXPECTED_STATUS_BIT),
        observed_status_bit,
        observed_status_bit_hex: hex_byte(observed_status_bit),
        expected_case_count: PPU_SPRITE_ZERO_HIT_EXPECTED_CASE_COUNT,
        observed_case_count,
        passed: observed_status_bit == PPU_SPRITE_ZERO_HIT_EXPECTED_STATUS_BIT
            && observed_case_count == PPU_SPRITE_ZERO_HIT_EXPECTED_CASE_COUNT,
    }
}

fn ppu_sprite_overflow_telemetry(ram: &[u8]) -> PpuSpriteOverflowTelemetry {
    let observed_status_bit = ram[(PPU_SPRITE_OVERFLOW_STATUS_ADDR & 0x07FF) as usize];
    let observed_case_count = ram[(PPU_SPRITE_OVERFLOW_CASE_COUNT_ADDR & 0x07FF) as usize];
    let false_positive_observed_status_bit =
        ram[(PPU_SPRITE_OVERFLOW_FALSE_POSITIVE_STATUS_ADDR & 0x07FF) as usize];
    let false_negative_observed_status_bit =
        ram[(PPU_SPRITE_OVERFLOW_FALSE_NEGATIVE_STATUS_ADDR & 0x07FF) as usize];
    let hardware_bug_matrix_passed = observed_status_bit == PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT
        && false_positive_observed_status_bit == PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT
        && false_negative_observed_status_bit == PPU_SPRITE_OVERFLOW_EXPECTED_CLEAR_STATUS_BIT;
    PpuSpriteOverflowTelemetry {
        expected_status_bit: PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT,
        expected_status_bit_hex: hex_byte(PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT),
        observed_status_bit,
        observed_status_bit_hex: hex_byte(observed_status_bit),
        false_positive_expected_status_bit: PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT,
        false_positive_expected_status_bit_hex: hex_byte(PPU_SPRITE_OVERFLOW_EXPECTED_STATUS_BIT),
        false_positive_observed_status_bit,
        false_positive_observed_status_bit_hex: hex_byte(false_positive_observed_status_bit),
        false_negative_expected_status_bit: PPU_SPRITE_OVERFLOW_EXPECTED_CLEAR_STATUS_BIT,
        false_negative_expected_status_bit_hex: hex_byte(
            PPU_SPRITE_OVERFLOW_EXPECTED_CLEAR_STATUS_BIT,
        ),
        false_negative_observed_status_bit,
        false_negative_observed_status_bit_hex: hex_byte(false_negative_observed_status_bit),
        expected_case_count: PPU_SPRITE_OVERFLOW_EXPECTED_CASE_COUNT,
        observed_case_count,
        hardware_bug_matrix_passed,
        restored_oam_byte_count: PPU_SPRITE_OVERFLOW_RESTORE_BYTES as u16,
        passed: hardware_bug_matrix_passed
            && observed_case_count == PPU_SPRITE_OVERFLOW_EXPECTED_CASE_COUNT,
    }
}

fn ppu_sprite_priority_telemetry(
    ram: &[u8],
    captured_sample: Option<&PpuSpritePriorityFrameSample>,
    final_frame: &[u32],
) -> PpuSpritePriorityTelemetry {
    let observed_case_count = ram[(PPU_SPRITE_PRIORITY_CASE_COUNT_ADDR & 0x07FF) as usize];
    let fallback_sample = PpuSpritePriorityFrameSample {
        front_color: sample_frame_color(
            final_frame,
            PPU_SPRITE_PRIORITY_FRONT_SAMPLE_X,
            PPU_SPRITE_PRIORITY_FRONT_SAMPLE_Y,
        ),
        behind_color: sample_frame_color(
            final_frame,
            PPU_SPRITE_PRIORITY_BEHIND_SAMPLE_X,
            PPU_SPRITE_PRIORITY_BEHIND_SAMPLE_Y,
        ),
    };
    let sample = captured_sample.copied().unwrap_or(fallback_sample);
    PpuSpritePriorityTelemetry {
        expected_case_count: PPU_SPRITE_PRIORITY_EXPECTED_CASE_COUNT,
        observed_case_count,
        front_sample_x: PPU_SPRITE_PRIORITY_FRONT_SAMPLE_X,
        front_sample_y: PPU_SPRITE_PRIORITY_FRONT_SAMPLE_Y,
        front_expected_color: PPU_SPRITE_PRIORITY_EXPECTED_FRONT_COLOR,
        front_expected_color_hex: hex_color(PPU_SPRITE_PRIORITY_EXPECTED_FRONT_COLOR),
        front_observed_color: sample.front_color,
        front_observed_color_hex: hex_color(sample.front_color),
        behind_sample_x: PPU_SPRITE_PRIORITY_BEHIND_SAMPLE_X,
        behind_sample_y: PPU_SPRITE_PRIORITY_BEHIND_SAMPLE_Y,
        behind_expected_color: PPU_SPRITE_PRIORITY_EXPECTED_BEHIND_COLOR,
        behind_expected_color_hex: hex_color(PPU_SPRITE_PRIORITY_EXPECTED_BEHIND_COLOR),
        behind_observed_color: sample.behind_color,
        behind_observed_color_hex: hex_color(sample.behind_color),
        passed: observed_case_count == PPU_SPRITE_PRIORITY_EXPECTED_CASE_COUNT
            && sample.front_color == PPU_SPRITE_PRIORITY_EXPECTED_FRONT_COLOR
            && sample.behind_color == PPU_SPRITE_PRIORITY_EXPECTED_BEHIND_COLOR,
    }
}

#[derive(Debug, Clone)]
struct Mapper1Mmc1Observation {
    observed_prg_values: [u8; 5],
    observed_chr_values: [u8; 4],
    observed_mirror_values: [u8; 3],
    observed_case_count: u8,
    cycles: u64,
    frames: u64,
    passed: bool,
    error: Option<String>,
}

impl Mapper1Mmc1Observation {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            observed_prg_values: [0; 5],
            observed_chr_values: [0; 4],
            observed_mirror_values: [0; 3],
            observed_case_count: 0,
            cycles: 0,
            frames: 0,
            passed: false,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct Mapper1Mmc1Prg32kObservation {
    observed_values: [u8; 10],
    observed_case_count: u8,
    cycles: u64,
    frames: u64,
    passed: bool,
    error: Option<String>,
}

impl Mapper1Mmc1Prg32kObservation {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            observed_values: [0; 10],
            observed_case_count: 0,
            cycles: 0,
            frames: 0,
            passed: false,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct Mapper3ChrBankObservation {
    observed_values: [u8; 4],
    observed_case_count: u8,
    cycles: u64,
    frames: u64,
    passed: bool,
    error: Option<String>,
}

impl Mapper3ChrBankObservation {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            observed_values: [0; 4],
            observed_case_count: 0,
            cycles: 0,
            frames: 0,
            passed: false,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct Mapper4Mmc3Observation {
    observed_prg_values: [u8; 3],
    observed_chr_values: [u8; 5],
    observed_mirror_values: [u8; 2],
    observed_irq_count: u8,
    observed_case_count: u8,
    cycles: u64,
    frames: u64,
    passed: bool,
    error: Option<String>,
}

impl Mapper4Mmc3Observation {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            observed_prg_values: [0; 3],
            observed_chr_values: [0; 5],
            observed_mirror_values: [0; 2],
            observed_irq_count: 0,
            observed_case_count: 0,
            cycles: 0,
            frames: 0,
            passed: false,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct Mapper4Mmc3EdgeObservation {
    observed_prg_values: [u8; 3],
    observed_chr_values: [u8; 8],
    observed_irq_counts: [u8; 2],
    observed_case_count: u8,
    cycles: u64,
    frames: u64,
    passed: bool,
    error: Option<String>,
}

impl Mapper4Mmc3EdgeObservation {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            observed_prg_values: [0; 3],
            observed_chr_values: [0; 8],
            observed_irq_counts: [0; 2],
            observed_case_count: 0,
            cycles: 0,
            frames: 0,
            passed: false,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct Mapper4Mmc3PrgRamObservation {
    observed_values: [u8; 4],
    sram_snapshot_values: [u8; 3],
    restored_values: [u8; 3],
    observed_case_count: u8,
    battery_backed: bool,
    cycles: u64,
    frames: u64,
    passed: bool,
    error: Option<String>,
}

impl Mapper4Mmc3PrgRamObservation {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            observed_values: [0; 4],
            sram_snapshot_values: [0; 3],
            restored_values: [0; 3],
            observed_case_count: 0,
            battery_backed: false,
            cycles: 0,
            frames: 0,
            passed: false,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct Mapper7AxromObservation {
    observed_prg_values: [u8; 4],
    observed_mirror_values: [u8; 3],
    observed_case_count: u8,
    cycles: u64,
    frames: u64,
    passed: bool,
    error: Option<String>,
}

impl Mapper7AxromObservation {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            observed_prg_values: [0; 4],
            observed_mirror_values: [0; 3],
            observed_case_count: 0,
            cycles: 0,
            frames: 0,
            passed: false,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct InputMaskSweepObservation {
    cases: Vec<InputMaskSweepCaseObservation>,
    error: Option<String>,
}

impl InputMaskSweepObservation {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            cases: Vec::new(),
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct InputMaskSweepCaseObservation {
    index: usize,
    joypad1_expected_mask: u8,
    joypad1_observed_mask: u8,
    joypad2_expected_mask: u8,
    joypad2_observed_mask: u8,
    observed_case_count: u8,
    cycles: u64,
    frames: u64,
    passed: bool,
    error: Option<String>,
}

impl InputMaskSweepCaseObservation {
    fn failed(
        index: usize,
        joypad1_mask: u8,
        joypad2_mask: u8,
        message: impl Into<String>,
    ) -> Self {
        Self {
            index,
            joypad1_expected_mask: joypad1_mask,
            joypad1_observed_mask: 0,
            joypad2_expected_mask: joypad2_mask,
            joypad2_observed_mask: 0,
            observed_case_count: 0,
            cycles: 0,
            frames: 0,
            passed: false,
            error: Some(message.into()),
        }
    }
}

#[derive(Debug, Clone)]
struct PpuScrollNametableWrapObservation {
    left_color: u32,
    right_color: u32,
    frames: u64,
    cycles: u64,
    passed: bool,
    error: Option<String>,
}

impl PpuScrollNametableWrapObservation {
    fn failed(message: impl Into<String>) -> Self {
        Self {
            left_color: 0,
            right_color: 0,
            frames: 0,
            cycles: 0,
            passed: false,
            error: Some(message.into()),
        }
    }
}

fn run_input_mask_sweep_variant() -> InputMaskSweepObservation {
    match try_run_input_mask_sweep_variant() {
        Ok(observation) => observation,
        Err(error) => InputMaskSweepObservation::failed(error),
    }
}

fn try_run_input_mask_sweep_variant() -> Result<InputMaskSweepObservation, String> {
    let rom = build_input_mask_sweep_variant_cartridge()?;
    let mut cases = Vec::with_capacity(INPUT_MASK_SWEEP_CASES.len());
    for (index, (joypad1_mask, joypad2_mask)) in INPUT_MASK_SWEEP_CASES.iter().copied().enumerate()
    {
        let case = try_run_input_mask_sweep_variant_case(&rom, index, joypad1_mask, joypad2_mask)
            .unwrap_or_else(|error| {
                InputMaskSweepCaseObservation::failed(index, joypad1_mask, joypad2_mask, error)
            });
        cases.push(case);
    }

    Ok(InputMaskSweepObservation { cases, error: None })
}

fn try_run_input_mask_sweep_variant_case(
    rom: &[u8],
    index: usize,
    joypad1_mask: u8,
    joypad2_mask: u8,
) -> Result<InputMaskSweepCaseObservation, String> {
    let cartridge = Cartridge::new(rom)?;
    let mut bus = Bus::new(cartridge);
    apply_joypad_mask(&mut bus, joypad1_mask);
    apply_joypad2_mask(&mut bus, joypad2_mask);
    bus.cpu_write(JOYPAD1_EXPECTED_MASK_ADDR as u16, joypad1_mask);
    bus.cpu_write(JOYPAD2_EXPECTED_MASK_ADDR as u16, joypad2_mask);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let cycle_limit = 20_000u64;

    while cycles < cycle_limit {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let _ = bus.apu.drain_samples();
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if matches!(status, STATUS_PASS | STATUS_FAIL) {
            let observed = read_input_mask_sweep_observed(&mut bus);
            let failure_code = read_ram_byte(&mut bus, FAILURE_CODE_ADDR);
            let passed = status == STATUS_PASS
                && observed.0 == joypad1_mask
                && observed.1 == joypad2_mask
                && observed.2 == 1;
            let error = if passed {
                None
            } else if status == STATUS_FAIL {
                Some(format!(
                    "input mask sweep case {index} reported FAIL with failure code 0x{failure_code:02X}"
                ))
            } else {
                Some(format!(
                    "input mask sweep case {index} reached PASS with mismatched host observations"
                ))
            };
            return Ok(InputMaskSweepCaseObservation {
                index,
                joypad1_expected_mask: joypad1_mask,
                joypad1_observed_mask: observed.0,
                joypad2_expected_mask: joypad2_mask,
                joypad2_observed_mask: observed.1,
                observed_case_count: observed.2,
                cycles,
                frames,
                passed,
                error,
            });
        }
    }

    let observed = read_input_mask_sweep_observed(&mut bus);
    Ok(InputMaskSweepCaseObservation {
        index,
        joypad1_expected_mask: joypad1_mask,
        joypad1_observed_mask: observed.0,
        joypad2_expected_mask: joypad2_mask,
        joypad2_observed_mask: observed.1,
        observed_case_count: observed.2,
        cycles,
        frames,
        passed: false,
        error: Some(format!(
            "input mask sweep case {index} timed out after {cycle_limit} cycles"
        )),
    })
}

fn read_input_mask_sweep_observed(bus: &mut Bus) -> (u8, u8, u8) {
    (
        bus.cpu_read(INPUT_MASK_SWEEP_JOYPAD1_OBSERVED_ADDR),
        bus.cpu_read(INPUT_MASK_SWEEP_JOYPAD2_OBSERVED_ADDR),
        bus.cpu_read(INPUT_MASK_SWEEP_CASE_COUNT_ADDR),
    )
}

fn run_mapper1_mmc1_variant() -> Mapper1Mmc1Observation {
    match try_run_mapper1_mmc1_variant() {
        Ok(observation) => observation,
        Err(error) => Mapper1Mmc1Observation::failed(error),
    }
}

fn try_run_mapper1_mmc1_variant() -> Result<Mapper1Mmc1Observation, String> {
    let rom = build_mapper1_mmc1_variant_cartridge()?;
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let cycle_limit = 50_000u64;

    while cycles < cycle_limit {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let _ = bus.apu.drain_samples();
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if matches!(status, STATUS_PASS | STATUS_FAIL) {
            let observed_prg_values = read_mapper1_mmc1_prg_observed_values(&mut bus);
            let observed_chr_values = read_mapper1_mmc1_chr_observed_values(&mut bus);
            let observed_mirror_values = read_mapper1_mmc1_mirror_observed_values(&mut bus);
            let observed_case_count = bus.cpu_read(MAPPER1_MMC1_CASE_COUNT_ADDR);
            let failure_code = read_ram_byte(&mut bus, FAILURE_CODE_ADDR);
            let passed = status == STATUS_PASS
                && observed_case_count == MAPPER1_EXPECTED_CASE_COUNT
                && observed_prg_values == MAPPER1_PRG_EXPECTED_VALUES
                && observed_chr_values == MAPPER1_CHR_EXPECTED_VALUES
                && observed_mirror_values == MAPPER1_MIRROR_EXPECTED_VALUES;
            let error = if passed {
                None
            } else if status == STATUS_FAIL {
                Some(format!(
                    "Mapper 1 MMC1 variant reported FAIL with failure code 0x{failure_code:02X}"
                ))
            } else {
                Some(
                    "Mapper 1 MMC1 variant reached PASS with mismatched host observations"
                        .to_string(),
                )
            };
            return Ok(Mapper1Mmc1Observation {
                observed_prg_values,
                observed_chr_values,
                observed_mirror_values,
                observed_case_count,
                cycles,
                frames,
                passed,
                error,
            });
        }
    }

    Ok(Mapper1Mmc1Observation {
        observed_prg_values: read_mapper1_mmc1_prg_observed_values(&mut bus),
        observed_chr_values: read_mapper1_mmc1_chr_observed_values(&mut bus),
        observed_mirror_values: read_mapper1_mmc1_mirror_observed_values(&mut bus),
        observed_case_count: bus.cpu_read(MAPPER1_MMC1_CASE_COUNT_ADDR),
        cycles,
        frames,
        passed: false,
        error: Some(format!(
            "Mapper 1 MMC1 variant timed out after {cycle_limit} cycles"
        )),
    })
}

fn read_mapper1_mmc1_prg_observed_values(bus: &mut Bus) -> [u8; 5] {
    let mut values = [0; 5];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER1_MMC1_PRG_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn read_mapper1_mmc1_chr_observed_values(bus: &mut Bus) -> [u8; 4] {
    let mut values = [0; 4];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER1_MMC1_CHR_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn read_mapper1_mmc1_mirror_observed_values(bus: &mut Bus) -> [u8; 3] {
    let mut values = [0; 3];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER1_MMC1_MIRROR_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn run_mapper1_mmc1_32k_prg_variant() -> Mapper1Mmc1Prg32kObservation {
    match try_run_mapper1_mmc1_32k_prg_variant() {
        Ok(observation) => observation,
        Err(error) => Mapper1Mmc1Prg32kObservation::failed(error),
    }
}

fn try_run_mapper1_mmc1_32k_prg_variant() -> Result<Mapper1Mmc1Prg32kObservation, String> {
    let rom = build_mapper1_mmc1_32k_prg_variant_cartridge()?;
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let cycle_limit = 50_000u64;

    while cycles < cycle_limit {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let _ = bus.apu.drain_samples();
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if matches!(status, STATUS_PASS | STATUS_FAIL) {
            let observed_values = read_mapper1_mmc1_32k_prg_observed_values(&mut bus);
            let observed_case_count = bus.cpu_read(MAPPER1_MMC1_32K_PRG_CASE_COUNT_ADDR);
            let failure_code = read_ram_byte(&mut bus, FAILURE_CODE_ADDR);
            let passed = status == STATUS_PASS
                && observed_case_count == MAPPER1_32K_EXPECTED_CASE_COUNT
                && observed_values == MAPPER1_32K_EXPECTED_VALUES;
            let error = if passed {
                None
            } else if status == STATUS_FAIL {
                Some(format!(
                    "Mapper 1 MMC1 32 KiB PRG variant reported FAIL with failure code 0x{failure_code:02X}"
                ))
            } else {
                Some(
                    "Mapper 1 MMC1 32 KiB PRG variant reached PASS with mismatched host observations"
                        .to_string(),
                )
            };
            return Ok(Mapper1Mmc1Prg32kObservation {
                observed_values,
                observed_case_count,
                cycles,
                frames,
                passed,
                error,
            });
        }
    }

    Ok(Mapper1Mmc1Prg32kObservation {
        observed_values: read_mapper1_mmc1_32k_prg_observed_values(&mut bus),
        observed_case_count: bus.cpu_read(MAPPER1_MMC1_32K_PRG_CASE_COUNT_ADDR),
        cycles,
        frames,
        passed: false,
        error: Some(format!(
            "Mapper 1 MMC1 32 KiB PRG variant timed out after {cycle_limit} cycles"
        )),
    })
}

fn read_mapper1_mmc1_32k_prg_observed_values(bus: &mut Bus) -> [u8; 10] {
    let mut values = [0; 10];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER1_MMC1_32K_PRG_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn run_mapper4_mmc3_variant() -> Mapper4Mmc3Observation {
    match try_run_mapper4_mmc3_variant() {
        Ok(observation) => observation,
        Err(error) => Mapper4Mmc3Observation::failed(error),
    }
}

fn try_run_mapper4_mmc3_variant() -> Result<Mapper4Mmc3Observation, String> {
    let rom = build_mapper4_mmc3_variant_cartridge()?;
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let cycle_limit = 120_000u64;

    while cycles < cycle_limit {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let _ = bus.apu.drain_samples();
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if matches!(status, STATUS_PASS | STATUS_FAIL) {
            let observed_prg_values = read_mapper4_mmc3_prg_observed_values(&mut bus);
            let observed_chr_values = read_mapper4_mmc3_chr_observed_values(&mut bus);
            let observed_mirror_values = read_mapper4_mmc3_mirror_observed_values(&mut bus);
            let observed_irq_count = bus.cpu_read(MAPPER4_MMC3_IRQ_OBSERVED_ADDR);
            let observed_case_count = bus.cpu_read(MAPPER4_MMC3_CASE_COUNT_ADDR);
            let failure_code = read_ram_byte(&mut bus, FAILURE_CODE_ADDR);
            let passed = status == STATUS_PASS
                && observed_case_count == MAPPER4_EXPECTED_CASE_COUNT
                && observed_prg_values == MAPPER4_PRG_EXPECTED_VALUES
                && observed_chr_values == MAPPER4_CHR_EXPECTED_VALUES
                && observed_mirror_values == MAPPER4_MIRROR_EXPECTED_VALUES
                && observed_irq_count == MAPPER4_EXPECTED_IRQ_COUNT;
            let error = if passed {
                None
            } else if status == STATUS_FAIL {
                Some(format!(
                    "Mapper 4 MMC3 variant reported FAIL with failure code 0x{failure_code:02X}"
                ))
            } else {
                Some(
                    "Mapper 4 MMC3 variant reached PASS with mismatched host observations"
                        .to_string(),
                )
            };
            return Ok(Mapper4Mmc3Observation {
                observed_prg_values,
                observed_chr_values,
                observed_mirror_values,
                observed_irq_count,
                observed_case_count,
                cycles,
                frames,
                passed,
                error,
            });
        }
    }

    Ok(Mapper4Mmc3Observation {
        observed_prg_values: read_mapper4_mmc3_prg_observed_values(&mut bus),
        observed_chr_values: read_mapper4_mmc3_chr_observed_values(&mut bus),
        observed_mirror_values: read_mapper4_mmc3_mirror_observed_values(&mut bus),
        observed_irq_count: bus.cpu_read(MAPPER4_MMC3_IRQ_OBSERVED_ADDR),
        observed_case_count: bus.cpu_read(MAPPER4_MMC3_CASE_COUNT_ADDR),
        cycles,
        frames,
        passed: false,
        error: Some(format!(
            "Mapper 4 MMC3 variant timed out after {cycle_limit} cycles"
        )),
    })
}

fn read_mapper4_mmc3_prg_observed_values(bus: &mut Bus) -> [u8; 3] {
    let mut values = [0; 3];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER4_MMC3_PRG_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn read_mapper4_mmc3_chr_observed_values(bus: &mut Bus) -> [u8; 5] {
    let mut values = [0; 5];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER4_MMC3_CHR_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn read_mapper4_mmc3_mirror_observed_values(bus: &mut Bus) -> [u8; 2] {
    let mut values = [0; 2];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER4_MMC3_MIRROR_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn run_mapper4_mmc3_edge_variant() -> Mapper4Mmc3EdgeObservation {
    match try_run_mapper4_mmc3_edge_variant() {
        Ok(observation) => observation,
        Err(error) => Mapper4Mmc3EdgeObservation::failed(error),
    }
}

fn try_run_mapper4_mmc3_edge_variant() -> Result<Mapper4Mmc3EdgeObservation, String> {
    let rom = build_mapper4_mmc3_edge_variant_cartridge()?;
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let cycle_limit = 160_000u64;

    while cycles < cycle_limit {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let _ = bus.apu.drain_samples();
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if matches!(status, STATUS_PASS | STATUS_FAIL) {
            let observed_prg_values = read_mapper4_mmc3_edge_prg_observed_values(&mut bus);
            let observed_chr_values = read_mapper4_mmc3_edge_chr_observed_values(&mut bus);
            let observed_irq_counts = read_mapper4_mmc3_edge_irq_observed_counts(&mut bus);
            let observed_case_count = bus.cpu_read(MAPPER4_MMC3_EDGE_CASE_COUNT_ADDR);
            let failure_code = read_ram_byte(&mut bus, FAILURE_CODE_ADDR);
            let passed = status == STATUS_PASS
                && observed_prg_values == MAPPER4_EDGE_PRG_EXPECTED_VALUES
                && observed_chr_values == MAPPER4_EDGE_CHR_EXPECTED_VALUES
                && observed_irq_counts == MAPPER4_EDGE_EXPECTED_IRQ_COUNTS
                && observed_case_count == MAPPER4_EDGE_EXPECTED_CASE_COUNT;
            let error = if passed {
                None
            } else if status == STATUS_FAIL {
                Some(format!(
                    "Mapper 4 MMC3 edge variant reported FAIL with failure code 0x{failure_code:02X}"
                ))
            } else {
                Some(
                    "Mapper 4 MMC3 edge variant reached PASS with mismatched host observations"
                        .to_string(),
                )
            };
            return Ok(Mapper4Mmc3EdgeObservation {
                observed_prg_values,
                observed_chr_values,
                observed_irq_counts,
                observed_case_count,
                cycles,
                frames,
                passed,
                error,
            });
        }
    }

    Ok(Mapper4Mmc3EdgeObservation {
        observed_prg_values: read_mapper4_mmc3_edge_prg_observed_values(&mut bus),
        observed_chr_values: read_mapper4_mmc3_edge_chr_observed_values(&mut bus),
        observed_irq_counts: read_mapper4_mmc3_edge_irq_observed_counts(&mut bus),
        observed_case_count: bus.cpu_read(MAPPER4_MMC3_EDGE_CASE_COUNT_ADDR),
        cycles,
        frames,
        passed: false,
        error: Some(format!(
            "Mapper 4 MMC3 edge variant timed out after {cycle_limit} cycles"
        )),
    })
}

fn read_mapper4_mmc3_edge_prg_observed_values(bus: &mut Bus) -> [u8; 3] {
    let mut values = [0; 3];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER4_MMC3_EDGE_PRG_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn read_mapper4_mmc3_edge_chr_observed_values(bus: &mut Bus) -> [u8; 8] {
    let mut values = [0; 8];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER4_MMC3_EDGE_CHR_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn read_mapper4_mmc3_edge_irq_observed_counts(bus: &mut Bus) -> [u8; 2] {
    let mut values = [0; 2];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER4_MMC3_EDGE_IRQ_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn run_mapper4_mmc3_prg_ram_variant() -> Mapper4Mmc3PrgRamObservation {
    match try_run_mapper4_mmc3_prg_ram_variant() {
        Ok(observation) => observation,
        Err(error) => Mapper4Mmc3PrgRamObservation::failed(error),
    }
}

fn try_run_mapper4_mmc3_prg_ram_variant() -> Result<Mapper4Mmc3PrgRamObservation, String> {
    let rom = build_mapper4_mmc3_prg_ram_variant_cartridge()?;
    let cartridge = Cartridge::new(&rom)?;
    let battery_backed = cartridge.has_battery;
    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let cycle_limit = 50_000u64;

    while cycles < cycle_limit {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let _ = bus.apu.drain_samples();
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if matches!(status, STATUS_PASS | STATUS_FAIL) {
            let observed_values = read_mapper4_mmc3_prg_ram_observed_values(&mut bus);
            let observed_case_count = bus.cpu_read(MAPPER4_MMC3_PRG_RAM_CASE_COUNT_ADDR);
            let sram = bus.get_sram();
            let sram_snapshot_values = read_mapper4_prg_ram_sram_values(&sram);
            let restored_values = restored_mapper4_prg_ram_values(&rom, &sram)?;
            let failure_code = read_ram_byte(&mut bus, FAILURE_CODE_ADDR);
            let passed = status == STATUS_PASS
                && battery_backed
                && observed_case_count == MAPPER4_PRG_RAM_EXPECTED_CASE_COUNT
                && observed_values == MAPPER4_PRG_RAM_EXPECTED_VALUES
                && sram_snapshot_values == MAPPER4_PRG_RAM_RESTORED_VALUES
                && restored_values == MAPPER4_PRG_RAM_RESTORED_VALUES;
            let error = if passed {
                None
            } else if status == STATUS_FAIL {
                Some(format!(
                    "Mapper 4 MMC3 PRG RAM variant reported FAIL with failure code 0x{failure_code:02X}"
                ))
            } else {
                Some(format!(
                    "Mapper 4 MMC3 PRG RAM variant reached PASS with mismatched host observations: battery_backed={}, observed {:?}, SRAM snapshot {:?}, restored {:?}, cases {}/{}",
                    battery_backed,
                    observed_values,
                    sram_snapshot_values,
                    restored_values,
                    observed_case_count,
                    MAPPER4_PRG_RAM_EXPECTED_CASE_COUNT
                ))
            };
            return Ok(Mapper4Mmc3PrgRamObservation {
                observed_values,
                sram_snapshot_values,
                restored_values,
                observed_case_count,
                battery_backed,
                cycles,
                frames,
                passed,
                error,
            });
        }
    }

    let sram = bus.get_sram();
    let sram_snapshot_values = read_mapper4_prg_ram_sram_values(&sram);
    let restored_values = restored_mapper4_prg_ram_values(&rom, &sram)?;
    Ok(Mapper4Mmc3PrgRamObservation {
        observed_values: read_mapper4_mmc3_prg_ram_observed_values(&mut bus),
        sram_snapshot_values,
        restored_values,
        observed_case_count: bus.cpu_read(MAPPER4_MMC3_PRG_RAM_CASE_COUNT_ADDR),
        battery_backed,
        cycles,
        frames,
        passed: false,
        error: Some(format!(
            "Mapper 4 MMC3 PRG RAM variant timed out after {cycle_limit} cycles"
        )),
    })
}

fn read_mapper4_mmc3_prg_ram_observed_values(bus: &mut Bus) -> [u8; 4] {
    let mut values = [0; 4];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER4_MMC3_PRG_RAM_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn read_mapper4_prg_ram_sram_values(sram: &[u8]) -> [u8; 3] {
    let mut values = [0; 3];
    for (index, &addr) in MAPPER4_PRG_RAM_RESTORED_ADDRS.iter().enumerate() {
        let offset = (addr - 0x6000) as usize;
        values[index] = sram.get(offset).copied().unwrap_or(0);
    }
    values
}

fn restored_mapper4_prg_ram_values(rom: &[u8], sram: &[u8]) -> Result<[u8; 3], String> {
    let mut cartridge = Cartridge::new(rom)?;
    cartridge.mapper.set_sram(sram);
    let mut values = [0; 3];
    for (index, &addr) in MAPPER4_PRG_RAM_RESTORED_ADDRS.iter().enumerate() {
        values[index] = cartridge.mapper.read_prg(addr);
    }
    Ok(values)
}

fn run_mapper7_axrom_variant() -> Mapper7AxromObservation {
    match try_run_mapper7_axrom_variant() {
        Ok(observation) => observation,
        Err(error) => Mapper7AxromObservation::failed(error),
    }
}

fn try_run_mapper7_axrom_variant() -> Result<Mapper7AxromObservation, String> {
    let rom = build_mapper7_axrom_variant_cartridge()?;
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let cycle_limit = 40_000u64;

    while cycles < cycle_limit {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let _ = bus.apu.drain_samples();
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if matches!(status, STATUS_PASS | STATUS_FAIL) {
            let observed_prg_values = read_mapper7_axrom_prg_observed_values(&mut bus);
            let observed_mirror_values = read_mapper7_axrom_mirror_observed_values(&mut bus);
            let observed_case_count = bus.cpu_read(MAPPER7_AXROM_CASE_COUNT_ADDR);
            let failure_code = read_ram_byte(&mut bus, FAILURE_CODE_ADDR);
            let passed = status == STATUS_PASS
                && observed_case_count == MAPPER7_EXPECTED_CASE_COUNT
                && observed_prg_values == MAPPER7_PRG_EXPECTED_VALUES
                && observed_mirror_values == MAPPER7_MIRROR_EXPECTED_VALUES;
            let error = if passed {
                None
            } else if status == STATUS_FAIL {
                Some(format!(
                    "Mapper 7 AxROM variant reported FAIL with failure code 0x{failure_code:02X}"
                ))
            } else {
                Some(
                    "Mapper 7 AxROM variant reached PASS with mismatched host observations"
                        .to_string(),
                )
            };
            return Ok(Mapper7AxromObservation {
                observed_prg_values,
                observed_mirror_values,
                observed_case_count,
                cycles,
                frames,
                passed,
                error,
            });
        }
    }

    Ok(Mapper7AxromObservation {
        observed_prg_values: read_mapper7_axrom_prg_observed_values(&mut bus),
        observed_mirror_values: read_mapper7_axrom_mirror_observed_values(&mut bus),
        observed_case_count: bus.cpu_read(MAPPER7_AXROM_CASE_COUNT_ADDR),
        cycles,
        frames,
        passed: false,
        error: Some(format!(
            "Mapper 7 AxROM variant timed out after {cycle_limit} cycles"
        )),
    })
}

fn read_mapper7_axrom_prg_observed_values(bus: &mut Bus) -> [u8; 4] {
    let mut values = [0; 4];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER7_AXROM_PRG_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn read_mapper7_axrom_mirror_observed_values(bus: &mut Bus) -> [u8; 3] {
    let mut values = [0; 3];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER7_AXROM_MIRROR_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn run_mapper3_chr_bank_variant() -> Mapper3ChrBankObservation {
    match try_run_mapper3_chr_bank_variant() {
        Ok(observation) => observation,
        Err(error) => Mapper3ChrBankObservation::failed(error),
    }
}

fn try_run_mapper3_chr_bank_variant() -> Result<Mapper3ChrBankObservation, String> {
    let rom = build_mapper3_chr_bank_variant_cartridge()?;
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let cycle_limit = 20_000u64;

    while cycles < cycle_limit {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            bus.apu.end_frame();
            let _ = bus.apu.drain_samples();
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if matches!(status, STATUS_PASS | STATUS_FAIL) {
            let observed_values = read_mapper3_chr_bank_observed_values(&mut bus);
            let observed_case_count = bus.cpu_read(MAPPER3_CHR_BANK_CASE_COUNT_ADDR);
            let failure_code = read_ram_byte(&mut bus, FAILURE_CODE_ADDR);
            let passed = status == STATUS_PASS
                && observed_case_count == MAPPER3_CHR_BANK_EXPECTED_CASE_COUNT
                && observed_values == MAPPER3_CHR_BANK_EXPECTED_VALUES;
            let error = if passed {
                None
            } else if status == STATUS_FAIL {
                Some(format!(
                    "Mapper 3 CHR-bank variant reported FAIL with failure code 0x{failure_code:02X}"
                ))
            } else {
                Some(
                    "Mapper 3 CHR-bank variant reached PASS with mismatched host observations"
                        .to_string(),
                )
            };
            return Ok(Mapper3ChrBankObservation {
                observed_values,
                observed_case_count,
                cycles,
                frames,
                passed,
                error,
            });
        }
    }

    Ok(Mapper3ChrBankObservation {
        observed_values: read_mapper3_chr_bank_observed_values(&mut bus),
        observed_case_count: bus.cpu_read(MAPPER3_CHR_BANK_CASE_COUNT_ADDR),
        cycles,
        frames,
        passed: false,
        error: Some(format!(
            "Mapper 3 CHR-bank variant timed out after {cycle_limit} cycles"
        )),
    })
}

fn read_mapper3_chr_bank_observed_values(bus: &mut Bus) -> [u8; 4] {
    let mut values = [0; 4];
    for (index, value) in values.iter_mut().enumerate() {
        *value = bus.cpu_read(MAPPER3_CHR_BANK_OBSERVED_BASE_ADDR + index as u16);
    }
    values
}

fn run_ppu_scroll_wrap_variant() -> PpuScrollNametableWrapObservation {
    match try_run_ppu_scroll_wrap_variant() {
        Ok(observation) => observation,
        Err(error) => PpuScrollNametableWrapObservation::failed(error),
    }
}

fn try_run_ppu_scroll_wrap_variant() -> Result<PpuScrollNametableWrapObservation, String> {
    let rom = build_ppu_scroll_wrap_variant_cartridge()?;
    let cartridge = Cartridge::new(&rom)?;
    let mut bus = Bus::new(cartridge);
    let mut cpu = Cpu::new();
    cpu.reset(&mut bus);

    let mut cycles = 0u64;
    let mut frames = 0u64;
    let mut captured_left = None;
    let mut captured_right = None;
    let cycle_limit = 160_000u64;

    while cycles < cycle_limit {
        cpu.clock(&mut bus);
        bus.tick(1);
        bus.tick_apu();
        cycles += 1;

        if bus.ppu.frame_complete() {
            frames += 1;
            let case_count = bus.cpu_read(PPU_SCROLL_SEAM_CASE_COUNT_ADDR);
            if read_ram_byte(&mut bus, CURRENT_TEST_ADDR) == PPU_SCROLL_SEAM_TEST_ID
                && case_count == PPU_SCROLL_SEAM_NAMETABLE_WRAP_CASE_COUNT
            {
                captured_left = Some(sample_frame_color(
                    &bus.ppu.frame_data,
                    PPU_SCROLL_SEAM_NAMETABLE_WRAP_LEFT_SAMPLE_X,
                    PPU_SCROLL_SEAM_NAMETABLE_WRAP_LEFT_SAMPLE_Y,
                ));
                captured_right = Some(sample_frame_color(
                    &bus.ppu.frame_data,
                    PPU_SCROLL_SEAM_NAMETABLE_WRAP_RIGHT_SAMPLE_X,
                    PPU_SCROLL_SEAM_NAMETABLE_WRAP_RIGHT_SAMPLE_Y,
                ));
            }
            bus.apu.end_frame();
            let _ = bus.apu.drain_samples();
        }

        let status = read_ram_byte(&mut bus, STATUS_ADDR);
        if let (STATUS_PASS, Some(left_color), Some(right_color)) =
            (status, captured_left, captured_right)
        {
            return Ok(PpuScrollNametableWrapObservation {
                left_color,
                right_color,
                frames,
                cycles,
                passed: left_color == PPU_SCROLL_SEAM_EXPECTED_NAMETABLE_WRAP_LEFT_COLOR
                    && right_color == PPU_SCROLL_SEAM_EXPECTED_NAMETABLE_WRAP_RIGHT_COLOR,
                error: None,
            });
        }
        if status == STATUS_FAIL {
            return Ok(PpuScrollNametableWrapObservation {
                left_color: captured_left.unwrap_or(0),
                right_color: captured_right.unwrap_or(0),
                frames,
                cycles,
                passed: false,
                error: Some("vertical-mirroring scroll-wrap variant reported FAIL".to_string()),
            });
        }
    }

    Ok(PpuScrollNametableWrapObservation {
        left_color: captured_left.unwrap_or(0),
        right_color: captured_right.unwrap_or(0),
        frames,
        cycles,
        passed: false,
        error: Some(format!(
            "vertical-mirroring scroll-wrap variant timed out after {cycle_limit} cycles"
        )),
    })
}

fn ppu_scroll_seam_telemetry(
    ram: &[u8],
    captured_sample: Option<&PpuScrollSeamFrameSample>,
    final_frame: &[u32],
    nametable_wrap: &PpuScrollNametableWrapObservation,
) -> PpuScrollSeamTelemetry {
    let observed_case_count = ram[(PPU_SCROLL_SEAM_CASE_COUNT_ADDR & 0x07FF) as usize];
    let sample = captured_sample.copied().unwrap_or_default();
    let left_color = sample.left_color.unwrap_or_else(|| {
        sample_frame_color(
            final_frame,
            PPU_SCROLL_SEAM_LEFT_SAMPLE_X,
            PPU_SCROLL_SEAM_LEFT_SAMPLE_Y,
        )
    });
    let right_color = sample.right_color.unwrap_or_else(|| {
        sample_frame_color(
            final_frame,
            PPU_SCROLL_SEAM_RIGHT_SAMPLE_X,
            PPU_SCROLL_SEAM_RIGHT_SAMPLE_Y,
        )
    });
    let coarse_left_color = sample.coarse_left_color.unwrap_or_else(|| {
        sample_frame_color(
            final_frame,
            PPU_SCROLL_SEAM_COARSE_LEFT_SAMPLE_X,
            PPU_SCROLL_SEAM_COARSE_LEFT_SAMPLE_Y,
        )
    });
    let coarse_right_color = sample.coarse_right_color.unwrap_or_else(|| {
        sample_frame_color(
            final_frame,
            PPU_SCROLL_SEAM_COARSE_RIGHT_SAMPLE_X,
            PPU_SCROLL_SEAM_COARSE_RIGHT_SAMPLE_Y,
        )
    });
    let top_color = sample.top_color.unwrap_or_else(|| {
        sample_frame_color(
            final_frame,
            PPU_SCROLL_SEAM_TOP_SAMPLE_X,
            PPU_SCROLL_SEAM_TOP_SAMPLE_Y,
        )
    });
    let bottom_color = sample.bottom_color.unwrap_or_else(|| {
        sample_frame_color(
            final_frame,
            PPU_SCROLL_SEAM_BOTTOM_SAMPLE_X,
            PPU_SCROLL_SEAM_BOTTOM_SAMPLE_Y,
        )
    });
    PpuScrollSeamTelemetry {
        expected_case_count: PPU_SCROLL_SEAM_EXPECTED_CASE_COUNT,
        observed_case_count,
        scroll_x: 0x04,
        coarse_scroll_x: 0x08,
        scroll_y: 0x04,
        left_sample_x: PPU_SCROLL_SEAM_LEFT_SAMPLE_X,
        left_sample_y: PPU_SCROLL_SEAM_LEFT_SAMPLE_Y,
        left_expected_color: PPU_SCROLL_SEAM_EXPECTED_LEFT_COLOR,
        left_expected_color_hex: hex_color(PPU_SCROLL_SEAM_EXPECTED_LEFT_COLOR),
        left_observed_color: left_color,
        left_observed_color_hex: hex_color(left_color),
        right_sample_x: PPU_SCROLL_SEAM_RIGHT_SAMPLE_X,
        right_sample_y: PPU_SCROLL_SEAM_RIGHT_SAMPLE_Y,
        right_expected_color: PPU_SCROLL_SEAM_EXPECTED_RIGHT_COLOR,
        right_expected_color_hex: hex_color(PPU_SCROLL_SEAM_EXPECTED_RIGHT_COLOR),
        right_observed_color: right_color,
        right_observed_color_hex: hex_color(right_color),
        coarse_left_sample_x: PPU_SCROLL_SEAM_COARSE_LEFT_SAMPLE_X,
        coarse_left_sample_y: PPU_SCROLL_SEAM_COARSE_LEFT_SAMPLE_Y,
        coarse_left_expected_color: PPU_SCROLL_SEAM_EXPECTED_COARSE_LEFT_COLOR,
        coarse_left_expected_color_hex: hex_color(PPU_SCROLL_SEAM_EXPECTED_COARSE_LEFT_COLOR),
        coarse_left_observed_color: coarse_left_color,
        coarse_left_observed_color_hex: hex_color(coarse_left_color),
        coarse_right_sample_x: PPU_SCROLL_SEAM_COARSE_RIGHT_SAMPLE_X,
        coarse_right_sample_y: PPU_SCROLL_SEAM_COARSE_RIGHT_SAMPLE_Y,
        coarse_right_expected_color: PPU_SCROLL_SEAM_EXPECTED_COARSE_RIGHT_COLOR,
        coarse_right_expected_color_hex: hex_color(PPU_SCROLL_SEAM_EXPECTED_COARSE_RIGHT_COLOR),
        coarse_right_observed_color: coarse_right_color,
        coarse_right_observed_color_hex: hex_color(coarse_right_color),
        nametable_wrap_mirroring: "vertical".to_string(),
        nametable_wrap_scroll_x: PPU_SCROLL_SEAM_NAMETABLE_WRAP_SCROLL_X,
        nametable_wrap_scroll_y: PPU_SCROLL_SEAM_NAMETABLE_WRAP_SCROLL_Y,
        nametable_wrap_left_sample_x: PPU_SCROLL_SEAM_NAMETABLE_WRAP_LEFT_SAMPLE_X,
        nametable_wrap_left_sample_y: PPU_SCROLL_SEAM_NAMETABLE_WRAP_LEFT_SAMPLE_Y,
        nametable_wrap_left_expected_color: PPU_SCROLL_SEAM_EXPECTED_NAMETABLE_WRAP_LEFT_COLOR,
        nametable_wrap_left_expected_color_hex: hex_color(
            PPU_SCROLL_SEAM_EXPECTED_NAMETABLE_WRAP_LEFT_COLOR,
        ),
        nametable_wrap_left_observed_color: nametable_wrap.left_color,
        nametable_wrap_left_observed_color_hex: hex_color(nametable_wrap.left_color),
        nametable_wrap_right_sample_x: PPU_SCROLL_SEAM_NAMETABLE_WRAP_RIGHT_SAMPLE_X,
        nametable_wrap_right_sample_y: PPU_SCROLL_SEAM_NAMETABLE_WRAP_RIGHT_SAMPLE_Y,
        nametable_wrap_right_expected_color: PPU_SCROLL_SEAM_EXPECTED_NAMETABLE_WRAP_RIGHT_COLOR,
        nametable_wrap_right_expected_color_hex: hex_color(
            PPU_SCROLL_SEAM_EXPECTED_NAMETABLE_WRAP_RIGHT_COLOR,
        ),
        nametable_wrap_right_observed_color: nametable_wrap.right_color,
        nametable_wrap_right_observed_color_hex: hex_color(nametable_wrap.right_color),
        nametable_wrap_frames: nametable_wrap.frames,
        nametable_wrap_cycles: nametable_wrap.cycles,
        nametable_wrap_passed: nametable_wrap.passed,
        nametable_wrap_error: nametable_wrap.error.clone(),
        top_sample_x: PPU_SCROLL_SEAM_TOP_SAMPLE_X,
        top_sample_y: PPU_SCROLL_SEAM_TOP_SAMPLE_Y,
        top_expected_color: PPU_SCROLL_SEAM_EXPECTED_TOP_COLOR,
        top_expected_color_hex: hex_color(PPU_SCROLL_SEAM_EXPECTED_TOP_COLOR),
        top_observed_color: top_color,
        top_observed_color_hex: hex_color(top_color),
        bottom_sample_x: PPU_SCROLL_SEAM_BOTTOM_SAMPLE_X,
        bottom_sample_y: PPU_SCROLL_SEAM_BOTTOM_SAMPLE_Y,
        bottom_expected_color: PPU_SCROLL_SEAM_EXPECTED_BOTTOM_COLOR,
        bottom_expected_color_hex: hex_color(PPU_SCROLL_SEAM_EXPECTED_BOTTOM_COLOR),
        bottom_observed_color: bottom_color,
        bottom_observed_color_hex: hex_color(bottom_color),
        passed: observed_case_count == PPU_SCROLL_SEAM_EXPECTED_CASE_COUNT
            && left_color == PPU_SCROLL_SEAM_EXPECTED_LEFT_COLOR
            && right_color == PPU_SCROLL_SEAM_EXPECTED_RIGHT_COLOR
            && coarse_left_color == PPU_SCROLL_SEAM_EXPECTED_COARSE_LEFT_COLOR
            && coarse_right_color == PPU_SCROLL_SEAM_EXPECTED_COARSE_RIGHT_COLOR
            && nametable_wrap.passed
            && top_color == PPU_SCROLL_SEAM_EXPECTED_TOP_COLOR
            && bottom_color == PPU_SCROLL_SEAM_EXPECTED_BOTTOM_COLOR,
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

#[derive(Debug, Clone, Copy)]
struct PpuSpritePriorityFrameSample {
    front_color: u32,
    behind_color: u32,
}

#[derive(Debug, Clone, Copy, Default)]
struct PpuScrollSeamFrameSample {
    left_color: Option<u32>,
    right_color: Option<u32>,
    coarse_left_color: Option<u32>,
    coarse_right_color: Option<u32>,
    top_color: Option<u32>,
    bottom_color: Option<u32>,
}

fn maybe_capture_diagnostic_render_frame(
    retained_frame: &mut Option<FrameTelemetry>,
    current_test: u8,
    frame: &[u32],
    validate_signature: bool,
    validation_reason: &'static str,
) {
    if retained_frame.is_some() || current_test != 10 {
        return;
    }
    let telemetry = frame_telemetry(frame, validate_signature, validation_reason);
    if telemetry.unique_colors >= 2 {
        *retained_frame = Some(telemetry);
    }
}

fn maybe_capture_ppu_sprite_priority_frame(
    retained_sample: &mut Option<PpuSpritePriorityFrameSample>,
    current_test: u8,
    frame: &[u32],
) {
    if current_test != PPU_SPRITE_PRIORITY_TEST_ID {
        return;
    }
    *retained_sample = Some(PpuSpritePriorityFrameSample {
        front_color: sample_frame_color(
            frame,
            PPU_SPRITE_PRIORITY_FRONT_SAMPLE_X,
            PPU_SPRITE_PRIORITY_FRONT_SAMPLE_Y,
        ),
        behind_color: sample_frame_color(
            frame,
            PPU_SPRITE_PRIORITY_BEHIND_SAMPLE_X,
            PPU_SPRITE_PRIORITY_BEHIND_SAMPLE_Y,
        ),
    });
}

fn maybe_capture_ppu_scroll_seam_frame(
    retained_sample: &mut Option<PpuScrollSeamFrameSample>,
    current_test: u8,
    observed_case_count: u8,
    frame: &[u32],
) {
    if current_test != PPU_SCROLL_SEAM_TEST_ID {
        return;
    }
    let sample = retained_sample.get_or_insert_with(PpuScrollSeamFrameSample::default);
    match observed_case_count {
        4 => {
            sample.left_color = Some(sample_frame_color(
                frame,
                PPU_SCROLL_SEAM_LEFT_SAMPLE_X,
                PPU_SCROLL_SEAM_LEFT_SAMPLE_Y,
            ));
            sample.right_color = Some(sample_frame_color(
                frame,
                PPU_SCROLL_SEAM_RIGHT_SAMPLE_X,
                PPU_SCROLL_SEAM_RIGHT_SAMPLE_Y,
            ));
            sample.top_color = Some(sample_frame_color(
                frame,
                PPU_SCROLL_SEAM_TOP_SAMPLE_X,
                PPU_SCROLL_SEAM_TOP_SAMPLE_Y,
            ));
            sample.bottom_color = Some(sample_frame_color(
                frame,
                PPU_SCROLL_SEAM_BOTTOM_SAMPLE_X,
                PPU_SCROLL_SEAM_BOTTOM_SAMPLE_Y,
            ));
        }
        PPU_SCROLL_SEAM_EXPECTED_CASE_COUNT => {
            sample.coarse_left_color = Some(sample_frame_color(
                frame,
                PPU_SCROLL_SEAM_COARSE_LEFT_SAMPLE_X,
                PPU_SCROLL_SEAM_COARSE_LEFT_SAMPLE_Y,
            ));
            sample.coarse_right_color = Some(sample_frame_color(
                frame,
                PPU_SCROLL_SEAM_COARSE_RIGHT_SAMPLE_X,
                PPU_SCROLL_SEAM_COARSE_RIGHT_SAMPLE_Y,
            ));
        }
        _ => {}
    }
}

fn frame_telemetry(
    frame: &[u32],
    validate_signature: bool,
    validation_reason: &'static str,
) -> FrameTelemetry {
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
    let checksum = hash_bytes(&bytes);
    let unique_colors = colors.len();
    FrameTelemetry {
        checksum,
        checksum_hex: hex_u64(checksum),
        expected_checksum: DIAGNOSTIC_RENDER_FRAME_EXPECTED_CHECKSUM,
        expected_checksum_hex: hex_u64(DIAGNOSTIC_RENDER_FRAME_EXPECTED_CHECKSUM),
        checksum_matches_expected: checksum == DIAGNOSTIC_RENDER_FRAME_EXPECTED_CHECKSUM,
        checksum_validation_enabled: validate_signature,
        checksum_validation_reason: validation_reason.to_string(),
        unique_colors,
        expected_unique_colors: DIAGNOSTIC_RENDER_FRAME_EXPECTED_UNIQUE_COLORS,
        unique_colors_match_expected: unique_colors
            == DIAGNOSTIC_RENDER_FRAME_EXPECTED_UNIQUE_COLORS,
        nonzero_pixels,
        expected_nonzero_pixels: DIAGNOSTIC_RENDER_FRAME_EXPECTED_NONZERO_PIXELS,
        nonzero_pixels_match_expected: nonzero_pixels
            == DIAGNOSTIC_RENDER_FRAME_EXPECTED_NONZERO_PIXELS,
    }
}

fn audio_telemetry(
    sample_count: usize,
    peak_abs: f32,
    sum_abs: f64,
    sum_squares: f64,
) -> AudioTelemetry {
    let mean_abs = if sample_count == 0 {
        0.0
    } else {
        (sum_abs / sample_count as f64) as f32
    };
    let rms_abs = if sample_count == 0 {
        0.0
    } else {
        (sum_squares / sample_count as f64).sqrt() as f32
    };
    let sample_count_passed = (APU_AUDIO_EXPECTED_MIN_SAMPLE_COUNT
        ..=APU_AUDIO_EXPECTED_MAX_SAMPLE_COUNT)
        .contains(&sample_count);
    let peak_abs_passed = audio_level_in_range(
        peak_abs,
        APU_AUDIO_EXPECTED_MIN_PEAK_ABS,
        APU_AUDIO_EXPECTED_MAX_PEAK_ABS,
    );
    let rms_abs_passed = audio_level_in_range(
        rms_abs,
        APU_AUDIO_EXPECTED_MIN_RMS_ABS,
        APU_AUDIO_EXPECTED_MAX_RMS_ABS,
    );
    let mean_abs_passed = audio_level_in_range(
        mean_abs,
        APU_AUDIO_EXPECTED_MIN_MEAN_ABS,
        APU_AUDIO_EXPECTED_MAX_MEAN_ABS,
    );

    AudioTelemetry {
        sample_count,
        expected_min_sample_count: APU_AUDIO_EXPECTED_MIN_SAMPLE_COUNT,
        expected_max_sample_count: APU_AUDIO_EXPECTED_MAX_SAMPLE_COUNT,
        sample_count_passed,
        peak_abs,
        expected_min_peak_abs: APU_AUDIO_EXPECTED_MIN_PEAK_ABS,
        expected_max_peak_abs: APU_AUDIO_EXPECTED_MAX_PEAK_ABS,
        peak_abs_passed,
        rms_abs,
        expected_min_rms_abs: APU_AUDIO_EXPECTED_MIN_RMS_ABS,
        expected_max_rms_abs: APU_AUDIO_EXPECTED_MAX_RMS_ABS,
        rms_abs_passed,
        mean_abs,
        expected_min_mean_abs: APU_AUDIO_EXPECTED_MIN_MEAN_ABS,
        expected_max_mean_abs: APU_AUDIO_EXPECTED_MAX_MEAN_ABS,
        mean_abs_passed,
        passed: sample_count_passed && peak_abs_passed && rms_abs_passed && mean_abs_passed,
    }
}

fn audio_level_in_range(value: f32, min: f32, max: f32) -> bool {
    value.is_finite() && value >= min && value <= max
}

fn sample_frame_color(frame: &[u32], x: usize, y: usize) -> u32 {
    frame.get(y * 256 + x).copied().unwrap_or(0)
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
        &["dma", "oam_dma_transfer_count"][..],
        &["dma", "oam_dma_total_active_cycles"][..],
        &["dma", "oam_dma_active_cycle_buckets"][..],
        &["dma", "oam_dma_active_cycle_parities"][..],
        &["dma", "oam_dma_phase_matrix_test_transfer_count"][..],
        &["dma", "oam_dma_phase_matrix_has_even_start"][..],
        &["dma", "oam_dma_phase_matrix_has_odd_start"][..],
        &["dma", "oam_dma_phase_matrix_passed"][..],
        &["dma", "dmc_dma_fetches_during_oam_dma"][..],
        &["dma", "dmc_dma_oam_overlap_observed"][..],
        &["dma", "dmc_dma_first_oam_overlap_test_name"][..],
        &["dma", "dmc_dma_first_fetch_cpu_cycle_parity"][..],
        &["dma", "dmc_dma_first_fetch_stall_cycles"][..],
        &["dma", "dmc_dma_first_oam_overlap_cpu_cycle_parity"][..],
        &["dma", "dmc_dma_first_oam_overlap_stall_cycles"][..],
        &["dma", "dmc_dma_oam_overlap_offsets"][..],
        &["dma", "dmc_dma_oam_overlap_transfer_indices"][..],
        &["dma", "dmc_dma_oam_overlap_phase_matrix_transfer_indices"][..],
        &[
            "dma",
            "dmc_dma_oam_overlap_phase_matrix_distinct_transfer_count",
        ][..],
        &["dma", "dmc_dma_oam_overlap_burst_train_passed"][..],
        &["dma", "dmc_dma_oam_overlap_position_buckets"][..],
        &["dma", "dmc_dma_oam_overlap_covered_position_buckets"][..],
        &["dma", "dmc_dma_oam_overlap_missing_position_buckets"][..],
        &["dma", "dmc_dma_oam_overlap_position_matrix_passed"][..],
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
    if probe_id == "ppu.frame_checksum"
        && (baseline_observed
            .as_deref()
            .is_some_and(|observed| observed.contains("validation disabled"))
            || current_observed
                .as_deref()
                .is_some_and(|observed| observed.contains("validation disabled")))
    {
        return;
    }
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
        &["cpu_branch_matrix", "taken_mask"][..],
        &["cpu_branch_matrix", "not_taken_mask"][..],
        &["cpu_branch_matrix", "page_cross_result"][..],
        &["cpu_branch_matrix", "observed_case_count"][..],
        &["cpu_branch_matrix", "passed"][..],
        &["cpu_stack_matrix", "tsx_result"][..],
        &["cpu_stack_matrix", "pull_result"][..],
        &["cpu_stack_matrix", "status_result"][..],
        &["cpu_stack_matrix", "jsr_result"][..],
        &["cpu_stack_matrix", "final_stack_pointer"][..],
        &["cpu_stack_matrix", "observed_case_count"][..],
        &["cpu_stack_matrix", "passed"][..],
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
        &["ppu_sprite_zero_hit", "observed_status_bit"][..],
        &["ppu_sprite_zero_hit", "observed_case_count"][..],
        &["ppu_sprite_zero_hit", "passed"][..],
        &["ppu_sprite_overflow", "observed_status_bit"][..],
        &["ppu_sprite_overflow", "false_positive_observed_status_bit"][..],
        &["ppu_sprite_overflow", "false_negative_observed_status_bit"][..],
        &["ppu_sprite_overflow", "observed_case_count"][..],
        &["ppu_sprite_overflow", "hardware_bug_matrix_passed"][..],
        &["ppu_sprite_overflow", "passed"][..],
        &["ppu_sprite_priority", "front_observed_color"][..],
        &["ppu_sprite_priority", "behind_observed_color"][..],
        &["ppu_sprite_priority", "observed_case_count"][..],
        &["ppu_sprite_priority", "passed"][..],
        &["ppu_vblank_timing", "first_nmi_latency_cycles"][..],
        &["ppu_vblank_timing", "inter_nmi_cycles"][..],
        &["ppu_vblank_timing", "edge_set_count"][..],
        &["ppu_vblank_timing", "edge_clear_count"][..],
        &["ppu_vblank_timing", "edge_nmi_trigger_count"][..],
        &["ppu_vblank_timing", "edge_first_set_ppu_scanline"][..],
        &["ppu_vblank_timing", "edge_first_set_ppu_dot"][..],
        &["ppu_vblank_timing", "edge_first_clear_ppu_scanline"][..],
        &["ppu_vblank_timing", "edge_first_clear_ppu_dot"][..],
        &["ppu_vblank_timing", "edge_second_set_ppu_scanline"][..],
        &["ppu_vblank_timing", "edge_second_set_ppu_dot"][..],
        &["ppu_vblank_timing", "edge_passed"][..],
        &["ppu_vblank_timing", "passed"][..],
        &["ppu_scroll_seam", "left_observed_color"][..],
        &["ppu_scroll_seam", "right_observed_color"][..],
        &["ppu_scroll_seam", "coarse_left_observed_color"][..],
        &["ppu_scroll_seam", "coarse_right_observed_color"][..],
        &["ppu_scroll_seam", "nametable_wrap_left_observed_color"][..],
        &["ppu_scroll_seam", "nametable_wrap_right_observed_color"][..],
        &["ppu_scroll_seam", "nametable_wrap_passed"][..],
        &["ppu_scroll_seam", "top_observed_color"][..],
        &["ppu_scroll_seam", "bottom_observed_color"][..],
        &["ppu_scroll_seam", "observed_case_count"][..],
        &["ppu_scroll_seam", "passed"][..],
        &["apu_status_matrix", "observed_mask"][..],
        &["apu_status_matrix", "observed_case_count"][..],
        &["apu_status_matrix", "pulse1_status_bit"][..],
        &["apu_status_matrix", "pulse2_status_bit"][..],
        &["apu_status_matrix", "triangle_status_bit"][..],
        &["apu_status_matrix", "noise_status_bit"][..],
        &["apu_status_matrix", "passed"][..],
        &["apu_dmc_status", "observed_bit"][..],
        &["apu_dmc_status", "observed_case_count"][..],
        &["apu_dmc_status", "dmc_status_bit"][..],
        &["apu_dmc_status", "passed"][..],
        &["audio", "sample_count"][..],
        &["audio", "sample_count_passed"][..],
        &["audio", "peak_abs"][..],
        &["audio", "peak_abs_passed"][..],
        &["audio", "rms_abs"][..],
        &["audio", "rms_abs_passed"][..],
        &["audio", "mean_abs"][..],
        &["audio", "mean_abs_passed"][..],
        &["audio", "passed"][..],
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

    if json_bool(current, &["frame", "checksum_validation_enabled"]).unwrap_or(true)
        && json_bool(baseline, &["frame", "checksum_validation_enabled"]).unwrap_or(true)
    {
        for path in [
            &["frame", "checksum"][..],
            &["frame", "checksum_matches_expected"][..],
            &["frame", "unique_colors"][..],
            &["frame", "nonzero_pixels"][..],
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

fn optional_u16(value: Option<u16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn optional_i16(value: Option<i16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn optional_string(value: Option<&str>) -> String {
    value.unwrap_or("none").to_string()
}

fn optional_pc(value: Option<u16>) -> String {
    value.map(format_pc).unwrap_or_else(|| "none".to_string())
}

fn format_audio_level(value: f32) -> String {
    format!("{value:.6}")
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

fn hex_color(value: u32) -> String {
    format!("0x{value:06X}")
}

fn hex_u64(value: u64) -> String {
    format!("0x{value:016X}")
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
            .any(|gap| gap.id == "ppu_pixel_pipeline"
                && gap
                    .current_coverage
                    .contains("coarse-X nametable-wrap sampling")
                && gap
                    .current_coverage
                    .contains("coarse-X tile-shift sampling")
                && gap
                    .current_coverage
                    .contains("PPUSTATUS vblank set/clear dot-edge timing")
                && gap
                    .current_coverage
                    .contains("sprite-overflow evaluation including hardware-bug")
                && gap
                    .current_coverage
                    .contains("expected full-frame render checksum")
                && !gap.missing_coverage.contains("vblank edge timing")
                && !gap
                    .missing_coverage
                    .contains("Sprite overflow hardware-bug")
                && !gap.suggested_next_test.contains("expected frame checksums")
                && gap
                    .missing_coverage
                    .contains("Per-dot rendering behavior beyond targeted")));
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
        assert!(telemetry.apu_status_matrix.passed);
        assert_eq!(telemetry.apu_status_matrix.observed_mask, 0x0F);
        assert_eq!(telemetry.apu_status_matrix.observed_case_count, 4);
        assert!(telemetry.apu_dmc_status.passed);
        assert_eq!(telemetry.apu_dmc_status.observed_bit, 0x10);
        assert_eq!(telemetry.apu_dmc_status.observed_case_count, 1);
        assert!(telemetry.audio.sample_count_passed);
        assert!(telemetry.audio.peak_abs_passed);
        assert!(telemetry.audio.rms_abs_passed);
        assert!(telemetry.audio.mean_abs_passed);
        assert!(telemetry.audio.passed);
        assert!(telemetry.frame.unique_colors >= 2);
        assert!(telemetry.frame.checksum_matches_expected);
        assert!(telemetry.frame.checksum_validation_enabled);
        assert_eq!(
            telemetry.frame.checksum,
            DIAGNOSTIC_RENDER_FRAME_EXPECTED_CHECKSUM
        );
        assert_eq!(
            telemetry.frame.nonzero_pixels,
            DIAGNOSTIC_RENDER_FRAME_EXPECTED_NONZERO_PIXELS
        );
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
