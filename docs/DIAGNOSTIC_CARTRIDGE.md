# OxideNES Diagnostic Cartridge

OxideNES includes a generated, IP-safe diagnostic cartridge for headless emulator validation. The cartridges are assembled from deterministic 6502 instructions and CHR byte patterns at runtime; no `.nes` file or third-party ROM content is committed. The main generated ROM uses Mapper 2/UXROM so the same cartridge can validate PRG bank switching and PRG RAM access through the normal CPU bus, while focused variant cartridges exercise additional mapper behavior through the same headless runner.

Run it with:

```powershell
cargo run --bin oxidenes-diagnostic -- --json target/diagnostics/telemetry.json --report target/diagnostics/report.md --dump-rom target/diagnostics/oxidenes-diagnostic.nes
```

The runner exits `0` only when the cartridge and host-side checks pass. It exits `1` for diagnostic failures or timeouts, and `2` for CLI/build errors.

Use `--joypad1 <BYTE>` and `--joypad2 <BYTE>` to override the host-side
controller masks used by the cartridge. Use `--expect-joypad1 <BYTE>` and
`--expect-joypad2 <BYTE>` when the cartridge should validate a non-default
expected mask instead of treating the host override as an intentional mismatch.
The defaults match the generated assertions: joypad 1 expects A + Right
(`0x81`) and joypad 2 expects Start + Down (`0x28`). Overriding only the host
mask is useful for failure-localization smokes because the run still emits
telemetry, triage JSON, and bundles before exiting `1`.

Use `--fault-injection <NAME>` with `--bundle-dir` to replay a named
intentional diagnostic fixture without regenerating the full scenario suite.
The supported names match the scenario-suite fault labels, such as
`joypad_strobe_high_hold`, `ppu_status_latch_reset`, and
`cpu_indirect_jmp_page_wrap`. Unknown names are rejected with exit code `2` and
the supported-name list.

Use `--json <FILE>` for the full machine-readable telemetry envelope and
`--report <FILE>` for a Markdown triage artifact built from the same run. The
report contains the verdict, derived analysis, failure localization, coverage
summary, debug focus, timing/timeline table, bounded instruction trace tail,
observation probe table, next actions, host failures, and final event tail. This gives CI
logs, issue attachments, and AI debugging sessions a stable human-readable
entry point while preserving the raw JSON for exact tooling.

Use `--triage-json <FILE>` for a compact machine-readable handoff artifact. It
summarizes verdict, health, current test, derived `debug_focus`, first failure,
failed/skipped probes, timing, optional baseline comparison, next actions,
artifact hints, the bounded instruction trace tail, and the final event tail
without requiring tools to scrape `report.md` or load the full telemetry
envelope first.

To compare a run against a known-good telemetry baseline:

```powershell
cargo run --bin oxidenes-diagnostic -- --json target/diagnostics/current.json --report target/diagnostics/current.md --baseline-json target/diagnostics/baseline.json --comparison-json target/diagnostics/comparison.json --comparison-report target/diagnostics/comparison.md --no-stdout
```

Baseline comparison hard-fails on result, health, coverage, structured probe, or
per-test outcome regressions and exits `1`. It records warning-level
differences for timing and observable artifact drift such as frame/OAM
checksums, allowing CI and AI agents to separate real correctness regressions
from values that need review.

To hand a complete run to an automated debugger, write a diagnostic bundle:

```powershell
cargo run --bin oxidenes-diagnostic -- --bundle-dir target/diagnostics/latest-bundle --baseline-json target/diagnostics/baseline.json --no-stdout
```

The bundle always includes:

- `manifest.json`: bundle schema, pass/fail state, runner config, artifact list,
  SHA-256 digests, and AI handoff hints
- `triage.json`: compact AI-readable debug focus, failure summary, next actions,
  trace anchors, and artifact pointers
- `telemetry.json`: full machine-readable diagnostic telemetry
- `report.md`: Markdown triage report for human and AI review
- `diagnostic.nes`: the generated IP-safe test cartridge used for the run

When `--baseline-json` is supplied, the bundle also includes `comparison.json`
and `comparison.md`. The runner still exits `1` for diagnostic or comparison
failures after writing the bundle, so CI can upload the artifact directory for
post-failure analysis.

GitHub Actions runs this bundle path in both CI and release workflows. The
uploaded artifact is named `oxidenes-diagnostic-bundle` and is the preferred
handoff package for automated debugging, release evidence, and failure triage.
Start with `manifest.json` for integrity and `triage.json` for the compact
machine-readable failure focus before loading `telemetry.json`.

To use the diagnostic cartridge as a repeatable profiling workload, run:

```powershell
python scripts/profile_diagnostic_cartridge.py --output-dir target/diagnostics/diagnostic-profile
```

The profiler builds `oxidenes-diagnostic` once, runs configurable warmup and
sample passes, stores each run's telemetry/report under `samples/`, and writes
`diagnostic-cartridge-profile.json` plus `diagnostic-cartridge-profile.md`.
The summary records wall-clock duration, emulated cycles per second, frames per
second, the most frequent slowest test, and the highest cartridge cycle-duration
tests. Use `--samples`, `--warmups`, `--profile debug`, `--binary`, or
`--skip-build` to tailor the workload. To compare against a prior profile, pass
`--baseline-json <FILE> --fail-on-regression`; by default the comparison allows
up to a 20% wall-time or throughput regression before failing.

GitHub Actions CI and release diagnostic jobs also run a short debug-profile
gate after the diagnostic e2e suite. When a prior checkout with the profiler is
available, the job writes a prior profile, compares the current run against it,
and fails on large throughput or wall-time regressions. The uploaded artifacts
are named `oxidenes-diagnostic-profile` and
`oxidenes-diagnostic-prior-profile`.

To generate a reusable AI debugging corpus with both healthy and known-bad
outcomes, write a diagnostic scenario suite:

```powershell
cargo run --bin oxidenes-diagnostic -- --scenario-suite-dir target/diagnostics/scenario-suite --no-stdout
```

For the full local observability loop, use the wrapper that generates the
scenario suite, runs the verifier, replays one selected scenario from its
`replay_args`, writes a root debug index, aggregate analysis, coverage ledger,
and telemetry catalog, optionally compares the run against a prior observability suite, and writes
`observability-run.json` plus `observability-run.md` into the suite directory:

```powershell
python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite
```

For a single local acceptance command that runs the full uploaded-artifact gate,
including observability generation, observability verification, all-route replay
matrix, top-route replay with narrow tests, route evidence verification, and one
root acceptance report, run:

```powershell
python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/scenario-suite
```

This writes `diagnostic-e2e-report.json` and `diagnostic-e2e-report.md` at the
suite root. The same command also writes
`diagnostic-ai-observability-index.json` and
`diagnostic-ai-observability-index.md`, a compact joined index across the
scenario dossiers, route table, telemetry catalog, coverage ledger, and route
evidence verifier. It also writes `diagnostic-ai-coverage-gap-plan.json` and
`diagnostic-ai-coverage-gap-plan.md`, turning the six known coverage gaps into
ranked test-design handoffs with current source/test anchors, telemetry
signals, and validation commands. This keeps the gaps explicit and does not
claim they are fixed. It also writes `diagnostic-ai-query-smoke.json` and
`diagnostic-ai-query-smoke.md`, proving that the index can answer deterministic
summary, top-route, scenario, focus-domain, probe, and coverage queries. It then
writes `diagnostic-ai-diagnosis-smoke.json` and
`diagnostic-ai-diagnosis-smoke.md`, proving the accepted AI index can select a
route, regenerate a focused replay bundle, run the mapped narrow tests, and
emit a compact diagnosis handoff. It also writes
`diagnostic-ai-fix-handoff-smoke.json` and
`diagnostic-ai-fix-handoff-smoke.md`, proving the selected route can be resolved
into source/test line anchors and fix-loop commands. It then writes
`diagnostic-ai-route-matrix.json` and `diagnostic-ai-route-matrix.md`, proving
every AI focus-domain route can regenerate a diagnosis, replay evidence, mapped
narrow-test results, source/test anchors, and fix-loop commands. It then writes
`diagnostic-ai-debug-packet.json`, `diagnostic-ai-debug-packet.md`, and
`ai-debug-packet/`, packaging the selected route's compact evidence, replay
telemetry, source/test context windows, and fix commands into one relocatable
AI handoff. It then writes `diagnostic-ai-debug-packet-verification.json` and
`diagnostic-ai-debug-packet-verification.md`, proving that the selected packet
can be validated from packet-local files, digests, route identity, replay
evidence, source/test context, and narrow commands without trusting the original
suite graph. It then writes `diagnostic-ai-debug-packet-matrix.json` and
`diagnostic-ai-debug-packet-matrix.md`, proving every AI route has the same
relocatable packet quality. It then writes
`diagnostic-ai-localization-eval.json` plus
`diagnostic-ai-localization-eval.md`, scoring whether every scenario matches
its expected health/focus-domain contract. It then writes
`diagnostic-ai-session-plan.json` plus `diagnostic-ai-session-plan.md`, turning
the accepted routes into deterministic debugger startup plans with read order,
commands, and stop conditions. It then writes
`diagnostic-ai-session-smoke.json` plus `diagnostic-ai-session-smoke.md`,
proving the selected startup plan route can be executed by an automated
consumer. Replay commands are validated against generated triage semantics, so
intentional negative-fixture nonzero exits are accepted only when triage agrees
with the selected focus domain and probe. It then writes
`diagnostic-ai-session-smoke-matrix.json`,
`diagnostic-ai-session-smoke-matrix.md`, and `ai-session-smoke-matrix/`,
proving every accepted startup plan route can be executed by the same automated
consumer path. It then writes
`diagnostic-ai-artifact-verification.json` and
`diagnostic-ai-artifact-verification.md`, proving the AI index, AI coverage gap
plan, query smoke, diagnosis smoke, fix handoff, AI route matrix, AI debug
packet, packet self-verification, all-route AI debug packet matrix,
localization evaluation, session plan, session smoke, and session smoke matrix
agree on route identities, artifact paths, and stop
conditions. Start with the e2e report to decide whether the diagnostic corpus
is trusted, then use the AI artifact verification, AI coverage gap plan, AI
session smoke matrix, AI session smoke, AI session plan, AI debug packet
matrix, AI debug packet, AI route matrix, AI index, query CLI, diagnosis
runner, or fix handoff before opening full telemetry.

