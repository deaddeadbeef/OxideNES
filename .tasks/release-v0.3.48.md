# Release v0.3.48

## Goal

Publish a small release for the diagnostic-cartridge schema 80 and schema 81
CPU load/store matrix coverage updates.

## Acceptance

- `Cargo.toml` and the `oxidenes` package entry in `Cargo.lock` report
  `0.3.48`.
- `CHANGELOG.md` moves the diagnostic load/store matrix entries under
  `## [0.3.48] - 2026-06-21`.
- Local dev-build CI passes on the release bump before tagging.
- A release binary smoke proves release metadata reports `oxidenes 0.3.48`.
- Tag `v0.3.48` is published only after the merge reaches `main`.
