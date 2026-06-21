# CPU Load/Store LDA Indirect-Indexed Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Narrow the CPU opcode-matrix gap by extending the generated `cpu_load_store_transfer_matrix` diagnostic with an official indirect-indexed `LDA (zp),Y` load case.

**Architecture:** Keep the existing generated cartridge and telemetry contract shape. Add one in-cartridge load subcase, bump the diagnostic telemetry schema, and update the verifier/test/docs expectations that consume the load mask and case count.

**Tech Stack:** Rust diagnostic cartridge generator, Rust integration tests, Python diagnostic artifact verifiers, local dev-build CI.

---

## File Structure

- Modify `src/diagnostic.rs`: schema constants, load mask/case-count expectations, failure/probe expected text, generated `LDA (zp),Y` case.
- Modify `tests/diagnostic_cartridge_tests.rs`: healthy and intentional-fault load/store matrix assertions.
- Modify `tests/diagnostic_cli_tests.rs`: telemetry schema assertions in generated CLI artifacts.
- Modify `scripts/verify_diagnostic_observability.py` and `scripts/verify_diagnostic_suite.py`: expected telemetry schema.
- Modify `docs/DIAGNOSTIC_CARTRIDGE.md`: schema 80 change note.
- Modify `docs/RELEASE_CANDIDATE_GATES.md`: release-gate evidence text for the refreshed load/store matrix.

## Tasks

- [x] Bump diagnostic telemetry schema to 80 and suite version to `diagnostic-cartridge-v80`.
- [x] Add an in-cartridge `LDA (zp),Y` case that reads through a zero-page pointer plus Y offset and records the fifth load subcase.
- [x] Update load/store matrix expected load mask to `0x1F` and expected case count to `16`.
- [x] Update diagnostic tests, CLI schema assertions, suite verifiers, failure descriptions, probe text, release-gate text, and diagnostic docs for the refreshed contract.

## Verification

- [x] `cargo fmt -- --check`
- [x] `git diff --check`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_runs_headlessly_to_pass --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cartridge_tests generated_diagnostic_cartridge_localizes_intentional_cpu_load_store_matrix_failure --target x86_64-pc-windows-msvc`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_ai_ready_scenario_suite --target x86_64-pc-windows-msvc`
- [x] `python scripts/run_local_ci.py --output-dir target/local-ci/cpu-load-store-lda-indy-dev --suite-dir target/local-ci/cpu-load-store-lda-indy-dev/scenario-suite --target x86_64-pc-windows-msvc --build-profile debug`

## Evidence

- Local CI report: `target/local-ci/cpu-load-store-lda-indy-dev/local-ci-report.md`
- Local CI status: passed, 13 commands, dirty pre-commit worktree.
- Diagnostic baseline: schema `80`, suite `diagnostic-cartridge-v80`, dev build `0.3.47-dev`, load/store cases `16/16`, load mask `0x1F`, store mask `0x7F`, `load_indirect_y_result=0x91`.
- Diagnostic E2E: 45 scenarios, AI route matrix `37/37`, debug packet matrix `37/37`, localization `45/45`, readiness `37/37`.
