# Diagnostic PPU Pixel Phase

## Goal

Reduce the `ppu_pixel_pipeline` coverage gap with a generated, IP-safe
diagnostic cartridge fixture that records a small scanline/window-local pixel
phase signature through host-observed frame samples, then expose it through
telemetry, probes, report text, tests, and AI fault routing.

## Acceptance

- The generated diagnostic cartridge still runs headlessly to pass.
- Telemetry records expected and observed PPU pixel-phase sample values.
- The report and probe catalog expose the pixel-phase fixture separately from
  the existing full-frame checksum and scroll/sprite probes.
- A named negative fixture localizes to a PPU pixel-pipeline focus domain.
- Focused cartridge tests, diagnostic scenario/e2e checks, formatting, full
  Rust tests, clippy, and the diagnostic profile pass before PR.

