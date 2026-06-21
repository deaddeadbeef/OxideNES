# Diagnostic CPU Load/Store LDX Zero-Page

## Goal

Extend the generated `cpu_load_store_transfer_matrix` diagnostic cartridge with
an explicit `LDX zp` load case so AI/debug telemetry can distinguish basic LDX
zero-page addressing regressions inside the broader CPU load/store/transfer
family.

## Acceptance

- [x] The generated diagnostic cartridge still runs headlessly to pass.
- [x] `cpu_load_store_matrix` records an eighth load subcase and expected load mask
  `0xFF`.
- [x] Telemetry retains a `load_x_zp_result` byte with expected value `0x24`.
- [x] The intentional load/store matrix fault still localizes to
  `cpu.load_store.transfer_matrix`.
- [x] CLI scenario-suite and observability verifiers expect the new telemetry schema.
- [x] Focused cartridge tests, diagnostic CLI suite test, formatting, diff check,
  and local dev-build CI pass before PR.

## Evidence

- Local dev-build CI passed 13/13 commands at
  `target/local-ci/cpu-load-store-ldx-zp-dev/local-ci-report.md`.
- Baseline telemetry reports schema `83`, suite `diagnostic-cartridge-v83`,
  `cpu_load_store_matrix.load_mask_hex=0xFF`,
  `observed_case_count=19`, `expected_case_count=19`, and
  `load_x_zp_result_hex=0x24`.
