#!/usr/bin/env python3
"""Profile the OxideNES diagnostic cartridge as a repeatable emulator workload."""

from __future__ import annotations

import argparse
import json
import math
import os
import statistics
import subprocess
import sys
import time
from collections import Counter
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


PROFILE_SCHEMA_VERSION = 1
OUTPUT_TAIL_LINES = 80


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def command_tail(output: str, limit: int = OUTPUT_TAIL_LINES) -> list[str]:
    return output.splitlines()[-limit:]


def generated_at_utc() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def script_path(*parts: str) -> Path:
    return Path(__file__).resolve().parent.parent.joinpath(*parts)


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
        "branch": run_git(["branch", "--show-current"], cwd),
        "dirty": bool(porcelain),
    }


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    return value if isinstance(value, dict) else {}


def write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def markdown_cell(value: Any) -> str:
    if value is None:
        return "-"
    return str(value).replace("|", r"\|").replace("\r", " ").replace("\n", " ")


def run_command(name: str, argv: list[str], cwd: Path) -> dict[str, Any]:
    started = time.monotonic()
    try:
        completed = subprocess.run(
            argv,
            cwd=cwd,
            text=True,
            capture_output=True,
            check=False,
        )
    except FileNotFoundError as exc:
        return {
            "name": name,
            "argv": argv,
            "exit_code": None,
            "status": "failed",
            "duration_seconds": round(time.monotonic() - started, 6),
            "stdout_tail": [],
            "stderr_tail": [str(exc)],
        }
    return {
        "name": name,
        "argv": argv,
        "exit_code": completed.returncode,
        "status": "passed" if completed.returncode == 0 else "failed",
        "duration_seconds": round(time.monotonic() - started, 6),
        "stdout_tail": command_tail(completed.stdout),
        "stderr_tail": command_tail(completed.stderr),
    }


def binary_suffix() -> str:
    return ".exe" if os.name == "nt" else ""


def default_binary_path(repo_root: Path, profile: str, target: str | None) -> Path:
    profile_dir = "release" if profile == "release" else "debug"
    target_root = repo_root / "target"
    if target:
        target_root = target_root / target
    return target_root / profile_dir / f"oxidenes-diagnostic{binary_suffix()}"


def build_binary(args: argparse.Namespace, repo_root: Path) -> dict[str, Any] | None:
    if args.binary or args.skip_build:
        return None
    argv = [args.cargo, "build", "--bin", "oxidenes-diagnostic"]
    if args.profile == "release":
        argv.append("--release")
    if args.target:
        argv.extend(["--target", args.target])
    return run_command("build_diagnostic_binary", argv, repo_root)


def metric_stats(values: list[float]) -> dict[str, Any]:
    finite = [value for value in values if math.isfinite(value)]
    if not finite:
        return {
            "count": 0,
            "min": None,
            "mean": None,
            "median": None,
            "max": None,
            "stdev": None,
        }
    return {
        "count": len(finite),
        "min": round(min(finite), 6),
        "mean": round(statistics.fmean(finite), 6),
        "median": round(statistics.median(finite), 6),
        "max": round(max(finite), 6),
        "stdev": round(statistics.stdev(finite), 6) if len(finite) > 1 else 0.0,
    }


def safe_rate(numerator: int | float | None, denominator: float) -> float | None:
    if numerator is None or denominator <= 0:
        return None
    return numerator / denominator


def timeline_summary(samples: list[dict[str, Any]]) -> list[dict[str, Any]]:
    by_test: dict[int, list[dict[str, Any]]] = {}
    for sample in samples:
        telemetry = as_dict(sample.get("telemetry"))
        for test in as_list(telemetry.get("timeline")):
            row = as_dict(test)
            test_id = row.get("test_id")
            if isinstance(test_id, int):
                by_test.setdefault(test_id, []).append(row)

    rows: list[dict[str, Any]] = []
    for test_id, entries in sorted(by_test.items()):
        durations = [
            entry.get("duration_cycles")
            for entry in entries
            if isinstance(entry.get("duration_cycles"), int)
        ]
        if not durations:
            continue
        first = entries[0]
        rows.append(
            {
                "test_id": test_id,
                "test_name": first.get("test_name"),
                "subsystem": first.get("subsystem"),
                "samples": len(durations),
                "duration_cycles": metric_stats([float(duration) for duration in durations]),
            }
        )
    rows.sort(key=lambda row: row["duration_cycles"].get("median") or 0, reverse=True)
    return rows


