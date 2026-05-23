# Product Readiness Audit - 2026-05-23

This audit records the first full-project pass toward making OxideNES feel like a customer-shippable open source product.

## Baseline

- Repository: `deaddeadbeef/OxideNES`
- Local branch/worktree: `product-polish-roadmap`
- Version: `0.3.1`
- Validation run: `cargo check`, `cargo test`, and `cargo clippy -- -D warnings` pass locally.
- Known formatting state: `cargo fmt -- --check` fails across many existing files. Treat repository formatting as a tracked product-readiness issue instead of running a broad formatter in unrelated work.

## Architecture Findings

- `src/main.rs` is the largest product risk at roughly 7,500 lines. It owns CLI parsing, config, menu UI, file browsing, input, game loop, window lifecycle, overlays, CRT frame composition, and recording integration.
- Core emulation modules are separated (`src/cpu.rs`, `src/ppu.rs`, `src/apu.rs`, `src/bus.rs`, `src/mapper.rs`), but `src/mapper.rs` is also large and will need focused compatibility work before 1.0.
- Rendering has a better structure than the rest of the app after prior extraction to `src/rendering.rs`, with benchmarks and regression tests already present.
- Packaging exists for GitHub releases and WiX, but release readiness still needs stronger artifact verification, installer smoke checks, and documented support boundaries.

## Customer-Facing Findings

- CLI help used the old `nes-emulator` command name even though the shipped binary is `oxidenes`.
- README config path said `~/.oxidenes/config.json`; the current code writes to `~/.nes-emulator/config.json`.
- CONTRIBUTING still referenced `master`; the default branch is `main`.
- The README correctly states that ROMs are not included, but the repo needs a dedicated IP compliance policy for contributors and release reviewers.

## IP Compliance Findings

- The repo does not bundle commercial ROMs or proprietary artwork.
- The initial baseline included `nestest.nes` as a test fixture credited as public domain. M4 removes that binary fixture and relies on generated in-memory test ROM content instead.
- `src/romdb.rs` includes factual compatibility metadata for commercial titles. That is not ROM content, but any future expansion should be reviewed carefully and kept user-overridable.
- Documentation examples should avoid naming specific commercial titles unless there is a concrete compatibility reason.

## Progressive Milestones

### M1 - Product Readiness Foundation

Goal: make the public repo safe, understandable, and supportable before adding larger features.

- Add an IP compliance policy and contributor checklist.
- Fix stale user-facing command names, branch names, and config path documentation.
- Add GitHub project tracking with issues and milestones.
- Define when small milestone tags, major milestone tags, and GitHub Releases should be created.
- Establish a formatting strategy without creating unrelated repo-wide churn.
- Add crash-resilience issues for ROM loading, update checking, recording, scripting, and save-state paths.

### M2 - Core Compatibility and Safety

Goal: make emulator behavior more trustworthy under malformed inputs and real user libraries.

- Split low-level serialization/deserialization helpers out of large modules where bounds checks are currently hand-rolled.
- Add compatibility fixtures that do not depend on proprietary ROM content.
- Add regression tests for malformed save states, recordings, scripts, and user metadata.
- Review mapper hints and built-in metadata for factual, non-promotional wording.

### M3 - UX, Packaging, and Support

Goal: make installation, onboarding, and troubleshooting feel like a product rather than a demo.

- Add release artifact smoke checks and installer verification.
- Document first-run flow, ROM folder setup, controls, config paths, save paths, and support expectations.
- Add a structured bug-report template that asks for OS, version, ROM name only, reproduction steps, and no ROM uploads.
- Add a support matrix for Windows, Linux, and macOS.

### M4 - 1.0 Release Candidate

Goal: freeze the product surface and ship a defensible 1.0 release candidate.

- Define minimum compatibility, performance, and packaging acceptance gates.
- Run a final IP compliance pass over assets, screenshots, docs, metadata, and release artifacts.
- Create signed/tagged release artifacts with reproducible release notes.

## First Execution Slice

### Task 1: Public Repo Safety and Stale User-Facing Text

