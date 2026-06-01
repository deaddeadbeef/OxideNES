#!/usr/bin/env python3
"""Build and verify AI debug packets for every diagnostic route."""

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


AI_DEBUG_PACKET_MATRIX_SCHEMA_VERSION = 1
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


def sorted_route_rows(route_matrix: dict[str, Any]) -> list[dict[str, Any]]:
    rows = [row for row in as_list(route_matrix.get("routes")) if isinstance(row, dict)]
    return sorted(
        rows,
        key=lambda row: (
            row.get("rank") if isinstance(row.get("rank"), int) else 1_000_000,
            str(row.get("route_id") or ""),
        ),
    )


def selection_identity(summary: dict[str, Any]) -> dict[str, Any]:
    selection = as_dict(summary.get("selection"))
    return {
        "route_id": selection.get("route_id"),
        "scenario_id": selection.get("scenario_id"),
        "focus_domain": selection.get("focus_domain"),
        "probe_id": selection.get("probe_id"),
    }


def identities_match(left: dict[str, Any], right: dict[str, Any]) -> bool:
    return all(
        left.get(key) == right.get(key)
        for key in ("route_id", "scenario_id", "focus_domain", "probe_id")
    )


def route_output_artifacts(route_dir: Path) -> dict[str, str]:
    packet_dir = route_dir / "ai-debug-packet"
    return {
        "diagnostic_ai_debug_packet_json": str(route_dir / "diagnostic-ai-debug-packet.json"),
        "diagnostic_ai_debug_packet_report": str(route_dir / "diagnostic-ai-debug-packet.md"),
        "diagnostic_ai_debug_packet_verification_json": str(
            route_dir / "diagnostic-ai-debug-packet-verification.json"
        ),
        "diagnostic_ai_debug_packet_verification_report": str(
            route_dir / "diagnostic-ai-debug-packet-verification.md"
        ),
        "diagnostic_ai_debug_packet_dir": str(packet_dir),
        "diagnostic_ai_debug_packet_manifest": str(packet_dir / "manifest.json"),
        "diagnostic_ai_debug_packet_readme": str(packet_dir / "README.md"),
        "diagnostic_ai_debug_packet_source_context": str(packet_dir / "source-context.json"),
    }


def required_route_artifact_names() -> list[str]:
    return [
        "diagnostic_ai_debug_packet_json",
        "diagnostic_ai_debug_packet_report",
        "diagnostic_ai_debug_packet_verification_json",
        "diagnostic_ai_debug_packet_verification_report",
        "diagnostic_ai_debug_packet_dir",
        "diagnostic_ai_debug_packet_manifest",
        "diagnostic_ai_debug_packet_readme",
        "diagnostic_ai_debug_packet_source_context",
    ]


def debug_packet_command(
    args: argparse.Namespace,
    route_id: str,
    artifacts: dict[str, str],
) -> list[str]:
    route_matrix_json = args.route_matrix_json or args.suite_dir / "diagnostic-ai-route-matrix.json"
    argv = [
        sys.executable,
        script_path("build_diagnostic_ai_debug_packet.py"),
        "--suite-dir",
        str(args.suite_dir),
        "--route-matrix-json",
        str(route_matrix_json),
        "--route-id",
        route_id,
        "--output-dir",
        artifacts["diagnostic_ai_debug_packet_dir"],
        "--summary-json",
        artifacts["diagnostic_ai_debug_packet_json"],
        "--summary-report",
        artifacts["diagnostic_ai_debug_packet_report"],
        "--context-lines",
        str(args.context_lines),
        "--max-anchors-per-file",
        str(args.max_anchors_per_file),
    ]
    return argv


def verify_packet_command(artifacts: dict[str, str]) -> list[str]:
    return [
        sys.executable,
        script_path("verify_diagnostic_ai_debug_packet.py"),
        "--packet-dir",
        artifacts["diagnostic_ai_debug_packet_dir"],
        "--summary-json",
        artifacts["diagnostic_ai_debug_packet_verification_json"],
        "--summary-report",
        artifacts["diagnostic_ai_debug_packet_verification_report"],
    ]


