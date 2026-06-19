#!/usr/bin/env python3
"""Build an AI-facing source/test fix handoff from a diagnostic diagnosis report."""

from __future__ import annotations

import argparse
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


FIX_HANDOFF_SCHEMA_VERSION = 1
DEFAULT_MAX_MATCHES_PER_FILE = 24
DIAGNOSTIC_TEST_CONTEXT_FILE = "tests/diagnostic_cartridge_tests.rs"


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


def sorted_unique(values: list[Any]) -> list[str]:
    return sorted({value for value in values if isinstance(value, str) and value})


def test_context_files(paths: list[str]) -> list[str]:
    return sorted_unique([*paths, DIAGNOSTIC_TEST_CONTEXT_FILE])


def command_text(command: dict[str, Any]) -> str:
    argv = [str(value) for value in as_list(command.get("argv"))]
    return " ".join(argv)


def command_records(commands: Any) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for command in as_list(commands):
        if not isinstance(command, dict):
            continue
        argv = [str(value) for value in as_list(command.get("argv"))]
        records.append(
            {
                "purpose": command.get("purpose"),
                "argv": argv,
                "text": " ".join(argv),
            }
        )
    return records


def diagnosis_json_candidates(args: argparse.Namespace) -> list[Path]:
    candidates: list[Path] = []
    if args.diagnosis_json:
        candidates.append(args.diagnosis_json)
    candidates.extend(
        [
            args.suite_dir / "diagnostic-ai-diagnosis-smoke.json",
            args.suite_dir / "ai-diagnosis-smoke" / "diagnostic-ai-diagnosis.json",
        ]
    )
    return candidates


def resolve_diagnosis_json(args: argparse.Namespace) -> Path:
    for candidate in diagnosis_json_candidates(args):
        if candidate.is_file():
            return candidate
    return args.diagnosis_json or args.suite_dir / "diagnostic-ai-diagnosis-smoke.json"


def output_artifacts(summary_json: Path, summary_report: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_fix_handoff_json": str(summary_json),
        "diagnostic_ai_fix_handoff_report": str(summary_report),
    }


def artifact_presence(artifacts: dict[str, str]) -> dict[str, bool]:
    result: dict[str, bool] = {}
    for name, value in artifacts.items():
        if not isinstance(value, str) or not value:
            result[name] = False
            continue
        path = Path(value)
        if not path.is_absolute():
            path = Path(value.replace("\\", "/"))
        result[name] = path.is_dir() if name.endswith("_dir") else path.is_file()
    return result


def resolve_workspace_path(repo_root: Path, value: str) -> Path:
    path = Path(value)
    if path.is_absolute():
        return path
    return repo_root / value.replace("\\", "/")


def normalized_path_text(value: Any) -> str:
    return value.replace("\\", "/") if isinstance(value, str) else ""


def localize_suite_artifact_path(suite_dir: Path, original_suite_dir: Any, value: Any) -> str:
    normalized = normalized_path_text(value)
    if not normalized:
        return ""
    path = Path(normalized)
    if path.exists():
        return normalized

    original_suite = normalized_path_text(original_suite_dir).rstrip("/")
    if original_suite and normalized.startswith(original_suite + "/"):
        suffix = normalized[len(original_suite) + 1 :]
        relocated = suite_dir / suffix
        if relocated.exists():
            return str(relocated)

    suite_name = suite_dir.name
    marker = f"/{suite_name}/"
    if marker in normalized:
        suffix = normalized.split(marker, 1)[1]
        relocated = suite_dir / suffix
        if relocated.exists():
            return str(relocated)

    return normalized


def line_matches(line: str, terms: list[str]) -> list[str]:
    lowered = line.lower()
    return [term for term in terms if term and term.lower() in lowered]


