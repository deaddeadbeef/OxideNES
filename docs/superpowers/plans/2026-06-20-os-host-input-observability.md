# OS Host Input Observability Plan

## Goal

Narrow the `input_port_matrix` gap by proving the live game-loop input path can
consume OS-typed keyboard and controller events through shared testable library
helpers before serializing controller state through `$4016` and `$4017`.

## Scope

- [x] Move string-to-`minifb::Key` and string-to-`gilrs::Button` binding
  resolution into `src/input_mapping.rs`.
- [x] Add OS-typed input snapshots for headless tests using real `minifb::Key`
  and `gilrs::Button` values.
- [x] Route the game loop through the OS-typed input-mapping helpers instead of
  private duplicate conversion functions.
- [x] Add a bus-level regression proving OS-typed keyboard, controller button,
  D-pad, and stick-derived events serialize through `$4016` and `$4017`.
- [x] Update diagnostic coverage-gap text so the remaining gap is live window
  and device polling, not typed adapter mapping.

## Verification

- [x] `cargo fmt -- --check`
- [x] `cargo test --test input_mapping_tests --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_standalone_triage_json --target x86_64-pc-windows-msvc`
- [x] `cargo test --lib diagnostic::tests::headless_diagnostic_passes_and_collects_telemetry --target x86_64-pc-windows-msvc`
- [x] `cargo test --target x86_64-pc-windows-msvc`
- [x] `cargo clippy -- -D warnings`
- [x] `python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/os-host-input-observability --skip-replay`
- [x] `python scripts/verify_diagnostic_observability.py --suite-dir target/diagnostics/os-host-input-observability`
- [x] `python scripts/verify_diagnostic_suite.py --suite-dir target/diagnostics/os-host-input-observability`
