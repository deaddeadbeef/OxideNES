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
- GitHub Actions CI is green on Windows, Linux, macOS, rustfmt, clippy, security audit, IP compliance, and diagnostic bundle jobs.

### Compatibility And Safety

- CPU fetch/decode execution is covered by generated in-memory test ROM content.
- The generated diagnostic cartridge bundle contains `manifest.json`, `triage.json`, `telemetry.json`, `report.md`, and `diagnostic.nes`; when a baseline is supplied it also contains `comparison.json` and `comparison.md`.
- `triage.json` and `telemetry.json` include explicit known coverage gaps so release evidence does not overstate cartridge coverage.
- `triage.json` and `telemetry.json` include a derived debug focus with the focus test/subsystem/domain, failed probe ids, final event, terminal instruction, and last focus-test instruction for AI-first triage.
- Diagnostic event tails include CPU register snapshots and diagnostic RAM watchpoints so failed headless runs preserve immediate execution-state context for automated triage.
- Diagnostic bundles include a bounded decoded instruction trace tail with diagnostic cartridge symbols so failed headless runs preserve the final opcode-boundary CPU/RAM context for automated triage.
- Diagnostic scenario suites include pass, joypad-1 mismatch, joypad-2 mismatch, and timeout bundles with expected-vs-actual debug-focus contracts, plus root `scenario-suite.json` and `scenario-suite.md` indexes for automated regression triage.
- `telemetry.json` includes actual and expected joypad masks for both controller ports, and the diagnostic cartridge exercises `$4016` and `$4017` serial reads.
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
