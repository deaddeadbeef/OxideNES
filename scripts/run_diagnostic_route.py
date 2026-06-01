#!/usr/bin/env python3
"""Execute one route from an OxideNES diagnostic investigation plan."""

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


ROUTE_CHECK_SCHEMA_VERSION = 1
ROUTE_MATRIX_SCHEMA_VERSION = 1
INVESTIGATION_PLAN_SCHEMA_VERSION = 1
OUTPUT_TAIL_LINES = 80


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def command_tail(output: str, limit: int = OUTPUT_TAIL_LINES) -> list[str]:
    return output.splitlines()[-limit:]


def run_command(argv: list[str], cwd: Path) -> dict[str, Any]:
    started = time.monotonic()
    completed = subprocess.run(
        argv,
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )
    return {
        "argv": argv,
        "exit_code": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": command_tail(completed.stdout),
        "stderr_tail": command_tail(completed.stderr),
    }


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


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    return value if isinstance(value, dict) else {}


def route_check_paths(output_dir: Path) -> dict[str, str]:
    replay_bundle_dir = output_dir / "replay-bundle"
    return {
        "route_check_json": str(output_dir / "diagnostic-route-check.json"),
        "route_check_report": str(output_dir / "diagnostic-route-check.md"),
        "replay_bundle_dir": str(replay_bundle_dir),
        "replay_bundle_manifest": str(replay_bundle_dir / "manifest.json"),
        "replay_bundle_triage_json": str(replay_bundle_dir / "triage.json"),
        "replay_bundle_telemetry_json": str(replay_bundle_dir / "telemetry.json"),
        "replay_bundle_report": str(replay_bundle_dir / "report.md"),
        "replay_bundle_rom": str(replay_bundle_dir / "diagnostic.nes"),
    }


def route_matrix_paths(output_dir: Path) -> dict[str, str]:
    return {
        "route_matrix_json": str(output_dir / "diagnostic-route-matrix.json"),
        "route_matrix_report": str(output_dir / "diagnostic-route-matrix.md"),
    }


def sanitize_path_component(value: str) -> str:
    cleaned = re.sub(r"[^A-Za-z0-9_.-]+", "-", value.strip())
    return cleaned.strip(".-") or "route"


def command_argv(command: dict[str, Any]) -> list[str]:
    return [str(value) for value in as_list(command.get("argv"))]


def command_text(command: dict[str, Any]) -> str:
    return " ".join(command_argv(command))


def replace_cargo(argv: list[str], cargo: str) -> list[str]:
    args = list(argv)
    if args and args[0] == "cargo":
        args[0] = cargo
    return args


def replay_args_for_route(command: dict[str, Any], cargo: str, replay_bundle_dir: Path) -> tuple[list[str], str | None]:
    args = replace_cargo(command_argv(command), cargo)
    if not args:
        return [], "replay command argv is empty"
    try:
        bundle_flag_index = args.index("--bundle-dir")
    except ValueError:
        return [], "replay command is missing --bundle-dir"
    if bundle_flag_index + 1 >= len(args):
        return [], "replay command --bundle-dir is missing a value"
    args[bundle_flag_index + 1] = str(replay_bundle_dir)
    return args, None


def test_args_for_route(command: dict[str, Any], cargo: str) -> list[str]:
    return replace_cargo(command_argv(command), cargo)


def investigation_routes(plan: dict[str, Any]) -> list[dict[str, Any]]:
    routes = [route for route in as_list(plan.get("routes")) if isinstance(route, dict)]
    return sorted(
        routes,
        key=lambda route: (
            route.get("rank") if isinstance(route.get("rank"), int) else 1_000_000,
            str(route.get("route_id") or ""),
        ),
    )


