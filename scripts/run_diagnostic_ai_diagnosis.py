#!/usr/bin/env python3
"""Run an AI-facing diagnosis handoff from an accepted OxideNES diagnostic suite."""

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


AI_DIAGNOSIS_SCHEMA_VERSION = 1
OUTPUT_TAIL_LINES = 80


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def as_str_list(value: Any) -> list[str]:
    return [item for item in as_list(value) if isinstance(item, str)]


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


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    return value if isinstance(value, dict) else {}


def generated_at_utc() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


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


def sanitize_path_component(value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9_.-]+", "-", value.strip())
    return cleaned.strip(".-") or "diagnosis"


def script_path(name: str) -> str:
    return str(Path("scripts") / name)


def index_path(args: argparse.Namespace) -> Path:
    return args.index_json or args.suite_dir / "diagnostic-ai-observability-index.json"


def scenario_cards(index: dict[str, Any]) -> list[dict[str, Any]]:
    return [card for card in as_list(index.get("scenario_cards")) if isinstance(card, dict)]


def focus_domains(index: dict[str, Any]) -> list[dict[str, Any]]:
    return [domain for domain in as_list(index.get("focus_domains")) if isinstance(domain, dict)]


def probe_rows(index: dict[str, Any]) -> list[dict[str, Any]]:
    return [row for row in as_list(index.get("probe_index")) if isinstance(row, dict)]


def find_by_key(rows: list[dict[str, Any]], key: str, value: str | None) -> dict[str, Any]:
    if not isinstance(value, str):
        return {}
    for row in rows:
        if row.get(key) == value:
            return row
    return {}


def find_focus_domain_for_route_id(index: dict[str, Any], route_id: str | None) -> dict[str, Any]:
    return find_by_key(focus_domains(index), "route_id", route_id)


def find_focus_domain_for_scenario(index: dict[str, Any], scenario: dict[str, Any]) -> dict[str, Any]:
    route_id = scenario.get("route_id")
    domain = find_focus_domain_for_route_id(index, route_id if isinstance(route_id, str) else None)
    if domain:
        return domain
    return find_by_key(
        focus_domains(index),
        "focus_domain",
        scenario.get("focus_domain") if isinstance(scenario.get("focus_domain"), str) else None,
    )


