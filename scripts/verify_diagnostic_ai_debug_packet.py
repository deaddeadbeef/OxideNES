#!/usr/bin/env python3
"""Verify a relocatable diagnostic AI debug packet without the original suite."""

from __future__ import annotations

import argparse
import hashlib
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_DEBUG_PACKET_VERIFICATION_SCHEMA_VERSION = 1


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {}
    return value if isinstance(value, dict) else {}


def generated_at_utc() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def markdown_cell(value: Any) -> str:
    if value is None:
        return "-"
    return str(value).replace("|", r"\|").replace("\r", " ").replace("\n", " ")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def packet_path(packet_dir: Path, relative_path: Any) -> Path | None:
    if not isinstance(relative_path, str) or not relative_path:
        return None
    path = packet_dir / relative_path.replace("\\", "/")
    try:
        path.resolve().relative_to(packet_dir.resolve())
    except ValueError:
        return None
    return path


def selection_identity(summary: dict[str, Any]) -> dict[str, Any]:
    selection = as_dict(summary.get("selection"))
    return {
        "route_id": selection.get("route_id"),
        "scenario_id": selection.get("scenario_id"),
        "focus_domain": selection.get("focus_domain"),
        "probe_id": selection.get("probe_id"),
    }


def matching_identity(left: dict[str, Any], right: dict[str, Any]) -> bool:
    return all(
        left.get(key) == right.get(key)
        for key in ("route_id", "scenario_id", "focus_domain", "probe_id")
    )


def context_window_count(rows: Any) -> int:
    return sum(int(as_dict(row).get("window_count") or 0) for row in as_list(rows))


def add_check(
    checks: list[dict[str, Any]],
    errors: list[str],
    name: str,
    passed: bool,
    detail: Any,
) -> None:
    checks.append({"name": name, "passed": bool(passed), "detail": detail})
    if not passed:
        errors.append(name)


def packet_file_records(packet_dir: Path, manifest: dict[str, Any]) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for record in as_list(as_dict(manifest.get("packet_manifest")).get("files")):
        if not isinstance(record, dict):
            continue
        expected_relative = record.get("packet_relative_path")
        path = packet_path(packet_dir, expected_relative)
        present = bool(path and path.is_file())
        actual_byte_count = path.stat().st_size if present and path else 0
        actual_sha256 = sha256_file(path) if present and path else ""
        records.append(
            {
                "name": record.get("name"),
                "packet_relative_path": expected_relative,
                "required": record.get("required") is True,
                "present": present,
                "expected_present": record.get("present"),
                "expected_byte_count": record.get("byte_count"),
                "actual_byte_count": actual_byte_count,
                "expected_sha256": record.get("sha256"),
                "actual_sha256": actual_sha256,
                "byte_count_matches": record.get("byte_count") == actual_byte_count,
                "sha256_matches": record.get("sha256") == actual_sha256,
                "path": str(path) if path else "",
            }
        )
    return records


def record_by_name(records: list[dict[str, Any]], name: str) -> dict[str, Any]:
    for record in records:
        if record.get("name") == name:
            return record
    return {}


def load_packet_json(records: list[dict[str, Any]], name: str) -> dict[str, Any]:
    record = record_by_name(records, name)
    path_text = record.get("path")
    if not isinstance(path_text, str) or not path_text:
        return {}
    return load_json(Path(path_text))


def stop_conditions_passed(manifest: dict[str, Any]) -> bool:
    conditions = [
        condition
        for condition in as_list(manifest.get("stop_conditions"))
        if isinstance(condition, dict)
    ]
    return bool(conditions) and all(condition.get("passed") is True for condition in conditions)