def select_route(
    plan: dict[str, Any],
    rank: int | None,
    route_id: str | None,
    focus_domain: str | None,
    scenario_id: str | None,
) -> tuple[dict[str, Any], str | None]:
    routes = investigation_routes(plan)
    if not routes:
        return {}, "investigation plan has no routes"
    if route_id:
        for route in routes:
            if route.get("route_id") == route_id:
                return route, None
        return {}, f"route not found for route_id={route_id}"
    if focus_domain:
        for route in routes:
            if route.get("focus_domain") == focus_domain:
                return route, None
        return {}, f"route not found for focus_domain={focus_domain}"
    if scenario_id:
        for route in routes:
            if route.get("primary_scenario_id") == scenario_id or scenario_id in as_list(route.get("scenario_ids")):
                return route, None
        return {}, f"route not found for scenario_id={scenario_id}"
    if rank is not None:
        for route in routes:
            if route.get("rank") == rank:
                return route, None
        return {}, f"route not found for rank={rank}"
    top = as_dict(plan.get("top_route"))
    top_route_id = top.get("route_id")
    if isinstance(top_route_id, str):
        for route in routes:
            if route.get("route_id") == top_route_id:
                return route, None
    return routes[0], None


def scenarios_by_id(suite_dir: Path) -> dict[str, dict[str, Any]]:
    manifest = load_json(suite_dir / "scenario-suite.json")
    return {
        scenario.get("id"): scenario
        for scenario in as_list(manifest.get("scenarios"))
        if isinstance(scenario, dict) and isinstance(scenario.get("id"), str)
    }


def required_replay_artifacts_present(artifacts: dict[str, str]) -> bool:
    required = {
        "replay_bundle_manifest",
        "replay_bundle_triage_json",
        "replay_bundle_telemetry_json",
        "replay_bundle_report",
        "replay_bundle_rom",
    }
    return all(Path(path).is_file() for name, path in artifacts.items() if name in required)


