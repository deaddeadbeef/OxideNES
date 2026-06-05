#!/usr/bin/env python3
"""Verify the AI-facing diagnostic artifact graph from an OxideNES suite."""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_ARTIFACT_VERIFICATION_SCHEMA_VERSION = 1
EXPECTED_SCENARIO_COUNT = 36
EXPECTED_ACTIONABLE_SCENARIO_COUNT = 28
EXPECTED_FOCUS_DOMAIN_COUNT = 28
EXPECTED_COVERAGE_GAP_COUNT = 6


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def as_str_list(value: Any) -> list[str]:
    return [item for item in as_list(value) if isinstance(item, str)]


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


def normalized_path_text(value: Any) -> str:
    return value.replace("\\", "/") if isinstance(value, str) else ""


def unique_strings(values: list[Any]) -> list[str]:
    result: list[str] = []
    seen: set[str] = set()
    for value in values:
        if not isinstance(value, str) or not value or value in seen:
            continue
        result.append(value)
        seen.add(value)
    return result


def path_candidates(suite_dir: Path, original_suite_dirs: list[str], value: Any) -> list[Path]:
    normalized = normalized_path_text(value)
    if not normalized:
        return []
    candidates = [Path(normalized)]
    for original_suite_dir in original_suite_dirs:
        original = normalized_path_text(original_suite_dir).rstrip("/")
        if original and normalized.startswith(original + "/"):
            candidates.append(suite_dir / normalized[len(original) + 1 :])
    return candidates


def resolve_artifact_path(
    suite_dir: Path,
    original_suite_dirs: list[str],
    value: Any,
) -> Path | None:
    for candidate in path_candidates(suite_dir, original_suite_dirs, value):
        if candidate.exists():
            return candidate
    candidates = path_candidates(suite_dir, original_suite_dirs, value)
    if candidates:
        return candidates[-1]
    return None


def artifact_exists(
    suite_dir: Path,
    original_suite_dirs: list[str],
    name: str,
    value: Any,
) -> bool:
    path = resolve_artifact_path(suite_dir, original_suite_dirs, value)
    if path is None:
        return False
    return path.is_dir() if name.endswith("_dir") else path.is_file()


def artifact_record(
    suite_dir: Path,
    original_suite_dirs: list[str],
    name: str,
    value: Any,
) -> dict[str, Any]:
    resolved = resolve_artifact_path(suite_dir, original_suite_dirs, value)
    return {
        "name": name,
        "path": str(value) if isinstance(value, str) else "",
        "resolved_path": str(resolved) if resolved is not None else "",
        "present": artifact_exists(suite_dir, original_suite_dirs, name, value),
    }


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def resolve_packet_record_path(
    suite_dir: Path,
    original_suite_dirs: list[str],
    record: dict[str, Any],
) -> Path | None:
    relative_path = record.get("relative_path")
    if isinstance(relative_path, str) and relative_path:
        candidate = suite_dir / relative_path.replace("\\", "/")
        if candidate.exists():
            return candidate
    return resolve_artifact_path(suite_dir, original_suite_dirs, record.get("path"))


def packet_record_valid(
    suite_dir: Path,
    original_suite_dirs: list[str],
    record: dict[str, Any],
) -> bool:
    path = resolve_packet_record_path(suite_dir, original_suite_dirs, record)
    expected_sha = record.get("sha256")
    if path is None or not path.is_file() or not isinstance(expected_sha, str):
        return False
    return sha256_file(path) == expected_sha


def find_by_key(rows: Any, key: str, value: Any) -> dict[str, Any]:
    if not isinstance(value, str):
        return {}
    for row in as_list(rows):
        if isinstance(row, dict) and row.get(key) == value:
            return row
    return {}


def output_artifacts(summary_json: Path, summary_report: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_artifact_verification_json": str(summary_json),
        "diagnostic_ai_artifact_verification_report": str(summary_report),
    }


def artifact_value(*values: Any) -> str:
    for value in values:
        if isinstance(value, str) and value:
            return value
    return ""


def input_artifacts(
    suite_dir: Path,
    ai_index: dict[str, Any],
    ai_coverage_gap_plan: dict[str, Any],
    ai_query: dict[str, Any],
    ai_diagnosis: dict[str, Any],
    ai_fix_handoff: dict[str, Any],
    ai_route_matrix: dict[str, Any],
    ai_debug_packet: dict[str, Any],
    ai_debug_packet_verification: dict[str, Any],
    ai_debug_packet_matrix: dict[str, Any],
    ai_localization_eval: dict[str, Any],
    ai_session_plan: dict[str, Any],
    ai_session_smoke: dict[str, Any],
    ai_session_smoke_matrix: dict[str, Any],
    e2e_report: dict[str, Any],
    check_e2e_report: bool,
    check_ai_route_matrix: bool,
    check_ai_debug_packet: bool,
    check_ai_debug_packet_matrix: bool,
) -> dict[str, str]:
    index_artifacts = as_dict(ai_index.get("artifacts"))
    coverage_gap_plan_artifacts = as_dict(ai_coverage_gap_plan.get("artifacts"))
    query_artifacts = as_dict(ai_query.get("artifacts"))
    diagnosis_artifacts = as_dict(ai_diagnosis.get("artifacts"))
    fix_artifacts = as_dict(ai_fix_handoff.get("artifacts"))
    route_matrix_artifacts = as_dict(ai_route_matrix.get("artifacts"))
    debug_packet_artifacts = as_dict(ai_debug_packet.get("artifacts"))
    debug_packet_verification_artifacts = as_dict(
        ai_debug_packet_verification.get("artifacts")
    )
    debug_packet_matrix_artifacts = as_dict(ai_debug_packet_matrix.get("artifacts"))
    localization_eval_artifacts = as_dict(ai_localization_eval.get("artifacts"))
    session_plan_artifacts = as_dict(ai_session_plan.get("artifacts"))
    session_smoke_artifacts = as_dict(ai_session_smoke.get("artifacts"))
    session_smoke_matrix_artifacts = as_dict(ai_session_smoke_matrix.get("artifacts"))
    e2e_artifacts = as_dict(e2e_report.get("artifacts"))

    artifacts = {
        "diagnostic_ai_index_json": artifact_value(
            index_artifacts.get("diagnostic_ai_index_json"),
            str(suite_dir / "diagnostic-ai-observability-index.json"),
        ),
        "diagnostic_ai_index_report": artifact_value(
            index_artifacts.get("diagnostic_ai_index_report"),
            str(suite_dir / "diagnostic-ai-observability-index.md"),
        ),
        "diagnostic_ai_coverage_gap_plan_json": artifact_value(
            coverage_gap_plan_artifacts.get("diagnostic_ai_coverage_gap_plan_json"),
            str(suite_dir / "diagnostic-ai-coverage-gap-plan.json"),
        ),
        "diagnostic_ai_coverage_gap_plan_report": artifact_value(
            coverage_gap_plan_artifacts.get("diagnostic_ai_coverage_gap_plan_report"),
            str(suite_dir / "diagnostic-ai-coverage-gap-plan.md"),
        ),
        "diagnostic_ai_query_smoke_json": artifact_value(
            query_artifacts.get("diagnostic_ai_query_smoke_json"),
            str(suite_dir / "diagnostic-ai-query-smoke.json"),
        ),
        "diagnostic_ai_query_smoke_report": artifact_value(
            query_artifacts.get("diagnostic_ai_query_smoke_report"),
            str(suite_dir / "diagnostic-ai-query-smoke.md"),
        ),
        "diagnostic_ai_diagnosis_smoke_json": artifact_value(
            diagnosis_artifacts.get("diagnostic_ai_diagnosis_json"),
            str(suite_dir / "diagnostic-ai-diagnosis-smoke.json"),
        ),
        "diagnostic_ai_diagnosis_smoke_report": artifact_value(
            diagnosis_artifacts.get("diagnostic_ai_diagnosis_report"),
            str(suite_dir / "diagnostic-ai-diagnosis-smoke.md"),
        ),
        "diagnosis_route_check_json": artifact_value(diagnosis_artifacts.get("route_check_json")),
        "diagnosis_route_check_report": artifact_value(
            diagnosis_artifacts.get("route_check_report")
        ),
        "diagnosis_replay_bundle_dir": artifact_value(
            diagnosis_artifacts.get("replay_bundle_dir")
        ),
        "diagnosis_replay_bundle_manifest": artifact_value(
            diagnosis_artifacts.get("replay_bundle_manifest")
        ),
        "diagnosis_replay_bundle_triage_json": artifact_value(
            diagnosis_artifacts.get("replay_bundle_triage_json")
        ),
        "diagnosis_replay_bundle_telemetry_json": artifact_value(
            diagnosis_artifacts.get("replay_bundle_telemetry_json")
        ),
        "diagnosis_replay_bundle_report": artifact_value(
            diagnosis_artifacts.get("replay_bundle_report")
        ),
        "diagnosis_replay_bundle_rom": artifact_value(
            diagnosis_artifacts.get("replay_bundle_rom")
        ),
        "diagnostic_ai_fix_handoff_smoke_json": artifact_value(
            fix_artifacts.get("diagnostic_ai_fix_handoff_json"),
            str(suite_dir / "diagnostic-ai-fix-handoff-smoke.json"),
        ),
        "diagnostic_ai_fix_handoff_smoke_report": artifact_value(
            fix_artifacts.get("diagnostic_ai_fix_handoff_report"),
            str(suite_dir / "diagnostic-ai-fix-handoff-smoke.md"),
        ),
        "fix_handoff_diagnosis_json": artifact_value(fix_artifacts.get("diagnosis_json")),
        "fix_handoff_route_check_json": artifact_value(fix_artifacts.get("route_check_json")),
        "fix_handoff_replay_bundle_triage_json": artifact_value(
            fix_artifacts.get("replay_bundle_triage_json")
        ),
        "fix_handoff_replay_bundle_telemetry_json": artifact_value(
            fix_artifacts.get("replay_bundle_telemetry_json")
        ),
    }
    if check_ai_route_matrix:
        artifacts.update(
            {
                "diagnostic_ai_route_matrix_json": artifact_value(
                    route_matrix_artifacts.get("diagnostic_ai_route_matrix_json"),
                    str(suite_dir / "diagnostic-ai-route-matrix.json"),
                ),
                "diagnostic_ai_route_matrix_report": artifact_value(
                    route_matrix_artifacts.get("diagnostic_ai_route_matrix_report"),
                    str(suite_dir / "diagnostic-ai-route-matrix.md"),
                ),
                "diagnostic_ai_route_matrix_dir": artifact_value(
                    route_matrix_artifacts.get("diagnostic_ai_route_matrix_dir"),
                    str(suite_dir / "ai-route-matrix"),
                ),
            }
        )
    if check_ai_debug_packet:
        artifacts.update(
            {
                "diagnostic_ai_debug_packet_json": artifact_value(
                    debug_packet_artifacts.get("diagnostic_ai_debug_packet_json"),
                    str(suite_dir / "diagnostic-ai-debug-packet.json"),
                ),
                "diagnostic_ai_debug_packet_report": artifact_value(
                    debug_packet_artifacts.get("diagnostic_ai_debug_packet_report"),
                    str(suite_dir / "diagnostic-ai-debug-packet.md"),
                ),
                "diagnostic_ai_debug_packet_verification_json": artifact_value(
                    debug_packet_verification_artifacts.get(
                        "diagnostic_ai_debug_packet_verification_json"
                    ),
                    str(suite_dir / "diagnostic-ai-debug-packet-verification.json"),
                ),
                "diagnostic_ai_debug_packet_verification_report": artifact_value(
                    debug_packet_verification_artifacts.get(
                        "diagnostic_ai_debug_packet_verification_report"
                    ),
                    str(suite_dir / "diagnostic-ai-debug-packet-verification.md"),
                ),
                "diagnostic_ai_debug_packet_dir": artifact_value(
                    debug_packet_artifacts.get("diagnostic_ai_debug_packet_dir"),
                    str(suite_dir / "ai-debug-packet"),
                ),
                "diagnostic_ai_debug_packet_manifest": artifact_value(
                    debug_packet_artifacts.get("diagnostic_ai_debug_packet_manifest"),
                    str(suite_dir / "ai-debug-packet" / "manifest.json"),
                ),
                "diagnostic_ai_debug_packet_readme": artifact_value(
                    debug_packet_artifacts.get("diagnostic_ai_debug_packet_readme"),
                    str(suite_dir / "ai-debug-packet" / "README.md"),
                ),
                "diagnostic_ai_debug_packet_source_context": artifact_value(
                    debug_packet_artifacts.get("diagnostic_ai_debug_packet_source_context"),
                    str(suite_dir / "ai-debug-packet" / "source-context.json"),
                ),
            }
        )
    if check_ai_debug_packet_matrix:
        artifacts.update(
            {
                "diagnostic_ai_debug_packet_matrix_json": artifact_value(
                    debug_packet_matrix_artifacts.get(
                        "diagnostic_ai_debug_packet_matrix_json"
                    ),
                    str(suite_dir / "diagnostic-ai-debug-packet-matrix.json"),
                ),
                "diagnostic_ai_debug_packet_matrix_report": artifact_value(
                    debug_packet_matrix_artifacts.get(
                        "diagnostic_ai_debug_packet_matrix_report"
                    ),
                    str(suite_dir / "diagnostic-ai-debug-packet-matrix.md"),
                ),
                "diagnostic_ai_debug_packet_matrix_dir": artifact_value(
                    debug_packet_matrix_artifacts.get(
                        "diagnostic_ai_debug_packet_matrix_dir"
                    ),
                    str(suite_dir / "ai-debug-packet-matrix"),
                ),
            }
        )
    if ai_localization_eval or check_e2e_report:
        artifacts.update(
            {
                "diagnostic_ai_localization_eval_json": artifact_value(
                    localization_eval_artifacts.get(
                        "diagnostic_ai_localization_eval_json"
                    ),
                    str(suite_dir / "diagnostic-ai-localization-eval.json"),
                ),
                "diagnostic_ai_localization_eval_report": artifact_value(
                    localization_eval_artifacts.get(
                        "diagnostic_ai_localization_eval_report"
                    ),
                    str(suite_dir / "diagnostic-ai-localization-eval.md"),
                ),
            }
        )
    if ai_session_plan or check_e2e_report:
        artifacts.update(
            {
                "diagnostic_ai_session_plan_json": artifact_value(
                    session_plan_artifacts.get("diagnostic_ai_session_plan_json"),
                    str(suite_dir / "diagnostic-ai-session-plan.json"),
                ),
                "diagnostic_ai_session_plan_report": artifact_value(
                    session_plan_artifacts.get("diagnostic_ai_session_plan_report"),
                    str(suite_dir / "diagnostic-ai-session-plan.md"),
                ),
            }
        )
    if ai_session_smoke or check_e2e_report:
        artifacts.update(
            {
                "diagnostic_ai_session_smoke_json": artifact_value(
                    session_smoke_artifacts.get("diagnostic_ai_session_smoke_json"),
                    str(suite_dir / "diagnostic-ai-session-smoke.json"),
                ),
                "diagnostic_ai_session_smoke_report": artifact_value(
                    session_smoke_artifacts.get("diagnostic_ai_session_smoke_report"),
                    str(suite_dir / "diagnostic-ai-session-smoke.md"),
                ),
            }
        )
    if ai_session_smoke_matrix or check_e2e_report:
        artifacts.update(
            {
                "diagnostic_ai_session_smoke_matrix_json": artifact_value(
                    session_smoke_matrix_artifacts.get(
                        "diagnostic_ai_session_smoke_matrix_json"
                    ),
                    str(suite_dir / "diagnostic-ai-session-smoke-matrix.json"),
                ),
                "diagnostic_ai_session_smoke_matrix_report": artifact_value(
                    session_smoke_matrix_artifacts.get(
                        "diagnostic_ai_session_smoke_matrix_report"
                    ),
                    str(suite_dir / "diagnostic-ai-session-smoke-matrix.md"),
                ),
                "diagnostic_ai_session_smoke_matrix_dir": artifact_value(
                    session_smoke_matrix_artifacts.get(
                        "diagnostic_ai_session_smoke_matrix_dir"
                    ),
                    str(suite_dir / "ai-session-smoke-matrix"),
                ),
            }
        )
    if check_e2e_report:
        artifacts.update(
            {
                "diagnostic_e2e_report_json": artifact_value(
                    e2e_artifacts.get("diagnostic_e2e_report_json"),
                    str(suite_dir / "diagnostic-e2e-report.json"),
                ),
                "diagnostic_e2e_report": artifact_value(
                    e2e_artifacts.get("diagnostic_e2e_report"),
                    str(suite_dir / "diagnostic-e2e-report.md"),
                ),
            }
        )
    return artifacts


