# CPU Load/Store STA Indirect-Indexed Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Narrow the CPU opcode-matrix gap by extending the generated `cpu_load_store_transfer_matrix` diagnostic with an official indirect-indexed `STA (zp),Y` store case.

**Architecture:** Keep the existing generated cartridge and telemetry contract shape. Add one in-cartridge store subcase, bump the diagnostic telemetry schema, and update the verifier/test/docs expectations that consume the store mask and case count.

**Tech Stack:** Rust diagnostic cartridge generator, Rust integration tests, Python diagnostic artifact verifiers, local dev-build CI.

---

## File Structure

- Modify `src/diagnostic.rs`: schema constants, store mask/case-count expectations, failure/probe expected text, generated `STA (zp),Y` case, and assembler helper for opcode `0x91`.
- Modify `tests/diagnostic_cartridge_tests.rs`: healthy and intentional-fault load/store matrix assertions.
- Modify `tests/diagnostic_cli_tests.rs`: telemetry schema assertions in generated CLI artifacts.
- Modify `scripts/verify_diagnostic_observability.py` and `scripts/verify_diagnostic_suite.py`: expected telemetry schema.
- Modify `docs/DIAGNOSTIC_CARTRIDGE.md`: schema 79 change note.

## Tasks

- [x] Bump diagnostic telemetry schema to 79 and suite version to `diagnostic-cartridge-v79`.
- [x] Add an assembler helper for opcode `0x91` (`STA (zp),Y`).
- [x] Add an in-cartridge `STA (zp),Y` case that writes through a zero-page pointer plus Y offset and records the seventh store subcase.
- [x] Update load/store matrix expected store mask to `0x7F` and expected case count to `15`.
- [x] Update diagnostic tests, CLI schema assertions, suite verifiers, failure descriptions, probe text, and diagnostic docs for the refreshed contract.

## Verification

- [x] `cargo fmt -- --check`
- [x] `git diff --check`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_localizes_intentional_cpu_load_store_matrix_failure --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite --target x86_64-pc-windows-msvc`
- [x] `python scripts/run_local_ci.py --output-dir target/local-ci/cpu-load-store-sta-indy-dev --suite-dir target/local-ci/cpu-load-store-sta-indy-dev/scenario-suite --target x86_64-pc-windows-msvc --build-profile debug`