def build_summary(
    packet_dir: Path,
    manifest_json: Path,
    summary_json: Path,
    summary_report: Path,
) -> dict[str, Any]:
    manifest = load_json(manifest_json)
    records = packet_file_records(packet_dir, manifest)
    packet_manifest = as_dict(manifest.get("packet_manifest"))
    context_summary = as_dict(manifest.get("context_summary"))
    context = load_packet_json(records, "source_context_json")
    diagnosis = load_packet_json(records, "diagnosis_json")
    fix_handoff = load_packet_json(records, "fix_handoff_json")
    route_check = load_packet_json(records, "route_check_json")
    replay_triage = load_packet_json(records, "replay_triage_json")
    replay_telemetry = load_packet_json(records, "replay_telemetry_json")
    ai_index = load_packet_json(records, "ai_index_json")
    ai_query = load_packet_json(records, "ai_query_smoke_json")
    route_matrix = load_packet_json(records, "ai_route_matrix_json")

    required_records = [record for record in records if record.get("required")]
    missing_required = [
        record.get("name") for record in required_records if record.get("present") is not True
    ]
    digest_mismatches = [
        record.get("name")
        for record in records
        if record.get("present") is True
        and (record.get("byte_count_matches") is not True or record.get("sha256_matches") is not True)
    ]
    expected_present_mismatches = [
        record.get("name")
        for record in records
        if record.get("expected_present") is True and record.get("present") is not True
    ]

    source_window_count = context_window_count(context.get("source_context"))
    test_window_count = context_window_count(context.get("test_context"))
    manifest_identity = selection_identity(manifest)
    diagnosis_identity = selection_identity(diagnosis)
    fix_identity = selection_identity(fix_handoff)
    read_order_missing = [
        step.get("artifact")
        for step in as_list(manifest.get("read_order"))
        if isinstance(step, dict)
        and not (packet_path(packet_dir, step.get("artifact")) or Path()).is_file()
    ]

    checks: list[dict[str, Any]] = []
    errors: list[str] = []
    add_check(checks, errors, "packet_dir_exists", packet_dir.is_dir(), str(packet_dir))
    add_check(checks, errors, "manifest_loaded", bool(manifest), str(manifest_json))
    add_check(checks, errors, "manifest_status_passed", manifest.get("status") == "passed", manifest.get("status"))
    add_check(
        checks,
        errors,
        "selection_identity_present",
        all(isinstance(manifest_identity.get(key), str) and manifest_identity.get(key) for key in manifest_identity),
        manifest_identity,
    )
    add_check(
        checks,
        errors,
        "packet_file_count_matches",
        packet_manifest.get("file_count") == len(records),
        {"declared": packet_manifest.get("file_count"), "actual": len(records)},
    )
    add_check(
        checks,
        errors,
        "required_file_count_matches",
        packet_manifest.get("required_file_count") == len(required_records),
        {"declared": packet_manifest.get("required_file_count"), "actual": len(required_records)},
    )
    add_check(
        checks,
        errors,
        "required_packet_files_present",
        not missing_required and packet_manifest.get("missing_required_file_count") == 0,
        {"declared_missing": packet_manifest.get("missing_required_file_count"), "missing": missing_required},
    )
    add_check(
        checks,
        errors,
        "packet_file_digests_match",
        not digest_mismatches,
        digest_mismatches,
    )
    add_check(
        checks,
        errors,
        "packet_expected_presence_matches",
        not expected_present_mismatches,
        expected_present_mismatches,
    )
    add_check(checks, errors, "read_order_artifacts_present", not read_order_missing, read_order_missing)
    add_check(checks, errors, "readme_present", (packet_dir / "README.md").is_file(), str(packet_dir / "README.md"))
    add_check(checks, errors, "source_context_loaded", bool(context), record_by_name(records, "source_context_json"))
    add_check(
        checks,
        errors,
        "source_context_windows_present",
        source_window_count > 0 and source_window_count == context_summary.get("source_window_count"),
        {"declared": context_summary.get("source_window_count"), "actual": source_window_count},
    )
    add_check(
        checks,
        errors,
        "test_context_windows_present",
        test_window_count > 0 and test_window_count == context_summary.get("test_window_count"),
        {"declared": context_summary.get("test_window_count"), "actual": test_window_count},
    )
    add_check(checks, errors, "diagnosis_loaded", bool(diagnosis), record_by_name(records, "diagnosis_json"))
    add_check(checks, errors, "diagnosis_passed", diagnosis.get("status") == "passed", diagnosis.get("status"))
    add_check(checks, errors, "fix_handoff_loaded", bool(fix_handoff), record_by_name(records, "fix_handoff_json"))
    add_check(checks, errors, "fix_handoff_passed", fix_handoff.get("status") == "passed", fix_handoff.get("status"))
    add_check(
        checks,
        errors,
        "diagnosis_identity_matches_manifest",
        matching_identity(manifest_identity, diagnosis_identity),
        {"manifest": manifest_identity, "diagnosis": diagnosis_identity},
    )
    add_check(
        checks,
        errors,
        "fix_handoff_identity_matches_manifest",
        matching_identity(manifest_identity, fix_identity),
        {"manifest": manifest_identity, "fix_handoff": fix_identity},
    )
    add_check(checks, errors, "route_check_loaded", bool(route_check), record_by_name(records, "route_check_json"))
    add_check(checks, errors, "route_check_passed", route_check.get("status") == "passed", route_check.get("status"))
    add_check(checks, errors, "replay_triage_loaded", bool(replay_triage), record_by_name(records, "replay_triage_json"))
    add_check(checks, errors, "replay_telemetry_loaded", bool(replay_telemetry), record_by_name(records, "replay_telemetry_json"))
    add_check(checks, errors, "ai_index_loaded", bool(ai_index), record_by_name(records, "ai_index_json"))
    add_check(checks, errors, "ai_index_passed", ai_index.get("status") == "passed", ai_index.get("status"))
    add_check(checks, errors, "ai_query_loaded", bool(ai_query), record_by_name(records, "ai_query_smoke_json"))
    add_check(checks, errors, "ai_query_passed", ai_query.get("status") == "passed", ai_query.get("status"))
    add_check(
        checks,
        errors,
        "ai_route_matrix_passed",
        not route_matrix or route_matrix.get("status") == "passed",
        route_matrix.get("status"),
    )
    add_check(
        checks,
        errors,
        "narrow_test_commands_present",
        bool(as_list(as_dict(fix_handoff.get("fix_commands")).get("narrow_test_commands"))),
        len(as_list(as_dict(fix_handoff.get("fix_commands")).get("narrow_test_commands"))),
    )
    add_check(
        checks,
        errors,
        "packet_stop_conditions_passed",
        stop_conditions_passed(manifest),
        as_list(manifest.get("stop_conditions")),
    )

    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_debug_packet_verification_schema_version": AI_DEBUG_PACKET_VERIFICATION_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "packet_dir": str(packet_dir),
        "manifest_json": str(manifest_json),
        "selection": manifest_identity,
        "summary": {
            "check_count": len(checks),
            "passed_check_count": sum(1 for check in checks if check.get("passed") is True),
            "packet_file_count": len(records),
            "required_file_count": len(required_records),
            "missing_required_file_count": len(missing_required),
            "digest_mismatch_count": len(digest_mismatches),
            "source_window_count": source_window_count,
            "test_window_count": test_window_count,
            "narrow_test_command_count": len(as_list(as_dict(fix_handoff.get("fix_commands")).get("narrow_test_commands"))),
        },
        "packet_files": records,
        "checks": checks,
        "artifacts": {
            "diagnostic_ai_debug_packet_verification_json": str(summary_json),
            "diagnostic_ai_debug_packet_verification_report": str(summary_report),
            "diagnostic_ai_debug_packet_manifest": str(manifest_json),
            "diagnostic_ai_debug_packet_readme": str(packet_dir / "README.md"),
        },
        "errors": errors,
        "ai_handoff": [
            "Use this verifier when a debug packet has been copied away from its original scenario suite.",
            "A passed verifier means packet-local files, digests, route identity, source/test context, replay evidence, and narrow commands are internally consistent.",
            "If this verifier fails, rebuild the debug packet before allowing an automated debugger to edit emulator code from it.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    selection = as_dict(summary.get("selection"))
    lines = [
        "# Diagnostic AI Debug Packet Verification",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Packet dir | {markdown_cell(summary.get('packet_dir'))} |",
        f"| Manifest JSON | {markdown_cell(summary.get('manifest_json'))} |",
        f"| Route | {markdown_cell(selection.get('route_id'))} |",
        f"| Scenario | {markdown_cell(selection.get('scenario_id'))} |",
        f"| Focus domain | {markdown_cell(selection.get('focus_domain'))} |",
        f"| Probe | {markdown_cell(selection.get('probe_id'))} |",
        f"| Checks | {totals.get('passed_check_count')}/{totals.get('check_count')} |",
        f"| Packet files | {totals.get('packet_file_count')} |",
        f"| Missing required files | {totals.get('missing_required_file_count')} |",
        f"| Digest mismatches | {totals.get('digest_mismatch_count')} |",
        f"| Source windows | {totals.get('source_window_count')} |",
        f"| Test windows | {totals.get('test_window_count')} |",
        f"| Narrow commands | {totals.get('narrow_test_command_count')} |",
        "",
        "## Checks",
        "",
        "| Name | Passed | Detail |",
        "| --- | --- | --- |",
    ]
    for check in as_list(summary.get("checks")):
        if isinstance(check, dict):
            lines.append(
                f"| {markdown_cell(check.get('name'))} | {check.get('passed')} | {markdown_cell(check.get('detail'))} |"
            )
    lines.extend(["", "## Packet Files", "", "| Name | Present | Digest matches | Path |", "| --- | --- | --- | --- |"])
    for record in as_list(summary.get("packet_files")):
        if isinstance(record, dict):
            lines.append(
                f"| {markdown_cell(record.get('name'))} | {record.get('present')} | {record.get('sha256_matches')} | {markdown_cell(record.get('packet_relative_path'))} |"
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
        "--packet-dir",
        required=True,
        type=Path,
        help="Directory containing manifest.json and copied AI debug packet files.",
    )
    parser.add_argument(
        "--manifest-json",
        type=Path,
        help="Explicit packet manifest path. Defaults to <packet-dir>/manifest.json.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write packet verification JSON.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write packet verification Markdown.",
    )
    parser.add_argument("--json", action="store_true", help="Print verification JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    packet_dir = args.packet_dir
    manifest_json = args.manifest_json or packet_dir / "manifest.json"
    summary_json = args.summary_json or packet_dir.parent / "diagnostic-ai-debug-packet-verification.json"
    summary_report = args.summary_report or packet_dir.parent / "diagnostic-ai-debug-packet-verification.md"
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)
    summary = build_summary(packet_dir, manifest_json, summary_json, summary_report)
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        totals = as_dict(summary.get("summary"))
        selection = as_dict(summary.get("selection"))
        print(
            "Diagnostic AI debug packet verification "
            f"{summary['status']}: packet_dir={packet_dir} "
            f"route={selection.get('route_id')} "
            f"checks={totals.get('passed_check_count')}/{totals.get('check_count')} "
            f"files={totals.get('packet_file_count')} "
            f"digest_mismatches={totals.get('digest_mismatch_count')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