def scan_file(
    repo_root: Path,
    path_text: str,
    terms: list[str],
    max_matches: int,
) -> dict[str, Any]:
    path = resolve_workspace_path(repo_root, path_text)
    exists = path.is_file()
    matches: list[dict[str, Any]] = []
    line_count = 0
    if exists:
        lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
        line_count = len(lines)
        for line_no, line in enumerate(lines, start=1):
            matched_terms = line_matches(line, terms)
            if not matched_terms:
                continue
            matches.append(
                {
                    "line": line_no,
                    "terms": matched_terms,
                    "text": line.strip()[:240],
                }
            )
            if len(matches) >= max_matches:
                break
    return {
        "path": path_text,
        "resolved_path": str(path),
        "exists": exists,
        "line_count": line_count,
        "matched_line_count": len(matches),
        "matches": matches,
    }


def scan_paths(
    repo_root: Path,
    paths: list[str],
    terms: list[str],
    max_matches: int,
) -> list[dict[str, Any]]:
    return [scan_file(repo_root, path, terms, max_matches) for path in paths]


def total_matches(rows: list[dict[str, Any]]) -> int:
    return sum(int(row.get("matched_line_count") or 0) for row in rows)


def missing_paths(rows: list[dict[str, Any]]) -> list[str]:
    return [str(row.get("path")) for row in rows if row.get("exists") is not True]


def test_search_terms(selection: dict[str, Any], evidence: dict[str, Any]) -> list[str]:
    return sorted_unique(
        [
            selection.get("scenario_id"),
            selection.get("focus_domain"),
            selection.get("probe_id"),
            *as_str_list(evidence.get("search_terms")),
        ]
    )


def narrow_commands(evidence: dict[str, Any]) -> list[dict[str, Any]]:
    commands = command_records(evidence.get("suggested_commands"))
    return [
        command
        for command in commands
        if "cargo test" in command.get("text", "") or command.get("purpose") != "Replay the first mapped scenario"
    ]


def replay_commands(evidence: dict[str, Any]) -> list[dict[str, Any]]:
    commands = command_records(evidence.get("suggested_commands"))
    return [
        command
        for command in commands
        if "cargo run --bin oxidenes-diagnostic" in command.get("text", "")
    ]


def verification_commands(args: argparse.Namespace, diagnosis: dict[str, Any]) -> list[dict[str, Any]]:
    selection = as_dict(diagnosis.get("selection"))
    route_id = selection.get("route_id")
    selector = ["--route-id", str(route_id)] if route_id else []
    return [
        {
            "purpose": "Regenerate the selected AI diagnosis handoff",
            "argv": [
                "python",
                "scripts/run_diagnostic_ai_diagnosis.py",
                "--suite-dir",
                str(args.suite_dir),
                *selector,
            ],
            "text": " ".join(
                [
                    "python",
                    "scripts/run_diagnostic_ai_diagnosis.py",
                    "--suite-dir",
                    str(args.suite_dir),
                    *selector,
                ]
            ),
        },
        {
            "purpose": "Run the full accepted diagnostic e2e gate after edits",
            "argv": [
                "python",
                "scripts/run_diagnostic_e2e.py",
                "--suite-dir",
                "target/diagnostics/scenario-suite",
            ],
            "text": "python scripts/run_diagnostic_e2e.py --suite-dir target/diagnostics/scenario-suite",
        },
    ]


def fix_loop(selection: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        {
            "order": 1,
            "action": "confirm_failure_signature",
            "purpose": "Verify the diagnosis replay still reports the expected health, focus domain, and failed probe before editing.",
        },
        {
            "order": 2,
            "action": "inspect_source_matches",
            "purpose": "Open the source_scan rows with the highest matched_line_count and inspect the listed line anchors.",
        },
        {
            "order": 3,
            "action": "make_minimal_emulator_fix",
            "purpose": f"Keep changes scoped to the selected focus domain {selection.get('focus_domain')}.",
        },
        {
            "order": 4,
            "action": "run_narrow_tests",
            "purpose": "Run fix_commands.narrow_test_commands before spending time on the full diagnostic suite.",
        },
        {
            "order": 5,
            "action": "rerun_diagnosis_and_e2e",
            "purpose": "Regenerate the diagnosis and then run the full diagnostic e2e gate to refresh AI-facing artifacts.",
        },
    ]


