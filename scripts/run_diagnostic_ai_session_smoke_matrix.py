#!/usr/bin/env python3
"""Execute every AI session-plan route as a consumer smoke matrix."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_SESSION_SMOKE_MATRIX_SCHEMA_VERSION = 1
OUTPUT_TAIL_LINES = 80


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def as_int(value: Any) -> int:
    return value if isinstance(value, int) else 0


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


def command_tail(output: str, limit: int = OUTPUT_TAIL_LINES) -> list[str]:
    return output.splitlines()[-limit:]


def script_path(name: str) -> str:
    return str(Path("scripts") / name)


def safe_component(value: Any) -> str:
    text = str(value or "route")
    return "".join(char if char.isalnum() or char in "._-" else "_" for char in text)


def session_plan_path(args: argparse.Namespace) -> Path:
    return args.session_plan_json or args.suite_dir / "diagnostic-ai-session-plan.json"


def matrix_dir_path(args: argparse.Namespace) -> Path:
    return args.matrix_dir or args.suite_dir / "ai-session-smoke-matrix"


def route_sessions(session_plan: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        session
        for session in as_list(session_plan.get("route_sessions"))
        if isinstance(session, dict)
    ]


def output_artifacts(summary_json: Path, summary_report: Path, matrix_dir: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_session_smoke_matrix_json": str(summary_json),
        "diagnostic_ai_session_smoke_matrix_report": str(summary_report),
        "diagnostic_ai_session_smoke_matrix_dir": str(matrix_dir),
    }


def run_smoke_for_route(
    repo_root: Path,
    args: argparse.Namespace,
    plan_path: Path,
    matrix_dir: Path,
    route_id: str,
) -> dict[str, Any]:
    route_dir = matrix_dir / safe_component(route_id)
    summary_json = route_dir / "diagnostic-ai-session-smoke.json"
    summary_report = route_dir / "diagnostic-ai-session-smoke.md"
    route_dir.mkdir(parents=True, exist_ok=True)
    argv = [
        sys.executable,
        script_path("run_diagnostic_ai_session_smoke.py"),
        "--suite-dir",
        str(args.suite_dir),
        "--session-plan-json",
        str(plan_path),
        "--route-id",
        route_id,
        "--summary-json",
        str(summary_json),
        "--summary-report",
        str(summary_report),
    ]
    started = time.monotonic()
    completed = subprocess.run(
        argv,
        cwd=repo_root,
        text=True,
        capture_output=True,
        check=False,
    )
    smoke = load_json(summary_json)
    summary = as_dict(smoke.get("summary"))
    selection = as_dict(smoke.get("selection"))
    passed = completed.returncode == 0 and smoke.get("status") == "passed"
    return {
        "route_id": route_id,
        "scenario_id": selection.get("scenario_id"),
        "focus_domain": selection.get("focus_domain"),
        "probe_id": selection.get("probe_id"),
        "status": "passed" if passed else "failed",
        "smoke_status": smoke.get("status"),
        "exit_code": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "summary": {
            "read_order_artifact_count": summary.get("read_order_artifact_count"),
            "read_order_present_count": summary.get("read_order_present_count"),
            "replay_command_count": summary.get("replay_command_count"),
            "replay_passed_count": summary.get("replay_passed_count"),
            "narrow_test_command_count": summary.get("narrow_test_command_count"),
            "narrow_test_passed_count": summary.get("narrow_test_passed_count"),
            "verification_command_count": summary.get("verification_command_count"),
            "stop_condition_count": summary.get("stop_condition_count"),
            "stop_condition_passed_count": summary.get("stop_condition_passed_count"),
        },
        "artifacts": {
            "diagnostic_ai_session_smoke_json": str(summary_json),
            "diagnostic_ai_session_smoke_report": str(summary_report),
            "diagnostic_ai_session_smoke_dir": str(route_dir),
        },
        "artifact_presence": {
            "diagnostic_ai_session_smoke_json": summary_json.is_file(),
            "diagnostic_ai_session_smoke_report": summary_report.is_file(),
            "diagnostic_ai_session_smoke_dir": route_dir.is_dir(),
        },
        "errors": as_list(smoke.get("errors")),
        "stdout_tail": command_tail(completed.stdout),
        "stderr_tail": command_tail(completed.stderr),
    }


def build_summary(
    args: argparse.Namespace,
    repo_root: Path,
    plan_path: Path,
    matrix_dir: Path,
    summary_json: Path,
    summary_report: Path,
) -> dict[str, Any]:
    started = time.monotonic()
    session_plan = load_json(plan_path)
    sessions = route_sessions(session_plan)
    rows = [
        run_smoke_for_route(repo_root, args, plan_path, matrix_dir, str(session.get("route_id")))
        for session in sessions
        if session.get("route_id")
    ]
    failed_routes = [row.get("route_id") for row in rows if row.get("status") != "passed"]
    errors: list[str] = []
    if session_plan.get("status") != "passed":
        errors.append("session plan status is not passed")
    if not rows:
        errors.append("session plan has no route sessions")
    if failed_routes:
        errors.append("session smoke routes failed: " + ", ".join(str(route) for route in failed_routes))
    summary_counts = {
        "route_count": len(rows),
        "passed_route_count": sum(1 for row in rows if row.get("status") == "passed"),
        "failed_route_count": len(failed_routes),
        "read_order_artifact_count": sum(
            as_int(as_dict(row.get("summary")).get("read_order_artifact_count"))
            for row in rows
        ),
        "read_order_present_count": sum(
            as_int(as_dict(row.get("summary")).get("read_order_present_count"))
            for row in rows
        ),
        "replay_command_count": sum(
            as_int(as_dict(row.get("summary")).get("replay_command_count"))
            for row in rows
        ),
        "replay_passed_count": sum(
            as_int(as_dict(row.get("summary")).get("replay_passed_count"))
            for row in rows
        ),
        "narrow_test_command_count": sum(
            as_int(as_dict(row.get("summary")).get("narrow_test_command_count"))
            for row in rows
        ),
        "narrow_test_passed_count": sum(
            as_int(as_dict(row.get("summary")).get("narrow_test_passed_count"))
            for row in rows
        ),
        "verification_command_count": sum(
            as_int(as_dict(row.get("summary")).get("verification_command_count"))
            for row in rows
        ),
        "stop_condition_count": sum(
            as_int(as_dict(row.get("summary")).get("stop_condition_count"))
            for row in rows
        ),
        "stop_condition_passed_count": sum(
            as_int(as_dict(row.get("summary")).get("stop_condition_passed_count"))
            for row in rows
        ),
        "duration_seconds": round(time.monotonic() - started, 3),
    }
    status = "passed" if not errors else "failed"
    artifacts = {
        **output_artifacts(summary_json, summary_report, matrix_dir),
        "diagnostic_ai_session_plan_json": str(plan_path),
    }
    return {
        "diagnostic_ai_session_smoke_matrix_schema_version": AI_SESSION_SMOKE_MATRIX_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(args.suite_dir),
        "session_plan_json": str(plan_path),
        "summary": summary_counts,
        "routes": rows,
        "artifacts": artifacts,
        "artifact_presence": {
            "diagnostic_ai_session_smoke_matrix_json": True,
            "diagnostic_ai_session_smoke_matrix_report": True,
            "diagnostic_ai_session_smoke_matrix_dir": matrix_dir.is_dir(),
            "diagnostic_ai_session_plan_json": plan_path.is_file(),
        },
        "errors": errors,
        "ai_handoff": [
            "Use this matrix to prove every diagnostic-ai-session-plan route can be executed by an automated consumer.",
            "Each row points to a per-route diagnostic-ai-session-smoke.json with replay validation and narrow-test command tails.",
            "Verification commands are recorded by each per-route smoke for post-edit use but are not executed in the matrix.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    lines = [
        "# Diagnostic AI Session Smoke Matrix",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Routes | {totals.get('passed_route_count')}/{totals.get('route_count')} |",
        f"| Failed routes | {totals.get('failed_route_count')} |",
        f"| Read-order artifacts | {totals.get('read_order_present_count')}/{totals.get('read_order_artifact_count')} |",
        f"| Replay commands | {totals.get('replay_passed_count')}/{totals.get('replay_command_count')} |",
        f"| Narrow-test commands | {totals.get('narrow_test_passed_count')}/{totals.get('narrow_test_command_count')} |",
        f"| Verification commands recorded | {totals.get('verification_command_count')} |",
        f"| Stop conditions | {totals.get('stop_condition_passed_count')}/{totals.get('stop_condition_count')} |",
        f"| Duration seconds | {totals.get('duration_seconds')} |",
        "",
        "## Routes",
        "",
        "| Route | Status | Scenario | Focus domain | Replay | Narrow tests | Read-order | Stop conditions | Seconds |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | ---: |",
    ]
    for row in as_list(summary.get("routes")):
        if not isinstance(row, dict):
            continue
        row_summary = as_dict(row.get("summary"))
        lines.append(
            f"| {markdown_cell(row.get('route_id'))} | {row.get('status')} | "
            f"{markdown_cell(row.get('scenario_id'))} | {markdown_cell(row.get('focus_domain'))} | "
            f"{row_summary.get('replay_passed_count')}/{row_summary.get('replay_command_count')} | "
            f"{row_summary.get('narrow_test_passed_count')}/{row_summary.get('narrow_test_command_count')} | "
            f"{row_summary.get('read_order_present_count')}/{row_summary.get('read_order_artifact_count')} | "
            f"{row_summary.get('stop_condition_passed_count')}/{row_summary.get('stop_condition_count')} | "
            f"{row.get('duration_seconds')} |"
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
        help="Directory produced by scripts/run_diagnostic_e2e.py.",
    )
    parser.add_argument(
        "--session-plan-json",
        type=Path,
        help="Explicit session-plan JSON path. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--matrix-dir",
        type=Path,
        help="Directory for per-route smoke outputs. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write matrix JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write matrix Markdown. Defaults inside --suite-dir.",
    )
    parser.add_argument("--json", action="store_true", help="Print the matrix JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    plan_path = session_plan_path(args)
    matrix_dir = matrix_dir_path(args)
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-session-smoke-matrix.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-session-smoke-matrix.md"
    matrix_dir.mkdir(parents=True, exist_ok=True)
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)
    summary = build_summary(args, repo_root, plan_path, matrix_dir, summary_json, summary_report)
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        totals = as_dict(summary.get("summary"))
        print(
            "Diagnostic AI session smoke matrix "
            f"{summary['status']}: suite={args.suite_dir} "
            f"routes={totals.get('passed_route_count')}/{totals.get('route_count')} "
            f"replay={totals.get('replay_passed_count')}/{totals.get('replay_command_count')} "
            f"narrow_tests={totals.get('narrow_test_passed_count')}/{totals.get('narrow_test_command_count')} "
            f"read_order={totals.get('read_order_present_count')}/{totals.get('read_order_artifact_count')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
