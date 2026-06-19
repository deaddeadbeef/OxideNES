# OxideNES

[![CI](https://github.com/deaddeadbeef/OxideNES/actions/workflows/ci.yml/badge.svg)](https://github.com/deaddeadbeef/OxideNES/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)

A feature-rich NES-compatible emulator written in Rust.

> **Note:** This emulator does not include any game ROMs. You must provide your own legally obtained .nes ROM files.

> *Screenshot placeholder: launch the emulator with a homebrew or other legally usable `.nes` file to capture product screenshots.*

## Features

- **20 mapper support** (0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 19, 24, 26, 34, 66, 69, 71, 79, 85, 206)
- **CRT simulation** — Scanlines, phosphor warmth, barrel distortion, shadow mask, aperture grille, glass reflections with chromatic aberration
- **Full input remapping** — Keyboard and controller for P1/P2, in-app rebinding UI
- **Save states** with thumbnail previews (4 slots)
- **Rewind** — Hold Backspace to rewind with VHS tape effect
- **Netplay** — Local network multiplayer via UDP
- **Lua scripting** — Write scripts to read memory, draw overlays
- **Achievement system** — Local achievement definitions with unlock notifications
- **Input recording** — Record and playback with FM2 export for TAS
- **ROM database** — CRC-based compatibility metadata with user overrides for mapper, mirroring, and battery fixes
- **Game Genie cheats** — Enter cheat codes in the pause menu
- **Battery save** — Automatic SRAM persistence for cartridges that use SRAM
- **Auto-updater** — Checks GitHub Releases for new versions
- **CRT settings** — Real-time adjustable scanlines, phosphor, vignette, blur, curvature, glass intensity
- **Fullscreen** — Toggle with F11

## Quick Start

### Pre-built Binaries
Download the latest release from the [Releases](../../releases) page. See the [user guide](docs/USER_GUIDE.md) for platform support, first-run setup, Linux dependencies, and data locations.

### Build from Source
```bash
# Prerequisites: Rust toolchain (https://rustup.rs)
cargo build --release
```

### Run
```bash
# With a ROM file
./target/release/oxidenes path/to/game.nes

# Or use the built-in file browser
./target/release/oxidenes

# Import a folder of .nes files into the default library
./target/release/oxidenes --import-roms path/to/rom-folder --import-mode copy

# Or leave ROMs in place and symlink them into the default library
./target/release/oxidenes --import-roms path/to/rom-folder --import-mode symlink
```

On first launch without a configured ROM directory, OxideNES opens a folder setup screen. Select the folder that contains your legally obtained `.nes` files; files are not bundled with the emulator.

## Controls

The table below lists default gameplay bindings. Menu controls, save slots, screenshots, recording, CRT hotkeys, and troubleshooting are covered in the [user guide](docs/USER_GUIDE.md).

| Action | Keyboard (P1) | Keyboard (P2) | Controller |
|--------|---------------|---------------|------------|
| D-Pad | WASD | Arrow Keys | D-Pad / Left Stick |
| A | K | Period | South Button |
| B | J | Comma | West Button |
| Start | Enter | Slash | Start |
| Select | Right Shift | Right Ctrl | Select |
| Turbo A | Z | Semicolon | East Button |
| Turbo B | X | Apostrophe | North Button |

### System Shortcuts
| Key | Action |
|-----|--------|
| Escape | Pause / Menu |
| F5 | Quick Save |
| F9 | Quick Load |
| F11 | Toggle Fullscreen |
| Backspace | Rewind (hold) |
| Tab | Fast Forward (hold) |
| Shift+R | Toggle Recording |
| Shift+P | Toggle Playback |
| Ctrl+R | Reset |
| F10 | Performance Overlay (Off → Basic → Detailed) |

## Configuration

Settings are stored in `%USERPROFILE%\.nes-emulator\config.json` on Windows and `~/.nes-emulator/config.json` on Linux/macOS. They can be edited in-app via the Settings menu.

See [docs/USER_GUIDE.md](docs/USER_GUIDE.md) for save-state, SRAM, screenshot, recording, cheat, achievement, and user metadata paths.

### ROM Metadata

OxideNES includes a small built-in compatibility metadata table and loads user overrides from `~/.nes-emulator/romdb.json`. Metadata is limited to factual cartridge fields used for compatibility, such as mapper, mirroring, PRG/CHR sizes, region, and battery-backed RAM.

User metadata is loaded after the built-in table, so matching CRC entries override built-in entries:

```json
{
  "1234ABCD": {
    "title": "Homebrew Test",
    "region": "US",
    "mapper": 0,
    "mirroring": "horizontal",
    "prg_size": 32768,
    "chr_size": 8192,
    "battery": false
  }
}
```

See [docs/ROM_METADATA_POLICY.md](docs/ROM_METADATA_POLICY.md) before adding or sharing metadata.

## Lua Scripting

Load `.lua` scripts via the `--script` flag:
```bash
oxidenes game.nes --script myscript.lua
```

### API
```lua
nes.read(addr)        -- Read byte from CPU memory
nes.framecount()      -- Current frame number
nes.message(text)     -- Show HUD message
nes.pixel(x, y, color) -- Draw overlay pixel
nes.log(text)         -- Print to stderr
```

## Headless Diagnostics

OxideNES can generate and run an IP-safe diagnostic cartridge headlessly:

```powershell
cargo run --bin oxidenes-diagnostic -- --bundle-dir target/diagnostics/latest-bundle --no-stdout
```

The generated cartridge exercises CPU, bus, PPU, APU, DMA, joypad, and frame/NMI paths, then emits an AI-ready bundle with telemetry JSON, Markdown report, generated cartridge, and a manifest with artifact hashes. See [docs/DIAGNOSTIC_CARTRIDGE.md](docs/DIAGNOSTIC_CARTRIDGE.md).

## Achievements

Place achievement JSON files in `~/.nes-emulator/achievements/{rom_md5}.json`:
```json
{
    "game_title": "Game Name",
    "achievements": [
        {"id": 1, "title": "Achievement", "description": "Do the thing", "points": 10, "conditions": "0xH0075>0"}
    ]
}
```

## Netplay

1. Player 1: Pause → Netplay → Host
2. Player 2: Pause → Netplay → Join → Enter IP:Port
3. Both players must load the same ROM

## Building

### Prerequisites
- Rust 1.70+ (install via [rustup](https://rustup.rs))
- Linux: `sudo apt-get install libudev-dev pkg-config libasound2-dev libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev`
- Windows/macOS: No additional dependencies

### Commands
```bash
cargo build --release    # Build
cargo test               # Run tests
cargo run --release -- game.nes  # Run
```

### Windows Installer
Requires [WiX Toolset](https://wixtoolset.org/):
```bash
# Build MSI installer
cargo build --release
# Then use WiX to compile wix/main.wxs
```

## Support

For installation help, first-run setup, troubleshooting, and the support matrix, read [docs/USER_GUIDE.md](docs/USER_GUIDE.md) and [SUPPORT.md](SUPPORT.md).

Public issues must not include ROMs, BIOS files, save files from commercial games, copyrighted screenshots, box art, manuals, music, logos, or recordings containing proprietary content. Provide the ROM name only when compatibility context is needed.

## Credits & Acknowledgments

- **Synthetic test fixtures** — Generated in `tests/common/synthetic_rom.rs` from deterministic byte patterns, with no bundled ROM content
- **NESdev Wiki** — Invaluable hardware documentation at [nesdev.org](https://www.nesdev.org/wiki/)
- **Rust Crates** — [minifb](https://crates.io/crates/minifb) (windowing), [cpal](https://crates.io/crates/cpal) (audio), [gilrs](https://crates.io/crates/gilrs) (gamepad), [serde](https://crates.io/crates/serde) (config)

## Legal

This is a clean-room NES-compatible emulator implementation. No proprietary Nintendo code, ROMs, BIOS files, artwork, screenshots, logos, manuals, or music are included. Users must supply their own legally obtained `.nes` ROM files.

See [docs/IP_COMPLIANCE.md](docs/IP_COMPLIANCE.md) and [docs/ROM_METADATA_POLICY.md](docs/ROM_METADATA_POLICY.md) for contributor and release rules that keep the public repository safe to distribute.

## License

This project is licensed under the MIT License — see [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
