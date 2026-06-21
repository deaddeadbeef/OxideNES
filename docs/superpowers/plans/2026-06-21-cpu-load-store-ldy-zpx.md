# CPU Load/Store LDY Zero-Page,X Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Narrow the CPU opcode-matrix gap by extending the generated `cpu_load_store_transfer_matrix` diagnostic with an official indexed zero-page `LDY zp,X` load case.

**Architecture:** Keep the existing generated cartridge and telemetry contract shape. Add one in-cartridge load subcase, retain the observed result in telemetry, bump the diagnostic telemetry schema, and update the verifier/test/docs expectations that consume the load mask and case count.

**Tech Stack:** Rust diagnostic cartridge generator, Rust integration tests, Python diagnostic artifact verifiers, local dev-build CI.

---

## File Structure

- Modify `src/diagnostic.rs`: schema constants, load mask/case-count expectations, failure/probe expected text, generated `LDY zp,X` case, telemetry result field, retained signal catalog, and assembler helper for opcode `0xB4`.
- Modify `tests/diagnostic_cartridge_tests.rs`: healthy and intentional-fault load/store matrix assertions.
- Modify `tests/diagnostic_cli_tests.rs`: telemetry schema and AI-ready scenario-suite assertions.
- Modify `scripts/verify_diagnostic_observability.py` and `scripts/verify_diagnostic_suite.py`: expected telemetry schema and load/store matrix text.
- Modify `docs/DIAGNOSTIC_CARTRIDGE.md`: schema 82 change note and load/store evidence text.
- Modify `docs/RELEASE_CANDIDATE_GATES.md`: release-gate evidence text for the refreshed load/store matrix.
- Modify `CHANGELOG.md`: unreleased slice note.
- Create `.tasks/diagnostic-cpu-load-store-ldy-zpx.md`: task acceptance note.

## Tasks

- [x] Bump diagnostic telemetry schema to 82 and suite version to `diagnostic-cartridge-v82`.
- [x] Add an assembler helper for opcode `0xB4` (`LDY zp,X`).
- [x] Add an in-cartridge `LDY zp,X` case that reads from zero page through an X-indexed effective address, records `load_y_zp_x_result=0x39`, and sets the seventh load subcase bit.
- [x] Update load/store matrix expected load mask to `0x7F` and expected case count to `18`.
- [x] Update diagnostic tests, CLI schema assertions, suite verifiers, failure descriptions, probe text, release-gate text, and diagnostic docs for the refreshed contract.

## Verification

- [x] `cargo fmt -- --check`
- [x] `git diff --check`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_localizes_intentional_cpu_load_store_matrix_failure --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite --target x86_64-pc-windows-msvc`
- [x] `python scripts/run_local_ci.py --output-dir target/local-ci/cpu-load-store-ldy-zpx-dev --suite-dir target/local-ci/cpu-load-store-ldy-zpx-dev/scenario-suite --target x86_64-pc-windows-msvc --build-profile debug`

## Evidence

- Dirty local CI report: `target/local-ci/cpu-load-store-ldy-zpx-dev/local-ci-report.md`
- Dirty local CI status: passed, 13 commands, pre-commit worktree.
- Diagnostic baseline: schema `82`, suite `diagnostic-cartridge-v82`, dev build `0.3.48-dev`, load/store cases `18/18`, load mask `0x7F`, store mask `0x7F`, `load_indirect_y_result=0x91`, `load_x_zp_y_result=0x42`, `load_y_zp_x_result=0x39`.
- Diagnostic E2E: 45 scenarios, debug packet matrix `37/37`, localization `45/45`, coverage plan `only_happy_paths=false`, ready gaps `6/6`.
