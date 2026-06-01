#!/usr/bin/env python3
"""Validate an OxideNES diagnostic observability run artifact directory."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


EXPECTED_RUN_SCHEMA = 1
EXPECTED_REPLAY_SCHEMA = 1
EXPECTED_DEBUG_INDEX_SCHEMA = 1
EXPECTED_ANALYSIS_SCHEMA = 1
EXPECTED_COMPARISON_SCHEMA = 1
EXPECTED_SCENARIO_COUNT = 18
EXPECTED_ACTIONABLE_SCENARIO_COUNT = 16
EXPECTED_SCENARIOS = {
    "pass",
    "input_mask_matrix_pass",
    "joypad1_mismatch",
    "joypad2_mismatch",
    "dma_oam_transfer_fault",
    "apu_status_fault",
    "cpu_zero_page_wrap_fault",
    "cpu_indirect_jmp_fault",
    "ppu_nmi_timeout_fault",
    "ppu_read_buffer_fault",
    "ppu_nametable_mirroring_fault",
    "joypad_strobe_reset_fault",
    "joypad_strobe_high_hold_fault",
    "ppu_vram_increment_32_fault",
    "ppu_status_latch_reset_fault",
    "mapper2_bank_switch_fault",
    "mapper2_prg_ram_fault",
    "timeout_cycle_limit",
}


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


class ObservabilityVerifier:
    def __init__(self, suite_dir: Path, run_json: Path, repo_root: Path) -> None:
        self.suite_dir = suite_dir
        self.run_json = run_json
        self.repo_root = repo_root
        self.errors: list[str] = []

    def verify(self) -> dict[str, Any]:
        run = self.read_json_file(self.run_json, "observability run JSON")
        self.expect_equal(
            run.get("observability_run_schema_version"),
            EXPECTED_RUN_SCHEMA,
            "observability run schema version",
        )
        self.expect_equal(run.get("status"), "passed", "observability run status")
        self.expect_equal(
            run.get("recommended_exit_code"),
            0,
            "observability run recommended_exit_code",
        )

        self.verify_commands(as_list(run.get("commands")))
        self.verify_suite_summary(as_dict(run.get("suite")))
        self.verify_run_artifacts(as_dict(run.get("artifacts")))
        self.verify_run_report(as_dict(run.get("artifacts")))
        self.verify_embedded_suite_verification(as_dict(run.get("verification")))

        debug_entries = self.verify_debug_index(as_dict(run.get("debug_index")))
        self.verify_analysis(as_dict(run.get("analysis")), debug_entries)
        self.verify_comparison(run.get("comparison"))
        self.verify_replay(run.get("replay"))

        return {
            "run_json": str(self.run_json),
            "suite_dir": str(self.suite_dir),
            "observability_run_schema_version": run.get("observability_run_schema_version"),
            "scenario_count": as_dict(run.get("suite")).get("scenario_count"),
            "debug_index_entries": len(debug_entries),
            "hypothesis_count": as_dict(run.get("analysis")).get("hypothesis_count"),
            "comparison_verdict": as_dict(run.get("comparison")).get("verdict")
            if isinstance(run.get("comparison"), dict)
            else None,
            "replay_scenario": as_dict(run.get("replay")).get("scenario_id")
            if isinstance(run.get("replay"), dict)
            else None,
        }

    def verify_commands(self, commands: list[Any]) -> None:
        names = [command.get("name") for command in commands if isinstance(command, dict)]
        self.expect_in("generate_scenario_suite", names, "observability commands")
        self.expect_in("verify_scenario_suite", names, "observability commands")
        for command in commands:
            if not isinstance(command, dict):
                self.errors.append("observability commands must be objects")
                continue
            if command.get("name") != "replay_scenario":
                self.expect_equal(command.get("exit_code"), 0, f"{command.get('name')} exit code")

    def verify_suite_summary(self, suite: dict[str, Any]) -> None:
        self.expect_equal(
            suite.get("scenario_count"),
            EXPECTED_SCENARIO_COUNT,
            "suite scenario_count",
        )
        self.expect_equal(suite.get("passed"), True, "suite passed")
        self.expect_equal(suite.get("observer_status"), "passed", "suite observer_status")
        self.expect_equal(
            suite.get("contract_mismatch_count"),
            0,
            "suite contract_mismatch_count",
        )
        self.expect_equal(
            suite.get("baseline_divergence_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "suite baseline_divergence_count",
        )
        self.expect_equal(
            suite.get("next_action_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "suite next_action_count",
        )
        self.expect_equal(
            suite.get("observation_count"),
            EXPECTED_SCENARIO_COUNT,
            "suite observation_count",
        )
        first_action = as_dict(suite.get("first_next_action"))
        self.expect_nonempty_string(
            first_action.get("primary_artifact"),
            "suite first_next_action primary_artifact",
        )

    def verify_run_artifacts(self, artifacts: dict[str, Any]) -> None:
        for name, value in artifacts.items():
            if self.is_directory_artifact(name):
                self.expect_existing_dir(value, f"run artifact {name}")
            else:
                self.expect_existing_file(value, f"run artifact {name}")

    def verify_run_report(self, artifacts: dict[str, Any]) -> None:
        report_path = self.resolve_existing_file(
            artifacts.get("observability_run_report"), "observability run report"
        )
        if report_path is None:
            return
        report = report_path.read_text(encoding="utf-8")
        for section in (
            "## Debug Index",
            "## Observability Analysis",
            "## Observability Comparison",
            "## Replay",
            "## AI Handoff",
        ):
            self.expect_contains(report, section, "observability run report")

    def verify_embedded_suite_verification(self, verification: dict[str, Any]) -> None:
        self.expect_equal(verification.get("passed"), True, "embedded suite verification passed")
        self.expect_equal(
            verification.get("scenario_count"),
            EXPECTED_SCENARIO_COUNT,
            "embedded suite verification scenario_count",
        )
        self.expect_equal(
            verification.get("next_actions"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "embedded suite verification next_actions",
        )

    def verify_debug_index(self, debug_index: dict[str, Any]) -> list[dict[str, Any]]:
        self.expect_equal(
            debug_index.get("debug_index_schema_version"),
            EXPECTED_DEBUG_INDEX_SCHEMA,
            "debug index schema version",
        )
        self.expect_equal(debug_index.get("status"), "passed", "debug index status")
        self.expect_equal(
            debug_index.get("entry_count"),
            EXPECTED_SCENARIO_COUNT,
            "debug index entry_count",
        )
        self.verify_artifact_map(as_dict(debug_index.get("artifacts")), "debug index")

        jsonl_path = self.resolve_existing_file(
            as_dict(debug_index.get("artifacts")).get("debug_index_jsonl"),
            "debug index JSONL",
        )
        entries = self.read_jsonl_file(jsonl_path, "debug index JSONL") if jsonl_path else []
        scenario_ids = {entry.get("scenario_id") for entry in entries}
        self.expect_equal(scenario_ids, EXPECTED_SCENARIOS, "debug index scenario ids")

        role_counts: dict[str, int] = {}
        for entry in entries:
            scenario_id = entry.get("scenario_id")
            label = f"debug index {scenario_id}"
            self.expect_equal(
                entry.get("debug_index_schema_version"),
                EXPECTED_DEBUG_INDEX_SCHEMA,
                f"{label} schema version",
            )
            role = entry.get("role")
            self.expect_nonempty_string(role, f"{label} role")
            if isinstance(role, str):
                role_counts[role] = role_counts.get(role, 0) + 1
            self.expect_nonempty_string(entry.get("outcome"), f"{label} outcome")
            self.expect_nonempty_string(entry.get("health"), f"{label} health")
            self.expect_nonempty_list(entry.get("replay_args"), f"{label} replay_args")
            self.expect_nonempty_string(
                as_dict(entry.get("artifacts")).get("triage_json"),
                f"{label} triage artifact",
            )
            focus = as_dict(entry.get("debug_focus"))
            if not focus:
                self.errors.append(f"{label} debug_focus must be an object")
                continue
            has_anchor = bool(focus.get("terminal_instruction")) or bool(
                focus.get("last_event")
            ) or bool(entry.get("event_tail_last"))
            if not has_anchor:
                self.errors.append(f"{label} must include a terminal instruction or last event")

        self.expect_equal(role_counts.get("baseline"), 1, "debug index baseline role count")
        self.expect_equal(
            role_counts.get("expected_pass_fixture"),
            1,
            "debug index expected_pass_fixture role count",
        )
        self.expect_equal(
            role_counts.get("expected_failure_fixture"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "debug index expected_failure_fixture role count",
        )
        return entries

    def verify_analysis(
        self, analysis: dict[str, Any], debug_entries: list[dict[str, Any]]
    ) -> None:
        self.expect_equal(
            analysis.get("observability_analysis_schema_version"),
            EXPECTED_ANALYSIS_SCHEMA,
            "observability analysis schema version",
        )
        self.expect_equal(analysis.get("status"), "passed", "observability analysis status")
        self.expect_equal(
            analysis.get("recommended_exit_code"),
            0,
            "observability analysis recommended_exit_code",
        )
        self.expect_equal(
            analysis.get("scenario_count"),
            EXPECTED_SCENARIO_COUNT,
            "observability analysis scenario_count",
        )
        self.expect_equal(
            analysis.get("actionable_scenario_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "observability analysis actionable_scenario_count",
        )
        self.expect_equal(
            analysis.get("hypothesis_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "observability analysis hypothesis_count",
        )
        self.expect_equal(analysis.get("errors"), [], "observability analysis errors")
        self.verify_artifact_map(as_dict(analysis.get("artifacts")), "observability analysis")

        ranked = as_list(analysis.get("ranked_hypotheses"))
        self.expect_equal(
            len(ranked),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "ranked_hypotheses count",
        )
        hypothesis_domains = set()
        for hypothesis in ranked:
            if not isinstance(hypothesis, dict):
                self.errors.append("ranked_hypotheses entries must be objects")
                continue
            domain = hypothesis.get("focus_domain")
            self.expect_nonempty_string(domain, "ranked_hypothesis focus_domain")
            if isinstance(domain, str):
                hypothesis_domains.add(domain)
            self.expect_nonempty_list(
                hypothesis.get("scenario_ids"),
                f"ranked_hypothesis {domain} scenario_ids",
            )
            suggested = as_dict(hypothesis.get("suggested_next_action"))
            primary_artifact = suggested.get("open_artifact")
            if not primary_artifact:
                primary_artifacts = as_list(hypothesis.get("primary_artifacts"))
                primary_artifact = primary_artifacts[0] if primary_artifacts else None
            self.expect_nonempty_string(
                primary_artifact,
                f"ranked_hypothesis {domain} suggested artifact",
            )

        debug_domains = {
            as_dict(entry.get("debug_focus")).get("focus_domain")
            for entry in debug_entries
            if entry.get("role") == "expected_failure_fixture"
        }
        self.expect_equal(
            hypothesis_domains,
            debug_domains,
            "observability analysis hypothesis domains",
        )

    def verify_comparison(self, comparison_value: Any) -> None:
        if comparison_value is None:
            return
        comparison = as_dict(comparison_value)
        self.expect_equal(
            comparison.get("observability_comparison_schema_version"),
            EXPECTED_COMPARISON_SCHEMA,
            "observability comparison schema version",
        )
        self.expect_equal(comparison.get("status"), "passed", "observability comparison status")
        self.expect_equal(
            comparison.get("recommended_exit_code"),
            0,
            "observability comparison recommended_exit_code",
        )
        self.expect_equal(comparison.get("errors"), [], "observability comparison errors")
        self.expect_equal(
            comparison.get("regression_count"),
            0,
            "observability comparison regression_count",
        )
        self.verify_artifact_map(as_dict(comparison.get("artifacts")), "observability comparison")

        for side in ("baseline", "current"):
            summary = as_dict(comparison.get(side))
            self.expect_equal(
                summary.get("scenario_count"),
                EXPECTED_SCENARIO_COUNT,
                f"observability comparison {side} scenario_count",
            )
            self.expect_equal(
                summary.get("hypothesis_count"),
                EXPECTED_ACTIONABLE_SCENARIO_COUNT,
                f"observability comparison {side} hypothesis_count",
            )
            self.verify_artifact_map(as_dict(summary.get("artifacts")), f"comparison {side}")

        scenario_changes = as_list(comparison.get("scenario_changes"))
        if scenario_changes:
            self.expect_equal(
                len(scenario_changes),
                EXPECTED_SCENARIO_COUNT,
                "observability comparison scenario_changes count",
            )
        hypothesis_changes = as_list(comparison.get("hypothesis_changes"))
        if hypothesis_changes:
            self.expect_equal(
                len(hypothesis_changes),
                EXPECTED_ACTIONABLE_SCENARIO_COUNT,
                "observability comparison hypothesis_changes count",
            )

    def verify_replay(self, replay_value: Any) -> None:
        if replay_value is None:
            return
        replay = as_dict(replay_value)
        self.expect_equal(
            replay.get("replay_run_schema_version"),
            EXPECTED_REPLAY_SCHEMA,
            "replay run schema version",
        )
        self.expect_equal(replay.get("status"), "passed", "replay status")
        self.expect_equal(replay.get("recommended_exit_code"), 0, "replay recommended_exit_code")
        self.expect_nonempty_string(replay.get("scenario_id"), "replay scenario_id")
        for field in (
            "exit_code_matches_expected",
            "health_matches_expected",
            "focus_test_matches_expected",
            "focus_domain_matches_expected",
            "required_artifacts_present",
        ):
            self.expect_equal(replay.get(field), True, f"replay {field}")
        self.verify_artifact_map(as_dict(replay.get("artifacts")), "replay")

    def verify_artifact_map(self, artifacts: dict[str, Any], label: str) -> None:
        if not artifacts:
            self.errors.append(f"{label} artifacts must be a non-empty object")
            return
        for name, value in artifacts.items():
            if self.is_directory_artifact(name):
                self.expect_existing_dir(value, f"{label} artifact {name}")
            else:
                self.expect_existing_file(value, f"{label} artifact {name}")

    def read_json_file(self, path: Path, label: str) -> dict[str, Any]:
        if not path.is_file():
            self.errors.append(f"missing {label}: {path}")
            return {}
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as error:
            self.errors.append(f"invalid JSON in {label}: {error}")
            return {}
        if not isinstance(value, dict):
            self.errors.append(f"{label} must contain a JSON object")
            return {}
        return value

    def read_jsonl_file(self, path: Path, label: str) -> list[dict[str, Any]]:
        entries: list[dict[str, Any]] = []
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if not line.strip():
                continue
            try:
                value = json.loads(line)
            except json.JSONDecodeError as error:
                self.errors.append(f"{label}:{line_number}: invalid JSON: {error}")
                continue
            if isinstance(value, dict):
                entries.append(value)
            else:
                self.errors.append(f"{label}:{line_number}: expected JSON object")
        return entries

    def resolve_existing_file(self, value: Any, label: str) -> Path | None:
        path = self.resolve_path(value)
        if path is None:
            self.errors.append(f"{label} path must be a string")
            return None
        if not path.is_file():
            self.errors.append(f"missing {label}: {path}")
            return None
        return path

    def expect_existing_file(self, value: Any, label: str) -> None:
        self.resolve_existing_file(value, label)

    def expect_existing_dir(self, value: Any, label: str) -> None:
        path = self.resolve_path(value)
        if path is None:
            self.errors.append(f"{label} path must be a string")
            return
        if not path.is_dir():
            self.errors.append(f"missing {label}: {path}")

    def resolve_path(self, value: Any) -> Path | None:
        if not isinstance(value, str) or not value:
            return None
        candidates = [Path(value)]
        normalized = value.replace("\\", "/")
        if normalized != value:
            candidates.append(Path(normalized))
        for candidate in candidates:
            if candidate.is_absolute():
                if candidate.exists():
                    return candidate
            else:
                joined = self.repo_root / candidate
                if joined.exists():
                    return joined
        candidate = candidates[0]
        return candidate if candidate.is_absolute() else self.repo_root / candidate

    def is_directory_artifact(self, name: str) -> bool:
        return name == "suite_dir" or name.endswith("_dir")

    def expect_equal(self, actual: Any, expected: Any, label: str) -> None:
        if actual != expected:
            self.errors.append(f"{label}: expected {expected!r}, got {actual!r}")

    def expect_contains(self, text: str, needle: str, label: str) -> None:
        if needle not in text:
            self.errors.append(f"{label}: missing {needle!r}")

    def expect_in(self, needle: Any, haystack: list[Any], label: str) -> None:
        if needle not in haystack:
            self.errors.append(f"{label}: missing {needle!r}")

    def expect_nonempty_string(self, value: Any, label: str) -> None:
        if not isinstance(value, str) or not value:
            self.errors.append(f"{label} must be a non-empty string")

    def expect_nonempty_list(self, value: Any, label: str) -> None:
        if not isinstance(value, list) or not value:
            self.errors.append(f"{label} must be a non-empty list")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate an OxideNES diagnostic observability run directory."
    )
    parser.add_argument(
        "--suite-dir",
        required=True,
        type=Path,
        help="Directory produced by scripts/run_diagnostic_observability.py.",
    )
    parser.add_argument(
        "--run-json",
        type=Path,
        help="Observability run JSON path. Defaults to <suite-dir>/observability-run.json.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Print a machine-readable verification summary.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    suite_dir = args.suite_dir
    run_json = args.run_json or suite_dir / "observability-run.json"
    verifier = ObservabilityVerifier(suite_dir, run_json, Path.cwd())
    summary = verifier.verify()
    if verifier.errors:
        for error in verifier.errors:
            print(f"diagnostic observability verification failed: {error}", file=sys.stderr)
        return 1

    if args.json:
        print(json.dumps({"passed": True, **summary}, indent=2, sort_keys=True))
    else:
        print(
            "Diagnostic observability verification passed: "
            f"run_schema={summary['observability_run_schema_version']} "
            f"scenarios={summary['scenario_count']} "
            f"debug_index={summary['debug_index_entries']} "
            f"hypotheses={summary['hypothesis_count']} "
            f"comparison={summary['comparison_verdict'] or '-'} "
            f"replay={summary['replay_scenario'] or '-'}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
