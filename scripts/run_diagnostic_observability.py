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
OBSERVABILITY_COMPARISON_SCHEMA_VERSION = 1
DIAGNOSTIC_COVERAGE_LEDGER_SCHEMA_VERSION = 1
DIAGNOSTIC_TELEMETRY_CATALOG_SCHEMA_VERSION = 1
DIAGNOSTIC_CODE_MAP_SCHEMA_VERSION = 1
INVESTIGATION_PLAN_SCHEMA_VERSION = 1
SCENARIO_DOSSIERS_SCHEMA_VERSION = 1
OUTPUT_TAIL_LINES = 80

DIAGNOSTIC_SUPPORT_FILES = [
    "src/diagnostic.rs",
    "src/bin/oxidenes-diagnostic.rs",
    "tests/diagnostic_cartridge_tests.rs",
    "tests/diagnostic_cli_tests.rs",
]

FOCUS_DOMAIN_CODE_MAP = {
    "apu.status": {
        "subsystem": "apu",
        "description": "APU status register and channel-enable behavior observed through $4015.",
        "source_files": ["src/apu.rs", "src/bus.rs"],
        "test_files": ["tests/apu_tests.rs"],
        "search_terms": ["0x4015", "status", "pub fn read", "self.apu.read"],
    },
    "cpu.addressing.zero_page_x_wrap": {
        "subsystem": "cpu",
        "description": "6502 zero-page indexed addressing wraparound for reads and writes.",
        "source_files": ["src/cpu.rs"],
        "test_files": ["tests/cpu_tests.rs"],
        "search_terms": [
            "AddressingMode::ZeroPageX",
            "wrapping_add(self.x)",
            "AddressingMode::ZeroPage",
            "cpu_zero_page_index_wrap",
        ],
    },
    "cpu.control_flow.indirect_jmp_page_wrap": {
        "subsystem": "cpu",
        "description": "Original 6502 indirect JMP page-wrap control-flow behavior.",
        "source_files": ["src/cpu.rs"],
        "test_files": ["tests/cpu_tests.rs"],
        "search_terms": ["JMP", "indirect", "page wrap", "$04FF"],
    },
    "cpu.addressing.page_cross_load": {
        "subsystem": "cpu",
        "description": "6502 load-addressing matrix cases that cross CPU pages for absolute,X and indirect,Y effective addresses.",
        "source_files": ["src/cpu.rs"],
        "test_files": ["tests/cpu_tests.rs"],
        "search_terms": [
            "AddressingMode::AbsoluteX",
            "AddressingMode::IndirectY",
            "page_crossed",
            "cpu_addressing_mode_matrix",
        ],
    },
    "dma.oam_transfer": {
        "subsystem": "dma",
        "description": "OAM DMA transfer, CPU stall bucket, and DMC overlap telemetry.",
        "source_files": ["src/bus.rs", "src/apu.rs", "src/ppu.rs"],
        "test_files": ["tests/bus_tests.rs", "tests/ppu_tests.rs"],
        "search_terms": ["oam_dma", "DMC", "$4014", "dma_oam_transfer"],
    },
    "dma.oam_phase_matrix": {
        "subsystem": "dma",
        "description": "Paired OAM DMA transfers that exercise odd/even start-phase stall buckets.",
        "source_files": ["src/bus.rs", "src/cpu.rs", "src/diagnostic.rs"],
        "test_files": ["tests/bus_tests.rs", "tests/diagnostic_cartridge_tests.rs"],
        "search_terms": [
            "oam_dma_phase_matrix",
            "dma_phase_matrix",
            "dma_active",
            "$4014",
            "oam_dma_active_cycle_buckets",
        ],
    },
    "emulator.progress_or_infinite_loop": {
        "subsystem": "emulator",
        "description": "Headless diagnostic progress, timeout, reset, and terminal loop detection.",
        "source_files": ["src/diagnostic.rs", "src/cpu.rs", "src/bus.rs", "src/ppu.rs"],
        "test_files": ["tests/diagnostic_cartridge_tests.rs"],
        "search_terms": ["max_cycles", "timed_out", "hang", "runtime.completed"],
    },
    "joypad.strobe_high_hold": {
        "subsystem": "joypad",
        "description": "Player-1 $4016 strobe-high repeated-read hold behavior.",
        "source_files": ["src/joypad.rs", "src/bus.rs"],
        "test_files": ["tests/joypad_tests.rs"],
        "search_terms": ["strobe", "button_index", "button_status", "joypad1.write"],
    },
    "joypad.strobe_reset": {
        "subsystem": "joypad",
        "description": "Player-1 mid-stream strobe reset and serial read-index behavior.",
        "source_files": ["src/joypad.rs", "src/bus.rs"],
        "test_files": ["tests/joypad_tests.rs"],
        "search_terms": ["strobe", "read_index", "$4016", "joypad_strobe_reset"],
    },
    "joypad.strobe_shift": {
        "subsystem": "joypad",
        "description": "Player-1 $4016 strobe and serial button shift reads.",
        "source_files": ["src/joypad.rs", "src/bus.rs"],
        "test_files": ["tests/joypad_tests.rs"],
        "search_terms": ["read", "write", "$4016", "joypad1"],
    },
    "joypad2.strobe_shift": {
        "subsystem": "joypad",
        "description": "Player-2 $4017 strobe and serial button shift reads.",
        "source_files": ["src/joypad.rs", "src/bus.rs"],
        "test_files": ["tests/joypad_tests.rs"],
        "search_terms": ["joypad2", "$4017", "strobe", "read"],
    },
    "joypad.input_port_matrix": {
        "subsystem": "joypad",
        "description": "Combined $4016/$4017 strobe-high, serial-shift, and overread matrix behavior.",
        "source_files": ["src/joypad.rs", "src/bus.rs"],
        "test_files": ["tests/joypad_tests.rs", "tests/diagnostic_cartridge_tests.rs"],
        "search_terms": [
            "$4016",
            "$4017",
            "input_port_serial_matrix",
            "button_index",
            "set_button_pressed",
        ],
    },
    "mapper.uxrom.prg_bank_switch": {
        "subsystem": "mapper",
        "description": "Mapper 2/UXROM switchable PRG bank selection and fixed-bank reads.",
        "source_files": ["src/mapper.rs", "src/cartridge.rs", "src/bus.rs"],
        "test_files": ["tests/mapper_tests.rs", "tests/cartridge_tests.rs"],
        "search_terms": ["Uxrom", "Mapper 2", "bank_select", "prg_bank_switch"],
    },
    "mapper.uxrom.prg_ram": {
        "subsystem": "mapper",
        "description": "Mapper 2 PRG RAM reads, writes, boundaries, and bank-select persistence.",
        "source_files": ["src/mapper.rs", "src/cartridge.rs", "src/bus.rs"],
        "test_files": ["tests/mapper_tests.rs", "tests/cartridge_tests.rs"],
        "search_terms": ["prg_ram", "$6000", "$7FFF", "mapper2_prg_ram"],
    },
    "ppu.nametables.horizontal_mirroring": {
        "subsystem": "ppu",
        "description": "Mapper-declared horizontal nametable mirroring through PPUDATA.",
        "source_files": ["src/ppu.rs", "src/cartridge.rs", "src/mapper.rs"],
        "test_files": ["tests/ppu_tests.rs", "tests/mapper_tests.rs"],
        "search_terms": ["horizontal_mirroring", "nametable", "$2000", "$2400"],
    },
    "ppu.nmi": {
        "subsystem": "ppu",
        "description": "PPUCTRL NMI enable, vblank NMI delivery, and rendered-frame progress.",
        "source_files": ["src/ppu.rs", "src/bus.rs"],
        "test_files": ["tests/ppu_tests.rs"],
        "search_terms": ["nmi", "vblank", "PPUCTRL", "ppu_nmi"],
    },
    "ppu.sprite_zero_hit": {
        "subsystem": "ppu",
        "description": "Sprite-zero-hit signaling when sprite 0 overlaps a non-transparent background pixel.",
        "source_files": ["src/ppu.rs", "src/bus.rs", "src/diagnostic.rs"],
        "test_files": ["tests/ppu_tests.rs", "tests/diagnostic_cartridge_tests.rs"],
        "search_terms": [
            "sprite_zero_hit",
            "sprite_zero_being_rendered",
            "PPUSTATUS",
            "0x2002",
            "sprite 0",
        ],
    },
    "ppu.sprite_overflow": {
        "subsystem": "ppu",
        "description": "Sprite overflow signaling when more than eight in-range sprites share a scanline.",
        "source_files": ["src/ppu.rs", "src/bus.rs", "src/diagnostic.rs"],
        "test_files": ["tests/ppu_tests.rs", "tests/diagnostic_cartridge_tests.rs"],
        "search_terms": [
            "sprite_count",
            "sprite overflow",
            "PPUSTATUS",
            "0x2002",
            "secondary OAM",
        ],
    },
    "ppu.sprite_priority": {
        "subsystem": "ppu",
        "description": "Sprite/background priority pixel muxing for front-priority and behind-background sprites.",
        "source_files": ["src/ppu.rs", "src/bus.rs", "src/diagnostic.rs"],
        "test_files": ["tests/ppu_tests.rs", "tests/diagnostic_cartridge_tests.rs"],
        "search_terms": [
            "fg_priority",
            "sprite_priority",
            "priority multiplexer",
            "ppu.sprite_priority",
            "frame_data",
        ],
    },
    "ppu.registers.ppudata_buffer": {
        "subsystem": "ppu",
        "description": "Non-palette PPUDATA read buffering and address auto-increment.",
        "source_files": ["src/ppu.rs", "src/bus.rs"],
        "test_files": ["tests/ppu_tests.rs"],
        "search_terms": ["ppu_data_buffer", "0x2007", "pub fn cpu_read", "self.ppu.cpu_read"],
    },
    "ppu.registers.ppudata_increment_32": {
        "subsystem": "ppu",
        "description": "PPUCTRL bit-2 PPUDATA increment-by-32 behavior.",
        "source_files": ["src/ppu.rs", "src/bus.rs"],
        "test_files": ["tests/ppu_tests.rs"],
        "search_terms": ["self.ctrl & 0x04", "wrapping_add", "0x2007", "pub fn cpu_write"],
    },
    "ppu.registers.status_latch_reset": {
        "subsystem": "ppu",
        "description": "PPUSTATUS read side effect that resets the PPUADDR/PPUSCROLL write latch.",
        "source_files": ["src/ppu.rs", "src/bus.rs"],
        "test_files": ["tests/ppu_tests.rs"],
        "search_terms": ["addr_latch", "self.w = false", "0x2002", "0x2006"],
    },
}

SCENARIO_TEST_FILTERS = {
    "apu_status_fault": "generated_diagnostic_cartridge_localizes_intentional_apu_status_failure",
    "cpu_indirect_jmp_fault": "generated_diagnostic_cartridge_localizes_intentional_cpu_indirect_jmp_failure",
    "cpu_addressing_matrix_fault": "generated_diagnostic_cartridge_localizes_intentional_cpu_addressing_matrix_failure",
    "cpu_zero_page_wrap_fault": "generated_diagnostic_cartridge_localizes_intentional_cpu_zero_page_wrap_failure",
    "dma_oam_transfer_fault": "generated_diagnostic_cartridge_localizes_intentional_dma_oam_transfer_failure",
    "dma_phase_matrix_fault": "generated_diagnostic_cartridge_localizes_intentional_dma_phase_matrix_failure",
    "joypad1_mismatch": "generated_diagnostic_cartridge_localizes_intentional_joypad_failure",
    "joypad2_mismatch": "generated_diagnostic_cartridge_localizes_intentional_joypad2_failure",
    "input_port_matrix_fault": "generated_diagnostic_cartridge_localizes_intentional_input_port_matrix_failure",
    "joypad_strobe_high_hold_fault": "generated_diagnostic_cartridge_localizes_intentional_joypad_strobe_high_hold_failure",
    "joypad_strobe_reset_fault": "generated_diagnostic_cartridge_localizes_intentional_joypad_strobe_reset_failure",
    "mapper2_bank_switch_fault": "generated_diagnostic_cartridge_localizes_intentional_mapper2_bank_switch_failure",
    "mapper2_prg_ram_fault": "generated_diagnostic_cartridge_localizes_intentional_mapper2_prg_ram_failure",
    "ppu_nametable_mirroring_fault": "generated_diagnostic_cartridge_localizes_intentional_ppu_nametable_mirroring_failure",
    "ppu_nmi_timeout_fault": "generated_diagnostic_cartridge_localizes_intentional_ppu_nmi_timeout",
    "ppu_read_buffer_fault": "generated_diagnostic_cartridge_localizes_intentional_ppu_read_buffer_failure",
    "ppu_sprite_overflow_fault": "generated_diagnostic_cartridge_localizes_intentional_ppu_sprite_overflow_failure",
    "ppu_sprite_priority_fault": "generated_diagnostic_cartridge_localizes_intentional_ppu_sprite_priority_failure",
    "ppu_sprite_zero_hit_fault": "generated_diagnostic_cartridge_localizes_intentional_ppu_sprite_zero_hit_failure",
    "ppu_status_latch_reset_fault": "generated_diagnostic_cartridge_localizes_intentional_ppu_status_latch_reset_failure",
    "ppu_vram_increment_32_fault": "generated_diagnostic_cartridge_localizes_intentional_ppu_vram_increment_32_failure",
    "timeout_cycle_limit": "generated_diagnostic_cartridge_localizes_timeout",
}


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
    diagnostic_coverage_ledger: dict[str, Any] | None,
    diagnostic_telemetry_catalog: dict[str, Any] | None,
    diagnostic_code_map: dict[str, Any] | None,
    investigation_plan: dict[str, Any] | None,
    scenario_dossiers: dict[str, Any] | None,
    observability_comparison: dict[str, Any] | None,
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
    if diagnostic_coverage_ledger:
        for name, path in diagnostic_coverage_ledger.get("artifacts", {}).items():
            artifacts[name] = str(path)
    if diagnostic_telemetry_catalog:
        for name, path in diagnostic_telemetry_catalog.get("artifacts", {}).items():
            artifacts[name] = str(path)
    if diagnostic_code_map:
        for name, path in diagnostic_code_map.get("artifacts", {}).items():
            artifacts[name] = str(path)
    if investigation_plan:
        for name, path in investigation_plan.get("artifacts", {}).items():
            artifacts[name] = str(path)
    if scenario_dossiers:
        for name, path in scenario_dossiers.get("artifacts", {}).items():
            artifacts[name] = str(path)
    if observability_comparison:
        for name, path in observability_comparison.get("artifacts", {}).items():
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