To build only the AI coverage-gap plan from an accepted observability suite,
run:

```powershell
python scripts/build_diagnostic_ai_coverage_gap_plan.py --suite-dir target/diagnostics/scenario-suite
```

The builder writes `diagnostic-ai-coverage-gap-plan.json` and
`diagnostic-ai-coverage-gap-plan.md`. Each known gap is mapped to the closest
focus domains, source files, tests, diagnostic files, telemetry signals, and
acceptance commands so the next cartridge fixture can be designed from one
artifact without overstating current coverage.

To query an accepted suite without hand-joining JSON files, run:

```powershell
python scripts/query_diagnostic_ai_index.py --suite-dir target/diagnostics/scenario-suite summary --json
python scripts/query_diagnostic_ai_index.py --suite-dir target/diagnostics/scenario-suite top-route --json
python scripts/query_diagnostic_ai_index.py --suite-dir target/diagnostics/scenario-suite scenario ppu_nmi_timeout_fault --json
python scripts/query_diagnostic_ai_index.py --suite-dir target/diagnostics/scenario-suite focus-domain ppu.nmi --json
python scripts/query_diagnostic_ai_index.py --suite-dir target/diagnostics/scenario-suite probe ppu.nmi_count --json
python scripts/query_diagnostic_ai_index.py --suite-dir target/diagnostics/scenario-suite coverage-gaps --json
```

Use `list scenarios`, `list focus-domains`, or `list probes` to discover valid
query ids. Use `smoke` to write the same query-smoke artifacts generated by the
e2e runner.

To turn an accepted AI index into an executable diagnosis handoff, run:

```powershell
python scripts/run_diagnostic_ai_diagnosis.py --suite-dir target/diagnostics/scenario-suite
```

By default this selects the e2e suite's top route, replays that scenario into
`ai-diagnosis/<route>/route-check/replay-bundle/`, runs the route's mapped
narrow tests, and writes `diagnostic-ai-diagnosis.json` plus
`diagnostic-ai-diagnosis.md`. Use `--route-id`, `--scenario-id`,
`--focus-domain`, or `--probe-id` when automation already knows the failure
handle. The JSON report joins the selected scenario, probe, route, source files,
test files, search terms, fresh replay artifacts, stop conditions, and ordered
next actions so an AI debugger can start from a single compact artifact.

To convert a passed diagnosis into source/test anchors for an automated fix
loop, run:

```powershell
python scripts/build_diagnostic_ai_fix_handoff.py --suite-dir target/diagnostics/scenario-suite
```

By default this consumes the e2e `diagnostic-ai-diagnosis-smoke.json`. Use
`--diagnosis-json <FILE>` to target a custom diagnosis run. It writes
`diagnostic-ai-fix-handoff.json` plus `diagnostic-ai-fix-handoff.md`, including
bounded source and test line matches for the selected search terms, mapped
narrow-test commands, replay commands, verification commands, stop conditions,
and a fix loop that tells an AI debugger what to inspect before editing.

To prove every AI focus-domain route can regenerate an executable diagnosis and
fix handoff, run:

```powershell
python scripts/run_diagnostic_ai_route_matrix.py --suite-dir target/diagnostics/scenario-suite
```

This writes `diagnostic-ai-route-matrix.json` plus
`diagnostic-ai-route-matrix.md`, with per-route diagnosis and fix-handoff files
under `ai-route-matrix/<route>/`. A passed matrix means all 23 focus-domain
routes replay, run their mapped narrow tests, resolve source/test anchors, and
meet their stop conditions.

To package a selected route into one relocatable AI debugger handoff, run:

```powershell
python scripts/build_diagnostic_ai_debug_packet.py --suite-dir target/diagnostics/scenario-suite
```

By default this consumes the e2e top-route diagnosis and fix handoff. Use
`--route-id <ID>` to package any route from `diagnostic-ai-route-matrix.json`,
`--route-matrix-json <FILE>` for a custom all-route matrix, or
`--diagnosis-json` and `--fix-handoff-json` for custom runs. It writes
`diagnostic-ai-debug-packet.json`, `diagnostic-ai-debug-packet.md`, and
`ai-debug-packet/`, including digest-checked copies of the compact AI index,
query smoke, selected diagnosis, fix handoff, route-check evidence, replay
triage, full replay telemetry, replay report, generated cartridge, and a
`source-context.json` file with bounded source/test windows around the mapped
line anchors.

To verify a copied debug packet without the original scenario suite, run:

```powershell
python scripts/verify_diagnostic_ai_debug_packet.py --packet-dir target/diagnostics/scenario-suite/ai-debug-packet
```

This writes `diagnostic-ai-debug-packet-verification.json` plus
`diagnostic-ai-debug-packet-verification.md`. A passed verifier means the
packet-local manifest, read order, required files, SHA-256 digests, selected
route identity, diagnosis, fix handoff, route check, replay triage/telemetry,
source/test context windows, and narrow commands are internally consistent.

To prove every AI focus-domain route can be packaged into a relocatable debug
packet, run:

```powershell
python scripts/run_diagnostic_ai_debug_packet_matrix.py --suite-dir target/diagnostics/scenario-suite
```

This writes `diagnostic-ai-debug-packet-matrix.json` plus
`diagnostic-ai-debug-packet-matrix.md`, with per-route packets under
`ai-debug-packet-matrix/<route-slug>/`. A passed matrix means every accepted AI
route has copied replay evidence, a packet-local verification result,
digest-checked packet files, source context, test context, matching route
identity, and passed packet stop conditions.

To score localization quality across the accepted scenario suite, run:

```powershell
python scripts/evaluate_diagnostic_ai_localization.py --suite-dir target/diagnostics/scenario-suite
```

This writes `diagnostic-ai-localization-eval.json` plus
`diagnostic-ai-localization-eval.md`. A passed evaluation means all 31
scenarios match their expected health and focus-domain contracts, the 23
intentional negative fixtures are not being reduced to happy-path evidence, and
each negative fixture has route evidence, source/test anchors, packet
self-verification, and a perfect localization score.

To build a deterministic startup plan for automated debugging sessions, run:

```powershell
python scripts/build_diagnostic_ai_session_plan.py --suite-dir target/diagnostics/scenario-suite
```

This writes `diagnostic-ai-session-plan.json` plus
`diagnostic-ai-session-plan.md`. A passed plan means all 23 accepted AI routes
have ordered read artifacts, replay commands, narrow-test commands,
verification commands, and stop conditions before an automated debugger starts
editing emulator code.

To smoke-test the selected startup plan route as an automated consumer, run:

```powershell
python scripts/run_diagnostic_ai_session_smoke.py --suite-dir target/diagnostics/scenario-suite
```

This writes `diagnostic-ai-session-smoke.json` plus
`diagnostic-ai-session-smoke.md`. A passed smoke means the selected route's
read-order artifacts resolve, its replay command is validated by generated
triage semantics, its narrow-test commands pass, its verification commands are
recorded for post-edit use, and its stop conditions are already satisfied.

To smoke-test every startup plan route as an automated consumer, run:

