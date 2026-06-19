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
- The diagnostic e2e runner passes and writes the root acceptance report, AI observability index, AI coverage gap plan, exhaustive diagnostic input sweep, AI query smoke, AI diagnosis smoke, AI fix-handoff smoke, AI route matrix, AI debug packet, AI debug packet verification, AI debug packet matrix, AI localization evaluation, AI session plan, AI session smoke, AI session smoke matrix, and AI artifact verification after observability, route matrix, top-route replay, and route evidence verification:
  `python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI coverage gap plan builder passes and proves the six known coverage gaps are mapped to current source/test anchors, telemetry signals, and validation commands before new cartridge fixtures are designed:
  `python scripts/build_diagnostic_ai_coverage_gap_plan.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI artifact verifier passes and cross-checks the completed AI artifact graph:
  `python scripts/verify_diagnostic_ai_artifacts.py --suite-dir target/diagnostics/scenario-suite --require-e2e-report --require-ai-route-matrix --require-ai-debug-packet --require-ai-debug-packet-matrix`
- The diagnostic AI route matrix passes and proves every AI focus-domain route can regenerate diagnosis and fix-handoff artifacts:
  `python scripts/run_diagnostic_ai_route_matrix.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI debug packet builder passes and packages a selected route into one relocatable debugger handoff with copied evidence and source/test context:
  `python scripts/build_diagnostic_ai_debug_packet.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI debug packet verifier passes and proves the selected packet can be validated from packet-local files, digests, route identity, replay evidence, source/test context, and narrow commands:
  `python scripts/verify_diagnostic_ai_debug_packet.py --packet-dir target/diagnostics/scenario-suite/ai-debug-packet`
