# CPU Load/Store STA Indexed-Indirect Diagnostic Plan

## Goal

Narrow the CPU opcode-matrix gap by extending the generated
`cpu_load_store_transfer_matrix` diagnostic with an official indexed-indirect
`STA (zp,X)` store case.

## Scope

- [x] Bump diagnostic telemetry schema to 78 and suite version to
  `diagnostic-cartridge-v78`.
- [x] Add an assembler helper for opcode `0x81` (`STA (zp,X)`).
- [x] Add an in-cartridge `STA (zp,X)` case that writes a sentinel through a
  zero-page pointer selected by `base + X` and records the sixth store subcase.
- [x] Update load/store matrix expected store mask to `0x3F` and expected case
  count to `14`.
- [x] Update diagnostic tests, CLI schema assertions, suite verifiers, failure
  descriptions, probe text, and diagnostic docs for the refreshed contract.

## Verification

- [x] `cargo fmt -- --check`
- [x] `git diff --check`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_localizes_intentional_cpu_load_store_matrix_failure --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite --target x86_64-pc-windows-msvc`
- [x] `python scripts/run_local_ci.py --output-dir target/local-ci/cpu-load-store-sta-indx-dev --suite-dir target/local-ci/cpu-load-store-sta-indx-dev/scenario-suite --target x86_64-pc-windows-msvc --build-profile debug`
