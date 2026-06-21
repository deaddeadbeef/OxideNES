# CPU ALU AND Zero-Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Narrow the CPU opcode-matrix gap by extending the generated `cpu_alu_index_matrix` diagnostic with an official `AND zp` logical case.

**Architecture:** Keep the existing generated cartridge and telemetry contract shape. Add one in-cartridge logical ALU subcase, retain the observed zero-page result in telemetry, bump the diagnostic telemetry schema, and update verifier/test/docs expectations that consume the logic mask and case count.

**Tech Stack:** Rust diagnostic cartridge generator, Rust integration tests, Python diagnostic artifact verifiers, local dev-build CI.

---

## File Structure

- Modify `src/diagnostic.rs`: schema constants, ALU/index mask/case-count expectations, failure/probe text, generated `AND zp` case, telemetry result field, retained signal catalog, and assembler helper for opcode `0x25`.
- Modify `tests/diagnostic_cartridge_tests.rs`: healthy and intentional-fault ALU/index matrix assertions.
- Modify `tests/diagnostic_cli_tests.rs`: telemetry schema assertions in generated CLI artifacts.
- Modify `scripts/verify_diagnostic_observability.py` and `scripts/verify_diagnostic_suite.py`: expected telemetry schema.
- Modify `docs/DIAGNOSTIC_CARTRIDGE.md`: schema 84 change note.
- Modify `docs/RELEASE_CANDIDATE_GATES.md`: release-gate evidence text for the refreshed ALU/index matrix.
- Modify `CHANGELOG.md`: unreleased slice note.
- Create `.tasks/diagnostic-cpu-alu-and-zp.md`: task acceptance note.

## Tasks

- [x] Bump diagnostic telemetry schema to 84 and suite version to `diagnostic-cartridge-v84`.
- [x] Add an assembler helper for opcode `0x25` (`AND zp`).
- [x] Add an in-cartridge `AND zp` case that reads from zero page, records `and_zp_result=0x88`, and sets the fourth logic subcase bit.
- [x] Update ALU/index matrix expected logic mask to `0x0F` and expected case count to `8`.
- [x] Update diagnostic tests, CLI schema assertions, suite verifiers, failure descriptions, probe text, release-gate text, and diagnostic docs for the refreshed contract.

## Verification

- [x] `cargo fmt -- --check`
- [x] `git diff --check`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_localizes_intentional_cpu_alu_index_matrix_failure --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite --target x86_64-pc-windows-msvc`
- [x] `python scripts/run_local_ci.py --output-dir target/local-ci/cpu-alu-and-zp-dev-rerun --suite-dir target/local-ci/cpu-alu-and-zp-dev-rerun/scenario-suite --target x86_64-pc-windows-msvc --build-profile debug`

## Evidence

- Focused healthy cartridge test passed with telemetry schema `84`, suite `diagnostic-cartridge-v84`, logic mask `0x0F`, index mask `0x0F`, `and_zp_result=0x88`, and eight ALU/index cases.
- Focused intentional `cpu_alu_index_matrix_fault` test passed and still localized to `cpu.alu_index.logic_flags`.
- Local dev-build CI passed 13/13 commands in `target/local-ci/cpu-alu-and-zp-dev-rerun/local-ci-report.md` with schema `84`, suite `diagnostic-cartridge-v84`, logic/index masks `0x0F/0x0F`, `and_zp_result=0x88`, and eight ALU/index cases.
