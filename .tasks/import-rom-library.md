# Import ROM library

## Goal

Add an import feature that can copy or symlink a folder of `.nes` ROMs into a
default fixed library folder used by OxideNES.

## Expected behavior

- Provide a deterministic default library location for imported ROMs.
- Import only NES ROM files from a selected folder.
- Support copy and symlink import modes.
- Keep imported ROMs out of source control.
- Preserve the existing configured `rom_directory` behavior where possible.

## Verification plan

- Focused unit tests for default path resolution and copy/symlink import behavior.
- File-browser or config tests for the default library integration.
- `cargo fmt --check`
- `cargo test`
- `cargo clippy -- -D warnings`
