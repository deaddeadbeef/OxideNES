# Diagnostic APU Status Matrix

## Goal

Deepen the diagnostic cartridge's APU coverage by turning the current narrow `$4015` status check into a multi-channel status matrix, then expose the result through host telemetry, probes, report text, tests, and verifiers.

## Acceptance

- The diagnostic cartridge still runs headlessly to pass.
- Telemetry records expected and observed APU status-matrix channel bits.
- The report and probe catalog expose the APU status-matrix evidence.
- Focused and full Rust tests pass.
- Diagnostic e2e, AI artifact, observability, and profile checks pass before PR.

## Evidence

- Focused pass and intentional APU-status fault tests pass.
- Diagnostic CLI scenario suite passes with telemetry schema 46.
- Diagnostic e2e artifact builder passes with 25 scenarios and 23 ready AI routes.
- Scenario-suite, observability, and AI artifact verifiers pass.
- `cargo fmt -- --check`, `git diff --check`, `cargo test`, `cargo clippy -- -D warnings`, and the diagnostic profile smoke pass.