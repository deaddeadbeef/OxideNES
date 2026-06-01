#!/usr/bin/env python3
"""Run and verify the OxideNES diagnostic scenario-suite observability corpus."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from collections import Counter
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Callable


RUN_SCHEMA_VERSION = 1
REPLAY_RUN_SCHEMA_VERSION = 1
DEBUG_INDEX_SCHEMA_VERSION = 1
OBSERVABILITY_ANALYSIS_SCHEMA_VERSION = 1
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


def load_jsonl(path: Path) -> tuple[list[dict[str, Any]], list[str]]:
    entries: list[dict[str, Any]] = []
    errors: list[str] = []
    if not path.is_file():
        return entries, [f"missing JSONL artifact: {path}"]
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            value = json.loads(line)
        except json.JSONDecodeError as error:
            errors.append(f"{path}:{line_number}: {error}")
            continue
        if isinstance(value, dict):
            entries.append(value)
        else:
            errors.append(f"{path}:{line_number}: expected JSON object")
    return entries, errors


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def as_int(value: Any, default: int = 0) -> int:
    return value if isinstance(value, int) and not isinstance(value, bool) else default


def artifact_paths(
    suite_dir: Path,
    summary_json: Path,
    summary_md: Path,
    replay_summary: dict[str, Any] | None,
    debug_index_summary: dict[str, Any] | None,
    observability_analysis: dict[str, Any] | None,
) -> dict[str, str]:
    artifacts = {
        "suite_dir": str(suite_dir),
        "scenario_suite_json": str(suite_dir / "scenario-suite.json"),
        "scenario_suite_report": str(suite_dir / "scenario-suite.md"),
        "scenario_suite_observer_json": str(suite_dir / "scenario-suite-observer.json"),
        "scenario_suite_observer_report": str(suite_dir / "scenario-suite-observer.md"),
        "observability_run_json": str(summary_json),
        "observability_run_report": str(summary_md),
    }
    if replay_summary:
        for name, path in replay_summary.get("artifacts", {}).items():
            artifact_name = name if name.startswith("replay_") else f"replay_{name}"
            artifacts[artifact_name] = str(path)
    if debug_index_summary:
        for name, path in debug_index_summary.get("artifacts", {}).items():
            artifacts[name] = str(path)
    if observability_analysis:
        for name, path in observability_analysis.get("artifacts", {}).items():
            artifacts[name] = str(path)
    return artifacts


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


def debug_index_paths(suite_dir: Path) -> dict[str, str]:
    return {
        "debug_index_jsonl": str(suite_dir / "diagnostic-debug-index.jsonl"),
        "debug_index_report": str(suite_dir / "diagnostic-debug-index.md"),
    }


def observability_analysis_paths(suite_dir: Path) -> dict[str, str]:
    return {
        "observability_analysis_json": str(suite_dir / "diagnostic-observability-analysis.json"),
        "observability_analysis_report": str(suite_dir / "diagnostic-observability-analysis.md"),
    }


def compact_instruction(value: Any) -> dict[str, Any] | None:
    instruction = as_dict(value)
    if not instruction:
        return None
    return {
        "sequence": instruction.get("sequence"),
        "cycle": instruction.get("cycle"),
        "frame": instruction.get("frame"),
        "current_test": instruction.get("current_test"),
        "current_test_name": instruction.get("current_test_name"),
        "pc_hex": instruction.get("pc_hex"),
        "instruction": instruction.get("instruction"),
        "symbol": instruction.get("symbol"),
        "status_hex": instruction.get("status_hex"),
        "failure_code_hex": instruction.get("failure_code_hex"),
    }


def compact_event(value: Any) -> dict[str, Any] | None:
    event = as_dict(value)
    if not event:
        return None
    return {
        "kind": event.get("kind"),
        "cycle": event.get("cycle"),
        "frame": event.get("frame"),
        "status_hex": event.get("status_hex"),
        "current_test": event.get("current_test"),
        "current_test_name": event.get("current_test_name"),
        "pc_hex": event.get("pc_hex"),
        "note": event.get("note"),
    }


def first_top_difference(comparison: dict[str, Any]) -> dict[str, Any] | None:
    top_differences = as_list(comparison.get("top_differences"))
    first = as_dict(top_differences[0]) if top_differences else {}
    if not first:
        return None
    return {
        "severity": first.get("severity"),
        "category": first.get("category"),
        "path": first.get("path"),
        "summary": first.get("summary"),
    }


def artifact_path(suite_dir: Path, relative_path: Any) -> Path:
    return suite_dir / str(relative_path)


def build_debug_index_entries(suite_dir: Path) -> tuple[list[dict[str, Any]], list[str]]:
    manifest = load_json(suite_dir / "scenario-suite.json")
    observer = load_json(suite_dir / "scenario-suite-observer.json")
    actions_by_id = {
        action.get("scenario_id"): action
        for action in as_list(observer.get("next_actions"))
        if isinstance(action, dict)
    }
    observations_by_id = {
        observation.get("scenario_id"): observation
        for observation in as_list(observer.get("observations"))
        if isinstance(observation, dict)
    }
    entries: list[dict[str, Any]] = []
    errors: list[str] = []

    for scenario in as_list(manifest.get("scenarios")):
        if not isinstance(scenario, dict):
            continue
        scenario_id = scenario.get("id")
        if not isinstance(scenario_id, str):
            errors.append("scenario without string id in scenario-suite.json")
            continue
        artifacts = as_dict(scenario.get("artifacts"))
        triage_path = artifacts.get("triage_json")
        triage = load_json(artifact_path(suite_dir, triage_path)) if triage_path else {}
        if not triage:
            errors.append(f"{scenario_id}: missing or invalid triage artifact")

        focus = as_dict(triage.get("debug_focus"))
        failure = as_dict(triage.get("failure"))
        probes = as_dict(triage.get("probes"))
        timing = as_dict(triage.get("timing"))
        event_tail = as_list(triage.get("event_tail"))
        comparison = as_dict(scenario.get("comparison"))
        action = as_dict(actions_by_id.get(scenario_id))
        observation = as_dict(observations_by_id.get(scenario_id))
        entry = {
            "debug_index_schema_version": DEBUG_INDEX_SCHEMA_VERSION,
            "scenario_id": scenario_id,
            "title": scenario.get("title"),
            "role": observation.get("role"),
            "outcome": observation.get("outcome"),
            "expected_passed": scenario.get("expected_passed"),
            "actual_passed": scenario.get("actual_passed"),
            "expectation_met": scenario.get("expectation_met"),
            "contract_all_matched": as_dict(scenario.get("contract")).get("all_matched"),
            "comparison_passed": comparison.get("passed"),
            "comparison_difference_count": comparison.get("difference_count"),
            "top_difference": first_top_difference(comparison),
            "health": triage.get("health"),
            "summary": triage.get("summary"),
            "current_test": as_dict(triage.get("current_test")),
            "debug_focus": {
                "health": focus.get("health"),
                "focus_test_id": focus.get("focus_test_id"),
                "focus_test_name": focus.get("focus_test_name"),
                "focus_subsystem": focus.get("focus_subsystem"),
                "focus_domain": focus.get("focus_domain"),
                "failure_kind": focus.get("failure_kind"),
                "failure_code_hex": focus.get("failure_code_hex"),
                "failed_probe_ids": as_list(focus.get("failed_probe_ids")),
                "skipped_probe_count": focus.get("skipped_probe_count"),
                "last_event": compact_event(focus.get("last_event")),
                "terminal_instruction": compact_instruction(focus.get("terminal_instruction")),
                "last_test_instruction": compact_instruction(focus.get("last_test_instruction")),
            },
            "failure": {
                "kind": failure.get("kind"),
                "test_id": failure.get("test_id"),
                "test_name": failure.get("test_name"),
                "subsystem": failure.get("subsystem"),
                "tier": failure.get("tier"),
                "failure_code_hex": failure.get("failure_code_hex"),
                "assertion": failure.get("assertion"),
                "expected": failure.get("expected"),
                "observed": failure.get("observed"),
                "likely_domain": failure.get("likely_domain"),
                "remediation_hint": failure.get("remediation_hint"),
            },
            "input": as_dict(triage.get("input")),
            "probes": {
                "total": probes.get("total_probes"),
                "passed": probes.get("passed_probes"),
                "failed": probes.get("failed_probes"),
                "skipped": probes.get("skipped_probes"),
                "first_failed_probe": probes.get("first_failed_probe"),
            },
            "coverage_gap_ids": [
                gap.get("id") for gap in as_list(triage.get("coverage_gaps")) if isinstance(gap, dict)
            ],
            "timing": {
                "cpu_cycles": timing.get("cpu_cycles"),
                "frames": timing.get("frames"),
                "timeout": timing.get("timeout"),
            },
            "event_tail_last": compact_event(event_tail[-1]) if event_tail else None,
            "next_action": {
                "priority": action.get("priority"),
                "action_type": action.get("action_type"),
                "primary_artifact": action.get("primary_artifact"),
                "evidence": as_list(action.get("evidence")),
            },
            "replay_args": as_list(scenario.get("replay_args")),
            "artifacts": artifacts,
        }
        if not entry["debug_focus"]["terminal_instruction"] and not entry["debug_focus"]["last_event"]:
            errors.append(f"{scenario_id}: missing terminal instruction or last-event debug anchor")
        entries.append(entry)

    return entries, errors


def write_debug_index_markdown(path: Path, entries: list[dict[str, Any]]) -> None:
    lines = [
        "# Diagnostic Debug Index",
        "",
        "| Scenario | Role | Health | Focus domain | Failure kind | Failed probes | Terminal instruction | Top difference | Next artifact |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for entry in entries:
        focus = as_dict(entry.get("debug_focus"))
        terminal = as_dict(focus.get("terminal_instruction"))
        top_difference = as_dict(entry.get("top_difference"))
        next_action = as_dict(entry.get("next_action"))
        terminal_label = " ".join(
            part
            for part in (
                str(terminal.get("pc_hex") or ""),
                str(terminal.get("instruction") or ""),
                str(terminal.get("symbol") or ""),
            )
            if part
        )
        lines.append(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |".format(
                entry.get("scenario_id"),
                entry.get("role") or "-",
                entry.get("health") or "-",
                focus.get("focus_domain") or "-",
                focus.get("failure_kind") or "-",
                markdown_cell(",".join(focus.get("failed_probe_ids") or []) or "-"),
                markdown_cell(terminal_label or "-"),
                markdown_cell(str(top_difference.get("path") or "-")),
                next_action.get("primary_artifact") or as_dict(entry.get("artifacts")).get("triage_json"),
            )
        )
    lines.extend(
        [
            "",
            "## AI Handoff",
            "",
            "- Read this index first when choosing a scenario or debug anchor.",
            "- Use `terminal_instruction` and `last_event` before loading full telemetry.",
            "- Use `replay_args` to regenerate one scenario when the indexed focus needs live confirmation.",
        ]
    )
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_debug_index(suite_dir: Path) -> dict[str, Any]:
    paths = debug_index_paths(suite_dir)
    entries, errors = build_debug_index_entries(suite_dir)
    jsonl_path = Path(paths["debug_index_jsonl"])
    report_path = Path(paths["debug_index_report"])
    jsonl_path.write_text(
        "".join(json.dumps(entry, sort_keys=True) + "\n" for entry in entries),
        encoding="utf-8",
    )
    write_debug_index_markdown(report_path, entries)
    scenario_ids = [entry.get("scenario_id") for entry in entries]
    return {
        "debug_index_schema_version": DEBUG_INDEX_SCHEMA_VERSION,
        "status": "passed" if not errors else "failed",
        "entry_count": len(entries),
        "scenario_ids": scenario_ids,
        "errors": errors,
        "artifacts": paths,
        "ai_handoff": [
            "Use diagnostic-debug-index.jsonl for one-row-per-scenario routing before opening per-scenario telemetry.",
            "Use diagnostic-debug-index.md for a compact human-readable scenario matrix in CI artifacts.",
        ],
    }


def unique_strings(values: list[Any]) -> list[str]:
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        if isinstance(value, str) and value and value not in seen:
            seen.add(value)
            result.append(value)
    return result


def entry_focus_domain(entry: dict[str, Any]) -> str | None:
    focus = as_dict(entry.get("debug_focus"))
    failure = as_dict(entry.get("failure"))
    domain = focus.get("focus_domain") or failure.get("likely_domain")
    return domain if isinstance(domain, str) and domain else None


def entry_focus_subsystem(entry: dict[str, Any]) -> str | None:
    focus = as_dict(entry.get("debug_focus"))
    subsystem = focus.get("focus_subsystem")
    if isinstance(subsystem, str) and subsystem:
        return subsystem
    domain = entry_focus_domain(entry)
    if domain and "." in domain:
        return domain.split(".", 1)[0]
    return domain


def entry_failed_probe_ids(entry: dict[str, Any]) -> list[str]:
    focus = as_dict(entry.get("debug_focus"))
    return unique_strings(as_list(focus.get("failed_probe_ids")))


def entry_primary_artifact(entry: dict[str, Any]) -> str | None:
    next_action = as_dict(entry.get("next_action"))
    artifact = next_action.get("primary_artifact") or as_dict(entry.get("artifacts")).get(
        "triage_json"
    )
    return artifact if isinstance(artifact, str) and artifact else None


def entry_debug_anchor(entry: dict[str, Any]) -> dict[str, Any]:
    focus = as_dict(entry.get("debug_focus"))
    terminal = as_dict(focus.get("terminal_instruction"))
    last_event = as_dict(focus.get("last_event")) or as_dict(entry.get("event_tail_last"))
    if terminal:
        return {
            "kind": "terminal_instruction",
            "pc_hex": terminal.get("pc_hex"),
            "instruction": terminal.get("instruction"),
            "symbol": terminal.get("symbol"),
            "cycle": terminal.get("cycle"),
            "frame": terminal.get("frame"),
        }
    if last_event:
        return {
            "kind": "last_event",
            "event_kind": last_event.get("kind"),
            "pc_hex": last_event.get("pc_hex"),
            "cycle": last_event.get("cycle"),
            "frame": last_event.get("frame"),
            "note": last_event.get("note"),
        }
    return {"kind": "missing"}


def entry_analysis_score(entry: dict[str, Any]) -> int:
    score = min(as_int(entry.get("comparison_difference_count")), 50)
    health = entry.get("health")
    failed_probe_count = len(entry_failed_probe_ids(entry))
    if health and health != "healthy":
        score += 50
    if health == "timed_out":
        score += 20
    if health == "host_validation_failed":
        score += 15
    if entry.get("comparison_passed") is False:
        score += 10
    if entry.get("contract_all_matched") is False:
        score += 40
    if entry.get("expectation_met") is False:
        score += 40
    score += min(failed_probe_count, 5) * 3
    return score


def scenario_analysis_brief(entry: dict[str, Any]) -> dict[str, Any]:
    focus = as_dict(entry.get("debug_focus"))
    failure = as_dict(entry.get("failure"))
    next_action = as_dict(entry.get("next_action"))
    top_difference = as_dict(entry.get("top_difference"))
    return {
        "scenario_id": entry.get("scenario_id"),
        "title": entry.get("title"),
        "role": entry.get("role"),
        "outcome": entry.get("outcome"),
        "health": entry.get("health"),
        "score": entry_analysis_score(entry),
        "focus_subsystem": entry_focus_subsystem(entry),
        "focus_domain": entry_focus_domain(entry),
        "failure_kind": focus.get("failure_kind") or failure.get("kind"),
        "failure_code_hex": focus.get("failure_code_hex") or failure.get("failure_code_hex"),
        "failed_probe_ids": entry_failed_probe_ids(entry),
        "debug_anchor": entry_debug_anchor(entry),
        "top_difference": {
            "path": top_difference.get("path"),
            "category": top_difference.get("category"),
            "summary": top_difference.get("summary"),
        },
        "primary_artifact": entry_primary_artifact(entry),
        "replay_args": as_list(entry.get("replay_args")),
        "next_action": {
            "priority": next_action.get("priority"),
            "action_type": next_action.get("action_type"),
            "evidence": as_list(next_action.get("evidence")),
        },
    }


def count_by(
    entries: list[dict[str, Any]],
    field_name: str,
    value_fn: Callable[[dict[str, Any]], Any],
) -> list[dict[str, Any]]:
    values_by_scenario: list[tuple[str, str]] = []
    for entry in entries:
        value = value_fn(entry)
        if value in (None, ""):
            continue
        values_by_scenario.append((str(entry.get("scenario_id")), str(value)))
    counts = Counter(value for _, value in values_by_scenario)
    rows: list[dict[str, Any]] = []
    for value, count in sorted(counts.items(), key=lambda item: (-item[1], item[0])):
        rows.append(
            {
                field_name: value,
                "count": count,
                "scenario_ids": [
                    scenario_id
                    for scenario_id, scenario_value in values_by_scenario
                    if scenario_value == value
                ],
            }
        )
    return rows


def build_ranked_hypotheses(entries: list[dict[str, Any]]) -> list[dict[str, Any]]:
    grouped: dict[str, list[dict[str, Any]]] = {}
    actionable = [
        entry
        for entry in entries
        if entry.get("health") != "healthy" or as_dict(entry.get("next_action")).get("action_type")
    ]
    for entry in actionable:
        domain = entry_focus_domain(entry) or entry_focus_subsystem(entry) or "unfocused"
        grouped.setdefault(domain, []).append(entry)

    hypotheses: list[dict[str, Any]] = []
    for domain, domain_entries in grouped.items():
        scenario_briefs = sorted(
            [scenario_analysis_brief(entry) for entry in domain_entries],
            key=lambda brief: (-as_int(brief.get("score")), str(brief.get("scenario_id"))),
        )
        score = max(as_int(brief.get("score")) for brief in scenario_briefs) + (
            len(scenario_briefs) - 1
        ) * 5
        contract_matched = all(entry.get("contract_all_matched") is True for entry in domain_entries)
        has_focus = all(entry_focus_domain(entry) for entry in domain_entries)
        confidence = "high" if contract_matched and has_focus else "medium"
        first = scenario_briefs[0]
        hypotheses.append(
            {
                "rank": 0,
                "score": score,
                "confidence": confidence,
                "focus_domain": domain,
                "focus_subsystem": entry_focus_subsystem(domain_entries[0]),
                "scenario_count": len(domain_entries),
                "scenario_ids": [brief.get("scenario_id") for brief in scenario_briefs],
                "healths": unique_strings([entry.get("health") for entry in domain_entries]),
                "failure_kinds": unique_strings(
                    [brief.get("failure_kind") for brief in scenario_briefs]
                ),
                "failed_probe_ids": unique_strings(
                    [
                        probe_id
                        for brief in scenario_briefs
                        for probe_id in as_list(brief.get("failed_probe_ids"))
                    ]
                ),
                "primary_artifacts": unique_strings(
                    [brief.get("primary_artifact") for brief in scenario_briefs]
                ),
                "suggested_next_action": {
                    "scenario_id": first.get("scenario_id"),
                    "open_artifact": first.get("primary_artifact"),
                    "debug_anchor": first.get("debug_anchor"),
                    "replay_args": first.get("replay_args"),
                },
                "evidence": scenario_briefs,
            }
        )

    hypotheses.sort(
        key=lambda hypothesis: (
            -as_int(hypothesis.get("score")),
            str(hypothesis.get("focus_domain")),
        )
    )
    for index, hypothesis in enumerate(hypotheses, start=1):
        hypothesis["rank"] = index
    return hypotheses


def build_observability_analysis(
    suite_dir: Path, debug_index_summary: dict[str, Any] | None, repo_root: Path
) -> dict[str, Any]:
    paths = observability_analysis_paths(suite_dir)
    debug_paths = debug_index_summary.get("artifacts", {}) if debug_index_summary else {}
    debug_index_path = Path(
        debug_paths.get("debug_index_jsonl")
        or debug_index_paths(suite_dir)["debug_index_jsonl"]
    )
    entries, errors = load_jsonl(debug_index_path)
    if debug_index_summary and debug_index_summary.get("status") != "passed":
        errors.extend(
            f"debug index failed: {error}"
            for error in as_list(debug_index_summary.get("errors"))
        )
    if not entries:
        errors.append("debug index has no scenario entries")
    ranked_hypotheses = build_ranked_hypotheses(entries)
    scenario_priority = sorted(
        [scenario_analysis_brief(entry) for entry in entries],
        key=lambda brief: (-as_int(brief.get("score")), str(brief.get("scenario_id"))),
    )
    health_counts = count_by(entries, "health", lambda entry: entry.get("health"))
    role_counts = count_by(entries, "role", lambda entry: entry.get("role"))
    outcome_counts = count_by(entries, "outcome", lambda entry: entry.get("outcome"))
    focus_domain_counts = count_by(entries, "focus_domain", entry_focus_domain)
    focus_subsystem_counts = count_by(entries, "focus_subsystem", entry_focus_subsystem)
    coverage_gap_counts = Counter(
        gap_id
        for entry in entries
        for gap_id in as_list(entry.get("coverage_gap_ids"))
        if isinstance(gap_id, str) and gap_id
    )
    return {
        "observability_analysis_schema_version": OBSERVABILITY_ANALYSIS_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": "passed" if not errors else "failed",
        "recommended_exit_code": 0 if not errors else 1,
        "git": git_metadata(repo_root),
        "source_artifacts": {
            "debug_index_jsonl": str(debug_index_path),
            "debug_index_report": debug_paths.get("debug_index_report")
            or debug_index_paths(suite_dir)["debug_index_report"],
        },
        "artifacts": paths,
        "scenario_count": len(entries),
        "actionable_scenario_count": len(
            [
                entry
                for entry in entries
                if entry.get("health") != "healthy"
                or as_dict(entry.get("next_action")).get("action_type")
            ]
        ),
        "baseline_scenario_ids": [
            entry.get("scenario_id")
            for entry in entries
            if entry.get("health") == "healthy"
            and entry.get("role") in {"baseline", "expected_pass_fixture"}
        ],
        "health_counts": health_counts,
        "role_counts": role_counts,
        "outcome_counts": outcome_counts,
        "focus_domain_counts": focus_domain_counts,
        "focus_subsystem_counts": focus_subsystem_counts,
        "coverage_gap_counts": [
            {"coverage_gap_id": gap_id, "count": count}
            for gap_id, count in sorted(
                coverage_gap_counts.items(), key=lambda item: (-item[1], item[0])
            )
        ],
        "hypothesis_count": len(ranked_hypotheses),
        "ranked_hypotheses": ranked_hypotheses,
        "scenario_priority": scenario_priority,
        "errors": errors,
        "ai_handoff": [
            "Start with ranked_hypotheses[0] when choosing the highest-signal subsystem/domain to inspect.",
            "Use suggested_next_action.replay_args to regenerate the focused scenario before opening raw telemetry.",
            "Use scenario_priority for per-scenario ordering when multiple domains tie or the aggregate hypothesis is too broad.",
        ],
    }


def write_observability_analysis_markdown(path: Path, analysis: dict[str, Any]) -> None:
    top = (
        as_dict(as_list(analysis.get("ranked_hypotheses"))[0])
        if analysis.get("ranked_hypotheses")
        else {}
    )
    lines = [
        "# Diagnostic Observability Analysis",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {analysis.get('status')} |",
        f"| Generated at UTC | {analysis.get('generated_at_utc')} |",
        f"| Git commit | {analysis.get('git', {}).get('short_commit', '')} |",
        f"| Scenario count | {analysis.get('scenario_count')} |",
        f"| Actionable scenarios | {analysis.get('actionable_scenario_count')} |",
        f"| Hypotheses | {analysis.get('hypothesis_count')} |",
        f"| Top focus domain | {top.get('focus_domain', '-')} |",
        "",
        "## Ranked Hypotheses",
        "",
        "| Rank | Score | Confidence | Focus domain | Subsystem | Scenarios | Failed probes | Open first |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for hypothesis in as_list(analysis.get("ranked_hypotheses")):
        if not isinstance(hypothesis, dict):
            continue
        next_action = as_dict(hypothesis.get("suggested_next_action"))
        lines.append(
            "| {} | {} | {} | {} | {} | {} | {} | {} |".format(
                hypothesis.get("rank"),
                hypothesis.get("score"),
                hypothesis.get("confidence"),
                markdown_cell(str(hypothesis.get("focus_domain") or "-")),
                markdown_cell(str(hypothesis.get("focus_subsystem") or "-")),
                markdown_cell(
                    ",".join(str(value) for value in as_list(hypothesis.get("scenario_ids")))
                ),
                markdown_cell(
                    ",".join(
                        str(value) for value in as_list(hypothesis.get("failed_probe_ids"))
                    )
                ),
                markdown_cell(str(next_action.get("open_artifact") or "-")),
            )
        )

    lines.extend(
        [
            "",
            "## Health Counts",
            "",
            "| Health | Count | Scenarios |",
            "| --- | --- | --- |",
        ]
    )
    for row in as_list(analysis.get("health_counts")):
        if isinstance(row, dict):
            lines.append(
                "| {} | {} | {} |".format(
                    row.get("health"),
                    row.get("count"),
                    markdown_cell(
                        ",".join(str(value) for value in as_list(row.get("scenario_ids")))
                    ),
                )
            )

    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(analysis.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if analysis.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(analysis.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_observability_analysis(
    suite_dir: Path, debug_index_summary: dict[str, Any] | None, repo_root: Path
) -> dict[str, Any]:
    analysis = build_observability_analysis(suite_dir, debug_index_summary, repo_root)
    artifacts = as_dict(analysis.get("artifacts"))
    json_path = Path(str(artifacts["observability_analysis_json"]))
    report_path = Path(str(artifacts["observability_analysis_report"]))
    json_path.write_text(json.dumps(analysis, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_observability_analysis_markdown(report_path, analysis)
    return analysis


def scenarios_by_id(manifest: dict[str, Any]) -> dict[str, dict[str, Any]]:
    scenarios = manifest.get("scenarios")
    if not isinstance(scenarios, list):
        return {}
    return {
        scenario.get("id"): scenario
        for scenario in scenarios
        if isinstance(scenario, dict) and isinstance(scenario.get("id"), str)
    }


def select_replay_scenario_id(
    manifest: dict[str, Any], observer: dict[str, Any], requested_scenario_id: str | None
) -> str | None:
    if requested_scenario_id:
        return requested_scenario_id
    actions = observer.get("next_actions")
    if isinstance(actions, list) and actions:
        first_action = actions[0]
        if isinstance(first_action, dict) and isinstance(first_action.get("scenario_id"), str):
            return first_action["scenario_id"]
    baseline = manifest.get("baseline_scenario_id")
    return baseline if isinstance(baseline, str) else None


def effective_replay_args(
    source_args: list[Any], cargo: str, replay_bundle_dir: Path
) -> tuple[list[str], str | None]:
    if not all(isinstance(arg, str) for arg in source_args):
        return [], "replay_args must be a string array"
    args = list(source_args)
    if args and args[0] == "cargo":
        args[0] = cargo
    try:
        bundle_flag_index = args.index("--bundle-dir")
    except ValueError:
        return [], "replay_args are missing --bundle-dir"
    if bundle_flag_index + 1 >= len(args):
        return [], "replay_args --bundle-dir is missing a value"
    args[bundle_flag_index + 1] = str(replay_bundle_dir)
    return args, None


def replay_artifact_paths(replay_dir: Path, replay_bundle_dir: Path) -> dict[str, str]:
    return {
        "replay_run_json": str(replay_dir / "replay-run.json"),
        "replay_run_report": str(replay_dir / "replay-run.md"),
        "bundle_dir": str(replay_bundle_dir),
        "bundle_manifest": str(replay_bundle_dir / "manifest.json"),
        "bundle_triage_json": str(replay_bundle_dir / "triage.json"),
        "bundle_telemetry_json": str(replay_bundle_dir / "telemetry.json"),
        "bundle_report": str(replay_bundle_dir / "report.md"),
        "bundle_rom": str(replay_bundle_dir / "diagnostic.nes"),
    }


def build_replay_summary(
    suite_dir: Path,
    replay_dir: Path,
    replay_bundle_dir: Path,
    requested_scenario_id: str | None,
    cargo: str,
    repo_root: Path,
) -> dict[str, Any]:
    manifest = load_json(suite_dir / "scenario-suite.json")
    observer = load_json(suite_dir / "scenario-suite-observer.json")
    scenario_id = select_replay_scenario_id(manifest, observer, requested_scenario_id)
    scenario = scenarios_by_id(manifest).get(scenario_id or "")
    artifacts = replay_artifact_paths(replay_dir, replay_bundle_dir)

    base_summary: dict[str, Any] = {
        "replay_run_schema_version": REPLAY_RUN_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": "failed",
        "recommended_exit_code": 1,
        "git": git_metadata(repo_root),
        "suite_dir": str(suite_dir),
        "scenario_id": scenario_id,
        "requested_scenario_id": requested_scenario_id,
        "artifacts": artifacts,
        "ai_handoff": [
            "Start with bundle_triage_json for the focused replay result.",
            "Use bundle_telemetry_json only after the replay triage focus is insufficient.",
            "If exit_code_matches_expected is false, inspect command stderr_tail before emulator telemetry.",
        ],
    }

    if not isinstance(scenario, dict):
        base_summary["error"] = f"scenario not found in scenario-suite.json: {scenario_id}"
        return base_summary

    source_args = scenario.get("replay_args")
    if not isinstance(source_args, list):
        base_summary["error"] = f"scenario {scenario_id} has no replay_args array"
        return base_summary

    effective_args, replay_arg_error = effective_replay_args(source_args, cargo, replay_bundle_dir)
    base_summary["source_replay_args"] = source_args
    base_summary["effective_replay_args"] = effective_args
    if replay_arg_error is not None:
        base_summary["error"] = replay_arg_error
        return base_summary

    replay_dir.mkdir(parents=True, exist_ok=True)
    replay_command = run_command(effective_args, repo_root)
    bundle_manifest = load_json(replay_bundle_dir / "manifest.json")
    triage = load_json(replay_bundle_dir / "triage.json")
    debug_focus = triage.get("debug_focus") if isinstance(triage.get("debug_focus"), dict) else {}

    expected_exit_code = scenario.get("expected_runner_exit_code")
    expected_focus_test_id = scenario.get("expected_focus_test_id")
    expected_focus_domain = scenario.get("expected_focus_domain")
    expected_health = scenario.get("expected_health")
    actual_focus_test_id = debug_focus.get("focus_test_id")
    actual_focus_domain = debug_focus.get("focus_domain")
    actual_health = triage.get("health")
    exit_code_matches_expected = replay_command.get("exit_code") == expected_exit_code
    health_matches_expected = actual_health == expected_health
    focus_test_matches_expected = (
        True if expected_focus_test_id is None else actual_focus_test_id == expected_focus_test_id
    )
    focus_domain_matches_expected = (
        True if expected_focus_domain is None else actual_focus_domain == expected_focus_domain
    )
    required_artifacts_present = all(
        Path(path).is_file()
        for name, path in artifacts.items()
        if name
        in {
            "bundle_manifest",
            "bundle_triage_json",
            "bundle_telemetry_json",
            "bundle_report",
            "bundle_rom",
        }
    )
    replay_passed = (
        exit_code_matches_expected
        and health_matches_expected
        and focus_test_matches_expected
        and focus_domain_matches_expected
        and required_artifacts_present
    )

    base_summary.update(
        {
            "status": "passed" if replay_passed else "failed",
            "recommended_exit_code": 0 if replay_passed else 1,
            "scenario_title": scenario.get("title"),
            "scenario_purpose": scenario.get("purpose"),
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
            "required_artifacts_present": required_artifacts_present,
            "bundle_passed": bundle_manifest.get("passed"),
            "command": {"name": "replay_scenario", **replay_command},
        }
    )
    return base_summary


def write_replay_markdown(path: Path, summary: dict[str, Any]) -> None:
    lines = [
        "# Diagnostic Scenario Replay",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Git commit | {summary.get('git', {}).get('short_commit', '')} |",
        f"| Scenario | {summary.get('scenario_id')} |",
        f"| Expected exit code | {summary.get('expected_runner_exit_code')} |",
        f"| Actual exit code | {summary.get('command', {}).get('exit_code')} |",
        f"| Exit code matches expected | {summary.get('exit_code_matches_expected')} |",
        f"| Expected health | {summary.get('expected_health')} |",
        f"| Actual health | {summary.get('actual_health')} |",
        f"| Expected focus domain | {summary.get('expected_focus_domain')} |",
        f"| Actual focus domain | {summary.get('actual_focus_domain')} |",
        f"| Required artifacts present | {summary.get('required_artifacts_present')} |",
        "",
        "## Replay Args",
        "",
        "| Kind | Args |",
        "| --- | --- |",
        f"| Source | {markdown_cell(' '.join(summary.get('source_replay_args', [])))} |",
        f"| Effective | {markdown_cell(' '.join(summary.get('effective_replay_args', [])))} |",
        "",
        "## Artifacts",
        "",
        "| Name | Path |",
        "| --- | --- |",
    ]
    for name, artifact_path in summary.get("artifacts", {}).items():
        lines.append(f"| {name} | {artifact_path} |")
    lines.extend(["", "## AI Handoff", ""])
    for instruction in summary.get("ai_handoff", []):
        lines.append(f"- {instruction}")
    if summary.get("error"):
        lines.extend(["", "## Error", "", str(summary["error"])])
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def build_run_summary(
    suite_dir: Path,
    summary_json: Path,
    summary_md: Path,
    generate_command: dict[str, Any],
    verify_command: dict[str, Any] | None,
    verification_summary: dict[str, Any],
    debug_index_summary: dict[str, Any] | None,
    observability_analysis: dict[str, Any] | None,
    replay_summary: dict[str, Any] | None,
    repo_root: Path,
) -> dict[str, Any]:
    commands = [
        {"name": "generate_scenario_suite", **generate_command},
    ]
    if verify_command is not None:
        commands.append({"name": "verify_scenario_suite", **verify_command})
    if replay_summary and isinstance(replay_summary.get("command"), dict):
        commands.append(replay_summary["command"])

    status = "passed"
    if command_failed(generate_command) or verify_command is None or command_failed(verify_command):
        status = "failed"
    if debug_index_summary and debug_index_summary.get("status") != "passed":
        status = "failed"
    if observability_analysis and observability_analysis.get("status") != "passed":
        status = "failed"
    if replay_summary and replay_summary.get("status") != "passed":
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
        "debug_index": debug_index_summary,
        "analysis": observability_analysis,
        "replay": replay_summary,
        "suite": suite,
        "artifacts": artifact_paths(
            suite_dir,
            summary_json,
            summary_md,
            replay_summary,
            debug_index_summary,
            observability_analysis,
        ),
        "ai_handoff": [
            "Start with suite.first_next_action and open its primary_artifact.",
            "Use analysis.ranked_hypotheses for ranked subsystem/domain hypotheses across the suite.",
            "Use debug_index.artifacts.debug_index_jsonl for one-row-per-scenario routing before raw telemetry.",
            "Use replay.artifacts.bundle_triage_json for the focused replay evidence of the selected scenario.",
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
    debug_index = summary.get("debug_index") or {}
    lines.extend(
        [
            "",
            "## Debug Index",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Status | {debug_index.get('status', '-')} |",
            f"| Entries | {debug_index.get('entry_count', '-')} |",
            f"| JSONL | {debug_index.get('artifacts', {}).get('debug_index_jsonl', '-')} |",
            f"| Report | {debug_index.get('artifacts', {}).get('debug_index_report', '-')} |",
        ]
    )
    analysis = summary.get("analysis") or {}
    top_hypothesis = as_dict(as_list(analysis.get("ranked_hypotheses"))[0]) if analysis else {}
    lines.extend(
        [
            "",
            "## Observability Analysis",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Status | {analysis.get('status', '-')} |",
            f"| Hypotheses | {analysis.get('hypothesis_count', '-')} |",
            f"| Top focus domain | {top_hypothesis.get('focus_domain', '-')} |",
            f"| JSON | {analysis.get('artifacts', {}).get('observability_analysis_json', '-')} |",
            f"| Report | {analysis.get('artifacts', {}).get('observability_analysis_report', '-')} |",
        ]
    )
    replay = summary.get("replay") or {}
    lines.extend(
        [
            "",
            "## Replay",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Status | {replay.get('status', '-')} |",
            f"| Scenario | {replay.get('scenario_id', '-')} |",
            f"| Expected exit code | {replay.get('expected_runner_exit_code', '-')} |",
            f"| Actual exit code | {replay.get('command', {}).get('exit_code', '-')} |",
            f"| Exit code matches expected | {replay.get('exit_code_matches_expected', '-')} |",
            f"| Bundle triage | {replay.get('artifacts', {}).get('bundle_triage_json', '-')} |",
        ]
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
        "--replay-scenario",
        help=(
            "Scenario id to replay after suite verification. "
            "Defaults to the observer's first next action, then the baseline scenario."
        ),
    )
    parser.add_argument(
        "--replay-output-dir",
        type=Path,
        help=(
            "Directory for focused replay evidence. "
            "Defaults to <suite-dir>/replay-runs/<scenario-id>."
        ),
    )
    parser.add_argument(
        "--skip-replay",
        action="store_true",
        help="Skip the focused scenario replay after verifying the suite.",
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
    debug_index_summary: dict[str, Any] | None = None
    observability_analysis: dict[str, Any] | None = None
    replay_summary: dict[str, Any] | None = None
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
            debug_index_summary = write_debug_index(suite_dir)
            observability_analysis = write_observability_analysis(
                suite_dir, debug_index_summary, repo_root
            )
            if not args.skip_replay:
                manifest = load_json(suite_dir / "scenario-suite.json")
                observer = load_json(suite_dir / "scenario-suite-observer.json")
                replay_scenario_id = select_replay_scenario_id(
                    manifest, observer, args.replay_scenario
                )
                replay_dir = args.replay_output_dir or (
                    suite_dir / "replay-runs" / str(replay_scenario_id or "unknown")
                )
                replay_bundle_dir = replay_dir / "bundle"
                replay_summary = build_replay_summary(
                    suite_dir,
                    replay_dir,
                    replay_bundle_dir,
                    args.replay_scenario,
                    args.cargo,
                    repo_root,
                )
                replay_json_path = Path(replay_summary["artifacts"]["replay_run_json"])
                replay_md_path = Path(replay_summary["artifacts"]["replay_run_report"])
                replay_json_path.parent.mkdir(parents=True, exist_ok=True)
                replay_json_path.write_text(
                    json.dumps(replay_summary, indent=2, sort_keys=True) + "\n",
                    encoding="utf-8",
                )
                write_replay_markdown(replay_md_path, replay_summary)

    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_md.parent.mkdir(parents=True, exist_ok=True)
    summary = build_run_summary(
        suite_dir,
        summary_json,
        summary_md,
        generate_command,
        verify_command,
        verification_summary,
        debug_index_summary,
        observability_analysis,
        replay_summary,
        repo_root,
    )
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_md, summary)

    if command_failed(generate_command):
        print_failed_command(generate_command)
    if verify_command is not None and command_failed(verify_command):
        print_failed_command(verify_command)
    if replay_summary and replay_summary.get("status") == "failed":
        replay_command = replay_summary.get("command")
        if isinstance(replay_command, dict):
            print_failed_command(replay_command)

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        debug_index = summary.get("debug_index") or {}
        debug_note = ""
        if debug_index:
            debug_note = (
                f" debug_index={debug_index.get('entry_count')}:{debug_index.get('status')}"
            )
        analysis = summary.get("analysis") or {}
        analysis_note = ""
        if analysis:
            analysis_note = (
                f" analysis={analysis.get('hypothesis_count')}:{analysis.get('status')}"
            )
        replay = summary.get("replay") or {}
        replay_note = ""
        if replay:
            replay_note = (
                f" replay={replay.get('scenario_id')}:{replay.get('status')}"
            )
        print(
            "Diagnostic observability run "
            f"{summary['status']}: suite={suite_dir} "
            f"summary_json={summary_json} summary_report={summary_md}"
            f"{debug_note}{analysis_note}{replay_note}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
