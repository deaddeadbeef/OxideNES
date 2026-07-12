# Import ROM library

## Goal

Add an import feature that can copy or symlink a folder of `.nes` ROMs into a
default fixed library folder used by OxideNES.

## Expected behavior

- [x] Provide a deterministic default library location for imported ROMs.
- [x] Import only NES ROM files from a selected folder.
- [x] Support copy and symlink import modes.
- [x] Keep imported ROMs out of source control.
- [x] Preserve the existing configured `rom_directory` behavior where possible.
- [x] Expose copy and symlink import from the in-app Settings menu.
- [x] Make the fixed default library active after a successful import.
- [x] Allow an external folder to be designated as the active library root.
- [x] Report import success or failure in the app.

## Verification plan

- Focused unit tests for default path resolution and copy/symlink import behavior.
- File-browser or config tests for the default library integration.
- `cargo fmt --check`
- `cargo test`
- `cargo clippy -- -D warnings`
