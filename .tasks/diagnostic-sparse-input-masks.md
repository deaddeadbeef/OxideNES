# Diagnostic sparse input-mask fixtures

## Goal

Extend the AI-facing diagnostic scenario suite beyond all-pressed/all-released
and extreme asymmetric controller masks with representative mixed-bit per-port
fixtures.

## Changes

- Add two generated pass scenarios:
  - `input_mask_sparse_bits_pass`: joypad 1 `0x81`, joypad 2 `0x18`.
  - `input_mask_nibble_split_pass`: joypad 1 `0x0F`, joypad 2 `0xF0`.
- Keep the intentional negative/actionable fixture count at 23 while increasing
  the total scenario corpus to 31 and expected-pass fixtures to 8.
- Update suite, observability, localization, AI artifact, query-smoke, CLI, and
  documentation contracts so the sparse/dense mask fixtures are release-gated.
- Narrow the remaining input-port coverage gap to broader exhaustive per-port
  sweeps, disconnected-controller electrical defaults, and host remapping.

## Verification plan

- `cargo test generated_diagnostic_cartridge_runs_configured_input_mask_matrix_to_pass --test diagnostic_cartridge_tests -- --nocapture`
- `cargo test --test diagnostic_cli_tests -- --nocapture`
- `cargo test --test diagnostic_cartridge_tests -- --nocapture`
- `python scripts/run_diagnostic_e2e.py --suite-dir target\diagnostics\scenario-suite-sparse-input-final`
- `python scripts/verify_diagnostic_suite.py --suite-dir target\diagnostics\scenario-suite-sparse-input-final`
- `python scripts/verify_diagnostic_observability.py --suite-dir target\diagnostics\scenario-suite-sparse-input-final`
- `python scripts/verify_diagnostic_ai_artifacts.py --suite-dir target\diagnostics\scenario-suite-sparse-input-final --require-e2e-report --require-ai-route-matrix --require-ai-debug-packet --require-ai-debug-packet-matrix`
- `python scripts/profile_diagnostic_cartridge.py --output-dir target\diagnostics\diagnostic-profile-sparse-input --samples 3 --warmups 1`
- `cargo fmt -- --check`
- `git diff --check`
- `cargo test`
- `cargo clippy -- -D warnings`
