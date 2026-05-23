# Product Polish Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn OxideNES from a strong emulator prototype into a public open source product with clear safety rules, tracked milestones, reliable validation, and customer-facing polish.

**Architecture:** Keep the first PR limited to docs, tracking, and low-risk user-facing text. Larger code work will be split by milestone so future PRs can isolate `src/main.rs` extraction, malformed-input resilience, compatibility testing, and packaging hardening.

**Tech Stack:** Rust 2021, Cargo, minifb, cpal, gilrs, mlua, GitHub Actions, GitHub Issues, GitHub Projects v2, WiX for Windows installer packaging.

---

## File Structure

- `docs/IP_COMPLIANCE.md`: public policy for ROMs, screenshots, generated art, trademarks, and release assets.
- `docs/PRODUCT_READINESS_AUDIT_2026-05-23.md`: audit findings, current validation evidence, and milestone scope.
- `docs/superpowers/plans/2026-05-23-product-polish-roadmap.md`: executable roadmap for this work.
- `README.md`: user-facing setup, config, legal, and support documentation.
- `CONTRIBUTING.md`: contributor workflow, branch target, validation, and IP checklist.
- `src/main.rs`: CLI help/version command name fix.

## Task 1: Product Safety and Stale Text

**Files:**
- Create: `docs/IP_COMPLIANCE.md`
- Create: `docs/PRODUCT_READINESS_AUDIT_2026-05-23.md`
- Create: `docs/superpowers/plans/2026-05-23-product-polish-roadmap.md`
- Modify: `README.md`
- Modify: `CONTRIBUTING.md`
- Modify: `src/main.rs`

- [x] **Step 1: Fix CLI binary name**

Patch `src/main.rs` so `--help` and `--version` show the actual shipped binary name:

```rust
println!("    oxidenes [OPTIONS] [ROM_FILE]");
println!("oxidenes {}", env!("CARGO_PKG_VERSION"));
```

- [x] **Step 2: Add public IP policy**

Create `docs/IP_COMPLIANCE.md` with these sections:

```markdown
# IP Compliance Policy

OxideNES is a clean-room emulator project. The repository and release artifacts must remain safe for public open source distribution.
```

Include rules that prohibit commercial ROMs, BIOS files, copyrighted screenshots, box art, manuals, music, logos, icons, or character artwork.

- [x] **Step 3: Record product audit**

Create `docs/PRODUCT_READINESS_AUDIT_2026-05-23.md` with baseline validation, architecture findings, customer-facing findings, IP compliance findings, and M1-M4 milestones.

- [x] **Step 4: Update README**

Update README to state:

```markdown
Settings are stored in `~/.nes-emulator/config.json`.
```

Also link `docs/IP_COMPLIANCE.md` from the Legal section.

- [x] **Step 5: Update CONTRIBUTING**

Update CONTRIBUTING to target `main`, require `cargo clippy -- -D warnings`, and point contributors to `docs/IP_COMPLIANCE.md`.

## Task 2: GitHub Roadmap Tracking

**Files:**
- No repository file changes.
- GitHub project: `OxideNES Product Polish Roadmap`
- GitHub milestones: M1-M4 from `docs/PRODUCT_READINESS_AUDIT_2026-05-23.md`

- [x] **Step 1: Create project**

Run:

```powershell
gh project create --owner deaddeadbeef --title "OxideNES Product Polish Roadmap" --format json
```

Expected: JSON output with a project number and URL.

Created: https://github.com/users/deaddeadbeef/projects/2

- [x] **Step 2: Create milestones**

Run:

```powershell
gh api repos/deaddeadbeef/OxideNES/milestones -f title="M1 - Product Readiness Foundation" -f description="Public repo safety, docs, validation, and issue tracking."
gh api repos/deaddeadbeef/OxideNES/milestones -f title="M2 - Core Compatibility and Safety" -f description="Malformed-input resilience, compatibility fixtures, and core safety."
gh api repos/deaddeadbeef/OxideNES/milestones -f title="M3 - UX, Packaging, and Support" -f description="Installers, onboarding, support templates, and release smoke checks."
gh api repos/deaddeadbeef/OxideNES/milestones -f title="M4 - 1.0 Release Candidate" -f description="Final acceptance gates, IP review, and release candidate hardening."
```