```powershell
python scripts/run_diagnostic_ai_session_smoke_matrix.py --suite-dir target/diagnostics/scenario-suite
```

This writes `diagnostic-ai-session-smoke-matrix.json` plus
`diagnostic-ai-session-smoke-matrix.md`, with per-route smoke outputs under
`ai-session-smoke-matrix/`. A passed matrix means every accepted route can
resolve its read-order artifacts, validate replay by generated triage, pass its
narrow tests, retain verification commands, and satisfy stop conditions.

To validate the AI-facing artifact graph directly, run:

```powershell
python scripts/verify_diagnostic_ai_artifacts.py --suite-dir target/diagnostics/scenario-suite --require-e2e-report --require-ai-route-matrix --require-ai-debug-packet --require-ai-debug-packet-matrix
```

Use `--require-e2e-report` when validating a completed local or downloaded CI
artifact after `diagnostic-e2e-report.json` exists, and
`--require-ai-route-matrix` when the bundle should include all-route AI
diagnosis coverage. Use `--require-ai-debug-packet` when the bundle should
include a selected-route packet with digest-checked copied evidence and source
context, and `--require-ai-debug-packet-matrix` when the bundle should include
all-route packet coverage. The verifier checks that the AI index, query smoke,
diagnosis smoke, fix handoff, AI route matrix, AI debug packet, AI debug packet
matrix, packet self-verification, localization evaluation, and optional e2e
summary all passed, agree on route identities, include the expected
non-happy-path coverage counts, preserve source/test anchors, prove the session
plan has ready routes, commands, read-order artifacts, and stop conditions,
prove the selected session smoke executed replay and narrow-test commands, and
prove the session smoke matrix executed every accepted route startup contract
while still pointing to present artifacts after a CI bundle is
downloaded to a different directory. It also writes
`automation_readiness.routes`, a compact per-route map that tells an automated
debugger whether each route has replay evidence,
diagnosis, fix handoff, narrow tests, source/test anchors, packet
self-verification, and a debug packet with context windows.

The observability wrapper writes `diagnostic-debug-index.jsonl`,
`diagnostic-debug-index.md`, `diagnostic-observability-analysis.json`, and
`diagnostic-observability-analysis.md` alongside the generated scenario-suite
root files. It also writes `diagnostic-coverage-ledger.json` and
`diagnostic-coverage-ledger.md`, which summarize the happy-path versus
intentional-negative fixture balance, every cartridge test, paired negative
fixtures, subsystem/tier coverage, and known coverage gaps. It writes
`diagnostic-telemetry-catalog.json` and `diagnostic-telemetry-catalog.md`,
which catalog telemetry signal families, probe ids, event kinds, test signal
mappings, and retained trace fields. It also writes `diagnostic-code-map.json` and
`diagnostic-code-map.md`, which map each focus domain to the emulator source
files, regression tests, diagnostic support files, search terms, debug anchor,
first artifact to open, and replay/test commands an automated debugger should
use next. The wrapper then writes `diagnostic-investigation-plan.json` and
`diagnostic-investigation-plan.md`, which join the ranked hypotheses, debug
index rows, code-map entries, primary artifacts, replay commands, and ordered
handoff steps into executable debug routes. It also writes
`diagnostic-scenario-dossiers.json` and `diagnostic-scenario-dossiers.md`, which
pre-join each scenario's health, failed probes, start artifacts, replay args,
telemetry signal families, route, source files, tests, and next actions for
scenario-id-first automated debugging. When `--compare-suite-dir <DIR>` is supplied, it also writes
`diagnostic-observability-comparison.json` and
`diagnostic-observability-comparison.md`. The root scenario-suite files are
`scenario-suite.json`, `scenario-suite.md`, `scenario-suite-observer.json`, and
`scenario-suite-observer.md`. The suite also writes one full bundle per
scenario: `pass`, `input_mask_matrix_pass`,
`input_mask_all_released_pass`, `input_mask_all_pressed_pass`,
`input_mask_joypad1_pressed_pass`, `input_mask_joypad2_pressed_pass`,
`input_mask_sparse_bits_pass`, `input_mask_nibble_split_pass`,
`joypad1_mismatch`, `joypad2_mismatch`, `dma_oam_transfer_fault`,
`dma_phase_matrix_fault`,
`apu_status_fault`, `cpu_zero_page_wrap_fault`, `cpu_indirect_jmp_fault`,
`cpu_addressing_matrix_fault`, `input_port_matrix_fault`, `ppu_read_buffer_fault`,
`mapper2_bank_switch_fault`, `mapper2_prg_ram_fault`,
`ppu_nametable_mirroring_fault`, `ppu_sprite_zero_hit_fault`,
`ppu_sprite_overflow_fault`, `ppu_sprite_priority_fault`, `ppu_scroll_seam_fault`,
`joypad_strobe_reset_fault`,
`joypad_strobe_high_hold_fault`, `ppu_vram_increment_32_fault`,
`ppu_status_latch_reset_fault`,
`ppu_nmi_timeout_fault`, and
`timeout_cycle_limit`. The
observer JSON is the compact machine entry point: it turns the root attention
queue into ordered next actions, scenario observations, and evidence pointers so
an automated debugger can decide which artifact to open without traversing every
bundle first. The root manifest records each scenario's expected runner exit
code, expected health, expected focus test/domain, actual `debug_focus`, failed
probe ids, per-scenario baseline comparison summaries, explicit contract-match
breakdowns, single-scenario `replay_args`, a suite-level attention queue, and
artifact paths. The observer JSON mirrors the replay arguments on next actions
and observations so an AI debugger can regenerate one bundle before loading the
full telemetry corpus. The debug index adds one compact row per scenario with
the scenario role and outcome, health, focus domain, failure kind, failed probe
ids, terminal instruction or last event, top comparison difference, replay args,
and the first artifact to open. Use it as the first routing table when comparing
scenarios, choosing a replay, or handing a corpus to an automated debugger. The
observability analysis consumes that index and emits ranked focus-domain
hypotheses, health counts, scenario priority, suggested replay args, and the
first artifact to open for each candidate subsystem. Use it when an automated
debugger needs an aggregate cross-suite starting point before drilling into one
scenario. The coverage ledger answers whether the suite only contains happy
paths, maps negative fixtures back to cartridge tests and focus domains, and
keeps known untested risk areas attached to the uploaded artifact. The telemetry
catalog is the signal dictionary: use it to map failed probe ids, event kinds,
timeline entries, and instruction trace fields to the right JSON paths and
artifacts before opening full telemetry. The code map is the next routing layer:
use it after choosing a focus
domain to open the relevant emulator files, nearby regression tests, and focused
replay command without guessing where that domain lives in the repository. The
investigation plan is the executable routing layer: start with
`top_route`, open its primary artifact, replay the scenario, inspect the mapped
source/search terms, and run the listed narrow tests. When an automated debugger
already knows the scenario id, start with the scenario dossiers instead of
manually joining the debug index, telemetry catalog, code map, and route table.
The optional comparison artifact compares two observability suites by
scenario health/focus/probes/scores and hypothesis rank/score changes, then
reports matched, changed, or regressed verdicts with replay args and current-run
artifact pointers for any regression. Use `--fail-on-comparison-regression`
with `--compare-suite-dir` when the comparison should be a CI gate. The
`cpu_zero_page_wrap_fault`, `cpu_indirect_jmp_fault`,
`cpu_addressing_matrix_fault`, `input_port_matrix_fault`, and
`ppu_read_buffer_fault` scenarios use telemetry-visible fault injection to
corrupt deterministic CPU RAM and VRAM sentinels just before the cartridge
assertion reads them, while `dma_oam_transfer_fault` corrupts the host-observed
OAM DMA source byte before `$4014` and `apu_status_fault` disables `$4015`
just before the cartridge reads the APU status register.
`dma_phase_matrix_fault` stops the paired OAM DMA phase-matrix test before its
second cartridge-triggered transfer, proving odd/even start-phase regressions
localize to `dma.oam_phase_matrix`.
`mapper2_bank_switch_fault` switches the UXROM bank select back to bank 0 just
before the bank-1 sentinel read, proving mapper PRG bank-switch regressions
localize to `mapper.uxrom.prg_bank_switch`.
`mapper2_prg_ram_fault` corrupts the `$7FFF` PRG RAM sentinel just before the
cartridge reads it, proving Mapper 2 PRG RAM regressions localize to
`mapper.uxrom.prg_ram`.
`ppu_nametable_mirroring_fault` corrupts the `$2000/$2400` horizontal
nametable mirror pair before the cartridge reads it, proving mapper-declared
horizontal mirroring regressions localize to
`ppu.nametables.horizontal_mirroring`.
`ppu_sprite_zero_hit_fault` clears the deterministic sprite/background
collision observation before the cartridge asserts PPUSTATUS bit 6, proving
sprite-zero-hit regressions localize to `ppu.sprite_zero_hit`.
`ppu_sprite_overflow_fault` moves the post-eighth sprite candidates out of
range before the cartridge asserts PPUSTATUS bit 5, proving sprite-evaluation
overflow regressions localize to `ppu.sprite_overflow`.
`ppu_sprite_priority_fault` swaps the two overlapping sprite priority
attributes after OAM setup, proving sprite/background pixel-mux regressions
localize to `ppu.sprite_priority` through host-sampled frame colors.
`ppu_scroll_seam_fault` corrupts one side of a deterministic scroll-seam scene,
proving fine-X, coarse-X, coarse-X nametable-wrap, and vertical scroll
regressions localize to `ppu.scroll_seam` through host-sampled frame colors.
`joypad_strobe_reset_fault` consumes the reset A-button bit after a second
`$4016` strobe sequence, proving mid-stream joypad strobe reset regressions
localize to `joypad.strobe_reset`.
`input_mask_matrix_pass` runs joypad 1 with `0xAA` and joypad 2 with `0x55`,
`input_mask_all_released_pass` runs both ports with `0x00`, and
`input_mask_all_pressed_pass` runs both ports with `0xFF`.
`input_mask_joypad1_pressed_pass` runs joypad 1 with `0xFF` while joypad 2
stays `0x00`, and `input_mask_joypad2_pressed_pass` runs the inverse
`0x00`/`0xFF` mask pair. `input_mask_sparse_bits_pass` runs sparse
non-contiguous `0x81`/`0x18` masks, and `input_mask_nibble_split_pass` runs
complementary low/high nibble `0x0F`/`0xF0` masks. All seven fixtures set the
cartridge's expected masks to the same values. Together they prove
alternating, all-released, all-pressed, joypad-1-only pressed, joypad-2-only
pressed, sparse-bit, and nibble-split controller masks can be validated as
healthy fixtures without rebuilding the ROM.
`joypad_strobe_high_hold_fault` clears joypad 1's A button just before the
strobe-high hold test reads `$4016`, proving strobe-high read regressions
localize to `joypad.strobe_high_hold`.
`cpu_addressing_matrix_fault` corrupts the page-cross sentinel before the
generated cartridge exercises absolute,X and indirect,Y loads, proving CPU load
addressing regressions localize to `cpu.addressing.page_cross_load`.
`input_port_matrix_fault` clears joypad 2's Start button before the combined
input-port serial matrix, proving `$4016`/`$4017` strobe-high, serial-shift,
and overread regressions localize to `joypad.input_port_matrix`.
`ppu_vram_increment_32_fault` corrupts the `$2020` stride target after the
cartridge writes through `$2007` with PPUCTRL bit 2 set, proving PPUDATA
increment-by-32 regressions localize to `ppu.registers.ppudata_increment_32`.
`ppu_status_latch_reset_fault` leaves the PPUADDR latch half-written after the
cartridge has read PPUSTATUS, proving `$2002` latch-reset regressions localize
to `ppu.registers.status_latch_reset`.
`ppu_nmi_timeout_fault` disables PPU NMI delivery after the render-frame test
enables NMI, proving timeout localization can stay focused on `ppu.nmi` and the
active cartridge test. The suite can prove CPU addressing, CPU control-flow,
mapper PRG switching, mapper PRG RAM, PPU nametable mirroring, configurable
joypad mask-table fixtures, joypad strobe-reset behavior, joypad strobe-high hold behavior,
PPUDATA register increment behavior, PPUSTATUS write-latch reset behavior, DMA
host-observation, OAM DMA phase-matrix behavior, APU status, PPU assertion,
PPU sprite-zero-hit signaling, PPU sprite-overflow signaling, PPU
sprite-priority muxing, PPU scroll seams, and PPU progress-timeout failure
localization without requiring a broken emulator build. The Markdown reports add
suite analysis, observer next actions, an
attention queue, compact scenario matrices, contract matrix, baseline comparison
matrix, AI drilldown order, and bundle artifact maps for humans or agents
inspecting CI artifacts. The command exits `0` when all known-good and
intentionally failing scenarios match their expected debug-focus contracts. CI
and release workflows upload this corpus as `oxidenes-diagnostic-scenario-suite`.
Those workflows also resolve a prior commit when one is available, generate a
prior scenario suite in a temporary worktree, run the current observability
suite with `--compare-suite-dir` and `--fail-on-comparison-regression`, and
upload the prior corpus as `oxidenes-diagnostic-prior-scenario-suite`. On pull
requests the prior commit is the base SHA; on pushes it is the previous commit
from the event payload.