def build_route_row(
    args: argparse.Namespace,
    repo_root: Path,
    route: dict[str, Any],
    output_dir: Path,
) -> dict[str, Any]:
    route_id = str(route.get("route_id") or "")
    route_dir = output_dir / sanitize_path_component(route_id)
    route_dir.mkdir(parents=True, exist_ok=True)
    artifacts = route_output_artifacts(route_dir)
    execution = run_command(
        "build_diagnostic_ai_debug_packet",
        debug_packet_command(args, route_id, artifacts),
        repo_root,
    )
    verification_execution = run_command(
        "verify_diagnostic_ai_debug_packet",
        verify_packet_command(artifacts),
        repo_root,
    )
    packet = load_json(Path(artifacts["diagnostic_ai_debug_packet_json"]))
    packet_verification = load_json(
        Path(artifacts["diagnostic_ai_debug_packet_verification_json"])
    )
    manifest = as_dict(packet.get("packet_manifest"))
    context = as_dict(packet.get("context_summary"))
    verification_summary = as_dict(packet_verification.get("summary"))
    route_identity = as_dict(route.get("identity"))
    packet_identity = selection_identity(packet)
    artifact_flags = artifact_presence(artifacts)
    missing_artifacts = [
        name for name in required_route_artifact_names() if not artifact_flags.get(name)
    ]

    errors: list[str] = []
    if not route_id:
        errors.append("route is missing route_id")
    if execution.get("status") != "passed":
        errors.append("debug packet command did not pass")
    if verification_execution.get("status") != "passed":
        errors.append("debug packet verifier command did not pass")
    if packet.get("status") != "passed":
        errors.append("debug packet status is not passed")
    if packet_verification.get("status") != "passed":
        errors.append("debug packet verification status is not passed")
    if not identities_match(packet_identity, route_identity):
        errors.append("debug packet identity does not match route matrix identity")
    if not identities_match(selection_identity(packet_verification), route_identity):
        errors.append("debug packet verification identity does not match route matrix identity")
    if as_int(manifest.get("missing_required_file_count")) != 0:
        errors.append("debug packet has missing required files")
    if as_int(manifest.get("file_count")) < 1:
        errors.append("debug packet has no packet files")
    if as_int(context.get("source_window_count")) < 1:
        errors.append("debug packet has no source context windows")
    if as_int(context.get("test_window_count")) < 1:
        errors.append("debug packet has no test context windows")
    if not stop_conditions_passed(packet):
        errors.append("debug packet stop conditions did not all pass")
    if missing_artifacts:
        errors.append(f"missing debug packet artifacts: {', '.join(missing_artifacts)}")

    status = "passed" if not errors else "failed"
    return {
        "route_id": route_id,
        "rank": route.get("rank"),
        "focus_domain": route.get("focus_domain"),
        "primary_scenario_id": route.get("primary_scenario_id"),
        "status": status,
        "packet_status": packet.get("status"),
        "packet_verification_status": packet_verification.get("status"),
        "identity": packet_identity,
        "route_matrix_identity": route_identity,
        "identities_match": identities_match(packet_identity, route_identity),
        "packet_file_count": manifest.get("file_count"),
        "packet_verifier_check_count": verification_summary.get("check_count"),
        "packet_verifier_passed_check_count": verification_summary.get(
            "passed_check_count"
        ),
        "packet_verifier_digest_mismatch_count": verification_summary.get(
            "digest_mismatch_count"
        ),
        "missing_required_file_count": manifest.get("missing_required_file_count"),
        "source_window_count": context.get("source_window_count"),
        "test_window_count": context.get("test_window_count"),
        "stop_conditions_passed": stop_conditions_passed(packet),
        "artifacts": artifacts,
        "artifact_presence": artifact_flags,
        "missing_artifacts": missing_artifacts,
        "execution": execution,
        "verification_execution": verification_execution,
        "errors": errors,
    }