def original_suite_dirs(
    suite_dir: Path,
    artifacts: list[dict[str, Any]],
) -> list[str]:
    values: list[Any] = [str(suite_dir)]
    for artifact in artifacts:
        values.append(artifact.get("suite_dir"))
    return unique_strings(values)


def add_check(
    checks: list[dict[str, Any]],
    errors: list[str],
    name: str,
    passed: bool,
    detail: Any,
) -> None:
    checks.append({"name": name, "passed": passed, "detail": detail})
    if not passed:
        errors.append(f"check failed: {name}")


def stop_conditions_passed(summary: dict[str, Any]) -> bool:
    conditions = [
        condition
        for condition in as_list(summary.get("stop_conditions"))
        if isinstance(condition, dict)
    ]
    return bool(conditions) and all(condition.get("passed") is True for condition in conditions)


def command_count(summary: dict[str, Any], command_group: str, command_name: str) -> int:
    return len(as_list(as_dict(summary.get(command_group)).get(command_name)))


def as_int(value: Any, default: int = 0) -> int:
    try:
        return int(value)
    except (TypeError, ValueError):
        return default


def top_identity(
    ai_index: dict[str, Any],
    ai_query: dict[str, Any],
) -> dict[str, Any]:
    index_summary = as_dict(ai_index.get("summary"))
    query_summary = as_dict(ai_query.get("summary"))
    return {
        "route_id": index_summary.get("top_route_id"),
        "scenario_id": index_summary.get("top_route_scenario"),
        "focus_domain": index_summary.get("top_route_focus_domain"),
        "probe_id": query_summary.get("top_route_probe"),
    }


def selection_identity(summary: dict[str, Any]) -> dict[str, Any]:
    selection = as_dict(summary.get("selection"))
    return {
        "route_id": selection.get("route_id"),
        "scenario_id": selection.get("scenario_id"),
        "focus_domain": selection.get("focus_domain"),
        "probe_id": selection.get("probe_id"),
    }


def summary_identity(summary: dict[str, Any], key: str) -> dict[str, Any]:
    values = as_dict(summary.get(key))
    return {
        "route_id": values.get("route_id") or values.get("top_route_id"),
        "scenario_id": values.get("scenario_id") or values.get("top_route_scenario"),
        "focus_domain": values.get("focus_domain") or values.get("top_route_focus_domain"),
        "probe_id": values.get("probe_id") or values.get("top_route_probe"),
    }


def matching_identity(left: dict[str, Any], right: dict[str, Any], keys: list[str]) -> bool:
    return all(left.get(key) == right.get(key) for key in keys)


def artifact_names_missing(
    suite_dir: Path,
    original_dirs: list[str],
    artifacts: dict[str, Any],
) -> list[str]:
    return [
        name
        for name, value in artifacts.items()
        if not artifact_exists(suite_dir, original_dirs, name, value)
    ]


def automation_readiness(
    suite_dir: Path,
    original_dirs: list[str],
    ai_index: dict[str, Any],
    ai_route_matrix: dict[str, Any],
    ai_debug_packet_matrix: dict[str, Any],
) -> dict[str, Any]:
    focus_rows = [
        row for row in as_list(ai_index.get("focus_domains")) if isinstance(row, dict)
    ]
    route_rows = [
        row for row in as_list(ai_route_matrix.get("routes")) if isinstance(row, dict)
    ]
    packet_rows = [
        row
        for row in as_list(ai_debug_packet_matrix.get("routes"))
        if isinstance(row, dict)
    ]
    route_by_id = {
        row.get("route_id"): row for row in route_rows if isinstance(row.get("route_id"), str)
    }
    packet_by_id = {
        row.get("route_id"): row for row in packet_rows if isinstance(row.get("route_id"), str)
    }

    rows: list[dict[str, Any]] = []
    for focus_row in focus_rows:
        route_id = focus_row.get("route_id")
        route_row = as_dict(route_by_id.get(route_id))
        packet_row = as_dict(packet_by_id.get(route_id))
        route_identity = as_dict(route_row.get("identity"))
        packet_identity = as_dict(packet_row.get("identity"))
        route_artifacts = as_dict(route_row.get("artifacts"))
        packet_artifacts = as_dict(packet_row.get("artifacts"))
        missing_route_artifacts = artifact_names_missing(
            suite_dir,
            original_dirs,
            route_artifacts,
        )
        missing_packet_artifacts = artifact_names_missing(
            suite_dir,
            original_dirs,
            packet_artifacts,
        )
        route_errors: list[str] = []
        if not route_row:
            route_errors.append("missing AI route-matrix row")
        if not packet_row:
            route_errors.append("missing AI debug-packet-matrix row")
        if route_row.get("status") != "passed":
            route_errors.append("AI route-matrix row is not passed")
        if packet_row.get("status") != "passed":
            route_errors.append("AI debug-packet-matrix row is not passed")
        if packet_row.get("packet_verification_status") != "passed":
            route_errors.append("packet self-verification did not pass")
        if not matching_identity(
            route_identity,
            packet_identity,
            ["route_id", "scenario_id", "focus_domain", "probe_id"],
        ):
            route_errors.append("route and packet identities do not match")
        if as_int(route_row.get("source_match_count")) < 1:
            route_errors.append("route has no source matches")
        if as_int(route_row.get("test_match_count")) < 1:
            route_errors.append("route has no test matches")
        if as_int(route_row.get("narrow_test_command_count")) < 1:
            route_errors.append("route has no narrow test commands")
        if as_int(packet_row.get("packet_file_count")) < 1:
            route_errors.append("packet has no files")
        if as_int(packet_row.get("packet_verifier_check_count")) < 1:
            route_errors.append("packet self-verifier ran no checks")
        if as_int(packet_row.get("packet_verifier_digest_mismatch_count")) != 0:
            route_errors.append("packet self-verifier found digest mismatches")
        if as_int(packet_row.get("packet_verifier_check_count")) != as_int(
            packet_row.get("packet_verifier_passed_check_count")
        ):
            route_errors.append("packet self-verifier checks did not all pass")
        if as_int(packet_row.get("source_window_count")) < 1:
            route_errors.append("packet has no source context windows")
        if as_int(packet_row.get("test_window_count")) < 1:
            route_errors.append("packet has no test context windows")
        if route_row.get("diagnosis_stop_conditions_passed") is not True:
            route_errors.append("diagnosis stop conditions did not pass")
        if route_row.get("fix_handoff_stop_conditions_passed") is not True:
            route_errors.append("fix-handoff stop conditions did not pass")
        if packet_row.get("stop_conditions_passed") is not True:
            route_errors.append("packet stop conditions did not pass")
        if as_list(route_row.get("missing_artifacts")):
            route_errors.append("route matrix row reports missing artifacts")
        if as_list(packet_row.get("missing_artifacts")):
            route_errors.append("packet matrix row reports missing artifacts")
        if missing_route_artifacts:
            route_errors.append("route artifacts are absent")
        if missing_packet_artifacts:
            route_errors.append("packet artifacts are absent")

        rows.append(
            {
                "route_id": route_id,
                "rank": focus_row.get("rank"),
                "scenario_id": route_identity.get("scenario_id")
                or focus_row.get("primary_scenario_id"),
                "focus_domain": focus_row.get("focus_domain"),
                "probe_id": route_identity.get("probe_id"),
                "ready": not route_errors,
                "route_status": route_row.get("status"),
                "diagnosis_status": route_row.get("diagnosis_status"),
                "fix_handoff_status": route_row.get("fix_handoff_status"),
                "replay_status": route_row.get("replay_status"),
                "tests_status": route_row.get("tests_status"),
                "packet_status": packet_row.get("status"),
                "packet_verification_status": packet_row.get(
                    "packet_verification_status"
                ),
                "source_match_count": route_row.get("source_match_count"),
                "test_match_count": route_row.get("test_match_count"),
                "narrow_test_command_count": route_row.get("narrow_test_command_count"),
                "packet_file_count": packet_row.get("packet_file_count"),
                "packet_verifier_check_count": packet_row.get(
                    "packet_verifier_check_count"
                ),
                "packet_verifier_passed_check_count": packet_row.get(
                    "packet_verifier_passed_check_count"
                ),
                "packet_verifier_digest_mismatch_count": packet_row.get(
                    "packet_verifier_digest_mismatch_count"
                ),
                "source_window_count": packet_row.get("source_window_count"),
                "test_window_count": packet_row.get("test_window_count"),
                "artifacts": {
                    "diagnosis_json": route_artifacts.get("diagnostic_ai_diagnosis_json"),
                    "fix_handoff_json": route_artifacts.get(
                        "diagnostic_ai_fix_handoff_json"
                    ),
                    "route_check_json": route_artifacts.get("route_check_json"),
                    "replay_triage_json": route_artifacts.get(
                        "replay_bundle_triage_json"
                    ),
                    "replay_telemetry_json": route_artifacts.get(
                        "replay_bundle_telemetry_json"
                    ),
                    "debug_packet_json": packet_artifacts.get(
                        "diagnostic_ai_debug_packet_json"
                    ),
                    "debug_packet_verification_json": packet_artifacts.get(
                        "diagnostic_ai_debug_packet_verification_json"
                    ),
                    "debug_packet_dir": packet_artifacts.get(
                        "diagnostic_ai_debug_packet_dir"
                    ),
                    "debug_packet_source_context": packet_artifacts.get(
                        "diagnostic_ai_debug_packet_source_context"
                    ),
                },
                "missing_route_artifacts": missing_route_artifacts,
                "missing_packet_artifacts": missing_packet_artifacts,
                "errors": route_errors,
            }
        )

    ready_rows = [row for row in rows if row.get("ready") is True]
    failed_rows = [row for row in rows if row.get("ready") is not True]
    capabilities = {
        "query_index": ai_index.get("status") == "passed",
        "run_diagnosis": bool(rows) and not failed_rows,
        "build_fix_handoff": bool(rows) and not failed_rows,
        "run_narrow_tests": bool(rows)
        and all(as_int(row.get("narrow_test_command_count")) >= 1 for row in rows),
        "open_debug_packet": bool(rows) and not failed_rows,
        "verify_debug_packet": bool(rows)
        and all(row.get("packet_verification_status") == "passed" for row in rows),
        "inspect_source_context": bool(rows)
        and all(as_int(row.get("source_window_count")) >= 1 for row in rows),
        "inspect_test_context": bool(rows)
        and all(as_int(row.get("test_window_count")) >= 1 for row in rows),
    }
    return {
        "status": "ready" if rows and not failed_rows else "not_ready",
        "route_count": len(rows),
        "ready_route_count": len(ready_rows),
        "not_ready_route_count": len(failed_rows),
        "failed_routes": [row.get("route_id") for row in failed_rows],
        "capabilities": capabilities,
        "routes": rows,
        "ai_handoff": [
            "Use automation_readiness.routes as the compact route-by-route starting point after the strict verifier passes.",
            "A ready route has replay evidence, diagnosis, fix handoff, narrow tests, source/test anchors, packet self-verification, and a debug packet with context windows.",
            "Use artifacts.debug_packet_json for packet-first debugging or artifacts.fix_handoff_json for code-edit planning.",
        ],
    }