To validate a generated suite artifact before handing it to an automated
debugger or attaching it to a release, run:

```powershell
python scripts/verify_diagnostic_suite.py --suite-dir target/diagnostics/scenario-suite
```

The verifier checks the root schema versions, observer next actions,
observations, Markdown sections, and every artifact path referenced by the
manifest or observer report. CI and release workflows run this verifier before
uploading the scenario-suite artifact through `run_diagnostic_observability.py`.
CI and release workflows run `run_diagnostic_e2e.py` for the current suite so
the uploaded `oxidenes-diagnostic-scenario-suite` includes both the underlying
observability and route artifacts plus the root e2e acceptance report.

To validate the full observability wrapper output that AI tooling consumes,
including the debug index, aggregate analysis, coverage ledger, diagnostic code
map, telemetry catalog, diagnostic investigation plan, scenario dossiers,
optional cross-run comparison, focused replay evidence, and `observability-run.json`, run:

```powershell
python scripts/verify_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite
```

This verifier contract-checks the wrapper-level artifact graph after
`run_diagnostic_observability.py` completes. CI and release workflows run it
before uploading the current scenario-suite artifact so accepted diagnostic
corpora have both valid base suite files and valid AI-facing observability
summaries.

To execute one generated investigation route end to end, run:

```powershell
python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite
```

By default this selects `diagnostic-investigation-plan.json` `top_route`, runs
its replay command into `route-checks/<route>/replay-bundle/`, runs the route's
narrow diagnostic/subsystem test commands, and writes
`diagnostic-route-check.json` plus `diagnostic-route-check.md`. Use
`--rank`, `--route-id`, `--focus-domain`, or `--scenario-id` to execute a
specific route. To prove every generated route can regenerate focused replay
evidence, run:

```powershell
python scripts/run_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --all-routes --skip-tests --output-dir target/diagnostics/scenario-suite/route-replay-matrix
```

This writes `diagnostic-route-matrix.json` and `diagnostic-route-matrix.md`
alongside per-route `diagnostic-route-check.json`, `diagnostic-route-check.md`,
and `replay-bundle/` directories. CI and release workflows run this all-route
replay matrix after the observability verifier, then run the top-route command
with narrow tests so uploaded scenario-suite artifacts include both broad
route-replay proof and a full highest-ranked debug-route proof.

To validate the route evidence contract after those commands complete, run:

```powershell
python scripts/verify_diagnostic_route.py --suite-dir target/diagnostics/scenario-suite --require-matrix --require-top-route --expect-matrix-tests-skipped --write-summary
```

This verifier checks the route matrix schema/counts, every per-route replay
bundle referenced by the matrix, the top-route full check, expected-vs-actual
health/focus matches, required replay artifacts, and narrow-test command
results. With `--write-summary` it also writes
`diagnostic-route-evidence-verification.json` and
`diagnostic-route-evidence-verification.md` at the scenario-suite root with the
accepted verdict, matrix/top-route artifact pointers, verifier configuration,
errors, and AI handoff notes. CI and release workflows run it before uploading
the scenario-suite artifact so automated consumers can trust the accepted route
evidence without replaying the cartridge again.

