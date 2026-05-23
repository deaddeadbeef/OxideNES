# Test Fixture Policy

OxideNES test fixtures must remain safe for public open source distribution.

- Do not commit commercial ROMs, BIOS files, game screenshots, art, manuals, music, or copied metadata dumps.
- Prefer generated fixtures from `tests/common/synthetic_rom.rs`.
- Generated fixtures should use deterministic byte patterns that exercise emulator behavior without reproducing third-party content.
- Any binary fixture added in the future must have an explicit redistributable license and a source note in the same directory.