#!/usr/bin/env python3
"""Run the full OxideNES diagnostic cartridge observability acceptance flow."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


E2E_REPORT_SCHEMA_VERSION = 1
OUTPUT_TAIL_LINES = 80


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


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


def skipped_command(name: str, reason: str) -> dict[str, Any]:
    return {
        "name": name,
        "argv": [],
        "exit_code": None,
        "status": "skipped",
        "duration_seconds": 0,
        "skip_reason": reason,
        "stdout_tail": [],
        "stderr_tail": [],
    }


def command_passed(command: dict[str, Any]) -> bool:
    return command.get("status") == "passed" and command.get("exit_code") == 0


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    return value if isinstance(value, dict) else {}


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


def script_path(name: str) -> str:
    return str(Path("scripts") / name)


def artifact_paths(
    suite_dir: Path,
    summary_json: Path,
    summary_report: Path,
    route_verification: dict[str, Any],
) -> dict[str, str]:
    route_artifacts = as_dict(route_verification.get("artifacts"))
    return {
        "suite_dir": str(suite_dir),
        "diagnostic_e2e_report_json": str(summary_json),
        "diagnostic_e2e_report": str(summary_report),
        "observability_run_json": str(suite_dir / "observability-run.json"),
        "observability_run_report": str(suite_dir / "observability-run.md"),
        "scenario_dossiers_json": str(suite_dir / "diagnostic-scenario-dossiers.json"),
        "scenario_dossiers_report": str(suite_dir / "diagnostic-scenario-dossiers.md"),
        "diagnostic_telemetry_catalog_json": str(suite_dir / "diagnostic-telemetry-catalog.json"),
        "diagnostic_telemetry_catalog_report": str(suite_dir / "diagnostic-telemetry-catalog.md"),
        "investigation_plan_json": str(suite_dir / "diagnostic-investigation-plan.json"),
        "investigation_plan_report": str(suite_dir / "diagnostic-investigation-plan.md"),
        "route_evidence_verification_json": str(
            suite_dir / "diagnostic-route-evidence-verification.json"
        ),
        "route_evidence_verification_report": str(
            suite_dir / "diagnostic-route-evidence-verification.md"
        ),
        "route_matrix_json": str(
            route_artifacts.get("route_matrix_json")
            or suite_dir / "route-replay-matrix" / "diagnostic-route-matrix.json"
        ),
        "route_matrix_report": str(
            route_artifacts.get("route_matrix_report")
            or suite_dir / "route-replay-matrix" / "diagnostic-route-matrix.md"
        ),
        "top_route_check_json": str(route_artifacts.get("top_route_check_json") or ""),
        "top_route_check_report": str(route_artifacts.get("top_route_check_report") or ""),
    }


def existing_artifact_map(artifacts: dict[str, str]) -> dict[str, bool]:
    result: dict[str, bool] = {}
    for name, value in artifacts.items():
        if not value:
            result[name] = False
            continue
        path = Path(value)
        result[name] = path.is_dir() if name.endswith("_dir") else path.is_file()
    return result


def build_summary(
    suite_dir: Path,
    summary_json: Path,
    summary_report: Path,
    commands: list[dict[str, Any]],
    repo_root: Path,
) -> dict[str, Any]:
    observability = load_json(suite_dir / "observability-run.json")
    route_verification = load_json(suite_dir / "diagnostic-route-evidence-verification.json")
    route_matrix_path = suite_dir / "route-replay-matrix" / "diagnostic-route-matrix.json"
    route_matrix = load_json(route_matrix_path)
    top_route_path = Path(str(as_dict(route_verification.get("artifacts")).get("top_route_check_json") or ""))
    top_route_check = load_json(top_route_path) if str(top_route_path) else {}
    scenario_dossiers = load_json(suite_dir / "diagnostic-scenario-dossiers.json")

    artifacts = artifact_paths(suite_dir, summary_json, summary_report, route_verification)
    artifact_presence = existing_artifact_map(artifacts)
    artifact_presence["diagnostic_e2e_report_json"] = True
    artifact_presence["diagnostic_e2e_report"] = True
    errors: list[str] = []
    for command in commands:
        if not command_passed(command):
            errors.append(f"{command.get('name')} command {command.get('status')}")
    if observability.get("status") != "passed":
        errors.append("observability run status is not passed")
    if route_verification.get("status") != "passed":
        errors.append("route evidence verification status is not passed")
    if route_matrix.get("status") != "passed":
        errors.append("route matrix status is not passed")
    if scenario_dossiers.get("status") != "passed":
        errors.append("scenario dossiers status is not passed")
    missing = [name for name, present in artifact_presence.items() if not present]
    if missing:
        errors.append(f"missing artifacts: {', '.join(missing)}")

    status = "passed" if not errors else "failed"
    top_route = as_dict(as_dict(observability.get("investigation_plan")).get("top_route"))
    return {
        "diagnostic_e2e_report_schema_version": E2E_REPORT_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "git": git_metadata(repo_root),
        "suite_dir": str(suite_dir),
        "commands": commands,
        "observability": {
            "status": observability.get("status"),
            "scenario_count": as_dict(observability.get("suite")).get("scenario_count"),
            "debug_index_entries": as_dict(observability.get("debug_index")).get("entry_count"),
            "hypothesis_count": as_dict(observability.get("analysis")).get("hypothesis_count"),
            "telemetry_catalog_probes": as_dict(observability.get("telemetry_catalog")).get(
                "probe_count"
            ),
            "scenario_dossiers": as_dict(observability.get("scenario_dossiers")).get(
                "dossier_count"
            ),
            "actionable_dossiers": as_dict(observability.get("scenario_dossiers")).get(
                "actionable_dossier_count"
            ),
        },
        "routes": {
            "verification_status": route_verification.get("status"),
            "route_count": route_verification.get("route_count"),
            "matrix_route_count": route_verification.get("matrix_route_count"),
            "matrix_passed_route_count": route_verification.get("matrix_passed_route_count"),
            "top_route_verified": route_verification.get("top_route_verified"),
            "top_route_id": route_verification.get("top_route_id"),
            "top_route_scenario": as_dict(top_route_check.get("selection")).get("primary_scenario_id"),
        },
        "top_route": {
            "route_id": top_route.get("route_id"),
            "focus_domain": top_route.get("focus_domain"),
            "primary_scenario_id": top_route.get("primary_scenario_id"),
            "primary_artifact": top_route.get("primary_artifact"),
            "replay_args": as_list(top_route.get("replay_args")),
        },
        "artifacts": artifacts,
        "artifact_presence": artifact_presence,
        "errors": errors,
        "ai_handoff": [
            "Read this e2e report first to decide whether the diagnostic corpus is accepted.",
            "If status is failed, inspect errors and the failed command tails before opening telemetry.",
            "If status is passed, use top_route for the highest-signal failure and scenario_dossiers_json for scenario-id-first debugging.",
            "Use route_evidence_verification_json to prove the investigation routes can regenerate focused replay evidence.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    observability = as_dict(summary.get("observability"))
    routes = as_dict(summary.get("routes"))
    top_route = as_dict(summary.get("top_route"))
    lines = [
        "# Diagnostic E2E Report",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Git commit | {as_dict(summary.get('git')).get('short_commit', '')} |",
        f"| Suite dir | {summary.get('suite_dir')} |",
        "",
        "## Observability",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {observability.get('status')} |",
        f"| Scenarios | {observability.get('scenario_count')} |",
        f"| Debug-index entries | {observability.get('debug_index_entries')} |",
        f"| Hypotheses | {observability.get('hypothesis_count')} |",
        f"| Telemetry probes | {observability.get('telemetry_catalog_probes')} |",
        f"| Scenario dossiers | {observability.get('scenario_dossiers')} |",
        f"| Actionable dossiers | {observability.get('actionable_dossiers')} |",
        "",
        "## Routes",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Verification status | {routes.get('verification_status')} |",
        f"| Route count | {routes.get('route_count')} |",
        f"| Matrix routes | {routes.get('matrix_route_count')} |",
        f"| Matrix passed routes | {routes.get('matrix_passed_route_count')} |",
        f"| Top route verified | {routes.get('top_route_verified')} |",
        f"| Top route id | {routes.get('top_route_id')} |",
        f"| Top route scenario | {routes.get('top_route_scenario')} |",
        "",
        "## Top Route",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Route id | {top_route.get('route_id')} |",
        f"| Focus domain | {top_route.get('focus_domain')} |",
        f"| Scenario | {top_route.get('primary_scenario_id')} |",
        f"| Primary artifact | {top_route.get('primary_artifact')} |",
        "",
        "## Commands",
        "",
        "| Name | Status | Exit code | Duration seconds |",
        "| --- | --- | --- | ---: |",
    ]
    for command in as_list(summary.get("commands")):
        if not isinstance(command, dict):
            continue
        lines.append(
            f"| {command.get('name')} | {command.get('status')} | {command.get('exit_code')} | {command.get('duration_seconds')} |"
        )
    lines.extend(
        [
            "",
            "## Artifacts",
            "",
            "| Name | Present | Path |",
            "| --- | --- | --- |",
        ]
    )
    presence = as_dict(summary.get("artifact_presence"))
    for name, artifact_path in as_dict(summary.get("artifacts")).items():
        lines.append(f"| {name} | {presence.get(name)} | {artifact_path} |")
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
        type=Path,
        default=Path("target/diagnostics/e2e-scenario-suite"),
        help="Directory to write the generated diagnostic scenario suite.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write the diagnostic e2e JSON report. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the diagnostic e2e Markdown report. Defaults inside --suite-dir.",
    )
    parser.add_argument("--cargo", default="cargo", help="Cargo executable to use.")
    parser.add_argument(
        "--compare-suite-dir",
        type=Path,
        help="Optional prior diagnostic observability suite to compare against.",
    )
    parser.add_argument(
        "--fail-on-comparison-regression",
        action="store_true",
        help="Fail when --compare-suite-dir finds regressions.",
    )
    parser.add_argument("--json", action="store_true", help="Print the e2e report JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    suite_dir = args.suite_dir
    summary_json = args.summary_json or suite_dir / "diagnostic-e2e-report.json"
    summary_report = args.summary_report or suite_dir / "diagnostic-e2e-report.md"
    suite_dir.mkdir(parents=True, exist_ok=True)

    commands: list[dict[str, Any]] = []
    observability_argv = [
        sys.executable,
        script_path("run_diagnostic_observability.py"),
        "--suite-dir",
        str(suite_dir),
        "--cargo",
        args.cargo,
    ]
    if args.compare_suite_dir:
        observability_argv.extend(["--compare-suite-dir", str(args.compare_suite_dir)])
    if args.fail_on_comparison_regression:
        observability_argv.append("--fail-on-comparison-regression")
    commands.append(run_command("run_diagnostic_observability", observability_argv, repo_root))

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "verify_diagnostic_observability",
                [
                    sys.executable,
                    script_path("verify_diagnostic_observability.py"),
                    "--suite-dir",
                    str(suite_dir),
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command("verify_diagnostic_observability", "observability run failed")
        )

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "run_diagnostic_route_matrix",
                [
                    sys.executable,
                    script_path("run_diagnostic_route.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--all-routes",
                    "--skip-tests",
                    "--output-dir",
                    str(suite_dir / "route-replay-matrix"),
                    "--cargo",
                    args.cargo,
                ],
                repo_root,
            )
        )
    else:
        commands.append(skipped_command("run_diagnostic_route_matrix", "observability verifier failed"))

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "run_diagnostic_top_route",
                [
                    sys.executable,
                    script_path("run_diagnostic_route.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--cargo",
                    args.cargo,
                ],
                repo_root,
            )
        )
    else:
        commands.append(skipped_command("run_diagnostic_top_route", "route matrix failed"))

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "verify_diagnostic_route_evidence",
                [
                    sys.executable,
                    script_path("verify_diagnostic_route.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--require-matrix",
                    "--require-top-route",
                    "--expect-matrix-tests-skipped",
                    "--write-summary",
                ],
                repo_root,
            )
        )
    else:
        commands.append(skipped_command("verify_diagnostic_route_evidence", "top route failed"))

    summary = build_summary(suite_dir, summary_json, summary_report, commands, repo_root)
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        observability = as_dict(summary.get("observability"))
        routes = as_dict(summary.get("routes"))
        print(
            "Diagnostic e2e report "
            f"{summary['status']}: suite={suite_dir} "
            f"summary_json={summary_json} summary_report={summary_report} "
            f"scenarios={observability.get('scenario_count')} "
            f"dossiers={observability.get('scenario_dossiers')}:{observability.get('actionable_dossiers')} "
            f"routes={routes.get('matrix_passed_route_count')}:{routes.get('top_route_verified')}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
