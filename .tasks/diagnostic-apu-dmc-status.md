# Diagnostic APU DMC Status

## Goal

Deepen the diagnostic cartridge's APU/DMA coverage by turning the existing DMC sample-DMA setup into explicit `$4015` DMC active-bit evidence, then expose the result through host telemetry, probes, report text, tests, and verifiers.

## Acceptance

- The diagnostic cartridge still runs headlessly to pass.
- Telemetry records expected and observed DMC `$4015` active-bit evidence.
- The report and probe catalog expose the DMC status evidence separately from the non-DMC status matrix.
- Focused and full Rust tests pass.
- Diagnostic e2e, AI artifact, observability, and profile checks pass before PR.

## Evidence

- Baseline focused cartridge pass succeeded before edits.
- Focused `generated_diagnostic_cartridge_runs_headlessly_to_pass` passed after adding DMC status telemetry and phase realignment.
- `cargo test --test diagnostic_cartridge_tests -- --nocapture` passed: 31/31 tests.
- `cargo test --test diagnostic_cli_tests -- --nocapture` passed: 8/8 tests.
- `python scripts/run_diagnostic_e2e.py --suite-dir target\diagnostics\scenario-suite-v47-final` passed: 25 scenarios, 23/23 routes ready, AI artifacts passed.
- `python scripts/verify_diagnostic_suite.py --suite-dir target\diagnostics\scenario-suite-v47-final` passed.
- `python scripts/verify_diagnostic_observability.py --suite-dir target\diagnostics\scenario-suite-v47-final` passed with telemetry catalog `53:9`.
- `python scripts/verify_diagnostic_ai_artifacts.py --suite-dir target\diagnostics\scenario-suite-v47-final --require-e2e-report --require-ai-route-matrix --require-ai-debug-packet --require-ai-debug-packet-matrix` passed: 101/101 checks.
- `python scripts/profile_diagnostic_cartridge.py --output-dir target\diagnostics\diagnostic-profile-v47 --samples 3 --warmups 1` passed.
- `cargo fmt -- --check`, `git diff --check`, `cargo test`, and `cargo clippy -- -D warnings` passed.