def build_summary(
    args: argparse.Namespace,
    summary_json: Path,
    summary_report: Path,
) -> dict[str, Any]:
    suite_dir = args.suite_dir
    ai_index_path = suite_dir / "diagnostic-ai-observability-index.json"
    ai_coverage_gap_plan_path = suite_dir / "diagnostic-ai-coverage-gap-plan.json"
    ai_query_path = suite_dir / "diagnostic-ai-query-smoke.json"
    ai_diagnosis_path = suite_dir / "diagnostic-ai-diagnosis-smoke.json"
    ai_fix_handoff_path = suite_dir / "diagnostic-ai-fix-handoff-smoke.json"
    ai_route_matrix_path = suite_dir / "diagnostic-ai-route-matrix.json"
    ai_debug_packet_path = suite_dir / "diagnostic-ai-debug-packet.json"
    ai_debug_packet_verification_path = (
        suite_dir / "diagnostic-ai-debug-packet-verification.json"
    )
    ai_debug_packet_matrix_path = suite_dir / "diagnostic-ai-debug-packet-matrix.json"
    ai_localization_eval_path = suite_dir / "diagnostic-ai-localization-eval.json"
    ai_session_plan_path = suite_dir / "diagnostic-ai-session-plan.json"
    ai_session_smoke_path = suite_dir / "diagnostic-ai-session-smoke.json"
    ai_session_smoke_matrix_path = suite_dir / "diagnostic-ai-session-smoke-matrix.json"
    e2e_report_path = suite_dir / "diagnostic-e2e-report.json"

    ai_index = load_json(ai_index_path)
    ai_coverage_gap_plan = load_json(ai_coverage_gap_plan_path)
    ai_query = load_json(ai_query_path)
    ai_diagnosis = load_json(ai_diagnosis_path)
    ai_fix_handoff = load_json(ai_fix_handoff_path)
    ai_route_matrix = load_json(ai_route_matrix_path)
    ai_debug_packet = load_json(ai_debug_packet_path)
    ai_debug_packet_verification = load_json(ai_debug_packet_verification_path)
    ai_debug_packet_matrix = load_json(ai_debug_packet_matrix_path)
    ai_localization_eval = load_json(ai_localization_eval_path)
    ai_session_plan = load_json(ai_session_plan_path)
    ai_session_smoke = load_json(ai_session_smoke_path)
    ai_session_smoke_matrix = load_json(ai_session_smoke_matrix_path)
    e2e_report = load_json(e2e_report_path)
    check_e2e_report = args.require_e2e_report or bool(e2e_report)
    check_ai_route_matrix = args.require_ai_route_matrix or bool(ai_route_matrix)
    check_ai_debug_packet = args.require_ai_debug_packet or bool(ai_debug_packet)
    check_ai_debug_packet_matrix = args.require_ai_debug_packet_matrix or bool(
        ai_debug_packet_matrix
    )
    source_artifacts = [
        ai_index,
        ai_coverage_gap_plan,
        ai_query,
        ai_diagnosis,
        ai_fix_handoff,
        ai_route_matrix,
        ai_debug_packet,
        ai_debug_packet_verification,
        ai_debug_packet_matrix,
        ai_localization_eval,
        ai_session_plan,
        ai_session_smoke,
        ai_session_smoke_matrix,
        e2e_report,
    ]
    original_dirs = original_suite_dirs(suite_dir, source_artifacts)
    artifacts_to_check = input_artifacts(
        suite_dir,
        ai_index,
        ai_coverage_gap_plan,
        ai_query,
        ai_diagnosis,
        ai_fix_handoff,
        ai_route_matrix,
        ai_debug_packet,
        ai_debug_packet_verification,
        ai_debug_packet_matrix,
        ai_localization_eval,
        ai_session_plan,
        ai_session_smoke,
        ai_session_smoke_matrix,
        e2e_report,
        check_e2e_report,
        check_ai_route_matrix,
        check_ai_debug_packet,
        check_ai_debug_packet_matrix,
    )
    artifact_records = [
        artifact_record(suite_dir, original_dirs, name, value)
        for name, value in artifacts_to_check.items()
    ]
    artifact_presence = {record["name"]: record["present"] for record in artifact_records}
    readiness = automation_readiness(
        suite_dir,
        original_dirs,
        ai_index,
        ai_route_matrix,
        ai_debug_packet_matrix,
    )
    output_presence = {
        "diagnostic_ai_artifact_verification_json": True,
        "diagnostic_ai_artifact_verification_report": True,
    }

    checks: list[dict[str, Any]] = []
    errors: list[str] = []

    for path_name, payload in (
        ("diagnostic_ai_index", ai_index),
        ("diagnostic_ai_coverage_gap_plan", ai_coverage_gap_plan),
        ("diagnostic_ai_query_smoke", ai_query),
        ("diagnostic_ai_diagnosis_smoke", ai_diagnosis),
        ("diagnostic_ai_fix_handoff_smoke", ai_fix_handoff),
    ):
        add_check(checks, errors, f"{path_name}_loaded", bool(payload), path_name)
        add_check(
            checks,
            errors,
            f"{path_name}_passed",
            payload.get("status") == "passed",
            payload.get("status"),
        )

    if args.require_e2e_report:
        add_check(checks, errors, "diagnostic_e2e_report_loaded", bool(e2e_report), str(e2e_report_path))
    if check_e2e_report:
        add_check(
            checks,
            errors,
            "diagnostic_e2e_report_passed",
            e2e_report.get("status") == "passed",
            e2e_report.get("status"),
        )
    if args.require_ai_route_matrix:
        add_check(
            checks,
            errors,
            "diagnostic_ai_route_matrix_loaded",
            bool(ai_route_matrix),
            str(ai_route_matrix_path),
        )
    if check_ai_route_matrix:
        add_check(
            checks,
            errors,
            "diagnostic_ai_route_matrix_passed",
            ai_route_matrix.get("status") == "passed",
            ai_route_matrix.get("status"),
        )
    if args.require_ai_debug_packet:
        add_check(
            checks,
            errors,
            "diagnostic_ai_debug_packet_loaded",
            bool(ai_debug_packet),
            str(ai_debug_packet_path),
        )
        add_check(
            checks,
            errors,
            "diagnostic_ai_debug_packet_verification_loaded",
            bool(ai_debug_packet_verification),
            str(ai_debug_packet_verification_path),
        )
    if check_ai_debug_packet:
        add_check(
            checks,
            errors,
            "diagnostic_ai_debug_packet_passed",
            ai_debug_packet.get("status") == "passed",
            ai_debug_packet.get("status"),
        )
        add_check(
            checks,
            errors,
            "diagnostic_ai_debug_packet_verification_passed",
            ai_debug_packet_verification.get("status") == "passed",
            ai_debug_packet_verification.get("status"),
        )
    if args.require_ai_debug_packet_matrix:
        add_check(
            checks,
            errors,
            "diagnostic_ai_debug_packet_matrix_loaded",
            bool(ai_debug_packet_matrix),
            str(ai_debug_packet_matrix_path),
        )
    if check_ai_debug_packet_matrix:
        add_check(
            checks,
            errors,
            "diagnostic_ai_debug_packet_matrix_passed",
            ai_debug_packet_matrix.get("status") == "passed",
            ai_debug_packet_matrix.get("status"),
        )
    if check_e2e_report or ai_localization_eval:
        add_check(
            checks,
            errors,
            "diagnostic_ai_localization_eval_passed",
            ai_localization_eval.get("status") == "passed",
            ai_localization_eval.get("status"),
        )
    if check_e2e_report or ai_session_plan:
        add_check(
            checks,
            errors,
            "diagnostic_ai_session_plan_passed",
            ai_session_plan.get("status") == "passed",
            ai_session_plan.get("status"),
        )
    if check_e2e_report or ai_session_smoke:
        add_check(
            checks,
            errors,
            "diagnostic_ai_session_smoke_passed",
            ai_session_smoke.get("status") == "passed",
            ai_session_smoke.get("status"),
        )
    if check_e2e_report or ai_session_smoke_matrix:
        add_check(
            checks,
            errors,
            "diagnostic_ai_session_smoke_matrix_passed",
            ai_session_smoke_matrix.get("status") == "passed",
            ai_session_smoke_matrix.get("status"),
        )

    index_summary = as_dict(ai_index.get("summary"))
    coverage_gap_plan_summary = as_dict(ai_coverage_gap_plan.get("summary"))
    coverage_gap_plan_gaps = [
        row for row in as_list(ai_coverage_gap_plan.get("gaps")) if isinstance(row, dict)
    ]
    query_summary = as_dict(ai_query.get("summary"))
    coverage = as_dict(ai_index.get("coverage_limits"))
    add_check(
        checks,
        errors,
        "scenario_count",
        index_summary.get("scenario_count") == EXPECTED_SCENARIO_COUNT,
        index_summary.get("scenario_count"),
    )
    add_check(
        checks,
        errors,
        "actionable_scenario_count",
        index_summary.get("actionable_scenario_count") == EXPECTED_ACTIONABLE_SCENARIO_COUNT,
        index_summary.get("actionable_scenario_count"),
    )
    add_check(
        checks,
        errors,
        "focus_domain_count",
        index_summary.get("focus_domain_count") == EXPECTED_FOCUS_DOMAIN_COUNT,
        index_summary.get("focus_domain_count"),
    )
    add_check(
        checks,
        errors,
        "coverage_not_only_happy_paths",
        coverage.get("only_happy_paths") is False,
        coverage.get("only_happy_paths"),
    )
    add_check(
        checks,
        errors,
        "coverage_gaps_present",
        bool(as_list(coverage.get("coverage_gaps"))),
        coverage.get("known_gap_count"),
    )
    if ai_coverage_gap_plan or check_e2e_report:
        weak_gap_rows = [
            row.get("gap_id")
            for row in coverage_gap_plan_gaps
            if row.get("ready_for_test_design") is not True
            or not row.get("suggested_next_test")
            or not as_list(row.get("mapped_focus_domains"))
            or not as_list(row.get("source_files"))
            or not as_list(row.get("test_files"))
            or not as_list(row.get("telemetry_signals"))
            or not as_list(row.get("acceptance_commands"))
        ]
        add_check(
            checks,
            errors,
            "coverage_gap_plan_counts",
            coverage_gap_plan_summary.get("gap_count") == EXPECTED_COVERAGE_GAP_COUNT
            and coverage_gap_plan_summary.get("expected_gap_count")
            == EXPECTED_COVERAGE_GAP_COUNT
            and coverage_gap_plan_summary.get("known_gap_count")
            == coverage.get("known_gap_count")
            == EXPECTED_COVERAGE_GAP_COUNT
            and len(coverage_gap_plan_gaps) == EXPECTED_COVERAGE_GAP_COUNT,
            coverage_gap_plan_summary,
        )
        add_check(
            checks,
            errors,
            "coverage_gap_plan_not_happy_path_only",
            coverage_gap_plan_summary.get("only_happy_paths") is False,
            coverage_gap_plan_summary.get("only_happy_paths"),
        )
        add_check(
            checks,
            errors,
            "coverage_gap_plan_all_gaps_ready",
            coverage_gap_plan_summary.get("ready_gap_count") == EXPECTED_COVERAGE_GAP_COUNT
            and coverage_gap_plan_summary.get("mapped_gap_count")
            == EXPECTED_COVERAGE_GAP_COUNT
            and coverage_gap_plan_summary.get("source_anchor_gap_count")
            == EXPECTED_COVERAGE_GAP_COUNT
            and coverage_gap_plan_summary.get("test_anchor_gap_count")
            == EXPECTED_COVERAGE_GAP_COUNT
            and coverage_gap_plan_summary.get("telemetry_signal_gap_count")
            == EXPECTED_COVERAGE_GAP_COUNT
            and not weak_gap_rows,
            {"summary": coverage_gap_plan_summary, "weak_gap_rows": weak_gap_rows},
        )
    localization_summary = as_dict(ai_localization_eval.get("summary"))
    if ai_localization_eval or check_e2e_report:
        add_check(
            checks,
            errors,
            "localization_eval_scenario_count",
            localization_summary.get("scenario_count") == EXPECTED_SCENARIO_COUNT
            and localization_summary.get("passed_scenario_count")
            == EXPECTED_SCENARIO_COUNT,
            localization_summary,
        )
    session_plan_summary = as_dict(ai_session_plan.get("summary"))
    session_plan_routes = [
        row for row in as_list(ai_session_plan.get("route_sessions")) if isinstance(row, dict)
    ]
    session_smoke_summary = as_dict(ai_session_smoke.get("summary"))
    session_smoke_selection = as_dict(ai_session_smoke.get("selection"))
    session_smoke_matrix_summary = as_dict(ai_session_smoke_matrix.get("summary"))
    session_smoke_matrix_routes = [
        row for row in as_list(ai_session_smoke_matrix.get("routes")) if isinstance(row, dict)
    ]
    top_route_identity = top_identity(ai_index, ai_query)
    if ai_session_plan or check_e2e_report:
        add_check(
            checks,
            errors,
            "session_plan_route_count",
            session_plan_summary.get("route_count") == EXPECTED_ACTIONABLE_SCENARIO_COUNT
            and session_plan_summary.get("ready_route_count")
            == EXPECTED_ACTIONABLE_SCENARIO_COUNT
            and session_plan_summary.get("failed_route_count") == 0
            and len(session_plan_routes) == EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            session_plan_summary,
        )
        add_check(
            checks,
            errors,
            "session_plan_primary_route_matches_top_identity",
            session_plan_summary.get("primary_route_id") == top_route_identity.get("route_id")
            and session_plan_summary.get("primary_scenario_id")
            == top_route_identity.get("scenario_id")
            and session_plan_summary.get("primary_focus_domain")
            == top_route_identity.get("focus_domain"),
            session_plan_summary,
        )
        missing_command_routes = [
            row.get("route_id")
            for row in session_plan_routes
            if not as_list(as_dict(row.get("commands")).get("replay"))
            or not as_list(as_dict(row.get("commands")).get("narrow_tests"))
            or not as_list(as_dict(row.get("commands")).get("verification"))
        ]
        add_check(
            checks,
            errors,
            "session_plan_commands_present",
            not missing_command_routes
            and as_int(session_plan_summary.get("command_count"))
            >= EXPECTED_ACTIONABLE_SCENARIO_COUNT * 3,
            missing_command_routes,
        )
        missing_read_order_routes = [
            row.get("route_id")
            for row in session_plan_routes
            if not as_list(row.get("read_order"))
            or any(
                not isinstance(entry, dict)
                or not artifact_exists(
                    suite_dir,
                    original_dirs,
                    str(entry.get("name") or "session_plan_read_order"),
                    entry.get("path"),
                )
                for entry in as_list(row.get("read_order"))
            )
        ]
        add_check(
            checks,
            errors,
            "session_plan_read_order_artifacts_present",
            not missing_read_order_routes
            and as_int(session_plan_summary.get("read_order_artifact_count"))
            >= EXPECTED_ACTIONABLE_SCENARIO_COUNT * 8,
            missing_read_order_routes,
        )
        failed_stop_condition_routes = [
            row.get("route_id")
            for row in session_plan_routes
            if not as_list(row.get("stop_conditions"))
            or any(
                not isinstance(condition, dict) or condition.get("passed") is not True
                for condition in as_list(row.get("stop_conditions"))
            )
        ]
        add_check(
            checks,
            errors,
            "session_plan_stop_conditions_passed",
            not failed_stop_condition_routes
            and as_int(session_plan_summary.get("stop_condition_count"))
            >= EXPECTED_ACTIONABLE_SCENARIO_COUNT * 3,
            failed_stop_condition_routes,
        )
    if ai_session_smoke or check_e2e_report:
        add_check(
            checks,
            errors,
            "session_smoke_selects_primary_route",
            session_smoke_selection.get("route_id")
            == session_plan_summary.get("primary_route_id")
            == top_route_identity.get("route_id")
            and session_smoke_selection.get("scenario_id")
            == session_plan_summary.get("primary_scenario_id")
            == top_route_identity.get("scenario_id")
            and session_smoke_selection.get("focus_domain")
            == session_plan_summary.get("primary_focus_domain")
            == top_route_identity.get("focus_domain"),
            {
                "smoke": session_smoke_selection,
                "plan": session_plan_summary,
                "top": top_route_identity,
            },
        )
        add_check(
            checks,
            errors,
            "session_smoke_read_order_present",
            session_smoke_summary.get("read_order_present_count")
            == session_smoke_summary.get("read_order_artifact_count")
            and as_int(session_smoke_summary.get("read_order_artifact_count")) >= 9,
            session_smoke_summary,
        )
        add_check(
            checks,
            errors,
            "session_smoke_replay_passed",
            session_smoke_summary.get("replay_passed_count")
            == session_smoke_summary.get("replay_command_count")
            and as_int(session_smoke_summary.get("replay_command_count")) >= 1,
            session_smoke_summary,
        )
        add_check(
            checks,
            errors,
            "session_smoke_narrow_tests_passed",
            session_smoke_summary.get("narrow_test_passed_count")
            == session_smoke_summary.get("narrow_test_command_count")
            and as_int(session_smoke_summary.get("narrow_test_command_count")) >= 1,
            session_smoke_summary,
        )
        add_check(
            checks,
            errors,
            "session_smoke_verification_commands_recorded",
            as_int(session_smoke_summary.get("verification_command_count")) >= 1,
            session_smoke_summary,
        )
        add_check(
            checks,
            errors,
            "session_smoke_stop_conditions_passed",
            session_smoke_summary.get("stop_condition_passed_count")
            == session_smoke_summary.get("stop_condition_count")
            and as_int(session_smoke_summary.get("stop_condition_count")) >= 1,
            session_smoke_summary,
        )
    if ai_session_smoke_matrix or check_e2e_report:
        session_routes_by_id = {
            row.get("route_id"): row
            for row in session_plan_routes
            if isinstance(row.get("route_id"), str)
        }
        matrix_routes_by_id = {
            row.get("route_id"): row
            for row in session_smoke_matrix_routes
            if isinstance(row.get("route_id"), str)
        }
        missing_matrix_routes = [
            route_id
            for route_id in session_routes_by_id
            if route_id not in matrix_routes_by_id
        ]
        identity_mismatches = []
        weak_rows = []
        missing_row_artifacts = []
        for route_id, session_row in session_routes_by_id.items():
            matrix_row = as_dict(matrix_routes_by_id.get(route_id))
            row_summary = as_dict(matrix_row.get("summary"))
            if (
                matrix_row
                and (
                    matrix_row.get("scenario_id") != session_row.get("scenario_id")
                    or matrix_row.get("focus_domain") != session_row.get("focus_domain")
                    or matrix_row.get("probe_id") != session_row.get("probe_id")
                )
            ):
                identity_mismatches.append(route_id)
            if (
                not matrix_row
                or matrix_row.get("status") != "passed"
                or row_summary.get("read_order_present_count")
                != row_summary.get("read_order_artifact_count")
                or as_int(row_summary.get("read_order_artifact_count")) < 9
                or row_summary.get("replay_passed_count")
                != row_summary.get("replay_command_count")
                or as_int(row_summary.get("replay_command_count")) < 1
                or row_summary.get("narrow_test_passed_count")
                != row_summary.get("narrow_test_command_count")
                or as_int(row_summary.get("narrow_test_command_count")) < 1
                or as_int(row_summary.get("verification_command_count")) < 1
                or row_summary.get("stop_condition_passed_count")
                != row_summary.get("stop_condition_count")
                or as_int(row_summary.get("stop_condition_count")) < 1
            ):
                weak_rows.append(route_id)
            row_artifacts = as_dict(matrix_row.get("artifacts"))
            for artifact_name in (
                "diagnostic_ai_session_smoke_json",
                "diagnostic_ai_session_smoke_report",
            ):
                if not artifact_exists(
                    suite_dir,
                    original_dirs,
                    artifact_name,
                    row_artifacts.get(artifact_name),
                ):
                    missing_row_artifacts.append(f"{route_id}:{artifact_name}")
        add_check(
            checks,
            errors,
            "session_smoke_matrix_route_count",
            session_smoke_matrix_summary.get("route_count")
            == EXPECTED_ACTIONABLE_SCENARIO_COUNT
            and session_smoke_matrix_summary.get("passed_route_count")
            == EXPECTED_ACTIONABLE_SCENARIO_COUNT
            and session_smoke_matrix_summary.get("failed_route_count") == 0
            and len(session_smoke_matrix_routes) == EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            session_smoke_matrix_summary,
        )
        add_check(
            checks,
            errors,
            "session_smoke_matrix_routes_match_plan",
            not missing_matrix_routes and not identity_mismatches,
            {
                "missing_matrix_routes": missing_matrix_routes,
                "identity_mismatches": identity_mismatches,
            },
        )
        add_check(
            checks,
            errors,
            "session_smoke_matrix_all_rows_executable",
            not weak_rows,
            weak_rows,
        )
        add_check(
            checks,
            errors,
            "session_smoke_matrix_row_artifacts_present",
            not missing_row_artifacts,
            missing_row_artifacts,
        )
    if ai_session_plan or check_e2e_report:
        add_check(
            checks,
            errors,
            "localization_eval_negative_fixture_count",
            localization_summary.get("negative_fixture_count")
            == EXPECTED_ACTIONABLE_SCENARIO_COUNT
            and localization_summary.get("only_happy_paths") is False,
            localization_summary,
        )
        add_check(
            checks,
            errors,
            "localization_eval_focus_domains_match",
            localization_summary.get("focus_domain_match_count")
            == EXPECTED_ACTIONABLE_SCENARIO_COUNT
            and localization_summary.get("expected_focus_domain_count")
            == EXPECTED_FOCUS_DOMAIN_COUNT,
            localization_summary,
        )
        add_check(
            checks,
            errors,
            "localization_eval_routes_and_packets_ready",
            localization_summary.get("route_ready_count")
            == EXPECTED_ACTIONABLE_SCENARIO_COUNT
            and localization_summary.get("packet_self_verified_count")
            == EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            localization_summary,
        )
        add_check(
            checks,
            errors,
            "localization_eval_anchor_coverage",
            localization_summary.get("source_anchor_scenario_count")
            == EXPECTED_ACTIONABLE_SCENARIO_COUNT
            and localization_summary.get("test_anchor_scenario_count")
            == EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            localization_summary,
        )
        add_check(
            checks,
            errors,
            "localization_eval_scores_perfect",
            localization_summary.get("average_score") == 1.0
            and localization_summary.get("minimum_score") == 1.0,
            localization_summary,
        )
    add_check(
        checks,
        errors,
        "query_smoke_checks_passed",
        bool(as_list(ai_query.get("checks")))
        and all(
            as_dict(check).get("passed") is True for check in as_list(ai_query.get("checks"))
        ),
        len(as_list(ai_query.get("checks"))),
    )

    identity = top_route_identity
    diagnosis_identity = selection_identity(ai_diagnosis)
    fix_identity = selection_identity(ai_fix_handoff)
    focus_domain = find_by_key(
        ai_index.get("focus_domains"), "route_id", identity.get("route_id")
    )
    scenario = find_by_key(
        ai_index.get("scenario_cards"), "scenario_id", identity.get("scenario_id")
    )
    probe = find_by_key(ai_index.get("probe_index"), "probe_id", identity.get("probe_id"))
    add_check(
        checks,
        errors,
        "top_route_focus_domain_row",
        bool(focus_domain)
        and focus_domain.get("focus_domain") == identity.get("focus_domain")
        and focus_domain.get("primary_scenario_id") == identity.get("scenario_id"),
        identity,
    )
    add_check(
        checks,
        errors,
        "top_route_scenario_card",
        bool(scenario) and scenario.get("route_id") == identity.get("route_id"),
        identity.get("scenario_id"),
    )
    add_check(
        checks,
        errors,
        "top_route_probe_row",
        bool(probe) and identity.get("scenario_id") in as_str_list(probe.get("scenario_ids")),
        identity.get("probe_id"),
    )
    add_check(
        checks,
        errors,
        "query_matches_index_top_route",
        query_summary.get("top_route_id") == identity.get("route_id")
        and query_summary.get("top_route_scenario") == identity.get("scenario_id")
        and query_summary.get("top_route_focus_domain") == identity.get("focus_domain"),
        {"index": identity, "query": query_summary},
    )
    add_check(
        checks,
        errors,
        "diagnosis_matches_top_route",
        matching_identity(
            identity,
            diagnosis_identity,
            ["route_id", "scenario_id", "focus_domain", "probe_id"],
        ),
        {"top": identity, "diagnosis": diagnosis_identity},
    )
    add_check(
        checks,
        errors,
        "fix_handoff_matches_diagnosis",
        matching_identity(
            diagnosis_identity,
            fix_identity,
            ["route_id", "scenario_id", "focus_domain", "probe_id"],
        ),
        {"diagnosis": diagnosis_identity, "fix_handoff": fix_identity},
    )

    route_check = as_dict(ai_diagnosis.get("route_check"))
    add_check(
        checks,
        errors,
        "diagnosis_replay_passed",
        route_check.get("replay_status") == "passed"
        and route_check.get("tests_status") == "passed"
        and as_int(route_check.get("test_command_count")) >= 1,
        route_check,
    )
    add_check(
        checks,
        errors,
        "diagnosis_stop_conditions_passed",
        stop_conditions_passed(ai_diagnosis),
        as_list(ai_diagnosis.get("stop_conditions")),
    )
    add_check(
        checks,
        errors,
        "fix_handoff_source_matches",
        as_int(as_dict(ai_fix_handoff.get("source_scan")).get("source_match_count")) >= 1,
        as_dict(ai_fix_handoff.get("source_scan")).get("source_match_count"),
    )
    add_check(
        checks,
        errors,
        "fix_handoff_test_matches",
        as_int(as_dict(ai_fix_handoff.get("test_scan")).get("test_match_count")) >= 1,
        as_dict(ai_fix_handoff.get("test_scan")).get("test_match_count"),
    )
    add_check(
        checks,
        errors,
        "fix_handoff_narrow_commands",
        command_count(ai_fix_handoff, "fix_commands", "narrow_test_commands") >= 1,
        command_count(ai_fix_handoff, "fix_commands", "narrow_test_commands"),
    )
    add_check(
        checks,
        errors,
        "fix_handoff_stop_conditions_passed",
        stop_conditions_passed(ai_fix_handoff),
        as_list(ai_fix_handoff.get("stop_conditions")),
    )

    if check_ai_route_matrix:
        route_matrix_summary = as_dict(ai_route_matrix.get("summary"))
        route_rows = [row for row in as_list(ai_route_matrix.get("routes")) if isinstance(row, dict)]
        focus_rows = [
            row for row in as_list(ai_index.get("focus_domains")) if isinstance(row, dict)
        ]
        focus_by_route = {
            row.get("route_id"): row
            for row in focus_rows
            if isinstance(row.get("route_id"), str)
        }
        matrix_by_route = {
            row.get("route_id"): row
            for row in route_rows
            if isinstance(row.get("route_id"), str)
        }
        expected_route_count = index_summary.get("focus_domain_count")
        add_check(
            checks,
            errors,
            "ai_route_matrix_route_count",
            route_matrix_summary.get("route_count") == expected_route_count
            and len(route_rows) == expected_route_count,
            {
                "matrix": route_matrix_summary.get("route_count"),
                "rows": len(route_rows),
                "expected": expected_route_count,
            },
        )
        add_check(
            checks,
            errors,
            "ai_route_matrix_all_routes_passed",
            route_matrix_summary.get("passed_route_count") == expected_route_count
            and route_matrix_summary.get("failed_route_count") == 0,
            route_matrix_summary,
        )
        add_check(
            checks,
            errors,
            "ai_route_matrix_no_failures",
            route_matrix_summary.get("diagnosis_failure_count") == 0
            and route_matrix_summary.get("fix_handoff_failure_count") == 0
            and route_matrix_summary.get("replay_failure_count") == 0
            and route_matrix_summary.get("test_failure_count") == 0
            and route_matrix_summary.get("source_match_failure_count") == 0
            and route_matrix_summary.get("test_match_failure_count", 0) == 0
            and route_matrix_summary.get("narrow_command_failure_count") == 0
            and route_matrix_summary.get("stop_condition_failure_count") == 0
            and route_matrix_summary.get("missing_artifact_count") == 0,
            route_matrix_summary,
        )
        add_check(
            checks,
            errors,
            "ai_route_matrix_covers_focus_domains",
            set(focus_by_route) == set(matrix_by_route),
            {
                "missing": sorted(set(focus_by_route) - set(matrix_by_route)),
                "extra": sorted(set(matrix_by_route) - set(focus_by_route)),
            },
        )
        identity_mismatches = []
        route_artifact_missing = []
        for route_id, focus_row in focus_by_route.items():
            matrix_row = as_dict(matrix_by_route.get(route_id))
            route_identity = as_dict(matrix_row.get("identity"))
            if (
                matrix_row.get("focus_domain") != focus_row.get("focus_domain")
                or matrix_row.get("primary_scenario_id") != focus_row.get("primary_scenario_id")
                or route_identity.get("route_id") != route_id
                or route_identity.get("scenario_id") != focus_row.get("primary_scenario_id")
                or route_identity.get("focus_domain") != focus_row.get("focus_domain")
            ):
                identity_mismatches.append(route_id)
            if matrix_row.get("missing_artifacts"):
                route_artifact_missing.append(route_id)
        add_check(
            checks,
            errors,
            "ai_route_matrix_identities_match_index",
            not identity_mismatches,
            identity_mismatches,
        )
        add_check(
            checks,
            errors,
            "ai_route_matrix_route_artifacts_present",
            not route_artifact_missing,
            route_artifact_missing,
        )

    if check_ai_debug_packet:
        packet_identity = selection_identity(ai_debug_packet)
        packet_verification_identity = selection_identity(ai_debug_packet_verification)
        packet_manifest = as_dict(ai_debug_packet.get("packet_manifest"))
        context_summary = as_dict(ai_debug_packet.get("context_summary"))
        packet_verification_summary = as_dict(ai_debug_packet_verification.get("summary"))
        packet_route_matrix_row = find_by_key(
            ai_route_matrix.get("routes"), "route_id", packet_identity.get("route_id")
        )
        packet_route_matrix_identity = as_dict(packet_route_matrix_row.get("identity"))
        packet_matches_fix_handoff = matching_identity(
            packet_identity,
            fix_identity,
            ["route_id", "scenario_id", "focus_domain", "probe_id"],
        )
        packet_matches_route_matrix = matching_identity(
            packet_identity,
            packet_route_matrix_identity,
            ["route_id", "scenario_id", "focus_domain", "probe_id"],
        )
        packet_verification_matches_packet = matching_identity(
            packet_identity,
            packet_verification_identity,
            ["route_id", "scenario_id", "focus_domain", "probe_id"],
        )
        packet_records = [
            record
            for record in as_list(packet_manifest.get("files"))
            if isinstance(record, dict)
        ]
        missing_required = [
            record.get("name")
            for record in packet_records
            if record.get("required") is True and record.get("present") is not True
        ]
        invalid_digests = [
            record.get("name")
            for record in packet_records
            if not packet_record_valid(suite_dir, original_dirs, record)
        ]
        add_check(
            checks,
            errors,
            "ai_debug_packet_identity_known",
            packet_matches_fix_handoff or packet_matches_route_matrix,
            {
                "packet": packet_identity,
                "fix_handoff": fix_identity,
                "route_matrix": packet_route_matrix_identity,
            },
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_required_files_present",
            packet_manifest.get("file_count") == len(packet_records)
            and packet_manifest.get("missing_required_file_count") == 0
            and not missing_required,
            {
                "file_count": packet_manifest.get("file_count"),
                "records": len(packet_records),
                "missing_required": missing_required,
            },
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_file_digests",
            bool(packet_records) and not invalid_digests,
            invalid_digests,
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_context_present",
            as_int(context_summary.get("source_window_count")) > 0
            and as_int(context_summary.get("test_window_count")) > 0,
            context_summary,
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_stop_conditions_passed",
            stop_conditions_passed(ai_debug_packet),
            as_list(ai_debug_packet.get("stop_conditions")),
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_self_verification_identity",
            packet_verification_matches_packet,
            {
                "packet": packet_identity,
                "verification": packet_verification_identity,
            },
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_self_verification_checks",
            packet_verification_summary.get("check_count")
            == packet_verification_summary.get("passed_check_count")
            and as_int(packet_verification_summary.get("digest_mismatch_count")) == 0
            and as_int(packet_verification_summary.get("packet_file_count"))
            == packet_manifest.get("file_count"),
            packet_verification_summary,
        )

    if check_ai_debug_packet_matrix:
        packet_matrix_summary = as_dict(ai_debug_packet_matrix.get("summary"))
        packet_matrix_rows = [
            row
            for row in as_list(ai_debug_packet_matrix.get("routes"))
            if isinstance(row, dict)
        ]
        route_matrix_rows = [
            row for row in as_list(ai_route_matrix.get("routes")) if isinstance(row, dict)
        ]
        route_matrix_by_route = {
            row.get("route_id"): row
            for row in route_matrix_rows
            if isinstance(row.get("route_id"), str)
        }
        packet_matrix_by_route = {
            row.get("route_id"): row
            for row in packet_matrix_rows
            if isinstance(row.get("route_id"), str)
        }
        expected_route_count = index_summary.get("focus_domain_count")
        add_check(
            checks,
            errors,
            "ai_debug_packet_matrix_route_count",
            packet_matrix_summary.get("route_count") == expected_route_count
            and packet_matrix_summary.get("expected_route_count") == expected_route_count
            and len(packet_matrix_rows) == expected_route_count,
            {
                "matrix": packet_matrix_summary.get("route_count"),
                "expected": expected_route_count,
                "rows": len(packet_matrix_rows),
            },
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_matrix_all_routes_passed",
            packet_matrix_summary.get("passed_route_count") == expected_route_count
            and packet_matrix_summary.get("failed_route_count") == 0,
            packet_matrix_summary,
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_matrix_no_failures",
            packet_matrix_summary.get("packet_failure_count") == 0
            and packet_matrix_summary.get("packet_verification_failure_count") == 0
            and packet_matrix_summary.get("identity_failure_count") == 0
            and packet_matrix_summary.get("context_failure_count") == 0
            and packet_matrix_summary.get("stop_condition_failure_count") == 0
            and packet_matrix_summary.get("missing_artifact_count") == 0,
            packet_matrix_summary,
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_matrix_covers_route_matrix",
            set(route_matrix_by_route) == set(packet_matrix_by_route),
            {
                "missing": sorted(set(route_matrix_by_route) - set(packet_matrix_by_route)),
                "extra": sorted(set(packet_matrix_by_route) - set(route_matrix_by_route)),
            },
        )

        identity_mismatches = []
        context_missing = []
        packet_verification_failures = []
        route_artifact_missing = []
        route_artifact_absent = []
        for route_id, route_matrix_row in route_matrix_by_route.items():
            packet_matrix_row = as_dict(packet_matrix_by_route.get(route_id))
            route_identity = as_dict(route_matrix_row.get("identity"))
            packet_identity = as_dict(packet_matrix_row.get("identity"))
            if not matching_identity(
                route_identity,
                packet_identity,
                ["route_id", "scenario_id", "focus_domain", "probe_id"],
            ):
                identity_mismatches.append(route_id)
            if (
                as_int(packet_matrix_row.get("source_window_count")) < 1
                or as_int(packet_matrix_row.get("test_window_count")) < 1
                or as_int(packet_matrix_row.get("packet_file_count")) < 1
            ):
                context_missing.append(route_id)
            if (
                packet_matrix_row.get("packet_verification_status") != "passed"
                or as_int(packet_matrix_row.get("packet_verifier_digest_mismatch_count")) != 0
                or as_int(packet_matrix_row.get("packet_verifier_check_count")) < 1
                or as_int(packet_matrix_row.get("packet_verifier_check_count"))
                != as_int(packet_matrix_row.get("packet_verifier_passed_check_count"))
            ):
                packet_verification_failures.append(route_id)
            if packet_matrix_row.get("missing_artifacts"):
                route_artifact_missing.append(route_id)
            for artifact_name, artifact_path in as_dict(
                packet_matrix_row.get("artifacts")
            ).items():
                if not artifact_exists(
                    suite_dir,
                    original_dirs,
                    artifact_name,
                    artifact_path,
                ):
                    route_artifact_absent.append(f"{route_id}:{artifact_name}")
        add_check(
            checks,
            errors,
            "ai_debug_packet_matrix_identities_match_route_matrix",
            not identity_mismatches,
            identity_mismatches,
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_matrix_context_present",
            not context_missing,
            context_missing,
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_matrix_self_verification_passed",
            not packet_verification_failures,
            packet_verification_failures,
        )
        add_check(
            checks,
            errors,
            "ai_debug_packet_matrix_route_artifacts_present",
            not route_artifact_missing and not route_artifact_absent,
            {
                "missing_artifact_rows": route_artifact_missing,
                "absent_artifacts": route_artifact_absent,
            },
        )
        add_check(
            checks,
            errors,
            "automation_readiness_all_routes_ready",
            readiness.get("status") == "ready"
            and readiness.get("ready_route_count") == expected_route_count
            and readiness.get("not_ready_route_count") == 0,
            {
                "status": readiness.get("status"),
                "ready": readiness.get("ready_route_count"),
                "route_count": readiness.get("route_count"),
                "failed_routes": readiness.get("failed_routes"),
            },
        )

    if check_e2e_report:
        add_check(
            checks,
            errors,
            "e2e_ai_query_matches",
            matching_identity(
                identity,
                summary_identity(e2e_report, "ai_query"),
                ["route_id", "scenario_id", "focus_domain", "probe_id"],
            ),
            as_dict(e2e_report.get("ai_query")),
        )
        e2e_coverage_gap_plan = as_dict(e2e_report.get("ai_coverage_gap_plan"))
        if ai_coverage_gap_plan:
            add_check(
                checks,
                errors,
                "e2e_ai_coverage_gap_plan_matches",
                e2e_coverage_gap_plan.get("status") == ai_coverage_gap_plan.get("status")
                and e2e_coverage_gap_plan.get("gap_count")
                == coverage_gap_plan_summary.get("gap_count")
                and e2e_coverage_gap_plan.get("ready_gap_count")
                == coverage_gap_plan_summary.get("ready_gap_count")
                and e2e_coverage_gap_plan.get("telemetry_signal_gap_count")
                == coverage_gap_plan_summary.get("telemetry_signal_gap_count"),
                e2e_coverage_gap_plan,
            )
        add_check(
            checks,
            errors,
            "e2e_ai_diagnosis_matches",
            matching_identity(
                diagnosis_identity,
                summary_identity(e2e_report, "ai_diagnosis"),
                ["route_id", "scenario_id", "focus_domain", "probe_id"],
            ),
            as_dict(e2e_report.get("ai_diagnosis")),
        )
        add_check(
            checks,
            errors,
            "e2e_ai_fix_handoff_matches",
            matching_identity(
                fix_identity,
                summary_identity(e2e_report, "ai_fix_handoff"),
                ["route_id", "scenario_id", "focus_domain", "probe_id"],
            ),
            as_dict(e2e_report.get("ai_fix_handoff")),
        )
        if check_ai_route_matrix:
            e2e_route_matrix = as_dict(e2e_report.get("ai_route_matrix"))
            add_check(
                checks,
                errors,
                "e2e_ai_route_matrix_matches",
                e2e_route_matrix.get("status") == ai_route_matrix.get("status")
                and e2e_route_matrix.get("route_count")
                == as_dict(ai_route_matrix.get("summary")).get("route_count")
                and e2e_route_matrix.get("passed_route_count")
                == as_dict(ai_route_matrix.get("summary")).get("passed_route_count")
                and e2e_route_matrix.get("failed_route_count")
                == as_dict(ai_route_matrix.get("summary")).get("failed_route_count"),
                e2e_route_matrix,
            )
        if check_ai_debug_packet:
            e2e_debug_packet = as_dict(e2e_report.get("ai_debug_packet"))
            e2e_debug_packet_verification = as_dict(
                e2e_report.get("ai_debug_packet_verification")
            )
            packet_manifest = as_dict(ai_debug_packet.get("packet_manifest"))
            context_summary = as_dict(ai_debug_packet.get("context_summary"))
            packet_verification_summary = as_dict(
                ai_debug_packet_verification.get("summary")
            )
            add_check(
                checks,
                errors,
                "e2e_ai_debug_packet_matches",
                e2e_debug_packet.get("status") == ai_debug_packet.get("status")
                and e2e_debug_packet.get("file_count")
                == packet_manifest.get("file_count")
                and e2e_debug_packet.get("source_window_count")
                == context_summary.get("source_window_count")
                and e2e_debug_packet.get("test_window_count")
                == context_summary.get("test_window_count"),
                e2e_debug_packet,
            )
            add_check(
                checks,
                errors,
                "e2e_ai_debug_packet_verification_matches",
                e2e_debug_packet_verification.get("status")
                == ai_debug_packet_verification.get("status")
                and e2e_debug_packet_verification.get("check_count")
                == packet_verification_summary.get("check_count")
                and e2e_debug_packet_verification.get("passed_check_count")
                == packet_verification_summary.get("passed_check_count")
                and e2e_debug_packet_verification.get("digest_mismatch_count")
                == packet_verification_summary.get("digest_mismatch_count"),
                e2e_debug_packet_verification,
            )
        if check_ai_debug_packet_matrix:
            e2e_debug_packet_matrix = as_dict(e2e_report.get("ai_debug_packet_matrix"))
            packet_matrix_summary = as_dict(ai_debug_packet_matrix.get("summary"))
            add_check(
                checks,
                errors,
                "e2e_ai_debug_packet_matrix_matches",
                e2e_debug_packet_matrix.get("status")
                == ai_debug_packet_matrix.get("status")
                and e2e_debug_packet_matrix.get("route_count")
                == packet_matrix_summary.get("route_count")
                and e2e_debug_packet_matrix.get("passed_route_count")
                == packet_matrix_summary.get("passed_route_count")
                and e2e_debug_packet_matrix.get("failed_route_count")
                == packet_matrix_summary.get("failed_route_count")
                and e2e_debug_packet_matrix.get("packet_verification_failure_count")
                == packet_matrix_summary.get("packet_verification_failure_count")
                and e2e_debug_packet_matrix.get("context_failure_count")
                == packet_matrix_summary.get("context_failure_count"),
                e2e_debug_packet_matrix,
            )
        e2e_localization_eval = as_dict(e2e_report.get("ai_localization_eval"))
        if ai_localization_eval:
            add_check(
                checks,
                errors,
                "e2e_ai_localization_eval_matches",
                e2e_localization_eval.get("status") == ai_localization_eval.get("status")
                and e2e_localization_eval.get("scenario_count")
                == localization_summary.get("scenario_count")
                and e2e_localization_eval.get("passed_scenario_count")
                == localization_summary.get("passed_scenario_count")
                and e2e_localization_eval.get("negative_fixture_count")
                == localization_summary.get("negative_fixture_count")
                and e2e_localization_eval.get("average_score")
                == localization_summary.get("average_score"),
                e2e_localization_eval,
            )
        e2e_session_plan = as_dict(e2e_report.get("ai_session_plan"))
        if ai_session_plan:
            add_check(
                checks,
                errors,
                "e2e_ai_session_plan_matches",
                e2e_session_plan.get("status") == ai_session_plan.get("status")
                and e2e_session_plan.get("route_count")
                == session_plan_summary.get("route_count")
                and e2e_session_plan.get("ready_route_count")
                == session_plan_summary.get("ready_route_count")
                and e2e_session_plan.get("command_count")
                == session_plan_summary.get("command_count")
                and e2e_session_plan.get("primary_route_id")
                == session_plan_summary.get("primary_route_id"),
                e2e_session_plan,
            )
        e2e_session_smoke = as_dict(e2e_report.get("ai_session_smoke"))
        if ai_session_smoke:
            add_check(
                checks,
                errors,
                "e2e_ai_session_smoke_matches",
                e2e_session_smoke.get("status") == ai_session_smoke.get("status")
                and e2e_session_smoke.get("route_id")
                == session_smoke_selection.get("route_id")
                and e2e_session_smoke.get("read_order_present_count")
                == session_smoke_summary.get("read_order_present_count")
                and e2e_session_smoke.get("replay_passed_count")
                == session_smoke_summary.get("replay_passed_count")
                and e2e_session_smoke.get("narrow_test_passed_count")
                == session_smoke_summary.get("narrow_test_passed_count"),
                e2e_session_smoke,
            )
        e2e_session_smoke_matrix = as_dict(e2e_report.get("ai_session_smoke_matrix"))
        if ai_session_smoke_matrix:
            add_check(
                checks,
                errors,
                "e2e_ai_session_smoke_matrix_matches",
                e2e_session_smoke_matrix.get("status")
                == ai_session_smoke_matrix.get("status")
                and e2e_session_smoke_matrix.get("route_count")
                == session_smoke_matrix_summary.get("route_count")
                and e2e_session_smoke_matrix.get("passed_route_count")
                == session_smoke_matrix_summary.get("passed_route_count")
                and e2e_session_smoke_matrix.get("replay_passed_count")
                == session_smoke_matrix_summary.get("replay_passed_count")
                and e2e_session_smoke_matrix.get("narrow_test_passed_count")
                == session_smoke_matrix_summary.get("narrow_test_passed_count"),
                e2e_session_smoke_matrix,
            )

    missing_artifacts = [
        record["name"]
        for record in artifact_records
        if not record["present"]
    ]
    add_check(
        checks,
        errors,
        "required_artifacts_present",
        not missing_artifacts,
        missing_artifacts,
    )

    status = "passed" if not errors else "failed"
    artifacts = {
        **artifacts_to_check,
        **output_artifacts(summary_json, summary_report),
    }
    return {
        "diagnostic_ai_artifact_verification_schema_version": AI_ARTIFACT_VERIFICATION_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(suite_dir),
        "summary": {
            "check_count": len(checks),
            "passed_check_count": sum(1 for check in checks if check.get("passed") is True),
            "artifact_count": len(artifact_records),
            "missing_artifact_count": len(missing_artifacts),
            "e2e_report_checked": check_e2e_report,
            "ai_route_matrix_checked": check_ai_route_matrix,
            "ai_debug_packet_checked": check_ai_debug_packet,
            "ai_debug_packet_matrix_checked": check_ai_debug_packet_matrix,
            "scenario_count": index_summary.get("scenario_count"),
            "actionable_scenario_count": index_summary.get("actionable_scenario_count"),
            "focus_domain_count": index_summary.get("focus_domain_count"),
            "only_happy_paths": coverage.get("only_happy_paths"),
            "top_route_id": identity.get("route_id"),
            "top_route_scenario": identity.get("scenario_id"),
            "top_route_focus_domain": identity.get("focus_domain"),
            "top_route_probe": identity.get("probe_id"),
            "ai_coverage_gap_plan_status": ai_coverage_gap_plan.get("status"),
            "ai_coverage_gap_plan_gap_count": coverage_gap_plan_summary.get(
                "gap_count"
            ),
            "ai_coverage_gap_plan_ready_gap_count": coverage_gap_plan_summary.get(
                "ready_gap_count"
            ),
            "ai_coverage_gap_plan_mapped_gap_count": coverage_gap_plan_summary.get(
                "mapped_gap_count"
            ),
            "ai_coverage_gap_plan_source_anchor_gap_count": coverage_gap_plan_summary.get(
                "source_anchor_gap_count"
            ),
            "ai_coverage_gap_plan_test_anchor_gap_count": coverage_gap_plan_summary.get(
                "test_anchor_gap_count"
            ),
            "ai_coverage_gap_plan_telemetry_signal_gap_count": coverage_gap_plan_summary.get(
                "telemetry_signal_gap_count"
            ),
            "ai_coverage_gap_plan_validation_command_count": coverage_gap_plan_summary.get(
                "validation_command_count"
            ),
            "source_match_count": as_dict(ai_fix_handoff.get("source_scan")).get(
                "source_match_count"
            ),
            "test_match_count": as_dict(ai_fix_handoff.get("test_scan")).get(
                "test_match_count"
            ),
            "narrow_test_command_count": command_count(
                ai_fix_handoff, "fix_commands", "narrow_test_commands"
            ),
            "ai_route_count": as_dict(ai_route_matrix.get("summary")).get("route_count"),
            "ai_route_passed_count": as_dict(ai_route_matrix.get("summary")).get(
                "passed_route_count"
            ),
            "ai_debug_packet_file_count": as_dict(
                ai_debug_packet.get("packet_manifest")
            ).get("file_count"),
            "ai_debug_packet_verification_status": ai_debug_packet_verification.get(
                "status"
            ),
            "ai_debug_packet_verification_check_count": as_dict(
                ai_debug_packet_verification.get("summary")
            ).get("check_count"),
            "ai_debug_packet_verification_passed_check_count": as_dict(
                ai_debug_packet_verification.get("summary")
            ).get("passed_check_count"),
            "ai_debug_packet_verification_digest_mismatch_count": as_dict(
                ai_debug_packet_verification.get("summary")
            ).get("digest_mismatch_count"),
            "ai_debug_packet_source_window_count": as_dict(
                ai_debug_packet.get("context_summary")
            ).get("source_window_count"),
            "ai_debug_packet_test_window_count": as_dict(
                ai_debug_packet.get("context_summary")
            ).get("test_window_count"),
            "ai_debug_packet_matrix_route_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("route_count"),
            "ai_debug_packet_matrix_passed_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("passed_route_count"),
            "ai_debug_packet_matrix_context_failure_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("context_failure_count"),
            "ai_debug_packet_matrix_packet_verification_failure_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("packet_verification_failure_count"),
            "ai_debug_packet_matrix_packet_file_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("packet_file_count"),
            "ai_debug_packet_matrix_packet_verifier_check_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("packet_verifier_check_count"),
            "ai_debug_packet_matrix_packet_verifier_passed_check_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("packet_verifier_passed_check_count"),
            "ai_debug_packet_matrix_packet_verifier_digest_mismatch_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("packet_verifier_digest_mismatch_count"),
            "ai_localization_eval_status": ai_localization_eval.get("status"),
            "ai_localization_eval_scenario_count": localization_summary.get(
                "scenario_count"
            ),
            "ai_localization_eval_passed_scenario_count": localization_summary.get(
                "passed_scenario_count"
            ),
            "ai_localization_eval_negative_fixture_count": localization_summary.get(
                "negative_fixture_count"
            ),
            "ai_localization_eval_only_happy_paths": localization_summary.get(
                "only_happy_paths"
            ),
            "ai_localization_eval_focus_domain_match_count": localization_summary.get(
                "focus_domain_match_count"
            ),
            "ai_localization_eval_route_ready_count": localization_summary.get(
                "route_ready_count"
            ),
            "ai_localization_eval_packet_self_verified_count": localization_summary.get(
                "packet_self_verified_count"
            ),
            "ai_localization_eval_average_score": localization_summary.get(
                "average_score"
            ),
            "ai_localization_eval_minimum_score": localization_summary.get(
                "minimum_score"
            ),
            "ai_session_plan_status": ai_session_plan.get("status"),
            "ai_session_plan_route_count": session_plan_summary.get("route_count"),
            "ai_session_plan_ready_route_count": session_plan_summary.get(
                "ready_route_count"
            ),
            "ai_session_plan_failed_route_count": session_plan_summary.get(
                "failed_route_count"
            ),
            "ai_session_plan_primary_route_id": session_plan_summary.get(
                "primary_route_id"
            ),
            "ai_session_plan_primary_scenario_id": session_plan_summary.get(
                "primary_scenario_id"
            ),
            "ai_session_plan_command_count": session_plan_summary.get("command_count"),
            "ai_session_plan_read_order_artifact_count": session_plan_summary.get(
                "read_order_artifact_count"
            ),
            "ai_session_plan_stop_condition_count": session_plan_summary.get(
                "stop_condition_count"
            ),
            "ai_session_smoke_status": ai_session_smoke.get("status"),
            "ai_session_smoke_route_id": session_smoke_selection.get("route_id"),
            "ai_session_smoke_read_order_present_count": session_smoke_summary.get(
                "read_order_present_count"
            ),
            "ai_session_smoke_read_order_artifact_count": session_smoke_summary.get(
                "read_order_artifact_count"
            ),
            "ai_session_smoke_replay_passed_count": session_smoke_summary.get(
                "replay_passed_count"
            ),
            "ai_session_smoke_replay_command_count": session_smoke_summary.get(
                "replay_command_count"
            ),
            "ai_session_smoke_narrow_test_passed_count": session_smoke_summary.get(
                "narrow_test_passed_count"
            ),
            "ai_session_smoke_narrow_test_command_count": session_smoke_summary.get(
                "narrow_test_command_count"
            ),
            "ai_session_smoke_verification_command_count": session_smoke_summary.get(
                "verification_command_count"
            ),
            "ai_session_smoke_matrix_status": ai_session_smoke_matrix.get("status"),
            "ai_session_smoke_matrix_route_count": session_smoke_matrix_summary.get(
                "route_count"
            ),
            "ai_session_smoke_matrix_passed_route_count": session_smoke_matrix_summary.get(
                "passed_route_count"
            ),
            "ai_session_smoke_matrix_failed_route_count": session_smoke_matrix_summary.get(
                "failed_route_count"
            ),
            "ai_session_smoke_matrix_read_order_present_count": session_smoke_matrix_summary.get(
                "read_order_present_count"
            ),
            "ai_session_smoke_matrix_read_order_artifact_count": session_smoke_matrix_summary.get(
                "read_order_artifact_count"
            ),
            "ai_session_smoke_matrix_replay_passed_count": session_smoke_matrix_summary.get(
                "replay_passed_count"
            ),
            "ai_session_smoke_matrix_replay_command_count": session_smoke_matrix_summary.get(
                "replay_command_count"
            ),
            "ai_session_smoke_matrix_narrow_test_passed_count": session_smoke_matrix_summary.get(
                "narrow_test_passed_count"
            ),
            "ai_session_smoke_matrix_narrow_test_command_count": session_smoke_matrix_summary.get(
                "narrow_test_command_count"
            ),
            "ai_session_smoke_matrix_verification_command_count": session_smoke_matrix_summary.get(
                "verification_command_count"
            ),
            "automation_readiness_status": readiness.get("status"),
            "automation_ready_route_count": readiness.get("ready_route_count"),
            "automation_route_count": readiness.get("route_count"),
            "automation_not_ready_route_count": readiness.get("not_ready_route_count"),
        },
        "artifacts": artifacts,
        "artifact_presence": {
            **artifact_presence,
            **output_presence,
        },
        "artifact_records": artifact_records,
        "automation_readiness": readiness,
        "checks": checks,
        "errors": errors,
        "ai_handoff": [
            "Use this verifier before trusting a downloaded diagnostic scenario-suite artifact for automated debugging.",
            "A passed verifier proves the AI index, query smoke, diagnosis smoke, and fix handoff agree on route, scenario, focus domain, and probe.",
            "When diagnostic-ai-coverage-gap-plan.json is present, this verifier also proves known coverage gaps are mapped to current source/test anchors, telemetry signals, and validation commands.",
            "When diagnostic-ai-route-matrix.json is present, this verifier also proves every AI route can regenerate diagnosis and fix-handoff artifacts.",
            "When diagnostic-ai-debug-packet.json is present, this verifier also proves the selected route has a relocatable packet with digest-checked evidence and source/test context.",
            "When diagnostic-ai-debug-packet-matrix.json is present, this verifier also proves every AI route has a relocatable packet with source/test context.",
            "When diagnostic-ai-localization-eval.json is present, this verifier also proves the scenario corpus localizes expected health, focus domains, routes, anchors, and packets.",
            "When diagnostic-ai-session-plan.json is present, this verifier also proves every AI route has a deterministic debug startup plan.",
            "When diagnostic-ai-session-smoke.json is present, this verifier also proves the selected startup plan can be executed by an automated consumer.",
            "When diagnostic-ai-session-smoke-matrix.json is present, this verifier also proves every startup plan route can be executed by an automated consumer.",
            "Use automation_readiness.routes as the compact per-route map for automated debugger startup after this verifier passes.",
            "Use --require-e2e-report when validating a completed CI artifact after diagnostic-e2e-report.json has been written.",
            "If this verifier fails, repair the artifact graph before asking an AI debugger to edit emulator code.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    lines = [
        "# Diagnostic AI Artifact Verification",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Checks | {totals.get('passed_check_count')}/{totals.get('check_count')} |",
        f"| Required artifacts | {totals.get('artifact_count')} |",
        f"| Missing artifacts | {totals.get('missing_artifact_count')} |",
        f"| E2E report checked | {totals.get('e2e_report_checked')} |",
        f"| AI route matrix checked | {totals.get('ai_route_matrix_checked')} |",
        f"| AI routes | {totals.get('ai_route_passed_count')}/{totals.get('ai_route_count')} |",
        f"| AI debug packet checked | {totals.get('ai_debug_packet_checked')} |",
        f"| AI debug packet files | {totals.get('ai_debug_packet_file_count')} |",
        f"| AI debug packet verification | {totals.get('ai_debug_packet_verification_status')} |",
        f"| AI debug packet verification checks | {totals.get('ai_debug_packet_verification_passed_check_count')}/{totals.get('ai_debug_packet_verification_check_count')} |",
        f"| AI debug packet verification digest mismatches | {totals.get('ai_debug_packet_verification_digest_mismatch_count')} |",
        f"| AI debug packet source windows | {totals.get('ai_debug_packet_source_window_count')} |",
        f"| AI debug packet test windows | {totals.get('ai_debug_packet_test_window_count')} |",
        f"| AI debug packet matrix checked | {totals.get('ai_debug_packet_matrix_checked')} |",
        f"| AI debug packet matrix routes | {totals.get('ai_debug_packet_matrix_passed_count')}/{totals.get('ai_debug_packet_matrix_route_count')} |",
        f"| AI debug packet matrix context failures | {totals.get('ai_debug_packet_matrix_context_failure_count')} |",
        f"| AI debug packet matrix self-verification failures | {totals.get('ai_debug_packet_matrix_packet_verification_failure_count')} |",
        f"| AI debug packet matrix files | {totals.get('ai_debug_packet_matrix_packet_file_count')} |",
        f"| AI debug packet matrix verifier checks | {totals.get('ai_debug_packet_matrix_packet_verifier_passed_check_count')}/{totals.get('ai_debug_packet_matrix_packet_verifier_check_count')} |",
        f"| AI debug packet matrix verifier digest mismatches | {totals.get('ai_debug_packet_matrix_packet_verifier_digest_mismatch_count')} |",
        f"| AI localization evaluation | {totals.get('ai_localization_eval_status')} |",
        f"| AI localization scenarios | {totals.get('ai_localization_eval_passed_scenario_count')}/{totals.get('ai_localization_eval_scenario_count')} |",
        f"| AI localization negative fixtures | {totals.get('ai_localization_eval_negative_fixture_count')} |",
        f"| AI localization only happy paths | {totals.get('ai_localization_eval_only_happy_paths')} |",
        f"| AI localization focus matches | {totals.get('ai_localization_eval_focus_domain_match_count')}/{totals.get('ai_localization_eval_negative_fixture_count')} |",
        f"| AI localization route-ready fixtures | {totals.get('ai_localization_eval_route_ready_count')}/{totals.get('ai_localization_eval_negative_fixture_count')} |",
        f"| AI localization packet self-verified fixtures | {totals.get('ai_localization_eval_packet_self_verified_count')}/{totals.get('ai_localization_eval_negative_fixture_count')} |",
        f"| AI localization average score | {totals.get('ai_localization_eval_average_score')} |",
        f"| AI localization minimum score | {totals.get('ai_localization_eval_minimum_score')} |",
        f"| AI session plan | {totals.get('ai_session_plan_status')} |",
        f"| AI session plan routes | {totals.get('ai_session_plan_ready_route_count')}/{totals.get('ai_session_plan_route_count')} |",
        f"| AI session plan failed routes | {totals.get('ai_session_plan_failed_route_count')} |",
        f"| AI session plan primary route | {totals.get('ai_session_plan_primary_route_id')} |",
        f"| AI session plan primary scenario | {totals.get('ai_session_plan_primary_scenario_id')} |",
        f"| AI session plan commands | {totals.get('ai_session_plan_command_count')} |",
        f"| AI session plan read-order artifacts | {totals.get('ai_session_plan_read_order_artifact_count')} |",
        f"| AI session plan stop conditions | {totals.get('ai_session_plan_stop_condition_count')} |",
        f"| AI session smoke | {totals.get('ai_session_smoke_status')} |",
        f"| AI session smoke route | {totals.get('ai_session_smoke_route_id')} |",
        f"| AI session smoke read-order artifacts | {totals.get('ai_session_smoke_read_order_present_count')}/{totals.get('ai_session_smoke_read_order_artifact_count')} |",
        f"| AI session smoke replay commands | {totals.get('ai_session_smoke_replay_passed_count')}/{totals.get('ai_session_smoke_replay_command_count')} |",
        f"| AI session smoke narrow tests | {totals.get('ai_session_smoke_narrow_test_passed_count')}/{totals.get('ai_session_smoke_narrow_test_command_count')} |",
        f"| AI session smoke verification commands | {totals.get('ai_session_smoke_verification_command_count')} |",
        f"| AI session smoke matrix | {totals.get('ai_session_smoke_matrix_status')} |",
        f"| AI session smoke matrix routes | {totals.get('ai_session_smoke_matrix_passed_route_count')}/{totals.get('ai_session_smoke_matrix_route_count')} |",
        f"| AI session smoke matrix failed routes | {totals.get('ai_session_smoke_matrix_failed_route_count')} |",
        f"| AI session smoke matrix read-order artifacts | {totals.get('ai_session_smoke_matrix_read_order_present_count')}/{totals.get('ai_session_smoke_matrix_read_order_artifact_count')} |",
        f"| AI session smoke matrix replay commands | {totals.get('ai_session_smoke_matrix_replay_passed_count')}/{totals.get('ai_session_smoke_matrix_replay_command_count')} |",
        f"| AI session smoke matrix narrow tests | {totals.get('ai_session_smoke_matrix_narrow_test_passed_count')}/{totals.get('ai_session_smoke_matrix_narrow_test_command_count')} |",
        f"| AI session smoke matrix verification commands | {totals.get('ai_session_smoke_matrix_verification_command_count')} |",
        f"| Automation readiness | {totals.get('automation_readiness_status')} |",
        f"| Automation-ready routes | {totals.get('automation_ready_route_count')}/{totals.get('automation_route_count')} |",
        f"| Automation not-ready routes | {totals.get('automation_not_ready_route_count')} |",
        f"| Top route | {markdown_cell(totals.get('top_route_id'))} |",
        f"| Top scenario | {markdown_cell(totals.get('top_route_scenario'))} |",
        f"| Top focus domain | {markdown_cell(totals.get('top_route_focus_domain'))} |",
        f"| Top probe | {markdown_cell(totals.get('top_route_probe'))} |",
        f"| AI coverage gap plan | {totals.get('ai_coverage_gap_plan_status')} |",
        f"| AI coverage ready gaps | {totals.get('ai_coverage_gap_plan_ready_gap_count')}/{totals.get('ai_coverage_gap_plan_gap_count')} |",
        f"| AI coverage mapped gaps | {totals.get('ai_coverage_gap_plan_mapped_gap_count')} |",
        f"| AI coverage source anchors | {totals.get('ai_coverage_gap_plan_source_anchor_gap_count')} |",
        f"| AI coverage test anchors | {totals.get('ai_coverage_gap_plan_test_anchor_gap_count')} |",
        f"| AI coverage telemetry mappings | {totals.get('ai_coverage_gap_plan_telemetry_signal_gap_count')} |",
        f"| AI coverage validation commands | {totals.get('ai_coverage_gap_plan_validation_command_count')} |",
        f"| Fix handoff test matches | {totals.get('test_match_count')} |",
        "",
        "## Automation Readiness",
        "",
        "| Rank | Route | Scenario | Focus domain | Ready | Source matches | Test matches | Source windows | Test windows | Packet files |",
        "| ---: | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    readiness = as_dict(summary.get("automation_readiness"))
    for row in as_list(readiness.get("routes")):
        if not isinstance(row, dict):
            continue
        lines.append(
            f"| {row.get('rank')} | {markdown_cell(row.get('route_id'))} | "
            f"{markdown_cell(row.get('scenario_id'))} | "
            f"{markdown_cell(row.get('focus_domain'))} | {row.get('ready')} | "
            f"{row.get('source_match_count')} | {row.get('test_match_count')} | "
            f"{row.get('source_window_count')} | {row.get('test_window_count')} | "
            f"{row.get('packet_file_count')} |"
        )
    lines.extend(
        [
            "",
        "## Checks",
        "",
        "| Check | Passed | Detail |",
        "| --- | --- | --- |",
        ]
    )
    for check in as_list(summary.get("checks")):
        if not isinstance(check, dict):
            continue
        lines.append(
            f"| {markdown_cell(check.get('name'))} | {check.get('passed')} | {markdown_cell(check.get('detail'))} |"
        )
    lines.extend(
        [
            "",
            "## Artifacts",
            "",
            "| Name | Present | Resolved path |",
            "| --- | --- | --- |",
        ]
    )
    for record in as_list(summary.get("artifact_records")):
        if not isinstance(record, dict):
            continue
        lines.append(
            f"| {markdown_cell(record.get('name'))} | {record.get('present')} | {markdown_cell(record.get('resolved_path'))} |"
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
        "--summary-json",
        type=Path,
        help="Path to write the verifier JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the verifier Markdown report. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--require-e2e-report",
        action="store_true",
        help="Require diagnostic-e2e-report.json and cross-check its AI summaries.",
    )
    parser.add_argument(
        "--require-ai-route-matrix",
        action="store_true",
        help="Require diagnostic-ai-route-matrix.json and cross-check every AI route.",
    )
    parser.add_argument(
        "--require-ai-debug-packet",
        action="store_true",
        help="Require diagnostic-ai-debug-packet.json and verify the selected AI debug packet.",
    )
    parser.add_argument(
        "--require-ai-debug-packet-matrix",
        action="store_true",
        help=(
            "Require diagnostic-ai-debug-packet-matrix.json and verify every "
            "AI route has a debug packet."
        ),
    )
    parser.add_argument("--json", action="store_true", help="Print the verifier JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-artifact-verification.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-artifact-verification.md"
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
            "Diagnostic AI artifact verification "
            f"{summary['status']}: suite={args.suite_dir} "
            f"checks={totals.get('passed_check_count')}/{totals.get('check_count')} "
            f"missing_artifacts={totals.get('missing_artifact_count')} "
            f"route={totals.get('top_route_id')} "
            f"scenario={totals.get('top_route_scenario')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
