# ROM Metadata Policy

OxideNES can use CRC-keyed metadata to identify a user-provided ROM and correct factual cartridge information such as mapper, mirroring, and battery-backed RAM. This metadata exists only for compatibility and diagnostics.

## Scope

Built-in metadata may include only:

- CRC32 of the ROM payload used by OxideNES lookup
- Factual title needed to identify the user's own ROM
- Region
- Mapper number
- Mirroring mode
- PRG and CHR sizes
- Battery-backed RAM flag

Do not add descriptions, review text, marketing copy, publisher/developer claims, genre tags, screenshots, artwork, logos, manuals, music, or ROM data.

## User Overrides

User metadata is preferred for local corrections and unverified variants. OxideNES loads `~/.nes-emulator/romdb.json` after the built-in metadata, so matching CRC entries in the user file override built-in entries. CRC keys are normalized case-insensitively.

Example:

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

## Contributor Rules

- Prefer documenting a user override over expanding the built-in database.
- Add built-in entries only when they fix a concrete compatibility problem that cannot be solved more narrowly.
- Keep third-party names factual and nominative. Do not imply endorsement, partnership, or official status.
- Do not ask contributors or users to attach ROM files, BIOS files, screenshots from commercial games, manuals, or copied metadata dumps.
- Review built-in metadata additions under `docs/IP_COMPLIANCE.md` before merging.
