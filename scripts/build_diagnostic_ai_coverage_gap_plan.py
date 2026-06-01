#!/usr/bin/env python3
"""Build an AI-ready plan for diagnostic cartridge coverage gaps."""

from __future__ import annotations

import argparse
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_COVERAGE_GAP_PLAN_SCHEMA_VERSION = 1
EXPECTED_COVERAGE_GAP_COUNT = 6

SUBSYSTEM_ALIASES = {
    "apu": ["apu"],
    "cartridge": ["mapper", "cartridge"],
    "cpu": ["cpu", "emulator"],
    "dma": ["dma"],
    "input": ["joypad", "input"],
    "ppu": ["ppu"],
}

SUBSYSTEM_TELEMETRY_HINTS = {
    "apu": ["input_dma_audio", "events", "timeline", "probes", "debug_focus", "coverage_limits"],
    "cartridge": ["probes", "events", "instruction_trace", "debug_focus", "coverage_limits"],
    "cpu": ["instruction_trace", "timeline", "events", "probes", "debug_focus", "coverage_limits"],
    "dma": ["input_dma_audio", "events", "timeline", "probes", "debug_focus", "coverage_limits"],
    "input": ["input_dma_audio", "probes", "timeline", "events", "debug_focus", "coverage_limits"],
    "ppu": ["input_dma_audio", "events", "timeline", "probes", "debug_focus", "coverage_limits"],
}


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    return value if isinstance(value, dict) else {}


def generated_at_utc() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def markdown_cell(value: Any) -> str:
    if value is None:
        return "-"
    return str(value).replace("|", r"\|").replace("\r", " ").replace("\n", " ")


def output_artifacts(summary_json: Path, summary_report: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_coverage_gap_plan_json": str(summary_json),
        "diagnostic_ai_coverage_gap_plan_report": str(summary_report),
    }


