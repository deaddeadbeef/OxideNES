# CPU Load/Store LDX Zero-Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Narrow the CPU opcode-matrix gap by extending the generated `cpu_load_store_transfer_matrix` diagnostic with an official `LDX zp` load case.

**Architecture:** Keep the existing generated cartridge and telemetry contract shape. Add one in-cartridge load subcase, retain the observed result in telemetry, bump the diagnostic telemetry schema, and update the verifier/test/docs expectations that consume the load mask and case count.

**Tech Stack:** Rust diagnostic cartridge generator, Rust integration tests, Python diagnostic artifact verifiers, local dev-build CI.

---

## File Structure

- Modify `src/diagnostic.rs`: schema constants, load mask/case-count expectations, failure/probe expected text, generated `LDX zp` case, telemetry result field, retained signal catalog, and assembler helper for opcode `0xA6`.
- Modify `tests/diagnostic_cartridge_tests.rs`: healthy and intentional-fault load/store matrix assertions.
- Modify `tests/diagnostic_cli_tests.rs`: telemetry schema assertions in generated CLI artifacts.
- Modify `scripts/verify_diagnostic_observability.py` and `scripts/verify_diagnostic_suite.py`: expected telemetry schema.
- Modify `docs/DIAGNOSTIC_CARTRIDGE.md`: schema 83 change note.
- Modify `docs/RELEASE_CANDIDATE_GATES.md`: release-gate evidence text for the refreshed load/store matrix.
- Modify `CHANGELOG.md`: unreleased slice note.
- Create `.tasks/diagnostic-cpu-load-store-ldx-zp.md`: task acceptance note.

## Tasks

- [x] Bump diagnostic telemetry schema to 83 and suite version to `diagnostic-cartridge-v83`.
- [x] Add an assembler helper for opcode `0xA6` (`LDX zp`).
- [x] Add an in-cartridge `LDX zp` case that reads from zero page, records `load_x_zp_result=0x24`, and sets the eighth load subcase bit.
- [x] Update load/store matrix expected load mask to `0xFF` and expected case count to `19`.
- [x] Update diagnostic tests, CLI schema assertions, suite verifiers, failure descriptions, probe text, release-gate text, and diagnostic docs for the refreshed contract.

## Verification

- [x] `cargo fmt -- --check`
- [x] `git diff --check`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_localizes_intentional_cpu_load_store_matrix_failure --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite --target x86_64-pc-windows-msvc`
- [x] `python scripts/run_local_ci.py --output-dir target/local-ci/cpu-load-store-ldx-zp-dev --suite-dir target/local-ci/cpu-load-store-ldx-zp-dev/scenario-suite --target x86_64-pc-windows-msvc --build-profile debug`

## Evidence

- Focused pass-path cartridge test passed.
- Focused intentional load/store matrix fault localization test passed.
- Focused diagnostic CLI scenario-suite test passed.
- Local dev-build CI passed 13/13 commands: `target/local-ci/cpu-load-store-ldx-zp-dev/local-ci-report.md`.
- Diagnostic baseline: schema `83`, suite `diagnostic-cartridge-v83`, load/store cases `19/19`, load mask `0xFF`, store mask `0x7F`, `load_x_zp_result=0x24`.
