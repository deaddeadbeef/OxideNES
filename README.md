# 🎮 OxideNES

[![CI](https://github.com/deaddeadbeef/OxideNES/actions/workflows/ci.yml/badge.svg)](https://github.com/deaddeadbeef/OxideNES/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org/)

A feature-rich NES (Nintendo Entertainment System) emulator written in Rust.

> **Note:** This emulator does not include any game ROMs. You must provide your own legally obtained .nes ROM files.

> *Screenshot: Launch the emulator and drop a .nes ROM file to start playing!*

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
- **ROM database** — Auto-identifies ~50 popular games, fixes bad headers
- **Game Genie cheats** — Enter cheat codes in the pause menu
- **Battery save** — Automatic SRAM persistence for games like Zelda
- **Auto-updater** — Checks GitHub Releases for new versions
- **CRT settings** — Real-time adjustable scanlines, phosphor, vignette, blur, curvature, glass intensity
- **Fullscreen** — Toggle with F11

## Quick Start

### Pre-built Binaries
Download the latest release from the [Releases](../../releases) page.

### Build from Source
```bash
# Prerequisites: Rust toolchain (https://rustup.rs)
cargo build --release
```

### Run
```bash
# With a ROM file
./target/release/nes-emulator path/to/game.nes

# Or use the built-in file browser
./target/release/nes-emulator
```

## Controls

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

## Configuration

Settings are stored in `~/.oxidenes/config.json` and can be edited in-app via the Settings menu.

## Lua Scripting

Place `.lua` scripts in `~/.oxidenes/scripts/` or load via `--script` flag:
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

## Achievements

Place achievement JSON files in `~/.oxidenes/achievements/{rom_md5}.json`:
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
- Linux: `sudo apt install libasound2-dev libxkbcommon-dev libwayland-dev`
- Windows/macOS: No additional dependencies

### Commands
```bash
cargo build --release    # Build
cargo test               # Run tests
cargo run --release -- game.nes  # Run
```

### Windows Installer
Requires [Inno Setup](https://jrsoftware.org/isinfo.php):
```bash
iscc installer/nes-emulator.iss
```

## Credits & Acknowledgments

- **nestest.nes** — CPU test ROM by [Kevin Horton](http://wiki.nesdev.com/w/index.php/Emulator_tests), public domain
- **NESdev Wiki** — Invaluable hardware documentation at [nesdev.org](https://www.nesdev.org/wiki/)
- **Rust Crates** — [minifb](https://crates.io/crates/minifb) (windowing), [cpal](https://crates.io/crates/cpal) (audio), [gilrs](https://crates.io/crates/gilrs) (gamepad), [serde](https://crates.io/crates/serde) (config)

## Legal

This is a clean-room NES emulator implementation. No proprietary Nintendo code or assets are included. NES ROMs are not provided — users must supply their own legally obtained ROM files.

## License

This project is licensed under the MIT License — see [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
