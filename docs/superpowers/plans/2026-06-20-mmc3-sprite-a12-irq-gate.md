# MMC3 Sprite A12 IRQ Gate Diagnostic Implementation Plan

## Goal

Add a host-observed generated Mapper 4/MMC3 diagnostic variant that proves the
current scanline-level A12 IRQ gate handles sprite pattern-table source
selection, complementing the existing background A12 gate diagnostic.

## Scope

- [x] Bump diagnostic telemetry schema to 75 and suite version to
  `diagnostic-cartridge-v75`.
- [x] Add `Mapper4Mmc3SpriteA12GateTelemetry` with low-sprite IRQ count,
  high-sprite IRQ count, expected counts, cycles, frames, pass bit, and error.
- [x] Generate a Mapper 4 cartridge variant that enables sprites with
  `PPUCTRL=0x00`/`PPUMASK=0x10` and expects zero IRQs, then repeats with
  `PPUCTRL=0x08`/`PPUMASK=0x10` and expects one IRQ.
- [x] Thread the telemetry through `run_diagnostic`, host validation, report
  rows, and probe telemetry as `mapper4.mmc3_sprite_a12_irq_gate`.
- [x] Update diagnostic suite and observability verifiers for schema 75 and 92
  telemetry probes.
- [x] Document the schema 75 coverage and release-candidate evidence.

## Verification

- [x] `cargo fmt -- --check`
- [x] `cargo check --bin oxidenes-diagnostic`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite`
- [x] `python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/mmc3-sprite-a12-observability --skip-replay`
- [x] `python scripts/verify_diagnostic_observability.py --suite-dir target/diagnostics/mmc3-sprite-a12-observability`
- [x] `python scripts/verify_diagnostic_suite.py --suite-dir target/diagnostics/mmc3-sprite-a12-observability`
- [x] `python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/mmc3-sprite-a12-e2e`
