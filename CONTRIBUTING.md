# Contributing

Thanks for your interest in contributing!

## Getting Started

1. Fork and clone the repo
2. `cargo build` to verify the build
3. `cargo test` to run the test suite
4. Create a feature branch from `main`

## Development

- Run `cargo clippy -- -D warnings` before submitting
- Add tests for new functionality
- Keep commits focused and well-described
- Follow [docs/IP_COMPLIANCE.md](docs/IP_COMPLIANCE.md): do not commit commercial ROMs, copyrighted screenshots, box art, manuals, music, logos, icons, or trademarked character artwork

## Pull Requests

- Open a PR against `main`
- Describe what changed and why
- Ensure CI passes (build + test on all 3 platforms)

## Code Structure

| File | Purpose |
|------|---------|
| `src/main.rs` | Game loop, rendering, UI, input, menus |
| `src/cpu.rs` | MOS 6502 CPU emulation |
| `src/ppu.rs` | Picture Processing Unit |
| `src/apu.rs` | Audio Processing Unit |
| `src/bus.rs` | Memory bus / address space |
| `src/mapper.rs` | Cartridge mapper implementations |
| `src/cartridge.rs` | ROM loading, header parsing |
| `src/joypad.rs` | Joypad register emulation |
| `src/netplay.rs` | UDP netplay |
| `src/scripting.rs` | Lua scripting engine |
| `src/achievements.rs` | Achievement system |
| `src/recording.rs` | Input recording/playback |
| `src/romdb.rs` | ROM identification database |
| `src/updater.rs` | Auto-update checker |

## Reporting Issues

- Include the ROM name (no ROM files please)
- Include your OS and emulator version
- Screenshots or recordings help!
