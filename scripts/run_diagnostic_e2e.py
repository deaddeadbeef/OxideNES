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
        "diagnostic_ai_index_json": str(suite_dir / "diagnostic-ai-observability-index.json"),
        "diagnostic_ai_index_report": str(suite_dir / "diagnostic-ai-observability-index.md"),
        "diagnostic_ai_query_smoke_json": str(suite_dir / "diagnostic-ai-query-smoke.json"),
        "diagnostic_ai_query_smoke_report": str(suite_dir / "diagnostic-ai-query-smoke.md"),
        "diagnostic_ai_diagnosis_smoke_json": str(
            suite_dir / "diagnostic-ai-diagnosis-smoke.json"
        ),
        "diagnostic_ai_diagnosis_smoke_report": str(
            suite_dir / "diagnostic-ai-diagnosis-smoke.md"
        ),
        "diagnostic_ai_fix_handoff_smoke_json": str(
            suite_dir / "diagnostic-ai-fix-handoff-smoke.json"
        ),
        "diagnostic_ai_fix_handoff_smoke_report": str(
            suite_dir / "diagnostic-ai-fix-handoff-smoke.md"
        ),
        "diagnostic_ai_route_matrix_json": str(suite_dir / "diagnostic-ai-route-matrix.json"),
        "diagnostic_ai_route_matrix_report": str(suite_dir / "diagnostic-ai-route-matrix.md"),
        "diagnostic_ai_debug_packet_json": str(suite_dir / "diagnostic-ai-debug-packet.json"),
        "diagnostic_ai_debug_packet_report": str(suite_dir / "diagnostic-ai-debug-packet.md"),
        "diagnostic_ai_debug_packet_verification_json": str(
            suite_dir / "diagnostic-ai-debug-packet-verification.json"
        ),
        "diagnostic_ai_debug_packet_verification_report": str(
            suite_dir / "diagnostic-ai-debug-packet-verification.md"
        ),
        "diagnostic_ai_debug_packet_dir": str(suite_dir / "ai-debug-packet"),
        "diagnostic_ai_debug_packet_matrix_json": str(
            suite_dir / "diagnostic-ai-debug-packet-matrix.json"
        ),
        "diagnostic_ai_debug_packet_matrix_report": str(
            suite_dir / "diagnostic-ai-debug-packet-matrix.md"
        ),
        "diagnostic_ai_debug_packet_matrix_dir": str(suite_dir / "ai-debug-packet-matrix"),
        "diagnostic_ai_localization_eval_json": str(
            suite_dir / "diagnostic-ai-localization-eval.json"
        ),
        "diagnostic_ai_localization_eval_report": str(
            suite_dir / "diagnostic-ai-localization-eval.md"
        ),
        "diagnostic_ai_artifact_verification_json": str(
            suite_dir / "diagnostic-ai-artifact-verification.json"
        ),
        "diagnostic_ai_artifact_verification_report": str(
            suite_dir / "diagnostic-ai-artifact-verification.md"
        ),
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
    ai_index = load_json(suite_dir / "diagnostic-ai-observability-index.json")
    ai_query_smoke = load_json(suite_dir / "diagnostic-ai-query-smoke.json")
    ai_diagnosis_smoke = load_json(suite_dir / "diagnostic-ai-diagnosis-smoke.json")
    ai_fix_handoff_smoke = load_json(suite_dir / "diagnostic-ai-fix-handoff-smoke.json")
    ai_route_matrix = load_json(suite_dir / "diagnostic-ai-route-matrix.json")
    ai_debug_packet = load_json(suite_dir / "diagnostic-ai-debug-packet.json")
    ai_debug_packet_verification = load_json(
        suite_dir / "diagnostic-ai-debug-packet-verification.json"
    )
    ai_debug_packet_matrix = load_json(suite_dir / "diagnostic-ai-debug-packet-matrix.json")
    ai_localization_eval = load_json(suite_dir / "diagnostic-ai-localization-eval.json")
    ai_artifact_verification = load_json(
        suite_dir / "diagnostic-ai-artifact-verification.json"
    )

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
    if ai_index.get("status") != "passed":
        errors.append("diagnostic AI index status is not passed")
    if ai_query_smoke.get("status") != "passed":
        errors.append("diagnostic AI query smoke status is not passed")
    if ai_diagnosis_smoke.get("status") != "passed":
        errors.append("diagnostic AI diagnosis smoke status is not passed")
    if ai_fix_handoff_smoke.get("status") != "passed":
        errors.append("diagnostic AI fix handoff smoke status is not passed")
    if ai_route_matrix.get("status") != "passed":
        errors.append("diagnostic AI route matrix status is not passed")
    if ai_debug_packet.get("status") != "passed":
        errors.append("diagnostic AI debug packet status is not passed")
    if ai_debug_packet_verification.get("status") != "passed":
        errors.append("diagnostic AI debug packet verification status is not passed")
    if ai_debug_packet_matrix.get("status") != "passed":
        errors.append("diagnostic AI debug packet matrix status is not passed")
    if ai_localization_eval.get("status") != "passed":
        errors.append("diagnostic AI localization evaluation status is not passed")
    if ai_artifact_verification.get("status") != "passed":
        errors.append("diagnostic AI artifact verification status is not passed")
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
        "ai_index": {
            "status": ai_index.get("status"),
            "scenario_cards": as_dict(ai_index.get("summary")).get("scenario_count"),
            "focus_domains": as_dict(ai_index.get("summary")).get("focus_domain_count"),
            "failed_probe_ids": as_dict(ai_index.get("summary")).get("failed_probe_id_count"),
            "only_happy_paths": as_dict(ai_index.get("summary")).get("only_happy_paths"),
            "top_route_focus_domain": as_dict(ai_index.get("summary")).get(
                "top_route_focus_domain"
            ),
        },
        "ai_query": {
            "status": ai_query_smoke.get("status"),
            "top_route_id": as_dict(ai_query_smoke.get("summary")).get("top_route_id"),
            "top_route_scenario": as_dict(ai_query_smoke.get("summary")).get(
                "top_route_scenario"
            ),
            "top_route_focus_domain": as_dict(ai_query_smoke.get("summary")).get(
                "top_route_focus_domain"
            ),
            "top_route_probe": as_dict(ai_query_smoke.get("summary")).get("top_route_probe"),
            "check_count": len(as_list(ai_query_smoke.get("checks"))),
        },
        "ai_diagnosis": {
            "status": ai_diagnosis_smoke.get("status"),
            "route_id": as_dict(ai_diagnosis_smoke.get("selection")).get("route_id"),
            "scenario_id": as_dict(ai_diagnosis_smoke.get("selection")).get("scenario_id"),
            "focus_domain": as_dict(ai_diagnosis_smoke.get("selection")).get("focus_domain"),
            "probe_id": as_dict(ai_diagnosis_smoke.get("selection")).get("probe_id"),
            "replay_status": as_dict(ai_diagnosis_smoke.get("route_check")).get(
                "replay_status"
            ),
            "tests_status": as_dict(ai_diagnosis_smoke.get("route_check")).get("tests_status"),
            "test_command_count": as_dict(ai_diagnosis_smoke.get("route_check")).get(
                "test_command_count"
            ),
            "stop_condition_count": len(as_list(ai_diagnosis_smoke.get("stop_conditions"))),
        },
        "ai_fix_handoff": {
            "status": ai_fix_handoff_smoke.get("status"),
            "route_id": as_dict(ai_fix_handoff_smoke.get("selection")).get("route_id"),
            "scenario_id": as_dict(ai_fix_handoff_smoke.get("selection")).get("scenario_id"),
            "focus_domain": as_dict(ai_fix_handoff_smoke.get("selection")).get(
                "focus_domain"
            ),
            "probe_id": as_dict(ai_fix_handoff_smoke.get("selection")).get("probe_id"),
            "source_file_count": as_dict(ai_fix_handoff_smoke.get("source_scan")).get(
                "source_file_count"
            ),
            "source_match_count": as_dict(ai_fix_handoff_smoke.get("source_scan")).get(
                "source_match_count"
            ),
            "test_file_count": as_dict(ai_fix_handoff_smoke.get("test_scan")).get(
                "test_file_count"
            ),
            "test_match_count": as_dict(ai_fix_handoff_smoke.get("test_scan")).get(
                "test_match_count"
            ),
            "narrow_test_command_count": len(
                as_list(
                    as_dict(ai_fix_handoff_smoke.get("fix_commands")).get(
                        "narrow_test_commands"
                    )
                )
            ),
            "stop_condition_count": len(as_list(ai_fix_handoff_smoke.get("stop_conditions"))),
        },
        "ai_route_matrix": {
            "status": ai_route_matrix.get("status"),
            "route_count": as_dict(ai_route_matrix.get("summary")).get("route_count"),
            "passed_route_count": as_dict(ai_route_matrix.get("summary")).get(
                "passed_route_count"
            ),
            "failed_route_count": as_dict(ai_route_matrix.get("summary")).get(
                "failed_route_count"
            ),
            "diagnosis_failure_count": as_dict(ai_route_matrix.get("summary")).get(
                "diagnosis_failure_count"
            ),
            "fix_handoff_failure_count": as_dict(ai_route_matrix.get("summary")).get(
                "fix_handoff_failure_count"
            ),
            "replay_failure_count": as_dict(ai_route_matrix.get("summary")).get(
                "replay_failure_count"
            ),
            "test_failure_count": as_dict(ai_route_matrix.get("summary")).get(
                "test_failure_count"
            ),
            "source_match_failure_count": as_dict(ai_route_matrix.get("summary")).get(
                "source_match_failure_count"
            ),
            "test_match_failure_count": as_dict(ai_route_matrix.get("summary")).get(
                "test_match_failure_count"
            ),
            "missing_artifact_count": as_dict(ai_route_matrix.get("summary")).get(
                "missing_artifact_count"
            ),
        },
        "ai_debug_packet": {
            "status": ai_debug_packet.get("status"),
            "route_id": as_dict(ai_debug_packet.get("selection")).get("route_id"),
            "scenario_id": as_dict(ai_debug_packet.get("selection")).get("scenario_id"),
            "focus_domain": as_dict(ai_debug_packet.get("selection")).get(
                "focus_domain"
            ),
            "probe_id": as_dict(ai_debug_packet.get("selection")).get("probe_id"),
            "file_count": as_dict(ai_debug_packet.get("packet_manifest")).get(
                "file_count"
            ),
            "missing_required_file_count": as_dict(
                ai_debug_packet.get("packet_manifest")
            ).get("missing_required_file_count"),
            "source_window_count": as_dict(ai_debug_packet.get("context_summary")).get(
                "source_window_count"
            ),
            "test_window_count": as_dict(ai_debug_packet.get("context_summary")).get(
                "test_window_count"
            ),
        },
        "ai_debug_packet_verification": {
            "status": ai_debug_packet_verification.get("status"),
            "route_id": as_dict(ai_debug_packet_verification.get("selection")).get(
                "route_id"
            ),
            "scenario_id": as_dict(ai_debug_packet_verification.get("selection")).get(
                "scenario_id"
            ),
            "focus_domain": as_dict(ai_debug_packet_verification.get("selection")).get(
                "focus_domain"
            ),
            "probe_id": as_dict(ai_debug_packet_verification.get("selection")).get(
                "probe_id"
            ),
            "check_count": as_dict(ai_debug_packet_verification.get("summary")).get(
                "check_count"
            ),
            "passed_check_count": as_dict(
                ai_debug_packet_verification.get("summary")
            ).get("passed_check_count"),
            "packet_file_count": as_dict(ai_debug_packet_verification.get("summary")).get(
                "packet_file_count"
            ),
            "digest_mismatch_count": as_dict(
                ai_debug_packet_verification.get("summary")
            ).get("digest_mismatch_count"),
            "source_window_count": as_dict(
                ai_debug_packet_verification.get("summary")
            ).get("source_window_count"),
            "test_window_count": as_dict(
                ai_debug_packet_verification.get("summary")
            ).get("test_window_count"),
        },
        "ai_debug_packet_matrix": {
            "status": ai_debug_packet_matrix.get("status"),
            "route_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "route_count"
            ),
            "passed_route_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "passed_route_count"
            ),
            "failed_route_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "failed_route_count"
            ),
            "packet_failure_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "packet_failure_count"
            ),
            "packet_verification_failure_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("packet_verification_failure_count"),
            "identity_failure_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "identity_failure_count"
            ),
            "context_failure_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "context_failure_count"
            ),
            "stop_condition_failure_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("stop_condition_failure_count"),
            "missing_artifact_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "missing_artifact_count"
            ),
            "packet_file_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "packet_file_count"
            ),
            "packet_verifier_check_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("packet_verifier_check_count"),
            "packet_verifier_passed_check_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("packet_verifier_passed_check_count"),
            "packet_verifier_digest_mismatch_count": as_dict(
                ai_debug_packet_matrix.get("summary")
            ).get("packet_verifier_digest_mismatch_count"),
            "source_window_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "source_window_count"
            ),
            "test_window_count": as_dict(ai_debug_packet_matrix.get("summary")).get(
                "test_window_count"
            ),
        },
        "ai_localization_eval": {
            "status": ai_localization_eval.get("status"),
            "scenario_count": as_dict(ai_localization_eval.get("summary")).get(
                "scenario_count"
            ),
            "passed_scenario_count": as_dict(ai_localization_eval.get("summary")).get(
                "passed_scenario_count"
            ),
            "negative_fixture_count": as_dict(ai_localization_eval.get("summary")).get(
                "negative_fixture_count"
            ),
            "only_happy_paths": as_dict(ai_localization_eval.get("summary")).get(
                "only_happy_paths"
            ),
            "focus_domain_match_count": as_dict(ai_localization_eval.get("summary")).get(
                "focus_domain_match_count"
            ),
            "route_ready_count": as_dict(ai_localization_eval.get("summary")).get(
                "route_ready_count"
            ),
            "packet_self_verified_count": as_dict(
                ai_localization_eval.get("summary")
            ).get("packet_self_verified_count"),
            "source_anchor_scenario_count": as_dict(
                ai_localization_eval.get("summary")
            ).get("source_anchor_scenario_count"),
            "test_anchor_scenario_count": as_dict(
                ai_localization_eval.get("summary")
            ).get("test_anchor_scenario_count"),
            "average_score": as_dict(ai_localization_eval.get("summary")).get(
                "average_score"
            ),
            "minimum_score": as_dict(ai_localization_eval.get("summary")).get(
                "minimum_score"
            ),
        },
        "ai_artifact_verification": {
            "status": ai_artifact_verification.get("status"),
            "check_count": as_dict(ai_artifact_verification.get("summary")).get("check_count"),
            "passed_check_count": as_dict(ai_artifact_verification.get("summary")).get(
                "passed_check_count"
            ),
            "artifact_count": as_dict(ai_artifact_verification.get("summary")).get(
                "artifact_count"
            ),
            "missing_artifact_count": as_dict(
                ai_artifact_verification.get("summary")
            ).get("missing_artifact_count"),
            "e2e_report_checked": as_dict(ai_artifact_verification.get("summary")).get(
                "e2e_report_checked"
            ),
            "route_id": as_dict(ai_artifact_verification.get("summary")).get(
                "top_route_id"
            ),
            "scenario_id": as_dict(ai_artifact_verification.get("summary")).get(
                "top_route_scenario"
            ),
            "focus_domain": as_dict(ai_artifact_verification.get("summary")).get(
                "top_route_focus_domain"
            ),
            "probe_id": as_dict(ai_artifact_verification.get("summary")).get(
                "top_route_probe"
            ),
            "automation_readiness_status": as_dict(
                ai_artifact_verification.get("summary")
            ).get("automation_readiness_status"),
            "automation_ready_route_count": as_dict(
                ai_artifact_verification.get("summary")
            ).get("automation_ready_route_count"),
            "automation_route_count": as_dict(ai_artifact_verification.get("summary")).get(
                "automation_route_count"
            ),
            "automation_not_ready_route_count": as_dict(
                ai_artifact_verification.get("summary")
            ).get("automation_not_ready_route_count"),
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
            "If status is passed, use diagnostic_ai_index_json as the compact joined index before opening larger artifacts.",
            "Use diagnostic_ai_query_smoke_json to prove the AI index supports deterministic route, scenario, probe, and coverage queries.",
            "Use diagnostic_ai_diagnosis_smoke_json to prove an AI-selected route can regenerate replay evidence and mapped narrow-test results.",
            "Use diagnostic_ai_fix_handoff_smoke_json to resolve the selected route into source/test line anchors and fix-loop commands.",
            "Use diagnostic_ai_route_matrix_json to prove every AI route can regenerate diagnosis and fix-handoff artifacts.",
            "Use diagnostic_ai_debug_packet_json when an automated debugger needs one relocatable packet for the selected route.",
            "Use diagnostic_ai_debug_packet_verification_json to prove the selected packet is valid from packet-local files and digests.",
            "Use diagnostic_ai_debug_packet_matrix_json to prove every AI route can be packaged into a relocatable debug packet with source/test context.",
            "Use diagnostic_ai_localization_eval_json to score whether expected health, focus-domain, route, source/test, and packet evidence localize across the scenario corpus.",
            "Use diagnostic_ai_artifact_verification_json to prove the AI-facing artifact graph is internally consistent before automated fixes.",
            "Use diagnostic_ai_artifact_verification_json automation_readiness when an automated debugger needs one compact route-by-route readiness map.",
            "Use top_route for the highest-signal failure and scenario_dossiers_json for scenario-id-first debugging.",
            "Use route_evidence_verification_json to prove the investigation routes can regenerate focused replay evidence.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    observability = as_dict(summary.get("observability"))
    routes = as_dict(summary.get("routes"))
    ai_index = as_dict(summary.get("ai_index"))
    ai_query = as_dict(summary.get("ai_query"))
    ai_diagnosis = as_dict(summary.get("ai_diagnosis"))
    ai_fix_handoff = as_dict(summary.get("ai_fix_handoff"))
    ai_route_matrix = as_dict(summary.get("ai_route_matrix"))
    ai_debug_packet = as_dict(summary.get("ai_debug_packet"))
    ai_debug_packet_verification = as_dict(summary.get("ai_debug_packet_verification"))
    ai_debug_packet_matrix = as_dict(summary.get("ai_debug_packet_matrix"))
    ai_localization_eval = as_dict(summary.get("ai_localization_eval"))
    ai_artifact_verification = as_dict(summary.get("ai_artifact_verification"))
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
        "## AI Index",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_index.get('status')} |",
        f"| Scenario cards | {ai_index.get('scenario_cards')} |",
        f"| Focus domains | {ai_index.get('focus_domains')} |",
        f"| Failed probe ids | {ai_index.get('failed_probe_ids')} |",
        f"| Only happy paths | {ai_index.get('only_happy_paths')} |",
        f"| Top route focus domain | {ai_index.get('top_route_focus_domain')} |",
        "",
        "## AI Query",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_query.get('status')} |",
        f"| Query smoke checks | {ai_query.get('check_count')} |",
        f"| Top route id | {ai_query.get('top_route_id')} |",
        f"| Top route scenario | {ai_query.get('top_route_scenario')} |",
        f"| Top route focus domain | {ai_query.get('top_route_focus_domain')} |",
        f"| Top route probe | {ai_query.get('top_route_probe')} |",
        "",
        "## AI Diagnosis",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_diagnosis.get('status')} |",
        f"| Route id | {ai_diagnosis.get('route_id')} |",
        f"| Scenario | {ai_diagnosis.get('scenario_id')} |",
        f"| Focus domain | {ai_diagnosis.get('focus_domain')} |",
        f"| Probe | {ai_diagnosis.get('probe_id')} |",
        f"| Replay status | {ai_diagnosis.get('replay_status')} |",
        f"| Tests status | {ai_diagnosis.get('tests_status')} |",
        f"| Test commands | {ai_diagnosis.get('test_command_count')} |",
        f"| Stop conditions | {ai_diagnosis.get('stop_condition_count')} |",
        "",
        "## AI Fix Handoff",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_fix_handoff.get('status')} |",
        f"| Route id | {ai_fix_handoff.get('route_id')} |",
        f"| Scenario | {ai_fix_handoff.get('scenario_id')} |",
        f"| Focus domain | {ai_fix_handoff.get('focus_domain')} |",
        f"| Probe | {ai_fix_handoff.get('probe_id')} |",
        f"| Source files | {ai_fix_handoff.get('source_file_count')} |",
        f"| Source matches | {ai_fix_handoff.get('source_match_count')} |",
        f"| Test files | {ai_fix_handoff.get('test_file_count')} |",
        f"| Test matches | {ai_fix_handoff.get('test_match_count')} |",
        f"| Narrow test commands | {ai_fix_handoff.get('narrow_test_command_count')} |",
        f"| Stop conditions | {ai_fix_handoff.get('stop_condition_count')} |",
        "",
        "## AI Route Matrix",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_route_matrix.get('status')} |",
        f"| Routes | {ai_route_matrix.get('passed_route_count')}/{ai_route_matrix.get('route_count')} |",
        f"| Failed routes | {ai_route_matrix.get('failed_route_count')} |",
        f"| Diagnosis failures | {ai_route_matrix.get('diagnosis_failure_count')} |",
        f"| Fix-handoff failures | {ai_route_matrix.get('fix_handoff_failure_count')} |",
        f"| Replay failures | {ai_route_matrix.get('replay_failure_count')} |",
        f"| Test failures | {ai_route_matrix.get('test_failure_count')} |",
        f"| Source-match failures | {ai_route_matrix.get('source_match_failure_count')} |",
        f"| Test-match failures | {ai_route_matrix.get('test_match_failure_count')} |",
        f"| Missing artifacts | {ai_route_matrix.get('missing_artifact_count')} |",
        "",
        "## AI Debug Packet",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_debug_packet.get('status')} |",
        f"| Route id | {ai_debug_packet.get('route_id')} |",
        f"| Scenario | {ai_debug_packet.get('scenario_id')} |",
        f"| Focus domain | {ai_debug_packet.get('focus_domain')} |",
        f"| Probe | {ai_debug_packet.get('probe_id')} |",
        f"| Packet files | {ai_debug_packet.get('file_count')} |",
        f"| Missing required files | {ai_debug_packet.get('missing_required_file_count')} |",
        f"| Source windows | {ai_debug_packet.get('source_window_count')} |",
        f"| Test windows | {ai_debug_packet.get('test_window_count')} |",
        "",
        "## AI Debug Packet Verification",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_debug_packet_verification.get('status')} |",
        f"| Route id | {ai_debug_packet_verification.get('route_id')} |",
        f"| Scenario | {ai_debug_packet_verification.get('scenario_id')} |",
        f"| Focus domain | {ai_debug_packet_verification.get('focus_domain')} |",
        f"| Probe | {ai_debug_packet_verification.get('probe_id')} |",
        f"| Checks | {ai_debug_packet_verification.get('passed_check_count')}/{ai_debug_packet_verification.get('check_count')} |",
        f"| Packet files | {ai_debug_packet_verification.get('packet_file_count')} |",
        f"| Digest mismatches | {ai_debug_packet_verification.get('digest_mismatch_count')} |",
        f"| Source windows | {ai_debug_packet_verification.get('source_window_count')} |",
        f"| Test windows | {ai_debug_packet_verification.get('test_window_count')} |",
        "",
        "## AI Debug Packet Matrix",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_debug_packet_matrix.get('status')} |",
        f"| Routes | {ai_debug_packet_matrix.get('passed_route_count')}/{ai_debug_packet_matrix.get('route_count')} |",
        f"| Failed routes | {ai_debug_packet_matrix.get('failed_route_count')} |",
        f"| Packet failures | {ai_debug_packet_matrix.get('packet_failure_count')} |",
        f"| Packet verification failures | {ai_debug_packet_matrix.get('packet_verification_failure_count')} |",
        f"| Identity failures | {ai_debug_packet_matrix.get('identity_failure_count')} |",
        f"| Context failures | {ai_debug_packet_matrix.get('context_failure_count')} |",
        f"| Stop-condition failures | {ai_debug_packet_matrix.get('stop_condition_failure_count')} |",
        f"| Missing artifacts | {ai_debug_packet_matrix.get('missing_artifact_count')} |",
        f"| Packet files | {ai_debug_packet_matrix.get('packet_file_count')} |",
        f"| Packet verifier checks | {ai_debug_packet_matrix.get('packet_verifier_passed_check_count')}/{ai_debug_packet_matrix.get('packet_verifier_check_count')} |",
        f"| Packet verifier digest mismatches | {ai_debug_packet_matrix.get('packet_verifier_digest_mismatch_count')} |",
        f"| Source windows | {ai_debug_packet_matrix.get('source_window_count')} |",
        f"| Test windows | {ai_debug_packet_matrix.get('test_window_count')} |",
        "",
        "## AI Localization Evaluation",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_localization_eval.get('status')} |",
        f"| Scenarios | {ai_localization_eval.get('passed_scenario_count')}/{ai_localization_eval.get('scenario_count')} |",
        f"| Negative fixtures | {ai_localization_eval.get('negative_fixture_count')} |",
        f"| Only happy paths | {ai_localization_eval.get('only_happy_paths')} |",
        f"| Focus-domain matches | {ai_localization_eval.get('focus_domain_match_count')}/{ai_localization_eval.get('negative_fixture_count')} |",
        f"| Route-ready fixtures | {ai_localization_eval.get('route_ready_count')}/{ai_localization_eval.get('negative_fixture_count')} |",
        f"| Packet self-verified fixtures | {ai_localization_eval.get('packet_self_verified_count')}/{ai_localization_eval.get('negative_fixture_count')} |",
        f"| Source anchors | {ai_localization_eval.get('source_anchor_scenario_count')}/{ai_localization_eval.get('negative_fixture_count')} |",
        f"| Test anchors | {ai_localization_eval.get('test_anchor_scenario_count')}/{ai_localization_eval.get('negative_fixture_count')} |",
        f"| Average score | {ai_localization_eval.get('average_score')} |",
        f"| Minimum score | {ai_localization_eval.get('minimum_score')} |",
        "",
        "## AI Artifact Verification",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ai_artifact_verification.get('status')} |",
        f"| Checks | {ai_artifact_verification.get('passed_check_count')}/{ai_artifact_verification.get('check_count')} |",
        f"| Required artifacts | {ai_artifact_verification.get('artifact_count')} |",
        f"| Missing artifacts | {ai_artifact_verification.get('missing_artifact_count')} |",
        f"| E2E report checked | {ai_artifact_verification.get('e2e_report_checked')} |",
        f"| Route id | {ai_artifact_verification.get('route_id')} |",
        f"| Scenario | {ai_artifact_verification.get('scenario_id')} |",
        f"| Focus domain | {ai_artifact_verification.get('focus_domain')} |",
        f"| Probe | {ai_artifact_verification.get('probe_id')} |",
        f"| Automation readiness | {ai_artifact_verification.get('automation_readiness_status')} |",
        f"| Automation-ready routes | {ai_artifact_verification.get('automation_ready_route_count')}/{ai_artifact_verification.get('automation_route_count')} |",
        f"| Automation not-ready routes | {ai_artifact_verification.get('automation_not_ready_route_count')} |",
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

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "build_diagnostic_ai_index",
                [
                    sys.executable,
                    script_path("build_diagnostic_ai_index.py"),
                    "--suite-dir",
                    str(suite_dir),
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command("build_diagnostic_ai_index", "route evidence verification failed")
        )

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "query_diagnostic_ai_index_smoke",
                [
                    sys.executable,
                    script_path("query_diagnostic_ai_index.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "smoke",
                ],
                repo_root,
            )
        )
    else:
        commands.append(skipped_command("query_diagnostic_ai_index_smoke", "AI index failed"))

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "run_diagnostic_ai_diagnosis_smoke",
                [
                    sys.executable,
                    script_path("run_diagnostic_ai_diagnosis.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--output-dir",
                    str(suite_dir / "ai-diagnosis-smoke"),
                    "--summary-json",
                    str(suite_dir / "diagnostic-ai-diagnosis-smoke.json"),
                    "--summary-report",
                    str(suite_dir / "diagnostic-ai-diagnosis-smoke.md"),
                    "--cargo",
                    args.cargo,
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command("run_diagnostic_ai_diagnosis_smoke", "AI query smoke failed")
        )

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "build_diagnostic_ai_fix_handoff_smoke",
                [
                    sys.executable,
                    script_path("build_diagnostic_ai_fix_handoff.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--diagnosis-json",
                    str(suite_dir / "diagnostic-ai-diagnosis-smoke.json"),
                    "--summary-json",
                    str(suite_dir / "diagnostic-ai-fix-handoff-smoke.json"),
                    "--summary-report",
                    str(suite_dir / "diagnostic-ai-fix-handoff-smoke.md"),
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command("build_diagnostic_ai_fix_handoff_smoke", "AI diagnosis smoke failed")
        )

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "run_diagnostic_ai_route_matrix",
                [
                    sys.executable,
                    script_path("run_diagnostic_ai_route_matrix.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--output-dir",
                    str(suite_dir / "ai-route-matrix"),
                    "--summary-json",
                    str(suite_dir / "diagnostic-ai-route-matrix.json"),
                    "--summary-report",
                    str(suite_dir / "diagnostic-ai-route-matrix.md"),
                    "--cargo",
                    args.cargo,
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command(
                "run_diagnostic_ai_route_matrix",
                "AI fix handoff smoke failed",
            )
        )

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "build_diagnostic_ai_debug_packet",
                [
                    sys.executable,
                    script_path("build_diagnostic_ai_debug_packet.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--output-dir",
                    str(suite_dir / "ai-debug-packet"),
                    "--summary-json",
                    str(suite_dir / "diagnostic-ai-debug-packet.json"),
                    "--summary-report",
                    str(suite_dir / "diagnostic-ai-debug-packet.md"),
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command(
                "build_diagnostic_ai_debug_packet",
                "AI route matrix failed",
            )
        )

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "verify_diagnostic_ai_debug_packet",
                [
                    sys.executable,
                    script_path("verify_diagnostic_ai_debug_packet.py"),
                    "--packet-dir",
                    str(suite_dir / "ai-debug-packet"),
                    "--summary-json",
                    str(suite_dir / "diagnostic-ai-debug-packet-verification.json"),
                    "--summary-report",
                    str(suite_dir / "diagnostic-ai-debug-packet-verification.md"),
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command(
                "verify_diagnostic_ai_debug_packet",
                "AI debug packet failed",
            )
        )

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "run_diagnostic_ai_debug_packet_matrix",
                [
                    sys.executable,
                    script_path("run_diagnostic_ai_debug_packet_matrix.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--output-dir",
                    str(suite_dir / "ai-debug-packet-matrix"),
                    "--summary-json",
                    str(suite_dir / "diagnostic-ai-debug-packet-matrix.json"),
                    "--summary-report",
                    str(suite_dir / "diagnostic-ai-debug-packet-matrix.md"),
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command(
                "run_diagnostic_ai_debug_packet_matrix",
                "AI debug packet verification failed",
            )
        )

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "evaluate_diagnostic_ai_localization",
                [
                    sys.executable,
                    script_path("evaluate_diagnostic_ai_localization.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--summary-json",
                    str(suite_dir / "diagnostic-ai-localization-eval.json"),
                    "--summary-report",
                    str(suite_dir / "diagnostic-ai-localization-eval.md"),
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command(
                "evaluate_diagnostic_ai_localization",
                "AI debug packet matrix failed",
            )
        )

    if command_passed(commands[-1]):
        commands.append(
            run_command(
                "verify_diagnostic_ai_artifacts",
                [
                    sys.executable,
                    script_path("verify_diagnostic_ai_artifacts.py"),
                    "--suite-dir",
                    str(suite_dir),
                    "--require-ai-route-matrix",
                    "--require-ai-debug-packet",
                    "--require-ai-debug-packet-matrix",
                ],
                repo_root,
            )
        )
    else:
        commands.append(
            skipped_command(
                "verify_diagnostic_ai_artifacts",
                "AI localization evaluation failed",
            )
        )

    summary = build_summary(suite_dir, summary_json, summary_report, commands, repo_root)
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        observability = as_dict(summary.get("observability"))
        routes = as_dict(summary.get("routes"))
        ai_diagnosis = as_dict(summary.get("ai_diagnosis"))
        ai_fix_handoff = as_dict(summary.get("ai_fix_handoff"))
        ai_route_matrix = as_dict(summary.get("ai_route_matrix"))
        ai_debug_packet = as_dict(summary.get("ai_debug_packet"))
        ai_debug_packet_verification = as_dict(summary.get("ai_debug_packet_verification"))
        ai_debug_packet_matrix = as_dict(summary.get("ai_debug_packet_matrix"))
        ai_localization_eval = as_dict(summary.get("ai_localization_eval"))
        ai_artifact_verification = as_dict(summary.get("ai_artifact_verification"))
        print(
            "Diagnostic e2e report "
            f"{summary['status']}: suite={suite_dir} "
            f"summary_json={summary_json} summary_report={summary_report} "
            f"scenarios={observability.get('scenario_count')} "
            f"dossiers={observability.get('scenario_dossiers')}:{observability.get('actionable_dossiers')} "
            f"routes={routes.get('matrix_passed_route_count')}:{routes.get('top_route_verified')} "
            f"diagnosis={ai_diagnosis.get('status')}:{ai_diagnosis.get('route_id')} "
            f"fix_handoff={ai_fix_handoff.get('status')}:{ai_fix_handoff.get('source_match_count')} "
            f"ai_route_matrix={ai_route_matrix.get('status')}:{ai_route_matrix.get('passed_route_count')}/{ai_route_matrix.get('route_count')} "
            f"ai_debug_packet={ai_debug_packet.get('status')}:{ai_debug_packet.get('file_count')} "
            f"ai_packet_verify={ai_debug_packet_verification.get('status')}:{ai_debug_packet_verification.get('passed_check_count')}/{ai_debug_packet_verification.get('check_count')} "
            f"ai_debug_packet_matrix={ai_debug_packet_matrix.get('status')}:{ai_debug_packet_matrix.get('passed_route_count')}/{ai_debug_packet_matrix.get('route_count')} "
            f"ai_localization={ai_localization_eval.get('status')}:{ai_localization_eval.get('passed_scenario_count')}/{ai_localization_eval.get('scenario_count')} "
            f"ai_readiness={ai_artifact_verification.get('automation_readiness_status')}:{ai_artifact_verification.get('automation_ready_route_count')}/{ai_artifact_verification.get('automation_route_count')} "
            f"ai_artifacts={ai_artifact_verification.get('status')}:{ai_artifact_verification.get('missing_artifact_count')}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