def build_route_check(
    suite_dir: Path,
    plan_json: Path,
    output_dir: Path,
    route: dict[str, Any],
    cargo: str,
    skip_tests: bool,
    repo_root: Path,
) -> dict[str, Any]:
    output_dir.mkdir(parents=True, exist_ok=True)
    artifacts = route_check_paths(output_dir)
    replay_bundle_dir = Path(artifacts["replay_bundle_dir"])
    route_id = str(route.get("route_id") or "")
    scenario_id = str(route.get("primary_scenario_id") or "")
    scenario = scenarios_by_id(suite_dir).get(scenario_id, {})
    errors: list[str] = []
    if not scenario:
        errors.append(f"scenario not found in scenario-suite.json: {scenario_id}")

    suggested_commands = [
        command for command in as_list(route.get("suggested_commands")) if isinstance(command, dict)
    ]
    if not suggested_commands:
        errors.append(f"{route_id}: no suggested commands")
    replay_command_record = suggested_commands[0] if suggested_commands else {}
    replay_argv, replay_arg_error = replay_args_for_route(
        replay_command_record, cargo, replay_bundle_dir
    )
    replay_command: dict[str, Any] = {}
    if replay_arg_error:
        errors.append(replay_arg_error)
    else:
        replay_command = run_command(replay_argv, repo_root)

    bundle_manifest = load_json(Path(artifacts["replay_bundle_manifest"]))
    triage = load_json(Path(artifacts["replay_bundle_triage_json"]))
    debug_focus = as_dict(triage.get("debug_focus"))
    expected_exit_code = scenario.get("expected_runner_exit_code")
    expected_health = scenario.get("expected_health")
    expected_focus_test_id = scenario.get("expected_focus_test_id")
    expected_focus_domain = scenario.get("expected_focus_domain")
    actual_health = triage.get("health")
    actual_focus_test_id = debug_focus.get("focus_test_id")
    actual_focus_domain = debug_focus.get("focus_domain")
    exit_code_matches_expected = replay_command.get("exit_code") == expected_exit_code
    health_matches_expected = actual_health == expected_health
    focus_test_matches_expected = (
        True if expected_focus_test_id is None else actual_focus_test_id == expected_focus_test_id
    )
    focus_domain_matches_expected = (
        True if expected_focus_domain is None else actual_focus_domain == expected_focus_domain
    )
    artifacts_present = required_replay_artifacts_present(artifacts)
    replay_passed = (
        not replay_arg_error
        and exit_code_matches_expected
        and health_matches_expected
        and focus_test_matches_expected
        and focus_domain_matches_expected
        and artifacts_present
    )

    test_commands = []
    if not skip_tests:
        for command in suggested_commands[1:]:
            args = test_args_for_route(command, cargo)
            test_commands.append(
                {
                    "purpose": command.get("purpose"),
                    **run_command(args, repo_root),
                }
            )
    tests_passed = skip_tests or all(command.get("exit_code") == 0 for command in test_commands)
    status = "passed" if not errors and replay_passed and tests_passed else "failed"
    return {
        "diagnostic_route_check_schema_version": ROUTE_CHECK_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "git": git_metadata(repo_root),
        "suite_dir": str(suite_dir),
        "plan_json": str(plan_json),
        "selection": {
            "route_id": route_id,
            "rank": route.get("rank"),
            "focus_domain": route.get("focus_domain"),
            "primary_scenario_id": scenario_id,
        },
        "route": route,
        "artifacts": artifacts,
        "replay": {
            "status": "passed" if replay_passed else "failed",
            "command": replay_command,
            "source_command": replay_command_record,
            "expected_runner_exit_code": expected_exit_code,
            "expected_health": expected_health,
            "expected_focus_test_id": expected_focus_test_id,
            "expected_focus_domain": expected_focus_domain,
            "actual_health": actual_health,
            "actual_focus_test_id": actual_focus_test_id,
            "actual_focus_domain": actual_focus_domain,
            "exit_code_matches_expected": exit_code_matches_expected,
            "health_matches_expected": health_matches_expected,
            "focus_test_matches_expected": focus_test_matches_expected,
            "focus_domain_matches_expected": focus_domain_matches_expected,
            "required_artifacts_present": artifacts_present,
            "bundle_passed": bundle_manifest.get("passed"),
        },
        "tests": {
            "skipped": skip_tests,
            "status": "passed" if tests_passed else "failed",
            "command_count": len(test_commands),
            "commands": test_commands,
        },
        "errors": errors,
        "ai_handoff": [
            "Start with replay_bundle_triage_json for the focused route result.",
            "Use route.source_files and route.search_terms when the replay still matches the expected focus domain.",
            "Use tests.commands to see which narrow regression command failed after an emulator edit.",
        ],
    }


def write_route_check(summary: dict[str, Any]) -> tuple[Path, Path]:
    artifacts = as_dict(summary.get("artifacts"))
    json_path = Path(str(artifacts["route_check_json"]))
    report_path = Path(str(artifacts["route_check_report"]))
    json_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(report_path, summary)
    return json_path, report_path