def top_route_selection(index: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    summary = as_dict(index.get("summary"))
    domain = find_by_key(
        focus_domains(index),
        "focus_domain",
        summary.get("top_route_focus_domain") if isinstance(summary.get("top_route_focus_domain"), str) else None,
    )
    scenario = find_by_key(
        scenario_cards(index),
        "scenario_id",
        summary.get("top_route_scenario") if isinstance(summary.get("top_route_scenario"), str) else None,
    )
    return domain, scenario


def choose_probe_scenario(index: dict[str, Any], probe: dict[str, Any]) -> dict[str, Any]:
    top_domain, top_scenario = top_route_selection(index)
    probe_id = probe.get("probe_id")
    if isinstance(probe_id, str) and probe_id in as_str_list(top_domain.get("failed_probe_ids")):
        return top_scenario
    for scenario_id in as_str_list(probe.get("scenario_ids")):
        scenario = find_by_key(scenario_cards(index), "scenario_id", scenario_id)
        if scenario:
            return scenario
    return {}


def first_probe_for_domain_or_scenario(
    index: dict[str, Any],
    domain: dict[str, Any],
    scenario: dict[str, Any],
) -> dict[str, Any]:
    focus_domain = domain.get("focus_domain") or scenario.get("focus_domain")
    candidate_probe_ids = [
        *as_str_list(domain.get("failed_probe_ids")),
        *as_str_list(scenario.get("failed_probe_ids")),
    ]
    if "ppu.nmi_count" in candidate_probe_ids:
        candidate_probe_ids.insert(0, "ppu.nmi_count")
    for probe_id in candidate_probe_ids:
        probe = find_by_key(probe_rows(index), "probe_id", probe_id)
        if probe and probe.get("likely_domain") == focus_domain:
            return probe
    for probe_id in candidate_probe_ids:
        probe = find_by_key(probe_rows(index), "probe_id", probe_id)
        if probe:
            return probe
    return {}


def selected_route_id(domain: dict[str, Any], scenario: dict[str, Any]) -> str:
    route_id = domain.get("route_id") or scenario.get("route_id")
    return str(route_id or "")


def build_selection(args: argparse.Namespace, index: dict[str, Any]) -> dict[str, Any]:
    errors: list[str] = []
    requested = {
        "route_id": args.route_id,
        "scenario_id": args.scenario_id,
        "focus_domain": args.focus_domain,
        "probe_id": args.probe_id,
    }
    method = "top-route"
    domain: dict[str, Any] = {}
    scenario: dict[str, Any] = {}
    probe: dict[str, Any] = {}

    if args.route_id:
        method = "route-id"
        domain = find_by_key(focus_domains(index), "route_id", args.route_id)
        if not domain:
            errors.append(f"route not found in AI index: {args.route_id}")
        scenario = find_by_key(
            scenario_cards(index),
            "scenario_id",
            domain.get("primary_scenario_id") if isinstance(domain.get("primary_scenario_id"), str) else None,
        )
    elif args.scenario_id:
        method = "scenario-id"
        scenario = find_by_key(scenario_cards(index), "scenario_id", args.scenario_id)
        if not scenario:
            errors.append(f"scenario not found in AI index: {args.scenario_id}")
        domain = find_focus_domain_for_scenario(index, scenario)
    elif args.focus_domain:
        method = "focus-domain"
        domain = find_by_key(focus_domains(index), "focus_domain", args.focus_domain)
        if not domain:
            errors.append(f"focus domain not found in AI index: {args.focus_domain}")
        scenario = find_by_key(
            scenario_cards(index),
            "scenario_id",
            domain.get("primary_scenario_id") if isinstance(domain.get("primary_scenario_id"), str) else None,
        )
    elif args.probe_id:
        method = "probe-id"
        probe = find_by_key(probe_rows(index), "probe_id", args.probe_id)
        if not probe:
            errors.append(f"probe not found in AI index: {args.probe_id}")
        scenario = choose_probe_scenario(index, probe)
        domain = find_focus_domain_for_scenario(index, scenario)
    else:
        domain, scenario = top_route_selection(index)

    if not domain and scenario:
        domain = find_focus_domain_for_scenario(index, scenario)
    if not scenario and domain:
        scenario = find_by_key(
            scenario_cards(index),
            "scenario_id",
            domain.get("primary_scenario_id") if isinstance(domain.get("primary_scenario_id"), str) else None,
        )
    if not probe:
        probe = first_probe_for_domain_or_scenario(index, domain, scenario)

    route_id = selected_route_id(domain, scenario)
    if not route_id:
        errors.append("selected route is missing a route_id")
    if not scenario:
        errors.append("selected route is missing a scenario card")
    if not domain:
        errors.append("selected route is missing a focus-domain row")

    return {
        "method": method,
        "requested": requested,
        "route_id": route_id,
        "focus_domain": domain.get("focus_domain") or scenario.get("focus_domain"),
        "scenario_id": scenario.get("scenario_id") or domain.get("primary_scenario_id"),
        "probe_id": probe.get("probe_id"),
        "domain": domain,
        "scenario": scenario,
        "probe": probe,
        "errors": errors,
    }


def output_dir_for_selection(args: argparse.Namespace, selection: dict[str, Any]) -> Path:
    if args.output_dir:
        return args.output_dir
    label = (
        str(selection.get("route_id") or "")
        or str(selection.get("scenario_id") or "")
        or str(selection.get("focus_domain") or "")
        or "top-route"
    )
    return args.suite_dir / "ai-diagnosis" / sanitize_path_component(label)


def route_selector_args(selection: dict[str, Any]) -> list[str]:
    route_id = selection.get("route_id")
    scenario_id = selection.get("scenario_id")
    focus_domain = selection.get("focus_domain")
    if isinstance(route_id, str) and route_id:
        return ["--route-id", route_id]
    if isinstance(scenario_id, str) and scenario_id:
        return ["--scenario-id", scenario_id]
    if isinstance(focus_domain, str) and focus_domain:
        return ["--focus-domain", focus_domain]
    return []


def run_route_check(
    args: argparse.Namespace,
    repo_root: Path,
    selection: dict[str, Any],
    output_dir: Path,
) -> dict[str, Any]:
    route_dir = output_dir / "route-check"
    argv = [
        sys.executable,
        script_path("run_diagnostic_route.py"),
        "--suite-dir",
        str(args.suite_dir),
        "--output-dir",
        str(route_dir),
        "--cargo",
        args.cargo,
        *route_selector_args(selection),
    ]
    if args.skip_tests:
        argv.append("--skip-tests")
    return run_command("run_diagnostic_route", argv, repo_root)


def route_artifact_paths(output_dir: Path, route_check: dict[str, Any]) -> dict[str, str]:
    route_artifacts = as_dict(route_check.get("artifacts"))
    route_dir = output_dir / "route-check"
    replay_bundle_dir = route_dir / "replay-bundle"
    return {
        "diagnosis_output_dir": str(output_dir),
        "route_check_json": str(route_artifacts.get("route_check_json") or route_dir / "diagnostic-route-check.json"),
        "route_check_report": str(
            route_artifacts.get("route_check_report") or route_dir / "diagnostic-route-check.md"
        ),
        "replay_bundle_dir": str(route_artifacts.get("replay_bundle_dir") or replay_bundle_dir),
        "replay_bundle_manifest": str(
            route_artifacts.get("replay_bundle_manifest") or replay_bundle_dir / "manifest.json"
        ),
        "replay_bundle_triage_json": str(
            route_artifacts.get("replay_bundle_triage_json") or replay_bundle_dir / "triage.json"
        ),
        "replay_bundle_telemetry_json": str(
            route_artifacts.get("replay_bundle_telemetry_json") or replay_bundle_dir / "telemetry.json"
        ),
        "replay_bundle_report": str(
            route_artifacts.get("replay_bundle_report") or replay_bundle_dir / "report.md"
        ),
        "replay_bundle_rom": str(
            route_artifacts.get("replay_bundle_rom") or replay_bundle_dir / "diagnostic.nes"
        ),
    }


def artifact_presence(artifacts: dict[str, str]) -> dict[str, bool]:
    result: dict[str, bool] = {}
    for name, value in artifacts.items():
        path = Path(value)
        result[name] = path.is_dir() if name.endswith("_dir") else path.is_file()
    return result


def compact_command_records(commands: Any) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for command in as_list(commands):
        if not isinstance(command, dict):
            continue
        argv = [str(item) for item in as_list(command.get("argv"))]
        records.append(
            {
                "purpose": command.get("purpose"),
                "argv": argv,
                "text": " ".join(argv),
            }
        )
    return records


def evidence_summary(index: dict[str, Any], selection: dict[str, Any]) -> dict[str, Any]:
    domain = as_dict(selection.get("domain"))
    scenario = as_dict(selection.get("scenario"))
    start_artifacts = as_dict(scenario.get("start_artifacts"))
    coverage = as_dict(index.get("coverage_limits"))
    return {
        "primary_artifact": start_artifacts.get("primary_artifact") or domain.get("primary_artifact"),
        "triage_json": start_artifacts.get("triage_json"),
        "telemetry_json": start_artifacts.get("telemetry_json"),
        "comparison_json": start_artifacts.get("comparison_json"),
        "report_md": start_artifacts.get("report_md"),
        "debug_anchor": as_dict(domain.get("debug_anchor")) or as_dict(scenario.get("debug_anchor")),
        "failed_probe_ids": sorted(
            set(as_str_list(domain.get("failed_probe_ids")) + as_str_list(scenario.get("failed_probe_ids")))
        ),
        "source_files": as_str_list(domain.get("source_files")) or as_str_list(scenario.get("source_files")),
        "test_files": as_str_list(domain.get("test_files")) or as_str_list(scenario.get("test_files")),
        "diagnostic_files": as_str_list(domain.get("diagnostic_files")),
        "search_terms": as_str_list(domain.get("search_terms")) or as_str_list(scenario.get("search_terms")),
        "replay_args": as_list(domain.get("replay_args")) or as_list(scenario.get("replay_args")),
        "suggested_commands": compact_command_records(domain.get("suggested_commands"))
        or compact_command_records(scenario.get("suggested_commands")),
        "coverage_limits": {
            "only_happy_paths": coverage.get("only_happy_paths"),
            "known_gap_count": coverage.get("known_gap_count"),
            "coverage_gap_count": len(as_list(coverage.get("coverage_gaps"))),
        },
    }


def next_actions(selection: dict[str, Any], evidence: dict[str, Any], route_check: dict[str, Any]) -> list[dict[str, Any]]:
    artifacts = as_dict(route_check.get("artifacts"))
    source_files = as_str_list(evidence.get("source_files"))
    test_files = as_str_list(evidence.get("test_files"))
    commands = as_list(evidence.get("suggested_commands"))
    return [
        {
            "order": 1,
            "action": "open_route_replay_triage",
            "purpose": "Start from the just-regenerated focused replay result before loading full telemetry.",
            "artifact": artifacts.get("replay_bundle_triage_json"),
        },
        {
            "order": 2,
            "action": "compare_index_primary_artifact",
            "purpose": "Compare the accepted scenario artifact with the focused replay to confirm the same failure is still reproduced.",
            "artifact": evidence.get("primary_artifact"),
        },
        {
            "order": 3,
            "action": "inspect_mapped_source",
            "purpose": "Open the code files mapped to the selected focus domain and search for the indexed terms.",
            "paths": source_files,
            "search_terms": as_str_list(evidence.get("search_terms")),
        },
        {
            "order": 4,
            "action": "run_mapped_tests_after_edit",
            "purpose": "Use the narrow route tests first after an emulator change, then rerun the full diagnostic e2e gate.",
            "paths": test_files,
            "commands": commands,
        },
    ]


def stop_conditions(route_command: dict[str, Any], route_check: dict[str, Any]) -> list[dict[str, Any]]:
    replay = as_dict(route_check.get("replay"))
    tests = as_dict(route_check.get("tests"))
    return [
        {
            "name": "route_command_passed",
            "passed": command_passed(route_command),
            "detail": route_command.get("exit_code"),
        },
        {
            "name": "replay_matches_expected_exit_health_and_focus",
            "passed": replay.get("status") == "passed",
            "detail": {
                "expected_health": replay.get("expected_health"),
                "actual_health": replay.get("actual_health"),
                "expected_focus_domain": replay.get("expected_focus_domain"),
                "actual_focus_domain": replay.get("actual_focus_domain"),
            },
        },
        {
            "name": "narrow_tests_passed",
            "passed": tests.get("status") == "passed",
            "detail": {
                "skipped": tests.get("skipped"),
                "command_count": tests.get("command_count"),
            },
        },
    ]


def summary_artifacts(
    args: argparse.Namespace,
    output_dir: Path,
    route_check: dict[str, Any],
    summary_json: Path,
    summary_report: Path,
) -> dict[str, str]:
    return {
        "diagnostic_ai_diagnosis_json": str(summary_json),
        "diagnostic_ai_diagnosis_report": str(summary_report),
        **route_artifact_paths(output_dir, route_check),
    }


def build_summary(
    args: argparse.Namespace,
    repo_root: Path,
    index_json: Path,
    index: dict[str, Any],
    selection: dict[str, Any],
    output_dir: Path,
    summary_json: Path,
    summary_report: Path,
    route_command: dict[str, Any],
) -> dict[str, Any]:
    route_check_path = output_dir / "route-check" / "diagnostic-route-check.json"
    route_check = load_json(route_check_path)
    artifacts = summary_artifacts(args, output_dir, route_check, summary_json, summary_report)
    presence = artifact_presence(artifacts)
    presence["diagnostic_ai_diagnosis_json"] = True
    presence["diagnostic_ai_diagnosis_report"] = True
    evidence = evidence_summary(index, selection)

    errors = []
    if not index:
        errors.append(f"missing or invalid AI index: {index_json}")
    elif index.get("status") != "passed":
        errors.append(f"AI index status is {index.get('status')!r}, expected 'passed'")
    errors.extend(as_str_list(selection.get("errors")))
    if not command_passed(route_command):
        errors.append("run_diagnostic_route command failed")
    if not route_check:
        errors.append(f"missing route-check summary: {route_check_path}")
    elif route_check.get("status") != "passed":
        errors.append("route-check status is not passed")
    if not evidence.get("primary_artifact"):
        errors.append("selected scenario is missing a primary artifact")
    if not evidence.get("source_files"):
        errors.append("selected route is missing source files")
    if not evidence.get("test_files"):
        errors.append("selected route is missing test files")
    missing_artifacts = [
        name
        for name in (
            "route_check_json",
            "route_check_report",
            "replay_bundle_manifest",
            "replay_bundle_triage_json",
            "replay_bundle_telemetry_json",
            "replay_bundle_report",
            "replay_bundle_rom",
        )
        if not presence.get(name)
    ]
    if missing_artifacts:
        errors.append(f"missing diagnosis artifacts: {', '.join(missing_artifacts)}")

    status = "passed" if not errors else "failed"
    route_check_replay = as_dict(route_check.get("replay"))
    route_check_tests = as_dict(route_check.get("tests"))
    return {
        "diagnostic_ai_diagnosis_schema_version": AI_DIAGNOSIS_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "git": git_metadata(repo_root),
        "suite_dir": str(args.suite_dir),
        "index_json": str(index_json),
        "selection": {
            "method": selection.get("method"),
            "requested": as_dict(selection.get("requested")),
            "route_id": selection.get("route_id"),
            "focus_domain": selection.get("focus_domain"),
            "scenario_id": selection.get("scenario_id"),
            "probe_id": selection.get("probe_id"),
        },
        "evidence": evidence,
        "selected_focus_domain": as_dict(selection.get("domain")),
        "selected_scenario": as_dict(selection.get("scenario")),
        "selected_probe": as_dict(selection.get("probe")),
        "route_execution": route_command,
        "route_check": {
            "status": route_check.get("status"),
            "replay_status": route_check_replay.get("status"),
            "expected_health": route_check_replay.get("expected_health"),
            "actual_health": route_check_replay.get("actual_health"),
            "expected_focus_domain": route_check_replay.get("expected_focus_domain"),
            "actual_focus_domain": route_check_replay.get("actual_focus_domain"),
            "tests_status": route_check_tests.get("status"),
            "tests_skipped": route_check_tests.get("skipped"),
            "test_command_count": route_check_tests.get("command_count"),
            "errors": as_list(route_check.get("errors")),
        },
        "artifacts": artifacts,
        "artifact_presence": presence,
        "next_actions": next_actions(selection, evidence, route_check),
        "stop_conditions": stop_conditions(route_command, route_check),
        "errors": errors,
        "ai_handoff": [
            "Use this diagnosis report after diagnostic-e2e-report.json and diagnostic-ai-observability-index.json are accepted.",
            "The selected route has already been replayed into replay_bundle_dir; start from replay_bundle_triage_json.",
            "Treat stop_conditions as the gate before editing emulator code; if one failed, fix the diagnostic corpus or route first.",
            "After a code edit, run the route's mapped narrow tests and then scripts/run_diagnostic_e2e.py before claiming the diagnosis is closed.",
        ],
    }


def markdown_cell(value: Any) -> str:
    if value is None:
        return "-"
    return str(value).replace("|", r"\|").replace("\n", " ")


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    selection = as_dict(summary.get("selection"))
    route_check = as_dict(summary.get("route_check"))
    evidence = as_dict(summary.get("evidence"))
    lines = [
        "# Diagnostic AI Diagnosis",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Git commit | {as_dict(summary.get('git')).get('short_commit', '')} |",
        f"| Suite dir | {summary.get('suite_dir')} |",
        f"| Selection method | {selection.get('method')} |",
        f"| Route | {selection.get('route_id')} |",
        f"| Focus domain | {selection.get('focus_domain')} |",
        f"| Scenario | {selection.get('scenario_id')} |",
        f"| Probe | {selection.get('probe_id')} |",
        "",
        "## Route Check",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {route_check.get('status')} |",
        f"| Replay status | {route_check.get('replay_status')} |",
        f"| Expected health | {route_check.get('expected_health')} |",
        f"| Actual health | {route_check.get('actual_health')} |",
        f"| Expected focus domain | {route_check.get('expected_focus_domain')} |",
        f"| Actual focus domain | {route_check.get('actual_focus_domain')} |",
        f"| Tests status | {route_check.get('tests_status')} |",
        f"| Tests skipped | {route_check.get('tests_skipped')} |",
        f"| Test commands | {route_check.get('test_command_count')} |",
        "",
        "## Evidence",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Primary artifact | {markdown_cell(evidence.get('primary_artifact'))} |",
        f"| Replay triage | {markdown_cell(as_dict(summary.get('artifacts')).get('replay_bundle_triage_json'))} |",
        f"| Failed probes | {markdown_cell(', '.join(as_str_list(evidence.get('failed_probe_ids'))))} |",
        f"| Source files | {markdown_cell(', '.join(as_str_list(evidence.get('source_files'))))} |",
        f"| Test files | {markdown_cell(', '.join(as_str_list(evidence.get('test_files'))))} |",
        f"| Search terms | {markdown_cell(', '.join(as_str_list(evidence.get('search_terms'))))} |",
        "",
        "## Stop Conditions",
        "",
        "| Name | Passed | Detail |",
        "| --- | --- | --- |",
    ]
    for condition in as_list(summary.get("stop_conditions")):
        if not isinstance(condition, dict):
            continue
        lines.append(
            f"| {markdown_cell(condition.get('name'))} | {condition.get('passed')} | {markdown_cell(condition.get('detail'))} |"
        )
    lines.extend(["", "## Next Actions", "", "| Order | Action | Purpose |", "| ---: | --- | --- |"])
    for action in as_list(summary.get("next_actions")):
        if not isinstance(action, dict):
            continue
        lines.append(
            f"| {action.get('order')} | {markdown_cell(action.get('action'))} | {markdown_cell(action.get('purpose'))} |"
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
    selectors = parser.add_mutually_exclusive_group()
    selectors.add_argument("--route-id", help="Investigation route id to diagnose.")
    selectors.add_argument("--scenario-id", help="Scenario id to diagnose.")
    selectors.add_argument("--focus-domain", help="Focus-domain route to diagnose.")
    selectors.add_argument("--probe-id", help="Failed probe id to diagnose.")
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="Directory for diagnosis artifacts. Defaults to <suite-dir>/ai-diagnosis/<route-id>.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write the diagnosis JSON. Defaults inside --output-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the diagnosis Markdown report. Defaults inside --output-dir.",
    )
    parser.add_argument("--cargo", default="cargo", help="Cargo executable to use.")
    parser.add_argument("--skip-tests", action="store_true", help="Only run the selected route replay.")
    parser.add_argument("--json", action="store_true", help="Print the diagnosis JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    index_json = index_path(args)
    index = load_json(index_json)
    selection = build_selection(args, index)
    output_dir = output_dir_for_selection(args, selection)
    output_dir.mkdir(parents=True, exist_ok=True)
    summary_json = args.summary_json or output_dir / "diagnostic-ai-diagnosis.json"
    summary_report = args.summary_report or output_dir / "diagnostic-ai-diagnosis.md"
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)

    route_command = run_route_check(args, repo_root, selection, output_dir)
    summary = build_summary(
        args,
        repo_root,
        index_json,
        index,
        selection,
        output_dir,
        summary_json,
        summary_report,
        route_command,
    )
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        route_check = as_dict(summary.get("route_check"))
        selection_summary = as_dict(summary.get("selection"))
        print(
            "Diagnostic AI diagnosis "
            f"{summary['status']}: route={selection_summary.get('route_id')} "
            f"scenario={selection_summary.get('scenario_id')} "
            f"probe={selection_summary.get('probe_id')} "
            f"replay={route_check.get('replay_status')} "
            f"tests={route_check.get('test_command_count')}:{route_check.get('tests_status')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
