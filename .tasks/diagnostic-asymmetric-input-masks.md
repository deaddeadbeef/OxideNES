# Diagnostic asymmetric input-mask fixtures

## Goal

Extend the AI-facing diagnostic scenario suite so `$4016` and `$4017` can be
validated independently by generated healthy input-port fixtures.

## Changes

- Add two generated pass scenarios:
  - `input_mask_joypad1_pressed_pass`: joypad 1 `0xFF`, joypad 2 `0x00`.
  - `input_mask_joypad2_pressed_pass`: joypad 1 `0x00`, joypad 2 `0xFF`.
- Keep the intentional negative/actionable fixture count at 23 while increasing
  the total scenario corpus to 29 and expected-pass fixtures to 6.
- Update suite, observability, localization, AI artifact, query-smoke, CLI, and
  documentation contracts so the asymmetric mask fixtures are release-gated.
- Update the diagnostic coverage ledger so the remaining input-port gap is
  mixed-bit per-port sweeps beyond these extreme asymmetric cases.

## Verification plan

- `cargo test generated_diagnostic_cartridge_runs_configured_input_mask_matrix_to_pass --test diagnostic_cartridge_tests -- --nocapture`
- `cargo test --test diagnostic_cli_tests -- --nocapture`
- `cargo test --test diagnostic_cartridge_tests -- --nocapture`
- `python scripts/run_diagnostic_e2e.py --suite-dir target\diagnostics\scenario-suite-asymmetric-input-final`
- `python scripts/verify_diagnostic_suite.py --suite-dir target\diagnostics\scenario-suite-asymmetric-input-final`
- `python scripts/verify_diagnostic_observability.py --suite-dir target\diagnostics\scenario-suite-asymmetric-input-final`
- `python scripts/verify_diagnostic_ai_artifacts.py --suite-dir target\diagnostics\scenario-suite-asymmetric-input-final --require-e2e-report --require-ai-route-matrix --require-ai-debug-packet --require-ai-debug-packet-matrix`
- `python scripts/profile_diagnostic_cartridge.py --output-dir target\diagnostics\diagnostic-profile-asymmetric-input --samples 3 --warmups 1`
- `cargo fmt -- --check`
- `git diff --check`
- `cargo test`
- `cargo clippy -- -D warnings`
