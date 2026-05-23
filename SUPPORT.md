# Support

This project accepts public issue reports that can be discussed without uploading or redistributing third-party game assets.

## Support Matrix

| Platform | Status | Notes |
| --- | --- | --- |
| Windows x64 | Supported | CI builds `oxidenes.exe`; Windows installer work is tracked separately. |
| Linux x64 | Supported | CI builds on Ubuntu. Users may need distro-specific audio, input, X11, Wayland, and cursor packages. |
| macOS Apple Silicon | Supported | CI builds `aarch64-apple-darwin`. |
| macOS Intel | Best effort | Not currently published as a release artifact. |
| Other targets | Best effort | Build reports are welcome, but they are not release-blocking. |

## Before Filing An Issue

- Try the latest release or the current `main` branch.
- Check [docs/USER_GUIDE.md](docs/USER_GUIDE.md), especially the first-run, data-location, and troubleshooting sections.
- Reproduce the issue with the smallest set of steps you can.
- If the problem is ROM-specific, provide the ROM name only. Do not attach or link the ROM.

## What To Include

- OxideNES version, or `oxidenes --version` output.
- Operating system and architecture.
- Whether you used a release binary, installer, or source build.
- ROM name only, if relevant.
- Reproduction steps, expected behavior, and actual behavior.
- Logs or terminal output if OxideNES printed an error.
- Screenshots or recordings only when they show homebrew, public-domain, or original test content.

## No-ROM Upload Policy

Do not upload or link:

- ROMs, cartridge dumps, BIOS files, or firmware.
- Save files from commercial games.
- Copyrighted screenshots, gameplay recordings, music, manuals, box art, logos, icons, or trademarked character artwork.
- Archives that contain any of the above, even if they also include logs.

For compatibility triage, a factual game or test-ROM name is enough. Maintainers may ask for header fields, CRC values, mapper numbers, or logs, but not for copyrighted ROM files.

## Security And Abuse Reports

For crashes or malformed input handling, public issues are fine if they do not include restricted assets. If a report needs private coordination, open an issue with a minimal description and say that you need a private maintainer contact path.