**Files:**
- Create: `docs/IP_COMPLIANCE.md`
- Create: `docs/PRODUCT_READINESS_AUDIT_2026-05-23.md`
- Create: `docs/superpowers/plans/2026-05-23-product-polish-roadmap.md`
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`
- Modify: `src/main.rs`

- [x] **Step 1: Fix CLI command name**

Change help/version output in `src/main.rs` from:

```rust
println!("    nes-emulator [OPTIONS] [ROM_FILE]");
println!("nes-emulator {}", env!("CARGO_PKG_VERSION"));
```

to:

```rust
println!("    oxidenes [OPTIONS] [ROM_FILE]");
println!("oxidenes {}", env!("CARGO_PKG_VERSION"));
```

- [x] **Step 2: Add IP compliance policy**

Create `docs/IP_COMPLIANCE.md` with distribution rules, acceptable nominative references, contributor checklist, and current repository notes.

- [x] **Step 3: Fix stale docs**

Update README and CONTRIBUTING so default branch, binary name, support boundaries, and config path match the current code.

- [x] **Step 4: Create GitHub tracking**

Create a GitHub project named `OxideNES Product Polish Roadmap`, create the four milestones above, create issues for the M1-M4 work, and add those issues to the project.

Run:

```powershell
gh project create --owner deaddeadbeef --title "OxideNES Product Polish Roadmap" --format json
gh api repos/deaddeadbeef/OxideNES/milestones -f title="M1 - Product Readiness Foundation" -f description="Public repo safety, docs, validation, and issue tracking."
gh api repos/deaddeadbeef/OxideNES/milestones -f title="M2 - Core Compatibility and Safety" -f description="Malformed-input resilience, compatibility fixtures, and core safety."
gh api repos/deaddeadbeef/OxideNES/milestones -f title="M3 - UX, Packaging, and Support" -f description="Installers, onboarding, support templates, and release smoke checks."
gh api repos/deaddeadbeef/OxideNES/milestones -f title="M4 - 1.0 Release Candidate" -f description="Final acceptance gates, IP review, and release candidate hardening."
```

Created tracking:

- Project: https://github.com/users/deaddeadbeef/projects/2
- Milestones: https://github.com/deaddeadbeef/OxideNES/milestones
- Issues: #5 through #17

- [x] **Step 5: Validate**

Run:

```powershell
cargo check
cargo test
cargo clippy -- -D warnings
git diff --check
```

Expected result: all commands pass. `cargo fmt -- --check` is intentionally not part of this slice because the existing repository has broad formatting drift that will be handled as its own issue.

- [x] **Step 6: Commit and open PR**

Run:

```powershell
git add README.md CONTRIBUTING.md src/main.rs docs/IP_COMPLIANCE.md docs/PRODUCT_READINESS_AUDIT_2026-05-23.md docs/superpowers/plans/2026-05-23-product-polish-roadmap.md
git commit -m "docs: add product polish roadmap"
git push -u origin product-polish-roadmap
gh pr create --title "Add product polish roadmap and IP compliance policy" --body-file <generated-pr-body>
```

Opened: https://github.com/deaddeadbeef/OxideNES/pull/18

## Follow-Up Execution Order

1. M1 issue #17: Define milestone tag and release protocol.
2. M1 issue #6: Establish formatting baseline in a narrow PR that runs `cargo fmt` once and updates CI to enforce it after the formatter-only change.
3. M1 issue #7: Add bug report and release checklist templates.
4. M1 issue #8: Add crash-resilience tests for update, recording, scripting, and malformed user metadata.
5. M2 issue #10: Extract config and file-browser logic from `src/main.rs` without changing behavior.
6. M2 issue #11: Add non-proprietary compatibility fixtures and mapper regression gates.
7. M3 issue #13: Verify Windows installer behavior and document platform-specific prerequisites.

## Tag and Release Policy

- Create small milestone tags after meaningful merged checkpoints, such as `product-polish-m1-kickoff` for this foundation PR and `product-polish-m1-formatting` after the formatting baseline lands.
- Create major milestone tags when a full milestone closes, such as `product-polish-m1-complete` and `product-polish-m2-complete`.
- Publish GitHub Releases only for user-meaningful binary changes, packaging changes, or compatibility/stability improvements. Do not publish a binary release for docs-only tracking work unless users need the packaged artifact.