def build_route_matrix(
    suite_dir: Path,
    plan_json: Path,
    output_dir: Path,
    route_summaries: list[dict[str, Any]],
    repo_root: Path,
    skip_tests: bool,
) -> dict[str, Any]:
    passed_routes = [
        summary for summary in route_summaries if summary.get("status") == "passed"
    ]
    failed_routes = [
        summary for summary in route_summaries if summary.get("status") != "passed"
    ]
    replay_failures = [
        summary
        for summary in route_summaries
        if as_dict(summary.get("replay")).get("status") != "passed"
    ]
    test_failures = [
        summary
        for summary in route_summaries
        if as_dict(summary.get("tests")).get("status") != "passed"
    ]
    route_rows = []
    for summary in route_summaries:
        selection = as_dict(summary.get("selection"))
        replay = as_dict(summary.get("replay"))
        tests = as_dict(summary.get("tests"))
        artifacts = as_dict(summary.get("artifacts"))
        route_rows.append(
            {
                "route_id": selection.get("route_id"),
                "rank": selection.get("rank"),
                "focus_domain": selection.get("focus_domain"),
                "primary_scenario_id": selection.get("primary_scenario_id"),
                "status": summary.get("status"),
                "replay_status": replay.get("status"),
                "actual_health": replay.get("actual_health"),
                "actual_focus_domain": replay.get("actual_focus_domain"),
                "required_artifacts_present": replay.get("required_artifacts_present"),
                "tests_status": tests.get("status"),
                "tests_skipped": tests.get("skipped"),
                "test_command_count": tests.get("command_count"),
                "route_check_json": artifacts.get("route_check_json"),
                "route_check_report": artifacts.get("route_check_report"),
                "replay_bundle_triage_json": artifacts.get("replay_bundle_triage_json"),
            }
        )
    status = "passed" if route_summaries and not failed_routes else "failed"
    return {
        "diagnostic_route_matrix_schema_version": ROUTE_MATRIX_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "git": git_metadata(repo_root),
        "suite_dir": str(suite_dir),
        "plan_json": str(plan_json),
        "artifacts": route_matrix_paths(output_dir),
        "route_count": len(route_summaries),
        "passed_route_count": len(passed_routes),
        "failed_route_count": len(failed_routes),
        "replay_failure_count": len(replay_failures),
        "test_failure_count": len(test_failures),
        "tests_skipped": skip_tests,
        "routes": route_rows,
        "failed_routes": [
            as_dict(summary.get("selection")).get("route_id")
            for summary in failed_routes
        ],
        "ai_handoff": [
            "Start with routes where status is failed; open replay_bundle_triage_json before full telemetry.",
            "Use route_check_json for the exact replay command, expected-vs-actual focus checks, and narrow test results.",
            "A skipped-test matrix proves replay localization for every route; pair it with the top-route check for narrow-test coverage.",
        ],
    }


