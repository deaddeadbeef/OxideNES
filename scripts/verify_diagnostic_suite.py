#!/usr/bin/env python3
"""Validate an OxideNES diagnostic scenario suite artifact directory."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


EXPECTED_SCENARIO_SUITE_SCHEMA = 6
EXPECTED_OBSERVER_SCHEMA = 1
EXPECTED_TELEMETRY_SCHEMA = 19
EXPECTED_TRIAGE_SCHEMA = 6
EXPECTED_BUNDLE_SCHEMA = 2
EXPECTED_SCENARIOS = {
    "pass",
    "joypad1_mismatch",
    "joypad2_mismatch",
    "cpu_zero_page_wrap_fault",
    "ppu_read_buffer_fault",
    "timeout_cycle_limit",
}


class SuiteVerifier:
    def __init__(self, suite_dir: Path) -> None:
        self.suite_dir = suite_dir
        self.errors: list[str] = []

    def verify(self) -> dict[str, Any]:
        manifest = self.read_json("scenario-suite.json")
        observer = self.read_json("scenario-suite-observer.json")
        suite_report = self.read_text("scenario-suite.md")
        observer_report = self.read_text("scenario-suite-observer.md")

        self.expect_equal(
            manifest.get("scenario_suite_schema_version"),
            EXPECTED_SCENARIO_SUITE_SCHEMA,
            "scenario-suite.json scenario_suite_schema_version",
        )
        self.expect_equal(
            manifest.get("telemetry_schema_version"),
            EXPECTED_TELEMETRY_SCHEMA,
            "scenario-suite.json telemetry_schema_version",
        )
        self.expect_equal(
            manifest.get("triage_schema_version"),
            EXPECTED_TRIAGE_SCHEMA,
            "scenario-suite.json triage_schema_version",
        )
        self.expect_equal(
            manifest.get("bundle_schema_version"),
            EXPECTED_BUNDLE_SCHEMA,
            "scenario-suite.json bundle_schema_version",
        )
        self.expect_equal(manifest.get("passed"), True, "scenario-suite.json passed")
        self.expect_equal(
            manifest.get("recommended_exit_code"),
            0,
            "scenario-suite.json recommended_exit_code",
        )

        analysis = self.expect_dict(manifest.get("analysis"), "scenario-suite.json analysis")
        self.expect_equal(analysis.get("status"), "passed", "analysis status")
        self.expect_equal(
            analysis.get("contract_mismatch_count"),
            0,
            "analysis contract_mismatch_count",
        )
        self.expect_equal(
            analysis.get("baseline_divergence_count"),
            5,
            "analysis baseline_divergence_count",
        )

        scenarios = self.expect_list(manifest.get("scenarios"), "scenario-suite.json scenarios")
        scenario_ids = {scenario.get("id") for scenario in scenarios if isinstance(scenario, dict)}
        self.expect_equal(scenario_ids, EXPECTED_SCENARIOS, "scenario ids")
        self.expect_equal(
            manifest.get("scenario_count"),
            len(EXPECTED_SCENARIOS),
            "scenario-suite.json scenario_count",
        )

        root_artifacts = self.expect_dict(
            manifest.get("artifacts"), "scenario-suite.json artifacts"
        )
        self.expect_equal(
            root_artifacts.get("scenario_suite_observer_json"),
            "scenario-suite-observer.json",
            "root observer JSON artifact path",
        )
        self.expect_equal(
            root_artifacts.get("scenario_suite_observer_report"),
            "scenario-suite-observer.md",
            "root observer report artifact path",
        )

        self.expect_equal(
            observer.get("observer_schema_version"),
            EXPECTED_OBSERVER_SCHEMA,
            "observer observer_schema_version",
        )
        self.expect_equal(
            observer.get("scenario_suite_schema_version"),
            EXPECTED_SCENARIO_SUITE_SCHEMA,
            "observer scenario_suite_schema_version",
        )
        self.expect_equal(observer.get("status"), "passed", "observer status")
        self.expect_equal(
            observer.get("scenario_count"),
            len(EXPECTED_SCENARIOS),
            "observer scenario_count",
        )
        self.expect_equal(
            observer.get("contract_mismatch_count"),
            0,
            "observer contract_mismatch_count",
        )
        self.expect_equal(
            observer.get("baseline_divergence_count"),
            5,
            "observer baseline_divergence_count",
        )

        actions = self.expect_list(observer.get("next_actions"), "observer next_actions")
        observations = self.expect_list(observer.get("observations"), "observer observations")
        self.expect_equal(len(actions), 5, "observer next_actions count")
        self.expect_equal(len(observations), len(EXPECTED_SCENARIOS), "observer observations count")
        self.verify_observer_actions(actions)
        self.verify_observer_observations(observations)
        self.verify_artifact_paths(manifest, observer)
        self.expect_contains(suite_report, "## Attention Queue", "scenario-suite.md")
        self.expect_contains(suite_report, "## AI Drilldown", "scenario-suite.md")
        self.expect_contains(observer_report, "## Next Actions", "scenario-suite-observer.md")
        self.expect_contains(observer_report, "## Observations", "scenario-suite-observer.md")
        self.expect_contains(observer_report, "## Artifact Hints", "scenario-suite-observer.md")

        return {
            "suite_dir": str(self.suite_dir),
            "scenario_suite_schema_version": manifest.get("scenario_suite_schema_version"),
            "observer_schema_version": observer.get("observer_schema_version"),
            "scenario_count": len(scenarios),
            "next_actions": len(actions),
            "observations": len(observations),
        }

    def verify_observer_actions(self, actions: list[Any]) -> None:
        by_scenario = {
            action.get("scenario_id"): action
            for action in actions
            if isinstance(action, dict)
        }
        expected_action_ids = {
            "joypad1_mismatch",
            "joypad2_mismatch",
            "cpu_zero_page_wrap_fault",
            "ppu_read_buffer_fault",
            "timeout_cycle_limit",
        }
        self.expect_equal(set(by_scenario), expected_action_ids, "observer action scenario ids")

        timeout = by_scenario.get("timeout_cycle_limit")
        if not isinstance(timeout, dict):
            self.errors.append("missing observer action for timeout_cycle_limit")
            return

        self.expect_equal(
            timeout.get("priority"),
            "known_divergence",
            "timeout observer action priority",
        )
        self.expect_equal(
            timeout.get("action_type"),
            "inspect_known_divergence",
            "timeout observer action type",
        )
        self.expect_equal(
            timeout.get("primary_artifact"),
            "timeout_cycle_limit/comparison.json",
            "timeout observer primary_artifact",
        )
        evidence = self.expect_list(timeout.get("evidence"), "timeout observer evidence")
        self.expect_in(
            "comparison_difference_count=92",
            evidence,
            "timeout observer evidence",
        )

        ppu = by_scenario.get("ppu_read_buffer_fault")
        if not isinstance(ppu, dict):
            self.errors.append("missing observer action for ppu_read_buffer_fault")
            return

        self.expect_equal(
            ppu.get("priority"),
            "known_divergence",
            "PPU observer action priority",
        )
        self.expect_equal(
            ppu.get("action_type"),
            "inspect_known_divergence",
            "PPU observer action type",
        )
        self.expect_equal(
            ppu.get("primary_artifact"),
            "ppu_read_buffer_fault/comparison.json",
            "PPU observer primary_artifact",
        )
        ppu_evidence = self.expect_list(ppu.get("evidence"), "PPU observer evidence")
        self.expect_in(
            "focus_domain=ppu.registers.ppudata_buffer",
            ppu_evidence,
            "PPU observer evidence",
        )
        self.expect_in(
            "failed_probe_ids=cartridge.status.pass,cartridge.test.14.result",
            ppu_evidence,
            "PPU observer evidence",
        )

        cpu = by_scenario.get("cpu_zero_page_wrap_fault")
        if not isinstance(cpu, dict):
            self.errors.append("missing observer action for cpu_zero_page_wrap_fault")
            return

        self.expect_equal(
            cpu.get("priority"),
            "known_divergence",
            "CPU observer action priority",
        )
        self.expect_equal(
            cpu.get("action_type"),
            "inspect_known_divergence",
            "CPU observer action type",
        )
        self.expect_equal(
            cpu.get("primary_artifact"),
            "cpu_zero_page_wrap_fault/comparison.json",
            "CPU observer primary_artifact",
        )
        cpu_evidence = self.expect_list(cpu.get("evidence"), "CPU observer evidence")
        self.expect_in(
            "focus_domain=cpu.addressing.zero_page_x_wrap",
            cpu_evidence,
            "CPU observer evidence",
        )
        self.expect_in(
            "failed_probe_ids=cartridge.status.pass,cartridge.test.12.result",
            cpu_evidence,
            "CPU observer evidence",
        )
        self.expect_in(
            "top_difference_path=dma.oam_dma_observed",
            evidence,
            "timeout observer evidence",
        )

    def verify_observer_observations(self, observations: list[Any]) -> None:
        by_scenario = {
            observation.get("scenario_id"): observation
            for observation in observations
            if isinstance(observation, dict)
        }
        self.expect_equal(set(by_scenario), EXPECTED_SCENARIOS, "observer observation scenario ids")

        pass_observation = by_scenario.get("pass")
        if isinstance(pass_observation, dict):
            self.expect_equal(pass_observation.get("role"), "baseline", "pass observer role")
            self.expect_equal(
                pass_observation.get("outcome"),
                "matches_baseline",
                "pass observer outcome",
            )
            self.expect_equal(
                pass_observation.get("next_artifact"),
                "pass/triage.json",
                "pass observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for pass")

        timeout = by_scenario.get("timeout_cycle_limit")
        if isinstance(timeout, dict):
            self.expect_equal(
                timeout.get("role"),
                "expected_failure_fixture",
                "timeout observer role",
            )
            self.expect_equal(
                timeout.get("outcome"),
                "expected_baseline_divergence",
                "timeout observer outcome",
            )
            self.expect_equal(
                timeout.get("focus_domain"),
                "emulator.progress_or_infinite_loop",
                "timeout observer focus_domain",
            )
            self.expect_equal(
                timeout.get("comparison_difference_count"),
                92,
                "timeout observer comparison_difference_count",
            )
            self.expect_equal(
                timeout.get("next_artifact"),
                "timeout_cycle_limit/comparison.json",
                "timeout observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for timeout_cycle_limit")

        ppu = by_scenario.get("ppu_read_buffer_fault")
        if isinstance(ppu, dict):
            self.expect_equal(
                ppu.get("role"),
                "expected_failure_fixture",
                "PPU observer role",
            )
            self.expect_equal(
                ppu.get("outcome"),
                "expected_baseline_divergence",
                "PPU observer outcome",
            )
            self.expect_equal(
                ppu.get("focus_domain"),
                "ppu.registers.ppudata_buffer",
                "PPU observer focus_domain",
            )
            self.expect_equal(
                ppu.get("next_artifact"),
                "ppu_read_buffer_fault/comparison.json",
                "PPU observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for ppu_read_buffer_fault")

        cpu = by_scenario.get("cpu_zero_page_wrap_fault")
        if isinstance(cpu, dict):
            self.expect_equal(
                cpu.get("role"),
                "expected_failure_fixture",
                "CPU observer role",
            )
            self.expect_equal(
                cpu.get("outcome"),
                "expected_baseline_divergence",
                "CPU observer outcome",
            )
            self.expect_equal(
                cpu.get("focus_domain"),
                "cpu.addressing.zero_page_x_wrap",
                "CPU observer focus_domain",
            )
            self.expect_equal(
                cpu.get("next_artifact"),
                "cpu_zero_page_wrap_fault/comparison.json",
                "CPU observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for cpu_zero_page_wrap_fault")

    def verify_artifact_paths(self, manifest: dict[str, Any], observer: dict[str, Any]) -> None:
        root_artifacts = self.expect_dict(manifest.get("artifacts"), "scenario-suite artifacts")
        for field in (
            "scenario_suite_json",
            "scenario_suite_report",
            "scenario_suite_observer_json",
            "scenario_suite_observer_report",
        ):
            path = root_artifacts.get(field)
            if isinstance(path, str):
                self.expect_file(path, f"root artifact {field}")

        for scenario in self.expect_list(manifest.get("scenarios"), "scenario-suite scenarios"):
            if not isinstance(scenario, dict):
                continue
            artifacts = self.expect_dict(
                scenario.get("artifacts"), f"{scenario.get('id')} artifacts"
            )
            for field in (
                "bundle_manifest",
                "triage_json",
                "telemetry_json",
                "report_md",
                "comparison_json",
                "comparison_report",
                "diagnostic_rom",
            ):
                path = artifacts.get(field)
                if isinstance(path, str):
                    self.expect_file(path, f"{scenario.get('id')} artifact {field}")

        for action in self.expect_list(observer.get("next_actions"), "observer next_actions"):
            if not isinstance(action, dict):
                continue
            primary = action.get("primary_artifact")
            if isinstance(primary, str):
                self.expect_file(primary, f"{action.get('scenario_id')} primary_artifact")
            for artifact in self.expect_list(
                action.get("supporting_artifacts"),
                f"{action.get('scenario_id')} supporting_artifacts",
            ):
                if isinstance(artifact, str):
                    self.expect_file(artifact, f"{action.get('scenario_id')} supporting artifact")

        for observation in self.expect_list(observer.get("observations"), "observer observations"):
            if not isinstance(observation, dict):
                continue
            for field in ("bundle_manifest", "triage_json", "comparison_json", "telemetry_json"):
                path = observation.get(field)
                if isinstance(path, str):
                    self.expect_file(path, f"{observation.get('scenario_id')} observation {field}")

    def read_json(self, relative_path: str) -> dict[str, Any]:
        path = self.suite_dir / relative_path
        if not path.is_file():
            self.errors.append(f"missing JSON artifact: {relative_path}")
            return {}
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as err:
            self.errors.append(f"invalid JSON in {relative_path}: {err}")
            return {}
        if not isinstance(data, dict):
            self.errors.append(f"{relative_path} must contain a JSON object")
            return {}
        return data

    def read_text(self, relative_path: str) -> str:
        path = self.suite_dir / relative_path
        if not path.is_file():
            self.errors.append(f"missing text artifact: {relative_path}")
            return ""
        return path.read_text(encoding="utf-8")

    def expect_dict(self, value: Any, label: str) -> dict[str, Any]:
        if isinstance(value, dict):
            return value
        self.errors.append(f"{label} must be an object")
        return {}

    def expect_list(self, value: Any, label: str) -> list[Any]:
        if isinstance(value, list):
            return value
        self.errors.append(f"{label} must be an array")
        return []

    def expect_file(self, relative_path: str, label: str) -> None:
        if not (self.suite_dir / relative_path).is_file():
            self.errors.append(f"missing {label}: {relative_path}")

    def expect_equal(self, actual: Any, expected: Any, label: str) -> None:
        if actual != expected:
            self.errors.append(f"{label}: expected {expected!r}, got {actual!r}")

    def expect_contains(self, text: str, needle: str, label: str) -> None:
        if needle not in text:
            self.errors.append(f"{label}: missing {needle!r}")

    def expect_in(self, needle: Any, haystack: list[Any], label: str) -> None:
        if needle not in haystack:
            self.errors.append(f"{label}: missing {needle!r}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate an OxideNES diagnostic scenario suite directory."
    )
    parser.add_argument(
        "--suite-dir",
        required=True,
        type=Path,
        help="Directory produced by oxidenes-diagnostic --scenario-suite-dir.",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Print a machine-readable verification summary.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    verifier = SuiteVerifier(args.suite_dir)
    summary = verifier.verify()
    if verifier.errors:
        for error in verifier.errors:
            print(f"diagnostic suite verification failed: {error}", file=sys.stderr)
        return 1

    if args.json:
        print(json.dumps({"passed": True, **summary}, indent=2, sort_keys=True))
    else:
        print(
            "Diagnostic scenario suite verification passed: "
            f"schema={summary['scenario_suite_schema_version']} "
            f"observer_schema={summary['observer_schema_version']} "
            f"scenarios={summary['scenario_count']} "
            f"next_actions={summary['next_actions']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
