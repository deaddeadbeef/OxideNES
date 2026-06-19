# Mapper 3 Rendered CHR Bank Diagnostic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a generated Mapper 3/CNROM diagnostic variant proving CHR bank writes made while rendering is enabled are visible in host-sampled background pixels.

**Architecture:** Reuse the existing generated variant-cartridge path in `src/diagnostic.rs`. Add a second Mapper 3 variant that renders one tile from CHR bank 0, keeps background rendering enabled, switches to CHR bank 1, renders again, and exposes both sampled colors in telemetry, probes, reports, verifiers, docs, and tests.

**Tech Stack:** Rust diagnostic generator/telemetry, synthetic iNES cartridges, host frame sampling, Python diagnostic verifiers, Cargo tests.

---

### Task 1: Add Rendered Mapper 3 Variant Telemetry

**Files:**
- Modify: `src/diagnostic.rs`
- Test: `tests/diagnostic_cartridge_tests.rs`

- [x] **Step 1: Define constants and telemetry fields**

Add constants near the existing `MAPPER3_*` block:

```rust
const MAPPER3_RENDERED_CHR_BANK_EXPECTED_CASE_COUNT: u8 = 2;
const MAPPER3_RENDERED_CHR_BANK_OBSERVED_STAGE_ADDR: u16 = 0x030A;
const MAPPER3_RENDERED_CHR_BANK_CASE_COUNT_ADDR: u16 = 0x030B;
const MAPPER3_RENDERED_CHR_BANK_SAMPLE_X: usize = 8;
const MAPPER3_RENDERED_CHR_BANK_SAMPLE_Y: usize = 8;
const MAPPER3_RENDERED_CHR_BANK_BANK0_EXPECTED_COLOR: u32 = 0x64B0FF;
const MAPPER3_RENDERED_CHR_BANK_BANK1_EXPECTED_COLOR: u32 = 0xB53120;
```

Add `mapper3_rendered_chr_bank: Mapper3RenderedChrBankTelemetry` to `DiagnosticTelemetry`.

- [x] **Step 2: Add telemetry and observation structs**

Add a public serializable telemetry struct:

```rust
#[derive(Debug, Serialize)]
pub struct Mapper3RenderedChrBankTelemetry {
    pub mapper: u8,
    pub prg_banks: u8,
    pub chr_banks: u8,
    pub sample_x: usize,
    pub sample_y: usize,
    pub expected_case_count: u8,
    pub observed_case_count: u8,
    pub expected_banks: Vec<u8>,
    pub bank0_expected_color: u32,
    pub bank0_expected_color_hex: String,
    pub bank0_observed_color: u32,
    pub bank0_observed_color_hex: String,
    pub bank1_expected_color: u32,
    pub bank1_expected_color_hex: String,
    pub bank1_observed_color: u32,
    pub bank1_observed_color_hex: String,
    pub cycles: u64,
    pub frames: u64,
    pub passed: bool,
    pub error: Option<String>,
}
```

Add a private observation struct with the same observed colors, case count, cycles, frames, pass bit, and error.

- [x] **Step 3: Add generated cartridge and CHR ROM**

Add `build_mapper3_rendered_chr_bank_variant_cartridge()` next to `build_mapper3_chr_bank_variant_cartridge()`. Reuse mapper `3`, two PRG banks, four 8 KiB CHR banks, no Mapper 2 sentinels.

Add `build_mapper3_rendered_chr_bank_variant_chr_rom()` that makes tile `0x01` in bank 0 render palette index 1 and tile `0x01` in bank 1 render palette index 2:

```rust
for row in 0..8 {
    chr[CHR_BANK_SIZE + 16 + row] = 0x00;
    chr[CHR_BANK_SIZE + 16 + 8 + row] = 0xFF;
    chr[16 + row] = 0xFF;
    chr[16 + 8 + row] = 0x00;
}
```

- [x] **Step 4: Add the variant program**

Add `build_mapper3_rendered_chr_bank_variant_program_with_labels()`. The program must:

1. initialize diagnostic RAM, status, NMI count, and disable rendering;
2. select CHR bank 0 by writing `0x00` to `$8000`;
3. write palette `$3F00=0x0F`, `$3F01=0x21`, `$3F02=0x16`;
4. write tile `0x01` to `$2000`;
5. set scroll to 0/0 and enable background rendering with `$2001=0x0A`;
6. wait for two vblanks and write stage `1` to `MAPPER3_RENDERED_CHR_BANK_OBSERVED_STAGE_ADDR`;
7. while rendering stays enabled, write `0x01` to `$8000`;
8. wait for two more vblanks and write stage `2` plus case count `2`;
9. write `STATUS_PASS` and hang.

