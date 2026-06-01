#!/usr/bin/env python3
"""Execute an AI session-plan route as a consumer smoke test."""

from __future__ import annotations

import argparse
import json
import subprocess
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_SESSION_SMOKE_SCHEMA_VERSION = 1
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


def markdown_cell(value: Any) -> str:
    if value is None:
        return "-"
    return str(value).replace("|", r"\|").replace("\r", " ").replace("\n", " ")


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


def normalized_path_text(value: Any) -> str:
    return value.replace("\\", "/") if isinstance(value, str) else ""


def resolve_artifact_path(suite_dir: Path, original_suite_dir: Any, value: Any) -> Path | None:
    normalized = normalized_path_text(value)
    if not normalized:
        return None
    candidates = [Path(normalized)]
    original = normalized_path_text(original_suite_dir).rstrip("/")
    if original and normalized.startswith(original + "/"):
        candidates.append(suite_dir / normalized[len(original) + 1 :])
    for candidate in candidates:
        if candidate.exists():
            return candidate
    return candidates[-1] if candidates else None


def artifact_present(suite_dir: Path, original_suite_dir: Any, name: str, value: Any) -> bool:
    path = resolve_artifact_path(suite_dir, original_suite_dir, value)
    if path is None:
        return False
    return path.is_dir() if name.endswith("_dir") else path.is_file()


def artifact_record(
    suite_dir: Path,
    original_suite_dir: Any,
    entry: dict[str, Any],
) -> dict[str, Any]:
    name = str(entry.get("name") or "")
    path = entry.get("path")
    resolved = resolve_artifact_path(suite_dir, original_suite_dir, path)
    present = artifact_present(suite_dir, original_suite_dir, name, path)
    return {
        "name": name,
        "path": path if isinstance(path, str) else "",
        "resolved_path": str(resolved) if resolved is not None else "",
        "present": present,
        "required": entry.get("required") is True,
        "reason": entry.get("reason"),
    }


def session_plan_path(args: argparse.Namespace) -> Path:
    return args.session_plan_json or args.suite_dir / "diagnostic-ai-session-plan.json"


def route_sessions(session_plan: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        session
        for session in as_list(session_plan.get("route_sessions"))
        if isinstance(session, dict)
    ]


def select_session(args: argparse.Namespace, session_plan: dict[str, Any]) -> tuple[dict[str, Any], str]:
    sessions = route_sessions(session_plan)
    if args.route_id:
        return find_session(sessions, "route_id", args.route_id), "route-id"
    if args.scenario_id:
        return find_session(sessions, "scenario_id", args.scenario_id), "scenario-id"
    if args.focus_domain:
        return find_session(sessions, "focus_domain", args.focus_domain), "focus-domain"
    if args.probe_id:
        return find_session(sessions, "probe_id", args.probe_id), "probe-id"
    return (sessions[0] if sessions else {}), "top-route"


def find_session(sessions: list[dict[str, Any]], key: str, value: str) -> dict[str, Any]:
    for session in sessions:
        if session.get(key) == value:
            return session
    return {}


def output_artifacts(summary_json: Path, summary_report: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_session_smoke_json": str(summary_json),
        "diagnostic_ai_session_smoke_report": str(summary_report),
    }


def command_records(commands: Any) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for command in as_list(commands):
        if not isinstance(command, dict):
            continue
        argv = [str(item) for item in as_list(command.get("argv"))]
        if not argv:
            continue
        records.append(
            {
                "purpose": command.get("purpose"),
                "text": command.get("text") or " ".join(argv),
                "argv": argv,
            }
        )
    return records


def run_command_group(
    group_name: str,
    commands: list[dict[str, Any]],
    repo_root: Path,
) -> list[dict[str, Any]]:
    results: list[dict[str, Any]] = []
    for index, command in enumerate(commands, start=1):
        result = run_command(
            f"{group_name}_{index}",
            [str(item) for item in as_list(command.get("argv"))],
            repo_root,
        )
        result["purpose"] = command.get("purpose")
        result["text"] = command.get("text")
        results.append(result)
    return results


def command_arg_value(argv: list[Any], flag: str) -> str:
    values = [str(item) for item in argv]
    for index, value in enumerate(values):
        if value == flag and index + 1 < len(values):
            return values[index + 1]
    return ""


def command_path(repo_root: Path, value: str) -> Path:
    path = Path(value)
    return path if path.is_absolute() else repo_root / path


