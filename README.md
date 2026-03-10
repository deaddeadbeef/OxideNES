# NES Emulator

A cycle-accurate Nintendo Entertainment System emulator written in Rust, featuring a CRT TV-style interface with scanlines, barrel distortion, and a retro menu system.

## Features

- **14 mappers** — NROM, MMC1, UxROM, CNROM, MMC3, AxROM, MMC2, MMC4, Color Dreams, GxROM, FME-7, Camerica, NINA, Namcot (~85% of commercial NES games)
- **Full APU** — all 5 channels (Pulse×2, Triangle, Noise, DMC) with band-limited resampling
- **CRT simulation** — scanlines, horizontal blur, warm phosphor color, barrel distortion, vignette, screen glare
- **Save states** — 4 slots with quick save/load (F5/F9)
- **Battery SRAM** — automatic save/load for games like Zelda and Final Fantasy
- **Rewind** — hold Backspace to rewind ~5 seconds of gameplay
- **Fast forward** — hold Tab for 4× speed
- **Gamepad support** — Xbox/PlayStation controllers with analog stick, turbo buttons, 2-player
- **Game Genie** — cheat code support
- **Configurable controls** — key bindings saved to config file
- **NTSC/PAL** — region support with correct timing
- **nestest validated** — all official CPU opcodes pass automated testing

## Building

Requires [Rust](https://rustup.rs/) (1.70+).

```bash
git clone https://github.com/user/nes-emulator.git
cd nes-emulator
cargo build --release
```

The binary will be at `target/release/nes-emulator` (or `nes-emulator.exe` on Windows).

## Running

```bash
# Launch with menu (browse for ROMs)
./target/release/nes-emulator

# Launch directly with a ROM
./target/release/nes-emulator path/to/game.nes
```

## Controls

| Action | Keyboard | Gamepad |
|--------|----------|---------|
| D-Pad | W/A/S/D or Arrow Keys | D-Pad / Left Stick |
| A Button | K | South (A/×) |
| B Button | J | West (X/□) |
| Start | Enter | Start |
| Select | Right Shift | Select/Back |
| Turbo A | Z | East (B/○) |
| Turbo B | X | North (Y/△) |

### Emulator Keys

| Key | Action |
|-----|--------|
| **Escape** | Pause menu |
| **Tab** (hold) | Fast forward (4×) |
| **Backspace** (hold) | Rewind (~5s) |
| **F1** | Toggle CRT filter |
| **F2–F4, F6** | Select save slot 1–4 |
| **F5** | Quick save |
| **F8** | Screenshot (saved to ~/.nes-emulator/screenshots/) |
| **F9** | Quick load |
| **F10** | Toggle FPS counter |

## Configuration

Settings are saved to `~/.nes-emulator/config.json`:
- CRT filter on/off
- Barrel distortion on/off
- Audio volume
- Key bindings
- Region (NTSC/PAL)
- Recent games list

## Supported Mappers

| ID | Name | Notable Games |
|----|------|---------------|
| 0 | NROM | Super Mario Bros, Donkey Kong |
| 1 | MMC1 | Legend of Zelda, Metroid, Final Fantasy |
| 2 | UxROM | Mega Man, Castlevania, Contra |
| 3 | CNROM | Gradius, Paperboy |
| 4 | MMC3 | Super Mario Bros 3, Kirby's Adventure |
| 7 | AxROM | Battletoads, Marble Madness |
| 9 | MMC2 | Mike Tyson's Punch-Out!! |
| 10 | MMC4 | Fire Emblem |
| 11 | Color Dreams | Bible Adventures |
| 66 | GxROM | SMB/Duck Hunt combo |
| 69 | FME-7 | Batman: Return of the Joker |
| 71 | Camerica | Micro Machines |
| 79 | NINA | Various unlicensed |
| 206 | Namcot | Various Namco games |

## Testing

```bash
cargo test
```

Runs nestest.nes CPU validation (all official opcodes verified).

## License

MIT
