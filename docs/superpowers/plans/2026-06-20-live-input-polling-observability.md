# Live Input Polling Observability Plan

## Goal

Narrow the `input_port_matrix` gap by making the live gilrs controller polling
path testable outside a physical device session while keeping the game loop on
the shared library path.

## Scope

- [x] Move analog stick circular deadzone, cardinal snapping, diagonal gating,
  SOCD cleanup, and hysteresis into `src/input_mapping.rs`.
- [x] Route menu and gameplay analog-stick handling through the shared helper.
- [x] Add a gilrs polling helper that consumes a real `gilrs::Button` polling
  closure plus live stick axes and returns controller state, trigger flags, and
  intermediate D-pad/stick evidence.
- [x] Route gameplay controller polling through the helper.
- [x] Add bus-level regression coverage for polled D-pad buttons, analog stick
  axes, turbo gates, trigger flags, and `$4016` serialization.
- [x] Update diagnostic gap text so the remaining gap is actual minifb window
  and physical/virtual gilrs device polling, not the closure-driven polling
  logic.

## Verification

- [x] `cargo fmt -- --check`
- [x] `git diff --check`
- [x] `cargo test --test input_mapping_tests --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_standalone_triage_json --target x86_64-pc-windows-msvc`
- [x] `cargo test --lib diagnostic::tests::headless_diagnostic_passes_and_collects_telemetry --target x86_64-pc-windows-msvc`
- [x] `cargo test --target x86_64-pc-windows-msvc`
- [x] `cargo clippy -- -D warnings`
- [x] `python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/live-input-polling-observability --skip-replay`
- [x] `python scripts/verify_diagnostic_observability.py --suite-dir target/diagnostics/live-input-polling-observability`
- [x] `python scripts/verify_diagnostic_suite.py --suite-dir target/diagnostics/live-input-polling-observability`