`run_diagnostic_observability.py` also writes the root
`diagnostic-debug-index.jsonl`, `diagnostic-debug-index.md`,
`diagnostic-observability-analysis.json`, and
`diagnostic-observability-analysis.md` files, plus
`diagnostic-coverage-ledger.json`, `diagnostic-coverage-ledger.md`,
`diagnostic-telemetry-catalog.json`, `diagnostic-telemetry-catalog.md`,
`diagnostic-code-map.json`, `diagnostic-code-map.md`,
`diagnostic-investigation-plan.json`, `diagnostic-investigation-plan.md`,
`diagnostic-scenario-dossiers.json`, and `diagnostic-scenario-dossiers.md`.
`run_diagnostic_e2e.py` additionally writes `diagnostic-e2e-report.json`,
`diagnostic-e2e-report.md`, `diagnostic-ai-observability-index.json`, and
`diagnostic-ai-observability-index.md` after the observability and route
evidence gates finish. It then runs
`build_diagnostic_ai_coverage_gap_plan.py` and writes
`diagnostic-ai-coverage-gap-plan.json` plus
`diagnostic-ai-coverage-gap-plan.md`, proving the known coverage gaps are
ranked, mapped to current source/test anchors, linked to telemetry signals, and
ready for the next cartridge test-design pass. It then runs
`query_diagnostic_ai_index.py smoke` and
writes `diagnostic-ai-query-smoke.json` plus `diagnostic-ai-query-smoke.md`.
It also runs `run_diagnostic_ai_diagnosis.py` for the top route and writes
`diagnostic-ai-diagnosis-smoke.json` plus
`diagnostic-ai-diagnosis-smoke.md`, proving the compact index can drive an
end-to-end replay/test diagnosis handoff. It then runs
`build_diagnostic_ai_fix_handoff.py` and writes
`diagnostic-ai-fix-handoff-smoke.json` plus
`diagnostic-ai-fix-handoff-smoke.md`, proving the selected route has concrete
source/test anchors and bounded fix commands. It then runs
`run_diagnostic_ai_route_matrix.py` and writes
`diagnostic-ai-route-matrix.json` plus `diagnostic-ai-route-matrix.md`, proving
every AI route can regenerate diagnosis and fix-handoff artifacts. It then runs
`build_diagnostic_ai_debug_packet.py` and writes
`diagnostic-ai-debug-packet.json`, `diagnostic-ai-debug-packet.md`, and
`ai-debug-packet/`, proving the selected route can be consumed as one
relocatable packet with copied evidence and source/test context. It then runs
`verify_diagnostic_ai_debug_packet.py` and writes
`diagnostic-ai-debug-packet-verification.json` plus
`diagnostic-ai-debug-packet-verification.md`, proving the selected packet is
self-verifiable after it is copied away from the suite. It then runs
`run_diagnostic_ai_debug_packet_matrix.py` and writes
`diagnostic-ai-debug-packet-matrix.json` plus
`diagnostic-ai-debug-packet-matrix.md`, proving every accepted AI route can be
consumed as a relocatable packet with copied evidence, source/test context, and
packet-local verification evidence. It then runs
`evaluate_diagnostic_ai_localization.py` and writes
`diagnostic-ai-localization-eval.json` plus
`diagnostic-ai-localization-eval.md`, scoring whether every scenario matches
its expected health/focus-domain contract and whether every negative fixture
has route evidence, source/test anchors, and packet self-verification.
It then runs `build_diagnostic_ai_session_plan.py` and writes
`diagnostic-ai-session-plan.json` plus `diagnostic-ai-session-plan.md`,
turning all 23 accepted AI routes into deterministic debugger startup plans
with ordered artifacts, replay commands, narrow tests, verification commands,
and stop conditions.
It then runs `run_diagnostic_ai_session_smoke.py` and writes
`diagnostic-ai-session-smoke.json` plus `diagnostic-ai-session-smoke.md`,
proving the selected startup plan route is executable by an automated consumer
before the artifact graph is trusted.
It then runs `run_diagnostic_ai_session_smoke_matrix.py` and writes
`diagnostic-ai-session-smoke-matrix.json`,
`diagnostic-ai-session-smoke-matrix.md`, and `ai-session-smoke-matrix/`,
proving every accepted startup plan route is executable by the same consumer
path.
It then runs `verify_diagnostic_ai_artifacts.py` and writes
`diagnostic-ai-artifact-verification.json` plus
`diagnostic-ai-artifact-verification.md`, proving the AI-facing artifact graph
is internally consistent before automated debugger or fix loops consume it.
The AI index is the smallest single artifact for automated debuggers that need
to join scenario health, failed probes, start-artifact pointers, replay
arguments, mapped source files, narrow tests, and known coverage limits. The AI
coverage gap plan is the first artifact to open when extending the diagnostic
cartridge itself: it keeps known missing fixture classes explicit, mapped to
current code/tests, and backed by concrete telemetry and validation commands.
The query smoke proves the accepted artifact can be interrogated by scenario id,
focus domain, failed probe id, top route, and coverage posture. The diagnosis
smoke proves one selected route can be regenerated into fresh telemetry and
bounded test results without hand-joining the underlying artifacts. The fix
handoff smoke proves that diagnosis can be translated into code-inspection
anchors and exact regression commands. The AI route matrix proves the same
diagnosis and fix-handoff path works for every focus-domain route. The AI debug
packet gives an automated debugger one selected-route packet with compact
evidence, replay telemetry, source/test context, and fix commands. The packet
self-verifier proves that the selected packet is internally consistent after it is
copied away from the suite. The AI debug packet matrix proves that every route
has the same packet-level handoff quality instead of only the top route. The AI
localization evaluation proves the corpus is not only present but scoring at
the expected localization quality across healthy controls and intentional
negative fixtures. The AI session plan gives automated debuggers one compact
route-by-route startup contract with read order, commands, and stop conditions.
The AI session smoke proves the selected startup contract can drive replay and
narrow-test execution before a debugger edits emulator code.
The AI session smoke matrix proves that same executable startup contract for
every accepted route, not only the top route.
The AI artifact verification proves the uploaded bundle can be trusted as a
coherent graph before an automated debugger starts making emulator changes,
including the coverage gap plan's ready/mapped gap counts. Its
`automation_readiness` section gives automated agents one compact
route-by-route readiness map after a CI artifact is downloaded.
With
`--compare-suite-dir <DIR>` it also writes `diagnostic-observability-comparison.json` and
`diagnostic-observability-comparison.md`, and with
`--fail-on-comparison-regression` it exits non-zero when the current run
regresses against that prior suite. CI and release workflows use that flag when
a prior suite can be generated, so cross-run diagnostic regressions fail the
diagnostic bundle job before artifacts are accepted. It then writes focused
replay evidence under
`replay-runs/<scenario>/` by default. The replay summary records the source
`replay_args`, effective command, expected and actual exit code, expected and
actual health/focus values, required bundle-artifact presence, and paths to the
focused bundle's `manifest.json`, `triage.json`, `telemetry.json`, `report.md`,
and `diagnostic.nes`. Use `--replay-scenario <ID>` to target a specific
scenario, `--replay-output-dir <DIR>` to place the focused evidence elsewhere,
or `--skip-replay` when only suite generation and verification are needed.

## Coverage

The cartridge exercises the emulator through the normal CPU, bus, cartridge, PPU, APU, DMA, and joypad paths:

- CPU arithmetic and flags
- Stack, `JSR`, and `RTS`
- Zero-page indexed read/write wraparound
- Indirect `JMP ($xxFF)` page-wrap behavior
- 2 KiB CPU RAM mirroring
- PPU palette register write/read
- PPU non-palette PPUDATA read buffering, PPUCTRL-driven increment-by-32
  behavior, and PPUSTATUS write-latch reset behavior
- PPU horizontal nametable mirroring through CPU-driven PPUDATA reads
- PPU sprite-zero-hit signaling through a deterministic sprite/background
  overlap scene
- PPU sprite-overflow signaling through nine in-range sprites on one scanline
  plus hardware-bug false-positive and false-negative subcases
- OAM DMA from CPU page `$0300`, including host-observed CPU stall cycle bucket
  plus DMC sample-DMA overlap timing, stall-phase, and placement telemetry
- APU pulse-channel status register plus host-observed sample-count, peak,
  RMS, and mean absolute output envelope checks
- Mapper 1/MMC1 serial-register PRG bank switching, delayed commit after four
  writes, 4 KiB CHR bank switching, fixed-last PRG reads, 32 KiB PRG modes
  0/1 with ignored low PRG bank bit, and single-screen lower/upper nametable
  mirroring
- Mapper 2/UXROM PRG bank switching, fixed final-bank reads, and PRG RAM round-trips
- Mapper 3/CNROM CHR bank switching through CPU bank-select writes and
  PPU-visible pattern-table reads
- Mapper 4/MMC3 PRG R6/R7 bank switching, fixed-last PRG reads, 2 KiB and
  1 KiB CHR bank switching, horizontal/vertical mirroring control, and
  scanline IRQ delivery, plus a fixed-`$E000` edge variant for PRG-mode
  inversion, CHR inversion, and IRQ reload phases, and a battery-backed PRG
  RAM variant that validates `$6000-$7FFF` writes plus host SRAM restore
- Mapper 7/AxROM 32 KiB PRG bank switching and single-screen lower/upper
  nametable mirroring through CPU and PPU bus paths
- Joypad strobe and shift reads with configurable expected masks
- Joypad mid-stream strobe reset behavior
- Joypad strobe-high hold behavior
- Combined `$4016`/`$4017` strobe-high, serial-shift, and overread matrix
- Generated `$4016`/`$4017` input-mask sweep variant across 16 host-applied
  controller mask pairs
