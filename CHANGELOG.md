# Changelog

## [Unreleased]

## [0.3.47] - 2026-06-20

### Added
- Added local dev-build CI evidence runner output for fmt, IP compliance, offline security audit, diagnostic baseline/bundle/e2e, observability verifiers, diagnostic profile, build, test, smoke-binary, and clippy gates.
- Added host-input observability coverage for input remapping, disconnected/default controllers, injected host events, OS-typed minifb/gilrs input snapshots, and live gilrs polling paths.
- Added CLI coverage for the ROM folder import copy/symlink workflow.
- Expanded the CPU load/store/transfer diagnostic cartridge matrix with official `STA abs,X`, `STA zp,X`, `STA (zp,X)`, and `STA (zp),Y` store addressing-mode cases.

### Changed
- Bumped diagnostic telemetry to schema 79 and suite version to `diagnostic-cartridge-v79`.
- Updated the CPU load/store matrix contract to seven store cases, store mask `0x7F`, and fifteen total load/store/transfer cases.
- Made the local CI security-audit gate use cached offline advisory data so dev-build validation does not depend on GitHub during the run.

## [0.3.46] - 2026-06-20

### Added
- Added a generated Mapper 4/MMC3 sprite A12-source gate diagnostic variant that proves low-pattern sprite rendering suppresses MMC3 scanline IRQ clocks while high sprite pattern-table rendering still triggers the configured IRQ.

### Changed
- Bumped diagnostic telemetry to schema 75 and added the `mapper4.mmc3_sprite_a12_irq_gate` probe to the AI observability catalog.

## [0.3.45] - 2026-06-20

### Added
- Added a generated Mapper 4/MMC3 A12-source gate diagnostic variant that proves low-pattern background rendering suppresses MMC3 scanline IRQ clocks while high-pattern background rendering still triggers the configured IRQ.

### Changed
- Bumped diagnostic telemetry to schema 74 and added the `mapper4.mmc3_a12_irq_gate` probe to the AI observability catalog.

## [0.3.44] - 2026-06-20

### Added
- Added a generated Mapper 3/CNROM rendered CHR-bank diagnostic variant with host-sampled pixel telemetry for CHR bank writes while background rendering remains enabled.

### Changed
- Bumped diagnostic telemetry to schema 73 and added the `mapper3.rendered_chr_bank_switch` probe to the AI observability catalog.

## [0.3.43] - 2026-06-20

### Added
- Added the `ppu_attribute_quadrant_signature` diagnostic cartridge fixture with host-sampled attribute-table quadrant palette telemetry.
- Added the `ppu_attribute_quadrant_fault` scenario-suite fixture and AI route so automated debugging can localize attribute quadrant palette regressions to `ppu.attribute_quadrant`.

### Changed
- Bumped diagnostic telemetry to schema 72 and scenario-suite artifacts to schema 23 for the new 45-scenario, 37-route AI corpus.
- Hardened diagnostic AI fix-handoff generation so stale aggregate e2e reports are warnings instead of blocking per-route handoff creation.
- Updated the PPU pixel-pipeline coverage gap to account for deterministic attribute-quadrant coverage while keeping broader tile-fetch and sprite-mux risks explicit.

### Tests
- Added cartridge, CLI, verifier, e2e, and AI route-matrix coverage for the PPU attribute-quadrant fixture and route counts.

## [0.3.42] - 2026-06-20

### Added
- Added the `ppu_pixel_phase_signature` diagnostic cartridge fixture with host-sampled scanline-local background pixel phase telemetry.
- Added the `ppu_pixel_phase_fault` scenario-suite fixture and AI route so automated debugging can localize background pixel phase regressions to `ppu.pixel_phase`.

### Changed
- Bumped diagnostic telemetry to schema 71 and scenario-suite artifacts to schema 22 for the new 44-scenario, 36-route AI corpus.
- Updated the PPU pixel-pipeline coverage gap to account for deterministic pixel-phase coverage while keeping broader tile-fetch, attribute, and sprite-mux risks explicit.

### Tests
- Added cartridge, CLI, verifier, e2e, and AI artifact coverage for the PPU pixel-phase fixture and route counts.

## [0.3.41] - 2026-06-19

