#!/usr/bin/env python3
"""Build a relocatable AI debug packet from an accepted diagnostic suite."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_DEBUG_PACKET_SCHEMA_VERSION = 1
DEFAULT_CONTEXT_LINES = 3
DEFAULT_MAX_ANCHORS_PER_FILE = 8


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
    suite_name = suite_dir.name
    marker = f"/{suite_name}/"
    if marker in normalized:
        candidates.append(suite_dir / normalized.split(marker, 1)[1])
    return candidates


def resolve_artifact_path(
    suite_dir: Path,
    original_suite_dirs: list[str],
    value: Any,
) -> Path | None:
    candidates = path_candidates(suite_dir, original_suite_dirs, value)
    for candidate in candidates:
        if candidate.exists():
            return candidate
    return candidates[-1] if candidates else None


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def packet_record(name: str, output_dir: Path, path: Path) -> dict[str, Any]:
    return {
        "name": name,
        "relative_path": path.relative_to(output_dir.parent).as_posix(),
        "packet_relative_path": path.relative_to(output_dir).as_posix(),
        "path": str(path),
        "present": path.is_file(),
        "byte_count": path.stat().st_size if path.is_file() else 0,
        "sha256": sha256_file(path) if path.is_file() else "",
    }


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def copy_packet_file(
    name: str,
    source: Path | None,
    output_dir: Path,
    relative_path: str,
    required: bool,
    errors: list[str],
) -> dict[str, Any]:
    destination = output_dir / relative_path
    if source is None or not source.is_file():
        if required:
            errors.append(f"missing packet source: {name}")
        return {
            "name": name,
            "relative_path": destination.relative_to(output_dir.parent).as_posix(),
            "packet_relative_path": destination.relative_to(output_dir).as_posix(),
            "path": str(destination),
            "present": False,
            "required": required,
            "source_path": str(source) if source is not None else "",
            "byte_count": 0,
            "sha256": "",
        }
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    record = packet_record(name, output_dir, destination)
    record["required"] = required
    record["source_path"] = str(source)
    return record


def find_by_key(rows: Any, key: str, value: Any) -> dict[str, Any]:
    if not isinstance(value, str):
        return {}
    for row in as_list(rows):
        if isinstance(row, dict) and row.get(key) == value:
            return row
    return {}


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


def default_diagnosis_json(args: argparse.Namespace) -> Path:
    for candidate in (
        args.suite_dir / "diagnostic-ai-diagnosis-smoke.json",
        args.suite_dir / "ai-diagnosis-smoke" / "diagnostic-ai-diagnosis.json",
    ):
        if candidate.is_file():
            return candidate
    return args.suite_dir / "diagnostic-ai-diagnosis-smoke.json"


def default_fix_handoff_json(args: argparse.Namespace) -> Path:
    return args.suite_dir / "diagnostic-ai-fix-handoff-smoke.json"


def route_matrix_paths(
    args: argparse.Namespace,
    route_matrix: dict[str, Any],
    original_suite_dirs: list[str],
) -> tuple[Path | None, Path | None]:
    if not args.route_id:
        return None, None
    row = find_by_key(route_matrix.get("routes"), "route_id", args.route_id)
    artifacts = as_dict(row.get("artifacts"))
    diagnosis = resolve_artifact_path(
        args.suite_dir,
        original_suite_dirs,
        artifacts.get("diagnostic_ai_diagnosis_json"),
    )
    fix_handoff = resolve_artifact_path(
        args.suite_dir,
        original_suite_dirs,
        artifacts.get("diagnostic_ai_fix_handoff_json"),
    )
    return diagnosis, fix_handoff


def resolve_inputs(args: argparse.Namespace) -> tuple[Path, Path, dict[str, Any]]:
    route_matrix = load_json(args.suite_dir / "diagnostic-ai-route-matrix.json")
    original_suite_dirs = unique_strings([str(args.suite_dir), route_matrix.get("suite_dir")])
    matrix_diagnosis, matrix_fix = route_matrix_paths(args, route_matrix, original_suite_dirs)
    diagnosis_json = args.diagnosis_json or matrix_diagnosis or default_diagnosis_json(args)
    fix_handoff_json = args.fix_handoff_json or matrix_fix or default_fix_handoff_json(args)
    return diagnosis_json, fix_handoff_json, route_matrix


def resolve_workspace_path(repo_root: Path, value: Any) -> Path:
    path = Path(str(value or ""))
    if path.is_absolute():
        return path
    return repo_root / normalized_path_text(value)


def source_lines(path: Path) -> list[str]:
    if not path.is_file():
        return []
    return path.read_text(encoding="utf-8", errors="replace").splitlines()


def context_windows_for_file(
    repo_root: Path,
    scan_row: dict[str, Any],
    context_lines: int,
    max_anchors_per_file: int,
) -> dict[str, Any]:
    source_path = resolve_workspace_path(repo_root, scan_row.get("path"))
    lines = source_lines(source_path)
    windows: list[dict[str, Any]] = []
    for match in as_list(scan_row.get("matches"))[:max_anchors_per_file]:
        if not isinstance(match, dict):
            continue
        line_no = match.get("line")
        if not isinstance(line_no, int) or line_no < 1:
            continue
        start = max(1, line_no - context_lines)
        end = min(len(lines), line_no + context_lines)
        snippets = [
            {
                "line": current,
                "text": lines[current - 1],
                "is_anchor": current == line_no,
            }
            for current in range(start, end + 1)
            if 0 <= current - 1 < len(lines)
        ]
        windows.append(
            {
                "anchor_line": line_no,
                "matched_terms": as_str_list(match.get("terms")),
                "line_start": start,
                "line_end": end,
                "snippets": snippets,
            }
        )
    return {
        "path": scan_row.get("path"),
        "resolved_path": str(source_path),
        "exists": source_path.is_file(),
        "matched_line_count": scan_row.get("matched_line_count"),
        "window_count": len(windows),
        "windows": windows,
    }


def build_context(
    repo_root: Path,
    fix_handoff: dict[str, Any],
    context_lines: int,
    max_anchors_per_file: int,
) -> dict[str, Any]:
    source_scan = as_dict(fix_handoff.get("source_scan"))
    test_scan = as_dict(fix_handoff.get("test_scan"))
    return {
        "context_lines": context_lines,
        "max_anchors_per_file": max_anchors_per_file,
        "source_context": [
            context_windows_for_file(repo_root, row, context_lines, max_anchors_per_file)
            for row in as_list(source_scan.get("files"))
            if isinstance(row, dict)
        ],
        "test_context": [
            context_windows_for_file(repo_root, row, context_lines, max_anchors_per_file)
            for row in as_list(test_scan.get("files"))
            if isinstance(row, dict)
        ],
    }


def context_window_count(rows: Any) -> int:
    return sum(int(as_dict(row).get("window_count") or 0) for row in as_list(rows))


def output_artifacts(summary_json: Path, summary_report: Path, output_dir: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_debug_packet_json": str(summary_json),
        "diagnostic_ai_debug_packet_report": str(summary_report),
        "diagnostic_ai_debug_packet_dir": str(output_dir),
        "diagnostic_ai_debug_packet_manifest": str(output_dir / "manifest.json"),
        "diagnostic_ai_debug_packet_readme": str(output_dir / "README.md"),
        "diagnostic_ai_debug_packet_source_context": str(output_dir / "source-context.json"),
    }


def read_order() -> list[dict[str, Any]]:
    return [
        {
            "order": 1,
            "artifact": "manifest.json",
            "purpose": "Check packet status, selected route identity, file digests, and read order.",
        },
        {
            "order": 2,
            "artifact": "replay/triage.json",
            "purpose": "Confirm the failure signature and debug focus before reading full telemetry.",
        },
        {
            "order": 3,
            "artifact": "fix/diagnostic-ai-fix-handoff.json",
            "purpose": "Load bounded source/test anchors, commands, and fix-loop stop conditions.",
        },
        {
            "order": 4,
            "artifact": "source-context.json",
            "purpose": "Inspect compact source and test windows around matched anchors.",
        },
        {
            "order": 5,
            "artifact": "replay/telemetry.json",
            "purpose": "Open full replay telemetry only after the compact route evidence is accepted.",
        },
    ]


def packet_copy_specs(
    args: argparse.Namespace,
    diagnosis_json: Path,
    fix_handoff_json: Path,
    diagnosis: dict[str, Any],
    fix_handoff: dict[str, Any],
    route_matrix: dict[str, Any],
    original_suite_dirs: list[str],
) -> list[tuple[str, Path | None, str, bool]]:
    diagnosis_artifacts = as_dict(diagnosis.get("artifacts"))
    fix_artifacts = as_dict(fix_handoff.get("artifacts"))

    def suite_file(name: str) -> Path | None:
        path = args.suite_dir / name
        return path if path.is_file() else None

    def resolved(value: Any) -> Path | None:
        return resolve_artifact_path(args.suite_dir, original_suite_dirs, value)

    route_matrix_path = args.suite_dir / "diagnostic-ai-route-matrix.json"
    route_matrix_required = bool(route_matrix)
    return [
        ("ai_index_json", suite_file("diagnostic-ai-observability-index.json"), "index/diagnostic-ai-observability-index.json", True),
        ("ai_query_smoke_json", suite_file("diagnostic-ai-query-smoke.json"), "index/diagnostic-ai-query-smoke.json", True),
        ("ai_route_matrix_json", route_matrix_path if route_matrix_path.is_file() else None, "index/diagnostic-ai-route-matrix.json", route_matrix_required),
        ("diagnosis_json", diagnosis_json, "diagnosis/diagnostic-ai-diagnosis.json", True),
        ("diagnosis_report", resolved(diagnosis_artifacts.get("diagnostic_ai_diagnosis_report")), "diagnosis/diagnostic-ai-diagnosis.md", True),
        ("fix_handoff_json", fix_handoff_json, "fix/diagnostic-ai-fix-handoff.json", True),
        ("fix_handoff_report", resolved(fix_artifacts.get("diagnostic_ai_fix_handoff_report")), "fix/diagnostic-ai-fix-handoff.md", True),
        ("route_check_json", resolved(diagnosis_artifacts.get("route_check_json")), "route-check/diagnostic-route-check.json", True),
        ("route_check_report", resolved(diagnosis_artifacts.get("route_check_report")), "route-check/diagnostic-route-check.md", True),
        ("replay_manifest", resolved(diagnosis_artifacts.get("replay_bundle_manifest")), "replay/manifest.json", True),
        ("replay_triage_json", resolved(diagnosis_artifacts.get("replay_bundle_triage_json")), "replay/triage.json", True),
        ("replay_telemetry_json", resolved(diagnosis_artifacts.get("replay_bundle_telemetry_json")), "replay/telemetry.json", True),
        ("replay_report", resolved(diagnosis_artifacts.get("replay_bundle_report")), "replay/report.md", True),
        ("replay_rom", resolved(diagnosis_artifacts.get("replay_bundle_rom")), "replay/diagnostic.nes", True),
    ]


def build_summary(
    args: argparse.Namespace,
    repo_root: Path,
    diagnosis_json: Path,
    fix_handoff_json: Path,
    route_matrix: dict[str, Any],
    summary_json: Path,
    summary_report: Path,
    output_dir: Path,
) -> dict[str, Any]:
    output_dir.mkdir(parents=True, exist_ok=True)
    diagnosis = load_json(diagnosis_json)
    fix_handoff = load_json(fix_handoff_json)
    ai_index = load_json(args.suite_dir / "diagnostic-ai-observability-index.json")
    ai_query = load_json(args.suite_dir / "diagnostic-ai-query-smoke.json")
    original_suite_dirs = unique_strings(
        [
            str(args.suite_dir),
            diagnosis.get("suite_dir"),
            fix_handoff.get("suite_dir"),
            route_matrix.get("suite_dir"),
        ]
    )
    diagnosis_identity = selection_identity(diagnosis)
    fix_identity = selection_identity(fix_handoff)
    packet_errors: list[str] = []
    packet_files = [
        copy_packet_file(name, source, output_dir, relative, required, packet_errors)
        for name, source, relative, required in packet_copy_specs(
            args,
            diagnosis_json,
            fix_handoff_json,
            diagnosis,
            fix_handoff,
            route_matrix,
            original_suite_dirs,
        )
    ]
    context = build_context(
        repo_root,
        fix_handoff,
        args.context_lines,
        args.max_anchors_per_file,
    )
    source_context_json = output_dir / "source-context.json"
    write_json(source_context_json, context)
    source_context_record = packet_record("source_context_json", output_dir, source_context_json)
    source_context_record["required"] = True
    source_context_record["source_path"] = "generated"
    packet_files.append(source_context_record)

    fix_commands = as_dict(fix_handoff.get("fix_commands"))
    source_window_count = context_window_count(context.get("source_context"))
    test_window_count = context_window_count(context.get("test_context"))
    required_missing = [
        record.get("name")
        for record in packet_files
        if record.get("required") is True and record.get("present") is not True
    ]
    errors: list[str] = []
    if not diagnosis:
        errors.append(f"missing or invalid diagnosis JSON: {diagnosis_json}")
    elif diagnosis.get("status") != "passed":
        errors.append(f"diagnosis status is {diagnosis.get('status')!r}, expected 'passed'")
    if not fix_handoff:
        errors.append(f"missing or invalid fix handoff JSON: {fix_handoff_json}")
    elif fix_handoff.get("status") != "passed":
        errors.append(f"fix handoff status is {fix_handoff.get('status')!r}, expected 'passed'")
    if not matching_identity(diagnosis_identity, fix_identity):
        errors.append("diagnosis and fix handoff route identities do not match")
    if ai_index and ai_index.get("status") != "passed":
        errors.append("AI index status is not passed")
    if ai_query and ai_query.get("status") != "passed":
        errors.append("AI query smoke status is not passed")
    if route_matrix and route_matrix.get("status") != "passed":
        errors.append("AI route matrix status is not passed")
    if required_missing:
        errors.append(f"missing required packet files: {', '.join(str(name) for name in required_missing)}")
    if source_window_count < 1:
        errors.append("source context has no matched windows")
    if test_window_count < 1:
        errors.append("test context has no matched windows")
    if not as_list(fix_commands.get("narrow_test_commands")):
        errors.append("fix handoff has no narrow test commands")
    errors.extend(packet_errors)

    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_debug_packet_schema_version": AI_DEBUG_PACKET_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(args.suite_dir),
        "packet_dir": str(output_dir),
        "diagnosis_json": str(diagnosis_json),
        "fix_handoff_json": str(fix_handoff_json),
        "selection": diagnosis_identity,
        "failure_signature": as_dict(fix_handoff.get("failure_signature")),
        "packet_manifest": {
            "file_count": len(packet_files),
            "required_file_count": sum(1 for record in packet_files if record.get("required")),
            "missing_required_file_count": len(required_missing),
            "byte_count": sum(int(record.get("byte_count") or 0) for record in packet_files),
            "files": packet_files,
        },
        "context_summary": {
            "source_file_count": len(as_list(context.get("source_context"))),
            "source_window_count": source_window_count,
            "test_file_count": len(as_list(context.get("test_context"))),
            "test_window_count": test_window_count,
        },
        "source_context": context,
        "fix_commands": fix_commands,
        "read_order": read_order(),
        "artifacts": output_artifacts(summary_json, summary_report, output_dir),
        "stop_conditions": [
            {
                "name": "diagnosis_passed",
                "passed": diagnosis.get("status") == "passed",
                "detail": diagnosis.get("status"),
            },
            {
                "name": "fix_handoff_passed",
                "passed": fix_handoff.get("status") == "passed",
                "detail": fix_handoff.get("status"),
            },
            {
                "name": "route_identity_matches",
                "passed": matching_identity(diagnosis_identity, fix_identity),
                "detail": {"diagnosis": diagnosis_identity, "fix_handoff": fix_identity},
            },
            {
                "name": "required_packet_files_present",
                "passed": not required_missing,
                "detail": required_missing,
            },
            {
                "name": "source_and_test_context_present",
                "passed": source_window_count > 0 and test_window_count > 0,
                "detail": {
                    "source_window_count": source_window_count,
                    "test_window_count": test_window_count,
                },
            },
            {
                "name": "narrow_test_commands_present",
                "passed": bool(as_list(fix_commands.get("narrow_test_commands"))),
                "detail": len(as_list(fix_commands.get("narrow_test_commands"))),
            },
        ],
        "errors": errors,
        "ai_handoff": [
            "Use this packet as the first self-contained artifact for an AI debugger working on the selected diagnostic route.",
            "Start with ai-debug-packet/manifest.json, then replay/triage.json, then fix/diagnostic-ai-fix-handoff.json.",
            "Use source-context.json before opening full source files; it contains bounded windows around mapped source/test anchors.",
            "Run fix_commands.narrow_test_commands after an edit, then regenerate the diagnosis packet and full diagnostic e2e suite.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    selection = as_dict(summary.get("selection"))
    manifest = as_dict(summary.get("packet_manifest"))
    context = as_dict(summary.get("context_summary"))
    commands = as_dict(summary.get("fix_commands"))
    lines = [
        "# Diagnostic AI Debug Packet",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Packet dir | {markdown_cell(summary.get('packet_dir'))} |",
        f"| Route | {markdown_cell(selection.get('route_id'))} |",
        f"| Scenario | {markdown_cell(selection.get('scenario_id'))} |",
        f"| Focus domain | {markdown_cell(selection.get('focus_domain'))} |",
        f"| Probe | {markdown_cell(selection.get('probe_id'))} |",
        f"| Packet files | {manifest.get('file_count')} |",
        f"| Missing required files | {manifest.get('missing_required_file_count')} |",
        f"| Source windows | {context.get('source_window_count')} |",
        f"| Test windows | {context.get('test_window_count')} |",
        "",
        "## Read Order",
        "",
        "| Order | Artifact | Purpose |",
        "| ---: | --- | --- |",
    ]
    for step in as_list(summary.get("read_order")):
        if isinstance(step, dict):
            lines.append(
                f"| {step.get('order')} | {markdown_cell(step.get('artifact'))} | {markdown_cell(step.get('purpose'))} |"
            )
    lines.extend(["", "## Packet Files", "", "| Name | Present | Bytes | Path |", "| --- | --- | ---: | --- |"])
    for record in as_list(manifest.get("files")):
        if isinstance(record, dict):
            lines.append(
                f"| {markdown_cell(record.get('name'))} | {record.get('present')} | {record.get('byte_count')} | {markdown_cell(record.get('relative_path'))} |"
            )
    lines.extend(["", "## Narrow Commands", "", "| Purpose | Command |", "| --- | --- |"])
    for command in as_list(commands.get("narrow_test_commands")):
        if isinstance(command, dict):
            lines.append(
                f"| {markdown_cell(command.get('purpose'))} | `{markdown_cell(command.get('text'))}` |"
            )
    lines.extend(["", "## Stop Conditions", "", "| Name | Passed | Detail |", "| --- | --- | --- |"])
    for condition in as_list(summary.get("stop_conditions")):
        if isinstance(condition, dict):
            lines.append(
                f"| {markdown_cell(condition.get('name'))} | {condition.get('passed')} | {markdown_cell(condition.get('detail'))} |"
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
    parser.add_argument("--route-id", help="Route id to package from diagnostic-ai-route-matrix.json.")
    parser.add_argument("--diagnosis-json", type=Path, help="Explicit diagnosis JSON to package.")
    parser.add_argument("--fix-handoff-json", type=Path, help="Explicit fix-handoff JSON to package.")
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="Directory for the copied packet files. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write the debug-packet JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the debug-packet Markdown report. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--context-lines",
        type=int,
        default=DEFAULT_CONTEXT_LINES,
        help="Source/test lines to include before and after each matched anchor.",
    )
    parser.add_argument(
        "--max-anchors-per-file",
        type=int,
        default=DEFAULT_MAX_ANCHORS_PER_FILE,
        help="Maximum matched anchors to expand per source or test file.",
    )
    parser.add_argument("--json", action="store_true", help="Print the debug packet JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    diagnosis_json, fix_handoff_json, route_matrix = resolve_inputs(args)
    output_dir = args.output_dir or args.suite_dir / "ai-debug-packet"
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-debug-packet.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-debug-packet.md"
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)
    summary = build_summary(
        args,
        repo_root,
        diagnosis_json,
        fix_handoff_json,
        route_matrix,
        summary_json,
        summary_report,
        output_dir,
    )
    write_json(summary_json, summary)
    write_markdown(summary_report, summary)
    write_json(output_dir / "manifest.json", summary)
    shutil.copy2(summary_report, output_dir / "README.md")

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        selection = as_dict(summary.get("selection"))
        manifest = as_dict(summary.get("packet_manifest"))
        context = as_dict(summary.get("context_summary"))
        print(
            "Diagnostic AI debug packet "
            f"{summary['status']}: route={selection.get('route_id')} "
            f"scenario={selection.get('scenario_id')} "
            f"files={manifest.get('file_count')} "
            f"source_windows={context.get('source_window_count')} "
            f"test_windows={context.get('test_window_count')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