- Taken CPU branch crossing a page boundary
- Joypad reads after the eighth latched button
- PPU NMI delivery, host-observed vblank timing windows, PPUSTATUS
  vblank set/clear dot-edge timing, rendered frame production, and an expected
  deterministic full-frame render checksum
- Player-2 `$4017` strobe and shift reads with an independent Start + Down mask

## Telemetry Protocol

The top-level JSON includes a `schema_version` and `suite` envelope for
automated consumers. Each test result includes its subsystem, tier, intent,
expected observations, result byte address, raw result byte, and pass/fail
state. Event records include a normalized event kind plus the active test name
when the current test id is known.

Schema version `2` added a failure catalog and structured
`verdict.failure` object. When the cartridge reports a failing assertion, the
failure object maps the raw failure code to the failing test, subsystem,
assertion, expected observation, observed mismatch, likely emulator domain, and
remediation hint. This keeps intentional or real failures focused on the first
localized cartridge assertion instead of mixing in downstream host checks that
could not run after the early stop.

Schema version `3` adds an `analysis` object for automated consumers. It
summarizes health, test/subsystem/tier coverage, first-failure domain, failing
test/subsystem, event transition count, and next debugging actions. This is
derived from the raw telemetry so an AI or CI report can start with a compact
diagnostic summary and drill into raw events only when needed.

Schema version `4` adds a top-level `timeline` array and an
`analysis.timing` summary. Each timeline entry maps one diagnostic test to its
observed start/end cycles, frame span, duration, outcome, end reason, terminal
status byte, and terminal PC. This makes slow tests, skipped tests, assertion
failures, and mid-test timeouts explicit without forcing automated consumers to
reconstruct timing from raw events first.

Schema version `5` adds top-level `probes` and `analysis.probe_summary`.
Probes normalize each cartridge result byte and host-observed emulator signal
into `passed`, `failed`, or `skipped` records with expected value, observed
value, subsystem, test context, and likely domain. This lets CI or an AI
debugger rank concrete failed observations without scraping report text or
guessing whether downstream checks were merely not reached after an early
cartridge assertion.

The cartridge writes status bytes into CPU RAM:

- `$00F0`: status, `0x01` running, `0x80` pass, `0xE0` fail
- `$00F1`: current test id
- `$00F2`: failure code
- `$00F3`: signature byte, expected `0xA5`
- `$00F4`: NMI count
- `$0200..`: per-test result slots, `0x01` means pass

The host runner adds emulator-side telemetry that the cartridge cannot inspect
directly: CPU registers, frame count, RAM checksum, OAM checksum,
rendered-frame expected/observed checksum and color counts, APU status-matrix
bits, audio sample count/peak/RMS/mean envelope, status/frame events, current-test transition events, a bounded
instruction-boundary trace, failure-localization metadata, per-test
timeline/duration telemetry, structured observation probes, and a derived
analysis summary.

Schema version `6` adds `analysis.coverage_gaps` and includes the same
high-level limits in `triage.json`. These records name known untested risk
areas, what the generated cartridge currently covers, what it does not prove,
and the next diagnostic cartridge that should be built. Passing diagnostics
therefore remain useful without implying full CPU, PPU, mapper, APU, DMA, or
input compatibility.

Schema version `7` adds top-level `input` telemetry and an executable joypad-2
diagnostic test. The input record includes the actual and expected joypad masks
for both ports, and the cartridge now reads `$4017` after a shared `$4016`
strobe to prove player-2 serial input reaches the CPU bus independently from
player 1.

Schema version `8` adds top-level `dma` telemetry for the cartridge's OAM DMA
test. The host runner records whether OAM DMA was observed, whether it
completed, the active CPU-stall cycle count, the expected 513-514 cycle bucket,
start/end cycles, and the associated diagnostic test. This turns DMA evidence
from a final OAM checksum into timing-aware telemetry that automated debuggers
can compare against baselines.

Schema version `9` makes the OAM DMA test exercise DMC interaction instead of
only the clean transfer path. The cartridge primes fastest-rate DMC playback
before starting OAM DMA, and the host runner records first active-cycle parity,
DMC sample fetch counts, whether a DMC fetch was serviced during the OAM DMA
window, the overlap cycle/test context, and the queued/post-OAM DMC stall
cycles. The report, triage JSON, probes, and baseline comparison all expose
these fields so timing regressions can be localized to DMA interleaving rather
than reduced to a final OAM checksum.

Schema version `10` makes DMC sample-DMA stall timing phase-aware. DMC DMA
service now records the CPU cycle parity used for the fetch and assigns the
expected 3-cycle or 4-cycle CPU stall bucket from that phase. The diagnostic
bundle reports the first DMC fetch parity/bucket, the first DMC/OAM overlap
parity/bucket, 3-cycle versus 4-cycle fetch counts, and verifies that the
post-OAM DMC stall window matches the overlap service bucket.

Schema version `11` adds execution snapshots to each diagnostic event. Event
records now include CPU registers, pending CPU cycles, and diagnostic RAM
watchpoints such as failure code, signature, NMI count, and the active test
result byte. The compact triage JSON event tail exposes the same snapshot data
so automated debuggers can start from the final transitions without loading the
full event stream, and baseline comparison warns when final CPU or diagnostic
RAM state drifts from a known-good run.

Schema version `12` adds top-level `instruction_trace` telemetry. The host
runner samples instruction-boundary CPU state before opcode fetches, skips
OAM DMA and DMC stall cycles, records the PRG opcode when the PC points into
cartridge space, and retains the final 64 entries as a bounded tail. The report
and compact triage JSON expose the last 16 entries so automated debuggers can
start from the final executed instructions, while baseline comparison warns
when retained trace counts drift from a known-good run.

Schema version `13` makes instruction trace tails self-describing. Each trace
entry now includes decoded instruction text, mnemonic, addressing mode, operand
bytes, and nearest diagnostic cartridge symbol plus offset. The generated
cartridge labels each test start and control-flow helper, so a failing run can
point automated debuggers at locations such as `test_07_joypad_strobe_shift`
or `hang` without requiring separate disassembly of `diagnostic.nes`.

Schema version `14` adds `analysis.debug_focus`, a derived first-stop triage
object for AI debuggers and CI reports. It records the focus test/subsystem,
likely domain, failure kind, failed probe ids, skipped probe count, final event,
terminal instruction, and last instruction associated with the focus test.
`triage.json` exposes the same object at top level as `debug_focus` so tools can
choose the first drilldown target before reading the full event stream or trace
tail.

Schema version `15` adds the `cpu_zero_page_index_wrap` edge-case cartridge
test. It verifies that `LDA $FF,X` and `STA $FF,X` with `X=0x81` wrap to `$0080`
inside page zero, and adds failure-catalog domains for zero-page indexed CPU
addressing regressions.

Schema version `16` adds the `cpu_indirect_jmp_page_wrap` edge-case cartridge
test. It verifies that `JMP ($04FF)` reads the high byte from `$0400`, matching
the original 6502 indirect-jump page-wrap behavior, and gives AI triage a
specific `cpu.control_flow.indirect_jmp_page_wrap` failure domain.

Schema version `17` adds the `ppu_vram_read_buffer` edge-case cartridge test. It
verifies that non-palette `$2007` reads are delayed through the PPU read buffer
and auto-increment across `$2000`/`$2001`, giving AI triage specific
`ppu.registers.ppudata_buffer` and `ppu.registers.ppudata_increment` failure
domains.

Schema version `18` adds telemetry-visible diagnostic fault injection and the
`ppu_read_buffer_fault` scenario-suite fixture. The fixture keeps the generated
cartridge IP-safe and deterministic while proving that AI handoff artifacts can
localize a PPUDATA read-buffer assertion to `ppu.registers.ppudata_buffer`.

Schema version `19` adds the `cpu_zero_page_wrap_fault` scenario-suite fixture.
The fixture injects a zero-page sentinel mismatch just before `LDA $FF,X` in the
generated cartridge, proving that AI handoff artifacts can localize CPU
zero-page indexed wrap failures to `cpu.addressing.zero_page_x_wrap`.

Schema version `20` adds the `cpu_indirect_jmp_fault` scenario-suite fixture.
The fixture injects an indirect-JMP pointer high-byte mismatch just before
`JMP ($04FF)` in the generated cartridge, proving that AI handoff artifacts can
localize CPU control-flow page-wrap failures to
`cpu.control_flow.indirect_jmp_page_wrap`.

Schema version `21` adds the `dma_oam_transfer_fault` scenario-suite fixture and
host-validation debug-focus localization from the first failed probe. The fixture
corrupts the `$0300` OAM DMA source byte just before `$4014`, leaving the
cartridge assertions passing while host telemetry reports `oam.dma_checksum` and
focuses AI triage on `dma.oam_transfer`.

