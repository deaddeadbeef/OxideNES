#!/usr/bin/env python3
"""Run and verify the OxideNES diagnostic scenario-suite observability corpus."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


RUN_SCHEMA_VERSION = 1
OUTPUT_TAIL_LINES = 80


def command_tail(output: str, limit: int = OUTPUT_TAIL_LINES) -> list[str]:
    lines = output.splitlines()
    return lines[-limit:]


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
    data = json.loads(path.read_text(encoding="utf-8"))
    return data if isinstance(data, dict) else {}


def artifact_paths(suite_dir: Path, summary_json: Path, summary_md: Path) -> dict[str, str]:
    return {
        "suite_dir": str(suite_dir),
        "scenario_suite_json": str(suite_dir / "scenario-suite.json"),
        "scenario_suite_report": str(suite_dir / "scenario-suite.md"),
        "scenario_suite_observer_json": str(suite_dir / "scenario-suite-observer.json"),
        "scenario_suite_observer_report": str(suite_dir / "scenario-suite-observer.md"),
        "observability_run_json": str(summary_json),
        "observability_run_report": str(summary_md),
    }


def suite_summary(suite_dir: Path) -> dict[str, Any]:
    manifest = load_json(suite_dir / "scenario-suite.json")
    observer = load_json(suite_dir / "scenario-suite-observer.json")
    actions = observer.get("next_actions")
    observations = observer.get("observations")
    first_action = actions[0] if isinstance(actions, list) and actions else None
    return {
        "suite_dir": str(suite_dir),
        "scenario_suite_schema_version": manifest.get("scenario_suite_schema_version"),
        "observer_schema_version": observer.get("observer_schema_version"),
        "suite_name": manifest.get("suite_name"),
        "suite_version": manifest.get("suite_version"),
        "passed": manifest.get("passed"),
        "observer_status": observer.get("status"),
        "summary": observer.get("summary") or manifest.get("analysis", {}).get("summary"),
        "scenario_count": manifest.get("scenario_count"),
        "contract_mismatch_count": observer.get("contract_mismatch_count"),
        "baseline_divergence_count": observer.get("baseline_divergence_count"),
        "next_action_count": len(actions) if isinstance(actions, list) else 0,
        "observation_count": len(observations) if isinstance(observations, list) else 0,
        "first_next_action": first_action,
    }


def command_failed(command: dict[str, Any]) -> bool:
    return command.get("exit_code") != 0


def build_run_summary(
    suite_dir: Path,
    summary_json: Path,
    summary_md: Path,
    generate_command: dict[str, Any],
    verify_command: dict[str, Any] | None,
    verification_summary: dict[str, Any],
    repo_root: Path,
) -> dict[str, Any]:
    commands = [
        {"name": "generate_scenario_suite", **generate_command},
    ]
    if verify_command is not None:
        commands.append({"name": "verify_scenario_suite", **verify_command})

    status = "passed"
    if command_failed(generate_command) or verify_command is None or command_failed(verify_command):
        status = "failed"

    suite = suite_summary(suite_dir)
    return {
        "observability_run_schema_version": RUN_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "git": git_metadata(repo_root),
        "commands": commands,
        "verification": verification_summary,
        "suite": suite,
        "artifacts": artifact_paths(suite_dir, summary_json, summary_md),
        "ai_handoff": [
            "Start with suite.first_next_action and open its primary_artifact.",
            "Use scenario-suite-observer.json for ordered next actions and compact observations.",
            "Use scenario-suite.json only when full contract details are needed.",
            "Use per-scenario telemetry.json only after triage.json and comparison.json are insufficient.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    suite = summary.get("suite", {})
    first_action = suite.get("first_next_action") or {}
    commands = summary.get("commands", [])
    lines = [
        "# Diagnostic Observability Run",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Git commit | {summary.get('git', {}).get('short_commit', '')} |",
        f"| Git dirty | {summary.get('git', {}).get('dirty', '')} |",
        f"| Suite schema | {suite.get('scenario_suite_schema_version')} |",
        f"| Observer schema | {suite.get('observer_schema_version')} |",
        f"| Scenario count | {suite.get('scenario_count')} |",
        f"| Next actions | {suite.get('next_action_count')} |",
        f"| Observations | {suite.get('observation_count')} |",
        f"| Summary | {markdown_cell(str(suite.get('summary', '')))} |",
        "",
        "## First Action",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Priority | {first_action.get('priority', '-')} |",
        f"| Action | {first_action.get('action_type', '-')} |",
        f"| Scenario | {first_action.get('scenario_id', '-')} |",
        f"| Primary artifact | {first_action.get('primary_artifact', '-')} |",
        "",
        "## Commands",
        "",
        "| Command | Exit code | Duration seconds |",
        "| --- | --- | --- |",
    ]
    for command in commands:
        lines.append(
            f"| {command.get('name')} | {command.get('exit_code')} | {command.get('duration_seconds')} |"
        )
    lines.extend(
        [
            "",
            "## Artifacts",
            "",
            "| Name | Path |",
            "| --- | --- |",
        ]
    )
    for name, artifact_path in summary.get("artifacts", {}).items():
        lines.append(f"| {name} | {artifact_path} |")
    lines.extend(["", "## AI Handoff", ""])
    for instruction in summary.get("ai_handoff", []):
        lines.append(f"- {instruction}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def markdown_cell(value: str) -> str:
    return value.replace("|", r"\|").replace("\r", " ").replace("\n", " ")


def print_failed_command(command: dict[str, Any]) -> None:
    print(
        f"command failed: {' '.join(command.get('argv', []))}",
        file=sys.stderr,
    )
    for label in ("stdout_tail", "stderr_tail"):
        tail = command.get(label, [])
        if tail:
            print(f"{label}:", file=sys.stderr)
            for line in tail:
                print(line, file=sys.stderr)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--suite-dir",
        type=Path,
        default=Path("target/diagnostics/observability-suite"),
        help="Directory to write the generated diagnostic scenario suite.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write the observability run JSON summary. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the observability run Markdown summary. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--cargo",
        default="cargo",
        help="Cargo executable to use when generating the suite.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Print the observability run summary JSON to stdout.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    suite_dir = args.suite_dir
    summary_json = args.summary_json or suite_dir / "observability-run.json"
    summary_md = args.summary_report or suite_dir / "observability-run.md"

    generate_argv = [
        args.cargo,
        "run",
        "--bin",
        "oxidenes-diagnostic",
        "--",
        "--scenario-suite-dir",
        str(suite_dir),
        "--no-stdout",
    ]
    generate_command = run_command(generate_argv, repo_root)
    verify_command: dict[str, Any] | None = None
    verification_summary: dict[str, Any] = {}
    if not command_failed(generate_command):
        verify_argv = [
            sys.executable,
            str(Path("scripts") / "verify_diagnostic_suite.py"),
            "--suite-dir",
            str(suite_dir),
            "--json",
        ]
        verify_command = run_command(verify_argv, repo_root)
        if not command_failed(verify_command) and verify_command["stdout_tail"]:
            verification_summary = json.loads("\n".join(verify_command["stdout_tail"]))

    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_md.parent.mkdir(parents=True, exist_ok=True)
    summary = build_run_summary(
        suite_dir,
        summary_json,
        summary_md,
        generate_command,
        verify_command,
        verification_summary,
        repo_root,
    )
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_md, summary)

    if command_failed(generate_command):
        print_failed_command(generate_command)
    if verify_command is not None and command_failed(verify_command):
        print_failed_command(verify_command)

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        print(
            "Diagnostic observability run "
            f"{summary['status']}: suite={suite_dir} "
            f"summary_json={summary_json} summary_report={summary_md}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
