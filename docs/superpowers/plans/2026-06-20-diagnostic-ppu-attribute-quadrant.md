# PPU Attribute Quadrant Diagnostic Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an IP-safe diagnostic cartridge fixture that proves PPU attribute-table quadrant palette selection through host-observed frame samples and AI-readable failure routing.

**Architecture:** Extend the existing generated diagnostic cartridge next to `ppu_pixel_phase_signature`. The cartridge writes one nametable attribute byte with four quadrant palette selections, renders a deterministic background tile block, and host telemetry samples one pixel from each quadrant. The same observable is threaded through diagnostics, scenario generation, verifiers, AI route artifacts, tests, and docs.

**Tech Stack:** Rust diagnostic generator and telemetry, Python diagnostic artifact verifiers, Cargo tests, generated iNES cartridge.

---

### Task 1: Add The Cartridge Fixture And Host Telemetry

**Files:**
- Modify: `src/diagnostic.rs`
- Test: `tests/diagnostic_cartridge_tests.rs`

- [x] **Step 1: Define IDs and constants**

Add `PPU_ATTRIBUTE_QUADRANT_TEST_ID` after `PPU_PIXEL_PHASE_TEST_ID`, bump telemetry schema/version from `71` to `72`, add `PPU_ATTRIBUTE_QUADRANT_FAULT_LABEL`, reserve RAM addresses after `0x0307`, and define four sample coordinates/colors:

```rust
const PPU_ATTRIBUTE_QUADRANT_TEST_ID: u8 = 49;
const PPU_ATTRIBUTE_QUADRANT_CASE_COUNT_ADDR: u16 = 0x0308;
const PPU_ATTRIBUTE_QUADRANT_ATTR_BYTE_ADDR: u16 = 0x0309;
const PPU_ATTRIBUTE_QUADRANT_EXPECTED_CASE_COUNT: u8 = 4;
const PPU_ATTRIBUTE_QUADRANT_EXPECTED_ATTR_BYTE: u8 = 0xE4;
```

- [x] **Step 2: Generate the synthetic PPU program**

Add `ProgramBuilder::ppu_attribute_quadrant_signature()` after `ppu_pixel_phase_signature()` and call it in `build_program_with_labels()`. The program must clear VRAM, write palette entries `$3F01/$3F05/$3F09/$3F0D`, place solid tile `0x02` in a 4x4 tile area, write attribute byte `$E4` at `$23C0`, enable rendering, wait for rendered output, store the case count and attribute byte in diagnostic RAM, then pass test `49`.

- [x] **Step 3: Capture and validate frame samples**

Add `PpuAttributeQuadrantFrameSample`, `maybe_capture_ppu_attribute_quadrant_frame()`, and `ppu_attribute_quadrant_telemetry()` using samples from top-left, top-right, bottom-left, and bottom-right attribute quadrants. Host validation must produce a clear `PPU attribute-quadrant mismatch` failure when any sampled color or case count differs.

- [x] **Step 4: Add the fault route**

Add `DiagnosticFaultInjection::PpuAttributeQuadrant`, parse string `ppu_attribute_quadrant`, map it to the new label, and in `apply_diagnostic_fault_injection()` overwrite `$23C0` with `0x00` so three quadrant colors fail while the cartridge still reaches host validation.

### Task 2: Thread AI Observability Artifacts

**Files:**
- Modify: `src/diagnostic.rs`
- Modify: `src/bin/oxidenes-diagnostic.rs`
- Modify: `scripts/run_diagnostic_observability.py`
- Modify: `scripts/verify_diagnostic_suite.py`
- Modify: `scripts/verify_diagnostic_observability.py`
- Modify: `scripts/verify_diagnostic_ai_artifacts.py`
- Modify: `scripts/query_diagnostic_ai_index.py`
- Modify: `scripts/evaluate_diagnostic_ai_localization.py`
- Modify: `scripts/build_diagnostic_ai_session_plan.py`

- [x] **Step 1: Add telemetry probe and observable comparison path**

Add probe `ppu.attribute_quadrant.signature` with likely domain `ppu.attribute_quadrant`, plus comparison observables for the fixture's pass bit, case count, attribute byte, and four sampled colors.

- [x] **Step 2: Add the scenario**

Bump `DIAGNOSTIC_SCENARIO_SUITE_SCHEMA_VERSION` from `22` to `23` and add `ppu_attribute_quadrant_fault` after `ppu_pixel_phase_fault`, with focus domain `ppu.attribute_quadrant`.

- [x] **Step 3: Update AI route maps and expected counts**

Add `ppu.attribute_quadrant` to `FOCUS_DOMAIN_CODE_MAP`, add `ppu_attribute_quadrant_fault` to `SCENARIO_TEST_FILTERS`, and update expected counts from 44/36 to 45/37 where the full scenario suite and actionable AI routes are asserted.

### Task 3: Update Tests, Docs, And Release Metadata

**Files:**
- Modify: `tests/diagnostic_cartridge_tests.rs`
- Modify: `tests/diagnostic_cli_tests.rs`
- Modify: `docs/DIAGNOSTIC_CARTRIDGE.md`
- Modify: `docs/RELEASE_CANDIDATE_GATES.md`
- Modify: `CHANGELOG.md`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

- [x] **Step 1: Add focused Rust tests**

Assert the healthy telemetry includes test `49`, the new PPU probe, `ppu_attribute_quadrant.passed == true`, expected case count `4`, attribute byte `0xE4`, and four distinct expected host colors. Add an intentional-fault test proving focus domain `ppu.attribute_quadrant` and failed probe `ppu.attribute_quadrant.signature`.

- [x] **Step 2: Update CLI and artifact assertions**

Update diagnostic CLI tests for schema `72`, scenario-suite schema `23`, scenario count `45`, actionable route count `37`, test count `41`, and probe count after verifying the actual generated corpus.

- [x] **Step 3: Update docs and version metadata**

Document the new fixture in the diagnostic cartridge guide and release gates. If the slice validates and is release-worthy, bump crate version to `0.3.43` and add a changelog entry.

### Task 4: Verify Locally From The Dev Build

**Files:**
- Generated: `target/diagnostics/ci-scenario-suite`
- Generated: `target/diagnostics/ci-profile`

- [x] **Step 1: Run focused checks**

Run:

```powershell
cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_localizes_intentional_ppu_attribute_quadrant_failure
cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass
cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite
```

- [x] **Step 2: Run accepted diagnostic gates**

Run:

```powershell
python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/ci-scenario-suite
python scripts/profile_diagnostic_cartridge.py --output-dir target/diagnostics/ci-profile --profile debug --samples 3 --warmups 1 --skip-build --max-regression-percent 50
```

- [x] **Step 3: Run CI-equivalent Rust gates**

Run:

```powershell
cargo fmt -- --check
cargo test --target x86_64-pc-windows-msvc
cargo clippy -- -D warnings
```