Schema version `22` adds the `apu_status_fault` scenario-suite fixture. The
fixture disables `$4015` just before the `apu_status_register` cartridge test
reads the APU status register, proving that AI handoff artifacts localize the
assertion to `apu.status` and failure code `0x61`.

Schema version `23` adds the `ppu_nmi_timeout_fault` scenario-suite fixture and
active-test timeout localization. The fixture disables PPUCTRL NMI delivery
after `ppu_nmi_and_render_frame` enables rendering, causing the cartridge to
stall in its NMI wait loop. Telemetry now keeps that timeout focused on
`ppu.nmi`, marks `ppu.nmi_count` and `ppu.vblank_timing.nmi_window` failed, and
preserves the stalled loop symbol in the instruction trace tail.

Schema version `24` changes the generated diagnostic cartridge to Mapper 2/UXROM
and adds the `mapper2_prg_bank_switch` integration test plus the
`mapper2_bank_switch_fault` scenario-suite fixture. The cartridge executes from
the fixed final PRG bank, writes mapper-select values through `$8000`, verifies
distinct switchable-bank sentinels at `$8000`, and verifies the fixed-bank
sentinel at `$FF00`. The intentional fault fixture localizes bank-select
regressions to `mapper.uxrom.prg_bank_switch`.

Schema version `25` adds the `mapper2_prg_ram_roundtrip` integration test plus
the `mapper2_prg_ram_fault` scenario-suite fixture. The cartridge writes
sentinels to `$6000` and `$7FFF`, verifies both boundaries through the CPU
cartridge RAM window, changes Mapper 2 bank select through `$8000`, and verifies
the lower sentinel still persists. The intentional fault fixture corrupts the
upper sentinel before the read and localizes PRG RAM regressions to
`mapper.uxrom.prg_ram`.

Schema version `26` adds the `ppu_horizontal_nametable_mirroring` integration
test plus the `ppu_nametable_mirroring_fault` scenario-suite fixture. The
cartridge writes nametable sentinels through PPUDATA, verifies that `$2400`
mirrors `$2000`, verifies that `$2C00` mirrors `$2800`, and verifies the two
horizontal mirror pairs remain independent. The intentional fault fixture
corrupts the first mirror pair and localizes mirroring regressions to
`ppu.nametables.horizontal_mirroring`.

Schema version `27` adds the `joypad_strobe_reset_midstream` edge-case cartridge
test plus the `joypad_strobe_reset_fault` scenario-suite fixture. The cartridge
reads `$4016`, toggles strobe high then low mid-stream, verifies the next read
returns the A-button bit again, and verifies serial advancement resumes at the B
button. The intentional fault fixture consumes the reset A bit and localizes
reset-index regressions to `joypad.strobe_reset`.

Schema version `28` adds the `ppu_vram_increment_32` edge-case cartridge test
plus the `ppu_vram_increment_32_fault` scenario-suite fixture. The cartridge
sets PPUCTRL bit 2, verifies sequential `$2007` writes stride from `$2000` to
`$2020`, then clears bit 2 and verifies `$2007` returns to increment-by-1
behavior. The intentional fault fixture corrupts the `$2020` stride sentinel and
localizes PPU register increment regressions to
`ppu.registers.ppudata_increment_32`.

Schema version `29` adds the `ppu_status_latch_reset` edge-case cartridge test
plus the `ppu_status_latch_reset_fault` scenario-suite fixture. The cartridge
leaves PPUADDR in a half-written state, reads PPUSTATUS, then verifies the next
PPUADDR high/low pair writes PPUDATA to `$2100`. The intentional fault fixture
re-enters the half-written latch state before the address pair and localizes
PPUSTATUS latch-reset regressions to `ppu.registers.status_latch_reset`.

Schema version `30` makes joypad expected masks explicit runtime inputs, adds
the `joypad_strobe_high_hold` edge-case cartridge test, and adds the
`input_mask_matrix_pass` plus `joypad_strobe_high_hold_fault` scenario-suite
fixtures. The healthy input fixture validates non-default `0xAA`/`0x55`
controller masks without rebuilding the ROM. The edge-case test verifies
repeated `$4016` reads while strobe is high keep returning the configured A bit
and that the first post-strobe-low serial read still starts at A. The intentional
fault fixture localizes strobe-high hold regressions to
`joypad.strobe_high_hold`.

Schema version `31` adds the `cpu_addressing_mode_matrix` edge-case cartridge
test plus the `cpu_addressing_matrix_fault` scenario-suite fixture. The
generated cartridge records absolute,X no-cross, absolute,X page-cross, and
indirect,Y page-cross load results in RAM, exposes them as
`cpu_addressing_matrix` telemetry, and localizes page-cross load regressions to
`cpu.addressing.page_cross_load`.

Schema version `32` adds the `input_port_serial_matrix` edge-case cartridge
test plus the `input_port_matrix_fault` scenario-suite fixture. The generated
cartridge records both ports' strobe-high and overread observations in RAM,
asserts all eight serial bits for `$4016` and `$4017` against configured masks,
exposes `input_port_matrix` telemetry and a `joypad.input_port_matrix.results`
probe, and localizes combined input-port regressions to
`joypad.input_port_matrix`.

Schema version `33` adds the `oam_dma_phase_matrix` edge-case cartridge test
plus the `dma_phase_matrix_fault` scenario-suite fixture. The generated
cartridge triggers two additional `$4014` OAM DMA transfers, host telemetry
records per-transfer active-cycle buckets and start parities, and the
`dma.oam_phase_matrix` probe verifies the accepted run covers both 513-cycle
and 514-cycle OAM DMA phases.

Schema version `34` adds the `ppu_sprite_zero_hit` cartridge test plus the
`ppu_sprite_zero_hit_fault` scenario-suite fixture. The generated cartridge
builds a deterministic visible overlap between solid background tile 2 and
sprite 0 tile 2, asserts PPUSTATUS bit 6 after two vblanks, records
`ppu_sprite_zero_hit` telemetry, and exposes `cartridge.test.25.result` plus
`ppu.sprite_zero_hit.status` probes for automated localization.

Schema version `35` adds the `ppu_sprite_overflow` cartridge test plus the
`ppu_sprite_overflow_fault` scenario-suite fixture. The generated cartridge
places nine synthetic sprites on one visible scanline, asserts PPUSTATUS bit 5
after sprite evaluation completes, restores the full OAM page from the DMA
source pattern before reporting pass or fail, records `ppu_sprite_overflow`
telemetry, and exposes `cartridge.test.26.result` plus
`ppu.sprite_overflow.status` probes for automated localization.

Schema version `36` adds the `ppu_sprite_priority_mux` cartridge test plus the
`ppu_sprite_priority_fault` scenario-suite fixture. The generated cartridge
builds two deterministic background/sprite overlaps, verifies one front-priority
sprite sample and one behind-background sample through host-observed frame
pixels, records `ppu_sprite_priority` telemetry, and exposes
`cartridge.test.27.result` plus `ppu.sprite_priority.samples` probes for
automated localization.

Schema version `37` adds the `ppu_fine_x_scroll_seam` cartridge test plus the
`ppu_scroll_seam_fault` scenario-suite fixture. The generated cartridge builds a
deterministic horizontal background tile seam, scrolls it by fine X, verifies
left and right seam samples through host-observed frame pixels, records
`ppu_scroll_seam` telemetry, and exposes `cartridge.test.28.result` plus
`ppu.scroll_seam.samples` probes for automated localization.

Schema version `38` renames that test to `ppu_scroll_seam_matrix` and expands
it into a four-sample scroll seam matrix. The same deterministic frame now
checks left/right fine-X samples plus top/bottom vertical-scroll samples, so
the PPU pixel-pipeline coverage gap no longer lists vertical scroll seams as
missing.

Schema version `39` adds top-level `ppu_vblank_timing` telemetry for the
existing `ppu_nmi_and_render_frame` cartridge test. The host records the CPU
cycle when the cartridge enters the render-enabled NMI wait loop, the first NMI
cycle, the second NMI cycle, the first-NMI latency, and the inter-NMI interval,
then exposes `ppu.vblank_timing.nmi_window` as a pass/fail probe.

Schema version `40` expands the `ppu_scroll_seam_matrix` host-observed frame
samples from four to six. The cartridge now renders the existing fine-X and
vertical seam phase, then renders a coarse-X phase with an 8-pixel horizontal
scroll so `ppu_scroll_seam` telemetry records coarse-left and coarse-right
tile-shift samples in addition to the existing left/right/top/bottom samples.