def markdown_cell(value: str) -> str:
    return value.replace("|", r"\|").replace("\r", " ").replace("\n", " ")


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    replay = as_dict(summary.get("replay"))
    tests = as_dict(summary.get("tests"))
    selection = as_dict(summary.get("selection"))
    lines = [
        "# Diagnostic Route Check",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Git commit | {summary.get('git', {}).get('short_commit', '')} |",
        f"| Route | {selection.get('route_id')} |",
        f"| Focus domain | {selection.get('focus_domain')} |",
        f"| Scenario | {selection.get('primary_scenario_id')} |",
        "",
        "## Replay",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {replay.get('status')} |",
        f"| Expected exit code | {replay.get('expected_runner_exit_code')} |",
        f"| Actual exit code | {as_dict(replay.get('command')).get('exit_code')} |",
        f"| Exit code matches expected | {replay.get('exit_code_matches_expected')} |",
        f"| Expected health | {replay.get('expected_health')} |",
        f"| Actual health | {replay.get('actual_health')} |",
        f"| Expected focus domain | {replay.get('expected_focus_domain')} |",
        f"| Actual focus domain | {replay.get('actual_focus_domain')} |",
        f"| Required artifacts present | {replay.get('required_artifacts_present')} |",
        "",
        "## Narrow Tests",
        "",
        "| Purpose | Exit code | Duration seconds |",
        "| --- | --- | --- |",
    ]
    if tests.get("skipped"):
        lines.append("| skipped | - | - |")
    elif not as_list(tests.get("commands")):
        lines.append("| none | - | - |")
    else:
        for command in as_list(tests.get("commands")):
            if isinstance(command, dict):
                lines.append(
                    "| {} | {} | {} |".format(
                        markdown_cell(str(command.get("purpose") or "")),
                        command.get("exit_code"),
                        command.get("duration_seconds"),
                    )
                )
    lines.extend(["", "## Artifacts", "", "| Name | Path |", "| --- | --- |"])
    for name, artifact_path in as_dict(summary.get("artifacts")).items():
        lines.append(f"| {name} | {artifact_path} |")
    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(summary.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if summary.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(summary.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_matrix_markdown(path: Path, summary: dict[str, Any]) -> None:
    lines = [
        "# Diagnostic Route Matrix",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Git commit | {summary.get('git', {}).get('short_commit', '')} |",
        f"| Routes | {summary.get('route_count')} |",
        f"| Passed routes | {summary.get('passed_route_count')} |",
        f"| Failed routes | {summary.get('failed_route_count')} |",
        f"| Replay failures | {summary.get('replay_failure_count')} |",
        f"| Test failures | {summary.get('test_failure_count')} |",
        f"| Tests skipped | {summary.get('tests_skipped')} |",
        "",
        "## Routes",
        "",
        "| Rank | Route | Focus domain | Scenario | Status | Replay | Health | Tests | Artifacts |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for route in as_list(summary.get("routes")):
        if not isinstance(route, dict):
            continue
        lines.append(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |".format(
                route.get("rank"),
                markdown_cell(str(route.get("route_id") or "")),
                markdown_cell(str(route.get("focus_domain") or "")),
                markdown_cell(str(route.get("primary_scenario_id") or "")),
                route.get("status"),
                route.get("replay_status"),
                route.get("actual_health"),
                "skipped" if route.get("tests_skipped") else route.get("tests_status"),
                markdown_cell(str(route.get("route_check_json") or "")),
            )
        )
    lines.extend(["", "## Artifacts", "", "| Name | Path |", "| --- | --- |"])
    for name, artifact_path in as_dict(summary.get("artifacts")).items():
        lines.append(f"| {name} | {artifact_path} |")
    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(summary.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if summary.get("failed_routes"):
        lines.extend(["", "## Failed Routes", ""])
        for route_id in as_list(summary.get("failed_routes")):
            lines.append(f"- {route_id}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_route_matrix(summary: dict[str, Any]) -> tuple[Path, Path]:
    artifacts = as_dict(summary.get("artifacts"))
    json_path = Path(str(artifacts["route_matrix_json"]))
    report_path = Path(str(artifacts["route_matrix_report"]))
    json_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_matrix_markdown(report_path, summary)
    return json_path, report_path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--suite-dir",
        type=Path,
        default=Path("target/diagnostics/observability-suite"),
        help="Directory produced by scripts/run_diagnostic_observability.py.",
    )
    parser.add_argument(
        "--plan-json",
        type=Path,
        help="Investigation plan JSON path. Defaults to <suite-dir>/diagnostic-investigation-plan.json.",
    )
    parser.add_argument("--rank", type=int, help="Route rank to execute.")
    parser.add_argument("--route-id", help="Route id to execute.")
    parser.add_argument("--focus-domain", help="Focus domain route to execute.")
    parser.add_argument("--scenario-id", help="Scenario route to execute.")
    parser.add_argument(
        "--all-routes",
        action="store_true",
        help="Execute every route from the investigation plan and write a route matrix.",
    )
    parser.add_argument(
        "--max-routes",
        type=int,
        help="Limit --all-routes to the first N ranked routes.",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        help=(
            "Directory for route-check artifacts. Defaults to <suite-dir>/route-checks/<route-id>; "
            "with --all-routes, this is the matrix root and per-route subdirectories are created inside it."
        ),
    )
    parser.add_argument("--cargo", default="cargo", help="Cargo executable to use.")
    parser.add_argument("--skip-tests", action="store_true", help="Only run the route replay.")
    parser.add_argument("--json", action="store_true", help="Print route-check JSON to stdout.")
    return parser.parse_args()


def validate_matrix_args(args: argparse.Namespace) -> str | None:
    if not args.all_routes:
        if args.max_routes is not None:
            return "--max-routes requires --all-routes"
        return None
    selectors = [
        name
        for name, value in (
            ("--rank", args.rank),
            ("--route-id", args.route_id),
            ("--focus-domain", args.focus_domain),
            ("--scenario-id", args.scenario_id),
        )
        if value is not None
    ]
    if selectors:
        return "--all-routes cannot be combined with " + ", ".join(selectors)
    if args.max_routes is not None and args.max_routes < 1:
        return "--max-routes must be at least 1"
    return None


def run_single_route(
    args: argparse.Namespace,
    suite_dir: Path,
    plan_json: Path,
    plan: dict[str, Any],
    repo_root: Path,
) -> int:
    route, route_error = select_route(
        plan, args.rank, args.route_id, args.focus_domain, args.scenario_id
    )
    if route_error:
        print(f"diagnostic route check failed: {route_error}", file=sys.stderr)
        return 2
    route_id = str(route.get("route_id") or "route")
    output_dir = args.output_dir or suite_dir / "route-checks" / sanitize_path_component(route_id)
    summary = build_route_check(
        suite_dir,
        plan_json,
        output_dir,
        route,
        args.cargo,
        args.skip_tests,
        repo_root,
    )
    json_path, report_path = write_route_check(summary)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        tests = as_dict(summary.get("tests"))
        replay = as_dict(summary.get("replay"))
        print(
            "Diagnostic route check "
            f"{summary['status']}: route={route_id} "
            f"scenario={as_dict(summary.get('selection')).get('primary_scenario_id')} "
            f"replay={replay.get('status')} "
            f"tests={tests.get('command_count')}:{tests.get('status')} "
            f"summary_json={json_path} summary_report={report_path}"
        )
    return int(summary["recommended_exit_code"])


def run_route_matrix(
    args: argparse.Namespace,
    suite_dir: Path,
    plan_json: Path,
    plan: dict[str, Any],
    repo_root: Path,
) -> int:
    routes = investigation_routes(plan)
    if not routes:
        print("diagnostic route matrix failed: investigation plan has no routes", file=sys.stderr)
        return 2
    if args.max_routes is not None:
        routes = routes[: args.max_routes]
    output_root = args.output_dir or suite_dir / "route-checks"
    output_root.mkdir(parents=True, exist_ok=True)
    route_summaries = []
    for route in routes:
        route_id = str(route.get("route_id") or "route")
        route_output_dir = output_root / sanitize_path_component(route_id)
        summary = build_route_check(
            suite_dir,
            plan_json,
            route_output_dir,
            route,
            args.cargo,
            args.skip_tests,
            repo_root,
        )
        write_route_check(summary)
        route_summaries.append(summary)
    matrix = build_route_matrix(
        suite_dir,
        plan_json,
        output_root,
        route_summaries,
        repo_root,
        args.skip_tests,
    )
    json_path, report_path = write_route_matrix(matrix)
    if args.json:
        print(json.dumps(matrix, indent=2, sort_keys=True))
    else:
        print(
            "Diagnostic route matrix "
            f"{matrix['status']}: routes={matrix['route_count']} "
            f"passed={matrix['passed_route_count']} failed={matrix['failed_route_count']} "
            f"replay_failures={matrix['replay_failure_count']} "
            f"test_failures={matrix['test_failure_count']} "
            f"tests_skipped={matrix['tests_skipped']} "
            f"summary_json={json_path} summary_report={report_path}"
        )
    return int(matrix["recommended_exit_code"])


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    suite_dir = args.suite_dir
    plan_json = args.plan_json or suite_dir / "diagnostic-investigation-plan.json"
    plan = load_json(plan_json)
    if plan.get("investigation_plan_schema_version") != INVESTIGATION_PLAN_SCHEMA_VERSION:
        print(
            f"diagnostic route check failed: invalid investigation plan schema in {plan_json}",
            file=sys.stderr,
        )
        return 2
    if plan.get("status") != "passed":
        print(
            f"diagnostic route check failed: investigation plan status is {plan.get('status')}",
            file=sys.stderr,
        )
        return 2
    args_error = validate_matrix_args(args)
    if args_error:
        print(f"diagnostic route check failed: {args_error}", file=sys.stderr)
        return 2
    if args.all_routes:
        return run_route_matrix(args, suite_dir, plan_json, plan, repo_root)
    return run_single_route(args, suite_dir, plan_json, plan, repo_root)


if __name__ == "__main__":
    raise SystemExit(main())
