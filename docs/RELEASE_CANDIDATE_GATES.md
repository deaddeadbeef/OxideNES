# Release Candidate Gates

OxideNES can publish a 1.0 release candidate only when every required gate below passes and the evidence is recorded in a dated acceptance file. The M4 local evidence run is recorded in `docs/RELEASE_CANDIDATE_EVIDENCE_2026-05-23.md`.

## Required Gates

### Source And CI

- `cargo fmt -- --check`
- `cargo check`
- `cargo test`
- `cargo clippy -- -D warnings`
- `git diff --check`
- `cargo audit`
- The diagnostic cartridge bundle commands pass and write an AI-ready bundle:
  `cargo run --bin oxidenes-diagnostic -- --json target/diagnostics/release-baseline.json --no-stdout`
  then
  `cargo run --bin oxidenes-diagnostic -- --bundle-dir target/diagnostics/latest-bundle --baseline-json target/diagnostics/release-baseline.json --no-stdout`
- The diagnostic scenario suite command passes and writes an AI-ready corpus:
  `cargo run --bin oxidenes-diagnostic -- --scenario-suite-dir target/diagnostics/scenario-suite --no-stdout`
- The diagnostic scenario suite verifier passes:
  `python scripts/verify_diagnostic_suite.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic observability runner passes and writes `observability-run.json`, `observability-run.md`, `diagnostic-debug-index.jsonl`, `diagnostic-debug-index.md`, `diagnostic-observability-analysis.json`, `diagnostic-observability-analysis.md`, `diagnostic-coverage-ledger.json`, `diagnostic-coverage-ledger.md`, `diagnostic-telemetry-catalog.json`, `diagnostic-telemetry-catalog.md`, `diagnostic-code-map.json`, `diagnostic-code-map.md`, `diagnostic-investigation-plan.json`, `diagnostic-investigation-plan.md`, `diagnostic-scenario-dossiers.json`, `diagnostic-scenario-dossiers.md`, and focused `replay-runs/<scenario>/` evidence:
  `python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic observability verifier passes and contract-checks the AI-facing wrapper artifacts:
  `python scripts/verify_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic e2e runner passes and writes the root acceptance report, AI observability index, AI query smoke, and AI diagnosis smoke after observability, route matrix, top-route replay, and route evidence verification:
  `python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI diagnosis runner passes and writes an executable route handoff with fresh replay evidence and mapped narrow-test results:
  `python scripts/run_diagnostic_ai_diagnosis.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic route matrix passes for every investigation route and writes `route-replay-matrix/diagnostic-route-matrix.json`, `diagnostic-route-matrix.md`, per-route `diagnostic-route-check.json`, and focused replay-bundle evidence:
  `python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --all-routes --skip-tests --output-dir target/diagnostics/scenario-suite/route-replay-matrix`
- The diagnostic route-check runner passes for the top investigation route and writes `route-checks/<route>/diagnostic-route-check.json`, `diagnostic-route-check.md`, and focused replay-bundle evidence:
  `python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic route evidence verifier passes and contract-checks both the all-route replay matrix and top-route full check, then writes `diagnostic-route-evidence-verification.json` and `diagnostic-route-evidence-verification.md`:
  `python scripts/verify_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --require-matrix --require-top-route --expect-matrix-tests-skipped --write-summary`
- When a prior suite is available, the diagnostic observability comparison passes without regressions:
  `python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite --compare-suite-dir target/diagnostics/prior-scenario-suite --fail-on-comparison-regression`
- CI and release diagnostic-bundle jobs generate the prior suite from the pull request base SHA or previous push SHA when available, run the comparison gate, and upload `oxidenes-diagnostic-prior-scenario-suite` with the current `oxidenes-diagnostic-scenario-suite`.
- GitHub Actions CI is green on Windows, Linux, macOS, rustfmt, clippy, security audit, IP compliance, and diagnostic bundle jobs.

### Compatibility And Safety

