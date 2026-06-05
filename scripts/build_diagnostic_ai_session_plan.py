#!/usr/bin/env python3
"""Build a deterministic AI debug session plan from an accepted diagnostic suite."""

from __future__ import annotations

import argparse
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_SESSION_PLAN_SCHEMA_VERSION = 1
EXPECTED_ROUTE_COUNT = 29


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


def row_by_key(rows: Any, key: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for row in as_list(rows):
        if not isinstance(row, dict):
            continue
        value = row.get(key)
        if isinstance(value, str) and value:
            result[value] = row
    return result


def load_artifact_json(value: Any) -> dict[str, Any]:
    if not isinstance(value, str) or not value:
        return {}
    return load_json(Path(value))


def command_group(fix_handoff: dict[str, Any], key: str) -> list[dict[str, Any]]:
    return [
        command
        for command in as_list(as_dict(fix_handoff.get("fix_commands")).get(key))
        if isinstance(command, dict)
    ]


def output_artifacts(summary_json: Path, summary_report: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_session_plan_json": str(summary_json),
        "diagnostic_ai_session_plan_report": str(summary_report),
    }


def artifact_value(*values: Any) -> str:
    for value in values:
        if isinstance(value, str) and value:
            return value
    return ""


def read_order_entries(
    readiness_artifacts: dict[str, Any],
    route_artifacts: dict[str, Any],
    packet_artifacts: dict[str, Any],
) -> list[dict[str, Any]]:
    specs = [
        (
            "debug_packet_manifest",
            "Verify copied packet file list and SHA-256 digests before reading evidence.",
            packet_artifacts.get("diagnostic_ai_debug_packet_manifest"),
            "",
        ),
        (
            "debug_packet_verification",
            "Confirm packet-local verification already passed.",
            readiness_artifacts.get("debug_packet_verification_json"),
            packet_artifacts.get("diagnostic_ai_debug_packet_verification_json"),
        ),
        (
            "source_context",
            "Open bounded source and test windows before editing code.",
            readiness_artifacts.get("debug_packet_source_context"),
            packet_artifacts.get("diagnostic_ai_debug_packet_source_context"),
        ),
        (
            "diagnosis",
            "Read the selected route diagnosis, replay result, and mapped probe.",
            readiness_artifacts.get("diagnosis_json"),
            route_artifacts.get("diagnostic_ai_diagnosis_json"),
        ),
        (
            "fix_handoff",
            "Use the fix-loop commands, source/test anchors, and stop conditions.",
            readiness_artifacts.get("fix_handoff_json"),
            route_artifacts.get("diagnostic_ai_fix_handoff_json"),
        ),
        (
            "route_check",
            "Inspect focused replay and narrow-test evidence for this route.",
            readiness_artifacts.get("route_check_json"),
            route_artifacts.get("route_check_json"),
        ),
        (
            "replay_triage",
            "Use compact failure focus before opening full telemetry.",
            readiness_artifacts.get("replay_triage_json"),
            route_artifacts.get("replay_bundle_triage_json"),
        ),
        (
            "replay_telemetry",
            "Open raw trace and event telemetry only after compact artifacts.",
            readiness_artifacts.get("replay_telemetry_json"),
            route_artifacts.get("replay_bundle_telemetry_json"),
        ),
        (
            "debug_packet",
            "Use the relocatable packet JSON as the route-local evidence manifest.",
            readiness_artifacts.get("debug_packet_json"),
            packet_artifacts.get("diagnostic_ai_debug_packet_json"),
        ),
    ]
    return [
        {
            "name": name,
            "path": artifact_value(primary, fallback),
            "reason": reason,
            "required": True,
        }
        for name, reason, primary, fallback in specs
    ]


def fallback_readiness_rows(
    ai_route_matrix: dict[str, Any],
    ai_debug_packet_matrix: dict[str, Any],
) -> list[dict[str, Any]]:
    packet_rows = row_by_key(ai_debug_packet_matrix.get("routes"), "route_id")
    rows: list[dict[str, Any]] = []
    for index, route_row in enumerate(as_list(ai_route_matrix.get("routes")), start=1):
        if not isinstance(route_row, dict):
            continue
        route_id = route_row.get("route_id")
        packet_row = as_dict(packet_rows.get(route_id))
        identity = as_dict(route_row.get("identity"))
        packet_identity = as_dict(packet_row.get("identity"))
        artifacts = {
            "debug_packet_dir": as_dict(packet_row.get("artifacts")).get(
                "diagnostic_ai_debug_packet_dir"
            ),
            "debug_packet_json": as_dict(packet_row.get("artifacts")).get(
                "diagnostic_ai_debug_packet_json"
            ),
            "debug_packet_source_context": as_dict(packet_row.get("artifacts")).get(
                "diagnostic_ai_debug_packet_source_context"
            ),
            "debug_packet_verification_json": as_dict(packet_row.get("artifacts")).get(
                "diagnostic_ai_debug_packet_verification_json"
            ),
            "diagnosis_json": as_dict(route_row.get("artifacts")).get(
                "diagnostic_ai_diagnosis_json"
            ),
            "fix_handoff_json": as_dict(route_row.get("artifacts")).get(
                "diagnostic_ai_fix_handoff_json"
            ),
            "replay_telemetry_json": as_dict(route_row.get("artifacts")).get(
                "replay_bundle_telemetry_json"
            ),
            "replay_triage_json": as_dict(route_row.get("artifacts")).get(
                "replay_bundle_triage_json"
            ),
            "route_check_json": as_dict(route_row.get("artifacts")).get(
                "route_check_json"
            ),
        }
        ready = (
            route_row.get("status") == "passed"
            and route_row.get("fix_handoff_status") == "passed"
            and route_row.get("replay_status") == "passed"
            and route_row.get("tests_status") == "passed"
            and packet_row.get("status") == "passed"
            and packet_row.get("packet_verification_status") == "passed"
            and identity == packet_identity
            and int(route_row.get("source_match_count") or 0) > 0
            and int(route_row.get("test_match_count") or 0) > 0
            and int(packet_row.get("source_window_count") or 0) > 0
            and int(packet_row.get("test_window_count") or 0) > 0
        )
        rows.append(
            {
                "rank": index,
                "route_id": route_id,
                "scenario_id": identity.get("scenario_id"),
                "focus_domain": identity.get("focus_domain"),
                "probe_id": identity.get("probe_id"),
                "ready": ready,
                "source_match_count": route_row.get("source_match_count"),
                "test_match_count": route_row.get("test_match_count"),
                "source_window_count": packet_row.get("source_window_count"),
                "test_window_count": packet_row.get("test_window_count"),
                "packet_file_count": packet_row.get("packet_file_count"),
                "artifacts": artifacts,
            }
        )
    return rows


def session_stop_conditions(
    readiness_row: dict[str, Any],
    packet_row: dict[str, Any],
    localization_row: dict[str, Any],
    fix_handoff: dict[str, Any],
) -> list[dict[str, Any]]:
    conditions = [
        condition
        for condition in as_list(fix_handoff.get("stop_conditions"))
        if isinstance(condition, dict)
    ]
    conditions.extend(
        [
            {
                "name": "automation_readiness_row_ready",
                "passed": readiness_row.get("ready") is True,
                "detail": readiness_row.get("route_id"),
            },
            {
                "name": "packet_self_verification_passed",
                "passed": packet_row.get("packet_verification_status") == "passed"
                and int(packet_row.get("packet_verifier_digest_mismatch_count") or 0) == 0
                and int(packet_row.get("packet_verifier_check_count") or 0)
                == int(packet_row.get("packet_verifier_passed_check_count") or 0),
                "detail": {
                    "status": packet_row.get("packet_verification_status"),
                    "checks": packet_row.get("packet_verifier_check_count"),
                    "passed_checks": packet_row.get("packet_verifier_passed_check_count"),
                    "digest_mismatches": packet_row.get(
                        "packet_verifier_digest_mismatch_count"
                    ),
                },
            },
            {
                "name": "localization_scorecard_passed",
                "passed": localization_row.get("status") == "passed"
                and float(localization_row.get("score") or 0.0) == 1.0,
                "detail": {
                    "status": localization_row.get("status"),
                    "score": localization_row.get("score"),
                },
            },
        ]
    )
    return conditions


def build_session(
    readiness_row: dict[str, Any],
    route_row: dict[str, Any],
    packet_row: dict[str, Any],
    localization_row: dict[str, Any],
) -> dict[str, Any]:
    readiness_artifacts = as_dict(readiness_row.get("artifacts"))
    route_artifacts = as_dict(route_row.get("artifacts"))
    packet_artifacts = as_dict(packet_row.get("artifacts"))
    fix_handoff = load_artifact_json(
        artifact_value(
            readiness_artifacts.get("fix_handoff_json"),
            route_artifacts.get("diagnostic_ai_fix_handoff_json"),
        )
    )
    read_order = read_order_entries(readiness_artifacts, route_artifacts, packet_artifacts)
    commands = {
        "replay": command_group(fix_handoff, "replay_commands"),
        "narrow_tests": command_group(fix_handoff, "narrow_test_commands"),
        "verification": command_group(fix_handoff, "verification_commands"),
    }
    stop_conditions = session_stop_conditions(
        readiness_row,
        packet_row,
        localization_row,
        fix_handoff,
    )
    errors: list[str] = []
    missing_read_entries = [
        entry.get("name") for entry in read_order if not isinstance(entry.get("path"), str) or not entry.get("path")
    ]
    if missing_read_entries:
        errors.append(f"missing read-order paths: {', '.join(str(name) for name in missing_read_entries)}")
    if not commands["replay"]:
        errors.append("missing replay command")
    if not commands["narrow_tests"]:
        errors.append("missing narrow-test commands")
    if not commands["verification"]:
        errors.append("missing verification commands")
    failed_stop_conditions = [
        condition.get("name")
        for condition in stop_conditions
        if condition.get("passed") is not True
    ]
    if failed_stop_conditions:
        errors.append(
            "failed stop conditions: "
            + ", ".join(str(name) for name in failed_stop_conditions)
        )

    return {
        "rank": readiness_row.get("rank"),
        "route_id": readiness_row.get("route_id"),
        "scenario_id": readiness_row.get("scenario_id"),
        "focus_domain": readiness_row.get("focus_domain"),
        "probe_id": readiness_row.get("probe_id"),
        "ready": readiness_row.get("ready") is True and not errors,
        "status": "passed" if not errors else "failed",
        "source_match_count": readiness_row.get("source_match_count"),
        "test_match_count": readiness_row.get("test_match_count"),
        "source_window_count": readiness_row.get("source_window_count"),
        "test_window_count": readiness_row.get("test_window_count"),
        "packet_file_count": readiness_row.get("packet_file_count"),
        "localization_score": localization_row.get("score"),
        "read_order": read_order,
        "commands": commands,
        "stop_conditions": stop_conditions,
        "artifacts": {
            **readiness_artifacts,
            "route_matrix_diagnosis_json": route_artifacts.get(
                "diagnostic_ai_diagnosis_json"
            ),
            "route_matrix_fix_handoff_json": route_artifacts.get(
                "diagnostic_ai_fix_handoff_json"
            ),
            "packet_matrix_debug_packet_json": packet_artifacts.get(
                "diagnostic_ai_debug_packet_json"
            ),
        },
        "errors": errors,
    }


def build_summary(
    suite_dir: Path,
    summary_json: Path,
    summary_report: Path,
) -> dict[str, Any]:
    ai_index = load_json(suite_dir / "diagnostic-ai-observability-index.json")
    ai_artifact_verification = load_json(suite_dir / "diagnostic-ai-artifact-verification.json")
    ai_route_matrix = load_json(suite_dir / "diagnostic-ai-route-matrix.json")
    ai_debug_packet_matrix = load_json(suite_dir / "diagnostic-ai-debug-packet-matrix.json")
    ai_localization_eval = load_json(suite_dir / "diagnostic-ai-localization-eval.json")

    readiness = as_dict(ai_artifact_verification.get("automation_readiness"))
    readiness_rows = [
        row for row in as_list(readiness.get("routes")) if isinstance(row, dict)
    ]
    if not readiness_rows:
        readiness_rows = fallback_readiness_rows(ai_route_matrix, ai_debug_packet_matrix)
    route_rows = row_by_key(ai_route_matrix.get("routes"), "route_id")
    packet_rows = row_by_key(ai_debug_packet_matrix.get("routes"), "route_id")
    localization_rows = row_by_key(
        ai_localization_eval.get("scenario_scorecards"),
        "route_id",
    )
    sessions = [
        build_session(
            row,
            as_dict(route_rows.get(row.get("route_id"))),
            as_dict(packet_rows.get(row.get("route_id"))),
            as_dict(localization_rows.get(row.get("route_id"))),
        )
        for row in readiness_rows
    ]
    ready_sessions = [session for session in sessions if session.get("ready") is True]
    failed_sessions = [session for session in sessions if session.get("ready") is not True]
    command_count = sum(
        len(as_list(as_dict(session.get("commands")).get(group)))
        for session in sessions
        for group in ("replay", "narrow_tests", "verification")
    )
    read_order_count = sum(len(as_list(session.get("read_order"))) for session in sessions)
    stop_condition_count = sum(len(as_list(session.get("stop_conditions"))) for session in sessions)
    primary = sessions[0] if sessions else {}

    errors: list[str] = []
    if ai_index.get("status") != "passed":
        errors.append("AI index status is not passed")
    if ai_artifact_verification and ai_artifact_verification.get("status") != "passed":
        errors.append("AI artifact verification status is not passed")
    if readiness and readiness.get("status") != "ready":
        errors.append("automation readiness status is not ready")
    if ai_route_matrix.get("status") != "passed":
        errors.append("AI route matrix status is not passed")
    if ai_debug_packet_matrix.get("status") != "passed":
        errors.append("AI debug packet matrix status is not passed")
    if ai_localization_eval.get("status") != "passed":
        errors.append("AI localization evaluation status is not passed")
    if len(sessions) != EXPECTED_ROUTE_COUNT:
        errors.append(f"expected {EXPECTED_ROUTE_COUNT} route sessions, found {len(sessions)}")
    for session in failed_sessions:
        errors.append(f"{session.get('route_id')}: session plan is not ready")

    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_session_plan_schema_version": AI_SESSION_PLAN_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(suite_dir),
        "summary": {
            "route_count": len(sessions),
            "ready_route_count": len(ready_sessions),
            "failed_route_count": len(failed_sessions),
            "primary_route_id": primary.get("route_id"),
            "primary_scenario_id": primary.get("scenario_id"),
            "primary_focus_domain": primary.get("focus_domain"),
            "primary_probe_id": primary.get("probe_id"),
            "command_count": command_count,
            "read_order_artifact_count": read_order_count,
            "stop_condition_count": stop_condition_count,
        },
        "artifacts": {
            **output_artifacts(summary_json, summary_report),
            "diagnostic_e2e_report_json": str(suite_dir / "diagnostic-e2e-report.json"),
            "diagnostic_ai_artifact_verification_json": str(
                suite_dir / "diagnostic-ai-artifact-verification.json"
            ),
            "diagnostic_ai_route_matrix_json": str(
                suite_dir / "diagnostic-ai-route-matrix.json"
            ),
            "diagnostic_ai_debug_packet_matrix_json": str(
                suite_dir / "diagnostic-ai-debug-packet-matrix.json"
            ),
            "diagnostic_ai_localization_eval_json": str(
                suite_dir / "diagnostic-ai-localization-eval.json"
            ),
        },
        "entrypoint": {
            "default_route_id": primary.get("route_id"),
            "default_scenario_id": primary.get("scenario_id"),
            "default_focus_domain": primary.get("focus_domain"),
            "default_probe_id": primary.get("probe_id"),
            "artifact_verification_json": str(
                suite_dir / "diagnostic-ai-artifact-verification.json"
            ),
            "session_plan_json": str(summary_json),
        },
        "operating_rules": [
            "Start with diagnostic-ai-artifact-verification.json and require status=passed before editing emulator code.",
            "Use route_sessions[0] unless a caller supplies a specific route, scenario, focus domain, or probe.",
            "Read each route's read_order artifacts in order before opening full telemetry.",
            "Run replay and narrow_tests commands before editing when reproducing a failure.",
            "After editing, run the route verification commands and the full diagnostic e2e gate.",
            "Stop and repair the artifact graph if any session stop condition becomes false.",
        ],
        "route_sessions": sessions,
        "errors": errors,
        "ai_handoff": [
            "Use this plan as the deterministic startup contract for automated emulator debugging.",
            "Every ready route includes ordered artifacts, replay commands, narrow tests, verification commands, and stop conditions.",
            "If the plan fails, regenerate the diagnostic e2e suite before asking an AI debugger to make source edits.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    lines = [
        "# Diagnostic AI Session Plan",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Routes | {totals.get('ready_route_count')}/{totals.get('route_count')} |",
        f"| Primary route | {markdown_cell(totals.get('primary_route_id'))} |",
        f"| Primary scenario | {markdown_cell(totals.get('primary_scenario_id'))} |",
        f"| Primary focus domain | {markdown_cell(totals.get('primary_focus_domain'))} |",
        f"| Commands | {totals.get('command_count')} |",
        f"| Read-order artifacts | {totals.get('read_order_artifact_count')} |",
        f"| Stop conditions | {totals.get('stop_condition_count')} |",
        "",
        "## Route Sessions",
        "",
        "| Rank | Route | Scenario | Focus domain | Ready | Commands | Read-order artifacts | Stop conditions |",
        "| ---: | --- | --- | --- | --- | ---: | ---: | ---: |",
    ]
    for session in as_list(summary.get("route_sessions")):
        if not isinstance(session, dict):
            continue
        commands = as_dict(session.get("commands"))
        command_count = sum(
            len(as_list(commands.get(group)))
            for group in ("replay", "narrow_tests", "verification")
        )
        lines.append(
            f"| {session.get('rank')} | {markdown_cell(session.get('route_id'))} | "
            f"{markdown_cell(session.get('scenario_id'))} | "
            f"{markdown_cell(session.get('focus_domain'))} | {session.get('ready')} | "
            f"{command_count} | {len(as_list(session.get('read_order')))} | "
            f"{len(as_list(session.get('stop_conditions')))} |"
        )
    lines.extend(["", "## Operating Rules", ""])
    for rule in as_list(summary.get("operating_rules")):
        lines.append(f"- {rule}")
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
        "--summary-json",
        type=Path,
        help="Path to write session-plan JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write session-plan Markdown. Defaults inside --suite-dir.",
    )
    parser.add_argument("--json", action="store_true", help="Print the session plan JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-session-plan.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-session-plan.md"
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)
    summary = build_summary(args.suite_dir, summary_json, summary_report)
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        totals = as_dict(summary.get("summary"))
        print(
            "Diagnostic AI session plan "
            f"{summary['status']}: suite={args.suite_dir} "
            f"routes={totals.get('ready_route_count')}/{totals.get('route_count')} "
            f"commands={totals.get('command_count')} "
            f"read_order={totals.get('read_order_artifact_count')} "
            f"stop_conditions={totals.get('stop_condition_count')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
