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
EXPECTED_COVERAGE_LEDGER_SCHEMA = 1
EXPECTED_TELEMETRY_CATALOG_SCHEMA = 1
EXPECTED_CODE_MAP_SCHEMA = 1
EXPECTED_INVESTIGATION_PLAN_SCHEMA = 1
EXPECTED_SCENARIO_DOSSIERS_SCHEMA = 1
EXPECTED_TELEMETRY_SCHEMA = 36
EXPECTED_SCENARIO_COUNT = 24
EXPECTED_ACTIONABLE_SCENARIO_COUNT = 22
EXPECTED_PASS_SCENARIO_COUNT = 2
EXPECTED_CARTRIDGE_TEST_COUNT = 27
EXPECTED_COVERAGE_GAP_COUNT = 6
EXPECTED_PROBE_COUNT = 44
EXPECTED_EVENT_KIND_COUNT = 9
EXPECTED_SIGNAL_FAMILY_COUNT = 8
EXPECTED_TRACE_RETAINED_INSTRUCTION_COUNT = 64
EXPECTED_SIGNAL_FAMILIES = {
    "verdict",
    "debug_focus",
    "probes",
    "timeline",
    "events",
    "instruction_trace",
    "input_dma_audio",
    "coverage_limits",
}
EXPECTED_EVENT_KINDS = {
    "reset",
    "status_changed",
    "test_changed",
    "oam_dma_started",
    "oam_dma_completed",
    "dmc_dma_fetched",
    "dmc_dma_oam_overlap",
    "frame_complete",
    "post_pass_frame_complete",
}
EXPECTED_SCENARIOS = {
    "pass",
    "input_mask_matrix_pass",
    "joypad1_mismatch",
    "joypad2_mismatch",
    "dma_oam_transfer_fault",
    "dma_phase_matrix_fault",
    "apu_status_fault",
    "cpu_zero_page_wrap_fault",
    "cpu_indirect_jmp_fault",
    "cpu_addressing_matrix_fault",
    "input_port_matrix_fault",
    "ppu_nmi_timeout_fault",
    "ppu_read_buffer_fault",
    "ppu_nametable_mirroring_fault",
    "ppu_sprite_overflow_fault",
    "ppu_sprite_priority_fault",
    "ppu_sprite_zero_hit_fault",
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
        self.verify_coverage_ledger(as_dict(run.get("coverage_ledger")), debug_entries)
        self.verify_telemetry_catalog(as_dict(run.get("telemetry_catalog")))
        code_map_domains = self.verify_code_map(as_dict(run.get("code_map")), debug_entries)
        investigation_routes = self.verify_investigation_plan(
            as_dict(run.get("investigation_plan")),
            debug_entries,
            code_map_domains,
        )
        self.verify_scenario_dossiers(
            as_dict(run.get("scenario_dossiers")),
            debug_entries,
            investigation_routes,
        )
        self.verify_comparison(run.get("comparison"))
        self.verify_replay(run.get("replay"))

        return {
            "run_json": str(self.run_json),
            "suite_dir": str(self.suite_dir),
            "observability_run_schema_version": run.get("observability_run_schema_version"),
            "scenario_count": as_dict(run.get("suite")).get("scenario_count"),
            "debug_index_entries": len(debug_entries),
            "hypothesis_count": as_dict(run.get("analysis")).get("hypothesis_count"),
            "coverage_ledger_tests": as_dict(run.get("coverage_ledger")).get("test_count"),
            "telemetry_catalog_probes": as_dict(run.get("telemetry_catalog")).get("probe_count"),
            "telemetry_catalog_event_kinds": as_dict(run.get("telemetry_catalog")).get(
                "event_kind_count"
            ),
            "code_map_domains": len(code_map_domains),
            "investigation_routes": len(investigation_routes),
            "scenario_dossiers": as_dict(run.get("scenario_dossiers")).get("dossier_count"),
            "actionable_dossiers": as_dict(run.get("scenario_dossiers")).get(
                "actionable_dossier_count"
            ),
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
            "## Coverage Ledger",
            "## Telemetry Catalog",
            "## Diagnostic Code Map",
            "## Investigation Plan",
            "## Scenario Dossiers",
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

    def verify_coverage_ledger(
        self, ledger: dict[str, Any], debug_entries: list[dict[str, Any]]
    ) -> None:
        self.expect_equal(
            ledger.get("diagnostic_coverage_ledger_schema_version"),
            EXPECTED_COVERAGE_LEDGER_SCHEMA,
            "diagnostic coverage ledger schema version",
        )
        self.expect_equal(ledger.get("status"), "passed", "diagnostic coverage ledger status")
        self.expect_equal(
            ledger.get("recommended_exit_code"),
            0,
            "diagnostic coverage ledger recommended_exit_code",
        )
        self.expect_equal(
            ledger.get("scenario_count"),
            EXPECTED_SCENARIO_COUNT,
            "diagnostic coverage ledger scenario_count",
        )
        self.expect_equal(
            ledger.get("test_count"),
            EXPECTED_CARTRIDGE_TEST_COUNT,
            "diagnostic coverage ledger test_count",
        )
        self.expect_equal(
            ledger.get("happy_path_scenario_count"),
            EXPECTED_PASS_SCENARIO_COUNT,
            "diagnostic coverage ledger happy_path_scenario_count",
        )
        self.expect_equal(
            ledger.get("negative_fixture_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "diagnostic coverage ledger negative_fixture_count",
        )
        self.expect_equal(
            ledger.get("known_gap_count"),
            EXPECTED_COVERAGE_GAP_COUNT,
            "diagnostic coverage ledger known_gap_count",
        )
        self.expect_equal(ledger.get("errors"), [], "diagnostic coverage ledger errors")
        self.verify_artifact_map(as_dict(ledger.get("artifacts")), "diagnostic coverage ledger")

        artifact_json = self.resolve_existing_file(
            as_dict(ledger.get("artifacts")).get("diagnostic_coverage_ledger_json"),
            "diagnostic coverage ledger JSON",
        )
        if artifact_json:
            artifact_data = self.read_json_file(artifact_json, "diagnostic coverage ledger JSON")
            self.expect_equal(
                artifact_data.get("diagnostic_coverage_ledger_schema_version"),
                EXPECTED_COVERAGE_LEDGER_SCHEMA,
                "diagnostic coverage ledger artifact schema version",
            )

        posture = as_dict(ledger.get("coverage_posture"))
        self.expect_equal(posture.get("only_happy_paths"), False, "coverage posture only_happy_paths")
        self.expect_equal(
            len(as_list(posture.get("happy_path_scenario_ids"))),
            EXPECTED_PASS_SCENARIO_COUNT,
            "coverage posture happy_path_scenario_ids",
        )
        self.expect_equal(
            len(as_list(posture.get("negative_fixture_scenario_ids"))),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "coverage posture negative_fixture_scenario_ids",
        )

        tests = [test for test in as_list(ledger.get("tests")) if isinstance(test, dict)]
        self.expect_equal(
            len(tests),
            EXPECTED_CARTRIDGE_TEST_COUNT,
            "diagnostic coverage ledger tests",
        )
        self.expect_equal(
            {test.get("id") for test in tests},
            set(range(1, EXPECTED_CARTRIDGE_TEST_COUNT + 1)),
            "diagnostic coverage ledger test ids",
        )
        for test in tests:
            label = f"diagnostic coverage ledger test {test.get('id')}"
            self.expect_nonempty_string(test.get("name"), f"{label} name")
            self.expect_nonempty_string(test.get("subsystem"), f"{label} subsystem")
            self.expect_nonempty_string(test.get("tier"), f"{label} tier")
            self.expect_nonempty_string(test.get("intent"), f"{label} intent")
            self.expect_equal(test.get("baseline_passed"), True, f"{label} baseline_passed")

        negative_fixtures = [
            fixture
            for fixture in as_list(ledger.get("negative_fixtures"))
            if isinstance(fixture, dict)
        ]
        self.expect_equal(
            len(negative_fixtures),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "diagnostic coverage ledger negative_fixtures",
        )
        fixture_ids = {fixture.get("scenario_id") for fixture in negative_fixtures}
        debug_fixture_ids = {
            entry.get("scenario_id")
            for entry in debug_entries
            if entry.get("role") == "expected_failure_fixture"
        }
        self.expect_equal(fixture_ids, debug_fixture_ids, "coverage ledger negative fixture ids")
        fixture_domains = {fixture.get("expected_focus_domain") for fixture in negative_fixtures}
        debug_domains = {
            as_dict(entry.get("debug_focus")).get("focus_domain")
            for entry in debug_entries
            if entry.get("role") == "expected_failure_fixture"
        }
        self.expect_equal(fixture_domains, debug_domains, "coverage ledger negative domains")
        for fixture in negative_fixtures:
            label = f"diagnostic coverage ledger fixture {fixture.get('scenario_id')}"
            self.expect_nonempty_string(
                fixture.get("expected_health"),
                f"{label} expected_health",
            )
            self.expect_nonempty_string(
                fixture.get("expected_focus_domain"),
                f"{label} expected_focus_domain",
            )
            self.expect_nonempty_list(fixture.get("failed_probe_ids"), f"{label} failed_probe_ids")
            self.expect_nonempty_list(fixture.get("replay_args"), f"{label} replay_args")
            self.expect_existing_file(fixture.get("primary_artifact"), f"{label} primary_artifact")
            self.expect_existing_file(fixture.get("telemetry_json"), f"{label} telemetry_json")
            self.expect_existing_file(fixture.get("comparison_json"), f"{label} comparison_json")

        gaps = [gap for gap in as_list(ledger.get("coverage_gaps")) if isinstance(gap, dict)]
        self.expect_equal(
            len(gaps),
            EXPECTED_COVERAGE_GAP_COUNT,
            "diagnostic coverage ledger coverage_gaps",
        )
        for gap in gaps:
            label = f"diagnostic coverage ledger gap {gap.get('id')}"
            self.expect_nonempty_string(gap.get("id"), f"{label} id")
            self.expect_nonempty_string(gap.get("subsystem"), f"{label} subsystem")
            self.expect_nonempty_string(gap.get("risk"), f"{label} risk")
            self.expect_nonempty_string(
                gap.get("missing_coverage"),
                f"{label} missing_coverage",
            )
            self.expect_nonempty_string(
                gap.get("suggested_next_test"),
                f"{label} suggested_next_test",
            )

    def verify_telemetry_catalog(self, catalog: dict[str, Any]) -> None:
        self.expect_equal(
            catalog.get("diagnostic_telemetry_catalog_schema_version"),
            EXPECTED_TELEMETRY_CATALOG_SCHEMA,
            "diagnostic telemetry catalog schema version",
        )
        self.expect_equal(catalog.get("status"), "passed", "diagnostic telemetry catalog status")
        self.expect_equal(
            catalog.get("recommended_exit_code"),
            0,
            "diagnostic telemetry catalog recommended_exit_code",
        )
        self.expect_equal(
            catalog.get("telemetry_schema_version"),
            EXPECTED_TELEMETRY_SCHEMA,
            "diagnostic telemetry catalog telemetry_schema_version",
        )
        self.expect_equal(
            catalog.get("test_count"),
            EXPECTED_CARTRIDGE_TEST_COUNT,
            "diagnostic telemetry catalog test_count",
        )
        self.expect_equal(
            catalog.get("probe_count"),
            EXPECTED_PROBE_COUNT,
            "diagnostic telemetry catalog probe_count",
        )
        self.expect_equal(
            catalog.get("event_kind_count"),
            EXPECTED_EVENT_KIND_COUNT,
            "diagnostic telemetry catalog event_kind_count",
        )
        self.expect_equal(
            catalog.get("timeline_entry_count"),
            EXPECTED_CARTRIDGE_TEST_COUNT,
            "diagnostic telemetry catalog timeline_entry_count",
        )
        self.expect_equal(
            catalog.get("trace_retained_instruction_count"),
            EXPECTED_TRACE_RETAINED_INSTRUCTION_COUNT,
            "diagnostic telemetry catalog trace_retained_instruction_count",
        )
        self.expect_equal(
            catalog.get("signal_family_count"),
            EXPECTED_SIGNAL_FAMILY_COUNT,
            "diagnostic telemetry catalog signal_family_count",
        )
        self.expect_equal(catalog.get("errors"), [], "diagnostic telemetry catalog errors")
        self.verify_artifact_map(as_dict(catalog.get("artifacts")), "diagnostic telemetry catalog")

        artifact_json = self.resolve_existing_file(
            as_dict(catalog.get("artifacts")).get("diagnostic_telemetry_catalog_json"),
            "diagnostic telemetry catalog JSON",
        )
        if artifact_json:
            artifact_data = self.read_json_file(artifact_json, "diagnostic telemetry catalog JSON")
            self.expect_equal(
                artifact_data,
                catalog,
                "diagnostic telemetry catalog artifact payload",
            )
            self.expect_equal(
                artifact_data.get("diagnostic_telemetry_catalog_schema_version"),
                EXPECTED_TELEMETRY_CATALOG_SCHEMA,
                "diagnostic telemetry catalog artifact schema version",
            )

        families = [
            family
            for family in as_list(catalog.get("signal_families"))
            if isinstance(family, dict)
        ]
        self.expect_equal(
            {family.get("id") for family in families},
            EXPECTED_SIGNAL_FAMILIES,
            "diagnostic telemetry catalog signal families",
        )
        for family in families:
            label = f"diagnostic telemetry catalog family {family.get('id')}"
            self.expect_equal(family.get("available"), True, f"{label} available")
            self.expect_nonempty_string(family.get("purpose"), f"{label} purpose")
            self.expect_nonempty_string(family.get("first_artifact"), f"{label} first_artifact")
            self.expect_nonempty_list(family.get("telemetry_paths"), f"{label} telemetry_paths")
            self.expect_nonempty_string(family.get("ai_usage"), f"{label} ai_usage")

        probes = [
            probe
            for probe in as_list(catalog.get("probe_catalog"))
            if isinstance(probe, dict)
        ]
        self.expect_equal(
            len(probes),
            EXPECTED_PROBE_COUNT,
            "diagnostic telemetry catalog probe_catalog",
        )
        probe_ids = {probe.get("id") for probe in probes}
        for expected_probe in (
            "runtime.completed",
            "cartridge.status.pass",
            "cartridge.test.21.result",
            "cartridge.test.25.result",
            "cartridge.test.26.result",
            "cartridge.test.27.result",
            "ppu.sprite_priority.samples",
            "ppu.sprite_overflow.status",
            "ppu.sprite_zero_hit.status",
            "dma.dmc_stall_phase",
            "apu.sample_count",
        ):
            self.expect_in(expected_probe, list(probe_ids), "diagnostic telemetry catalog probes")
        for probe in probes:
            label = f"diagnostic telemetry catalog probe {probe.get('id')}"
            self.expect_nonempty_string(probe.get("id"), f"{label} id")
            self.expect_nonempty_string(probe.get("source"), f"{label} source")
            self.expect_nonempty_string(probe.get("status"), f"{label} status")
            self.expect_nonempty_string(probe.get("description"), f"{label} description")
            self.expect_nonempty_string(probe.get("expected"), f"{label} expected")
            self.expect_nonempty_string(probe.get("observed"), f"{label} observed")
            self.expect_nonempty_string(probe.get("likely_domain"), f"{label} likely_domain")

        event_kinds = [
            event_kind
            for event_kind in as_list(catalog.get("event_kind_catalog"))
            if isinstance(event_kind, dict)
        ]
        self.expect_equal(
            {event_kind.get("kind") for event_kind in event_kinds},
            EXPECTED_EVENT_KINDS,
            "diagnostic telemetry catalog event kinds",
        )
        for event_kind in event_kinds:
            self.expect_nonempty_string(
                event_kind.get("kind"),
                "diagnostic telemetry catalog event kind",
            )
            if not isinstance(event_kind.get("count"), int) or event_kind.get("count", 0) < 1:
                self.errors.append(
                    f"diagnostic telemetry catalog event {event_kind.get('kind')} count must be positive"
                )

        test_signals = [
            test for test in as_list(catalog.get("test_signals")) if isinstance(test, dict)
        ]
        self.expect_equal(
            len(test_signals),
            EXPECTED_CARTRIDGE_TEST_COUNT,
            "diagnostic telemetry catalog test_signals",
        )
        self.expect_equal(
            {test.get("id") for test in test_signals},
            set(range(1, EXPECTED_CARTRIDGE_TEST_COUNT + 1)),
            "diagnostic telemetry catalog test signal ids",
        )
        for test in test_signals:
            label = f"diagnostic telemetry catalog test {test.get('id')}"
            self.expect_nonempty_string(test.get("name"), f"{label} name")
            self.expect_nonempty_string(test.get("subsystem"), f"{label} subsystem")
            self.expect_nonempty_string(test.get("result_probe_id"), f"{label} result_probe_id")
            self.expect_equal(test.get("result_probe_present"), True, f"{label} result_probe_present")
            self.expect_equal(test.get("timeline_present"), True, f"{label} timeline_present")
            self.expect_nonempty_list(test.get("probe_ids"), f"{label} probe_ids")

        trace = as_dict(catalog.get("trace_catalog"))
        self.expect_equal(
            trace.get("retained_instruction_count"),
            EXPECTED_TRACE_RETAINED_INSTRUCTION_COUNT,
            "diagnostic telemetry catalog trace retained count",
        )
        self.expect_equal(trace.get("truncated"), True, "diagnostic telemetry catalog trace truncated")
        trace_fields = set(as_list(trace.get("tail_fields")))
        for expected_field in ("instruction", "symbol", "cpu", "diagnostic_ram"):
            self.expect_in(expected_field, list(trace_fields), "diagnostic telemetry catalog trace fields")

    def verify_code_map(
        self, code_map: dict[str, Any], debug_entries: list[dict[str, Any]]
    ) -> set[Any]:
        self.expect_equal(
            code_map.get("diagnostic_code_map_schema_version"),
            EXPECTED_CODE_MAP_SCHEMA,
            "diagnostic code map schema version",
        )
        self.expect_equal(code_map.get("status"), "passed", "diagnostic code map status")
        self.expect_equal(
            code_map.get("recommended_exit_code"),
            0,
            "diagnostic code map recommended_exit_code",
        )
        self.expect_equal(
            code_map.get("scenario_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "diagnostic code map scenario_count",
        )
        self.expect_equal(
            code_map.get("focus_domain_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "diagnostic code map focus_domain_count",
        )
        self.expect_equal(
            code_map.get("unknown_focus_domains"),
            [],
            "diagnostic code map unknown_focus_domains",
        )
        self.expect_equal(code_map.get("errors"), [], "diagnostic code map errors")
        self.verify_artifact_map(as_dict(code_map.get("artifacts")), "diagnostic code map")

        artifact_json = self.resolve_existing_file(
            as_dict(code_map.get("artifacts")).get("diagnostic_code_map_json"),
            "diagnostic code map JSON",
        )
        if artifact_json:
            artifact_data = self.read_json_file(artifact_json, "diagnostic code map JSON")
            self.expect_equal(
                artifact_data.get("diagnostic_code_map_schema_version"),
                EXPECTED_CODE_MAP_SCHEMA,
                "diagnostic code map artifact schema version",
            )

        focus_entries = as_list(code_map.get("focus_domains"))
        self.expect_equal(
            len(focus_entries),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "diagnostic code map focus domain entries",
        )
        code_map_domains = {
            entry.get("focus_domain")
            for entry in focus_entries
            if isinstance(entry, dict)
        }
        debug_domains = {
            as_dict(entry.get("debug_focus")).get("focus_domain")
            for entry in debug_entries
            if entry.get("role") == "expected_failure_fixture"
        }
        self.expect_equal(code_map_domains, debug_domains, "diagnostic code map domains")

        for entry in focus_entries:
            if not isinstance(entry, dict):
                self.errors.append("diagnostic code map entries must be objects")
                continue
            domain = entry.get("focus_domain")
            label = f"diagnostic code map {domain}"
            self.expect_nonempty_string(domain, f"{label} focus_domain")
            self.expect_nonempty_string(entry.get("description"), f"{label} description")
            self.expect_nonempty_list(entry.get("scenario_ids"), f"{label} scenario_ids")
            self.expect_nonempty_string(entry.get("primary_artifact"), f"{label} primary_artifact")
            self.expect_nonempty_list(entry.get("replay_args"), f"{label} replay_args")
            self.expect_nonempty_list(
                entry.get("suggested_commands"),
                f"{label} suggested_commands",
            )
            for group in ("source_files", "test_files", "diagnostic_files"):
                records = as_list(entry.get(group))
                self.expect_nonempty_list(records, f"{label} {group}")
                for record in records:
                    if not isinstance(record, dict):
                        self.errors.append(f"{label} {group} entries must be objects")
                        continue
                    self.expect_equal(record.get("exists"), True, f"{label} {group} exists")
                    self.expect_existing_file(record.get("path"), f"{label} {group} path")
        return code_map_domains

    def verify_investigation_plan(
        self,
        plan: dict[str, Any],
        debug_entries: list[dict[str, Any]],
        code_map_domains: set[Any],
    ) -> list[dict[str, Any]]:
        self.expect_equal(
            plan.get("investigation_plan_schema_version"),
            EXPECTED_INVESTIGATION_PLAN_SCHEMA,
            "investigation plan schema version",
        )
        self.expect_equal(plan.get("status"), "passed", "investigation plan status")
        self.expect_equal(
            plan.get("recommended_exit_code"),
            0,
            "investigation plan recommended_exit_code",
        )
        self.expect_equal(
            plan.get("route_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "investigation plan route_count",
        )
        self.expect_equal(
            plan.get("focus_domain_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "investigation plan focus_domain_count",
        )
        self.expect_equal(plan.get("errors"), [], "investigation plan errors")
        self.verify_artifact_map(as_dict(plan.get("artifacts")), "investigation plan")
        self.verify_artifact_map(
            as_dict(plan.get("source_artifacts")),
            "investigation plan source artifacts",
        )

        artifact_json = self.resolve_existing_file(
            as_dict(plan.get("artifacts")).get("investigation_plan_json"),
            "investigation plan JSON",
        )
        if artifact_json:
            artifact_data = self.read_json_file(artifact_json, "investigation plan JSON")
            self.expect_equal(
                artifact_data.get("investigation_plan_schema_version"),
                EXPECTED_INVESTIGATION_PLAN_SCHEMA,
                "investigation plan artifact schema version",
            )

        routes = [route for route in as_list(plan.get("routes")) if isinstance(route, dict)]
        self.expect_equal(
            len(routes),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "investigation plan routes",
        )
        route_domains = {route.get("focus_domain") for route in routes}
        debug_domains = {
            as_dict(entry.get("debug_focus")).get("focus_domain")
            for entry in debug_entries
            if entry.get("role") == "expected_failure_fixture"
        }
        self.expect_equal(route_domains, debug_domains, "investigation plan domains")
        self.expect_equal(route_domains, code_map_domains, "investigation plan code-map domains")
        self.expect_equal(
            [route.get("rank") for route in routes],
            list(range(1, EXPECTED_ACTIONABLE_SCENARIO_COUNT + 1)),
            "investigation plan route ranks",
        )

        top_route = as_dict(plan.get("top_route"))
        self.expect_nonempty_string(top_route.get("route_id"), "investigation plan top route id")
        self.expect_nonempty_string(
            top_route.get("focus_domain"),
            "investigation plan top route focus_domain",
        )
        self.expect_existing_file(
            top_route.get("primary_artifact"),
            "investigation plan top route primary_artifact",
        )

        expected_scenarios = {
            entry.get("scenario_id")
            for entry in debug_entries
            if entry.get("role") == "expected_failure_fixture"
        }
        route_scenarios = {route.get("primary_scenario_id") for route in routes}
        self.expect_equal(
            route_scenarios,
            expected_scenarios,
            "investigation plan primary scenarios",
        )

        for route in routes:
            domain = route.get("focus_domain")
            label = f"investigation plan {domain}"
            self.expect_nonempty_string(route.get("route_id"), f"{label} route_id")
            self.expect_nonempty_string(domain, f"{label} focus_domain")
            self.expect_nonempty_string(route.get("focus_subsystem"), f"{label} focus_subsystem")
            self.expect_nonempty_string(
                route.get("primary_scenario_id"),
                f"{label} primary_scenario_id",
            )
            self.expect_nonempty_list(route.get("scenario_ids"), f"{label} scenario_ids")
            self.expect_existing_file(route.get("primary_artifact"), f"{label} primary_artifact")
            self.expect_nonempty_list(route.get("replay_args"), f"{label} replay_args")
            self.expect_nonempty_list(
                route.get("suggested_commands"),
                f"{label} suggested_commands",
            )
            self.expect_nonempty_list(route.get("why_this_route"), f"{label} why_this_route")
            self.expect_nonempty_list(route.get("handoff_steps"), f"{label} handoff_steps")
            self.expect_nonempty_list(route.get("stop_conditions"), f"{label} stop_conditions")
            if not as_dict(route.get("debug_anchor")):
                self.errors.append(f"{label} debug_anchor must be an object")

            start_artifacts = as_dict(route.get("start_artifacts"))
            for artifact_name in ("primary_artifact", "triage_json", "telemetry_json"):
                self.expect_existing_file(
                    start_artifacts.get(artifact_name),
                    f"{label} start artifact {artifact_name}",
                )

            for group in ("source_files", "test_files", "diagnostic_files"):
                records = as_list(route.get(group))
                self.expect_nonempty_list(records, f"{label} {group}")
                for record in records:
                    if not isinstance(record, dict):
                        self.errors.append(f"{label} {group} entries must be objects")
                        continue
                    self.expect_equal(record.get("exists"), True, f"{label} {group} exists")
                    self.expect_existing_file(record.get("path"), f"{label} {group} path")

            for step in as_list(route.get("handoff_steps")):
                if not isinstance(step, dict):
                    self.errors.append(f"{label} handoff_steps entries must be objects")
                    continue
                self.expect_nonempty_string(step.get("action"), f"{label} step action")
                self.expect_nonempty_string(step.get("purpose"), f"{label} step purpose")
        return routes

    def verify_scenario_dossiers(
        self,
        summary: dict[str, Any],
        debug_entries: list[dict[str, Any]],
        investigation_routes: list[dict[str, Any]],
    ) -> None:
        self.expect_equal(
            summary.get("scenario_dossiers_schema_version"),
            EXPECTED_SCENARIO_DOSSIERS_SCHEMA,
            "scenario dossiers schema version",
        )
        self.expect_equal(summary.get("status"), "passed", "scenario dossiers status")
        self.expect_equal(
            summary.get("recommended_exit_code"),
            0,
            "scenario dossiers recommended_exit_code",
        )
        self.expect_equal(
            summary.get("dossier_count"),
            EXPECTED_SCENARIO_COUNT,
            "scenario dossiers dossier_count",
        )
        self.expect_equal(
            summary.get("actionable_dossier_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "scenario dossiers actionable_dossier_count",
        )
        self.expect_equal(
            summary.get("healthy_dossier_count"),
            EXPECTED_PASS_SCENARIO_COUNT,
            "scenario dossiers healthy_dossier_count",
        )
        self.expect_equal(summary.get("errors"), [], "scenario dossiers errors")
        self.verify_artifact_map(as_dict(summary.get("artifacts")), "scenario dossiers")
        self.verify_artifact_map(
            as_dict(summary.get("source_artifacts")),
            "scenario dossiers source artifacts",
        )

        artifact_json = self.resolve_existing_file(
            as_dict(summary.get("artifacts")).get("scenario_dossiers_json"),
            "scenario dossiers JSON",
        )
        if artifact_json:
            artifact_data = self.read_json_file(artifact_json, "scenario dossiers JSON")
            self.expect_equal(artifact_data, summary, "scenario dossiers artifact payload")
            self.expect_equal(
                artifact_data.get("scenario_dossiers_schema_version"),
                EXPECTED_SCENARIO_DOSSIERS_SCHEMA,
                "scenario dossiers artifact schema version",
            )

        dossiers = [
            dossier
            for dossier in as_list(summary.get("dossiers"))
            if isinstance(dossier, dict)
        ]
        self.expect_equal(len(dossiers), EXPECTED_SCENARIO_COUNT, "scenario dossiers rows")
        dossier_ids = {dossier.get("scenario_id") for dossier in dossiers}
        debug_ids = {entry.get("scenario_id") for entry in debug_entries}
        self.expect_equal(dossier_ids, EXPECTED_SCENARIOS, "scenario dossier scenario ids")
        self.expect_equal(dossier_ids, debug_ids, "scenario dossiers match debug index")

        route_by_scenario = {
            route.get("primary_scenario_id"): route
            for route in investigation_routes
            if isinstance(route.get("primary_scenario_id"), str)
        }
        expected_failure_ids = {
            entry.get("scenario_id")
            for entry in debug_entries
            if entry.get("role") == "expected_failure_fixture"
        }
        healthy_ids = {
            dossier.get("scenario_id")
            for dossier in dossiers
            if dossier.get("health") == "healthy"
        }
        self.expect_equal(
            len(healthy_ids),
            EXPECTED_PASS_SCENARIO_COUNT,
            "scenario dossiers healthy ids",
        )

        for dossier in dossiers:
            scenario_id = dossier.get("scenario_id")
            label = f"scenario dossier {scenario_id}"
            self.expect_nonempty_string(scenario_id, f"{label} scenario_id")
            self.expect_nonempty_string(dossier.get("role"), f"{label} role")
            self.expect_nonempty_string(dossier.get("health"), f"{label} health")
            self.expect_nonempty_string(dossier.get("summary"), f"{label} summary")
            self.expect_nonempty_list(dossier.get("replay_args"), f"{label} replay_args")
            self.expect_nonempty_list(
                dossier.get("signal_family_ids"),
                f"{label} signal_family_ids",
            )
            self.expect_nonempty_list(dossier.get("signal_families"), f"{label} signal_families")
            self.expect_nonempty_list(dossier.get("next_actions"), f"{label} next_actions")

            start_artifacts = as_dict(dossier.get("start_artifacts"))
            for artifact_name in ("triage_json", "telemetry_json", "report_md"):
                self.expect_existing_file(
                    start_artifacts.get(artifact_name),
                    f"{label} start artifact {artifact_name}",
                )
            if scenario_id in expected_failure_ids:
                self.expect_nonempty_string(dossier.get("focus_domain"), f"{label} focus_domain")
                self.expect_nonempty_list(dossier.get("failed_probe_ids"), f"{label} failed_probe_ids")
                route = as_dict(dossier.get("route"))
                expected_route = as_dict(route_by_scenario.get(scenario_id))
                self.expect_equal(
                    route.get("route_id"),
                    expected_route.get("route_id"),
                    f"{label} route_id",
                )
                self.expect_equal(
                    route.get("rank"),
                    expected_route.get("rank"),
                    f"{label} route rank",
                )
                self.expect_existing_file(route.get("primary_artifact"), f"{label} route artifact")
                self.expect_nonempty_list(route.get("suggested_commands"), f"{label} route commands")
                self.expect_nonempty_list(route.get("source_files"), f"{label} route source files")
                self.expect_nonempty_list(route.get("test_files"), f"{label} route test files")
            else:
                self.expect_equal(dossier.get("route"), None, f"{label} route")

            for family in as_list(dossier.get("signal_families")):
                if not isinstance(family, dict):
                    self.errors.append(f"{label} signal families entries must be objects")
                    continue
                self.expect_nonempty_string(family.get("id"), f"{label} signal family id")
                self.expect_nonempty_list(
                    family.get("telemetry_paths"),
                    f"{label} signal telemetry_paths",
                )
                self.expect_nonempty_string(
                    family.get("first_artifact"),
                    f"{label} signal first_artifact",
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

        current = as_dict(comparison.get("current"))
        self.expect_equal(
            current.get("scenario_count"),
            EXPECTED_SCENARIO_COUNT,
            "observability comparison current scenario_count",
        )
        self.expect_equal(
            current.get("hypothesis_count"),
            EXPECTED_ACTIONABLE_SCENARIO_COUNT,
            "observability comparison current hypothesis_count",
        )
        self.verify_artifact_map(as_dict(current.get("artifacts")), "comparison current")

        baseline = as_dict(comparison.get("baseline"))
        baseline_scenario_count = baseline.get("scenario_count")
        if not isinstance(baseline_scenario_count, int) or not (
            1 <= baseline_scenario_count <= EXPECTED_SCENARIO_COUNT
        ):
            self.errors.append(
                "observability comparison baseline scenario_count: "
                f"expected 1..{EXPECTED_SCENARIO_COUNT}, got {baseline_scenario_count}"
            )
        baseline_hypothesis_count = baseline.get("hypothesis_count")
        if not isinstance(baseline_hypothesis_count, int) or not (
            1 <= baseline_hypothesis_count <= EXPECTED_ACTIONABLE_SCENARIO_COUNT
        ):
            self.errors.append(
                "observability comparison baseline hypothesis_count: "
                f"expected 1..{EXPECTED_ACTIONABLE_SCENARIO_COUNT}, got {baseline_hypothesis_count}"
            )
        self.verify_artifact_map(as_dict(baseline.get("artifacts")), "comparison baseline")

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
            f"coverage_tests={summary['coverage_ledger_tests']} "
            f"telemetry_catalog={summary['telemetry_catalog_probes']}:{summary['telemetry_catalog_event_kinds']} "
            f"code_map={summary['code_map_domains']} "
            f"investigation_routes={summary['investigation_routes']} "
            f"scenario_dossiers={summary['scenario_dossiers']}:{summary['actionable_dossiers']} "
            f"comparison={summary['comparison_verdict'] or '-'} "
            f"replay={summary['replay_scenario'] or '-'}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
