# OxideNES User Guide

OxideNES is a NES-compatible emulator. It does not include game ROMs, BIOS files, screenshots, artwork, music, or other third-party game assets. Use only ROM files that you are legally allowed to use.

## Install

### Pre-built Releases

Download the latest release from the GitHub Releases page. Current release artifact names are:

| Platform | Artifact |
| --- | --- |
| Windows x64 | `oxidenes-windows-x64.exe` and, when published, `oxidenes-windows-installer` |
| Linux x64 | `oxidenes-linux-x64` |
| macOS Apple Silicon | `oxidenes-macos-arm64` |

### Build From Source

Install Rust with `rustup`, then run:

```bash
cargo build --release
```

The release binary is written to `target/release/oxidenes` on Linux/macOS or `target/release/oxidenes.exe` on Windows.

### Linux Dependencies

For Ubuntu or Debian-style systems, install build dependencies before compiling:

```bash
sudo apt-get update
sudo apt-get install -y libudev-dev pkg-config libasound2-dev libxkbcommon-dev libwayland-dev libx11-dev libxcursor-dev
```

Runtime packages vary by distribution, but audio, gamepad, X11, Wayland, cursor, and keyboard-common libraries must be available.

## First Run

When OxideNES starts without a configured ROM directory, it opens a first-run folder setup screen. The browser starts from `~/.nes-emulator/roms` if it exists, then falls back to the user's Downloads folder, then the current directory.

Use the folder setup screen to choose the folder that contains your legally obtained `.nes` files:

| Action | Keyboard | Controller |
| --- | --- | --- |
| Move selection | Arrow keys | D-pad or left stick |
| Open folder | Enter | South/A button |
| Go to parent folder | Backspace or Escape | East/B button |
| Use current folder | Tab | Select |

Files are shown for context in folder setup, but selecting the ROM folder chooses the current directory. After setup, the normal file browser lists `.nes` files and folders from the saved ROM directory.

You can also launch a specific ROM directly:

```bash
oxidenes path/to/game.nes
```

To manage ROM folders inside OxideNES, open **Settings > ROM Library**. The
screen displays the fixed default library root and the currently active root.
Available actions are:

| Action | Result |
| --- | --- |
| Import Folder: Copy | Copies `.nes` files from the selected source folder into the fixed default library and makes it active |
| Import Folder: Symlink | Creates file links in the fixed default library and makes it active |
| Choose Active Folder | Uses the selected folder directly without copying or linking its files |

In either import picker, navigate with the arrow keys and Enter, use Backspace
to move to the parent folder, and press Tab to import the current folder. Escape
cancels the import and returns to ROM Library settings. Existing target files
are left untouched.

The equivalent command-line import is:

```bash
oxidenes --import-roms path/to/rom-folder --import-mode copy
```

The import command copies only `.nes` files into `~/.nes-emulator/roms`, skips
non-ROM entries, leaves existing target files untouched, and updates
`config.json` so the built-in browser opens that default library. Use
`--import-mode symlink` to leave ROM files in the source folder and create links
in the default library instead. Symlink mode depends on the operating system's
permissions for creating file links.

## Controls

Input bindings can be changed in-app from the Settings menu. Defaults are:

| Action | Keyboard P1 | Keyboard P2 | Controller |
| --- | --- | --- | --- |
| D-pad | WASD | Arrow keys | D-pad or left stick |
| A | K | Period | South button |
| B | J | Comma | West button |
| Start | Enter | Slash | Start |
| Select | Right Shift | Right Ctrl | Select |
| Turbo A | Z | Semicolon | East button |
| Turbo B | X | Apostrophe | North button |

Menu controls:

| Action | Keyboard | Controller |
| --- | --- | --- |
| Move | Arrow keys | D-pad or left stick |
| Confirm/open | Enter | South/A button or Start |
| Back | Escape | East/B button |
| Page | Page Up / Page Down | Left/right trigger |
| Favorite selected ROM | F | West/X button |
| Select current folder in setup | Tab | Select |

Gameplay shortcuts:

| Key | Action |
| --- | --- |
| Escape | Pause/menu |
| F1 | Toggle CRT filter |
| F2/F3/F4/F6 | Select save slot 1/2/3/4 |
| F5 | Save state to current slot |
| F7 | Reset loaded game |
| F8 | Save screenshot |
| F9 | Load state from current slot |
| F10 | Cycle performance overlay |
| F11 | Toggle fullscreen |
| F12 | Toggle help overlay |
| Backspace | Rewind while held |
| Tab | Fast forward while held |
| M | Toggle mute |
| - / = | Decrease/increase brightness |
| [ / ] | Decrease/increase contrast |
| Shift+R | Start/stop input recording |
| Shift+P | Start/stop input playback |
| Start+Select held | Return to the main menu from gameplay |

## Data Locations

OxideNES stores user data under the user's home directory:

| Platform | Base directory |
| --- | --- |
| Windows | `%USERPROFILE%\.nes-emulator` |
| Linux/macOS | `~/.nes-emulator` |

Important files and folders:

| Path under base directory | Purpose |
| --- | --- |
| `config.json` | Settings, input bindings, recent games, favorites, selected ROM directory |
| `romdb.json` | Optional user ROM metadata overrides |
| `saves/` | Save states, thumbnails, and battery-backed SRAM files |
| `screenshots/` | PPM screenshots saved with F8 |
| `recordings/` | Input recordings and FM2 exports |
| `cheats/` | Per-ROM cheat-code files |
| `achievements/` | Optional local achievement definitions |

Save-state and SRAM filenames are based on the loaded ROM file stem. If you rename or move a ROM, old saves may not appear until the file stem matches again.

## ROM Metadata

User metadata overrides live in `romdb.json` and are keyed by CRC. Keep entries factual and minimal: mapper, mirroring, PRG/CHR sizes, region, and battery-backed RAM. See [ROM_METADATA_POLICY.md](ROM_METADATA_POLICY.md) before adding or sharing metadata.

## Troubleshooting

### The App Opens To Folder Setup Every Time

Choose a folder with Tab/Select from the folder setup screen. If `config.json` cannot be written, check permissions on the base directory listed above.

### No ROMs Appear In The Browser

The browser shows `.nes` files only. Confirm the selected folder contains files with a `.nes` extension and that OxideNES has permission to read the folder.

### Audio Does Not Play On Linux

Install the Linux dependencies listed above and confirm your user session has access to the active audio device.

### A Controller Is Not Detected

Connect the controller before launching OxideNES, then check the Settings input menu. On Linux, confirm `libudev` is installed and the device is visible to the user session.

### Netplay Does Not Connect

Both players must load the same ROM content and be able to reach each other over UDP. The default port is `7777`; adjust local firewall rules if needed.

### Save States Do Not Load

Save states are tied to the current ROM file stem and the selected save slot. Select the same slot used when saving and keep ROM file names stable.

## Support And No-ROM Policy

When reporting a problem, provide the OxideNES version, operating system, ROM name only, and reproduction steps. Do not upload ROMs, BIOS files, save files from commercial games, copyrighted screenshots, box art, manuals, music, logos, or recordings containing proprietary content. See the root [SUPPORT.md](../SUPPORT.md) file for the full policy.