- CPU fetch/decode execution is covered by generated in-memory test ROM content.
- The generated diagnostic cartridge bundle contains `manifest.json`, `triage.json`, `telemetry.json`, `report.md`, and `diagnostic.nes`; when a baseline is supplied it also contains `comparison.json` and `comparison.md`.
- `triage.json` and `telemetry.json` include explicit known coverage gaps so release evidence does not overstate cartridge coverage.
- `triage.json` and `telemetry.json` include a derived debug focus with the focus test/subsystem/domain, failed probe ids, final event, terminal instruction, and last focus-test instruction for AI-first triage.
- Diagnostic event tails include CPU register snapshots and diagnostic RAM watchpoints so failed headless runs preserve immediate execution-state context for automated triage.
- Diagnostic bundles include a bounded decoded instruction trace tail with diagnostic cartridge symbols so failed headless runs preserve the final opcode-boundary CPU/RAM context for automated triage.
- Diagnostic scenario suites include the pass baseline, a non-default input-mask pass fixture, intentional joypad, CPU, PPU, mapper, DMA, APU, and timeout fixtures with expected-vs-actual debug-focus contracts, per-clause contract-match breakdowns, root baseline-comparison summaries, single-scenario replay args, a suite-level attention queue, plus `scenario-suite.json`, `scenario-suite.md`, `scenario-suite-observer.json`, and `scenario-suite-observer.md` indexes for automated regression triage.
- Diagnostic observability runs add `diagnostic-debug-index.jsonl` and `diagnostic-debug-index.md` as root one-row-per-scenario indexes for AI routing before opening per-scenario telemetry.
- Diagnostic observability runs add `diagnostic-observability-analysis.json` and `diagnostic-observability-analysis.md` as aggregate ranked subsystem/domain hypotheses for automated debuggers.
- Diagnostic observability runs add `diagnostic-coverage-ledger.json` and `diagnostic-coverage-ledger.md` to make happy-path versus intentional-negative coverage, cartridge test coverage, and known gaps explicit for automated reviewers.
- Diagnostic observability runs add `diagnostic-telemetry-catalog.json` and `diagnostic-telemetry-catalog.md` to catalog signal families, probe ids, event kinds, test signal mappings, and retained trace fields before full telemetry inspection.
- Diagnostic observability runs add `diagnostic-code-map.json` and `diagnostic-code-map.md` to map focus domains to emulator source files, diagnostic support files, regression tests, replay commands, and search terms.
- Diagnostic observability runs add `diagnostic-investigation-plan.json` and `diagnostic-investigation-plan.md` to convert ranked hypotheses and code-map entries into ordered debug routes with primary artifacts, replay commands, source/test paths, and handoff steps.
- Diagnostic observability runs add `diagnostic-scenario-dossiers.json` and `diagnostic-scenario-dossiers.md` to pre-join scenario health, failed probes, telemetry signal families, replay args, route hints, source files, tests, and next actions.
- Diagnostic e2e runs add `diagnostic-e2e-report.json` and `diagnostic-e2e-report.md` as the root acceptance verdict tying observability, route matrix, top-route replay, route evidence verification, and key AI artifact pointers together.
- Diagnostic e2e runs add `diagnostic-ai-observability-index.json` and `diagnostic-ai-observability-index.md` as the compact joined control-plane artifact for automated debuggers that need scenario cards, focus-domain routes, failed-probe lookup, mapped source/tests, replay commands, and coverage limits before loading full telemetry.
- Diagnostic e2e runs add `diagnostic-ai-query-smoke.json` and `diagnostic-ai-query-smoke.md` to prove the AI index can answer deterministic summary, top-route, scenario, focus-domain, probe, and coverage queries.
- Diagnostic e2e runs add `diagnostic-ai-diagnosis-smoke.json` and `diagnostic-ai-diagnosis-smoke.md` to prove an AI-selected route can regenerate focused replay telemetry and mapped narrow-test results.
- Diagnostic AI diagnosis runs add `ai-diagnosis/<route>/diagnostic-ai-diagnosis.json`, `diagnostic-ai-diagnosis.md`, and a nested route-check replay bundle to join the selected route, scenario, probe, source files, tests, search terms, stop conditions, and next actions for automated debugging.
- Diagnostic route matrices add `route-replay-matrix/diagnostic-route-matrix.json` and `diagnostic-route-matrix.md` plus per-route replay bundles to prove every generated route can regenerate focused replay evidence.
- Diagnostic route checks add `route-checks/<route>/diagnostic-route-check.json` and `diagnostic-route-check.md` to prove the selected route can regenerate focused replay evidence and pass its narrow regression-test commands.
- Diagnostic observability comparisons add `diagnostic-observability-comparison.json` and `diagnostic-observability-comparison.md` to report cross-run scenario regressions, health/focus/probe drift, and hypothesis rank/score changes.
- Diagnostic workflow artifacts include the current and prior scenario suites when a prior comparison ref is available, so automated reviewers can inspect both sides of a comparison failure.
- `python scripts/verify_diagnostic_suite.py --suite-dir target/diagnostics/scenario-suite` validates the uploaded scenario-suite artifact contract before release evidence is accepted.
- `python scripts/verify_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite` validates `observability-run.json`, root artifact pointers, debug-index rows, ranked hypotheses, coverage-ledger counts and known gaps, telemetry-catalog signal/probe/event/trace mappings, code-map source/test paths, investigation-plan routes, scenario-dossier joins, optional comparison summaries, and focused replay evidence before AI-facing observability evidence is accepted.
- `python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/scenario-suite` runs the full current-suite acceptance sequence and records command statuses, route proof counts, top route, scenario-dossier counts, AI-index counts, query-smoke checks, diagnosis-smoke replay/test checks, required artifact presence, errors, and AI handoff pointers in the e2e report before release evidence is accepted.
- `python scripts/query_diagnostic_ai_index.py --suite-dir target/diagnostics/scenario-suite smoke` validates the accepted AI index can be queried by top route, scenario, focus domain, failed probe, and coverage posture without hand-joining the underlying JSON files.
- `python scripts/run_diagnostic_ai_diagnosis.py --suite-dir target/diagnostics/scenario-suite` validates that the accepted AI index can drive a selected route into a fresh route-check bundle, mapped narrow tests, stop conditions, and next actions without hand-joining the underlying JSON files.
- `python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --all-routes --skip-tests --output-dir target/diagnostics/scenario-suite/route-replay-matrix` executes every investigation route replay into isolated bundles before route-table evidence is accepted.
- `python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite` executes the top investigation route by replaying its scenario into an isolated bundle and running its mapped narrow tests before route-level evidence is accepted.
- `python scripts/verify_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --require-matrix --require-top-route --expect-matrix-tests-skipped --write-summary` validates route matrix schema/counts, per-route replay bundles, top-route expected-vs-actual health/focus checks, and narrow-test command results, then persists the accepted verifier verdict before route evidence is accepted.
- `python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite` records the command status, git commit, verifier summary, debug-index status and entry count, analysis status and hypothesis count, coverage-ledger test/negative-fixture/gap counts, telemetry-catalog probe/event-kind/trace counts, scenario-dossier/actionable counts, optional comparison verdict/regression count, first AI next action, focused replay status, expected-vs-actual replay exit/health/focus checks, and artifact pointers for debugging handoff.
- `telemetry.json` includes actual and expected joypad masks for both controller ports, and the diagnostic cartridge exercises `$4016` and `$4017` serial reads, configurable expected masks, mid-stream strobe reset, and strobe-high hold behavior.
- `telemetry.json` includes OAM DMA start/end, active-cycle, first active-cycle parity, and DMC-overlap telemetry proving the transfer completed within the expected 513-514 CPU-cycle bucket while a phase-specific 3-cycle or 4-cycle DMC sample-DMA stall bucket was serviced during the OAM stall window.
- Mapper construction and bank-switching regressions cover supported mappers with synthetic fixtures.
- PPU mirroring and save-state truncation regressions pass.
- Malformed user inputs for save states, recordings, scripts, updater payloads, ROM metadata, and cartridge headers fail closed without panics.
- Compatibility claims stay limited to the supported mapper list and tested behavior. Do not claim complete commercial-library compatibility for 1.0.

