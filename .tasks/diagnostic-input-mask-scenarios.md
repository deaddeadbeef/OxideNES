# Diagnostic input-mask scenario matrix

## Goal

Expand the AI-facing diagnostic scenario suite so the generated cartridge runs
healthy input-port fixtures across more than one non-default controller mask.

## Changes

- Generate input-mask pass scenarios from a fixed table:
  - `input_mask_matrix_pass`: joypad 1 `0xAA`, joypad 2 `0x55`.
  - `input_mask_all_released_pass`: joypad 1 `0x00`, joypad 2 `0x00`.
  - `input_mask_all_pressed_pass`: joypad 1 `0xFF`, joypad 2 `0xFF`.
- Keep the negative/actionable fixture count at 23 while increasing the total
  scenario corpus to 27 and expected-pass fixtures to 4.
- Update suite, observability, localization, AI artifact, query-smoke, CLI, and
  documentation contracts so the new mask table is release-gated.
- Update the diagnostic coverage ledger to move generated input masks from a
  missing area into current coverage.

## Verification plan

- `cargo test generated_diagnostic_cartridge_runs_configured_input_mask_matrix_to_pass --test diagnostic_cartridge_tests -- --nocapture`
- `cargo test --test diagnostic_cli_tests -- --nocapture`
- `cargo test --test diagnostic_cartridge_tests -- --nocapture`
- `python scripts/run_diagnostic_e2e.py --suite-dir target\diagnostics\scenario-suite-input-mask-final`
- `python scripts/verify_diagnostic_suite.py --suite-dir target\diagnostics\scenario-suite-input-mask-final`
- `python scripts/verify_diagnostic_observability.py --suite-dir target\diagnostics\scenario-suite-input-mask-final`
- `python scripts/verify_diagnostic_ai_artifacts.py --suite-dir target\diagnostics\scenario-suite-input-mask-final --require-e2e-report --require-ai-route-matrix --require-ai-debug-packet --require-ai-debug-packet-matrix`
- `python scripts/profile_diagnostic_cartridge.py --output-dir target\diagnostics\diagnostic-profile-input-mask --samples 3 --warmups 1`
- `cargo fmt -- --check`
- `git diff --check`
- `cargo test`
- `cargo clippy -- -D warnings`