- The diagnostic AI debug packet matrix passes and proves every AI focus-domain route can be packaged into a relocatable debugger handoff with copied evidence and source/test context:
  `python scripts/run_diagnostic_ai_debug_packet_matrix.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI localization evaluator passes and proves the scenario corpus localizes expected health/focus-domain contracts across healthy controls and intentional negative fixtures:
  `python scripts/evaluate_diagnostic_ai_localization.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI session-plan builder passes and proves every accepted AI route has deterministic read-order artifacts, replay commands, narrow-test commands, verification commands, and stop conditions:
  `python scripts/build_diagnostic_ai_session_plan.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI session smoke passes and proves the selected startup plan route can be consumed end to end, with read-order artifacts present, replay validated by generated triage, narrow tests passed, verification commands recorded, and stop conditions satisfied:
  `python scripts/run_diagnostic_ai_session_smoke.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI session smoke matrix passes and proves every accepted startup plan route can be consumed end to end by the same automated consumer path:
  `python scripts/run_diagnostic_ai_session_smoke_matrix.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI diagnosis runner passes and writes an executable route handoff with fresh replay evidence and mapped narrow-test results:
  `python scripts/run_diagnostic_ai_diagnosis.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic AI fix-handoff builder passes and resolves the selected diagnosis to source/test anchors and fix-loop commands:
  `python scripts/build_diagnostic_ai_fix_handoff.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic route matrix passes for every investigation route and writes `route-replay-matrix/diagnostic-route-matrix.json`, `diagnostic-route-matrix.md`, per-route `diagnostic-route-check.json`, and focused replay-bundle evidence:
  `python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --all-routes --skip-tests --output-dir target/diagnostics/scenario-suite/route-replay-matrix`
- The diagnostic route-check runner passes for the top investigation route and writes `route-checks/<route>/diagnostic-route-check.json`, `diagnostic-route-check.md`, and focused replay-bundle evidence:
  `python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite`
- The diagnostic route evidence verifier passes and contract-checks both the all-route replay matrix and top-route full check, then writes `diagnostic-route-evidence-verification.json` and `diagnostic-route-evidence-verification.md`:
  `python scripts/verify_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --require-matrix --require-top-route --expect-matrix-tests-skipped --write-summary`
- When a prior suite is available, the diagnostic observability comparison passes without regressions:
  `python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite --compare-suite-dir target/diagnostics/prior-scenario-suite --fail-on-comparison-regression`
- CI and release diagnostic-bundle jobs generate the prior suite from the pull request base SHA or previous push SHA when available, run the comparison gate, and upload `oxidenes-diagnostic-prior-scenario-suite` with the current `oxidenes-diagnostic-scenario-suite`.
- CI and release diagnostic-bundle jobs run a short diagnostic cartridge profile gate, upload `oxidenes-diagnostic-profile`, and when a prior checkout already contains the profiler, compare against `oxidenes-diagnostic-prior-profile` with a large-regression failure threshold.
- GitHub Actions CI is green on Windows, Linux, macOS, rustfmt, clippy, security audit, IP compliance, and diagnostic bundle jobs.

### Compatibility And Safety

- CPU fetch/decode execution is covered by generated in-memory test ROM content.
- The generated diagnostic cartridge bundle contains `manifest.json`, `triage.json`, `telemetry.json`, `report.md`, and `diagnostic.nes`; when a baseline is supplied it also contains `comparison.json` and `comparison.md`.
- Diagnostic `manifest.json`, `triage.json`, `telemetry.json`, scenario-suite root artifacts, input-sweep artifacts, and diagnostic profiles include build metadata with `version`, `build_type`, and `package_version` so RC evidence proves whether it came from a dev or release binary. Diagnostic profiles also record `config.target_dir` and fail when sampled telemetry is missing build metadata, so alternate `CARGO_TARGET_DIR` runs cannot silently profile a stale binary.
- `triage.json` and `telemetry.json` include explicit known coverage gaps so release evidence does not overstate cartridge coverage.
- `triage.json` and `telemetry.json` include a derived debug focus with the focus test/subsystem/domain, failed probe ids, final event, terminal instruction, and last focus-test instruction for AI-first triage.
- Diagnostic event tails include CPU register snapshots and diagnostic RAM watchpoints so failed headless runs preserve immediate execution-state context for automated triage.
- Diagnostic bundles include a bounded decoded instruction trace tail with diagnostic cartridge symbols so failed headless runs preserve the final opcode-boundary CPU/RAM context for automated triage.
- `telemetry.json` includes `ppu_vblank_timing`, the `ppu.vblank_timing.nmi_window` probe, and the `ppu.vblank_timing.edge_dots` probe, proving the cartridge-observed first/inter-NMI cadence stays inside the accepted NTSC timing window and PPUSTATUS vblank set/clear transitions occur on the expected PPU dots.
- `telemetry.json` includes expected-vs-observed full-frame render signature fields and the `ppu.frame_checksum` probe, proving the canonical default diagnostic background frame matches the accepted checksum before release evidence is accepted.
- Diagnostic scenario suites include the pass baseline, generated input-mask pass fixtures for alternating, all-released, all-pressed, joypad-1-only pressed, joypad-2-only pressed, sparse-bit, and nibble-split controller masks, intentional joypad, CPU, PPU, mapper, DMA, APU, and timeout fixtures, including CPU status/BIT flag coverage plus sprite-zero-hit and sprite-overflow PPU collision/evaluation fixtures with direct, hardware-bug false-positive, and hardware-bug false-negative sprite-overflow subcases, with expected-vs-actual debug-focus contracts, per-clause contract-match breakdowns, root baseline-comparison summaries, single-scenario replay args, a suite-level attention queue, plus `scenario-suite.json`, `scenario-suite.md`, `scenario-suite-observer.json`, and `scenario-suite-observer.md` indexes for automated regression triage.
- Diagnostic telemetry includes APU status-matrix evidence for `$4015` bits 0-3 plus the `apu.status_matrix` probe, DMC active-bit `$4015` evidence plus the `apu.dmc_status` probe, and APU audio output envelope evidence: sample-count, peak, RMS, and mean absolute windows plus the `apu.output_envelope` probe.
- Diagnostic observability runs add `diagnostic-debug-index.jsonl` and `diagnostic-debug-index.md` as root one-row-per-scenario indexes for AI routing before opening per-scenario telemetry.
- Diagnostic observability runs add `diagnostic-observability-analysis.json` and `diagnostic-observability-analysis.md` as aggregate ranked subsystem/domain hypotheses for automated debuggers.
- Diagnostic observability runs add `diagnostic-coverage-ledger.json` and `diagnostic-coverage-ledger.md` to make happy-path versus intentional-negative coverage, cartridge test coverage, and known gaps explicit for automated reviewers.
- Diagnostic observability runs add `diagnostic-telemetry-catalog.json` and `diagnostic-telemetry-catalog.md` to catalog signal families, probe ids, event kinds, test signal mappings, and retained trace fields before full telemetry inspection.
- Diagnostic observability runs add `diagnostic-code-map.json` and `diagnostic-code-map.md` to map focus domains to emulator source files, diagnostic support files, regression tests, replay commands, and search terms.
- Diagnostic observability runs add `diagnostic-investigation-plan.json` and `diagnostic-investigation-plan.md` to convert ranked hypotheses and code-map entries into ordered debug routes with primary artifacts, replay commands, source/test paths, and handoff steps.
- Diagnostic observability runs add `diagnostic-scenario-dossiers.json` and `diagnostic-scenario-dossiers.md` to pre-join scenario health, failed probes, telemetry signal families, replay args, route hints, source files, tests, and next actions.
- Diagnostic e2e runs add `diagnostic-e2e-report.json` and `diagnostic-e2e-report.md` as the root acceptance verdict tying observability, route matrix, top-route replay, route evidence verification, and key AI artifact pointers together.
- Diagnostic e2e runs add `diagnostic-ai-observability-index.json` and `diagnostic-ai-observability-index.md` as the compact joined control-plane artifact for automated debuggers that need scenario cards, focus-domain routes, failed-probe lookup, mapped source/tests, replay commands, and coverage limits before loading full telemetry.
- Diagnostic e2e runs add `diagnostic-ai-coverage-gap-plan.json` and `diagnostic-ai-coverage-gap-plan.md` to turn the six known diagnostic coverage gaps into ranked test-design handoffs with current source/test anchors, diagnostic files, telemetry signals, and acceptance commands.
- Diagnostic e2e runs add `diagnostic-input-sweep.json` and `diagnostic-input-sweep.md` to prove the `input_port_matrix` companion artifact exhaustively validates all 65,536 two-port joypad mask pairs, strobe-high hold, low-strobe serial reads, and post-exhaustion reads before AI debugging evidence is accepted.
- Diagnostic e2e runs add `diagnostic-ai-query-smoke.json` and `diagnostic-ai-query-smoke.md` to prove the AI index can answer deterministic summary, top-route, scenario, focus-domain, probe, and coverage queries.
- Diagnostic e2e runs add `diagnostic-ai-diagnosis-smoke.json` and `diagnostic-ai-diagnosis-smoke.md` to prove an AI-selected route can regenerate focused replay telemetry and mapped narrow-test results.
- Diagnostic AI diagnosis runs add `ai-diagnosis/<route>/diagnostic-ai-diagnosis.json`, `diagnostic-ai-diagnosis.md`, and a nested route-check replay bundle to join the selected route, scenario, probe, source files, tests, search terms, stop conditions, and next actions for automated debugging.
- Diagnostic e2e runs add `diagnostic-ai-fix-handoff-smoke.json` and `diagnostic-ai-fix-handoff-smoke.md` to prove a diagnosis can be resolved into source/test line anchors, replay commands, narrow tests, verification commands, and fix-loop stop conditions.
- Diagnostic AI fix-handoff runs add `diagnostic-ai-fix-handoff.json` and `diagnostic-ai-fix-handoff.md` to give automated debuggers bounded code-inspection anchors before editing emulator source.
- Diagnostic e2e runs add `diagnostic-ai-route-matrix.json` and `diagnostic-ai-route-matrix.md` to prove every AI focus-domain route can regenerate diagnosis, replay evidence, narrow tests, source/test anchors, and fix-handoff commands.
- Diagnostic e2e runs add `diagnostic-ai-debug-packet.json`, `diagnostic-ai-debug-packet.md`, and `ai-debug-packet/` to provide a selected-route packet with digest-checked copied evidence, replay telemetry, source/test context windows, and fix-loop commands.
- Diagnostic e2e runs add `diagnostic-ai-debug-packet-verification.json` and `diagnostic-ai-debug-packet-verification.md` to prove the selected debug packet can be validated from packet-local files and SHA-256 digests after it is copied away from the suite.
- Diagnostic e2e runs add `diagnostic-ai-debug-packet-matrix.json`, `diagnostic-ai-debug-packet-matrix.md`, and `ai-debug-packet-matrix/` to prove every AI focus-domain route can be packaged into a digest-checked, packet-self-verified debug packet with source/test context windows.
- Diagnostic e2e runs add `diagnostic-ai-localization-eval.json` and `diagnostic-ai-localization-eval.md` to score whether every scenario matches its expected health/focus-domain contract and every negative fixture has route evidence, source/test anchors, packet self-verification, and a perfect localization score.
- Diagnostic e2e runs add `diagnostic-ai-session-plan.json` and `diagnostic-ai-session-plan.md` to turn every accepted AI route into a deterministic debugger startup plan with ordered artifacts, replay commands, narrow tests, verification commands, and stop conditions.
- Diagnostic e2e runs add `diagnostic-ai-session-smoke.json` and `diagnostic-ai-session-smoke.md` to prove the selected startup plan route can be consumed end to end, including triage-validated replay, narrow tests, recorded verification commands, and stop conditions.
- Diagnostic e2e runs add `diagnostic-ai-session-smoke-matrix.json`, `diagnostic-ai-session-smoke-matrix.md`, and `ai-session-smoke-matrix/` to prove every accepted startup plan route can be consumed end to end with per-route smoke outputs.
- Diagnostic e2e runs add `diagnostic-ai-artifact-verification.json` and `diagnostic-ai-artifact-verification.md` to prove the AI-facing artifact graph is internally consistent before automated debugger or fix loops consume it.
- Diagnostic AI artifact verification includes `automation_readiness.routes` so a downloaded suite exposes one compact route-by-route map of replay, diagnosis, fix-handoff, narrow-test, source/test anchor, packet self-verification, and debug-packet readiness.
- Diagnostic route matrices add `route-replay-matrix/diagnostic-route-matrix.json` and `diagnostic-route-matrix.md` plus per-route replay bundles to prove every generated route can regenerate focused replay evidence.
- Diagnostic route checks add `route-checks/<route>/diagnostic-route-check.json` and `diagnostic-route-check.md` to prove the selected route can regenerate focused replay evidence and pass its narrow regression-test commands.
- Diagnostic observability comparisons add `diagnostic-observability-comparison.json` and `diagnostic-observability-comparison.md` to report cross-run scenario regressions, health/focus/probe drift, and hypothesis rank/score changes.
- Diagnostic workflow artifacts include the current and prior scenario suites when a prior comparison ref is available, so automated reviewers can inspect both sides of a comparison failure.
- `python scripts/verify_diagnostic_suite.py --suite-dir target/diagnostics/scenario-suite` validates the uploaded scenario-suite artifact contract before release evidence is accepted.
- `python scripts/verify_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite` validates `observability-run.json`, root artifact pointers, debug-index rows, ranked hypotheses, coverage-ledger counts and known gaps, telemetry-catalog signal/probe/event/trace mappings, code-map source/test paths, investigation-plan routes, scenario-dossier joins, optional comparison summaries, and focused replay evidence before AI-facing observability evidence is accepted.
- `python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/scenario-suite` runs the full current-suite acceptance sequence and records command statuses, route proof counts, top route, scenario-dossier counts, AI-index counts, exhaustive input-sweep status/counts, AI coverage-gap ready/mapped/source/test/telemetry/companion-evidence counts, query-smoke checks, diagnosis-smoke replay/test checks, fix-handoff source/test anchors, AI route-matrix counts, AI debug-packet file/context counts, AI debug-packet self-verifier checks, AI debug-packet matrix route/context/self-verifier counts, AI localization scorecard counts, AI session-plan route/command/read-order/stop-condition counts, AI session-smoke replay/narrow-test counts, AI session-smoke matrix route/replay/narrow-test counts, required artifact presence, errors, and AI handoff pointers in the e2e report before release evidence is accepted.
- `python scripts/build_diagnostic_ai_coverage_gap_plan.py --suite-dir target/diagnostics/scenario-suite` validates that every known diagnostic coverage gap is ready for the next cartridge fixture design pass with source files, regression tests, diagnostic files, telemetry signals, suggested next tests, acceptance commands, and any validated companion artifacts such as the input-port sweep.
- `python scripts/query_diagnostic_ai_index.py --suite-dir target/diagnostics/scenario-suite smoke` validates the accepted AI index can be queried by top route, scenario, focus domain, failed probe, and coverage posture without hand-joining the underlying JSON files.
- `python scripts/run_diagnostic_ai_diagnosis.py --suite-dir target/diagnostics/scenario-suite` validates that the accepted AI index can drive a selected route into a fresh route-check bundle, mapped narrow tests, stop conditions, and next actions without hand-joining the underlying JSON files.
- `python scripts/build_diagnostic_ai_fix_handoff.py --suite-dir target/diagnostics/scenario-suite` validates that a passed diagnosis can be resolved to concrete source matches, test matches, narrow commands, and verification commands before emulator edits begin.
- `python scripts/run_diagnostic_ai_route_matrix.py --suite-dir target/diagnostics/scenario-suite` validates that every accepted AI focus-domain route can drive diagnosis, fresh replay evidence, mapped narrow tests, source/test anchors, and fix-loop commands before release evidence is accepted.
- `python scripts/build_diagnostic_ai_debug_packet.py --suite-dir target/diagnostics/scenario-suite` validates that the selected accepted route can be packaged into one relocatable handoff with copied index/query/diagnosis/fix/replay evidence, source/test context windows, and digest-checked packet records before release evidence is accepted.
- `python scripts/verify_diagnostic_ai_debug_packet.py --packet-dir target/diagnostics/scenario-suite/ai-debug-packet` validates that one copied packet is self-consistent without the original suite graph before an automated debugger consumes it.
- `python scripts/run_diagnostic_ai_debug_packet_matrix.py --suite-dir target/diagnostics/scenario-suite` validates that every accepted AI focus-domain route can be packaged into one relocatable handoff with copied index/query/diagnosis/fix/replay evidence, source/test context windows, digest-checked packet records, and packet-local self-verification before release evidence is accepted.
- `python scripts/evaluate_diagnostic_ai_localization.py --suite-dir target/diagnostics/scenario-suite` validates that the accepted scenario suite has 35 negative fixtures, 35 matched focus domains, route-ready evidence, source/test anchors, packet self-verification, and perfect localization scores before release evidence is accepted.
- `python scripts/build_diagnostic_ai_session_plan.py --suite-dir target/diagnostics/scenario-suite` validates that every accepted AI route has read-order artifacts, replay commands, narrow-test commands, verification commands, and stop conditions before automated debugger sessions begin.
- `python scripts/run_diagnostic_ai_session_smoke.py --suite-dir target/diagnostics/scenario-suite` validates that the selected startup plan route can resolve read-order artifacts, regenerate focused replay evidence with expected triage semantics, pass narrow tests, retain verification commands, and satisfy stop conditions before automated debugger sessions begin.
- `python scripts/run_diagnostic_ai_session_smoke_matrix.py --suite-dir target/diagnostics/scenario-suite` validates that every accepted startup plan route can resolve read-order artifacts, regenerate focused replay evidence with expected triage semantics, pass narrow tests, retain verification commands, and satisfy stop conditions before automated debugger sessions begin.
- `python scripts/verify_diagnostic_ai_artifacts.py --suite-dir target/diagnostics/scenario-suite --require-e2e-report --require-ai-route-matrix --require-ai-debug-packet --require-ai-debug-packet-matrix` validates that the AI index, AI coverage gap plan, diagnostic input sweep, input-sweep companion evidence in the gap plan, query smoke, diagnosis smoke, fix handoff, AI route matrix, AI debug packet, packet self-verification, AI debug packet matrix, localization evaluation, session plan, session smoke, session smoke matrix, e2e summary, route identities, coverage posture, stop conditions, digest-checked packet files, automation readiness, and required artifact paths agree before release evidence is accepted.
- `python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --all-routes --skip-tests --output-dir target/diagnostics/scenario-suite/route-replay-matrix` executes every investigation route replay into isolated bundles before route-table evidence is accepted.
- `python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite` executes the top investigation route by replaying its scenario into an isolated bundle and running its mapped narrow tests before route-level evidence is accepted.
- `python scripts/verify_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --require-matrix --require-top-route --expect-matrix-tests-skipped --write-summary` validates route matrix schema/counts, per-route replay bundles, top-route expected-vs-actual health/focus checks, and narrow-test command results, then persists the accepted verifier verdict before route evidence is accepted.
- `python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite` records the command status, git commit, verifier summary, debug-index status and entry count, analysis status and hypothesis count, coverage-ledger test/negative-fixture/gap counts, telemetry-catalog probe/event-kind/trace counts, scenario-dossier/actionable counts, optional comparison verdict/regression count, first AI next action, focused replay status, expected-vs-actual replay exit/health/focus checks, and artifact pointers for debugging handoff.
- `telemetry.json` includes actual and expected joypad masks for both controller ports, and the diagnostic cartridge exercises `$4016` and `$4017` serial reads, configurable expected masks, mid-stream strobe reset, strobe-high hold behavior, and a combined input-port matrix covering strobe-high, serial-shift, and overread behavior.
- `telemetry.json` includes PPU sprite-zero-hit status and case-count probes so automated debuggers can localize sprite/background collision signaling regressions to `ppu.sprite_zero_hit`.
- `telemetry.json` includes PPU sprite-overflow status, case-count, and OAM-restore probes so automated debuggers can localize sprite evaluation regressions to `ppu.sprite_overflow` without confusing the fixture with DMA checksum drift.
- `telemetry.json` includes PPU sprite-priority frame-pixel samples so automated debuggers can localize sprite/background priority mux regressions to `ppu.sprite_priority`.
- `telemetry.json` includes PPU fine-X, coarse-X, coarse-X nametable-wrap, and vertical scroll-seam frame-pixel samples so automated debuggers can localize background seam regressions to `ppu.scroll_seam`.
- `telemetry.json` includes the deterministic render-frame checksum and expected `ppu.frame_checksum` probe for the canonical default fixture so automated debuggers can localize full-frame render drift to `ppu.rendering.frame_signature`.
- `telemetry.json` includes OAM DMA start/end, active-cycle, per-transfer phase-matrix buckets/parities, and DMC-overlap telemetry proving the accepted run covers both 513-cycle and 514-cycle OAM DMA buckets while a phase-specific 3-cycle or 4-cycle DMC sample-DMA stall bucket was serviced during the OAM stall window.
- Mapper construction and bank-switching regressions cover supported mappers with synthetic fixtures.
- PPU mirroring and save-state truncation regressions pass.
- Malformed user inputs for save states, recordings, scripts, updater payloads, ROM metadata, and cartridge headers fail closed without panics.
- Compatibility claims stay limited to the supported mapper list and tested behavior. Do not claim complete commercial-library compatibility for 1.0.

### Performance

- Rendering and CRT regression tests pass.
- `python scripts/profile_diagnostic_cartridge.py --output-dir target/diagnostics/diagnostic-profile` passes and records the diagnostic cartridge's wall-clock throughput, cycles per second, frames per second, and slowest cartridge tests for performance review.
- `cargo test --test version_cli_tests` passes in the default dev build and proves `--help`/`--version` expose the `-dev` build metadata.
- `OXIDENES_RELEASE=1 cargo test --test version_cli_tests` passes and proves release-mode `--help`/`--version` expose the clean package version.
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