### Added
- Added a CPU status/BIT diagnostic cartridge matrix covering BIT zero-page/absolute status behavior plus SEC/CLC, SEI/CLI, SED/CLD, and CLV flag transitions.
- Added the `cpu_status_matrix_fault` scenario-suite fixture and AI route so automated debugging can localize status flag regressions to `cpu.status.bit_flags`.

### Changed
- Bumped diagnostic telemetry to schema 70 and scenario-suite artifacts to schema 21 for the new status/BIT telemetry and 43-scenario, 35-route AI corpus.
- Hardened the diagnostic cartridge profiler so default binary resolution honors `CARGO_TARGET_DIR`, records `config.target_dir`, and fails when sampled telemetry is missing build `version`, `build_type`, or `package_version`.

### Tests
- Added cartridge, CLI, verifier, e2e, and AI artifact coverage for the status/BIT fixture and route counts.
- Added profiler-script integration coverage for `CARGO_TARGET_DIR` plus missing-build-metadata validation.

## [0.3.40] - 2026-06-19

### Added
- Added release-vs-dev build metadata to diagnostic telemetry, triage JSON, bundle manifests, scenario-suite root artifacts, input-sweep artifacts, observability/e2e summaries, and diagnostic profiles.
- Added verifier coverage so AI-facing diagnostic suites must expose consistent `version`, `build_type`, and `package_version` fields.

### Changed
- Bumped diagnostic telemetry to schema 69 and the affected wrapper artifact schemas for bundle, triage, scenario-suite, observer, and input-sweep outputs.
- Included post-`v0.3.39` diagnostic observability work: input-sweep coverage-gap evidence, dev-build CLI version metadata, and build-identifiable diagnostic evidence.

## [0.3.4] - 2026-06-02

### Added
- Added a diagnostic cartridge profiler with JSON, Markdown, and CI release-gate output for headless performance telemetry.

### Changed
- Expanded the PPU scroll seam diagnostic into a four-sample fine-X and vertical scroll matrix.
- Updated diagnostic telemetry schema 38 and release evidence docs to expose the new scroll seam probes.

## [0.3.3] - 2026-06-02

### Added
- Added the headless diagnostic test cartridge, telemetry schema, Markdown report, triage bundle, and scenario-suite observability workflow.
- Added AI-oriented diagnostic indexes, route matrices, debug packets, localization scoring, session plans, and artifact verification.
- Added targeted CPU, PPU, DMA, APU, joypad, and mapper diagnostic fixtures for failure replay and source/test localization.

## [0.3.2] - 2026-05-23

### Security
- Updated the updater TLS dependency path by moving `rustls-webpki` from 0.103.9 to 0.103.13.
- Added `cargo audit` to CI and release workflows so dependency advisories block future releases when they become actionable vulnerabilities.

### Compliance
- Removed the bundled `nestest.nes` binary test fixture and replaced it with generated in-memory CPU program coverage.
- Added an executable IP compliance audit and documented final M4 release-candidate evidence.

### Changed
- Updated `minifb` from 0.27 to 0.28.
- Tightened release checklist validation, issue-template YAML, and public no-ROM guidance.
## [0.3.1] - 2026-04-26

### Added
- Added automatic marquee scrolling for selected long game names in the home screen favorites list, recent games list, ROM browser, and first-run folder setup browser
- Kept row markers, favorite stars, directory prefixes, and file size labels fixed while only the selected title text scrolls

### Changed
- Long unselected game names now truncate with an ellipsis instead of silently cutting off mid-title
- Added a short pause before marquee scrolling starts so readable names do not move immediately

### Fixed
- Fixed long ROM names like "TEENAGE MUTANT NINJA..." being impossible to read completely from the browser list
- Updated the release workflow so platform binaries are staged with their final download names before upload, avoiding ambiguous assets such as a generic `oxidenes` file

### Tests
- Added unit coverage for marquee pause/scroll behavior and character-count-based ellipsis truncation

## [0.3.0] - 2026-04-26

