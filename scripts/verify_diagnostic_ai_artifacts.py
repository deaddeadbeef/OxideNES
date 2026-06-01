#!/usr/bin/env python3
"""Verify the AI-facing diagnostic artifact graph from an OxideNES suite."""

from __future__ import annotations

import argparse
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_ARTIFACT_VERIFICATION_SCHEMA_VERSION = 1
EXPECTED_SCENARIO_COUNT = 18
EXPECTED_ACTIONABLE_SCENARIO_COUNT = 16
EXPECTED_FOCUS_DOMAIN_COUNT = 16


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
    ai_query: dict[str, Any],
    ai_diagnosis: dict[str, Any],
    ai_fix_handoff: dict[str, Any],
    e2e_report: dict[str, Any],
    check_e2e_report: bool,
) -> dict[str, str]:
    index_artifacts = as_dict(ai_index.get("artifacts"))
    query_artifacts = as_dict(ai_query.get("artifacts"))
    diagnosis_artifacts = as_dict(ai_diagnosis.get("artifacts"))
    fix_artifacts = as_dict(ai_fix_handoff.get("artifacts"))
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


def build_summary(
    args: argparse.Namespace,
    summary_json: Path,
    summary_report: Path,
) -> dict[str, Any]:
    suite_dir = args.suite_dir
    ai_index_path = suite_dir / "diagnostic-ai-observability-index.json"
    ai_query_path = suite_dir / "diagnostic-ai-query-smoke.json"
    ai_diagnosis_path = suite_dir / "diagnostic-ai-diagnosis-smoke.json"
    ai_fix_handoff_path = suite_dir / "diagnostic-ai-fix-handoff-smoke.json"
    e2e_report_path = suite_dir / "diagnostic-e2e-report.json"

    ai_index = load_json(ai_index_path)
    ai_query = load_json(ai_query_path)
    ai_diagnosis = load_json(ai_diagnosis_path)
    ai_fix_handoff = load_json(ai_fix_handoff_path)
    e2e_report = load_json(e2e_report_path)
    check_e2e_report = args.require_e2e_report or bool(e2e_report)
    source_artifacts = [ai_index, ai_query, ai_diagnosis, ai_fix_handoff, e2e_report]
    original_dirs = original_suite_dirs(suite_dir, source_artifacts)
    artifacts_to_check = input_artifacts(
        suite_dir,
        ai_index,
        ai_query,
        ai_diagnosis,
        ai_fix_handoff,
        e2e_report,
        check_e2e_report,
    )
    artifact_records = [
        artifact_record(suite_dir, original_dirs, name, value)
        for name, value in artifacts_to_check.items()
    ]
    artifact_presence = {record["name"]: record["present"] for record in artifact_records}
    output_presence = {
        "diagnostic_ai_artifact_verification_json": True,
        "diagnostic_ai_artifact_verification_report": True,
    }

    checks: list[dict[str, Any]] = []
    errors: list[str] = []

    for path_name, payload in (
        ("diagnostic_ai_index", ai_index),
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

    index_summary = as_dict(ai_index.get("summary"))
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

    identity = top_identity(ai_index, ai_query)
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
            "scenario_count": index_summary.get("scenario_count"),
            "actionable_scenario_count": index_summary.get("actionable_scenario_count"),
            "focus_domain_count": index_summary.get("focus_domain_count"),
            "only_happy_paths": coverage.get("only_happy_paths"),
            "top_route_id": identity.get("route_id"),
            "top_route_scenario": identity.get("scenario_id"),
            "top_route_focus_domain": identity.get("focus_domain"),
            "top_route_probe": identity.get("probe_id"),
            "source_match_count": as_dict(ai_fix_handoff.get("source_scan")).get(
                "source_match_count"
            ),
            "narrow_test_command_count": command_count(
                ai_fix_handoff, "fix_commands", "narrow_test_commands"
            ),
        },
        "artifacts": artifacts,
        "artifact_presence": {
            **artifact_presence,
            **output_presence,
        },
        "artifact_records": artifact_records,
        "checks": checks,
        "errors": errors,
        "ai_handoff": [
            "Use this verifier before trusting a downloaded diagnostic scenario-suite artifact for automated debugging.",
            "A passed verifier proves the AI index, query smoke, diagnosis smoke, and fix handoff agree on route, scenario, focus domain, and probe.",
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
        f"| Top route | {markdown_cell(totals.get('top_route_id'))} |",
        f"| Top scenario | {markdown_cell(totals.get('top_route_scenario'))} |",
        f"| Top focus domain | {markdown_cell(totals.get('top_route_focus_domain'))} |",
        f"| Top probe | {markdown_cell(totals.get('top_route_probe'))} |",
        "",
        "## Checks",
        "",
        "| Check | Passed | Detail |",
        "| --- | --- | --- |",
    ]
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