def extract_sample(
    index: int,
    role: str,
    command: dict[str, Any],
    telemetry_path: Path,
    report_path: Path,
) -> dict[str, Any]:
    telemetry = load_json(telemetry_path)
    verdict = as_dict(telemetry.get("verdict"))
    analysis = as_dict(telemetry.get("analysis"))
    suite = as_dict(telemetry.get("suite"))
    build_metadata = as_dict(suite.get("build"))
    timing = as_dict(analysis.get("timing"))
    slowest = as_dict(timing.get("slowest_test"))
    duration_seconds = float(command.get("duration_seconds") or 0)
    cycles = telemetry.get("cycles") if isinstance(telemetry.get("cycles"), int) else None
    frames = telemetry.get("frames") if isinstance(telemetry.get("frames"), int) else None
    return {
        "sample_index": index,
        "role": role,
        "status": "passed"
        if command.get("exit_code") == 0 and verdict.get("passed") is True
        else "failed",
        "duration_seconds": duration_seconds,
        "cycles": cycles,
        "frames": frames,
        "cycles_per_second": safe_rate(cycles, duration_seconds),
        "frames_per_second": safe_rate(frames, duration_seconds),
        "telemetry_schema_version": telemetry.get("schema_version"),
        "suite_version": suite.get("version"),
        "build_metadata": build_metadata,
        "build_version": build_metadata.get("version"),
        "build_type": build_metadata.get("build_type"),
        "health": analysis.get("health"),
        "passed": verdict.get("passed"),
        "test_count": len(as_list(telemetry.get("tests"))),
        "probe_count": len(as_list(telemetry.get("probes"))),
        "slowest_test": slowest,
        "telemetry_path": str(telemetry_path),
        "report_path": str(report_path),
        "command": command,
        "telemetry": telemetry,
    }


def run_diagnostic_sample(
    index: int,
    role: str,
    binary: Path,
    output_dir: Path,
    repo_root: Path,
) -> dict[str, Any]:
    sample_dir = output_dir / "samples" / f"{role}-{index:03d}"
    sample_dir.mkdir(parents=True, exist_ok=True)
    telemetry_path = sample_dir / "telemetry.json"
    report_path = sample_dir / "report.md"
    command = run_command(
        f"{role}_{index:03d}",
        [
            str(binary),
            "--json",
            str(telemetry_path),
            "--report",
            str(report_path),
            "--no-stdout",
        ],
        repo_root,
    )
    return extract_sample(index, role, command, telemetry_path, report_path)


def comparison_summary(
    current: dict[str, Any], baseline: dict[str, Any], threshold_percent: float
) -> dict[str, Any]:
    baseline_wall = as_dict(as_dict(baseline.get("aggregates")).get("wall_seconds")).get("mean")
    current_wall = as_dict(as_dict(current.get("aggregates")).get("wall_seconds")).get("mean")
    baseline_cps = as_dict(as_dict(baseline.get("aggregates")).get("cycles_per_second")).get("mean")
    current_cps = as_dict(as_dict(current.get("aggregates")).get("cycles_per_second")).get("mean")

    checks: list[dict[str, Any]] = []
    if isinstance(baseline_wall, (int, float)) and isinstance(current_wall, (int, float)):
        regression = ((current_wall - baseline_wall) / baseline_wall) * 100 if baseline_wall else 0
        checks.append(
            {
                "metric": "wall_seconds.mean",
                "baseline": baseline_wall,
                "current": current_wall,
                "regression_percent": round(regression, 3),
                "passed": regression <= threshold_percent,
            }
        )
    if isinstance(baseline_cps, (int, float)) and isinstance(current_cps, (int, float)):
        regression = ((baseline_cps - current_cps) / baseline_cps) * 100 if baseline_cps else 0
        checks.append(
            {
                "metric": "cycles_per_second.mean",
                "baseline": baseline_cps,
                "current": current_cps,
                "regression_percent": round(regression, 3),
                "passed": regression <= threshold_percent,
            }
        )

    return {
        "baseline_path": current.get("comparison", {}).get("baseline_path"),
        "threshold_percent": threshold_percent,
        "checks": checks,
        "passed": all(check.get("passed") for check in checks) if checks else True,
    }


