# Changelog

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
