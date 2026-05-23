# IP Compliance Policy

OxideNES is a clean-room emulator project. The repository and release artifacts must remain safe for public open source distribution.

## Distribution Rules

- Do not commit commercial ROMs, BIOS files, cartridge dumps, copyrighted screenshots, box art, manuals, music, logos, icons, or trademarked character artwork.
- Do not include generated art that imitates protected game characters, game packaging, console trade dress, or third-party logos.
- Do not commit binary ROM fixtures. Prefer generated fixtures from `tests/common/synthetic_rom.rs`.
- User-provided ROMs, save data, scripts, achievements, and metadata belong outside the repository.

## Acceptable References

- Use "NES-compatible" or "NES emulator" only to describe compatibility.
- Third-party game, console, and publisher names may appear only as factual, nominative references when needed for compatibility, issue diagnosis, or user-provided metadata examples.
- Do not use third-party names in a way that suggests endorsement, partnership, or official status.

## Contributor Checklist

Before opening a pull request:

- Confirm that every added binary asset has a redistributable license and is not a ROM, BIOS, save file, screenshot, recording, game asset, or trademarked artwork.
- Confirm that screenshots or recordings use homebrew, public-domain, or original test ROM content.
- Confirm that documentation examples do not encourage downloading ROMs.
- Confirm that release notes do not include proprietary art, copied manual text, or promotional game copy.

## Current Repository Notes

- No binary ROM fixtures are tracked. CPU, mapper, and PPU regression tests use generated in-memory fixtures.
- `src/romdb.rs` contains compatibility metadata and should stay factual, minimal, and user-overridable. Follow `docs/ROM_METADATA_POLICY.md` for user overrides and any proposed built-in metadata changes.
