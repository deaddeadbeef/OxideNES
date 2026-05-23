# M4 Release Candidate Evidence - 2026-05-23

Branch: `product-polish-m4-rc-gates`
Base commit: `08e0c7c`
Local time: `2026-05-23T21:00:42.1768858+08:00`
Rust toolchain: `rustc 1.92.0 (ded5c06cf 2025-12-08)`, `cargo 1.92.0 (344c4567c 2025-10-21)`

This file records the M4 local release-candidate gate run. GitHub Actions checks on the pull request remain the required cross-platform evidence before merging or tagging a user-facing release.

## Source And Security Gates

| Gate | Result | Evidence |
| --- | --- | --- |
| `cargo fmt -- --check` | Pass | Completed with exit code 0. |
| `cargo check` | Pass | Completed with exit code 0 after dependency updates. |
| `cargo test` | Pass | Unit, integration, and doc tests completed with exit code 0; generated CPU program fixture replaced the prior binary ROM test. |
| `cargo clippy -- -D warnings` | Pass | Completed with exit code 0. |
| `git diff --check` | Pass | Completed with exit code 0. |
| `cargo audit` | Pass | Updated `rustls-webpki` from `0.103.9` to `0.103.13`; no vulnerabilities remain. One allowed unmaintained warning remains for `instant 0.1.13` through `minifb 0.28.0`. |
| `python scripts/ip_compliance_audit.py` | Pass | `IP compliance audit passed (70 tracked files checked)`. |

## Compatibility And Safety Gates

- CPU fetch/decode execution: pass via `tests/cpu_program_tests.rs`, which generates in-memory iNES content and exercises reset, PRG fetch, zero-page writes, arithmetic, DEX, and BNE loop behavior.
- Mapper fixtures: pass via synthetic mapper tests for construction and bank-switching behavior.
- PPU fixtures: pass via synthetic nametable mirroring and save-state truncation tests.
- Malformed inputs: pass through existing save-state, recording, scripting, updater, ROM metadata, cartridge-header, netplay packet, and state-reader regression tests.
- IP-sensitive compatibility fixtures: pass after removing `nestest.nes`; no tracked `.nes` files remain.

## Packaging Gates

| Gate | Result | Evidence |
| --- | --- | --- |
| `OXIDENES_RELEASE=1 cargo build --release` | Pass | Completed with exit code 0. |
| `target\release\oxidenes.exe --version` | Pass | Printed `oxidenes 0.3.1`. |
| Windows binary asset validation | Pass | `staged asset OK: target\release-assets\oxidenes-windows-x64.exe`; `release asset checks passed`. |
| `OXIDENES_RELEASE=1 cargo wix --target x86_64-pc-windows-msvc --nocapture` | Pass | WiX compiler/linker completed and produced `target\wix\oxidenes-0.3.1-x86_64.msi`. |
| MSI/WiX input validation | Pass | Active package inputs are `wix/license.rtf` and `$(var.cargotargetbindir)/oxidenes.exe`; `release asset checks passed`. |

## Performance Evidence

Short Criterion samples were run on the local Windows machine with `--sample-size 10 --warm-up-time 1 --measurement-time 1`.

| Benchmark | Result |
| --- | --- |
| `cargo bench --bench crt_bench` / `crt_filter` | `[4.2181 ms 4.3461 ms 4.5960 ms]` |
| `cargo bench --bench crt_bench` / `phosphor_bloom` | `[3.0671 ms 3.3472 ms 3.5398 ms]` |
| `cargo bench --bench crt_bench` / `scanline_glow` | `[88.497 us 94.687 us 98.841 us]` |
| `cargo bench --bench glass_bench` / `glass_inner_loop` | `[4.6490 ms 4.6706 ms 4.7028 ms]` |

## IP Review

See `docs/IP_REVIEW_2026-05-23.md` for the file-by-file release/IP review summary.

Local result: pass. Cross-platform GitHub Actions must pass before M4 issues are closed and before any release tag or GitHub Release is published.
