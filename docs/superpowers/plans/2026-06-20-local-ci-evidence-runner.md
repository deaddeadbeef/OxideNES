# Local CI Evidence Runner Plan

## Goal

Make local dev-build validation reproducible and AI-readable by wrapping the
same diagnostic, build, test, audit, and smoke gates into a command that writes
JSON, Markdown, and command-log evidence without relying on GitHub Actions.

## Scope

- [x] Add `scripts/run_local_ci.py`.
- [x] Default the runner to debug/dev build metadata while keeping a release
  profile option for parity checks.
- [x] Record command argv, exit code, duration, stdout/stderr tails, full logs,
  git metadata, configured target, diagnostic artifact paths, and summary
  counts.
- [x] Include diagnostic bundle, diagnostic e2e, diagnostic observability
  verification, diagnostic suite verification, profile, build/test, binary
  smoke, clippy, IP compliance, fmt, and cargo-audit steps.
- [x] Add a dry-run mode for cheap report-contract tests.
- [x] Document the runner in the diagnostic and release-gate docs.

## Verification

- [x] `cargo fmt -- --check`
- [x] `git diff --check`
- [x] `cargo test --test local_ci_script_tests --target x86_64-pc-windows-msvc`
- [x] `python scripts/run_local_ci.py --output-dir target/local-ci/runner-smoke --suite-dir target/local-ci/runner-smoke/scenario-suite --target x86_64-pc-windows-msvc --dry-run`
- [x] `python scripts/run_local_ci.py --output-dir target/local-ci/runner-thin --suite-dir target/local-ci/runner-thin/scenario-suite --target x86_64-pc-windows-msvc --skip-security-audit --skip-diagnostic-bundle --skip-diagnostic-e2e --skip-profile --skip-build-test --skip-clippy`
- [x] `cargo test --test diagnostic_cli_tests diagnostic_cli_writes_standalone_triage_json --target x86_64-pc-windows-msvc`
- [x] `cargo test --target x86_64-pc-windows-msvc`
- [x] `cargo clippy -- -D warnings`