Schema version `41` adds a vertical-mirroring scroll-wrap variant cartridge to
the host diagnostic run. The main diagnostic cartridge remains Mapper 2 with
horizontal mirroring so it still covers horizontal nametable aliasing. The
variant renders tile 31 of `$2000` and tile 0 of `$2400` with contrasting
patterns, scrolls X by 248 pixels, and records nametable-wrap left/right frame
samples, variant mirroring, frames, cycles, pass status, and any variant error
inside `ppu_scroll_seam` telemetry.

Schema version `42` expands `ppu_vblank_timing` from coarse CPU-cycle NMI
windows into dot-edge evidence. The diagnostic host samples the PPU before and
after each PPU tick while `ppu_nmi_and_render_frame` is active, records the
first two PPUSTATUS vblank set edges at scanline 241 dot 1, records the first
pre-render clear edge at scanline -1 dot 1, retains total set/clear edge and
NMI-trigger counts, and exposes `ppu.vblank_timing.edge_dots` as a pass/fail
probe alongside `ppu.vblank_timing.nmi_window`.

Schema version `43` expands `ppu_sprite_overflow` into a three-case cartridge
matrix. The test still records the direct nine-sprites-on-one-scanline overflow
case, then adds a false-positive scene where the hardware-bug evaluator reads a
later sprite tile byte as a Y coordinate, and a false-negative scene where an
out-of-range sprite shifts the evaluator away from a real ninth sprite's Y byte.
Telemetry now records each subcase status bit, and the
`ppu.sprite_overflow.hardware_bug_matrix` probe separates hardware-bug behavior
from the broader sprite-overflow status probe.

Schema version `44` promotes the host-observed deterministic render frame into
an expected-vs-observed PPU signature. Top-level `frame` telemetry now records
the expected checksum, observed checksum, hex forms, match status, validation
mode, expected unique color count, and expected nonzero pixel count. The
`ppu.frame_checksum` probe fails when the canonical default no-fault fixture's
rendered full frame drifts from the accepted diagnostic signature. Non-default
input-timing fixtures report the probe as validation-disabled while still
passing and recording their observed checksums; intentional fault fixtures keep
the probe out of the failure focus so the intended fault remains localized.

Schema version `45` expands top-level `audio` telemetry from sample count and
peak into a bounded APU output envelope. The host runner now records expected
sample-count, peak, RMS, and mean absolute windows plus pass booleans, and adds
the `apu.output_envelope` probe so silent, clipped, or obviously unstable audio
output can fail as a direct expected-vs-observed observation.

Schema version `46` expands the `apu_status_register` cartridge test from a
pulse-1 status bit into a non-DMC `$4015` channel matrix. The cartridge now
programs pulse 1, pulse 2, triangle, and noise length counters, records the
observed bits 0-3 mask plus case count in RAM, exposes top-level
`apu_status_matrix` telemetry, and adds the `apu.status_matrix` probe for
expected-vs-observed APU status evidence.

Schema version `47` adds DMC status-register evidence to the existing
OAM/DMC overlap path. After the cartridge primes DMC sample playback and before
it runs the OAM DMA transfer, it records `$4015` bit 4 plus a case count in RAM,
exposes top-level `apu_dmc_status` telemetry, and adds the `apu.dmc_status`
probe so DMC-active status regressions localize separately from the non-DMC
channel matrix.

Schema version `48` adds DMC/OAM overlap placement telemetry. The host runner
records the OAM-transfer index, offset inside the active OAM DMA stall window,
beginning/middle/end bucket, covered buckets, missing buckets, and
`dma.dmc_overlap_placement` probe. At introduction, the cartridge covered
beginning and end placements and kept the missing middle-placement case
explicit in the DMA coverage gap instead of implying full interleaving
coverage.

Schema version `49` extends the post-render `oam_dma_phase_matrix` edge-case
test. After its odd/even OAM phase checks, the cartridge primes DMC playback,
triggers paired OAM DMA transfers, and aligns the second transfer so a DMC
sample fetch lands in the middle third of the OAM DMA stall window. The
accepted run must now cover beginning, middle, and end overlap buckets; the
remaining DMA coverage gap at that schema was repeated burst trains and longer
CPU/APU interleaving sequences.

Schema version `50` extends the same DMC-active `oam_dma_phase_matrix`
sequence into a three-transfer OAM DMA burst train. Telemetry now reports the
phase-matrix transfer indices that observed DMC/OAM overlap, the distinct
phase-matrix overlap transfer count, the expected minimum transfer count, and
the `dma.dmc_overlap_burst_train` probe.

Schema version `51` adds a generated Mapper 3/CNROM variant cartridge to the
host diagnostic run. The variant writes CHR bank values `0` through `3` to
`$8000`, performs buffered PPUDATA reads from CHR address `$0010`, records the
four observed bank-specific sentinel bytes in RAM, and exposes top-level
`mapper3_chr_bank` telemetry plus the `mapper3.chr_bank_switch` probe.

Schema version `52` adds a generated input-mask sweep variant cartridge to the
host diagnostic run. The variant receives joypad-1 and joypad-2 expected masks
from the host, strobes `$4016`, reconstructs both ports' eight serial bits into
RAM, compares the reconstructed bytes to the expected masks, and exposes
top-level `input_mask_sweep` telemetry plus the
`joypad.input_mask_sweep.results` probe across 16 stratified mask pairs.

Schema version `53` adds a generated Mapper 7/AxROM variant cartridge to the
host diagnostic run. Because AxROM swaps the whole 32 KiB CPU window, the
variant replicates the diagnostic program and vectors in every 32 KiB bank,
then writes bank-select values through `$8000`, reads PRG sentinels from
`$8000`, toggles single-screen lower/upper mirroring through bit 4, and exposes
top-level `mapper7_axrom` telemetry plus the `mapper7.axrom_switching` probe.

Schema version `54` adds a generated Mapper 1/MMC1 variant cartridge to the
host diagnostic run. The variant writes MMC1 serial-register bits through
`$8000`, `$A000`, `$C000`, and `$E000`, proves a PRG bank write does not commit
after only four serial bits, commits the fifth bit, reads switchable and fixed
PRG sentinels, reads 4 KiB CHR bank sentinels through PPUDATA, toggles
single-screen lower/upper mirroring, and exposes top-level `mapper1_mmc1`
telemetry plus the `mapper1.mmc1_shift_register` probe.

Schema version `55` adds a generated Mapper 4/MMC3 variant cartridge to the
host diagnostic run. The variant writes MMC3 bank-select/data registers for
R6/R7 PRG banking plus R0/R1/R2/R3 CHR banking, reads PRG and CHR sentinels
through CPU and PPUDATA paths, toggles horizontal and vertical mirroring through
`$A000`, enables rendering long enough for the PPU to clock the MMC3 scanline
counter, observes one mapper IRQ through the CPU IRQ vector, and exposes
top-level `mapper4_mmc3` telemetry plus the `mapper4.mmc3_banks_irq` probe.

Schema version `56` adds a second generated Mapper 4/MMC3 edge variant. This
variant runs from the fixed `$E000-$FFFF` PRG window so it can safely set MMC3
PRG inversion, read the fixed second-last bank at `$8000`, R7 at `$A000`, and
R6 at `$C000`, then sets CHR inversion and reads all eight 1 KiB CHR windows
through PPUDATA. It also runs two IRQ reload phases, including a zero-latch
reload, and exposes top-level `mapper4_mmc3_edge` telemetry plus the
`mapper4.mmc3_inversion_irq_reload` probe.

Schema version `57` adds a second generated Mapper 1/MMC1 PRG variant for
32 KiB PRG modes. The variant copies its diagnostic program and vectors into
both 32 KiB bank pairs, writes MMC1 control values for PRG modes 0 and 1, reads
paired sentinels at `$8000` and `$E000`, proves odd PRG bank writes ignore bit 0
in 32 KiB mode, and exposes top-level `mapper1_mmc1_32k_prg` telemetry plus the
`mapper1.mmc1_32k_prg_mode` probe.

Schema version `58` adds a generated Mapper 4/MMC3 battery-backed PRG RAM
variant. The variant declares the iNES battery flag, writes and reads `$6000`,
`$67FF`, and `$7FFF`, rewrites `$6000` to prove mutability, then the host runner
captures mapper SRAM and restores it into a fresh cartridge. It exposes top-level
`mapper4_mmc3_prg_ram` telemetry plus the
`mapper4.mmc3_prg_ram_persistence` probe.

Scenario suite schema version `8` and observer schema version `2` add
`replay_args` arrays for each scenario, observer action, and observation. These
arguments call `cargo run --bin oxidenes-diagnostic -- --bundle-dir target/diagnostics/replay/<scenario>`
with the exact cycle, joypad, expected mask, and fault-injection settings needed
to regenerate that one bundle.
