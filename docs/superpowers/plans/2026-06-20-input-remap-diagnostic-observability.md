# Input Remap Diagnostic Observability Plan

## Goal

Reduce the `input_port_matrix` coverage gap by making host input-remapping
serialization testable outside the live window/controller loop while keeping the
game loop on the same code path.

## Scope

- [x] Add a library input-mapping module that resolves configured keyboard and
  controller bindings into NES joypad bit order.
- [x] Route the game loop through the shared input-mapping helpers before
  applying button state to joypad 1 and joypad 2.
- [x] Add regression tests for custom keyboard bindings, custom controller face
  button bindings, turbo gates, merged input sources, and joypad serial bit
  order.
- [x] Add bus-level default/disconnected controller coverage for both joypad
  ports, including released serial bits and post-exhaustion high reads.
- [x] Add a headless host-event snapshot fixture that injects keyboard,
  controller button, D-pad, and stick-derived events, applies them to a real
  `Bus`, and validates `$4016`/`$4017` serial masks.
- [x] Update the diagnostic coverage gap text so host input-remapping fixtures
  disconnected/default port behavior, and headless host-event snapshots are
  covered and the remaining gap is OS-backed event injection and broader
  in-cartridge iteration.
- [x] Document the release-candidate evidence requirement.

## Verification

- [x] `cargo fmt -- --check`
- [x] `cargo test --test input_mapping_tests --target x86_64-pc-windows-msvc`
- [x] `cargo test --test input_mapping_tests host_input_snapshot_serializes_injected_events_through_bus_ports --target x86_64-pc-windows-msvc`
- [x] `cargo test --test bus_tests bus_default_joypads_read_released_then_overread_high --target x86_64-pc-windows-msvc`
- [x] `cargo test --lib diagnostic::tests::headless_diagnostic_passes_and_collects_telemetry --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_standalone_triage_json --target x86_64-pc-windows-msvc`
- [x] `python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/input-remap-observability --skip-replay`
- [x] `python scripts/verify_diagnostic_observability.py --suite-dir target/diagnostics/input-remap-observability`
- [x] `python scripts/verify_diagnostic_suite.py --suite-dir target/diagnostics/input-remap-observability`
- [x] `cargo clippy -- -D warnings`
