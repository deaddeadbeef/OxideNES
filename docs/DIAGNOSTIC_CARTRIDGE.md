# OxideNES Diagnostic Cartridge

OxideNES includes a generated, IP-safe diagnostic cartridge for headless emulator validation. The cartridge is assembled from deterministic 6502 instructions and CHR byte patterns at runtime; no `.nes` file or third-party ROM content is committed.

Run it with:

```powershell
cargo run --bin oxidenes-diagnostic -- --json target/diagnostics/telemetry.json --dump-rom target/diagnostics/oxidenes-diagnostic.nes
```

The runner exits `0` only when the cartridge and host-side checks pass. It exits `1` for diagnostic failures or timeouts, and `2` for CLI/build errors.

## Coverage

The cartridge exercises the emulator through the normal CPU, bus, cartridge, PPU, APU, DMA, and joypad paths:

- CPU arithmetic and flags
- Stack, `JSR`, and `RTS`
- 2 KiB CPU RAM mirroring
- PPU palette register write/read
- OAM DMA from CPU page `$0300`
- APU pulse-channel status register
- Joypad strobe and shift reads
- PPU NMI delivery and rendered frame production

## Telemetry Protocol

The cartridge writes status bytes into CPU RAM:

- `$00F0`: status, `0x01` running, `0x80` pass, `0xE0` fail
- `$00F1`: current test id
- `$00F2`: failure code
- `$00F3`: signature byte, expected `0xA5`
- `$00F4`: NMI count
- `$0200..`: per-test result slots, `0x01` means pass

The host runner adds emulator-side telemetry that the cartridge cannot inspect directly: CPU registers, frame count, RAM checksum, OAM checksum, rendered-frame checksum/color count, audio sample count/peak, and status/frame events.
