# Release v0.3.49

## Goal

Publish a small release for the diagnostic-cartridge schema 82 and schema 83
CPU load/store matrix coverage updates.

## Acceptance

- `Cargo.toml` and the `oxidenes` package entry in `Cargo.lock` report
  `0.3.49`.
- `CHANGELOG.md` moves the diagnostic load/store matrix entries under
  `## [0.3.49] - 2026-06-21`.
- Local dev-build CI passes on the release bump before tagging.
- A release binary smoke proves release metadata reports `oxidenes 0.3.49`.
- Tag `v0.3.49` is published only after the merge reaches `main`.

## Verification

- `cargo fmt -- --check`: passed.
- `git diff --check`: passed.
- `python scripts/run_local_ci.py --output-dir target/local-ci/release-v0.3.49-dev --suite-dir target/local-ci/release-v0.3.49-dev/scenario-suite --target x86_64-pc-windows-msvc --build-profile debug`: passed 13/13 commands.
- `OXIDENES_RELEASE=1 cargo test --test version_cli_tests --target x86_64-pc-windows-msvc`: passed 5/5 tests.
- `OXIDENES_RELEASE=1 cargo build --release --target x86_64-pc-windows-msvc`: passed.
- `target/x86_64-pc-windows-msvc/release/oxidenes.exe --version`: printed `oxidenes 0.3.49`.
