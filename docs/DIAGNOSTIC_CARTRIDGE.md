# OxideNES Diagnostic Cartridge

OxideNES includes a generated, IP-safe diagnostic cartridge for headless emulator validation. The cartridge is assembled from deterministic 6502 instructions and CHR byte patterns at runtime; no `.nes` file or third-party ROM content is committed. The generated ROM uses Mapper 2/UXROM so the same cartridge can validate PRG bank switching and PRG RAM access through the normal CPU bus.

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
handoff steps into executable debug routes. When `--compare-suite-dir <DIR>` is supplied, it also writes
`diagnostic-observability-comparison.json` and
`diagnostic-observability-comparison.md`. The root scenario-suite files are
`scenario-suite.json`, `scenario-suite.md`, `scenario-suite-observer.json`, and
`scenario-suite-observer.md`. The suite also writes one full bundle per
scenario: `pass`, `input_mask_matrix_pass`,
`joypad1_mismatch`, `joypad2_mismatch`, `dma_oam_transfer_fault`,
`apu_status_fault`, `cpu_zero_page_wrap_fault`, `cpu_indirect_jmp_fault`,
`ppu_read_buffer_fault`, `mapper2_bank_switch_fault`, `mapper2_prg_ram_fault`,
`ppu_nametable_mirroring_fault`, `joypad_strobe_reset_fault`,
`joypad_strobe_high_hold_fault`, `ppu_vram_increment_32_fault`,
`ppu_status_latch_reset_fault`, `ppu_nmi_timeout_fault`, and
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
source/search terms, and run the listed narrow tests. The optional comparison artifact compares two observability suites by
scenario health/focus/probes/scores and hypothesis rank/score changes, then
reports matched, changed, or regressed verdicts with replay args and current-run
artifact pointers for any regression. Use `--fail-on-comparison-regression`
with `--compare-suite-dir` when the comparison should be a CI gate. The
`cpu_zero_page_wrap_fault`, `cpu_indirect_jmp_fault`, and
`ppu_read_buffer_fault` scenarios use telemetry-visible fault injection to
corrupt deterministic CPU RAM and VRAM sentinels just before the cartridge
assertion reads them, while `dma_oam_transfer_fault` corrupts the host-observed
OAM DMA source byte before `$4014` and `apu_status_fault` disables `$4015`
just before the cartridge reads the APU status register.
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
`joypad_strobe_reset_fault` consumes the reset A-button bit after a second
`$4016` strobe sequence, proving mid-stream joypad strobe reset regressions
localize to `joypad.strobe_reset`.
`input_mask_matrix_pass` runs joypad 1 with `0xAA` and joypad 2 with `0x55`
while setting the cartridge's expected masks to the same values, proving
non-default controller masks can be validated as a healthy fixture without
rebuilding the ROM.
`joypad_strobe_high_hold_fault` clears joypad 1's A button just before the
strobe-high hold test reads `$4016`, proving strobe-high read regressions
localize to `joypad.strobe_high_hold`.
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
joypad masks, joypad strobe-reset behavior, joypad strobe-high hold behavior,
PPUDATA register increment behavior, PPUSTATUS write-latch reset behavior, DMA
host-observation, APU status, PPU assertion, and PPU progress-timeout failure
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

To validate the full observability wrapper output that AI tooling consumes,
including the debug index, aggregate analysis, coverage ledger, diagnostic code
map, telemetry catalog, diagnostic investigation plan, optional cross-run
comparison, focused replay evidence, and `observability-run.json`, run:

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
`diagnostic-investigation-plan.json`, and `diagnostic-investigation-plan.md`.
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
- OAM DMA from CPU page `$0300`, including host-observed CPU stall cycle bucket
  and DMC sample-DMA overlap telemetry
- APU pulse-channel status register
- Mapper 2/UXROM PRG bank switching, fixed final-bank reads, and PRG RAM round-trips
- Joypad strobe and shift reads with configurable expected masks
- Joypad mid-stream strobe reset behavior
- Joypad strobe-high hold behavior
- Taken CPU branch crossing a page boundary
- Joypad reads after the eighth latched button
- PPU NMI delivery and rendered frame production
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
rendered-frame checksum/color count, audio sample count/peak, status/frame
events, current-test transition events, a bounded instruction-boundary trace,
failure-localization metadata, per-test timeline/duration telemetry, structured
observation probes, and a derived analysis summary.

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
`ppu.nmi`, marks `ppu.nmi_count` failed, and preserves the stalled loop symbol
in the instruction trace tail.

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

Scenario suite schema version `8` and observer schema version `2` add
`replay_args` arrays for each scenario, observer action, and observation. These
arguments call `cargo run --bin oxidenes-diagnostic -- --bundle-dir target/diagnostics/replay/<scenario>`
with the exact cycle, joypad, expected mask, and fault-injection settings needed
to regenerate that one bundle.