def diagnostic_coverage_ledger_paths(suite_dir: Path) -> dict[str, str]:
    return {
        "diagnostic_coverage_ledger_json": str(suite_dir / "diagnostic-coverage-ledger.json"),
        "diagnostic_coverage_ledger_report": str(suite_dir / "diagnostic-coverage-ledger.md"),
    }


def diagnostic_telemetry_catalog_paths(suite_dir: Path) -> dict[str, str]:
    return {
        "diagnostic_telemetry_catalog_json": str(suite_dir / "diagnostic-telemetry-catalog.json"),
        "diagnostic_telemetry_catalog_report": str(suite_dir / "diagnostic-telemetry-catalog.md"),
    }


def diagnostic_code_map_paths(suite_dir: Path) -> dict[str, str]:
    return {
        "diagnostic_code_map_json": str(suite_dir / "diagnostic-code-map.json"),
        "diagnostic_code_map_report": str(suite_dir / "diagnostic-code-map.md"),
    }


def investigation_plan_paths(suite_dir: Path) -> dict[str, str]:
    return {
        "investigation_plan_json": str(suite_dir / "diagnostic-investigation-plan.json"),
        "investigation_plan_report": str(suite_dir / "diagnostic-investigation-plan.md"),
    }


def scenario_dossiers_paths(suite_dir: Path) -> dict[str, str]:
    return {
        "scenario_dossiers_json": str(suite_dir / "diagnostic-scenario-dossiers.json"),
        "scenario_dossiers_report": str(suite_dir / "diagnostic-scenario-dossiers.md"),
    }


