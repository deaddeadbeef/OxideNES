# OxideNES Diagnostic Cartridge

OxideNES includes a generated, IP-safe diagnostic cartridge for headless emulator validation. The cartridge is assembled from deterministic 6502 instructions and CHR byte patterns at runtime; no `.nes` file or third-party ROM content is committed.

Run it with:

```powershell
cargo run --bin oxidenes-diagnostic -- --json target/diagnostics/telemetry.json --report target/diagnostics/report.md --dump-rom target/diagnostics/oxidenes-diagnostic.nes
```

The runner exits `0` only when the cartridge and host-side checks pass. It exits `1` for diagnostic failures or timeouts, and `2` for CLI/build errors.

Use `--json <FILE>` for the full machine-readable telemetry envelope and
`--report <FILE>` for a Markdown triage artifact built from the same run. The
report contains the verdict, derived analysis, failure localization, coverage
summary, timing/timeline table, next actions, host failures, and final event
tail. This gives CI logs, issue attachments, and AI debugging sessions a stable
human-readable entry point while preserving the raw JSON for exact tooling.

To compare a run against a known-good telemetry baseline:

```powershell
cargo run --bin oxidenes-diagnostic -- --json target/diagnostics/current.json --report target/diagnostics/current.md --baseline-json target/diagnostics/baseline.json --comparison-json target/diagnostics/comparison.json --comparison-report target/diagnostics/comparison.md --no-stdout
```

Baseline comparison hard-fails on result, health, coverage, or per-test outcome
regressions and exits `1`. It records warning-level differences for timing and
observable artifact drift such as frame/OAM checksums, allowing CI and AI agents
to separate real correctness regressions from values that need review.

## Coverage

The cartridge exercises the emulator through the normal CPU, bus, cartridge, PPU, APU, DMA, and joypad paths:

- CPU arithmetic and flags
- Stack, `JSR`, and `RTS`
- 2 KiB CPU RAM mirroring
- PPU palette register write/read
- OAM DMA from CPU page `$0300`
- APU pulse-channel status register
- Joypad strobe and shift reads
- Taken CPU branch crossing a page boundary
- Joypad reads after the eighth latched button
- PPU NMI delivery and rendered frame production

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

The cartridge writes status bytes into CPU RAM:

- `$00F0`: status, `0x01` running, `0x80` pass, `0xE0` fail
- `$00F1`: current test id
- `$00F2`: failure code
- `$00F3`: signature byte, expected `0xA5`
- `$00F4`: NMI count
- `$0200..`: per-test result slots, `0x01` means pass

The host runner adds emulator-side telemetry that the cartridge cannot inspect directly: CPU registers, frame count, RAM checksum, OAM checksum, rendered-frame checksum/color count, audio sample count/peak, status/frame events, current-test transition events, failure-localization metadata, per-test timeline/duration telemetry, and a derived analysis summary.
