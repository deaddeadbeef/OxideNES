# OxideNES Diagnostic Cartridge

OxideNES includes a generated, IP-safe diagnostic cartridge for headless emulator validation. The cartridge is assembled from deterministic 6502 instructions and CHR byte patterns at runtime; no `.nes` file or third-party ROM content is committed.

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
summary, timing/timeline table, observation probe table, next actions, host
failures, and final event tail. This gives CI logs, issue attachments, and AI
debugging sessions a stable human-readable entry point while preserving the raw
JSON for exact tooling.

Use `--triage-json <FILE>` for a compact machine-readable handoff artifact. It
summarizes verdict, health, current test, first failure, failed/skipped probes,
timing, optional baseline comparison, next actions, artifact hints, and the
final event tail without requiring tools to scrape `report.md` or load the full
telemetry envelope first.

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
- `triage.json`: compact AI-readable failure focus, next actions, and artifact
  pointers
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

## Coverage

The cartridge exercises the emulator through the normal CPU, bus, cartridge, PPU, APU, DMA, and joypad paths:

- CPU arithmetic and flags
- Stack, `JSR`, and `RTS`
- 2 KiB CPU RAM mirroring
- PPU palette register write/read
- OAM DMA from CPU page `$0300`, including host-observed CPU stall cycle bucket
  and DMC sample-DMA overlap telemetry
- APU pulse-channel status register
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
events, current-test transition events, failure-localization metadata, per-test
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