def output_artifacts(summary_json: Path, summary_report: Path, output_dir: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_debug_packet_matrix_json": str(summary_json),
        "diagnostic_ai_debug_packet_matrix_report": str(summary_report),
        "diagnostic_ai_debug_packet_matrix_dir": str(output_dir),
    }


def build_summary(
    args: argparse.Namespace,
    repo_root: Path,
    route_matrix_path: Path,
    summary_json: Path,
    summary_report: Path,
    output_dir: Path,
    routes: list[dict[str, Any]],
    route_rows: list[dict[str, Any]],
) -> dict[str, Any]:
    failed_rows = [row for row in route_rows if row.get("status") != "passed"]
    packet_failures = [row for row in route_rows if row.get("packet_status") != "passed"]
    verification_failures = [
        row for row in route_rows if row.get("packet_verification_status") != "passed"
    ]
    identity_failures = [row for row in route_rows if row.get("identities_match") is not True]
    context_failures = [
        row
        for row in route_rows
        if as_int(row.get("source_window_count")) < 1
        or as_int(row.get("test_window_count")) < 1
    ]
    stop_condition_failures = [
        row for row in route_rows if row.get("stop_conditions_passed") is not True
    ]
    missing_artifact_count = sum(len(as_list(row.get("missing_artifacts"))) for row in route_rows)
    artifacts = output_artifacts(summary_json, summary_report, output_dir)
    errors: list[str] = []
    if not routes:
        errors.append("AI route matrix has no routes")
    if len(route_rows) != len(routes):
        errors.append("debug packet row count does not match route count")
    for row in failed_rows:
        errors.append(f"{row.get('route_id')}: {'; '.join(str(error) for error in as_list(row.get('errors')))}")

    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_debug_packet_matrix_schema_version": AI_DEBUG_PACKET_MATRIX_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "git": git_metadata(repo_root),
        "suite_dir": str(args.suite_dir),
        "route_matrix_json": str(route_matrix_path),
        "artifacts": artifacts,
        "artifact_presence": {
            **artifact_presence(artifacts),
            "diagnostic_ai_debug_packet_matrix_json": True,
            "diagnostic_ai_debug_packet_matrix_report": True,
        },
        "summary": {
            "route_count": len(route_rows),
            "expected_route_count": len(routes),
            "passed_route_count": len(route_rows) - len(failed_rows),
            "failed_route_count": len(failed_rows),
            "packet_failure_count": len(packet_failures),
            "packet_verification_failure_count": len(verification_failures),
            "identity_failure_count": len(identity_failures),
            "context_failure_count": len(context_failures),
            "stop_condition_failure_count": len(stop_condition_failures),
            "missing_artifact_count": missing_artifact_count,
            "packet_file_count": sum(as_int(row.get("packet_file_count")) for row in route_rows),
            "packet_verifier_check_count": sum(
                as_int(row.get("packet_verifier_check_count")) for row in route_rows
            ),
            "packet_verifier_passed_check_count": sum(
                as_int(row.get("packet_verifier_passed_check_count")) for row in route_rows
            ),
            "packet_verifier_digest_mismatch_count": sum(
                as_int(row.get("packet_verifier_digest_mismatch_count"))
                for row in route_rows
            ),
            "source_window_count": sum(as_int(row.get("source_window_count")) for row in route_rows),
            "test_window_count": sum(as_int(row.get("test_window_count")) for row in route_rows),
            "context_lines": args.context_lines,
            "max_anchors_per_file": args.max_anchors_per_file,
        },
        "routes": route_rows,
        "failed_routes": [row.get("route_id") for row in failed_rows],
        "errors": errors,
        "ai_handoff": [
            "Use this matrix to prove every AI focus-domain route can be packaged as a self-contained debug packet.",
            "Start with failed_routes when status is failed; otherwise use routes[].artifacts for per-route packet manifests.",
            "Each passed row has a packet-local verifier result, digest-checked files, replay evidence, source/test context windows, and fix-loop commands.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    lines = [
        "# Diagnostic AI Debug Packet Matrix",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Routes | {totals.get('passed_route_count')}/{totals.get('route_count')} |",
        f"| Packet failures | {totals.get('packet_failure_count')} |",
        f"| Packet verification failures | {totals.get('packet_verification_failure_count')} |",
        f"| Identity failures | {totals.get('identity_failure_count')} |",
        f"| Context failures | {totals.get('context_failure_count')} |",
        f"| Stop-condition failures | {totals.get('stop_condition_failure_count')} |",
        f"| Missing artifacts | {totals.get('missing_artifact_count')} |",
        f"| Packet files | {totals.get('packet_file_count')} |",
        f"| Packet verifier checks | {totals.get('packet_verifier_passed_check_count')}/{totals.get('packet_verifier_check_count')} |",
        f"| Packet verifier digest mismatches | {totals.get('packet_verifier_digest_mismatch_count')} |",
        f"| Source windows | {totals.get('source_window_count')} |",
        f"| Test windows | {totals.get('test_window_count')} |",
        "",
        "## Routes",
        "",
        "| Rank | Route | Focus domain | Scenario | Status | Packet verify | Files | Source windows | Test windows | Packet |",
        "| ---: | --- | --- | --- | --- | --- | ---: | ---: | ---: | --- |",
    ]
    for row in as_list(summary.get("routes")):
        if not isinstance(row, dict):
            continue
        artifacts = as_dict(row.get("artifacts"))
        lines.append(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |".format(
                row.get("rank"),
                markdown_cell(row.get("route_id")),
                markdown_cell(row.get("focus_domain")),
                markdown_cell(row.get("primary_scenario_id")),
                row.get("status"),
                row.get("packet_verification_status"),
                row.get("packet_file_count"),
                row.get("source_window_count"),
                row.get("test_window_count"),
                markdown_cell(artifacts.get("diagnostic_ai_debug_packet_json")),
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
        "--route-matrix-json",
        type=Path,
        help="Explicit diagnostic AI route matrix JSON path. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="Directory for per-route debug packets. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write the packet-matrix JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the packet-matrix Markdown report. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--context-lines",
        type=int,
        default=3,
        help="Source/test lines to include before and after each matched anchor.",
    )
    parser.add_argument(
        "--max-anchors-per-file",
        type=int,
        default=8,
        help="Maximum matched anchors to expand per source or test file.",
    )
    parser.add_argument("--json", action="store_true", help="Print the packet matrix JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    route_matrix_path = args.route_matrix_json or args.suite_dir / "diagnostic-ai-route-matrix.json"
    route_matrix = load_json(route_matrix_path)
    output_dir = args.output_dir or args.suite_dir / "ai-debug-packet-matrix"
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-debug-packet-matrix.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-debug-packet-matrix.md"
    output_dir.mkdir(parents=True, exist_ok=True)
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)

    routes = sorted_route_rows(route_matrix)
    route_rows = [build_route_row(args, repo_root, route, output_dir) for route in routes]
    summary = build_summary(
        args,
        repo_root,
        route_matrix_path,
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
            "Diagnostic AI debug packet matrix "
            f"{summary['status']}: suite={args.suite_dir} "
            f"routes={totals.get('passed_route_count')}/{totals.get('route_count')} "
            f"packet_failures={totals.get('packet_failure_count')} "
            f"packet_verification_failures={totals.get('packet_verification_failure_count')} "
            f"context_failures={totals.get('context_failure_count')} "
            f"missing_artifacts={totals.get('missing_artifact_count')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