def build_profile(args: argparse.Namespace, repo_root: Path) -> dict[str, Any]:
    output_dir = args.output_dir
    output_dir.mkdir(parents=True, exist_ok=True)
    build = build_binary(args, repo_root)
    binary = args.binary or default_binary_path(repo_root, args.profile, args.target)

    samples: list[dict[str, Any]] = []
    warmups: list[dict[str, Any]] = []
    for index in range(1, args.warmups + 1):
        warmups.append(run_diagnostic_sample(index, "warmup", binary, output_dir, repo_root))
    for index in range(1, args.samples + 1):
        samples.append(run_diagnostic_sample(index, "sample", binary, output_dir, repo_root))

    sample_status = "passed" if all(sample.get("status") == "passed" for sample in samples) else "failed"
    wall_values = [float(sample["duration_seconds"]) for sample in samples]
    cycles_per_second = [
        float(value)
        for sample in samples
        for value in [sample.get("cycles_per_second")]
        if isinstance(value, (int, float))
    ]
    frames_per_second = [
        float(value)
        for sample in samples
        for value in [sample.get("frames_per_second")]
        if isinstance(value, (int, float))
    ]
    slowest_counter = Counter(
        as_dict(sample.get("slowest_test")).get("test_name")
        for sample in samples
        if as_dict(sample.get("slowest_test")).get("test_name")
    )
    build_metadata = next(
        (
            as_dict(sample.get("build_metadata"))
            for sample in samples + warmups
            if as_dict(sample.get("build_metadata"))
        ),
        {},
    )

    profile: dict[str, Any] = {
        "diagnostic_cartridge_profile_schema_version": PROFILE_SCHEMA_VERSION,
        "generated_at": generated_at_utc(),
        "status": sample_status,
        "repo": git_metadata(repo_root),
        "config": {
            "samples": args.samples,
            "warmups": args.warmups,
            "profile": args.profile,
            "target": args.target,
            "binary": str(binary),
            "skip_build": args.skip_build,
        },
        "build_metadata": build_metadata,
        "build": build,
        "aggregates": {
            "wall_seconds": metric_stats(wall_values),
            "cycles_per_second": metric_stats(cycles_per_second),
            "frames_per_second": metric_stats(frames_per_second),
        },
        "slowest_test_counts": [
            {"test_name": name, "count": count} for name, count in slowest_counter.most_common()
        ],
        "top_tests_by_cycle_duration": timeline_summary(samples)[: args.top_tests],
        "warmups": [
            {key: value for key, value in sample.items() if key != "telemetry"} for sample in warmups
        ],
        "samples": [
            {key: value for key, value in sample.items() if key != "telemetry"} for sample in samples
        ],
        "artifacts": {
            "profile_json": str(output_dir / "diagnostic-cartridge-profile.json"),
            "profile_report": str(output_dir / "diagnostic-cartridge-profile.md"),
            "sample_dir": str(output_dir / "samples"),
        },
        "errors": [],
    }

    if build and build.get("exit_code") != 0:
        profile["errors"].append("diagnostic binary build failed")
    if not binary.is_file():
        profile["errors"].append(f"diagnostic binary was not found: {binary}")
    failed_samples = [
        sample["sample_index"] for sample in samples if sample.get("status") != "passed"
    ]
    if failed_samples:
        profile["errors"].append(f"diagnostic samples failed: {failed_samples}")

    if args.baseline_json:
        profile["comparison"] = {"baseline_path": str(args.baseline_json)}
        baseline = load_json(args.baseline_json)
        comparison = comparison_summary(profile, baseline, args.max_regression_percent)
        profile["comparison"] = comparison
        if args.fail_on_regression and not comparison.get("passed"):
            profile["errors"].append("profile comparison exceeded regression threshold")

    profile["status"] = "failed" if profile["errors"] else "passed"
    return profile