### Performance

- Rendering and CRT regression tests pass.
- Run at least one local release build and version smoke check before an RC tag.
- Run Criterion rendering benchmarks before a user-facing 1.0 RC release and record the local host and date. Benchmarks are informational unless a clear regression is observed.

### Packaging

- `OXIDENES_RELEASE=1 cargo build --release` succeeds.
- The release binary prints the expected version with `--version`.
- `scripts/check_release_assets.py` validates staged platform artifacts.
- Windows installer jobs build, validate WiX inputs, install, launch `--version`, and uninstall in CI before publishing installer artifacts.
- Published artifact names must remain platform-specific: `oxidenes-windows-x64.exe`, `oxidenes-linux-x64`, `oxidenes-macos-arm64`, and `oxidenes-windows-installer`.

### Updater

- Updater parsing tests reject malformed JSON, invalid tags, current or older versions, and missing assets without panics.
- The app may open GitHub release URLs only after validating the URL prefix. It must not silently download, replace, or execute update binaries.

### Documentation And Support

- README, user guide, support policy, release checklist, and issue templates describe first-run setup, supported platforms, data locations, validation, and the no-ROM upload policy.
- Public examples must use the `oxidenes` command name and must not encourage ROM downloads.

### IP Compliance

- `python scripts/ip_compliance_audit.py` passes.
- No tracked ROMs, BIOS files, save files, screenshots, recordings, game art, manuals, music, icons, third-party logos, or archive bundles are present.
- Tests use generated fixtures unless a future binary fixture has explicit redistribution permission and passes a maintainer review.
- Release artifacts and WiX sources are validated by `scripts/check_release_assets.py`.
- Built-in ROM metadata remains factual, minimal, and user-overridable under `docs/ROM_METADATA_POLICY.md`.

## Release Decision

A GitHub Release is appropriate only for user-meaningful binary, packaging, compatibility, or stability changes. Documentation-only or planning-only checkpoints should use milestone tags without publishing binaries.