Expected: each command returns a milestone object with `state` set to `open`.

- [x] **Step 3: Create M1 issues**

Create these issues and assign milestone `M1 - Product Readiness Foundation`:

```text
M1: Add IP compliance and public repo safety policy
M1: Establish a rustfmt baseline and enforce formatting in CI
M1: Add issue templates and release checklist
M1: Add crash-resilience tests for user-facing file/network paths
M1: Review built-in ROM metadata for factual, non-promotional scope
M1: Define milestone tag and release protocol
```

- [x] **Step 4: Create M2-M4 issues**

Create these issues and assign the matching milestone:

```text
M2: Extract config and file-browser logic from src/main.rs
M2: Add non-proprietary compatibility fixtures and mapper regression gates
M2: Harden save-state and recording deserialization boundaries
M3: Add release artifact and Windows installer smoke checks
M3: Document first-run, support, and platform-specific setup
M4: Define and execute 1.0 release candidate acceptance gates
M4: Run final IP compliance review before 1.0
```

- [x] **Step 5: Add all issues to project**

Run `gh project item-add` once per issue URL using the created project number.

Expected: each issue appears in `OxideNES Product Polish Roadmap`.

Created and added to project:

- #5 M1: Add IP compliance and public repo safety policy
- #6 M1: Establish a rustfmt baseline and enforce formatting in CI
- #7 M1: Add issue templates and release checklist
- #8 M1: Add crash-resilience tests for user-facing file and network paths
- #9 M1: Review built-in ROM metadata for factual, non-promotional scope
- #17 M1: Define milestone tag and release protocol
- #10 M2: Extract config and file-browser logic from src/main.rs
- #11 M2: Add non-proprietary compatibility fixtures and mapper regression gates
- #12 M2: Harden save-state and recording deserialization boundaries
- #13 M3: Add release artifact and Windows installer smoke checks
- #14 M3: Document first-run, support, and platform-specific setup
- #15 M4: Define and execute 1.0 release candidate acceptance gates
- #16 M4: Run final IP compliance review before 1.0

## Task 2A: Milestone Tags and Releases

**Files:**
- Modify: `docs/PRODUCT_READINESS_AUDIT_2026-05-23.md`

- [x] **Step 1: Add tag policy**

Document this policy:

```text
Create small milestone tags after meaningful merged checkpoints, such as product-polish-m1-kickoff.
Create major milestone tags when a full milestone closes, such as product-polish-m1-complete.
Publish GitHub Releases only for user-meaningful binary changes, packaging changes, or compatibility/stability improvements.
```

- [ ] **Step 2: Create first small milestone tag after merge**

After this PR is merged to `main`, run:

```powershell
git fetch origin main --tags
git tag -a product-polish-m1-kickoff origin/main -m "Product polish M1 kickoff"
git push origin product-polish-m1-kickoff
```

Expected: tag points to the merged foundation commit on `main`. Do not create a GitHub Release for this docs-only checkpoint.

## Task 3: Validation and PR

**Files:**
- Validate all repository changes from Task 1.

- [x] **Step 1: Run checks**

Run:

```powershell
cargo check
cargo test
cargo clippy -- -D warnings
git diff --check
```

Expected: all commands pass.

- [x] **Step 2: Record formatting exception**

Run:

```powershell
cargo fmt -- --check
```

Expected: fails on pre-existing formatting drift. Do not run broad `cargo fmt` in this PR; keep formatting enforcement as a dedicated M1 issue.

- [x] **Step 3: Commit**

Run:

```powershell
git add README.md CONTRIBUTING.md src/main.rs docs/IP_COMPLIANCE.md docs/PRODUCT_READINESS_AUDIT_2026-05-23.md docs/superpowers/plans/2026-05-23-product-polish-roadmap.md
git commit -m "docs: add product polish roadmap"
```

- [x] **Step 4: Push and open PR**

Run:

```powershell
git push -u origin product-polish-roadmap
gh pr create --title "Add product polish roadmap and IP compliance policy" --body-file pr-body.md
```

Expected: PR opens against `main`.

Opened: https://github.com/deaddeadbeef/OxideNES/pull/18