def observability_comparison_paths(suite_dir: Path) -> dict[str, str]:
    return {
        "observability_comparison_json": str(
            suite_dir / "diagnostic-observability-comparison.json"
        ),
        "observability_comparison_report": str(
            suite_dir / "diagnostic-observability-comparison.md"
        ),
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


def focus_domain_subsystem(focus_domain: Any) -> str:
    if not isinstance(focus_domain, str) or not focus_domain:
        return "unknown"
    if "." in focus_domain:
        return focus_domain.split(".", 1)[0]
    return focus_domain


def scenario_telemetry_path(suite_dir: Path, scenario: dict[str, Any]) -> Path | None:
    telemetry_json = as_dict(scenario.get("artifacts")).get("telemetry_json")
    if not isinstance(telemetry_json, str) or not telemetry_json:
        return None
    return artifact_path(suite_dir, telemetry_json)


def suite_artifact_text(suite_dir: Path, value: Any) -> str:
    if not isinstance(value, str) or not value:
        return ""
    return str(artifact_path(suite_dir, value))


def build_diagnostic_coverage_ledger(suite_dir: Path) -> dict[str, Any]:
    manifest = load_json(suite_dir / "scenario-suite.json")
    observer = load_json(suite_dir / "scenario-suite-observer.json")
    scenarios = [scenario for scenario in as_list(manifest.get("scenarios")) if isinstance(scenario, dict)]
    observations = [
        observation
        for observation in as_list(observer.get("observations"))
        if isinstance(observation, dict)
    ]
    observations_by_id = {
        observation.get("scenario_id"): observation
        for observation in observations
        if isinstance(observation.get("scenario_id"), str)
    }
    scenarios_by_id = {
        scenario.get("id"): scenario
        for scenario in scenarios
        if isinstance(scenario.get("id"), str)
    }
    baseline_id = manifest.get("baseline_scenario_id")
    baseline_scenario_id = baseline_id if isinstance(baseline_id, str) and baseline_id else "pass"
    baseline_scenario = as_dict(scenarios_by_id.get(baseline_scenario_id))
    baseline_telemetry_path = scenario_telemetry_path(suite_dir, baseline_scenario)
    baseline_telemetry = load_json(baseline_telemetry_path) if baseline_telemetry_path else {}
    baseline_analysis = as_dict(baseline_telemetry.get("analysis"))
    baseline_coverage = as_dict(baseline_analysis.get("coverage"))
    coverage_gaps = [
        gap
        for gap in as_list(baseline_analysis.get("coverage_gaps"))
        if isinstance(gap, dict)
    ]

    errors: list[str] = []
    if not manifest:
        errors.append(f"missing scenario-suite.json in {suite_dir}")
    if not observer:
        errors.append(f"missing scenario-suite-observer.json in {suite_dir}")
    if baseline_telemetry_path is None or not baseline_telemetry_path.is_file():
        errors.append(f"missing baseline telemetry for {baseline_scenario_id}")
    if not baseline_telemetry:
        errors.append(f"invalid baseline telemetry for {baseline_scenario_id}")

    positive_scenarios = [scenario for scenario in scenarios if scenario.get("expected_passed") is True]
    negative_scenarios = [scenario for scenario in scenarios if scenario.get("expected_passed") is False]
    if not positive_scenarios:
        errors.append("coverage ledger requires at least one expected-pass scenario")
    if not negative_scenarios:
        errors.append("coverage ledger requires at least one expected-failure scenario")
    if not coverage_gaps:
        errors.append("coverage ledger requires baseline analysis.coverage_gaps")

    role_counts = Counter(
        observation.get("role")
        for observation in observations
        if isinstance(observation.get("role"), str)
    )
    failure_kind_counts = Counter(
        scenario.get("failure_kind") or scenario.get("expected_health") or "unknown"
        for scenario in negative_scenarios
    )
    test_subsystem_by_id = {
        test.get("id"): test.get("subsystem")
        for test in as_list(baseline_telemetry.get("tests"))
        if isinstance(test, dict)
        and isinstance(test.get("id"), int)
        and isinstance(test.get("subsystem"), str)
    }
    negative_focus_domains = unique_strings(
        [
            scenario.get("expected_focus_domain") or scenario.get("actual_focus_domain")
            for scenario in negative_scenarios
        ]
    )
    negative_by_subsystem = Counter(
        str(
            test_subsystem_by_id.get(
                scenario.get("expected_focus_test_id") or scenario.get("actual_focus_test_id")
            )
            or focus_domain_subsystem(
                scenario.get("expected_focus_domain") or scenario.get("actual_focus_domain")
            )
        )
        for scenario in negative_scenarios
    )

    negative_by_test_id: dict[int, list[str]] = {}
    for scenario in negative_scenarios:
        focus_test_id = scenario.get("expected_focus_test_id") or scenario.get("actual_focus_test_id")
        if isinstance(focus_test_id, int) and isinstance(scenario.get("id"), str):
            negative_by_test_id.setdefault(focus_test_id, []).append(str(scenario["id"]))

    tests: list[dict[str, Any]] = []
    for test in as_list(baseline_telemetry.get("tests")):
        if not isinstance(test, dict):
            continue
        test_id = test.get("id")
        mapped_scenarios = sorted(negative_by_test_id.get(test_id, [])) if isinstance(test_id, int) else []
        tests.append(
            {
                "id": test_id,
                "name": test.get("name"),
                "subsystem": test.get("subsystem"),
                "tier": test.get("tier"),
                "intent": test.get("intent"),
                "expected_observations": as_list(test.get("expected_observations")),
                "result_addr": test.get("result_addr"),
                "baseline_passed": test.get("passed"),
                "negative_scenario_ids": mapped_scenarios,
                "has_negative_fixture": bool(mapped_scenarios),
            }
        )
    if not tests:
        errors.append("coverage ledger requires baseline telemetry tests")

    subsystem_rows: list[dict[str, Any]] = []
    for row in as_list(baseline_coverage.get("subsystem_summary")):
        if not isinstance(row, dict):
            continue
        subsystem = row.get("subsystem")
        negative_count = negative_by_subsystem.get(str(subsystem), 0)
        subsystem_rows.append(
            {
                **row,
                "negative_fixture_count": negative_count,
                "has_negative_fixture": negative_count > 0,
            }
        )
    for subsystem, count in sorted(negative_by_subsystem.items()):
        if not any(row.get("subsystem") == subsystem for row in subsystem_rows):
            subsystem_rows.append(
                {
                    "subsystem": subsystem,
                    "total": 0,
                    "passed": 0,
                    "failed": 0,
                    "negative_fixture_count": count,
                    "has_negative_fixture": True,
                }
            )

    negative_fixtures = []
    for scenario in negative_scenarios:
        scenario_id = scenario.get("id")
        observation = as_dict(observations_by_id.get(scenario_id))
        scenario_artifacts = as_dict(scenario.get("artifacts"))
        negative_fixtures.append(
            {
                "scenario_id": scenario_id,
                "title": scenario.get("title"),
                "purpose": scenario.get("purpose"),
                "expected_health": scenario.get("expected_health"),
                "expected_focus_domain": scenario.get("expected_focus_domain"),
                "expected_focus_test_id": scenario.get("expected_focus_test_id"),
                "failure_kind": scenario.get("failure_kind"),
                "failed_probe_ids": as_list(scenario.get("failed_probe_ids")),
                "replay_args": as_list(scenario.get("replay_args")),
                "primary_artifact": suite_artifact_text(
                    suite_dir,
                    observation.get("next_artifact") or scenario_artifacts.get("triage_json"),
                ),
                "telemetry_json": suite_artifact_text(
                    suite_dir, scenario_artifacts.get("telemetry_json")
                ),
                "comparison_json": suite_artifact_text(
                    suite_dir, scenario_artifacts.get("comparison_json")
                ),
            }
        )

    artifacts = diagnostic_coverage_ledger_paths(suite_dir)
    artifacts.update(
        {
            "baseline_telemetry_json": str(baseline_telemetry_path) if baseline_telemetry_path else "",
            "scenario_suite_json": str(suite_dir / "scenario-suite.json"),
            "scenario_suite_observer_json": str(suite_dir / "scenario-suite-observer.json"),
        }
    )

    return {
        "diagnostic_coverage_ledger_schema_version": DIAGNOSTIC_COVERAGE_LEDGER_SCHEMA_VERSION,
        "status": "passed" if not errors else "failed",
        "recommended_exit_code": 0 if not errors else 1,
        "suite_dir": str(suite_dir),
        "telemetry_schema_version": baseline_telemetry.get("schema_version"),
        "scenario_suite_schema_version": manifest.get("scenario_suite_schema_version"),
        "observer_schema_version": observer.get("observer_schema_version"),
        "baseline_scenario_id": baseline_scenario_id,
        "test_count": len(tests),
        "scenario_count": len(scenarios),
        "happy_path_scenario_count": len(positive_scenarios),
        "negative_fixture_count": len(negative_scenarios),
        "known_gap_count": len(coverage_gaps),
        "coverage_posture": {
            "only_happy_paths": len(negative_scenarios) == 0,
            "happy_path_scenario_ids": [
                str(scenario.get("id"))
                for scenario in positive_scenarios
                if isinstance(scenario.get("id"), str)
            ],
            "negative_fixture_scenario_ids": [
                str(scenario.get("id"))
                for scenario in negative_scenarios
                if isinstance(scenario.get("id"), str)
            ],
            "summary": (
                f"{len(positive_scenarios)} expected-pass scenario(s), "
                f"{len(negative_scenarios)} expected-failure fixture(s), "
                f"{len(tests)} cartridge test(s), and {len(coverage_gaps)} known gap(s)."
            ),
        },
        "role_counts": dict(sorted(role_counts.items())),
        "failure_kind_counts": dict(sorted(failure_kind_counts.items())),
        "negative_focus_domains": sorted(negative_focus_domains),
        "subsystem_coverage": subsystem_rows,
        "tier_coverage": as_list(baseline_coverage.get("tier_summary")),
        "tests": tests,
        "negative_fixtures": negative_fixtures,
        "coverage_gaps": coverage_gaps,
        "errors": errors,
        "artifacts": artifacts,
        "ai_handoff": [
            "Read this ledger when auditing whether the cartridge only covers happy paths.",
            "Use tests to see every baseline cartridge assertion and negative_scenario_ids to find paired failure fixtures.",
            "Use negative_fixtures to replay intentional failures and inspect their expected focus domains.",
            "Use coverage_gaps before claiming broad emulator compatibility beyond this cartridge.",
        ],
    }


def write_diagnostic_coverage_ledger_markdown(path: Path, ledger: dict[str, Any]) -> None:
    posture = as_dict(ledger.get("coverage_posture"))
    lines = [
        "# Diagnostic Coverage Ledger",
        "",
        "## Verdict",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {ledger.get('status')} |",
        f"| Only happy paths | {posture.get('only_happy_paths')} |",
        f"| Cartridge tests | {ledger.get('test_count')} |",
        f"| Expected-pass scenarios | {ledger.get('happy_path_scenario_count')} |",
        f"| Expected-failure fixtures | {ledger.get('negative_fixture_count')} |",
        f"| Known gaps | {ledger.get('known_gap_count')} |",
        f"| Summary | {markdown_cell(str(posture.get('summary', '')))} |",
        "",
        "## Subsystem Coverage",
        "",
        "| Subsystem | Tests | Passed | Negative Fixtures |",
        "| --- | ---: | ---: | ---: |",
    ]
    for row in as_list(ledger.get("subsystem_coverage")):
        if not isinstance(row, dict):
            continue
        lines.append(
            f"| {row.get('subsystem')} | {row.get('total')} | {row.get('passed')} | {row.get('negative_fixture_count')} |"
        )

    lines.extend(
        [
            "",
            "## Tier Coverage",
            "",
            "| Tier | Tests | Passed |",
            "| --- | ---: | ---: |",
        ]
    )
    for row in as_list(ledger.get("tier_coverage")):
        if not isinstance(row, dict):
            continue
        lines.append(f"| {row.get('tier')} | {row.get('total')} | {row.get('passed')} |")

    lines.extend(
        [
            "",
            "## Cartridge Tests",
            "",
            "| ID | Name | Subsystem | Tier | Negative Fixtures |",
            "| ---: | --- | --- | --- | --- |",
        ]
    )
    for test in as_list(ledger.get("tests")):
        if not isinstance(test, dict):
            continue
        negative_ids = ", ".join(str(value) for value in as_list(test.get("negative_scenario_ids"))) or "-"
        lines.append(
            f"| {test.get('id')} | {test.get('name')} | {test.get('subsystem')} | {test.get('tier')} | {negative_ids} |"
        )

    lines.extend(
        [
            "",
            "## Negative Fixtures",
            "",
            "| Scenario | Expected Health | Focus Domain | Focus Test | Failure Kind |",
            "| --- | --- | --- | ---: | --- |",
        ]
    )
    for fixture in as_list(ledger.get("negative_fixtures")):
        if not isinstance(fixture, dict):
            continue
        lines.append(
            "| "
            f"{fixture.get('scenario_id')} | "
            f"{fixture.get('expected_health')} | "
            f"{fixture.get('expected_focus_domain')} | "
            f"{fixture.get('expected_focus_test_id')} | "
            f"{fixture.get('failure_kind')} |"
        )

    lines.extend(
        [
            "",
            "## Known Coverage Gaps",
            "",
            "| ID | Subsystem | Missing Coverage | Suggested Next Test |",
            "| --- | --- | --- | --- |",
        ]
    )
    for gap in as_list(ledger.get("coverage_gaps")):
        if not isinstance(gap, dict):
            continue
        lines.append(
            "| "
            f"{gap.get('id')} | "
            f"{gap.get('subsystem')} | "
            f"{markdown_cell(str(gap.get('missing_coverage', '')))} | "
            f"{markdown_cell(str(gap.get('suggested_next_test', '')))} |"
        )

    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(ledger.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if ledger.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(ledger.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_diagnostic_coverage_ledger(suite_dir: Path) -> dict[str, Any]:
    ledger = build_diagnostic_coverage_ledger(suite_dir)
    artifacts = as_dict(ledger.get("artifacts"))
    json_path = Path(str(artifacts["diagnostic_coverage_ledger_json"]))
    report_path = Path(str(artifacts["diagnostic_coverage_ledger_report"]))
    json_path.write_text(json.dumps(ledger, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_diagnostic_coverage_ledger_markdown(report_path, ledger)
    return ledger


def value_kind(value: Any) -> str:
    if isinstance(value, dict):
        return "object"
    if isinstance(value, list):
        return "array"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int) and not isinstance(value, bool):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    if value is None:
        return "null"
    return type(value).__name__


def counter_dict(values: list[Any]) -> dict[str, int]:
    counts = Counter(str(value) for value in values if value is not None)
    return dict(sorted(counts.items()))


def telemetry_signal_family(
    family_id: str,
    title: str,
    purpose: str,
    telemetry_paths: list[str],
    triage_paths: list[str],
    first_artifact: str,
    ai_usage: str,
    available: bool,
) -> dict[str, Any]:
    return {
        "id": family_id,
        "title": title,
        "purpose": purpose,
        "telemetry_paths": telemetry_paths,
        "triage_paths": triage_paths,
        "first_artifact": first_artifact,
        "ai_usage": ai_usage,
        "available": available,
    }


def build_diagnostic_telemetry_catalog(suite_dir: Path) -> dict[str, Any]:
    manifest = load_json(suite_dir / "scenario-suite.json")
    scenarios = [scenario for scenario in as_list(manifest.get("scenarios")) if isinstance(scenario, dict)]
    scenarios_by_id = {
        scenario.get("id"): scenario
        for scenario in scenarios
        if isinstance(scenario.get("id"), str)
    }
    baseline_id = manifest.get("baseline_scenario_id")
    baseline_scenario_id = baseline_id if isinstance(baseline_id, str) and baseline_id else "pass"
    baseline_scenario = as_dict(scenarios_by_id.get(baseline_scenario_id))
    baseline_artifacts = as_dict(baseline_scenario.get("artifacts"))
    baseline_telemetry_path = scenario_telemetry_path(suite_dir, baseline_scenario)
    baseline_triage_path = (
        artifact_path(suite_dir, baseline_artifacts["triage_json"])
        if isinstance(baseline_artifacts.get("triage_json"), str)
        else None
    )
    baseline_telemetry = load_json(baseline_telemetry_path) if baseline_telemetry_path else {}
    baseline_triage = load_json(baseline_triage_path) if baseline_triage_path else {}

    errors: list[str] = []
    if not manifest:
        errors.append(f"missing scenario-suite.json in {suite_dir}")
    if baseline_telemetry_path is None or not baseline_telemetry_path.is_file():
        errors.append(f"missing baseline telemetry for {baseline_scenario_id}")
    if baseline_triage_path is None or not baseline_triage_path.is_file():
        errors.append(f"missing baseline triage for {baseline_scenario_id}")
    if not baseline_telemetry:
        errors.append(f"invalid baseline telemetry for {baseline_scenario_id}")
    if not baseline_triage:
        errors.append(f"invalid baseline triage for {baseline_scenario_id}")

    tests = [test for test in as_list(baseline_telemetry.get("tests")) if isinstance(test, dict)]
    probes = [probe for probe in as_list(baseline_telemetry.get("probes")) if isinstance(probe, dict)]
    events = [event for event in as_list(baseline_telemetry.get("events")) if isinstance(event, dict)]
    timeline = [entry for entry in as_list(baseline_telemetry.get("timeline")) if isinstance(entry, dict)]
    trace = as_dict(baseline_telemetry.get("instruction_trace"))
    trace_tail = [entry for entry in as_list(trace.get("tail")) if isinstance(entry, dict)]

    if not tests:
        errors.append("telemetry catalog requires baseline telemetry tests")
    if not probes:
        errors.append("telemetry catalog requires baseline telemetry probes")
    if not events:
        errors.append("telemetry catalog requires baseline telemetry events")
    if not timeline:
        errors.append("telemetry catalog requires baseline telemetry timeline")
    if not trace_tail:
        errors.append("telemetry catalog requires retained instruction trace tail")

    probe_ids = unique_strings([probe.get("id") for probe in probes])
    probes_by_test_id: dict[int, list[str]] = {}
    for probe in probes:
        test_id = probe.get("test_id")
        probe_id = probe.get("id")
        if isinstance(test_id, int) and isinstance(probe_id, str):
            probes_by_test_id.setdefault(test_id, []).append(probe_id)

    timeline_test_ids = {
        entry.get("test_id")
        for entry in timeline
        if isinstance(entry.get("test_id"), int)
    }
    test_signals = []
    for test in tests:
        test_id = test.get("id")
        result_probe_id = f"cartridge.test.{test_id}.result" if isinstance(test_id, int) else ""
        test_signals.append(
            {
                "id": test_id,
                "name": test.get("name"),
                "subsystem": test.get("subsystem"),
                "tier": test.get("tier"),
                "intent": test.get("intent"),
                "result_addr": test.get("result_addr"),
                "result_probe_id": result_probe_id,
                "result_probe_present": result_probe_id in probe_ids,
                "timeline_present": test_id in timeline_test_ids,
                "probe_ids": sorted(probes_by_test_id.get(test_id, []))
                if isinstance(test_id, int)
                else [],
                "expected_observations": as_list(test.get("expected_observations")),
            }
        )

    probe_catalog = [
        {
            "id": probe.get("id"),
            "source": probe.get("source"),
            "subsystem": probe.get("subsystem"),
            "test_id": probe.get("test_id"),
            "test_name": probe.get("test_name"),
            "status": probe.get("status"),
            "description": probe.get("description"),
            "expected": probe.get("expected"),
            "observed": probe.get("observed"),
            "likely_domain": probe.get("likely_domain"),
        }
        for probe in probes
    ]

    event_kind_catalog = []
    event_kinds = sorted({str(event.get("kind")) for event in events if event.get("kind") is not None})
    for kind in event_kinds:
        matching = [event for event in events if event.get("kind") == kind]
        first_event = matching[0] if matching else {}
        last_event = matching[-1] if matching else {}
        event_kind_catalog.append(
            {
                "kind": kind,
                "count": len(matching),
                "first_cycle": first_event.get("cycle"),
                "last_cycle": last_event.get("cycle"),
                "first_frame": first_event.get("frame"),
                "last_frame": last_event.get("frame"),
            }
        )

    trace_fields = sorted({field for entry in trace_tail for field in entry})
    top_level_fields = [
        {
            "path": key,
            "kind": value_kind(value),
        }
        for key, value in sorted(baseline_telemetry.items())
    ]
    baseline_triage_text = str(baseline_triage_path) if baseline_triage_path else ""
    baseline_telemetry_text = str(baseline_telemetry_path) if baseline_telemetry_path else ""
    signal_families = [
        telemetry_signal_family(
            "verdict",
            "Verdict and terminal status",
            "Answers whether the run passed, failed, or timed out and why.",
            ["verdict", "verdict.failure", "analysis.health"],
            ["health", "failure", "recommended_exit_code"],
            baseline_triage_text,
            "Read before deeper telemetry to decide whether the run is healthy or needs triage.",
            bool(baseline_telemetry.get("verdict")) and bool(baseline_triage.get("health")),
        ),
        telemetry_signal_family(
            "debug_focus",
            "Debug focus",
            "Points to the first actionable test, subsystem, domain, failed probes, final event, and terminal instruction.",
            ["analysis.debug_focus"],
            ["debug_focus"],
            baseline_triage_text,
            "Use as the first drilldown target for a failed or timed-out scenario.",
            bool(as_dict(baseline_telemetry.get("analysis")).get("debug_focus"))
            and bool(baseline_triage.get("debug_focus")),
        ),
        telemetry_signal_family(
            "probes",
            "Structured observation probes",
            "Normalizes cartridge result bytes and host-observed signals into passed, failed, or skipped checks.",
            ["probes", "analysis.probe_summary"],
            ["probes"],
            baseline_telemetry_text,
            "Use failed probe ids to rank concrete broken observations before reading full events.",
            bool(probes),
        ),
        telemetry_signal_family(
            "timeline",
            "Per-test timing timeline",
            "Maps every diagnostic test to start/end cycles, frames, outcome, and terminal status.",
            ["timeline", "analysis.timing"],
            ["timing"],
            baseline_telemetry_text,
            "Use when a test is slow, skipped, timed out, or has suspicious cycle/frame drift.",
            bool(timeline),
        ),
        telemetry_signal_family(
            "events",
            "Runtime event stream",
            "Records reset, status changes, test transitions, DMA/DMC events, and frame completions.",
            ["events"],
            ["event_tail"],
            baseline_telemetry_text,
            "Use event kinds and tails to reconstruct the final state transition sequence.",
            bool(events),
        ),
        telemetry_signal_family(
            "instruction_trace",
            "Decoded instruction trace tail",
            "Retains final instruction-boundary CPU state with decoded mnemonic and nearest diagnostic cartridge symbol.",
            ["instruction_trace"],
            ["instruction_trace"],
            baseline_telemetry_text,
            "Use terminal instructions and symbols to anchor CPU-side debugging without disassembling the ROM first.",
            bool(trace_tail),
        ),
        telemetry_signal_family(
            "input_dma_audio",
            "Host-observed input, DMA, and audio side channels",
            "Captures controller masks, OAM/DMC DMA timing, frame output, and APU sample evidence.",
            ["input", "dma", "oam", "frame", "audio"],
            ["input", "dma"],
            baseline_telemetry_text,
            "Use for failures where the cartridge passes but host-observed side effects are wrong.",
            bool(baseline_telemetry.get("input")) and bool(baseline_telemetry.get("dma")),
        ),
        telemetry_signal_family(
            "coverage_limits",
            "Coverage and known limits",
            "Summarizes exercised subsystems, tiers, and explicit known coverage gaps.",
            ["analysis.coverage", "analysis.coverage_gaps"],
            ["coverage", "coverage_gaps"],
            baseline_triage_text,
            "Use before making broad compatibility claims from a passing cartridge run.",
            bool(as_dict(baseline_telemetry.get("analysis")).get("coverage_gaps")),
        ),
    ]

    artifacts = diagnostic_telemetry_catalog_paths(suite_dir)
    artifacts.update(
        {
            "baseline_telemetry_json": baseline_telemetry_text,
            "baseline_triage_json": baseline_triage_text,
            "scenario_suite_json": str(suite_dir / "scenario-suite.json"),
        }
    )

    return {
        "diagnostic_telemetry_catalog_schema_version": DIAGNOSTIC_TELEMETRY_CATALOG_SCHEMA_VERSION,
        "status": "passed" if not errors else "failed",
        "recommended_exit_code": 0 if not errors else 1,
        "suite_dir": str(suite_dir),
        "baseline_scenario_id": baseline_scenario_id,
        "telemetry_schema_version": baseline_telemetry.get("schema_version"),
        "triage_schema_version": baseline_triage.get("triage_schema_version"),
        "test_count": len(tests),
        "probe_count": len(probes),
        "event_count": len(events),
        "event_kind_count": len(event_kind_catalog),
        "timeline_entry_count": len(timeline),
        "trace_retained_instruction_count": trace.get("retained_instruction_count"),
        "trace_captured_instruction_count": trace.get("captured_instruction_count"),
        "signal_family_count": len(signal_families),
        "top_level_fields": top_level_fields,
        "signal_families": signal_families,
        "probe_status_counts": counter_dict([probe.get("status") for probe in probes]),
        "probe_source_counts": counter_dict([probe.get("source") for probe in probes]),
        "probe_catalog": probe_catalog,
        "event_kind_catalog": event_kind_catalog,
        "test_signals": test_signals,
        "trace_catalog": {
            "retention_limit": trace.get("retention_limit"),
            "retained_instruction_count": trace.get("retained_instruction_count"),
            "captured_instruction_count": trace.get("captured_instruction_count"),
            "truncated": trace.get("truncated"),
            "tail_fields": trace_fields,
        },
        "artifacts": artifacts,
        "errors": errors,
        "ai_handoff": [
            "Read this catalog before loading full telemetry when you need to know what each signal family means.",
            "Use signal_families to choose the right artifact and JSON path for the debugging question.",
            "Use probe_catalog to map failed_probe_ids to expected observations, observed values, and likely domains.",
            "Use event_kind_catalog and trace_catalog when reconstructing the final execution path.",
        ],
    }


def write_diagnostic_telemetry_catalog_markdown(path: Path, catalog: dict[str, Any]) -> None:
    lines = [
        "# Diagnostic Telemetry Catalog",
        "",
        "## Verdict",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {catalog.get('status')} |",
        f"| Telemetry schema | {catalog.get('telemetry_schema_version')} |",
        f"| Triage schema | {catalog.get('triage_schema_version')} |",
        f"| Tests | {catalog.get('test_count')} |",
        f"| Probes | {catalog.get('probe_count')} |",
        f"| Event kinds | {catalog.get('event_kind_count')} |",
        f"| Timeline entries | {catalog.get('timeline_entry_count')} |",
        f"| Retained trace instructions | {catalog.get('trace_retained_instruction_count')} |",
        "",
        "## Signal Families",
        "",
        "| ID | Purpose | First Artifact | AI Usage |",
        "| --- | --- | --- | --- |",
    ]
    for family in as_list(catalog.get("signal_families")):
        if not isinstance(family, dict):
            continue
        lines.append(
            "| "
            f"{family.get('id')} | "
            f"{markdown_cell(str(family.get('purpose', '')))} | "
            f"{family.get('first_artifact')} | "
            f"{markdown_cell(str(family.get('ai_usage', '')))} |"
        )

    lines.extend(
        [
            "",
            "## Probe Summary",
            "",
            "| Group | Counts |",
            "| --- | --- |",
            f"| Status | {json.dumps(catalog.get('probe_status_counts', {}), sort_keys=True)} |",
            f"| Source | {json.dumps(catalog.get('probe_source_counts', {}), sort_keys=True)} |",
            "",
            "## Probe Catalog",
            "",
            "| ID | Source | Subsystem | Test | Likely Domain |",
            "| --- | --- | --- | --- | --- |",
        ]
    )
    for probe in as_list(catalog.get("probe_catalog")):
        if not isinstance(probe, dict):
            continue
        lines.append(
            "| "
            f"{probe.get('id')} | "
            f"{probe.get('source')} | "
            f"{probe.get('subsystem')} | "
            f"{probe.get('test_id') or '-'} | "
            f"{probe.get('likely_domain')} |"
        )

    lines.extend(
        [
            "",
            "## Event Kinds",
            "",
            "| Kind | Count | First Cycle | Last Cycle |",
            "| --- | ---: | ---: | ---: |",
        ]
    )
    for event_kind in as_list(catalog.get("event_kind_catalog")):
        if not isinstance(event_kind, dict):
            continue
        lines.append(
            f"| {event_kind.get('kind')} | {event_kind.get('count')} | {event_kind.get('first_cycle')} | {event_kind.get('last_cycle')} |"
        )

    lines.extend(
        [
            "",
            "## Test Signals",
            "",
            "| ID | Name | Result Probe | Timeline | Probe Count |",
            "| ---: | --- | --- | --- | ---: |",
        ]
    )
    for test in as_list(catalog.get("test_signals")):
        if not isinstance(test, dict):
            continue
        lines.append(
            "| "
            f"{test.get('id')} | "
            f"{test.get('name')} | "
            f"{test.get('result_probe_id')} | "
            f"{test.get('timeline_present')} | "
            f"{len(as_list(test.get('probe_ids')))} |"
        )

    trace = as_dict(catalog.get("trace_catalog"))
    lines.extend(
        [
            "",
            "## Trace Catalog",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Captured instructions | {trace.get('captured_instruction_count')} |",
            f"| Retained instructions | {trace.get('retained_instruction_count')} |",
            f"| Retention limit | {trace.get('retention_limit')} |",
            f"| Truncated | {trace.get('truncated')} |",
            f"| Tail fields | {', '.join(str(field) for field in as_list(trace.get('tail_fields')))} |",
            "",
            "## Artifacts",
            "",
            "| Name | Path |",
            "| --- | --- |",
        ]
    )
    for name, artifact_path in as_dict(catalog.get("artifacts")).items():
        lines.append(f"| {name} | {artifact_path} |")
    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(catalog.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if catalog.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(catalog.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_diagnostic_telemetry_catalog(suite_dir: Path) -> dict[str, Any]:
    catalog = build_diagnostic_telemetry_catalog(suite_dir)
    artifacts = as_dict(catalog.get("artifacts"))
    json_path = Path(str(artifacts["diagnostic_telemetry_catalog_json"]))
    report_path = Path(str(artifacts["diagnostic_telemetry_catalog_report"]))
    json_path.write_text(json.dumps(catalog, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_diagnostic_telemetry_catalog_markdown(report_path, catalog)
    return catalog


def path_records(repo_root: Path, paths: list[str]) -> list[dict[str, Any]]:
    records = []
    for path in unique_strings(paths):
        records.append(
            {
                "path": path,
                "exists": (repo_root / path).is_file(),
            }
        )
    return records


def command_record(purpose: str, argv: list[Any]) -> dict[str, Any]:
    return {
        "purpose": purpose,
        "argv": [str(value) for value in argv if value is not None],
    }


def command_text(command: dict[str, Any]) -> str:
    return " ".join(str(value) for value in as_list(command.get("argv")))


def subsystem_test_command(test_path: str) -> dict[str, Any] | None:
    path = Path(test_path)
    if path.suffix != ".rs" or path.parent.name != "tests":
        return None
    return command_record(
        f"Run {path.stem} regression tests",
        ["cargo", "test", "--test", path.stem],
    )


def scenario_test_command(scenario_id: str) -> dict[str, Any] | None:
    test_filter = SCENARIO_TEST_FILTERS.get(scenario_id)
    if not test_filter:
        return None
    return command_record(
        f"Run diagnostic localization test for {scenario_id}",
        ["cargo", "test", "--test", "diagnostic_cartridge_tests", test_filter],
    )


def code_map_debug_anchor(entry: dict[str, Any]) -> dict[str, Any]:
    return entry_debug_anchor(entry)


def build_code_map_entry(
    focus_domain: str, entries: list[dict[str, Any]], repo_root: Path
) -> dict[str, Any]:
    mapping = as_dict(FOCUS_DOMAIN_CODE_MAP.get(focus_domain))
    scenario_ids = sorted(
        str(entry.get("scenario_id"))
        for entry in entries
        if isinstance(entry.get("scenario_id"), str)
    )
    first_entry = sorted(entries, key=lambda entry: str(entry.get("scenario_id")))[0]
    source_files = as_list(mapping.get("source_files"))
    test_files = as_list(mapping.get("test_files"))
    diagnostic_files = DIAGNOSTIC_SUPPORT_FILES
    commands: list[dict[str, Any]] = []

    replay_args = as_list(first_entry.get("replay_args"))
    if replay_args:
        commands.append(command_record("Replay the first mapped scenario", replay_args))
    for scenario_id in scenario_ids:
        test_command = scenario_test_command(scenario_id)
        if test_command:
            commands.append(test_command)
    for test_file in test_files:
        if isinstance(test_file, str):
            test_command = subsystem_test_command(test_file)
            if test_command:
                commands.append(test_command)

    deduped_commands: list[dict[str, Any]] = []
    seen_commands: set[str] = set()
    for command in commands:
        key = command_text(command)
        if key and key not in seen_commands:
            seen_commands.add(key)
            deduped_commands.append(command)

    return {
        "focus_domain": focus_domain,
        "focus_subsystem": mapping.get("subsystem") or entry_focus_subsystem(first_entry),
        "description": mapping.get("description"),
        "scenario_ids": scenario_ids,
        "healths": unique_strings([entry.get("health") for entry in entries]),
        "failure_kinds": unique_strings(
            [as_dict(entry.get("debug_focus")).get("failure_kind") for entry in entries]
        ),
        "failed_probe_ids": unique_strings(
            [
                probe_id
                for entry in entries
                for probe_id in entry_failed_probe_ids(entry)
            ]
        ),
        "source_files": path_records(repo_root, [str(path) for path in source_files]),
        "test_files": path_records(repo_root, [str(path) for path in test_files]),
        "diagnostic_files": path_records(repo_root, diagnostic_files),
        "search_terms": unique_strings([str(term) for term in as_list(mapping.get("search_terms"))]),
        "primary_artifact": entry_primary_artifact(first_entry),
        "replay_args": replay_args,
        "suggested_commands": deduped_commands,
        "debug_anchor": code_map_debug_anchor(first_entry),
    }


def build_diagnostic_code_map(
    suite_dir: Path, debug_index_summary: dict[str, Any] | None, repo_root: Path
) -> dict[str, Any]:
    paths = diagnostic_code_map_paths(suite_dir)
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
    actionable_entries = [
        entry
        for entry in entries
        if entry.get("role") == "expected_failure_fixture" and entry_focus_domain(entry)
    ]
    if not actionable_entries:
        errors.append("debug index has no actionable focus-domain entries")
    entries_by_domain: dict[str, list[dict[str, Any]]] = {}
    for entry in actionable_entries:
        domain = entry_focus_domain(entry)
        if domain:
            entries_by_domain.setdefault(domain, []).append(entry)

    unknown_focus_domains = sorted(
        domain for domain in entries_by_domain if domain not in FOCUS_DOMAIN_CODE_MAP
    )
    for domain in unknown_focus_domains:
        errors.append(f"missing code map entry for focus domain: {domain}")

    focus_entries = [
        build_code_map_entry(domain, domain_entries, repo_root)
        for domain, domain_entries in sorted(entries_by_domain.items())
        if domain in FOCUS_DOMAIN_CODE_MAP
    ]
    for entry in focus_entries:
        for group in ("source_files", "test_files", "diagnostic_files"):
            for path_record in as_list(entry.get(group)):
                if isinstance(path_record, dict) and not path_record.get("exists"):
                    errors.append(
                        f"{entry.get('focus_domain')}: missing {group} path "
                        f"{path_record.get('path')}"
                    )
        if not as_list(entry.get("suggested_commands")):
            errors.append(f"{entry.get('focus_domain')}: missing suggested commands")

    return {
        "diagnostic_code_map_schema_version": DIAGNOSTIC_CODE_MAP_SCHEMA_VERSION,
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
        "scenario_count": len(actionable_entries),
        "focus_domain_count": len(focus_entries),
        "unknown_focus_domains": unknown_focus_domains,
        "focus_domains": focus_entries,
        "errors": errors,
        "ai_handoff": [
            "Use this code map after diagnostic-observability-analysis.json identifies the focus domain.",
            "Open source_files first for emulator behavior, diagnostic_files for cartridge probes, and test_files for nearby regression coverage.",
            "Run suggested_commands[0] to replay the mapped scenario before editing emulator code.",
        ],
    }


def write_diagnostic_code_map_markdown(path: Path, code_map: dict[str, Any]) -> None:
    lines = [
        "# Diagnostic Code Map",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {code_map.get('status')} |",
        f"| Generated at UTC | {code_map.get('generated_at_utc')} |",
        f"| Git commit | {code_map.get('git', {}).get('short_commit', '')} |",
        f"| Focus domains | {code_map.get('focus_domain_count')} |",
        f"| Scenarios | {code_map.get('scenario_count')} |",
        "",
        "## Focus Domains",
        "",
        "| Focus domain | Scenarios | Source files | Test files | Open first | First command |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for entry in as_list(code_map.get("focus_domains")):
        if not isinstance(entry, dict):
            continue
        commands = as_list(entry.get("suggested_commands"))
        first_command = as_dict(commands[0]) if commands else {}
        lines.append(
            "| {} | {} | {} | {} | {} | {} |".format(
                markdown_cell(str(entry.get("focus_domain") or "-")),
                markdown_cell(",".join(str(value) for value in as_list(entry.get("scenario_ids")))),
                markdown_cell(
                    ",".join(
                        str(path.get("path"))
                        for path in as_list(entry.get("source_files"))
                        if isinstance(path, dict)
                    )
                ),
                markdown_cell(
                    ",".join(
                        str(path.get("path"))
                        for path in as_list(entry.get("test_files"))
                        if isinstance(path, dict)
                    )
                ),
                markdown_cell(str(entry.get("primary_artifact") or "-")),
                markdown_cell(command_text(first_command) or "-"),
            )
        )
    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(code_map.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if code_map.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(code_map.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_diagnostic_code_map(
    suite_dir: Path, debug_index_summary: dict[str, Any] | None, repo_root: Path
) -> dict[str, Any]:
    code_map = build_diagnostic_code_map(suite_dir, debug_index_summary, repo_root)
    artifacts = as_dict(code_map.get("artifacts"))
    json_path = Path(str(artifacts["diagnostic_code_map_json"]))
    report_path = Path(str(artifacts["diagnostic_code_map_report"]))
    json_path.write_text(json.dumps(code_map, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_diagnostic_code_map_markdown(report_path, code_map)
    return code_map


def code_map_by_focus_domain(code_map: dict[str, Any] | None) -> dict[str, dict[str, Any]]:
    return {
        str(entry.get("focus_domain")): entry
        for entry in as_list(as_dict(code_map).get("focus_domains"))
        if isinstance(entry, dict) and isinstance(entry.get("focus_domain"), str)
    }


def scenario_changes_by_id(comparison: dict[str, Any] | None) -> dict[str, dict[str, Any]]:
    return {
        str(row.get("scenario_id")): row
        for row in as_list(as_dict(comparison).get("scenario_changes"))
        if isinstance(row, dict) and isinstance(row.get("scenario_id"), str)
    }


def relative_artifact_path(suite_dir: Path, relative_path: Any) -> str | None:
    if not isinstance(relative_path, str) or not relative_path:
        return None
    return str(suite_dir / relative_path)


def investigation_start_artifacts(
    suite_dir: Path, entry: dict[str, Any], primary_relative_path: str | None
) -> dict[str, str]:
    artifacts = as_dict(entry.get("artifacts"))
    start_artifacts: dict[str, str] = {}
    if primary_relative_path:
        start_artifacts["primary_artifact"] = str(suite_dir / primary_relative_path)
    for key in (
        "triage_json",
        "telemetry_json",
        "report_md",
        "comparison_json",
        "bundle_manifest",
    ):
        path = relative_artifact_path(suite_dir, artifacts.get(key))
        if path:
            start_artifacts[key] = path
    return start_artifacts


def investigation_route_steps(route: dict[str, Any]) -> list[dict[str, Any]]:
    start_artifacts = as_dict(route.get("start_artifacts"))
    suggested_commands = as_list(route.get("suggested_commands"))
    source_files = [
        path.get("path")
        for path in as_list(route.get("source_files"))
        if isinstance(path, dict) and path.get("path")
    ]
    test_files = [
        path.get("path")
        for path in as_list(route.get("test_files"))
        if isinstance(path, dict) and path.get("path")
    ]
    return [
        {
            "order": 1,
            "action": "open_primary_artifact",
            "artifact": start_artifacts.get("primary_artifact"),
            "purpose": "Confirm the exact failed probe, focus test, and top comparison difference.",
        },
        {
            "order": 2,
            "action": "replay_scenario",
            "command": suggested_commands[0] if suggested_commands else None,
            "purpose": "Regenerate the focused bundle before editing emulator code.",
        },
        {
            "order": 3,
            "action": "inspect_source",
            "paths": source_files,
            "search_terms": as_list(route.get("search_terms")),
            "purpose": "Read the mapped emulator implementation before touching diagnostic scaffolding.",
        },
        {
            "order": 4,
            "action": "run_regression_tests",
            "commands": suggested_commands[1:],
            "paths": test_files,
            "purpose": "Run the narrow diagnostic and subsystem tests for this focus domain.",
        },
    ]


def build_investigation_route(
    rank: int,
    hypothesis: dict[str, Any],
    suite_dir: Path,
    debug_entries: dict[str, dict[str, Any]],
    code_entries: dict[str, dict[str, Any]],
    comparison_changes: dict[str, dict[str, Any]],
    replay_summary: dict[str, Any] | None,
) -> tuple[dict[str, Any], list[str]]:
    errors: list[str] = []
    focus_domain = str(hypothesis.get("focus_domain") or "")
    code_entry = code_entries.get(focus_domain)
    if not code_entry:
        errors.append(f"missing code-map route for focus domain: {focus_domain}")
        code_entry = {}
    suggested = as_dict(hypothesis.get("suggested_next_action"))
    scenario_ids = [str(value) for value in as_list(hypothesis.get("scenario_ids")) if value]
    primary_scenario_id = str(suggested.get("scenario_id") or (scenario_ids[0] if scenario_ids else ""))
    debug_entry = debug_entries.get(primary_scenario_id, {})
    evidence_rows = [
        row
        for row in as_list(hypothesis.get("evidence"))
        if isinstance(row, dict) and row.get("scenario_id") == primary_scenario_id
    ]
    evidence = as_dict(evidence_rows[0]) if evidence_rows else {}
    primary_relative_path = (
        entry_primary_artifact(debug_entry)
        or str(code_entry.get("primary_artifact") or "")
        or str(suggested.get("open_artifact") or "")
    )
    start_artifacts = investigation_start_artifacts(
        suite_dir, debug_entry, primary_relative_path or None
    )
    first_artifact = start_artifacts.get("primary_artifact")
    if not first_artifact:
        errors.append(f"{focus_domain}: missing primary artifact")
    elif not Path(first_artifact).is_file():
        errors.append(f"{focus_domain}: missing primary artifact path {first_artifact}")

    suggested_commands = as_list(code_entry.get("suggested_commands"))
    if not suggested_commands:
        errors.append(f"{focus_domain}: missing suggested commands")
    replay_args = as_list(code_entry.get("replay_args")) or as_list(suggested.get("replay_args"))
    route_replay: dict[str, Any] | None = None
    if as_dict(replay_summary).get("scenario_id") == primary_scenario_id:
        route_replay = {
            "status": replay_summary.get("status"),
            "artifacts": replay_summary.get("artifacts"),
            "effective_replay_args": replay_summary.get("effective_replay_args"),
            "exit_code_matches_expected": replay_summary.get("exit_code_matches_expected"),
            "health_matches_expected": replay_summary.get("health_matches_expected"),
            "focus_domain_matches_expected": replay_summary.get("focus_domain_matches_expected"),
        }

    next_action = as_dict(evidence.get("next_action")) or as_dict(debug_entry.get("next_action"))
    why = unique_strings(
        [
            f"rank={rank} score={hypothesis.get('score')} confidence={hypothesis.get('confidence')}",
            f"primary_scenario={primary_scenario_id}",
            f"health={debug_entry.get('health') or ','.join(str(value) for value in as_list(hypothesis.get('healths')))}",
            f"failed_probe_ids={','.join(entry_failed_probe_ids(debug_entry))}",
            *[str(value) for value in as_list(next_action.get("evidence"))],
        ]
    )
    route = {
        "route_id": f"{rank:02d}-{focus_domain}",
        "rank": rank,
        "score": hypothesis.get("score"),
        "confidence": hypothesis.get("confidence"),
        "focus_domain": focus_domain,
        "focus_subsystem": hypothesis.get("focus_subsystem")
        or code_entry.get("focus_subsystem")
        or entry_focus_subsystem(debug_entry),
        "description": code_entry.get("description"),
        "scenario_ids": scenario_ids,
        "primary_scenario_id": primary_scenario_id,
        "healths": as_list(hypothesis.get("healths")),
        "failure_kinds": as_list(hypothesis.get("failure_kinds")),
        "failed_probe_ids": unique_strings(
            as_list(hypothesis.get("failed_probe_ids")) or entry_failed_probe_ids(debug_entry)
        ),
        "primary_artifact": first_artifact,
        "primary_artifact_relative": primary_relative_path,
        "start_artifacts": start_artifacts,
        "debug_anchor": code_entry.get("debug_anchor")
        or suggested.get("debug_anchor")
        or entry_debug_anchor(debug_entry),
        "replay_args": replay_args,
        "suggested_commands": suggested_commands,
        "source_files": as_list(code_entry.get("source_files")),
        "test_files": as_list(code_entry.get("test_files")),
        "diagnostic_files": as_list(code_entry.get("diagnostic_files")),
        "search_terms": as_list(code_entry.get("search_terms")),
        "comparison": comparison_changes.get(primary_scenario_id),
        "focused_replay": route_replay,
        "why_this_route": why,
        "stop_conditions": [
            "The focused replay no longer matches this route's focus domain.",
            "The primary artifact no longer reports the expected failed probes.",
            "The mapped narrow regression tests pass after the emulator edit.",
        ],
    }
    route["handoff_steps"] = investigation_route_steps(route)
    return route, errors


def build_investigation_plan(
    suite_dir: Path,
    debug_index_summary: dict[str, Any] | None,
    observability_analysis: dict[str, Any] | None,
    diagnostic_code_map: dict[str, Any] | None,
    observability_comparison: dict[str, Any] | None,
    replay_summary: dict[str, Any] | None,
    repo_root: Path,
) -> dict[str, Any]:
    paths = investigation_plan_paths(suite_dir)
    errors: list[str] = []
    if as_dict(observability_analysis).get("status") != "passed":
        errors.append("observability analysis is not passed")
    if as_dict(diagnostic_code_map).get("status") != "passed":
        errors.append("diagnostic code map is not passed")
    debug_paths = as_dict(debug_index_summary).get("artifacts", {})
    debug_index_path = Path(
        as_dict(debug_paths).get("debug_index_jsonl")
        or debug_index_paths(suite_dir)["debug_index_jsonl"]
    )
    entries, debug_errors = load_jsonl(debug_index_path)
    errors.extend(debug_errors)
    debug_entries = entries_by_scenario_id(entries)
    code_entries = code_map_by_focus_domain(diagnostic_code_map)
    comparison_changes = scenario_changes_by_id(observability_comparison)

    routes: list[dict[str, Any]] = []
    for rank, hypothesis in enumerate(
        as_list(as_dict(observability_analysis).get("ranked_hypotheses")), start=1
    ):
        if not isinstance(hypothesis, dict):
            errors.append("ranked hypothesis entry is not an object")
            continue
        route, route_errors = build_investigation_route(
            rank,
            hypothesis,
            suite_dir,
            debug_entries,
            code_entries,
            comparison_changes,
            replay_summary,
        )
        routes.append(route)
        errors.extend(route_errors)
    if not routes:
        errors.append("investigation plan has no routes")

    focus_domains = unique_strings([route.get("focus_domain") for route in routes])
    top_route = routes[0] if routes else {}
    source_artifacts = {
        "debug_index_jsonl": str(debug_index_path),
        "observability_analysis_json": as_dict(as_dict(observability_analysis).get("artifacts")).get(
            "observability_analysis_json"
        ),
        "diagnostic_code_map_json": as_dict(as_dict(diagnostic_code_map).get("artifacts")).get(
            "diagnostic_code_map_json"
        ),
    }
    if observability_comparison:
        source_artifacts["observability_comparison_json"] = as_dict(
            observability_comparison.get("artifacts")
        ).get("observability_comparison_json")
    if replay_summary:
        source_artifacts["replay_run_json"] = as_dict(replay_summary.get("artifacts")).get(
            "replay_run_json"
        )

    return {
        "investigation_plan_schema_version": INVESTIGATION_PLAN_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": "passed" if not errors else "failed",
        "recommended_exit_code": 0 if not errors else 1,
        "git": git_metadata(repo_root),
        "suite_dir": str(suite_dir),
        "source_artifacts": source_artifacts,
        "artifacts": paths,
        "route_count": len(routes),
        "focus_domain_count": len(focus_domains),
        "top_route": {
            "route_id": top_route.get("route_id"),
            "focus_domain": top_route.get("focus_domain"),
            "primary_scenario_id": top_route.get("primary_scenario_id"),
            "primary_artifact": top_route.get("primary_artifact"),
            "replay_args": top_route.get("replay_args"),
        },
        "routes": routes,
        "errors": errors,
        "ai_handoff": [
            "Start with top_route, then follow routes[0].handoff_steps in order.",
            "Use primary_artifact and start_artifacts.triage_json before loading full telemetry.",
            "Run replay_args or suggested_commands[0] to regenerate the focused bundle before editing code.",
            "Inspect source_files and search_terms, then run the route's regression-test commands.",
        ],
    }


def write_investigation_plan_markdown(path: Path, plan: dict[str, Any]) -> None:
    top = as_dict(plan.get("top_route"))
    lines = [
        "# Diagnostic Investigation Plan",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {plan.get('status')} |",
        f"| Generated at UTC | {plan.get('generated_at_utc')} |",
        f"| Git commit | {plan.get('git', {}).get('short_commit', '')} |",
        f"| Routes | {plan.get('route_count')} |",
        f"| Focus domains | {plan.get('focus_domain_count')} |",
        f"| Top focus domain | {top.get('focus_domain', '-')} |",
        f"| Top scenario | {top.get('primary_scenario_id', '-')} |",
        "",
        "## Routes",
        "",
        "| Rank | Focus domain | Scenario | Health | Failed probes | Open first | Replay command | Source files | Test files |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for route in as_list(plan.get("routes")):
        if not isinstance(route, dict):
            continue
        first_command = as_dict(as_list(route.get("suggested_commands"))[0]) if route.get("suggested_commands") else {}
        lines.append(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |".format(
                route.get("rank"),
                markdown_cell(str(route.get("focus_domain") or "-")),
                markdown_cell(str(route.get("primary_scenario_id") or "-")),
                markdown_cell(",".join(str(value) for value in as_list(route.get("healths")))),
                markdown_cell(
                    ",".join(str(value) for value in as_list(route.get("failed_probe_ids")))
                ),
                markdown_cell(str(route.get("primary_artifact") or "-")),
                markdown_cell(command_text(first_command) or "-"),
                markdown_cell(
                    ",".join(
                        str(record.get("path"))
                        for record in as_list(route.get("source_files"))
                        if isinstance(record, dict)
                    )
                ),
                markdown_cell(
                    ",".join(
                        str(record.get("path"))
                        for record in as_list(route.get("test_files"))
                        if isinstance(record, dict)
                    )
                ),
            )
        )
    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(plan.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if plan.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(plan.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_investigation_plan(
    suite_dir: Path,
    debug_index_summary: dict[str, Any] | None,
    observability_analysis: dict[str, Any] | None,
    diagnostic_code_map: dict[str, Any] | None,
    observability_comparison: dict[str, Any] | None,
    replay_summary: dict[str, Any] | None,
    repo_root: Path,
) -> dict[str, Any]:
    plan = build_investigation_plan(
        suite_dir,
        debug_index_summary,
        observability_analysis,
        diagnostic_code_map,
        observability_comparison,
        replay_summary,
        repo_root,
    )
    artifacts = as_dict(plan.get("artifacts"))
    json_path = Path(str(artifacts["investigation_plan_json"]))
    report_path = Path(str(artifacts["investigation_plan_report"]))
    json_path.write_text(json.dumps(plan, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_investigation_plan_markdown(report_path, plan)
    return plan


def route_by_primary_scenario_id(plan: dict[str, Any] | None) -> dict[str, dict[str, Any]]:
    return {
        str(route.get("primary_scenario_id")): route
        for route in as_list(as_dict(plan).get("routes"))
        if isinstance(route, dict) and isinstance(route.get("primary_scenario_id"), str)
    }


def telemetry_signal_lookup(catalog: dict[str, Any] | None) -> dict[str, dict[str, Any]]:
    return {
        str(family.get("id")): {
            "id": family.get("id"),
            "telemetry_paths": as_list(family.get("telemetry_paths")),
            "triage_paths": as_list(family.get("triage_paths")),
            "first_artifact": family.get("first_artifact"),
            "ai_usage": family.get("ai_usage"),
        }
        for family in as_list(as_dict(catalog).get("signal_families"))
        if isinstance(family, dict) and isinstance(family.get("id"), str)
    }


def scenario_signal_family_ids(entry: dict[str, Any], route: dict[str, Any]) -> list[str]:
    focus = as_dict(entry.get("debug_focus"))
    failure = as_dict(entry.get("failure"))
    domain = str(route.get("focus_domain") or focus.get("focus_domain") or "")
    ids = ["verdict", "debug_focus", "probes", "timeline", "events", "instruction_trace"]
    if domain.startswith("joypad") or domain.startswith("dma") or domain.startswith("apu"):
        ids.append("input_dma_audio")
    if as_list(entry.get("coverage_gap_ids")):
        ids.append("coverage_limits")
    if failure.get("kind") == "timeout":
        ids.append("coverage_limits")
    return unique_strings(ids)


def compact_route_for_dossier(route: dict[str, Any]) -> dict[str, Any] | None:
    if not route:
        return None
    return {
        "route_id": route.get("route_id"),
        "rank": route.get("rank"),
        "score": route.get("score"),
        "confidence": route.get("confidence"),
        "focus_domain": route.get("focus_domain"),
        "primary_scenario_id": route.get("primary_scenario_id"),
        "primary_artifact": route.get("primary_artifact"),
        "replay_args": as_list(route.get("replay_args")),
        "suggested_commands": as_list(route.get("suggested_commands")),
        "source_files": as_list(route.get("source_files")),
        "test_files": as_list(route.get("test_files")),
        "diagnostic_files": as_list(route.get("diagnostic_files")),
        "search_terms": as_list(route.get("search_terms")),
        "debug_anchor": as_dict(route.get("debug_anchor")),
        "handoff_steps": as_list(route.get("handoff_steps")),
        "stop_conditions": as_list(route.get("stop_conditions")),
    }


def scenario_dossier_next_actions(dossier: dict[str, Any]) -> list[dict[str, Any]]:
    route = as_dict(dossier.get("route"))
    start_artifacts = as_dict(dossier.get("start_artifacts"))
    source_files = [
        path.get("path")
        for path in as_list(route.get("source_files"))
        if isinstance(path, dict) and path.get("path")
    ]
    return [
        {
            "order": 1,
            "action": "open_triage",
            "artifact": start_artifacts.get("triage_json") or dossier.get("primary_artifact"),
            "purpose": "Read the compact health, failure, and debug-focus summary.",
        },
        {
            "order": 2,
            "action": "inspect_telemetry_signals",
            "signal_family_ids": as_list(dossier.get("signal_family_ids")),
            "failed_probe_ids": as_list(dossier.get("failed_probe_ids")),
            "purpose": "Open only the cataloged signal paths that explain this scenario.",
        },
        {
            "order": 3,
            "action": "replay_scenario",
            "command": " ".join(str(part) for part in as_list(dossier.get("replay_args"))),
            "purpose": "Regenerate the focused bundle before editing emulator code.",
        },
        {
            "order": 4,
            "action": "inspect_mapped_code",
            "paths": source_files,
            "search_terms": as_list(route.get("search_terms")),
            "purpose": "Use the route-to-code map before changing diagnostic scaffolding.",
        },
        {
            "order": 5,
            "action": "run_narrow_tests",
            "commands": as_list(route.get("suggested_commands"))[1:],
            "purpose": "Confirm the subsystem route after an emulator fix.",
        },
    ]


def build_scenario_dossiers(
    suite_dir: Path,
    debug_index_summary: dict[str, Any] | None,
    diagnostic_telemetry_catalog: dict[str, Any] | None,
    diagnostic_code_map: dict[str, Any] | None,
    investigation_plan: dict[str, Any] | None,
    repo_root: Path,
) -> dict[str, Any]:
    paths = scenario_dossiers_paths(suite_dir)
    errors: list[str] = []
    if as_dict(debug_index_summary).get("status") != "passed":
        errors.append("debug index is not passed")
    if as_dict(diagnostic_telemetry_catalog).get("status") != "passed":
        errors.append("diagnostic telemetry catalog is not passed")
    if as_dict(diagnostic_code_map).get("status") != "passed":
        errors.append("diagnostic code map is not passed")
    if as_dict(investigation_plan).get("status") != "passed":
        errors.append("investigation plan is not passed")

    debug_paths = as_dict(debug_index_summary).get("artifacts", {})
    debug_index_path = Path(
        as_dict(debug_paths).get("debug_index_jsonl")
        or debug_index_paths(suite_dir)["debug_index_jsonl"]
    )
    entries, debug_errors = load_jsonl(debug_index_path)
    errors.extend(debug_errors)
    route_by_scenario = route_by_primary_scenario_id(investigation_plan)
    signal_lookup = telemetry_signal_lookup(diagnostic_telemetry_catalog)

    dossiers: list[dict[str, Any]] = []
    for entry in entries:
        scenario_id = str(entry.get("scenario_id") or "")
        route = route_by_scenario.get(scenario_id, {})
        focus = as_dict(entry.get("debug_focus"))
        failure = as_dict(entry.get("failure"))
        primary_relative = entry_primary_artifact(entry)
        start_artifacts = investigation_start_artifacts(suite_dir, entry, primary_relative)
        signal_family_ids = scenario_signal_family_ids(entry, route)
        signal_families = [
            signal_lookup[family_id]
            for family_id in signal_family_ids
            if family_id in signal_lookup
        ]
        if entry.get("role") == "expected_failure_fixture" and not route:
            errors.append(f"{scenario_id}: missing investigation route")
        if not start_artifacts.get("triage_json"):
            errors.append(f"{scenario_id}: missing triage start artifact")
        if not start_artifacts.get("telemetry_json"):
            errors.append(f"{scenario_id}: missing telemetry start artifact")

        dossier = {
            "scenario_id": scenario_id,
            "title": entry.get("title"),
            "role": entry.get("role"),
            "outcome": entry.get("outcome"),
            "health": entry.get("health"),
            "expected_passed": entry.get("expected_passed"),
            "actual_passed": entry.get("actual_passed"),
            "expectation_met": entry.get("expectation_met"),
            "contract_all_matched": entry.get("contract_all_matched"),
            "comparison_passed": entry.get("comparison_passed"),
            "summary": entry.get("summary"),
            "focus_domain": route.get("focus_domain") or focus.get("focus_domain"),
            "focus_subsystem": route.get("focus_subsystem") or focus.get("focus_subsystem"),
            "focus_test_id": focus.get("focus_test_id"),
            "focus_test_name": focus.get("focus_test_name"),
            "failure_kind": focus.get("failure_kind") or failure.get("kind"),
            "failure_code_hex": focus.get("failure_code_hex") or failure.get("failure_code_hex"),
            "failure": failure,
            "failed_probe_ids": entry_failed_probe_ids(entry),
            "first_failed_probe": as_dict(entry.get("probes")).get("first_failed_probe"),
            "probe_counts": entry.get("probes"),
            "coverage_gap_ids": as_list(entry.get("coverage_gap_ids")),
            "debug_anchor": route.get("debug_anchor") or entry_debug_anchor(entry),
            "primary_artifact": route.get("primary_artifact") or start_artifacts.get("primary_artifact"),
            "start_artifacts": start_artifacts,
            "replay_args": as_list(route.get("replay_args")) or as_list(entry.get("replay_args")),
            "signal_family_ids": signal_family_ids,
            "signal_families": signal_families,
            "route": compact_route_for_dossier(route) if route else None,
            "artifacts": as_dict(entry.get("artifacts")),
        }
        dossier["next_actions"] = scenario_dossier_next_actions(dossier)
        dossiers.append(dossier)

    actionable = [dossier for dossier in dossiers if dossier.get("role") == "expected_failure_fixture"]
    healthy = [dossier for dossier in dossiers if dossier.get("health") == "healthy"]
    source_artifacts = {
        "debug_index_jsonl": str(debug_index_path),
        "diagnostic_telemetry_catalog_json": as_dict(
            as_dict(diagnostic_telemetry_catalog).get("artifacts")
        ).get("diagnostic_telemetry_catalog_json"),
        "diagnostic_code_map_json": as_dict(as_dict(diagnostic_code_map).get("artifacts")).get(
            "diagnostic_code_map_json"
        ),
        "investigation_plan_json": as_dict(as_dict(investigation_plan).get("artifacts")).get(
            "investigation_plan_json"
        ),
        "scenario_suite_json": str(suite_dir / "scenario-suite.json"),
    }

    return {
        "scenario_dossiers_schema_version": SCENARIO_DOSSIERS_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": "passed" if not errors else "failed",
        "recommended_exit_code": 0 if not errors else 1,
        "git": git_metadata(repo_root),
        "suite_dir": str(suite_dir),
        "source_artifacts": source_artifacts,
        "artifacts": paths,
        "dossier_count": len(dossiers),
        "actionable_dossier_count": len(actionable),
        "healthy_dossier_count": len(healthy),
        "dossiers": dossiers,
        "errors": errors,
        "ai_handoff": [
            "Read diagnostic-scenario-dossiers.json when you already know the scenario id.",
            "Use each dossier's start_artifacts before opening full telemetry.",
            "Use signal_families to load only the telemetry paths relevant to the scenario.",
            "Use route.suggested_commands and next_actions to replay, inspect mapped code, and run narrow tests.",
        ],
    }


def write_scenario_dossiers_markdown(path: Path, summary: dict[str, Any]) -> None:
    lines = [
        "# Diagnostic Scenario Dossiers",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Dossiers | {summary.get('dossier_count')} |",
        f"| Actionable dossiers | {summary.get('actionable_dossier_count')} |",
        f"| Healthy dossiers | {summary.get('healthy_dossier_count')} |",
        "",
        "## Dossiers",
        "",
        "| Scenario | Role | Health | Focus domain | Failed probes | Route | Primary artifact |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    for dossier in as_list(summary.get("dossiers")):
        if not isinstance(dossier, dict):
            continue
        route = as_dict(dossier.get("route"))
        lines.append(
            "| "
            f"{dossier.get('scenario_id')} | "
            f"{dossier.get('role') or '-'} | "
            f"{dossier.get('health') or '-'} | "
            f"{dossier.get('focus_domain') or '-'} | "
            f"{markdown_cell(','.join(str(value) for value in as_list(dossier.get('failed_probe_ids'))) or '-')} | "
            f"{route.get('route_id') or '-'} | "
            f"{markdown_cell(str(dossier.get('primary_artifact') or '-'))} |"
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
    for name, artifact_path in as_dict(summary.get("artifacts")).items():
        lines.append(f"| {name} | {artifact_path} |")
    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(summary.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if summary.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(summary.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_scenario_dossiers(
    suite_dir: Path,
    debug_index_summary: dict[str, Any] | None,
    diagnostic_telemetry_catalog: dict[str, Any] | None,
    diagnostic_code_map: dict[str, Any] | None,
    investigation_plan: dict[str, Any] | None,
    repo_root: Path,
) -> dict[str, Any]:
    summary = build_scenario_dossiers(
        suite_dir,
        debug_index_summary,
        diagnostic_telemetry_catalog,
        diagnostic_code_map,
        investigation_plan,
        repo_root,
    )
    artifacts = as_dict(summary.get("artifacts"))
    json_path = Path(str(artifacts["scenario_dossiers_json"]))
    report_path = Path(str(artifacts["scenario_dossiers_report"]))
    json_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_scenario_dossiers_markdown(report_path, summary)
    return summary


def load_observability_snapshot(suite_dir: Path) -> dict[str, Any]:
    analysis_path = suite_dir / "diagnostic-observability-analysis.json"
    debug_index_path = suite_dir / "diagnostic-debug-index.jsonl"
    errors: list[str] = []
    analysis = load_json(analysis_path)
    if not analysis:
        errors.append(f"missing or invalid observability analysis: {analysis_path}")
    debug_entries, debug_errors = load_jsonl(debug_index_path)
    errors.extend(debug_errors)
    expected_count = analysis.get("scenario_count")
    if isinstance(expected_count, int) and expected_count != len(debug_entries):
        errors.append(
            f"{suite_dir}: analysis scenario_count={expected_count} "
            f"but debug-index entries={len(debug_entries)}"
        )
    return {
        "suite_dir": str(suite_dir),
        "analysis": analysis,
        "debug_entries": debug_entries,
        "errors": errors,
        "artifacts": {
            "observability_analysis_json": str(analysis_path),
            "debug_index_jsonl": str(debug_index_path),
        },
    }


def entries_by_scenario_id(entries: list[dict[str, Any]]) -> dict[str, dict[str, Any]]:
    return {
        str(entry.get("scenario_id")): entry
        for entry in entries
        if isinstance(entry.get("scenario_id"), str)
    }


def hypotheses_by_domain(analysis: dict[str, Any]) -> dict[str, dict[str, Any]]:
    return {
        str(hypothesis.get("focus_domain")): hypothesis
        for hypothesis in as_list(analysis.get("ranked_hypotheses"))
        if isinstance(hypothesis, dict) and isinstance(hypothesis.get("focus_domain"), str)
    }


def health_severity(health: Any) -> int:
    order = {
        None: 0,
        "healthy": 0,
        "cartridge_assertion_failed": 2,
        "host_validation_failed": 2,
        "timed_out": 3,
    }
    return order.get(health, 1)


def sorted_strings(values: list[Any]) -> list[str]:
    return sorted(unique_strings(values))


def compare_failed_probe_ids(
    baseline_entry: dict[str, Any] | None, current_entry: dict[str, Any] | None
) -> tuple[list[str], list[str], list[str]]:
    baseline = set(entry_failed_probe_ids(baseline_entry or {}))
    current = set(entry_failed_probe_ids(current_entry or {}))
    return (
        sorted(current),
        sorted(current - baseline),
        sorted(baseline - current),
    )


def compare_scenario(
    scenario_id: str,
    baseline_entry: dict[str, Any] | None,
    current_entry: dict[str, Any] | None,
) -> dict[str, Any]:
    if baseline_entry is None:
        failed_probe_ids, added_probe_ids, _ = compare_failed_probe_ids(None, current_entry)
        return {
            "scenario_id": scenario_id,
            "classification": "added",
            "change_kinds": ["scenario_added"],
            "baseline_present": False,
            "current_present": True,
            "baseline_health": None,
            "current_health": current_entry.get("health") if current_entry else None,
            "baseline_focus_domain": None,
            "current_focus_domain": entry_focus_domain(current_entry or {}),
            "score_delta": entry_analysis_score(current_entry or {}),
            "current_score": entry_analysis_score(current_entry or {}),
            "failed_probe_ids": failed_probe_ids,
            "failed_probe_ids_added": added_probe_ids,
            "failed_probe_ids_removed": [],
            "current_primary_artifact": entry_primary_artifact(current_entry or {}),
            "current_replay_args": as_list((current_entry or {}).get("replay_args")),
            "current_debug_anchor": entry_debug_anchor(current_entry or {}),
        }
    if current_entry is None:
        failed_probe_ids, _, removed_probe_ids = compare_failed_probe_ids(baseline_entry, None)
        return {
            "scenario_id": scenario_id,
            "classification": "regression",
            "change_kinds": ["scenario_removed"],
            "baseline_present": True,
            "current_present": False,
            "baseline_health": baseline_entry.get("health"),
            "current_health": None,
            "baseline_focus_domain": entry_focus_domain(baseline_entry),
            "current_focus_domain": None,
            "score_delta": -entry_analysis_score(baseline_entry),
            "baseline_score": entry_analysis_score(baseline_entry),
            "failed_probe_ids": [],
            "failed_probe_ids_added": [],
            "failed_probe_ids_removed": removed_probe_ids or failed_probe_ids,
            "current_primary_artifact": None,
            "current_replay_args": [],
            "current_debug_anchor": {"kind": "missing"},
        }

    baseline_score = entry_analysis_score(baseline_entry)
    current_score = entry_analysis_score(current_entry)
    baseline_failure = as_dict(baseline_entry.get("debug_focus")).get("failure_kind")
    current_failure = as_dict(current_entry.get("debug_focus")).get("failure_kind")
    baseline_top_difference = as_dict(baseline_entry.get("top_difference")).get("path")
    current_top_difference = as_dict(current_entry.get("top_difference")).get("path")
    failed_probe_ids, added_probe_ids, removed_probe_ids = compare_failed_probe_ids(
        baseline_entry, current_entry
    )
    change_kinds: list[str] = []
    if baseline_entry.get("health") != current_entry.get("health"):
        change_kinds.append("health_changed")
    if entry_focus_domain(baseline_entry) != entry_focus_domain(current_entry):
        change_kinds.append("focus_domain_changed")
    if baseline_failure != current_failure:
        change_kinds.append("failure_kind_changed")
    if added_probe_ids or removed_probe_ids:
        change_kinds.append("failed_probe_ids_changed")
    if baseline_top_difference != current_top_difference:
        change_kinds.append("top_difference_changed")
    if baseline_score != current_score:
        change_kinds.append("score_changed")

    classification = "unchanged"
    if change_kinds:
        classification = "drift"
    if current_entry.get("expectation_met") is False or current_entry.get("contract_all_matched") is False:
        classification = "regression"
    elif baseline_entry.get("health") == "healthy" and current_entry.get("health") != "healthy":
        classification = "regression"
    elif health_severity(current_entry.get("health")) > health_severity(baseline_entry.get("health")):
        classification = "regression"
    elif baseline_entry.get("health") != "healthy" and current_entry.get("health") == "healthy":
        classification = "improvement"

    return {
        "scenario_id": scenario_id,
        "classification": classification,
        "change_kinds": change_kinds,
        "baseline_present": True,
        "current_present": True,
        "baseline_health": baseline_entry.get("health"),
        "current_health": current_entry.get("health"),
        "baseline_focus_domain": entry_focus_domain(baseline_entry),
        "current_focus_domain": entry_focus_domain(current_entry),
        "baseline_failure_kind": baseline_failure,
        "current_failure_kind": current_failure,
        "baseline_top_difference_path": baseline_top_difference,
        "current_top_difference_path": current_top_difference,
        "baseline_score": baseline_score,
        "current_score": current_score,
        "score_delta": current_score - baseline_score,
        "failed_probe_ids": failed_probe_ids,
        "failed_probe_ids_added": added_probe_ids,
        "failed_probe_ids_removed": removed_probe_ids,
        "current_primary_artifact": entry_primary_artifact(current_entry),
        "current_replay_args": as_list(current_entry.get("replay_args")),
        "current_debug_anchor": entry_debug_anchor(current_entry),
    }


def compare_hypothesis(
    focus_domain: str,
    baseline_hypothesis: dict[str, Any] | None,
    current_hypothesis: dict[str, Any] | None,
) -> dict[str, Any]:
    if baseline_hypothesis is None:
        return {
            "focus_domain": focus_domain,
            "classification": "added",
            "baseline_present": False,
            "current_present": True,
            "baseline_rank": None,
            "current_rank": current_hypothesis.get("rank") if current_hypothesis else None,
            "rank_delta": None,
            "baseline_score": None,
            "current_score": current_hypothesis.get("score") if current_hypothesis else None,
            "score_delta": current_hypothesis.get("score") if current_hypothesis else None,
            "scenario_ids_added": as_list((current_hypothesis or {}).get("scenario_ids")),
            "scenario_ids_removed": [],
        }
    if current_hypothesis is None:
        return {
            "focus_domain": focus_domain,
            "classification": "removed",
            "baseline_present": True,
            "current_present": False,
            "baseline_rank": baseline_hypothesis.get("rank"),
            "current_rank": None,
            "rank_delta": None,
            "baseline_score": baseline_hypothesis.get("score"),
            "current_score": None,
            "score_delta": -as_int(baseline_hypothesis.get("score")),
            "scenario_ids_added": [],
            "scenario_ids_removed": as_list(baseline_hypothesis.get("scenario_ids")),
        }

    baseline_scenarios = set(str(value) for value in as_list(baseline_hypothesis.get("scenario_ids")))
    current_scenarios = set(str(value) for value in as_list(current_hypothesis.get("scenario_ids")))
    score_delta = as_int(current_hypothesis.get("score")) - as_int(baseline_hypothesis.get("score"))
    rank_delta = as_int(current_hypothesis.get("rank")) - as_int(baseline_hypothesis.get("rank"))
    classification = "unchanged"
    if (
        score_delta
        or rank_delta
        or baseline_scenarios != current_scenarios
        or baseline_hypothesis.get("confidence") != current_hypothesis.get("confidence")
    ):
        classification = "changed"
    return {
        "focus_domain": focus_domain,
        "classification": classification,
        "baseline_present": True,
        "current_present": True,
        "baseline_rank": baseline_hypothesis.get("rank"),
        "current_rank": current_hypothesis.get("rank"),
        "rank_delta": rank_delta,
        "baseline_score": baseline_hypothesis.get("score"),
        "current_score": current_hypothesis.get("score"),
        "score_delta": score_delta,
        "baseline_confidence": baseline_hypothesis.get("confidence"),
        "current_confidence": current_hypothesis.get("confidence"),
        "scenario_ids_added": sorted(current_scenarios - baseline_scenarios),
        "scenario_ids_removed": sorted(baseline_scenarios - current_scenarios),
    }


def comparison_counts(rows: list[dict[str, Any]]) -> dict[str, int]:
    counts = Counter(str(row.get("classification")) for row in rows)
    return {
        "added": counts.get("added", 0),
        "removed": counts.get("removed", 0),
        "regression": counts.get("regression", 0),
        "improvement": counts.get("improvement", 0),
        "drift": counts.get("drift", 0),
        "changed": counts.get("changed", 0),
        "unchanged": counts.get("unchanged", 0),
    }


def scenario_comparison_sort_key(row: dict[str, Any]) -> tuple[int, str]:
    priority = {
        "regression": 0,
        "removed": 1,
        "added": 2,
        "improvement": 3,
        "drift": 4,
        "unchanged": 5,
    }
    return (priority.get(str(row.get("classification")), 9), str(row.get("scenario_id")))


def build_observability_comparison(
    baseline_suite_dir: Path,
    current_suite_dir: Path,
    fail_on_regression: bool,
    repo_root: Path,
) -> dict[str, Any]:
    paths = observability_comparison_paths(current_suite_dir)
    baseline = load_observability_snapshot(baseline_suite_dir)
    current = load_observability_snapshot(current_suite_dir)
    errors = [
        f"baseline: {error}" for error in as_list(baseline.get("errors"))
    ] + [f"current: {error}" for error in as_list(current.get("errors"))]

    baseline_entries = entries_by_scenario_id(as_list(baseline.get("debug_entries")))
    current_entries = entries_by_scenario_id(as_list(current.get("debug_entries")))
    scenario_ids = sorted(set(baseline_entries) | set(current_entries))
    scenario_changes = [
        compare_scenario(
            scenario_id,
            baseline_entries.get(scenario_id),
            current_entries.get(scenario_id),
        )
        for scenario_id in scenario_ids
    ]
    scenario_changes.sort(key=scenario_comparison_sort_key)
    scenario_counts = comparison_counts(scenario_changes)

    baseline_hypotheses = hypotheses_by_domain(as_dict(baseline.get("analysis")))
    current_hypotheses = hypotheses_by_domain(as_dict(current.get("analysis")))
    focus_domains = sorted(set(baseline_hypotheses) | set(current_hypotheses))
    hypothesis_changes = [
        compare_hypothesis(
            focus_domain,
            baseline_hypotheses.get(focus_domain),
            current_hypotheses.get(focus_domain),
        )
        for focus_domain in focus_domains
    ]
    hypothesis_changes.sort(
        key=lambda row: (
            0 if row.get("classification") != "unchanged" else 1,
            str(row.get("focus_domain")),
        )
    )
    hypothesis_counts = comparison_counts(hypothesis_changes)

    regression_count = scenario_counts["regression"]
    changed_count = len(
        [row for row in scenario_changes if row.get("classification") != "unchanged"]
    )
    verdict = "matched"
    if regression_count:
        verdict = "regressed"
    elif changed_count or hypothesis_counts["changed"] or hypothesis_counts["added"] or hypothesis_counts["removed"]:
        verdict = "changed"
    if errors:
        verdict = "invalid"
    status = "failed" if errors or (fail_on_regression and regression_count) else "passed"

    return {
        "observability_comparison_schema_version": OBSERVABILITY_COMPARISON_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": status,
        "verdict": verdict,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "fail_on_regression": fail_on_regression,
        "git": git_metadata(repo_root),
        "baseline": {
            "suite_dir": str(baseline_suite_dir),
            "scenario_count": as_dict(baseline.get("analysis")).get("scenario_count"),
            "hypothesis_count": as_dict(baseline.get("analysis")).get("hypothesis_count"),
            "artifacts": baseline.get("artifacts"),
        },
        "current": {
            "suite_dir": str(current_suite_dir),
            "scenario_count": as_dict(current.get("analysis")).get("scenario_count"),
            "hypothesis_count": as_dict(current.get("analysis")).get("hypothesis_count"),
            "artifacts": current.get("artifacts"),
        },
        "artifacts": paths,
        "scenario_change_counts": scenario_counts,
        "hypothesis_change_counts": hypothesis_counts,
        "regression_count": regression_count,
        "changed_scenario_count": changed_count,
        "scenario_changes": scenario_changes,
        "regressions": [
            row for row in scenario_changes if row.get("classification") == "regression"
        ],
        "hypothesis_changes": hypothesis_changes,
        "errors": errors,
        "ai_handoff": [
            "Start with regressions before inspecting drift or hypothesis rank changes.",
            "Use scenario_changes.current_replay_args to regenerate the current focused scenario.",
            "Use current_primary_artifact for the first current-run artifact to open.",
            "If verdict is changed without regressions, inspect hypothesis_changes for rank or coverage drift.",
        ],
    }


def write_observability_comparison_markdown(path: Path, comparison: dict[str, Any]) -> None:
    lines = [
        "# Diagnostic Observability Comparison",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {comparison.get('status')} |",
        f"| Verdict | {comparison.get('verdict')} |",
        f"| Generated at UTC | {comparison.get('generated_at_utc')} |",
        f"| Git commit | {comparison.get('git', {}).get('short_commit', '')} |",
        f"| Baseline suite | {comparison.get('baseline', {}).get('suite_dir')} |",
        f"| Current suite | {comparison.get('current', {}).get('suite_dir')} |",
        f"| Regressions | {comparison.get('regression_count')} |",
        f"| Changed scenarios | {comparison.get('changed_scenario_count')} |",
        "",
        "## Scenario Changes",
        "",
        "| Class | Scenario | Health | Focus domain | Score delta | Failed probes added | Open current |",
        "| --- | --- | --- | --- | --- | --- | --- |",
    ]
    for row in as_list(comparison.get("scenario_changes")):
        if not isinstance(row, dict) or row.get("classification") == "unchanged":
            continue
        health = f"{row.get('baseline_health') or '-'} -> {row.get('current_health') or '-'}"
        domain = (
            f"{row.get('baseline_focus_domain') or '-'} -> "
            f"{row.get('current_focus_domain') or '-'}"
        )
        lines.append(
            "| {} | {} | {} | {} | {} | {} | {} |".format(
                row.get("classification"),
                row.get("scenario_id"),
                markdown_cell(health),
                markdown_cell(domain),
                row.get("score_delta"),
                markdown_cell(",".join(str(value) for value in as_list(row.get("failed_probe_ids_added")))),
                markdown_cell(str(row.get("current_primary_artifact") or "-")),
            )
        )
    if not [row for row in as_list(comparison.get("scenario_changes")) if isinstance(row, dict) and row.get("classification") != "unchanged"]:
        lines.append("| unchanged | - | - | - | 0 | - | - |")

    lines.extend(
        [
            "",
            "## Hypothesis Changes",
            "",
            "| Class | Focus domain | Rank delta | Score delta | Scenarios added | Scenarios removed |",
            "| --- | --- | --- | --- | --- | --- |",
        ]
    )
    for row in as_list(comparison.get("hypothesis_changes")):
        if not isinstance(row, dict) or row.get("classification") == "unchanged":
            continue
        lines.append(
            "| {} | {} | {} | {} | {} | {} |".format(
                row.get("classification"),
                markdown_cell(str(row.get("focus_domain") or "-")),
                row.get("rank_delta") if row.get("rank_delta") is not None else "-",
                row.get("score_delta") if row.get("score_delta") is not None else "-",
                markdown_cell(",".join(str(value) for value in as_list(row.get("scenario_ids_added")))),
                markdown_cell(",".join(str(value) for value in as_list(row.get("scenario_ids_removed")))),
            )
        )
    if not [row for row in as_list(comparison.get("hypothesis_changes")) if isinstance(row, dict) and row.get("classification") != "unchanged"]:
        lines.append("| unchanged | - | - | 0 | - | - |")

    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(comparison.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if comparison.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(comparison.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_observability_comparison(
    baseline_suite_dir: Path,
    current_suite_dir: Path,
    fail_on_regression: bool,
    repo_root: Path,
) -> dict[str, Any]:
    comparison = build_observability_comparison(
        baseline_suite_dir, current_suite_dir, fail_on_regression, repo_root
    )
    artifacts = as_dict(comparison.get("artifacts"))
    json_path = Path(str(artifacts["observability_comparison_json"]))
    report_path = Path(str(artifacts["observability_comparison_report"]))
    json_path.write_text(json.dumps(comparison, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_observability_comparison_markdown(report_path, comparison)
    return comparison


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
    diagnostic_coverage_ledger: dict[str, Any] | None,
    diagnostic_telemetry_catalog: dict[str, Any] | None,
    diagnostic_code_map: dict[str, Any] | None,
    investigation_plan: dict[str, Any] | None,
    scenario_dossiers: dict[str, Any] | None,
    observability_comparison: dict[str, Any] | None,
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
    if diagnostic_coverage_ledger and diagnostic_coverage_ledger.get("status") != "passed":
        status = "failed"
    if diagnostic_telemetry_catalog and diagnostic_telemetry_catalog.get("status") != "passed":
        status = "failed"
    if diagnostic_code_map and diagnostic_code_map.get("status") != "passed":
        status = "failed"
    if investigation_plan and investigation_plan.get("status") != "passed":
        status = "failed"
    if scenario_dossiers and scenario_dossiers.get("status") != "passed":
        status = "failed"
    if observability_comparison and observability_comparison.get("status") != "passed":
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
        "coverage_ledger": diagnostic_coverage_ledger,
        "telemetry_catalog": diagnostic_telemetry_catalog,
        "code_map": diagnostic_code_map,
        "investigation_plan": investigation_plan,
        "scenario_dossiers": scenario_dossiers,
        "comparison": observability_comparison,
        "replay": replay_summary,
        "suite": suite,
        "artifacts": artifact_paths(
            suite_dir,
            summary_json,
            summary_md,
            replay_summary,
            debug_index_summary,
            observability_analysis,
            diagnostic_coverage_ledger,
            diagnostic_telemetry_catalog,
            diagnostic_code_map,
            investigation_plan,
            scenario_dossiers,
            observability_comparison,
        ),
        "ai_handoff": [
            "Start with investigation_plan.top_route and follow routes[0].handoff_steps in order.",
            "Use scenario_dossiers when you already know a scenario id and need joined probe, route, telemetry, and code pointers.",
            "Use suite.first_next_action when inspecting the base scenario-suite observer.",
            "Use coverage_ledger to audit happy-path versus negative-fixture coverage and known gaps.",
            "Use telemetry_catalog to understand signal families, probes, event kinds, and trace fields before loading full telemetry.",
            "Use analysis.ranked_hypotheses for ranked subsystem/domain hypotheses across the suite.",
            "Use code_map.focus_domains to jump from a focus domain to source files, tests, and replay commands.",
            "Use comparison.regressions first when --compare-suite-dir is supplied.",
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
    coverage_ledger = summary.get("coverage_ledger") or {}
    lines.extend(
        [
            "",
            "## Coverage Ledger",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Status | {coverage_ledger.get('status', '-')} |",
            f"| Cartridge tests | {coverage_ledger.get('test_count', '-')} |",
            f"| Expected-pass scenarios | {coverage_ledger.get('happy_path_scenario_count', '-')} |",
            f"| Expected-failure fixtures | {coverage_ledger.get('negative_fixture_count', '-')} |",
            f"| Known gaps | {coverage_ledger.get('known_gap_count', '-')} |",
            f"| JSON | {coverage_ledger.get('artifacts', {}).get('diagnostic_coverage_ledger_json', '-')} |",
            f"| Report | {coverage_ledger.get('artifacts', {}).get('diagnostic_coverage_ledger_report', '-')} |",
        ]
    )
    telemetry_catalog = summary.get("telemetry_catalog") or {}
    lines.extend(
        [
            "",
            "## Telemetry Catalog",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Status | {telemetry_catalog.get('status', '-')} |",
            f"| Signal families | {telemetry_catalog.get('signal_family_count', '-')} |",
            f"| Probes | {telemetry_catalog.get('probe_count', '-')} |",
            f"| Event kinds | {telemetry_catalog.get('event_kind_count', '-')} |",
            f"| Timeline entries | {telemetry_catalog.get('timeline_entry_count', '-')} |",
            f"| Retained trace instructions | {telemetry_catalog.get('trace_retained_instruction_count', '-')} |",
            f"| JSON | {telemetry_catalog.get('artifacts', {}).get('diagnostic_telemetry_catalog_json', '-')} |",
            f"| Report | {telemetry_catalog.get('artifacts', {}).get('diagnostic_telemetry_catalog_report', '-')} |",
        ]
    )
    code_map = summary.get("code_map") or {}
    lines.extend(
        [
            "",
            "## Diagnostic Code Map",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Status | {code_map.get('status', '-')} |",
            f"| Focus domains | {code_map.get('focus_domain_count', '-')} |",
            f"| Scenarios | {code_map.get('scenario_count', '-')} |",
            f"| JSON | {code_map.get('artifacts', {}).get('diagnostic_code_map_json', '-')} |",
            f"| Report | {code_map.get('artifacts', {}).get('diagnostic_code_map_report', '-')} |",
        ]
    )
    investigation_plan = summary.get("investigation_plan") or {}
    top_route = as_dict(investigation_plan.get("top_route"))
    lines.extend(
        [
            "",
            "## Investigation Plan",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Status | {investigation_plan.get('status', '-')} |",
            f"| Routes | {investigation_plan.get('route_count', '-')} |",
            f"| Focus domains | {investigation_plan.get('focus_domain_count', '-')} |",
            f"| Top focus domain | {top_route.get('focus_domain', '-')} |",
            f"| Top scenario | {top_route.get('primary_scenario_id', '-')} |",
            f"| JSON | {investigation_plan.get('artifacts', {}).get('investigation_plan_json', '-')} |",
            f"| Report | {investigation_plan.get('artifacts', {}).get('investigation_plan_report', '-')} |",
        ]
    )
    scenario_dossiers = summary.get("scenario_dossiers") or {}
    lines.extend(
        [
            "",
            "## Scenario Dossiers",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Status | {scenario_dossiers.get('status', '-')} |",
            f"| Dossiers | {scenario_dossiers.get('dossier_count', '-')} |",
            f"| Actionable dossiers | {scenario_dossiers.get('actionable_dossier_count', '-')} |",
            f"| Healthy dossiers | {scenario_dossiers.get('healthy_dossier_count', '-')} |",
            f"| JSON | {scenario_dossiers.get('artifacts', {}).get('scenario_dossiers_json', '-')} |",
            f"| Report | {scenario_dossiers.get('artifacts', {}).get('scenario_dossiers_report', '-')} |",
        ]
    )
    comparison = summary.get("comparison") or {}
    lines.extend(
        [
            "",
            "## Observability Comparison",
            "",
            "| Field | Value |",
            "| --- | --- |",
            f"| Status | {comparison.get('status', '-')} |",
            f"| Verdict | {comparison.get('verdict', '-')} |",
            f"| Regressions | {comparison.get('regression_count', '-')} |",
            f"| Changed scenarios | {comparison.get('changed_scenario_count', '-')} |",
            f"| JSON | {comparison.get('artifacts', {}).get('observability_comparison_json', '-')} |",
            f"| Report | {comparison.get('artifacts', {}).get('observability_comparison_report', '-')} |",
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
        "--compare-suite-dir",
        type=Path,
        help=(
            "Optional prior diagnostic observability suite to compare against the "
            "newly generated suite."
        ),
    )
    parser.add_argument(
        "--fail-on-comparison-regression",
        action="store_true",
        help="Return a failing exit code when --compare-suite-dir finds regressions.",
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
    diagnostic_coverage_ledger: dict[str, Any] | None = None
    diagnostic_telemetry_catalog: dict[str, Any] | None = None
    diagnostic_code_map: dict[str, Any] | None = None
    investigation_plan: dict[str, Any] | None = None
    scenario_dossiers: dict[str, Any] | None = None
    observability_comparison: dict[str, Any] | None = None
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
            diagnostic_coverage_ledger = write_diagnostic_coverage_ledger(suite_dir)
            diagnostic_telemetry_catalog = write_diagnostic_telemetry_catalog(suite_dir)
            diagnostic_code_map = write_diagnostic_code_map(
                suite_dir, debug_index_summary, repo_root
            )
            if args.compare_suite_dir:
                observability_comparison = write_observability_comparison(
                    args.compare_suite_dir,
                    suite_dir,
                    args.fail_on_comparison_regression,
                    repo_root,
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
            investigation_plan = write_investigation_plan(
                suite_dir,
                debug_index_summary,
                observability_analysis,
                diagnostic_code_map,
                observability_comparison,
                replay_summary,
                repo_root,
            )
            scenario_dossiers = write_scenario_dossiers(
                suite_dir,
                debug_index_summary,
                diagnostic_telemetry_catalog,
                diagnostic_code_map,
                investigation_plan,
                repo_root,
            )

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
        diagnostic_coverage_ledger,
        diagnostic_telemetry_catalog,
        diagnostic_code_map,
        investigation_plan,
        scenario_dossiers,
        observability_comparison,
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
        coverage_ledger = summary.get("coverage_ledger") or {}
        coverage_note = ""
        if coverage_ledger:
            coverage_note = (
                " coverage_ledger="
                f"{coverage_ledger.get('test_count')}:"
                f"{coverage_ledger.get('negative_fixture_count')}:"
                f"{coverage_ledger.get('status')}"
            )
        telemetry_catalog = summary.get("telemetry_catalog") or {}
        telemetry_note = ""
        if telemetry_catalog:
            telemetry_note = (
                " telemetry_catalog="
                f"{telemetry_catalog.get('probe_count')}:"
                f"{telemetry_catalog.get('event_kind_count')}:"
                f"{telemetry_catalog.get('status')}"
            )
        code_map = summary.get("code_map") or {}
        code_map_note = ""
        if code_map:
            code_map_note = (
                f" code_map={code_map.get('focus_domain_count')}:{code_map.get('status')}"
            )
        investigation = summary.get("investigation_plan") or {}
        investigation_note = ""
        if investigation:
            investigation_note = (
                f" investigation_plan={investigation.get('route_count')}:{investigation.get('status')}"
            )
        dossiers = summary.get("scenario_dossiers") or {}
        dossiers_note = ""
        if dossiers:
            dossiers_note = (
                " scenario_dossiers="
                f"{dossiers.get('dossier_count')}:"
                f"{dossiers.get('actionable_dossier_count')}:"
                f"{dossiers.get('status')}"
            )
        comparison = summary.get("comparison") or {}
        comparison_note = ""
        if comparison:
            comparison_note = (
                f" comparison={comparison.get('verdict')}:{comparison.get('status')}"
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
            f"{debug_note}{analysis_note}{coverage_note}{telemetry_note}{code_map_note}{investigation_note}{dossiers_note}{comparison_note}{replay_note}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
