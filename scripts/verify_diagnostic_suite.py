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
EXPECTED_TELEMETRY_SCHEMA = 26
EXPECTED_TRIAGE_SCHEMA = 6
EXPECTED_BUNDLE_SCHEMA = 2
EXPECTED_SCENARIOS = {
    "pass",
    "joypad1_mismatch",
    "joypad2_mismatch",
    "dma_oam_transfer_fault",
    "apu_status_fault",
    "cpu_zero_page_wrap_fault",
    "cpu_indirect_jmp_fault",
    "ppu_nmi_timeout_fault",
    "ppu_read_buffer_fault",
    "ppu_nametable_mirroring_fault",
    "mapper2_bank_switch_fault",
    "mapper2_prg_ram_fault",
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
            12,
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
            12,
            "observer baseline_divergence_count",
        )

        actions = self.expect_list(observer.get("next_actions"), "observer next_actions")
        observations = self.expect_list(observer.get("observations"), "observer observations")
        self.expect_equal(len(actions), 12, "observer next_actions count")
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
            "dma_oam_transfer_fault",
            "apu_status_fault",
            "cpu_zero_page_wrap_fault",
            "cpu_indirect_jmp_fault",
            "ppu_nmi_timeout_fault",
            "ppu_read_buffer_fault",
            "ppu_nametable_mirroring_fault",
            "mapper2_bank_switch_fault",
            "mapper2_prg_ram_fault",
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
            "comparison_difference_count=101",
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

        ppu_mirroring = by_scenario.get("ppu_nametable_mirroring_fault")
        if not isinstance(ppu_mirroring, dict):
            self.errors.append("missing observer action for ppu_nametable_mirroring_fault")
            return

        self.expect_equal(
            ppu_mirroring.get("priority"),
            "known_divergence",
            "PPU mirroring observer action priority",
        )
        self.expect_equal(
            ppu_mirroring.get("action_type"),
            "inspect_known_divergence",
            "PPU mirroring observer action type",
        )
        self.expect_equal(
            ppu_mirroring.get("primary_artifact"),
            "ppu_nametable_mirroring_fault/comparison.json",
            "PPU mirroring observer primary_artifact",
        )
        ppu_mirroring_evidence = self.expect_list(
            ppu_mirroring.get("evidence"), "PPU mirroring observer evidence"
        )
        self.expect_in(
            "focus_domain=ppu.nametables.horizontal_mirroring",
            ppu_mirroring_evidence,
            "PPU mirroring observer evidence",
        )
        self.expect_in(
            "failed_probe_ids=cartridge.status.pass,cartridge.test.17.result",
            ppu_mirroring_evidence,
            "PPU mirroring observer evidence",
        )

        mapper = by_scenario.get("mapper2_bank_switch_fault")
        if not isinstance(mapper, dict):
            self.errors.append("missing observer action for mapper2_bank_switch_fault")
            return

        self.expect_equal(
            mapper.get("priority"),
            "known_divergence",
            "mapper observer action priority",
        )
        self.expect_equal(
            mapper.get("action_type"),
            "inspect_known_divergence",
            "mapper observer action type",
        )
        self.expect_equal(
            mapper.get("primary_artifact"),
            "mapper2_bank_switch_fault/comparison.json",
            "mapper observer primary_artifact",
        )
        mapper_evidence = self.expect_list(
            mapper.get("evidence"), "mapper observer evidence"
        )
        self.expect_in(
            "focus_domain=mapper.uxrom.prg_bank_switch",
            mapper_evidence,
            "mapper observer evidence",
        )
        self.expect_in(
            "failed_probe_ids=cartridge.status.pass,cartridge.test.15.result",
            mapper_evidence,
            "mapper observer evidence",
        )

        prg_ram = by_scenario.get("mapper2_prg_ram_fault")
        if not isinstance(prg_ram, dict):
            self.errors.append("missing observer action for mapper2_prg_ram_fault")
            return

        self.expect_equal(
            prg_ram.get("priority"),
            "known_divergence",
            "mapper PRG RAM observer action priority",
        )
        self.expect_equal(
            prg_ram.get("action_type"),
            "inspect_known_divergence",
            "mapper PRG RAM observer action type",
        )
        self.expect_equal(
            prg_ram.get("primary_artifact"),
            "mapper2_prg_ram_fault/comparison.json",
            "mapper PRG RAM observer primary_artifact",
        )
        prg_ram_evidence = self.expect_list(
            prg_ram.get("evidence"), "mapper PRG RAM observer evidence"
        )
        self.expect_in(
            "focus_domain=mapper.uxrom.prg_ram",
            prg_ram_evidence,
            "mapper PRG RAM observer evidence",
        )
        self.expect_in(
            "failed_probe_ids=cartridge.status.pass,cartridge.test.16.result",
            prg_ram_evidence,
            "mapper PRG RAM observer evidence",
        )

        ppu_nmi = by_scenario.get("ppu_nmi_timeout_fault")
        if not isinstance(ppu_nmi, dict):
            self.errors.append("missing observer action for ppu_nmi_timeout_fault")
            return

        self.expect_equal(
            ppu_nmi.get("priority"),
            "known_divergence",
            "PPU NMI observer action priority",
        )
        self.expect_equal(
            ppu_nmi.get("action_type"),
            "inspect_known_divergence",
            "PPU NMI observer action type",
        )
        self.expect_equal(
            ppu_nmi.get("primary_artifact"),
            "ppu_nmi_timeout_fault/comparison.json",
            "PPU NMI observer primary_artifact",
        )
        ppu_nmi_evidence = self.expect_list(
            ppu_nmi.get("evidence"), "PPU NMI observer evidence"
        )
        self.expect_in(
            "health=timed_out",
            ppu_nmi_evidence,
            "PPU NMI observer evidence",
        )
        self.expect_in(
            "focus_domain=ppu.nmi",
            ppu_nmi_evidence,
            "PPU NMI observer evidence",
        )
        self.expect_in(
            "failed_probe_ids=runtime.completed,cartridge.status.pass,cartridge.test.10.result,ppu.nmi_count",
            ppu_nmi_evidence,
            "PPU NMI observer evidence",
        )

        dma = by_scenario.get("dma_oam_transfer_fault")
        if not isinstance(dma, dict):
            self.errors.append("missing observer action for dma_oam_transfer_fault")
            return

        self.expect_equal(
            dma.get("priority"),
            "known_divergence",
            "DMA observer action priority",
        )
        self.expect_equal(
            dma.get("action_type"),
            "inspect_known_divergence",
            "DMA observer action type",
        )
        self.expect_equal(
            dma.get("primary_artifact"),
            "dma_oam_transfer_fault/comparison.json",
            "DMA observer primary_artifact",
        )
        dma_evidence = self.expect_list(dma.get("evidence"), "DMA observer evidence")
        self.expect_in(
            "focus_domain=dma.oam_transfer",
            dma_evidence,
            "DMA observer evidence",
        )
        self.expect_in(
            "failed_probe_ids=oam.dma_checksum",
            dma_evidence,
            "DMA observer evidence",
        )

        apu = by_scenario.get("apu_status_fault")
        if not isinstance(apu, dict):
            self.errors.append("missing observer action for apu_status_fault")
            return

        self.expect_equal(
            apu.get("priority"),
            "known_divergence",
            "APU observer action priority",
        )
        self.expect_equal(
            apu.get("action_type"),
            "inspect_known_divergence",
            "APU observer action type",
        )
        self.expect_equal(
            apu.get("primary_artifact"),
            "apu_status_fault/comparison.json",
            "APU observer primary_artifact",
        )
        apu_evidence = self.expect_list(apu.get("evidence"), "APU observer evidence")
        self.expect_in(
            "focus_domain=apu.status",
            apu_evidence,
            "APU observer evidence",
        )
        self.expect_in(
            "failed_probe_ids=cartridge.status.pass,cartridge.test.6.result",
            apu_evidence,
            "APU observer evidence",
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

        jmp = by_scenario.get("cpu_indirect_jmp_fault")
        if not isinstance(jmp, dict):
            self.errors.append("missing observer action for cpu_indirect_jmp_fault")
            return

        self.expect_equal(
            jmp.get("priority"),
            "known_divergence",
            "indirect JMP observer action priority",
        )
        self.expect_equal(
            jmp.get("action_type"),
            "inspect_known_divergence",
            "indirect JMP observer action type",
        )
        self.expect_equal(
            jmp.get("primary_artifact"),
            "cpu_indirect_jmp_fault/comparison.json",
            "indirect JMP observer primary_artifact",
        )
        jmp_evidence = self.expect_list(
            jmp.get("evidence"), "indirect JMP observer evidence"
        )
        self.expect_in(
            "focus_domain=cpu.control_flow.indirect_jmp_page_wrap",
            jmp_evidence,
            "indirect JMP observer evidence",
        )
        self.expect_in(
            "failed_probe_ids=cartridge.status.pass,cartridge.test.13.result",
            jmp_evidence,
            "indirect JMP observer evidence",
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
                101,
                "timeout observer comparison_difference_count",
            )
            self.expect_equal(
                timeout.get("next_artifact"),
                "timeout_cycle_limit/comparison.json",
                "timeout observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for timeout_cycle_limit")

        dma = by_scenario.get("dma_oam_transfer_fault")
        if isinstance(dma, dict):
            self.expect_equal(
                dma.get("role"),
                "expected_failure_fixture",
                "DMA observer role",
            )
            self.expect_equal(
                dma.get("outcome"),
                "expected_baseline_divergence",
                "DMA observer outcome",
            )
            self.expect_equal(
                dma.get("health"),
                "host_validation_failed",
                "DMA observer health",
            )
            self.expect_equal(
                dma.get("focus_domain"),
                "dma.oam_transfer",
                "DMA observer focus_domain",
            )
            self.expect_equal(
                dma.get("next_artifact"),
                "dma_oam_transfer_fault/comparison.json",
                "DMA observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for dma_oam_transfer_fault")

        apu = by_scenario.get("apu_status_fault")
        if isinstance(apu, dict):
            self.expect_equal(
                apu.get("role"),
                "expected_failure_fixture",
                "APU observer role",
            )
            self.expect_equal(
                apu.get("outcome"),
                "expected_baseline_divergence",
                "APU observer outcome",
            )
            self.expect_equal(
                apu.get("health"),
                "cartridge_assertion_failed",
                "APU observer health",
            )
            self.expect_equal(
                apu.get("focus_domain"),
                "apu.status",
                "APU observer focus_domain",
            )
            self.expect_equal(
                apu.get("next_artifact"),
                "apu_status_fault/comparison.json",
                "APU observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for apu_status_fault")

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

        ppu_mirroring = by_scenario.get("ppu_nametable_mirroring_fault")
        if isinstance(ppu_mirroring, dict):
            self.expect_equal(
                ppu_mirroring.get("role"),
                "expected_failure_fixture",
                "PPU mirroring observer role",
            )
            self.expect_equal(
                ppu_mirroring.get("outcome"),
                "expected_baseline_divergence",
                "PPU mirroring observer outcome",
            )
            self.expect_equal(
                ppu_mirroring.get("health"),
                "cartridge_assertion_failed",
                "PPU mirroring observer health",
            )
            self.expect_equal(
                ppu_mirroring.get("focus_domain"),
                "ppu.nametables.horizontal_mirroring",
                "PPU mirroring observer focus_domain",
            )
            self.expect_equal(
                ppu_mirroring.get("next_artifact"),
                "ppu_nametable_mirroring_fault/comparison.json",
                "PPU mirroring observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for ppu_nametable_mirroring_fault")

        mapper = by_scenario.get("mapper2_bank_switch_fault")
        if isinstance(mapper, dict):
            self.expect_equal(
                mapper.get("role"),
                "expected_failure_fixture",
                "mapper observer role",
            )
            self.expect_equal(
                mapper.get("outcome"),
                "expected_baseline_divergence",
                "mapper observer outcome",
            )
            self.expect_equal(
                mapper.get("health"),
                "cartridge_assertion_failed",
                "mapper observer health",
            )
            self.expect_equal(
                mapper.get("focus_domain"),
                "mapper.uxrom.prg_bank_switch",
                "mapper observer focus_domain",
            )
            self.expect_equal(
                mapper.get("next_artifact"),
                "mapper2_bank_switch_fault/comparison.json",
                "mapper observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for mapper2_bank_switch_fault")

        prg_ram = by_scenario.get("mapper2_prg_ram_fault")
        if isinstance(prg_ram, dict):
            self.expect_equal(
                prg_ram.get("role"),
                "expected_failure_fixture",
                "mapper PRG RAM observer role",
            )
            self.expect_equal(
                prg_ram.get("outcome"),
                "expected_baseline_divergence",
                "mapper PRG RAM observer outcome",
            )
            self.expect_equal(
                prg_ram.get("health"),
                "cartridge_assertion_failed",
                "mapper PRG RAM observer health",
            )
            self.expect_equal(
                prg_ram.get("focus_domain"),
                "mapper.uxrom.prg_ram",
                "mapper PRG RAM observer focus_domain",
            )
            self.expect_equal(
                prg_ram.get("next_artifact"),
                "mapper2_prg_ram_fault/comparison.json",
                "mapper PRG RAM observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for mapper2_prg_ram_fault")

        ppu_nmi = by_scenario.get("ppu_nmi_timeout_fault")
        if isinstance(ppu_nmi, dict):
            self.expect_equal(
                ppu_nmi.get("role"),
                "expected_failure_fixture",
                "PPU NMI observer role",
            )
            self.expect_equal(
                ppu_nmi.get("outcome"),
                "expected_baseline_divergence",
                "PPU NMI observer outcome",
            )
            self.expect_equal(
                ppu_nmi.get("health"),
                "timed_out",
                "PPU NMI observer health",
            )
            self.expect_equal(
                ppu_nmi.get("focus_domain"),
                "ppu.nmi",
                "PPU NMI observer focus_domain",
            )
            self.expect_equal(
                ppu_nmi.get("next_artifact"),
                "ppu_nmi_timeout_fault/comparison.json",
                "PPU NMI observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for ppu_nmi_timeout_fault")

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

        jmp = by_scenario.get("cpu_indirect_jmp_fault")
        if isinstance(jmp, dict):
            self.expect_equal(
                jmp.get("role"),
                "expected_failure_fixture",
                "indirect JMP observer role",
            )
            self.expect_equal(
                jmp.get("outcome"),
                "expected_baseline_divergence",
                "indirect JMP observer outcome",
            )
            self.expect_equal(
                jmp.get("focus_domain"),
                "cpu.control_flow.indirect_jmp_page_wrap",
                "indirect JMP observer focus_domain",
            )
            self.expect_equal(
                jmp.get("next_artifact"),
                "cpu_indirect_jmp_fault/comparison.json",
                "indirect JMP observer next_artifact",
            )
        else:
            self.errors.append("missing observer observation for cpu_indirect_jmp_fault")

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