def unique_strings(values: list[Any]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if not isinstance(value, str) or not value or value in seen:
            continue
        seen.add(value)
        result.append(value)
    return result


def unique_file_records(records: list[Any]) -> list[dict[str, Any]]:
    by_path: dict[str, dict[str, Any]] = {}
    for record in records:
        item = as_dict(record)
        path = item.get("path")
        if not isinstance(path, str) or not path:
            continue
        by_path[path] = {"path": path, "exists": item.get("exists") is True}
    return [by_path[path] for path in sorted(by_path)]


def unique_commands(records: list[Any]) -> list[dict[str, Any]]:
    by_text: dict[str, dict[str, Any]] = {}
    for record in records:
        item = as_dict(record)
        argv = [str(part) for part in as_list(item.get("argv"))]
        if not argv:
            continue
        text = str(item.get("text") or " ".join(argv))
        by_text[text] = {
            "purpose": item.get("purpose"),
            "text": text,
            "argv": argv,
        }
    return [by_text[key] for key in sorted(by_text)]


def test_command_for_path(path: str) -> dict[str, Any]:
    stem = Path(path).stem
    return {
        "purpose": f"Run current {stem} regression tests while adding this coverage gap fixture.",
        "text": f"cargo test --test {stem}",
        "argv": ["cargo", "test", "--test", stem],
    }


def validation_commands(test_files: list[dict[str, Any]]) -> list[dict[str, Any]]:
    commands = [
        {
            "purpose": "Run the diagnostic cartridge regression suite after adding the new gap fixture.",
            "text": "cargo test --test diagnostic_cartridge_tests",
            "argv": ["cargo", "test", "--test", "diagnostic_cartridge_tests"],
        },
        {
            "purpose": "Regenerate the accepted diagnostic corpus and all AI-facing acceptance artifacts.",
            "text": "python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/scenario-suite",
            "argv": [
                "python",
                "scripts/run_diagnostic_e2e.py",
                "--suite-dir",
                "target/diagnostics/scenario-suite",
            ],
        },
        {
            "purpose": "Strictly verify the AI-facing diagnostic artifact graph after the gap fixture is added.",
            "text": "python scripts/verify_diagnostic_ai_artifacts.py --suite-dir target/diagnostics/scenario-suite --require-e2e-report --require-ai-route-matrix --require-ai-debug-packet --require-ai-debug-packet-matrix",
            "argv": [
                "python",
                "scripts/verify_diagnostic_ai_artifacts.py",
                "--suite-dir",
                "target/diagnostics/scenario-suite",
                "--require-e2e-report",
                "--require-ai-route-matrix",
                "--require-ai-debug-packet",
                "--require-ai-debug-packet-matrix",
            ],
        },
    ]
    commands.extend(
        test_command_for_path(str(record.get("path")))
        for record in test_files
        if isinstance(record.get("path"), str)
        and str(record.get("path", "")).startswith("tests/")
    )
    return unique_commands(commands)


def matching_code_entries(gap: dict[str, Any], code_map: dict[str, Any]) -> list[dict[str, Any]]:
    subsystem = str(gap.get("subsystem") or "")
    aliases = SUBSYSTEM_ALIASES.get(subsystem, [subsystem])
    entries = []
    for entry in as_list(code_map.get("focus_domains")):
        if not isinstance(entry, dict):
            continue
        focus_subsystem = str(entry.get("focus_subsystem") or "")
        focus_domain = str(entry.get("focus_domain") or "")
        if focus_subsystem in aliases or any(focus_domain.startswith(alias + ".") for alias in aliases):
            entries.append(entry)
    return entries


def telemetry_signals(gap: dict[str, Any], telemetry_catalog: dict[str, Any]) -> list[dict[str, Any]]:
    subsystem = str(gap.get("subsystem") or "")
    requested = SUBSYSTEM_TELEMETRY_HINTS.get(
        subsystem,
        ["debug_focus", "probes", "timeline", "events", "coverage_limits"],
    )
    by_id = {
        row.get("id"): row
        for row in as_list(telemetry_catalog.get("signal_families"))
        if isinstance(row, dict) and isinstance(row.get("id"), str)
    }
    return [
        {
            "id": signal_id,
            "title": by_id[signal_id].get("title"),
            "purpose": by_id[signal_id].get("purpose"),
            "telemetry_paths": as_list(by_id[signal_id].get("telemetry_paths")),
            "triage_paths": as_list(by_id[signal_id].get("triage_paths")),
        }
        for signal_id in requested
        if signal_id in by_id
    ]


def build_gap_entry(
    rank: int,
    gap: dict[str, Any],
    code_map: dict[str, Any],
    telemetry_catalog: dict[str, Any],
) -> dict[str, Any]:
    code_entries = matching_code_entries(gap, code_map)
    source_files = unique_file_records(
        [record for entry in code_entries for record in as_list(entry.get("source_files"))]
    )
    test_files = unique_file_records(
        [record for entry in code_entries for record in as_list(entry.get("test_files"))]
    )
    diagnostic_files = unique_file_records(
        [record for entry in code_entries for record in as_list(entry.get("diagnostic_files"))]
    )
    commands = unique_commands(
        [record for entry in code_entries for record in as_list(entry.get("suggested_commands"))]
    )
    signals = telemetry_signals(gap, telemetry_catalog)
    return {
        "rank": rank,
        "gap_id": gap.get("id"),
        "subsystem": gap.get("subsystem"),
        "risk": gap.get("risk"),
        "current_coverage": gap.get("current_coverage"),
        "missing_coverage": gap.get("missing_coverage"),
        "suggested_next_test": gap.get("suggested_next_test"),
        "priority_reason": "Known diagnostic cartridge coverage gap declared by telemetry metadata.",
        "mapped_focus_domains": unique_strings([entry.get("focus_domain") for entry in code_entries]),
        "mapped_scenario_ids": unique_strings(
            [scenario for entry in code_entries for scenario in as_list(entry.get("scenario_ids"))]
        ),
        "mapped_failed_probe_ids": unique_strings(
            [probe for entry in code_entries for probe in as_list(entry.get("failed_probe_ids"))]
        ),
        "source_files": source_files,
        "test_files": test_files,
        "diagnostic_files": diagnostic_files,
        "telemetry_signals": signals,
        "current_regression_commands": commands[:8],
        "acceptance_commands": validation_commands(test_files),
        "next_test_acceptance": [
            "Add or extend an IP-safe generated diagnostic fixture for this gap.",
            "Expose new observations through triage.json and telemetry.json, not only report text.",
            "Add a negative fixture when the behavior can regress independently.",
            "Regenerate the scenario suite and keep strict AI artifact verification passing.",
        ],
        "ready_for_test_design": bool(code_entries and source_files and test_files and signals),
    }


def build_summary(args: argparse.Namespace, summary_json: Path, summary_report: Path) -> dict[str, Any]:
    suite_dir = args.suite_dir
    coverage_ledger = load_json(suite_dir / "diagnostic-coverage-ledger.json")
    code_map = load_json(suite_dir / "diagnostic-code-map.json")
    telemetry_catalog = load_json(suite_dir / "diagnostic-telemetry-catalog.json")
    gaps = [
        gap
        for gap in as_list(coverage_ledger.get("coverage_gaps"))
        if isinstance(gap, dict)
    ]
    gap_entries = [
        build_gap_entry(rank, gap, code_map, telemetry_catalog)
        for rank, gap in enumerate(gaps, start=1)
    ]
    errors: list[str] = []
    if coverage_ledger.get("status") != "passed":
        errors.append("diagnostic coverage ledger status is not passed")
    if code_map.get("status") != "passed":
        errors.append("diagnostic code map status is not passed")
    if telemetry_catalog.get("status") != "passed":
        errors.append("diagnostic telemetry catalog status is not passed")
    if len(gap_entries) != EXPECTED_COVERAGE_GAP_COUNT:
        errors.append(f"expected {EXPECTED_COVERAGE_GAP_COUNT} coverage gaps")
    missing_routes = [entry.get("gap_id") for entry in gap_entries if not entry.get("mapped_focus_domains")]
    missing_sources = [entry.get("gap_id") for entry in gap_entries if not entry.get("source_files")]
    missing_tests = [entry.get("gap_id") for entry in gap_entries if not entry.get("test_files")]
    missing_signals = [entry.get("gap_id") for entry in gap_entries if not entry.get("telemetry_signals")]
    if missing_routes:
        errors.append("coverage gaps without mapped focus domains: " + ", ".join(map(str, missing_routes)))
    if missing_sources:
        errors.append("coverage gaps without source anchors: " + ", ".join(map(str, missing_sources)))
    if missing_tests:
        errors.append("coverage gaps without test anchors: " + ", ".join(map(str, missing_tests)))
    if missing_signals:
        errors.append("coverage gaps without telemetry signals: " + ", ".join(map(str, missing_signals)))

    status = "passed" if not errors else "failed"
    artifacts = {
        **output_artifacts(summary_json, summary_report),
        "diagnostic_coverage_ledger_json": str(suite_dir / "diagnostic-coverage-ledger.json"),
        "diagnostic_code_map_json": str(suite_dir / "diagnostic-code-map.json"),
        "diagnostic_telemetry_catalog_json": str(suite_dir / "diagnostic-telemetry-catalog.json"),
    }
    return {
        "diagnostic_ai_coverage_gap_plan_schema_version": AI_COVERAGE_GAP_PLAN_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(suite_dir),
        "summary": {
            "gap_count": len(gap_entries),
            "expected_gap_count": EXPECTED_COVERAGE_GAP_COUNT,
            "ready_gap_count": sum(1 for entry in gap_entries if entry.get("ready_for_test_design") is True),
            "mapped_gap_count": sum(1 for entry in gap_entries if entry.get("mapped_focus_domains")),
            "source_anchor_gap_count": sum(1 for entry in gap_entries if entry.get("source_files")),
            "test_anchor_gap_count": sum(1 for entry in gap_entries if entry.get("test_files")),
            "telemetry_signal_gap_count": sum(1 for entry in gap_entries if entry.get("telemetry_signals")),
            "validation_command_count": sum(len(as_list(entry.get("acceptance_commands"))) for entry in gap_entries),
            "known_gap_count": coverage_ledger.get("known_gap_count"),
            "only_happy_paths": as_dict(coverage_ledger.get("coverage_posture")).get("only_happy_paths"),
        },
        "gaps": gap_entries,
        "artifacts": artifacts,
        "artifact_presence": {
            "diagnostic_ai_coverage_gap_plan_json": True,
            "diagnostic_ai_coverage_gap_plan_report": True,
            "diagnostic_coverage_ledger_json": (suite_dir / "diagnostic-coverage-ledger.json").is_file(),
            "diagnostic_code_map_json": (suite_dir / "diagnostic-code-map.json").is_file(),
            "diagnostic_telemetry_catalog_json": (suite_dir / "diagnostic-telemetry-catalog.json").is_file(),
        },
        "errors": errors,
        "ai_handoff": [
            "Use this plan when expanding the diagnostic cartridge rather than guessing the next fixture.",
            "Each gap joins known missing coverage to current source/test anchors, telemetry signals, and validation commands.",
            "A passed plan does not claim the gaps are fixed; it proves they are explicit, ranked, and ready for test design.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    lines = [
        "# Diagnostic AI Coverage Gap Plan",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Ready gaps | {totals.get('ready_gap_count')}/{totals.get('gap_count')} |",
        f"| Mapped gaps | {totals.get('mapped_gap_count')}/{totals.get('gap_count')} |",
        f"| Source anchors | {totals.get('source_anchor_gap_count')}/{totals.get('gap_count')} |",
        f"| Test anchors | {totals.get('test_anchor_gap_count')}/{totals.get('gap_count')} |",
        f"| Telemetry signal mappings | {totals.get('telemetry_signal_gap_count')}/{totals.get('gap_count')} |",
        f"| Validation commands | {totals.get('validation_command_count')} |",
        f"| Only happy paths | {totals.get('only_happy_paths')} |",
        "",
        "## Gaps",
        "",
        "| Rank | Gap | Subsystem | Ready | Focus domains | Source files | Test files | Next test |",
        "| ---: | --- | --- | --- | ---: | ---: | ---: | --- |",
    ]
    for gap in as_list(summary.get("gaps")):
        if not isinstance(gap, dict):
            continue
        lines.append(
            f"| {gap.get('rank')} | {markdown_cell(gap.get('gap_id'))} | "
            f"{markdown_cell(gap.get('subsystem'))} | {gap.get('ready_for_test_design')} | "
            f"{len(as_list(gap.get('mapped_focus_domains')))} | "
            f"{len(as_list(gap.get('source_files')))} | "
            f"{len(as_list(gap.get('test_files')))} | "
            f"{markdown_cell(gap.get('suggested_next_test'))} |"
        )
    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(summary.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if summary.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(summary.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--suite-dir",
        required=True,
        type=Path,
        help="Directory produced by scripts/run_diagnostic_observability.py.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write coverage-gap plan JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write coverage-gap plan Markdown. Defaults inside --suite-dir.",
    )
    parser.add_argument("--json", action="store_true", help="Print the coverage-gap plan JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-coverage-gap-plan.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-coverage-gap-plan.md"
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)
    summary = build_summary(args, summary_json, summary_report)
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        totals = as_dict(summary.get("summary"))
        print(
            "Diagnostic AI coverage gap plan "
            f"{summary['status']}: suite={args.suite_dir} "
            f"gaps={totals.get('ready_gap_count')}/{totals.get('gap_count')} "
            f"mapped={totals.get('mapped_gap_count')} "
            f"telemetry={totals.get('telemetry_signal_gap_count')} "
            f"commands={totals.get('validation_command_count')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
