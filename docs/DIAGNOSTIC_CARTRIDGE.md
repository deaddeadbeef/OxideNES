# OxideNES Diagnostic Cartridge

OxideNES includes a generated, IP-safe diagnostic cartridge for headless emulator validation. The cartridge is assembled from deterministic 6502 instructions and CHR byte patterns at runtime; no `.nes` file or third-party ROM content is committed. The generated ROM uses Mapper 2/UXROM so the same cartridge can validate PRG bank switching and PRG RAM access through the normal CPU bus.

Run it with:

```powershell
cargo run --bin oxidenes-diagnostic -- --json target/diagnostics/telemetry.json --report target/diagnostics/report.md --dump-rom target/diagnostics/oxidenes-diagnostic.nes
```

The runner exits `0` only when the cartridge and host-side checks pass. It exits `1` for diagnostic failures or timeouts, and `2` for CLI/build errors.

Use `--joypad1 <BYTE>` and `--joypad2 <BYTE>` to override the host-side
controller masks used by the cartridge. The default masks match the generated
assertions: joypad 1 expects A + Right (`0x81`) and joypad 2 expects Start +
Down (`0x28`). Overriding either value is useful for failure-localization
smokes because the run still emits telemetry, triage JSON, and bundles before
exiting `1`.

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
scenario suite, runs the verifier, and writes `observability-run.json` plus
`observability-run.md` into the suite directory:

```powershell
python scripts/run_diagnostic_observability.py --suite-dir target/diagnostics/scenario-suite
```

The scenario suite writes `scenario-suite.json`, `scenario-suite.md`,
`scenario-suite-observer.json`, and `scenario-suite-observer.md` at the root,
plus one full bundle per scenario: `pass`, `joypad1_mismatch`,
`joypad2_mismatch`, `dma_oam_transfer_fault`, `apu_status_fault`,
`cpu_zero_page_wrap_fault`, `cpu_indirect_jmp_fault`, `ppu_read_buffer_fault`,
`mapper2_bank_switch_fault`, `mapper2_prg_ram_fault`,
`ppu_nametable_mirroring_fault`,
`ppu_nmi_timeout_fault`, and
`timeout_cycle_limit`. The
observer JSON is the compact machine entry point: it turns the root attention
queue into ordered next actions, scenario observations, and evidence pointers so
an automated debugger can decide which artifact to open without traversing every
bundle first. The root manifest records each scenario's expected runner exit
code, expected health, expected focus test/domain, actual `debug_focus`, failed
probe ids, per-scenario baseline comparison summaries, explicit contract-match
breakdowns, a suite-level attention queue, and artifact paths. The
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
`ppu_nmi_timeout_fault` disables PPU NMI delivery after the render-frame test
enables NMI, proving timeout localization can stay focused on `ppu.nmi` and the
active cartridge test. The suite can prove CPU addressing, CPU control-flow,
mapper PRG switching, mapper PRG RAM, PPU nametable mirroring, DMA host-observation, APU status, PPU assertion, and PPU
progress-timeout failure localization without requiring a broken emulator build. The Markdown reports add
suite analysis, observer next actions, an
attention queue, compact scenario matrices, contract matrix, baseline comparison
matrix, AI drilldown order, and bundle artifact maps for humans or agents
inspecting CI artifacts. The command exits `0` when all known-good and
intentionally failing scenarios match their expected debug-focus contracts. CI
and release workflows upload this corpus as `oxidenes-diagnostic-scenario-suite`.

To validate a generated suite artifact before handing it to an automated
debugger or attaching it to a release, run:

```powershell
python scripts/verify_diagnostic_suite.py --suite-dir target/diagnostics/scenario-suite
```

The verifier checks the root schema versions, observer next actions,
observations, Markdown sections, and every artifact path referenced by the
manifest or observer report. CI and release workflows run this verifier before
uploading the scenario-suite artifact through `run_diagnostic_observability.py`.

## Coverage

The cartridge exercises the emulator through the normal CPU, bus, cartridge, PPU, APU, DMA, and joypad paths:

- CPU arithmetic and flags
- Stack, `JSR`, and `RTS`
- Zero-page indexed read/write wraparound
- Indirect `JMP ($xxFF)` page-wrap behavior
- 2 KiB CPU RAM mirroring
- PPU palette register write/read
- PPU horizontal nametable mirroring through CPU-driven PPUDATA reads
- OAM DMA from CPU page `$0300`, including host-observed CPU stall cycle bucket
  and DMC sample-DMA overlap telemetry
- APU pulse-channel status register
- Mapper 2/UXROM PRG bank switching, fixed final-bank reads, and PRG RAM round-trips
- Joypad strobe and shift reads
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
