#!/usr/bin/env python3
"""Run local OxideNES CI/dev-build gates and write AI-readable evidence."""

from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import sys
import time
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


LOCAL_CI_SCHEMA_VERSION = 1
OUTPUT_TAIL_LINES = 80


@dataclass(frozen=True)
class StepSpec:
    name: str
    argv: list[str]
    env: dict[str, str] = field(default_factory=dict)
    skip_reason: str | None = None


def command_tail(output: str, limit: int = OUTPUT_TAIL_LINES) -> list[str]:
    return output.splitlines()[-limit:]


def generated_at_utc() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def markdown_cell(value: Any) -> str:
    if value is None:
        return "-"
    return str(value).replace("|", r"\|").replace("\r", " ").replace("\n", " ")


def script_path(name: str) -> str:
    return str(Path("scripts") / name)


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


def detect_rust_host_target(cwd: Path) -> str:
    completed = subprocess.run(
        ["rustc", "-vV"],
        cwd=cwd,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        return ""
    for line in completed.stdout.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ").strip()
    return ""


def binary_name() -> str:
    return "oxidenes.exe" if platform.system().lower() == "windows" else "oxidenes"


def target_dir(repo_root: Path) -> Path:
    configured = os.environ.get("CARGO_TARGET_DIR")
    if configured:
        return Path(configured)
    return repo_root / "target"


def build_binary_path(repo_root: Path, target: str, profile: str) -> Path:
    profile_dir = "release" if profile == "release" else "debug"
    return target_dir(repo_root) / target / profile_dir / binary_name()


def run_step(
    spec: StepSpec,
    repo_root: Path,
    log_dir: Path,
    index: int,
    dry_run: bool,
) -> dict[str, Any]:
    log_dir.mkdir(parents=True, exist_ok=True)
    slug = "".join(ch if ch.isalnum() else "-" for ch in spec.name.lower()).strip("-")
    log_path = log_dir / f"{index:02d}-{slug}.log"

    if spec.skip_reason:
        return {
            "name": spec.name,
            "argv": spec.argv,
            "env": spec.env,
            "exit_code": None,
            "status": "skipped",
            "skip_reason": spec.skip_reason,
            "duration_seconds": 0,
            "log_path": "",
            "stdout_tail": [],
            "stderr_tail": [],
        }

    if dry_run:
        return {
            "name": spec.name,
            "argv": spec.argv,
            "env": spec.env,
            "exit_code": None,
            "status": "skipped",
            "skip_reason": "dry-run",
            "duration_seconds": 0,
            "log_path": "",
            "stdout_tail": [],
            "stderr_tail": [],
        }

    env = os.environ.copy()
    env.update(spec.env)
    started = time.monotonic()
    completed = subprocess.run(
        spec.argv,
        cwd=repo_root,
        env=env,
        text=True,
        capture_output=True,
        check=False,
    )
    duration = round(time.monotonic() - started, 3)
    log_path.write_text(
        "\n".join(
            [
                f"$ {' '.join(spec.argv)}",
                "",
                "## stdout",
                completed.stdout,
                "",
                "## stderr",
                completed.stderr,
            ]
        ),
        encoding="utf-8",
    )
    return {
        "name": spec.name,
        "argv": spec.argv,
        "env": spec.env,
        "exit_code": completed.returncode,
        "status": "passed" if completed.returncode == 0 else "failed",
        "duration_seconds": duration,
        "log_path": str(log_path),
        "stdout_tail": command_tail(completed.stdout),
        "stderr_tail": command_tail(completed.stderr),
    }


def step_passed(step: dict[str, Any]) -> bool:
    return step.get("status") in {"passed", "skipped"} and step.get("exit_code") in {0, None}


def make_steps(args: argparse.Namespace, repo_root: Path) -> tuple[list[StepSpec], dict[str, str]]:
    python = sys.executable or "python"
    target = args.target or detect_rust_host_target(repo_root)
    if not target:
        raise SystemExit("could not detect rust host target; pass --target explicitly")

    diagnostics_dir = args.output_dir / "diagnostics"
    baseline_json = diagnostics_dir / "local-ci-baseline.json"
    bundle_dir = diagnostics_dir / "local-ci-bundle"
    suite_dir = args.suite_dir or diagnostics_dir / "scenario-suite"
    profile_dir = diagnostics_dir / "profile"
    build_profile = args.build_profile
    build_env = {"OXIDENES_RELEASE": "1"} if build_profile == "release" else {}
    build_args = ["cargo", "build", "--target", target]
    if build_profile == "release":
        build_args.insert(2, "--release")
    test_args = ["cargo", "test", "--target", target]
    binary = build_binary_path(repo_root, target, build_profile)

    steps = [
        StepSpec("fmt", ["cargo", "fmt", "--", "--check"]),
        StepSpec("ip-compliance", [python, script_path("ip_compliance_audit.py")]),
        StepSpec(
            "security-audit",
            ["cargo", "audit", "--no-fetch", "--stale"],
            skip_reason="--skip-security-audit" if args.skip_security_audit else None,
        ),
        StepSpec(
            "diagnostic-baseline-json",
            [
                "cargo",
                "run",
                "--bin",
                "oxidenes-diagnostic",
                "--",
                "--json",
                str(baseline_json),
                "--no-stdout",
            ],
            skip_reason="--skip-diagnostic-bundle" if args.skip_diagnostic_bundle else None,
        ),
        StepSpec(
            "diagnostic-bundle",
            [
                "cargo",
                "run",
                "--bin",
                "oxidenes-diagnostic",
                "--",
                "--bundle-dir",
                str(bundle_dir),
                "--baseline-json",
                str(baseline_json),
                "--no-stdout",
            ],
            skip_reason="--skip-diagnostic-bundle" if args.skip_diagnostic_bundle else None,
        ),
        StepSpec(
            "diagnostic-e2e",
            [python, script_path("run_diagnostic_e2e.py"), "--suite-dir", str(suite_dir)],
            skip_reason="--skip-diagnostic-e2e" if args.skip_diagnostic_e2e else None,
        ),
        StepSpec(
            "verify-diagnostic-observability",
            [python, script_path("verify_diagnostic_observability.py"), "--suite-dir", str(suite_dir)],
            skip_reason="--skip-diagnostic-e2e" if args.skip_diagnostic_e2e else None,
        ),
        StepSpec(
            "verify-diagnostic-suite",
            [python, script_path("verify_diagnostic_suite.py"), "--suite-dir", str(suite_dir)],
            skip_reason="--skip-diagnostic-e2e" if args.skip_diagnostic_e2e else None,
        ),
        StepSpec(
            "diagnostic-profile",
            [
                python,
                script_path("profile_diagnostic_cartridge.py"),
                "--output-dir",
                str(profile_dir),
                "--profile",
                "debug",
                "--samples",
                str(args.profile_samples),
                "--warmups",
                str(args.profile_warmups),
                "--skip-build",
                "--max-regression-percent",
                str(args.profile_max_regression_percent),
            ],
            skip_reason="--skip-profile" if args.skip_profile else None,
        ),
        StepSpec(
            "build",
            build_args,
            env=build_env,
            skip_reason="--skip-build-test" if args.skip_build_test else None,
        ),
        StepSpec(
            "test",
            test_args,
            env=build_env,
            skip_reason="--skip-build-test" if args.skip_build_test else None,
        ),
        StepSpec(
            "smoke-binary",
            [str(binary), "--version"],
            skip_reason="--skip-build-test" if args.skip_build_test else None,
        ),
        StepSpec(
            "clippy",
            ["cargo", "clippy", "--", "-D", "warnings"],
            skip_reason="--skip-clippy" if args.skip_clippy else None,
        ),
    ]
    artifacts = {
        "output_dir": str(args.output_dir),
        "report_json": str(args.output_dir / "local-ci-report.json"),
        "report_markdown": str(args.output_dir / "local-ci-report.md"),
        "log_dir": str(args.output_dir / "logs"),
        "diagnostic_baseline_json": str(baseline_json),
        "diagnostic_bundle_dir": str(bundle_dir),
        "diagnostic_scenario_suite_dir": str(suite_dir),
        "diagnostic_profile_dir": str(profile_dir),
        "binary": str(binary),
    }
    return steps, artifacts


def write_markdown_report(path: Path, report: dict[str, Any]) -> None:
    lines = [
        "# OxideNES Local CI Report",
        "",
        f"- Status: `{markdown_cell(report.get('status'))}`",
        f"- Generated: `{markdown_cell(report.get('generated_at_utc'))}`",
        f"- Profile: `{markdown_cell(report.get('config', {}).get('build_profile'))}`",
        f"- Target: `{markdown_cell(report.get('config', {}).get('target'))}`",
        f"- Commit: `{markdown_cell(report.get('git', {}).get('short_commit'))}`",
        f"- Dirty: `{markdown_cell(report.get('git', {}).get('dirty'))}`",
        "",
        "## Commands",
        "",
        "| Step | Status | Exit | Seconds | Log |",
        "| --- | --- | ---: | ---: | --- |",
    ]
    for step in report.get("commands", []):
        log_path = step.get("log_path") or step.get("skip_reason") or "-"
        lines.append(
            "| {name} | {status} | {exit_code} | {seconds} | {log} |".format(
                name=markdown_cell(step.get("name")),
                status=markdown_cell(step.get("status")),
                exit_code=markdown_cell(step.get("exit_code")),
                seconds=markdown_cell(step.get("duration_seconds")),
                log=markdown_cell(log_path),
            )
        )
    lines.extend(
        [
            "",
            "## Artifacts",
            "",
            "| Artifact | Path |",
            "| --- | --- |",
        ]
    )
    for name, artifact_path in report.get("artifacts", {}).items():
        lines.append(f"| {markdown_cell(name)} | `{markdown_cell(artifact_path)}` |")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run local OxideNES CI/dev-build gates and write evidence reports."
    )
    parser.add_argument("--output-dir", type=Path, default=Path("target/local-ci/dev"))
    parser.add_argument("--suite-dir", type=Path, default=None)
    parser.add_argument("--target", default="")
    parser.add_argument("--build-profile", choices=["debug", "release"], default="debug")
    parser.add_argument("--profile-samples", type=int, default=3)
    parser.add_argument("--profile-warmups", type=int, default=1)
    parser.add_argument("--profile-max-regression-percent", type=float, default=50.0)
    parser.add_argument("--skip-security-audit", action="store_true")
    parser.add_argument("--skip-diagnostic-bundle", action="store_true")
    parser.add_argument("--skip-diagnostic-e2e", action="store_true")
    parser.add_argument("--skip-profile", action="store_true")
    parser.add_argument("--skip-build-test", action="store_true")
    parser.add_argument("--skip-clippy", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo_root = Path(__file__).resolve().parent.parent
    args.output_dir = args.output_dir
    steps, artifacts = make_steps(args, repo_root)
    log_dir = Path(artifacts["log_dir"])
    commands = [
        run_step(step, repo_root, log_dir, index, args.dry_run)
        for index, step in enumerate(steps, start=1)
    ]
    failed = [step for step in commands if step.get("status") == "failed"]
    status = "failed" if failed else "planned" if args.dry_run else "passed"
    report = {
        "local_ci_schema_version": LOCAL_CI_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "dry_run": args.dry_run,
        "git": git_metadata(repo_root),
        "config": {
            "target": args.target or detect_rust_host_target(repo_root),
            "build_profile": args.build_profile,
            "profile_samples": args.profile_samples,
            "profile_warmups": args.profile_warmups,
            "profile_max_regression_percent": args.profile_max_regression_percent,
        },
        "commands": commands,
        "artifacts": artifacts,
        "summary": {
            "passed": sum(1 for step in commands if step.get("status") == "passed"),
            "failed": len(failed),
            "skipped": sum(1 for step in commands if step.get("status") == "skipped"),
        },
    }
    report_json = Path(artifacts["report_json"])
    report_markdown = Path(artifacts["report_markdown"])
    write_json(report_json, report)
    write_markdown_report(report_markdown, report)
    print(
        "Local CI {status}: report_json={json} report_markdown={md} commands={count}".format(
            status=status,
            json=report_json,
            md=report_markdown,
            count=len(commands),
        )
    )
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