- [x] **Step 5: Run and sample both frames**

Add `run_mapper3_rendered_chr_bank_variant()` and `try_run_mapper3_rendered_chr_bank_variant()`. In the host loop, when `bus.ppu.frame_complete()` fires, read `MAPPER3_RENDERED_CHR_BANK_OBSERVED_STAGE_ADDR` and sample `(8, 8)` from `bus.ppu.frame_data` into bank 0 color for stage 1 and bank 1 color for stage 2. Mark the observation passed only when stage 2 reaches `STATUS_PASS`, case count is `2`, and sampled colors match `0x64B0FF` and `0xB53120`.

### Task 2: Thread Probe, Report, Verifier, Docs

**Files:**
- Modify: `src/diagnostic.rs`
- Modify: `scripts/verify_diagnostic_observability.py`
- Modify: `docs/DIAGNOSTIC_CARTRIDGE.md`
- Modify: `docs/RELEASE_CANDIDATE_GATES.md`

- [x] **Step 1: Thread telemetry through run and validation**

Call `mapper3_rendered_chr_bank_telemetry(&run_mapper3_rendered_chr_bank_variant())` in `run_diagnostic()`, include it in `HostValidationInput` and `ProbeTelemetryInput`, and add a host validation message:

```text
Mapper 3 rendered CHR-bank variant mismatch: observed bank0 ... bank1 ...
```

- [x] **Step 2: Add report rows and probe**

Add report rows under `## Cartridge Mapper Variants` for:

```text
Mapper 3 rendered CHR-bank sample
Mapper 3 rendered CHR-bank colors / expected
Mapper 3 rendered CHR-bank cases / expected
Mapper 3 rendered CHR-bank frames / cycles / passed
Mapper 3 rendered CHR-bank error
```

Add probe `mapper3.rendered_chr_bank_switch` with likely domain `cartridge.mapper3_rendered_chr_bank`.

- [x] **Step 3: Update expected counts**

Increase the expected probe count in `scripts/verify_diagnostic_observability.py` from `89` to `90` and assert the new probe id appears in the telemetry catalog through the existing expected probe list.

- [x] **Step 4: Update docs**

Document that the Mapper 3 coverage now includes host-sampled rendered pixels after a render-enabled CHR bank switch. Narrow the mapper coverage gap text from “Active-render CHR/PRG switches” to “Active-render PRG switches, deeper MMC3 IRQ A12 filtering behavior...” because render-enabled CHR switching is now covered for CNROM.

### Task 3: Verify Locally

**Files:**
- Test: `tests/diagnostic_cartridge_tests.rs`
- Test: `scripts/verify_diagnostic_observability.py`

- [x] **Step 1: Add healthy telemetry assertions**

In `generated_diagnostic_cartridge_runs_headlessly_to_pass`, assert:

```rust
assert!(telemetry.mapper3_rendered_chr_bank.passed);
assert_eq!(telemetry.mapper3_rendered_chr_bank.observed_case_count, 2);
assert_eq!(telemetry.mapper3_rendered_chr_bank.bank0_observed_color_hex, "0x64B0FF");
assert_eq!(telemetry.mapper3_rendered_chr_bank.bank1_observed_color_hex, "0xB53120");
```

Also assert the probe `mapper3.rendered_chr_bank_switch` passed and report rows mention the rendered CHR-bank colors.

- [x] **Step 2: Run focused checks**

Run:

```powershell
cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass
cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite
python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/mapper3-rendered-chr-suite
python scripts/verify_diagnostic_observability.py --suite-dir target/diagnostics/mapper3-rendered-chr-suite
```

- [x] **Step 3: Run release-slice gates if the probe validates**

Run:

```powershell
cargo fmt -- --check
cargo test --test diagnostic_cartridge_tests
cargo test --test diagnostic_cli_tests
python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/ci-scenario-suite
python scripts/profile_diagnostic_cartridge.py --output-dir target/diagnostics/ci-profile --profile debug --samples 3 --warmups 1 --skip-build --max-regression-percent 50
```