def build_summary(
    args: argparse.Namespace,
    repo_root: Path,
    diagnosis_json: Path,
    summary_json: Path,
    summary_report: Path,
) -> dict[str, Any]:
    diagnosis = load_json(diagnosis_json)
    e2e_report = load_json(args.suite_dir / "diagnostic-e2e-report.json")
    selection = as_dict(diagnosis.get("selection"))
    evidence = as_dict(diagnosis.get("evidence"))
    diagnosis_artifacts = as_dict(diagnosis.get("artifacts"))
    original_suite_dir = diagnosis.get("suite_dir")
    artifacts = {
        **output_artifacts(summary_json, summary_report),
        "diagnosis_json": str(diagnosis_json),
        "diagnostic_e2e_report_json": str(args.suite_dir / "diagnostic-e2e-report.json"),
        "route_check_json": localize_suite_artifact_path(
            args.suite_dir,
            original_suite_dir,
            diagnosis_artifacts.get("route_check_json", ""),
        ),
        "replay_bundle_triage_json": localize_suite_artifact_path(
            args.suite_dir,
            original_suite_dir,
            diagnosis_artifacts.get("replay_bundle_triage_json", ""),
        ),
        "replay_bundle_telemetry_json": localize_suite_artifact_path(
            args.suite_dir,
            original_suite_dir,
            diagnosis_artifacts.get("replay_bundle_telemetry_json", ""),
        ),
    }
    source_files = as_str_list(evidence.get("source_files"))
    test_files = test_context_files(as_str_list(evidence.get("test_files")))
    search_terms = as_str_list(evidence.get("search_terms"))
    source_scan = scan_paths(repo_root, source_files, search_terms, args.max_matches_per_file)
    test_scan = scan_paths(
        repo_root,
        test_files,
        test_search_terms(selection, evidence),
        args.max_matches_per_file,
    )
    output_presence = artifact_presence(output_artifacts(summary_json, summary_report))
    output_presence["diagnostic_ai_fix_handoff_json"] = True
    output_presence["diagnostic_ai_fix_handoff_report"] = True
    presence = {
        **artifact_presence(artifacts),
        **output_presence,
    }

    narrow_test_commands = narrow_commands(evidence)
    replay_command_rows = replay_commands(evidence)
    route_check = as_dict(diagnosis.get("route_check"))
    errors: list[str] = []
    warnings: list[str] = []
    if not diagnosis:
        errors.append(f"missing or invalid diagnosis JSON: {diagnosis_json}")
    elif diagnosis.get("status") != "passed":
        errors.append(f"diagnosis status is {diagnosis.get('status')!r}, expected 'passed'")
    if e2e_report and e2e_report.get("status") != "passed":
        warnings.append("diagnostic e2e report status is not passed")
    if not source_files:
        errors.append("diagnosis evidence is missing source_files")
    if not test_files:
        errors.append("diagnosis evidence is missing test_files")
    for path in missing_paths(source_scan):
        errors.append(f"missing source file: {path}")
    for path in missing_paths(test_scan):
        errors.append(f"missing test file: {path}")
    if total_matches(source_scan) < 1:
        errors.append("source scan did not find any search-term matches")
    if total_matches(test_scan) < 1:
        errors.append("test scan did not find any search-term matches")
    if not narrow_test_commands:
        errors.append("diagnosis evidence is missing narrow test commands")
    if route_check.get("replay_status") != "passed":
        errors.append("diagnosis replay status is not passed")
    if route_check.get("tests_status") != "passed":
        errors.append("diagnosis narrow tests status is not passed")
    for artifact_name in (
        "diagnosis_json",
        "route_check_json",
        "replay_bundle_triage_json",
        "replay_bundle_telemetry_json",
    ):
        if not presence.get(artifact_name):
            errors.append(f"missing handoff artifact: {artifact_name}")

    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_fix_handoff_schema_version": FIX_HANDOFF_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(args.suite_dir),
        "diagnosis_json": str(diagnosis_json),
        "selection": {
            "method": selection.get("method"),
            "route_id": selection.get("route_id"),
            "scenario_id": selection.get("scenario_id"),
            "focus_domain": selection.get("focus_domain"),
            "probe_id": selection.get("probe_id"),
        },
        "failure_signature": {
            "expected_health": route_check.get("expected_health"),
            "actual_health": route_check.get("actual_health"),
            "expected_focus_domain": route_check.get("expected_focus_domain"),
            "actual_focus_domain": route_check.get("actual_focus_domain"),
            "failed_probe_ids": as_str_list(evidence.get("failed_probe_ids")),
            "debug_anchor": as_dict(evidence.get("debug_anchor")),
        },
        "source_scan": {
            "search_terms": search_terms,
            "source_file_count": len(source_scan),
            "source_match_count": total_matches(source_scan),
            "files": source_scan,
        },
        "test_scan": {
            "search_terms": test_search_terms(selection, evidence),
            "test_file_count": len(test_scan),
            "test_match_count": total_matches(test_scan),
            "files": test_scan,
        },
        "fix_commands": {
            "replay_commands": replay_command_rows,
            "narrow_test_commands": narrow_test_commands,
            "verification_commands": verification_commands(args, diagnosis),
        },
        "artifacts": artifacts,
        "artifact_presence": presence,
        "fix_loop": fix_loop(selection),
        "stop_conditions": [
            {
                "name": "diagnosis_passed",
                "passed": diagnosis.get("status") == "passed",
                "detail": diagnosis.get("status"),
            },
            {
                "name": "source_files_resolved",
                "passed": bool(source_scan) and not missing_paths(source_scan),
                "detail": source_files,
            },
            {
                "name": "source_search_matches_found",
                "passed": total_matches(source_scan) > 0,
                "detail": total_matches(source_scan),
            },
            {
                "name": "test_search_matches_found",
                "passed": total_matches(test_scan) > 0,
                "detail": total_matches(test_scan),
            },
            {
                "name": "narrow_test_commands_present",
                "passed": bool(narrow_test_commands),
                "detail": len(narrow_test_commands),
            },
            {
                "name": "replay_and_tests_passed",
                "passed": route_check.get("replay_status") == "passed"
                and route_check.get("tests_status") == "passed",
                "detail": {
                    "replay_status": route_check.get("replay_status"),
                    "tests_status": route_check.get("tests_status"),
                },
            },
        ],
        "errors": errors,
        "warnings": warnings,
        "ai_handoff": [
            "Use this fix handoff after diagnostic-ai-diagnosis.json has passed.",
            "Open source_scan.files[].matches first; they are bounded line anchors for the selected focus domain.",
            "Use fix_commands.narrow_test_commands after an edit before rerunning the full diagnostic e2e gate.",
            "If a stop condition fails, regenerate the diagnosis or fix the diagnostic artifact graph before editing emulator code.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    selection = as_dict(summary.get("selection"))
    source_scan = as_dict(summary.get("source_scan"))
    test_scan = as_dict(summary.get("test_scan"))
    commands = as_dict(summary.get("fix_commands"))
    lines = [
        "# Diagnostic AI Fix Handoff",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Diagnosis JSON | {markdown_cell(summary.get('diagnosis_json'))} |",
        f"| Route | {markdown_cell(selection.get('route_id'))} |",
        f"| Scenario | {markdown_cell(selection.get('scenario_id'))} |",
        f"| Focus domain | {markdown_cell(selection.get('focus_domain'))} |",
        f"| Probe | {markdown_cell(selection.get('probe_id'))} |",
        "",
        "## Source Scan",
        "",
        "| File | Exists | Matches | First anchors |",
        "| --- | --- | ---: | --- |",
    ]
    for row in as_list(source_scan.get("files")):
        if not isinstance(row, dict):
            continue
        anchors = ", ".join(
            f"{match.get('line')}:{'/'.join(as_str_list(match.get('terms')))}"
            for match in as_list(row.get("matches"))[:6]
            if isinstance(match, dict)
        )
        lines.append(
            f"| {markdown_cell(row.get('path'))} | {row.get('exists')} | {row.get('matched_line_count')} | {markdown_cell(anchors)} |"
        )
    lines.extend(
        [
            "",
            "## Test Scan",
            "",
            "| File | Exists | Matches | First anchors |",
            "| --- | --- | ---: | --- |",
        ]
    )
    for row in as_list(test_scan.get("files")):
        if not isinstance(row, dict):
            continue
        anchors = ", ".join(
            f"{match.get('line')}:{'/'.join(as_str_list(match.get('terms')))}"
            for match in as_list(row.get("matches"))[:6]
            if isinstance(match, dict)
        )
        lines.append(
            f"| {markdown_cell(row.get('path'))} | {row.get('exists')} | {row.get('matched_line_count')} | {markdown_cell(anchors)} |"
        )
    lines.extend(["", "## Fix Commands", "", "| Purpose | Command |", "| --- | --- |"])
    for command in as_list(commands.get("narrow_test_commands")):
        if isinstance(command, dict):
            lines.append(
                f"| {markdown_cell(command.get('purpose'))} | `{markdown_cell(command.get('text'))}` |"
            )
    for command in as_list(commands.get("verification_commands")):
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
    lines.extend(["", "## Fix Loop", "", "| Order | Action | Purpose |", "| ---: | --- | --- |"])
    for step in as_list(summary.get("fix_loop")):
        if isinstance(step, dict):
            lines.append(
                f"| {step.get('order')} | {markdown_cell(step.get('action'))} | {markdown_cell(step.get('purpose'))} |"
            )
    lines.extend(["", "## Artifacts", "", "| Name | Present | Path |", "| --- | --- | --- |"])
    presence = as_dict(summary.get("artifact_presence"))
    for name, artifact_path in as_dict(summary.get("artifacts")).items():
        lines.append(f"| {name} | {presence.get(name)} | {markdown_cell(artifact_path)} |")
    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(summary.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if summary.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(summary.get("errors")):
            lines.append(f"- {error}")
    if summary.get("warnings"):
        lines.extend(["", "## Warnings", ""])
        for warning in as_list(summary.get("warnings")):
            lines.append(f"- {warning}")
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
        "--diagnosis-json",
        type=Path,
        help=(
            "Path to diagnostic-ai-diagnosis.json. Defaults to the e2e "
            "diagnostic-ai-diagnosis-smoke.json when present."
        ),
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write the fix handoff JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the fix handoff Markdown report. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--max-matches-per-file",
        type=int,
        default=DEFAULT_MAX_MATCHES_PER_FILE,
        help="Maximum source or test line anchors to retain per file.",
    )
    parser.add_argument("--json", action="store_true", help="Print the fix handoff JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    diagnosis_json = resolve_diagnosis_json(args)
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-fix-handoff.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-fix-handoff.md"
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)
    summary = build_summary(args, repo_root, diagnosis_json, summary_json, summary_report)
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        selection = as_dict(summary.get("selection"))
        source_scan = as_dict(summary.get("source_scan"))
        print(
            "Diagnostic AI fix handoff "
            f"{summary['status']}: route={selection.get('route_id')} "
            f"scenario={selection.get('scenario_id')} "
            f"source_files={source_scan.get('source_file_count')} "
            f"source_matches={source_scan.get('source_match_count')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
