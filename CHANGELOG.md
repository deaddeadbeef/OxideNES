# Changelog

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
