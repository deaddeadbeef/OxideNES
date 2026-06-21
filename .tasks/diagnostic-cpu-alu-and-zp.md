# Diagnostic CPU ALU AND Zero-Page

## Goal

Extend the generated `cpu_alu_index_matrix` diagnostic cartridge with an
explicit `AND zp` logical ALU case so AI/debug telemetry can distinguish basic
zero-page logical addressing regressions inside the broader CPU ALU/index
family.

## Acceptance

- [x] The generated diagnostic cartridge still runs headlessly to pass.
- [x] `cpu_alu_index_matrix` records a fourth logic subcase and expected logic mask
  `0x0F`.
- [x] Telemetry retains an `and_zp_result` byte with expected value `0x88`.
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
- `python scripts/run_local_ci.py --output-dir target/local-ci/cpu-alu-and-zp-dev-rerun --suite-dir target/local-ci/cpu-alu-and-zp-dev-rerun/scenario-suite --target x86_64-pc-windows-msvc --build-profile debug`

Local CI evidence: `target/local-ci/cpu-alu-and-zp-dev-rerun/local-ci-report.md`
passed 13/13 commands against dirty precommit commit `0fdcec9`; the diagnostic
bundle reports schema `84`, suite `diagnostic-cartridge-v84`, logic/index masks
`0x0F/0x0F`, `and_zp_result=0x88`, and eight ALU/index cases.