def replay_validation(
    repo_root: Path,
    session: dict[str, Any],
    command: dict[str, Any],
) -> dict[str, Any]:
    bundle_dir = command_arg_value(as_list(command.get("argv")), "--bundle-dir")
    triage_path = command_path(repo_root, bundle_dir) / "triage.json" if bundle_dir else None
    triage = load_json(triage_path) if triage_path is not None else {}
    debug_focus = as_dict(triage.get("debug_focus"))
    failed_probe_ids = [
        item for item in as_list(debug_focus.get("failed_probe_ids")) if isinstance(item, str)
    ]
    expected_exit_code = triage.get("recommended_exit_code")
    exit_code_matches = command.get("exit_code") == expected_exit_code
    focus_domain_matches = debug_focus.get("focus_domain") == session.get("focus_domain")
    probe_matches = session.get("probe_id") in failed_probe_ids
    valid = (
        bool(triage)
        and exit_code_matches
        and focus_domain_matches
        and probe_matches
    )
    return {
        "bundle_dir": bundle_dir,
        "triage_json": str(triage_path) if triage_path is not None else "",
        "triage_present": bool(triage),
        "recommended_exit_code": expected_exit_code,
        "exit_code_matches_recommended": exit_code_matches,
        "actual_health": triage.get("health"),
        "actual_focus_domain": debug_focus.get("focus_domain"),
        "focus_domain_matches_session": focus_domain_matches,
        "probe_id": session.get("probe_id"),
        "failed_probe_ids": failed_probe_ids,
        "probe_matches_session": probe_matches,
        "passed": valid,
    }


