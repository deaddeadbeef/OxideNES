#!/usr/bin/env python3
"""Run AI diagnosis and fix-handoff checks for every diagnostic route."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_ROUTE_MATRIX_SCHEMA_VERSION = 1
OUTPUT_TAIL_LINES = 80


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


def command_tail(output: str, limit: int = OUTPUT_TAIL_LINES) -> list[str]:
    return output.splitlines()[-limit:]


def run_command(name: str, argv: list[str], cwd: Path) -> dict[str, Any]:
    started = time.monotonic()
    completed = subprocess.run(
        argv,
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )
    return {
        "name": name,
        "argv": argv,
        "exit_code": completed.returncode,
        "status": "passed" if completed.returncode == 0 else "failed",
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": command_tail(completed.stdout),
        "stderr_tail": command_tail(completed.stderr),
    }


def skipped_command(name: str, reason: str) -> dict[str, Any]:
    return {
        "name": name,
        "argv": [],
        "exit_code": None,
        "status": "skipped",
        "duration_seconds": 0,
        "skip_reason": reason,
        "stdout_tail": [],
        "stderr_tail": [],
    }


def command_passed(command: dict[str, Any]) -> bool:
    return command.get("status") == "passed" and command.get("exit_code") == 0


def run_git(args: list[str], cwd: Path, default: str = "") -> str:
    completed = subprocess.run(
        ["git", *args],
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        return default
    return completed.stdout.strip()


def git_metadata(cwd: Path) -> dict[str, Any]:
    porcelain = run_git(["status", "--porcelain"], cwd)
    return {
        "commit": run_git(["rev-parse", "HEAD"], cwd),
        "short_commit": run_git(["rev-parse", "--short", "HEAD"], cwd),
        "branch": run_git(["branch", "--show-current"], cwd),
        "dirty": bool(porcelain),
    }


def script_path(name: str) -> str:
    return str(Path("scripts") / name)


def sanitize_path_component(value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9_.-]+", "-", value.strip())
    return cleaned.strip(".-") or "route"


def markdown_cell(value: Any) -> str:
    if value is None:
        return "-"
    return str(value).replace("|", r"\|").replace("\r", " ").replace("\n", " ")


def focus_domains(index: dict[str, Any]) -> list[dict[str, Any]]:
    return [domain for domain in as_list(index.get("focus_domains")) if isinstance(domain, dict)]


def sorted_focus_domains(index: dict[str, Any]) -> list[dict[str, Any]]:
    return sorted(
        focus_domains(index),
        key=lambda row: (
            row.get("rank") if isinstance(row.get("rank"), int) else 1_000_000,
            str(row.get("route_id") or ""),
        ),
    )


def output_artifacts(
    summary_json: Path,
    summary_report: Path,
    output_dir: Path,
) -> dict[str, str]:
    return {
        "diagnostic_ai_route_matrix_json": str(summary_json),
        "diagnostic_ai_route_matrix_report": str(summary_report),
        "diagnostic_ai_route_matrix_dir": str(output_dir),
    }


def artifact_present(name: str, value: Any) -> bool:
    if not isinstance(value, str) or not value:
        return False
    path = Path(value)
    return path.is_dir() if name.endswith("_dir") else path.is_file()


def artifact_presence(artifacts: dict[str, str]) -> dict[str, bool]:
    return {name: artifact_present(name, value) for name, value in artifacts.items()}


def stop_conditions_passed(summary: dict[str, Any]) -> bool:
    conditions = [
        condition
        for condition in as_list(summary.get("stop_conditions"))
        if isinstance(condition, dict)
    ]
    return bool(conditions) and all(condition.get("passed") is True for condition in conditions)


def as_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def diagnosis_command(
    args: argparse.Namespace,
    route_id: str,
    output_dir: Path,
    diagnosis_json: Path,
    diagnosis_report: Path,
) -> list[str]:
    argv = [
        sys.executable,
        script_path("run_diagnostic_ai_diagnosis.py"),
        "--suite-dir",
        str(args.suite_dir),
        "--route-id",
        route_id,
        "--output-dir",
        str(output_dir),
        "--summary-json",
        str(diagnosis_json),
        "--summary-report",
        str(diagnosis_report),
        "--cargo",
        args.cargo,
    ]
    if args.skip_tests:
        argv.append("--skip-tests")
    return argv


def fix_handoff_command(
    args: argparse.Namespace,
    diagnosis_json: Path,
    fix_json: Path,
    fix_report: Path,
) -> list[str]:
    return [
        sys.executable,
        script_path("build_diagnostic_ai_fix_handoff.py"),
        "--suite-dir",
        str(args.suite_dir),
        "--diagnosis-json",
        str(diagnosis_json),
        "--summary-json",
        str(fix_json),
        "--summary-report",
        str(fix_report),
    ]


def route_artifacts(route_dir: Path) -> dict[str, str]:
    diagnosis_json = route_dir / "diagnostic-ai-diagnosis.json"
    diagnosis_report = route_dir / "diagnostic-ai-diagnosis.md"
    fix_json = route_dir / "diagnostic-ai-fix-handoff.json"
    fix_report = route_dir / "diagnostic-ai-fix-handoff.md"
    route_check_dir = route_dir / "route-check"
    replay_bundle_dir = route_check_dir / "replay-bundle"
    return {
        "route_output_dir": str(route_dir),
        "diagnostic_ai_diagnosis_json": str(diagnosis_json),
        "diagnostic_ai_diagnosis_report": str(diagnosis_report),
        "diagnostic_ai_fix_handoff_json": str(fix_json),
        "diagnostic_ai_fix_handoff_report": str(fix_report),
        "route_check_json": str(route_check_dir / "diagnostic-route-check.json"),
        "route_check_report": str(route_check_dir / "diagnostic-route-check.md"),
        "replay_bundle_dir": str(replay_bundle_dir),
        "replay_bundle_manifest": str(replay_bundle_dir / "manifest.json"),
        "replay_bundle_triage_json": str(replay_bundle_dir / "triage.json"),
        "replay_bundle_telemetry_json": str(replay_bundle_dir / "telemetry.json"),
        "replay_bundle_report": str(replay_bundle_dir / "report.md"),
        "replay_bundle_rom": str(replay_bundle_dir / "diagnostic.nes"),
    }


def required_route_artifact_names() -> list[str]:
    return [
        "route_output_dir",
        "diagnostic_ai_diagnosis_json",
        "diagnostic_ai_diagnosis_report",
        "diagnostic_ai_fix_handoff_json",
        "diagnostic_ai_fix_handoff_report",
        "route_check_json",
        "route_check_report",
        "replay_bundle_dir",
        "replay_bundle_manifest",
        "replay_bundle_triage_json",
        "replay_bundle_telemetry_json",
        "replay_bundle_report",
        "replay_bundle_rom",
    ]


def selection_identity(summary: dict[str, Any]) -> dict[str, Any]:
    selection = as_dict(summary.get("selection"))
    return {
        "route_id": selection.get("route_id"),
        "scenario_id": selection.get("scenario_id"),
        "focus_domain": selection.get("focus_domain"),
        "probe_id": selection.get("probe_id"),
    }


def build_route_row(
    args: argparse.Namespace,
    repo_root: Path,
    route: dict[str, Any],
    output_dir: Path,
) -> dict[str, Any]:
    route_id = str(route.get("route_id") or "")
    route_dir = output_dir / sanitize_path_component(route_id)
    route_dir.mkdir(parents=True, exist_ok=True)
    artifacts = route_artifacts(route_dir)
    diagnosis_json = Path(artifacts["diagnostic_ai_diagnosis_json"])
    diagnosis_report = Path(artifacts["diagnostic_ai_diagnosis_report"])
    fix_json = Path(artifacts["diagnostic_ai_fix_handoff_json"])
    fix_report = Path(artifacts["diagnostic_ai_fix_handoff_report"])

    diagnosis_execution = run_command(
        "run_diagnostic_ai_diagnosis",
        diagnosis_command(args, route_id, route_dir, diagnosis_json, diagnosis_report),
        repo_root,
    )
    diagnosis = load_json(diagnosis_json)

    if command_passed(diagnosis_execution):
        fix_execution = run_command(
            "build_diagnostic_ai_fix_handoff",
            fix_handoff_command(args, diagnosis_json, fix_json, fix_report),
            repo_root,
        )
    else:
        fix_execution = skipped_command(
            "build_diagnostic_ai_fix_handoff", "diagnosis command failed"
        )
    fix_handoff = load_json(fix_json)
    route_check = as_dict(diagnosis.get("route_check"))
    source_scan = as_dict(fix_handoff.get("source_scan"))
    test_scan = as_dict(fix_handoff.get("test_scan"))
    fix_commands = as_dict(fix_handoff.get("fix_commands"))
    artifact_flags = artifact_presence(artifacts)
    missing_artifacts = [
        name for name in required_route_artifact_names() if not artifact_flags.get(name)
    ]
    diagnosis_identity = selection_identity(diagnosis)
    fix_identity = selection_identity(fix_handoff)
    identities_match = all(
        diagnosis_identity.get(key) == fix_identity.get(key)
        for key in ("route_id", "scenario_id", "focus_domain", "probe_id")
    )
    tests_status_ok = (
        route_check.get("tests_status") == "passed"
        and (args.skip_tests or as_int(route_check.get("test_command_count")) >= 1)
    )
    source_matches = as_int(source_scan.get("source_match_count"))
    test_matches = as_int(test_scan.get("test_match_count"))
    narrow_commands = len(as_list(fix_commands.get("narrow_test_commands")))

    errors: list[str] = []
    if not route_id:
        errors.append("route is missing route_id")
    if diagnosis_execution.get("status") != "passed":
        errors.append("diagnosis command did not pass")
    if diagnosis.get("status") != "passed":
        errors.append("diagnosis status is not passed")
    if route_check.get("replay_status") != "passed":
        errors.append("diagnosis replay status is not passed")
    if not tests_status_ok:
        errors.append("diagnosis narrow tests did not pass")
    if fix_execution.get("status") != "passed":
        errors.append("fix handoff command did not pass")
    if fix_handoff.get("status") != "passed":
        errors.append("fix handoff status is not passed")
    if not identities_match:
        errors.append("diagnosis and fix handoff selections do not match")
    if source_matches < 1:
        errors.append("fix handoff has no source matches")
    if test_matches < 1:
        errors.append("fix handoff has no test matches")
    if narrow_commands < 1 and not args.skip_tests:
        errors.append("fix handoff has no narrow test commands")
    if not stop_conditions_passed(diagnosis):
        errors.append("diagnosis stop conditions did not all pass")
    if not stop_conditions_passed(fix_handoff):
        errors.append("fix handoff stop conditions did not all pass")
    if missing_artifacts:
        errors.append(f"missing route artifacts: {', '.join(missing_artifacts)}")

    status = "passed" if not errors else "failed"
    return {
        "route_id": route_id,
        "rank": route.get("rank"),
        "focus_domain": route.get("focus_domain"),
        "primary_scenario_id": route.get("primary_scenario_id"),
        "status": status,
        "diagnosis_status": diagnosis.get("status"),
        "fix_handoff_status": fix_handoff.get("status"),
        "replay_status": route_check.get("replay_status"),
        "tests_status": route_check.get("tests_status"),
        "tests_skipped": args.skip_tests,
        "test_command_count": route_check.get("test_command_count"),
        "source_match_count": source_matches,
        "test_match_count": test_matches,
        "narrow_test_command_count": narrow_commands,
        "diagnosis_stop_conditions_passed": stop_conditions_passed(diagnosis),
        "fix_handoff_stop_conditions_passed": stop_conditions_passed(fix_handoff),
        "identity": diagnosis_identity,
        "fix_handoff_identity": fix_identity,
        "identities_match": identities_match,
        "artifacts": artifacts,
        "artifact_presence": artifact_flags,
        "missing_artifacts": missing_artifacts,
        "diagnosis_execution": diagnosis_execution,
        "fix_handoff_execution": fix_execution,
        "errors": errors,
    }


def build_summary(
    args: argparse.Namespace,
    repo_root: Path,
    index_json: Path,
    summary_json: Path,
    summary_report: Path,
    output_dir: Path,
    routes: list[dict[str, Any]],
    route_rows: list[dict[str, Any]],
) -> dict[str, Any]:
    failed_rows = [row for row in route_rows if row.get("status") != "passed"]
    diagnosis_failures = [
        row for row in route_rows if row.get("diagnosis_status") != "passed"
    ]
    fix_failures = [
        row for row in route_rows if row.get("fix_handoff_status") != "passed"
    ]
    replay_failures = [row for row in route_rows if row.get("replay_status") != "passed"]
    test_failures = [row for row in route_rows if row.get("tests_status") != "passed"]
    source_match_failures = [
        row for row in route_rows if as_int(row.get("source_match_count")) < 1
    ]
    test_match_failures = [
        row for row in route_rows if as_int(row.get("test_match_count")) < 1
    ]
    narrow_command_failures = [
        row
        for row in route_rows
        if as_int(row.get("narrow_test_command_count")) < 1 and not args.skip_tests
    ]
    stop_condition_failures = [
        row
        for row in route_rows
        if row.get("diagnosis_stop_conditions_passed") is not True
        or row.get("fix_handoff_stop_conditions_passed") is not True
    ]
    missing_artifact_count = sum(len(as_list(row.get("missing_artifacts"))) for row in route_rows)
    artifacts = output_artifacts(summary_json, summary_report, output_dir)
    errors: list[str] = []
    if not routes:
        errors.append("AI index has no focus-domain routes")
    if len(route_rows) != len(routes):
        errors.append("route row count does not match focus-domain count")
    for row in failed_rows:
        errors.append(f"{row.get('route_id')}: {'; '.join(str(error) for error in as_list(row.get('errors')))}")

    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_route_matrix_schema_version": AI_ROUTE_MATRIX_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "git": git_metadata(repo_root),
        "suite_dir": str(args.suite_dir),
        "index_json": str(index_json),
        "artifacts": artifacts,
        "artifact_presence": {
            **artifact_presence(artifacts),
            "diagnostic_ai_route_matrix_json": True,
            "diagnostic_ai_route_matrix_report": True,
        },
        "summary": {
            "route_count": len(route_rows),
            "expected_route_count": len(routes),
            "passed_route_count": len(route_rows) - len(failed_rows),
            "failed_route_count": len(failed_rows),
            "diagnosis_failure_count": len(diagnosis_failures),
            "fix_handoff_failure_count": len(fix_failures),
            "replay_failure_count": len(replay_failures),
            "test_failure_count": len(test_failures),
            "source_match_failure_count": len(source_match_failures),
            "test_match_failure_count": len(test_match_failures),
            "narrow_command_failure_count": len(narrow_command_failures),
            "stop_condition_failure_count": len(stop_condition_failures),
            "missing_artifact_count": missing_artifact_count,
            "tests_skipped": args.skip_tests,
        },
        "routes": route_rows,
        "failed_routes": [row.get("route_id") for row in failed_rows],
        "errors": errors,
        "ai_handoff": [
            "Use this matrix to prove every AI focus-domain route can regenerate a diagnosis and source/test fix handoff.",
            "Start with failed_routes when status is failed; otherwise use routes[].artifacts for per-route diagnosis and fix-handoff files.",
            "Each passed row has replay telemetry, mapped narrow tests, source matches, stop conditions, and concrete fix-loop commands.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    lines = [
        "# Diagnostic AI Route Matrix",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Routes | {totals.get('passed_route_count')}/{totals.get('route_count')} |",
        f"| Diagnosis failures | {totals.get('diagnosis_failure_count')} |",
        f"| Fix-handoff failures | {totals.get('fix_handoff_failure_count')} |",
        f"| Replay failures | {totals.get('replay_failure_count')} |",
        f"| Test failures | {totals.get('test_failure_count')} |",
        f"| Source-match failures | {totals.get('source_match_failure_count')} |",
        f"| Test-match failures | {totals.get('test_match_failure_count')} |",
        f"| Missing artifacts | {totals.get('missing_artifact_count')} |",
        f"| Tests skipped | {totals.get('tests_skipped')} |",
        "",
        "## Routes",
        "",
        "| Rank | Route | Focus domain | Scenario | Probe | Status | Replay | Tests | Source matches | Test matches | Narrow commands |",
        "| ---: | --- | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: |",
    ]
    for row in as_list(summary.get("routes")):
        if not isinstance(row, dict):
            continue
        identity = as_dict(row.get("identity"))
        lines.append(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |".format(
                row.get("rank"),
                markdown_cell(row.get("route_id")),
                markdown_cell(row.get("focus_domain")),
                markdown_cell(row.get("primary_scenario_id")),
                markdown_cell(identity.get("probe_id")),
                row.get("status"),
                row.get("replay_status"),
                row.get("tests_status"),
                row.get("source_match_count"),
                row.get("test_match_count"),
                row.get("narrow_test_command_count"),
            )
        )
    lines.extend(["", "## Artifacts", "", "| Name | Present | Path |", "| --- | --- | --- |"])
    presence = as_dict(summary.get("artifact_presence"))
    for name, artifact_path in as_dict(summary.get("artifacts")).items():
        lines.append(f"| {markdown_cell(name)} | {presence.get(name)} | {markdown_cell(artifact_path)} |")
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
        help="Directory produced by scripts/run_diagnostic_e2e.py.",
    )
    parser.add_argument(
        "--index-json",
        type=Path,
        help="Explicit diagnostic AI index JSON path. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="Directory for per-route diagnosis/fix artifacts. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write the route-matrix JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the route-matrix Markdown report. Defaults inside --suite-dir.",
    )
    parser.add_argument("--cargo", default="cargo", help="Cargo executable to use.")
    parser.add_argument("--skip-tests", action="store_true", help="Only run route replays.")
    parser.add_argument("--json", action="store_true", help="Print the route matrix JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    index_json = args.index_json or args.suite_dir / "diagnostic-ai-observability-index.json"
    index = load_json(index_json)
    output_dir = args.output_dir or args.suite_dir / "ai-route-matrix"
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-route-matrix.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-route-matrix.md"
    output_dir.mkdir(parents=True, exist_ok=True)
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)

    routes = sorted_focus_domains(index)
    route_rows = [build_route_row(args, repo_root, route, output_dir) for route in routes]
    summary = build_summary(
        args,
        repo_root,
        index_json,
        summary_json,
        summary_report,
        output_dir,
        routes,
        route_rows,
    )
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        totals = as_dict(summary.get("summary"))
        print(
            "Diagnostic AI route matrix "
            f"{summary['status']}: suite={args.suite_dir} "
            f"routes={totals.get('passed_route_count')}/{totals.get('route_count')} "
            f"diagnosis_failures={totals.get('diagnosis_failure_count')} "
            f"fix_failures={totals.get('fix_handoff_failure_count')} "
            f"missing_artifacts={totals.get('missing_artifact_count')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