### Changed
- Rebuilt the CRT television frame around a heavier black consumer-TV cabinet instead of the previous flatter silver bezel
- Increased the window and TV canvas to give the cabinet more breathing room against the wall and table
- Redesigned the bottom control panel with a thicker face, centered OxideNES badge, speaker grilles, round front buttons, a round green LED, and a recessed input slot
- Added a more realistic wood tabletop/stand with plank seams, grain variation, contact shadow, and front lip shading
- Simplified the bezel stack after visual review so the screen surround reads as a single heavy CRT shell instead of layered LCD-like trim
- Added subtle CRT tube edge shading so the visible screen has rounded glass corners and more depth without warping gameplay geometry
- Reworked the glass reflection model with softer asymmetric highlights, side reflections, bottom bounce light, and cooler specular tint

### Fixed
- Removed the dynamic shifted-screen glass ghost that could look like lower-screen interference when sprites moved
- Kept the existing custom frame pacer after testing showed a presentation-timing change did not improve the artifact and hurt performance
- Restored the TV/background composition so the cabinet stands farther off the wall with a longer, cleaner shadow instead of clipping at the right edge
- Adjusted the TV proportions so the body feels thicker and heavier, closer to an old CRT television

### Performance
- Avoided the per-frame dynamic glass ghost blend while preserving glass tint and reflection effects
- Kept CRT screen curvature as a precomputed shading table, avoiding extra per-frame geometry work

## [0.2.3] - 2026-04-26

### Performance
- Added a flat CRT scaler fast path for the default non-barrel rendering mode
- Cached horizontally scaled NES rows and reused them during vertical blending
- Reduced `crt_filter` benchmark median from about 8.10ms to about 6.35ms on the local Windows benchmark run

### Tests
- Added byte-for-byte equivalence coverage for the new flat CRT masked/basic paths

## [0.2.2] - 2026-04-12

### Added
- Extracted rendering module (`src/rendering.rs`) with CRT filter, glass effects, bloom, and glow
- Criterion benchmarks for CRT filter and glass effects (`benches/crt_bench.rs`, `benches/glass_bench.rs`)
- 3-level performance overlay toggled with F10 (Off / Basic / Detailed)
- Parallelism regression tests

### Fixed
- Quick overlay menu (LB+RB) hidden under game frame due to unconditional CRT rendering overwriting composite buffer
- Quick overlay menu border clipping text on right side (widened from 20 to 24 tiles)
- Binary name references in README (`nes-emulator` → `oxidenes`)

## [0.2.1] - 2026-04-05

### Performance
- Precomputed fused scanline × vignette lookup table (`sv_table`) eliminating per-pixel multiply+shift in CRT hot loop

### Fixed
- CRT live-preview regression: scanline and vignette sliders now immediately rebuild `sv_table` so the preview is never stale

## [0.2.0] - 2026-04-01

### Performance
- SWAR bilinear interpolation: packs R+B channels into single u32 register, reducing per-pixel multiply count from 12 to 8
- Fused gamma/brightness/contrast LUT into CRT filter loop (eliminates separate 691K-pixel pass)
- Merged phosphor warmth × scanline/vignette into single multiply-shift stage (saves 6 ops/pixel)
- Eliminated ghost buffer memcpy (~2.7MB/frame) by reading ghost pixels directly from CRT output buffer

## [0.1.9] - 2026-03-26

### Performance
- Optimized frame pacer with additive timing to eliminate cumulative drift
- Disabled redundant minifb frame limiter (custom hybrid pacer handles timing exclusively)
- Added early exits for CRT phosphor bloom, scanline glow, and gamma/brightness when parameters are zero
- Eliminated chromatic aberration temp buffer copy (~1ms/frame savings via buffer swap)
- Removed redundant `.min(255)` bounds checks in scanline glow (mathematically proven safe)
- Added call-site guards for glass effects when intensity is zero
- Optimized rewind buffer with bulk byte serialization and pre-allocated snapshots
- Vectorization-friendly rewind deserialization using `chunks_exact` iterator pattern
- Cached PPU greyscale and emphasis flags (eliminates per-pixel mask recomputation)
- Added PPU color emphasis early exit for 61,440 pixels/frame
- Cached Mapper003 CHR bank count (eliminates division on ~200K CHR reads/frame)
- Added Bus cheat loop short-circuit with cached `has_enabled_cheats` flag
- Added APU external audio fast path (skips FP multiply when output is zero)

## [0.1.7] - 2026-03-24