def annotate_replay_results(
    repo_root: Path,
    session: dict[str, Any],
    replay_results: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    annotated: list[dict[str, Any]] = []
    for result in replay_results:
        annotated.append(
            {
                **result,
                "validation": replay_validation(repo_root, session, result),
            }
        )
    return annotated


def replay_passed(command: dict[str, Any]) -> bool:
    return as_dict(command.get("validation")).get("passed") is True


def stop_condition_records(session: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        condition
        for condition in as_list(session.get("stop_conditions"))
        if isinstance(condition, dict)
    ]


def build_summary(
    args: argparse.Namespace,
    repo_root: Path,
    plan_path: Path,
    session_plan: dict[str, Any],
    session: dict[str, Any],
    selection_method: str,
    summary_json: Path,
    summary_report: Path,
    replay_results: list[dict[str, Any]],
    narrow_test_results: list[dict[str, Any]],
) -> dict[str, Any]:
    commands = as_dict(session.get("commands"))
    replay_commands = command_records(commands.get("replay"))
    narrow_test_commands = command_records(commands.get("narrow_tests"))
    verification_commands = command_records(commands.get("verification"))
    read_order = [
        artifact_record(args.suite_dir, session_plan.get("suite_dir"), entry)
        for entry in as_list(session.get("read_order"))
        if isinstance(entry, dict)
    ]
    required_missing = [
        row.get("name")
        for row in read_order
        if row.get("required") is True and row.get("present") is not True
    ]
    stop_conditions = stop_condition_records(session)
    failed_plan_stop_conditions = [
        condition.get("name")
        for condition in stop_conditions
        if condition.get("passed") is not True
    ]
    replay_results = annotate_replay_results(repo_root, session, replay_results)
    command_results = [*replay_results, *narrow_test_results]
    failed_commands = [
        command.get("name")
        for command in replay_results
        if not replay_passed(command)
    ]
    failed_commands.extend(
        command.get("name")
        for command in narrow_test_results
        if not command_passed(command)
    )
    errors: list[str] = []
    if session_plan.get("status") != "passed":
        errors.append("session plan status is not passed")
    if not session:
        errors.append("selected route session was not found")
    elif session.get("ready") is not True:
        errors.append("selected route session is not ready")
    if required_missing:
        errors.append(
            "missing read-order artifacts: "
            + ", ".join(str(name) for name in required_missing)
        )
    if not replay_commands:
        errors.append("selected route has no replay commands")
    if not narrow_test_commands:
        errors.append("selected route has no narrow-test commands")
    if not verification_commands:
        errors.append("selected route has no verification commands")
    if failed_plan_stop_conditions:
        errors.append(
            "session plan stop conditions failed: "
            + ", ".join(str(name) for name in failed_plan_stop_conditions)
        )
    if failed_commands:
        errors.append(
            "executed session commands failed: "
            + ", ".join(str(name) for name in failed_commands)
        )

    status = "passed" if not errors else "failed"
    artifacts = {
        **output_artifacts(summary_json, summary_report),
        "diagnostic_ai_session_plan_json": str(plan_path),
    }
    return {
        "diagnostic_ai_session_smoke_schema_version": AI_SESSION_SMOKE_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(args.suite_dir),
        "session_plan_json": str(plan_path),
        "selection": {
            "method": selection_method,
            "route_id": session.get("route_id"),
            "scenario_id": session.get("scenario_id"),
            "focus_domain": session.get("focus_domain"),
            "probe_id": session.get("probe_id"),
        },
        "summary": {
            "read_order_artifact_count": len(read_order),
            "read_order_present_count": sum(1 for row in read_order if row.get("present") is True),
            "replay_command_count": len(replay_commands),
            "replay_passed_count": sum(1 for row in replay_results if replay_passed(row)),
            "narrow_test_command_count": len(narrow_test_commands),
            "narrow_test_passed_count": sum(1 for row in narrow_test_results if command_passed(row)),
            "verification_command_count": len(verification_commands),
            "executed_command_count": len(command_results),
            "executed_passed_count": sum(1 for row in replay_results if replay_passed(row))
            + sum(1 for row in narrow_test_results if command_passed(row)),
            "stop_condition_count": len(stop_conditions),
            "stop_condition_passed_count": sum(
                1 for condition in stop_conditions if condition.get("passed") is True
            ),
        },
        "read_order": read_order,
        "commands": {
            "replay": replay_commands,
            "narrow_tests": narrow_test_commands,
            "verification": verification_commands,
        },
        "executions": {
            "replay": replay_results,
            "narrow_tests": narrow_test_results,
        },
        "session_stop_conditions": stop_conditions,
        "artifacts": artifacts,
        "artifact_presence": {
            "diagnostic_ai_session_smoke_json": True,
            "diagnostic_ai_session_smoke_report": True,
            "diagnostic_ai_session_plan_json": plan_path.is_file(),
        },
        "errors": errors,
        "ai_handoff": [
            "Use this smoke report to prove an automated consumer can execute a route from diagnostic-ai-session-plan.json.",
            "Replay and narrow-test commands are executed directly from the selected session plan route.",
            "Verification commands are recorded for post-edit use but are not executed here to avoid recursive full-suite runs.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    selection = as_dict(summary.get("selection"))
    lines = [
        "# Diagnostic AI Session Smoke",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Route | {markdown_cell(selection.get('route_id'))} |",
        f"| Scenario | {markdown_cell(selection.get('scenario_id'))} |",
        f"| Focus domain | {markdown_cell(selection.get('focus_domain'))} |",
        f"| Probe | {markdown_cell(selection.get('probe_id'))} |",
        f"| Read-order artifacts | {totals.get('read_order_present_count')}/{totals.get('read_order_artifact_count')} |",
        f"| Replay commands | {totals.get('replay_passed_count')}/{totals.get('replay_command_count')} |",
        f"| Narrow-test commands | {totals.get('narrow_test_passed_count')}/{totals.get('narrow_test_command_count')} |",
        f"| Verification commands recorded | {totals.get('verification_command_count')} |",
        f"| Stop conditions | {totals.get('stop_condition_passed_count')}/{totals.get('stop_condition_count')} |",
        "",
        "## Executions",
        "",
        "| Name | Status | Validated | Exit code | Duration seconds | Purpose |",
        "| --- | --- | --- | --- | ---: | --- |",
    ]
    executions = as_dict(summary.get("executions"))
    for command in [*as_list(executions.get("replay")), *as_list(executions.get("narrow_tests"))]:
        if not isinstance(command, dict):
            continue
        validated = (
            replay_passed(command)
            if command.get("name", "").startswith("session_replay_")
            else command_passed(command)
        )
        lines.append(
            f"| {markdown_cell(command.get('name'))} | {command.get('status')} | {validated} | "
            f"{command.get('exit_code')} | {command.get('duration_seconds')} | "
            f"{markdown_cell(command.get('purpose'))} |"
        )
    lines.extend(
        [
            "",
            "## Read Order",
            "",
            "| Name | Present | Resolved path | Reason |",
            "| --- | --- | --- | --- |",
        ]
    )
    for row in as_list(summary.get("read_order")):
        if isinstance(row, dict):
            lines.append(
                f"| {markdown_cell(row.get('name'))} | {row.get('present')} | "
                f"{markdown_cell(row.get('resolved_path'))} | {markdown_cell(row.get('reason'))} |"
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
    selectors = parser.add_mutually_exclusive_group()
    selectors.add_argument("--route-id", help="Session route id to smoke.")
    selectors.add_argument("--scenario-id", help="Session scenario id to smoke.")
    selectors.add_argument("--focus-domain", help="Session focus domain to smoke.")
    selectors.add_argument("--probe-id", help="Session probe id to smoke.")
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write session-smoke JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write session-smoke Markdown. Defaults inside --suite-dir.",
    )
    parser.add_argument("--json", action="store_true", help="Print the session smoke JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    plan_path = session_plan_path(args)
    session_plan = load_json(plan_path)
    session, selection_method = select_session(args, session_plan)
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-session-smoke.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-session-smoke.md"
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)

    commands = as_dict(session.get("commands"))
    replay_commands = command_records(commands.get("replay"))
    narrow_test_commands = command_records(commands.get("narrow_tests"))
    replay_results = run_command_group("session_replay", replay_commands, repo_root)
    narrow_test_results = run_command_group("session_narrow_test", narrow_test_commands, repo_root)
    summary = build_summary(
        args,
        repo_root,
        plan_path,
        session_plan,
        session,
        selection_method,
        summary_json,
        summary_report,
        replay_results,
        narrow_test_results,
    )
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        totals = as_dict(summary.get("summary"))
        selection = as_dict(summary.get("selection"))
        print(
            "Diagnostic AI session smoke "
            f"{summary['status']}: suite={args.suite_dir} "
            f"route={selection.get('route_id')} "
            f"read_order={totals.get('read_order_present_count')}/{totals.get('read_order_artifact_count')} "
            f"replay={totals.get('replay_passed_count')}/{totals.get('replay_command_count')} "
            f"narrow_tests={totals.get('narrow_test_passed_count')}/{totals.get('narrow_test_command_count')} "
            f"verification_commands={totals.get('verification_command_count')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
