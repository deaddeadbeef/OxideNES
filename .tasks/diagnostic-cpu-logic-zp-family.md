# Diagnostic CPU Logic Zero-Page Family

## Goal

Extend the generated `cpu_alu_index_matrix` diagnostic cartridge with explicit
`ORA zp` and `EOR zp` logical ALU cases so AI/debug telemetry can distinguish
the full official zero-page logical accumulator family inside the broader CPU
ALU/index matrix.

## Acceptance

- [x] The generated diagnostic cartridge still runs headlessly to pass.
- [x] `cpu_alu_index_matrix` records six logic subcases and expected logic mask
  `0x3F`.
- [x] Telemetry retains `ora_zp_result=0x18` and `eor_zp_result=0x00` bytes.
- [x] The intentional ALU/index matrix fault still localizes to
  `cpu.alu_index.logic_flags`.
- [x] CLI scenario-suite and observability verifiers expect the new telemetry schema.
- [x] Focused cartridge tests, diagnostic CLI suite test, formatting, diff check,
  and local dev-build CI pass before PR.

## Evidence

- `cargo fmt -- --check`
- `git diff --check`
- `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass --target x86_64-pc-windows-msvc`
- `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_localizes_intentional_cpu_alu_index_matrix_failure --target x86_64-pc-windows-msvc`
- `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite --target x86_64-pc-windows-msvc`
- `python scripts/run_local_ci.py --output-dir target/local-ci/cpu-logic-zp-family-dev --suite-dir target/local-ci/cpu-logic-zp-family-dev/scenario-suite --target x86_64-pc-windows-msvc --build-profile debug`

Local CI evidence: `target/local-ci/cpu-logic-zp-family-dev/local-ci-report.md`
passed 13/13 commands against dirty precommit commit `4658af3`; the diagnostic
bundle reports schema `85`, suite `diagnostic-cartridge-v85`, logic/index masks
`0x3F/0x0F`, `ora_zp_result=0x18`, `eor_zp_result=0x00`, and ten ALU/index
cases.
