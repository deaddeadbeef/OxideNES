# Diagnostic DMC/OAM Overlap Placement

## Goal

Deepen the diagnostic cartridge's DMA observability by turning DMC/OAM overlap
evidence from a yes/no overlap into transfer-indexed placement telemetry.

## Scope

- Record DMC/OAM overlap offsets inside the active OAM DMA stall window.
- Bucket overlaps into beginning, middle, and end placement classes.
- Expose covered and missing placement buckets in telemetry, triage JSON,
  reports, probes, comparison drift warnings, and observability artifacts.
- Keep middle-placement coverage listed as a remaining DMA gap until the
  cartridge deliberately schedules that case.

## Verification Targets

- `cargo test generated_diagnostic_cartridge_runs_headlessly_to_pass --test diagnostic_cartridge_tests -- --nocapture`
- `cargo test --test diagnostic_cli_tests -- --nocapture`
- `python scripts/run_diagnostic_e2e.py --suite-dir target\diagnostics\scenario-suite-dmc-overlap-placement`
- `python scripts/verify_diagnostic_observability.py --suite-dir target\diagnostics\scenario-suite-dmc-overlap-placement`
