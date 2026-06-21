# Diagnostic CPU Load/Store LDX Zero-Page,Y

## Goal

Extend the generated `cpu_load_store_transfer_matrix` diagnostic cartridge with
an explicit `LDX zp,Y` indexed zero-page load case so AI/debug telemetry can
distinguish LDX zero-page indexed-addressing regressions inside the broader CPU
load/store/transfer family.

## Acceptance

- The generated diagnostic cartridge still runs headlessly to pass.
- `cpu_load_store_matrix` records a sixth load subcase and expected load mask
  `0x3F`.
- Telemetry retains a `load_x_zp_y_result` byte with expected value `0x42`.
- The intentional load/store matrix fault still localizes to
  `cpu.load_store.transfer_matrix`.
- CLI scenario-suite and observability verifiers expect the new telemetry schema.
- Focused cartridge tests, diagnostic CLI suite test, formatting, diff check,
  and local dev-build CI pass before PR.