def format_report(profile: dict[str, Any]) -> str:
    lines = [
        "# OxideNES Diagnostic Cartridge Profile",
        "",
        "## Summary",
        "",
        f"| Status | {markdown_cell(profile.get('status'))} |",
        f"| Generated | {markdown_cell(profile.get('generated_at'))} |",
        f"| Commit | {markdown_cell(as_dict(profile.get('repo')).get('commit'))} |",
        f"| Dirty worktree | {markdown_cell(as_dict(profile.get('repo')).get('dirty'))} |",
        f"| Binary | {markdown_cell(as_dict(profile.get('config')).get('binary'))} |",
        f"| Build version | {markdown_cell(as_dict(profile.get('build_metadata')).get('version'))} |",
        f"| Build type | {markdown_cell(as_dict(profile.get('build_metadata')).get('build_type'))} |",
        f"| Package version | {markdown_cell(as_dict(profile.get('build_metadata')).get('package_version'))} |",
        f"| Samples / warmups | {markdown_cell(as_dict(profile.get('config')).get('samples'))} / {markdown_cell(as_dict(profile.get('config')).get('warmups'))} |",
        "",
        "## Aggregates",
        "",
        "| Metric | Count | Min | Mean | Median | Max | Stddev |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for name, stats in as_dict(profile.get("aggregates")).items():
        row = as_dict(stats)
        lines.append(
            "| "
            + " | ".join(
                markdown_cell(value)
                for value in [
                    name,
                    row.get("count"),
                    row.get("min"),
                    row.get("mean"),
                    row.get("median"),
                    row.get("max"),
                    row.get("stdev"),
                ]
            )
            + " |"
        )

    lines.extend(
        [
            "",
            "## Slowest Tests",
            "",
            "| Test | Samples | Median cycles | Max cycles |",
            "| --- | ---: | ---: | ---: |",
        ]
    )
    for row in as_list(profile.get("top_tests_by_cycle_duration")):
        data = as_dict(row)
        stats = as_dict(data.get("duration_cycles"))
        lines.append(
            "| "
            + " | ".join(
                markdown_cell(value)
                for value in [
                    data.get("test_name"),
                    data.get("samples"),
                    stats.get("median"),
                    stats.get("max"),
                ]
            )
            + " |"
        )

    if profile.get("comparison"):
        lines.extend(["", "## Baseline Comparison", "", "| Metric | Baseline | Current | Regression % | Passed |", "| --- | ---: | ---: | ---: | --- |"])
        for check in as_list(as_dict(profile.get("comparison")).get("checks")):
            row = as_dict(check)
            lines.append(
                "| "
                + " | ".join(
                    markdown_cell(value)
                    for value in [
                        row.get("metric"),
                        row.get("baseline"),
                        row.get("current"),
                        row.get("regression_percent"),
                        row.get("passed"),
                    ]
                )
                + " |"
            )

    lines.extend(["", "## Samples", "", "| Sample | Status | Seconds | Cycles/s | Frames/s | Health | Slowest test |", "| ---: | --- | ---: | ---: | ---: | --- | --- |"])
    for sample in as_list(profile.get("samples")):
        row = as_dict(sample)
        slowest = as_dict(row.get("slowest_test"))
        lines.append(
            "| "
            + " | ".join(
                markdown_cell(value)
                for value in [
                    row.get("sample_index"),
                    row.get("status"),
                    row.get("duration_seconds"),
                    round(row.get("cycles_per_second"), 3)
                    if isinstance(row.get("cycles_per_second"), (int, float))
                    else None,
                    round(row.get("frames_per_second"), 3)
                    if isinstance(row.get("frames_per_second"), (int, float))
                    else None,
                    row.get("health"),
                    slowest.get("test_name"),
                ]
            )
            + " |"
        )

    if profile.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(profile.get("errors")):
            lines.append(f"- {markdown_cell(error)}")

    lines.append("")
    return "\n".join(lines)


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("must be >= 1")
    return parsed


def non_negative_int(value: str) -> int:
    parsed = int(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("must be >= 0")
    return parsed


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Profile the diagnostic cartridge as a repeatable emulator workload."
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=script_path("target", "diagnostics", "diagnostic-profile"),
        help="Directory for profile summaries and per-sample telemetry.",
    )
    parser.add_argument("--samples", type=positive_int, default=5)
    parser.add_argument("--warmups", type=non_negative_int, default=1)
    parser.add_argument("--top-tests", type=positive_int, default=8)
    parser.add_argument("--profile", choices=["debug", "release"], default="release")
    parser.add_argument("--target", default=None)
    parser.add_argument("--binary", type=Path, default=None)
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--cargo", default="cargo")
    parser.add_argument("--baseline-json", type=Path, default=None)
    parser.add_argument("--max-regression-percent", type=float, default=20.0)
    parser.add_argument("--fail-on-regression", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    repo_root = script_path()
    profile = build_profile(args, repo_root)
    profile_path = args.output_dir / "diagnostic-cartridge-profile.json"
    report_path = args.output_dir / "diagnostic-cartridge-profile.md"
    write_json(profile_path, profile)
    report_path.write_text(format_report(profile), encoding="utf-8")

    print(
        "Diagnostic cartridge profile "
        f"{profile['status']}: samples={args.samples} "
        f"wall_mean={as_dict(as_dict(profile.get('aggregates')).get('wall_seconds')).get('mean')}s "
        f"cycles_per_second_mean={as_dict(as_dict(profile.get('aggregates')).get('cycles_per_second')).get('mean')} "
        f"profile_json={profile_path} profile_report={report_path}"
    )
    return 0 if profile["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
