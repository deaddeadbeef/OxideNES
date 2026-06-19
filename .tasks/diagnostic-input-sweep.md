# Diagnostic Input Sweep

## Goal

Add an optional AI-readable diagnostic artifact that exhaustively validates the
two-port joypad mask matrix, strengthening the current `input_port_matrix`
coverage gap without slowing normal cartridge, scenario-suite, or release runs.

## Acceptance

- `oxidenes-diagnostic --input-sweep-json <FILE>` writes a standalone JSON
  artifact covering all 65,536 joypad-1/joypad-2 mask pairs.
- `--input-sweep-report <FILE>` writes a matching Markdown summary.
- The sweep records strobe-high hold, low-strobe serial bits, and post-eight-read
  behavior for both ports.
- The mode is mutually exclusive with normal diagnostic run outputs and config
  overrides.
- Focused CLI tests, full Rust tests, clippy, formatting, and a dev-binary smoke
  pass before PR.