### Fixed
- Crash on malformed GitHub API response in auto-updater (replaced unwrap with graceful error handling)
- Crash on corrupt/truncated ROM files with trainer flag (added bounds check before CRC hash)
- Moved version label to subtle lower-right corner (shows v0.x.y instead of prominent UPDATE banner)

### Security
- Restricted Lua scripting sandbox (removed debug library access)
- Added packet length validation tests for netplay (confirmed existing guards are safe)

### Changed
- Redesigned CRT TV bezel with thinner, proportional frame matching real vintage CRT TVs
- Game screen enlarged from 820×616 to 960×720 (integer 3× NES vertical scale)
- Screen fills ~80% of TV face (was ~53%), window reduced to 1100×954

## [0.1.6] - 2026-03-23

### Changed
- Redesigned CRT TV bezel with thinner, proportional frame matching real vintage CRT TVs
- Game screen enlarged from 820×616 to 960×720 (integer 3× NES vertical scale)
- Screen now fills ~80% of TV face (was ~53%)
- Window size reduced from 1200×1060 to 1100×954 for better 1080p compatibility
- Proportionally resized all TV elements (speaker grille, RCA jacks, buttons, badge)
- Console shelf panel reduced from 160px to 110px

## [0.1.5] - 2026-03-23

### Added
- Configurable netplay port (default remains 7777, changeable via in-game menu)
- Keepalive heartbeat every 2 seconds to prevent firewall UDP timeout
- Port picker in netplay submenu (digit entry with validation)

### Changed
- `host()` now uses the configured port instead of hardcoded 7777
- Default join address syncs when port is changed
- Host menu label shows currently configured port

## [0.1.4] - 2026-03-23

### Added
- Silver CRT TV frame with speaker grille, RCA jacks, and buttons for authentic retro aesthetic

### Fixed
- Adjusted CRT screen geometry to fit the new TV frame (screen height, position, panel spacing)

## [0.1.3] - 2026-03-23

### Fixed
- Resolved all clippy warnings for clean CI builds

### Changed
- Added libudev-dev to Linux CI build dependencies

## [0.1.2] - 2026-03-22

### Added
- First-run ROM folder setup with retro-styled UI dialog
- CRT realism enhancements for consumer TV authenticity (phosphor mask, refined scanlines)

### Changed
- Retro CRT TV-style OSD bars with segmented blocks, green phosphor pipe styling
- Full-width OSD bar spanning entire TV screen with universal brightness/contrast icons
- Thin pipe bars for cleaner OSD appearance

### Fixed
- blend_bilinear_rgb operator precedence bug and eliminated per-pixel divisions
- CRT mask visibility and controller input issues
- Distinguished dev and release builds

### Performance
- Optimized CRT pipeline: 18.4ms to 14.7ms per frame (20% faster)
- Eliminated per-pixel divisions and per-frame allocations in CRT rendering
- Single-slab ghost copy with low-intensity tint skip optimization
- Hybrid frame pacer with Windows timer resolution boost

## [0.1.1] - 2026-03-22

### Changed
- Replaced Inno Setup installer with WiX MSI (.msi) format
- MSI supports silent install via `msiexec /i`, desktop shortcut, .nes file association
- Added missing Linux build dependencies for CI (libudev-dev, libx11-dev)
- Fixed crate imports after package rename to oxidenes

## [0.1.0] - 2026-03-19

### Added
- Initial release
- 20 mapper support (0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 19, 24, 26, 34, 66, 69, 71, 79, 85, 206)
- CRT filter with configurable parameters (scanlines, phosphor, vignette, blur, curvature)
- Shadow mask and aperture grille simulation
- Glass reflection with chromatic aberration
- Full input remapping for keyboard and controller (P1 + P2)
- In-app rebinding UI with conflict detection
- Save states with thumbnail previews
- Rewind with VHS tape visual effect
- UDP netplay for local network multiplayer
- Lua scripting engine
- Local achievement system
- Input recording and playback with FM2 export
- ROM database with auto-identification
- Battery SRAM persistence
- Game Genie cheat code support
- Auto-update checker (GitHub Releases)
- Cross-platform CI/CD (Windows, Linux, macOS)
- Windows installer (Inno Setup)
- Built-in file browser
- Controls reference page
