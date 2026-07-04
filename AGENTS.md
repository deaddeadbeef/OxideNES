# Agent Instructions

## Build Artifact Hygiene

- Treat generated build outputs as disposable. Do not commit `target/`, installer outputs, logs, cache folders, or other regenerated artifacts unless a task explicitly asks for a release artifact.
- When disk usage is high or after heavy Rust build/test work, check the repository build footprint and clear stale artifacts with `cargo clean` from this repo root.
- Before deleting artifacts, confirm no build, test, packaging, or release process is currently using them.
