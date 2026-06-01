#!/usr/bin/env python3
"""Build a compact AI-facing index for an accepted OxideNES diagnostic suite."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_INDEX_SCHEMA_VERSION = 1


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def as_str_list(value: Any) -> list[str]:
    return [item for item in as_list(value) if isinstance(item, str)]


def sorted_unique(values: list[Any]) -> list[str]:
    return sorted({value for value in values if isinstance(value, str) and value})


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    return value if isinstance(value, dict) else {}


def command_text(command: dict[str, Any]) -> str:
    argv = as_list(command.get("argv"))
    return " ".join(str(part) for part in argv)


def command_records(commands: Any) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for command in as_list(commands):
        if not isinstance(command, dict):
            continue
        records.append(
            {
                "purpose": command.get("purpose"),
                "argv": as_list(command.get("argv")),
                "text": command_text(command),
            }
        )
    return records


def path_records(records: Any) -> list[str]:
    paths: list[str] = []
    for record in as_list(records):
        if isinstance(record, dict) and isinstance(record.get("path"), str):
            paths.append(record["path"])
    return sorted_unique(paths)


def source_artifact_paths(suite_dir: Path) -> dict[str, str]:
    return {
        "observability_run_json": str(suite_dir / "observability-run.json"),
        "scenario_dossiers_json": str(suite_dir / "diagnostic-scenario-dossiers.json"),
        "investigation_plan_json": str(suite_dir / "diagnostic-investigation-plan.json"),
        "telemetry_catalog_json": str(suite_dir / "diagnostic-telemetry-catalog.json"),
        "coverage_ledger_json": str(suite_dir / "diagnostic-coverage-ledger.json"),
        "code_map_json": str(suite_dir / "diagnostic-code-map.json"),
        "route_evidence_verification_json": str(
            suite_dir / "diagnostic-route-evidence-verification.json"
        ),
    }


def output_artifact_paths(suite_dir: Path, summary_json: Path, summary_report: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_index_json": str(summary_json),
        "diagnostic_ai_index_report": str(summary_report),
        "suite_dir": str(suite_dir),
    }


def artifact_presence(artifacts: dict[str, str]) -> dict[str, bool]:
    result: dict[str, bool] = {}
    for name, value in artifacts.items():
        path = Path(value)
        result[name] = path.is_dir() if name.endswith("_dir") else path.is_file()
    return result


def route_by_scenario_id(plan: dict[str, Any]) -> dict[str, dict[str, Any]]:
    routes: dict[str, dict[str, Any]] = {}
    for route in as_list(plan.get("routes")):
        if not isinstance(route, dict):
            continue
        scenario_id = route.get("primary_scenario_id")
        if isinstance(scenario_id, str):
            routes[scenario_id] = route
    return routes


def probe_catalog_by_id(catalog: dict[str, Any]) -> dict[str, dict[str, Any]]:
    probes: dict[str, dict[str, Any]] = {}
    for probe in as_list(catalog.get("probe_catalog")):
        if not isinstance(probe, dict):
            continue
        probe_id = probe.get("id")
        if isinstance(probe_id, str):
            probes[probe_id] = probe
    return probes


def signal_family_by_id(catalog: dict[str, Any]) -> dict[str, dict[str, Any]]:
    families: dict[str, dict[str, Any]] = {}
    for family in as_list(catalog.get("signal_families")):
        if not isinstance(family, dict):
            continue
        family_id = family.get("id")
        if isinstance(family_id, str):
            families[family_id] = family
    return families


def compact_debug_anchor(value: Any) -> dict[str, Any]:
    anchor = as_dict(value)
    return {
        "kind": anchor.get("kind"),
        "cycle": anchor.get("cycle"),
        "frame": anchor.get("frame"),
        "pc_hex": anchor.get("pc_hex"),
        "instruction": anchor.get("instruction"),
        "symbol": anchor.get("symbol"),
    }


def compact_next_actions(actions: Any) -> list[dict[str, Any]]:
    compacted: list[dict[str, Any]] = []
    for action in as_list(actions):
        if not isinstance(action, dict):
            continue
        compacted.append(
            {
                "order": action.get("order"),
                "action": action.get("action"),
                "purpose": action.get("purpose"),
                "artifact": action.get("artifact"),
                "command": action.get("command"),
                "paths": as_list(action.get("paths")),
                "commands": as_list(action.get("commands")),
            }
        )
    return compacted


def build_scenario_cards(
    dossiers: dict[str, Any],
    plan: dict[str, Any],
    catalog: dict[str, Any],
) -> list[dict[str, Any]]:
    routes = route_by_scenario_id(plan)
    families = signal_family_by_id(catalog)
    cards: list[dict[str, Any]] = []

    for dossier in as_list(dossiers.get("dossiers")):
        if not isinstance(dossier, dict):
            continue
        scenario_id = dossier.get("scenario_id")
        route = routes.get(scenario_id) if isinstance(scenario_id, str) else None
        start_artifacts = as_dict(dossier.get("start_artifacts"))
        signal_family_ids = as_str_list(dossier.get("signal_family_ids"))
        cards.append(
            {
                "scenario_id": scenario_id,
                "role": dossier.get("role"),
                "health": dossier.get("health"),
                "focus_domain": dossier.get("focus_domain"),
                "summary": dossier.get("summary"),
                "failed_probe_ids": as_str_list(dossier.get("failed_probe_ids")),
                "signal_family_ids": signal_family_ids,
                "signal_family_titles": [
                    as_dict(families.get(family_id)).get("title")
                    for family_id in signal_family_ids
                    if families.get(family_id)
                ],
                "start_artifacts": {
                    "primary_artifact": start_artifacts.get("primary_artifact"),
                    "triage_json": start_artifacts.get("triage_json"),
                    "telemetry_json": start_artifacts.get("telemetry_json"),
                    "comparison_json": start_artifacts.get("comparison_json"),
                    "report_md": start_artifacts.get("report_md"),
                },
                "replay_args": as_list(dossier.get("replay_args")),
                "route_id": as_dict(route).get("route_id") if route else None,
                "route_rank": as_dict(route).get("rank") if route else None,
                "debug_anchor": compact_debug_anchor(as_dict(route).get("debug_anchor"))
                if route
                else {},
                "source_files": path_records(as_dict(route).get("source_files")) if route else [],
                "test_files": path_records(as_dict(route).get("test_files")) if route else [],
                "search_terms": as_str_list(as_dict(route).get("search_terms")) if route else [],
                "suggested_commands": command_records(as_dict(route).get("suggested_commands"))
                if route
                else [],
                "next_actions": compact_next_actions(dossier.get("next_actions")),
            }
        )
    return cards


def build_focus_domains(plan: dict[str, Any]) -> list[dict[str, Any]]:
    domains: list[dict[str, Any]] = []
    for route in as_list(plan.get("routes")):
        if not isinstance(route, dict):
            continue
        domains.append(
            {
                "focus_domain": route.get("focus_domain"),
                "focus_subsystem": route.get("focus_subsystem"),
                "route_id": route.get("route_id"),
                "rank": route.get("rank"),
                "primary_scenario_id": route.get("primary_scenario_id"),
                "scenario_ids": as_str_list(route.get("scenario_ids")),
                "failed_probe_ids": as_str_list(route.get("failed_probe_ids")),
                "primary_artifact": route.get("primary_artifact"),
                "debug_anchor": compact_debug_anchor(route.get("debug_anchor")),
                "source_files": path_records(route.get("source_files")),
                "test_files": path_records(route.get("test_files")),
                "diagnostic_files": path_records(route.get("diagnostic_files")),
                "search_terms": as_str_list(route.get("search_terms")),
                "replay_args": as_list(route.get("replay_args")),
                "suggested_commands": command_records(route.get("suggested_commands")),
                "handoff_steps": compact_next_actions(route.get("handoff_steps")),
            }
        )
    return domains


def build_probe_index(
    scenario_cards: list[dict[str, Any]],
    catalog: dict[str, Any],
) -> list[dict[str, Any]]:
    probe_catalog = probe_catalog_by_id(catalog)
    scenarios_by_probe: dict[str, list[str]] = defaultdict(list)
    domains_by_probe: dict[str, list[str]] = defaultdict(list)
    artifacts_by_probe: dict[str, list[str]] = defaultdict(list)

    for card in scenario_cards:
        scenario_id = card.get("scenario_id")
        domain = card.get("focus_domain")
        primary_artifact = as_dict(card.get("start_artifacts")).get("primary_artifact")
        for probe_id in as_str_list(card.get("failed_probe_ids")):
            if isinstance(scenario_id, str):
                scenarios_by_probe[probe_id].append(scenario_id)
            if isinstance(domain, str):
                domains_by_probe[probe_id].append(domain)
            if isinstance(primary_artifact, str):
                artifacts_by_probe[probe_id].append(primary_artifact)

    rows: list[dict[str, Any]] = []
    for probe_id in sorted(scenarios_by_probe):
        probe = as_dict(probe_catalog.get(probe_id))
        rows.append(
            {
                "probe_id": probe_id,
                "scenario_ids": sorted_unique(scenarios_by_probe[probe_id]),
                "focus_domains": sorted_unique(domains_by_probe[probe_id]),
                "first_artifact": sorted_unique(artifacts_by_probe[probe_id])[0],
                "catalog_status": probe.get("status"),
                "source": probe.get("source"),
                "subsystem": probe.get("subsystem"),
                "test_id": probe.get("test_id"),
                "test_name": probe.get("test_name"),
                "likely_domain": probe.get("likely_domain"),
                "expected": probe.get("expected"),
                "observed": probe.get("observed"),
            }
        )
    return rows


def build_coverage_limits(ledger: dict[str, Any]) -> dict[str, Any]:
    posture = as_dict(ledger.get("coverage_posture"))
    return {
        "only_happy_paths": posture.get("only_happy_paths"),
        "summary": posture.get("summary"),
        "happy_path_scenario_ids": as_str_list(posture.get("happy_path_scenario_ids")),
        "negative_fixture_scenario_ids": as_str_list(
            posture.get("negative_fixture_scenario_ids")
        ),
        "known_gap_count": ledger.get("known_gap_count"),
        "coverage_gaps": as_list(ledger.get("coverage_gaps")),
        "subsystem_coverage": as_list(ledger.get("subsystem_coverage")),
        "tier_coverage": as_list(ledger.get("tier_coverage")),
    }


def build_entrypoints(
    source_artifacts: dict[str, str],
    output_artifacts: dict[str, str],
) -> list[dict[str, Any]]:
    return [
        {
            "id": "ai_index",
            "path": output_artifacts["diagnostic_ai_index_json"],
            "purpose": "Compact joined control-plane index for automated debugging.",
        },
        {
            "id": "observability_run",
            "path": source_artifacts["observability_run_json"],
            "purpose": "Root observability run summary and artifact map.",
        },
        {
            "id": "scenario_dossiers",
            "path": source_artifacts["scenario_dossiers_json"],
            "purpose": "Scenario-id-first joined cards with route, telemetry, and replay pointers.",
        },
        {
            "id": "investigation_plan",
            "path": source_artifacts["investigation_plan_json"],
            "purpose": "Ranked executable focus-domain routes.",
        },
        {
            "id": "telemetry_catalog",
            "path": source_artifacts["telemetry_catalog_json"],
            "purpose": "Signal-family, probe, event-kind, and timeline dictionary.",
        },
        {
            "id": "coverage_ledger",
            "path": source_artifacts["coverage_ledger_json"],
            "purpose": "Happy-path, negative-fixture, test, tier, and known-gap posture.",
        },
        {
            "id": "route_evidence_verification",
            "path": source_artifacts["route_evidence_verification_json"],
            "purpose": "Accepted replay-matrix and top-route proof.",
        },
    ]


def recommended_workflow() -> list[dict[str, Any]]:
    return [
        {
            "order": 1,
            "action": "check_acceptance",
            "artifact": "diagnostic-e2e-report.json",
            "purpose": "Confirm the uploaded suite is accepted before debugging emulator code.",
        },
        {
            "order": 2,
            "action": "choose_route_or_scenario",
            "artifact": "diagnostic-ai-observability-index.json",
            "purpose": "Pick a focus domain, failed probe, or scenario card from this compact index.",
        },
        {
            "order": 3,
            "action": "open_primary_artifact",
            "artifact": "scenario_cards[].start_artifacts.primary_artifact",
            "purpose": "Read the smallest evidence file that explains the selected failure.",
        },
        {
            "order": 4,
            "action": "replay_before_editing",
            "artifact": "scenario_cards[].replay_args",
            "purpose": "Regenerate the focused bundle to reproduce the exact telemetry locally.",
        },
        {
            "order": 5,
            "action": "inspect_mapped_code_and_tests",
            "artifact": "focus_domains[].source_files and focus_domains[].suggested_commands",
            "purpose": "Use mapped source, search terms, and narrow tests before changing code.",
        },
    ]


def build_summary(
    suite_dir: Path,
    summary_json: Path,
    summary_report: Path,
) -> dict[str, Any]:
    source_artifacts = source_artifact_paths(suite_dir)
    output_artifacts = output_artifact_paths(suite_dir, summary_json, summary_report)
    observability = load_json(Path(source_artifacts["observability_run_json"]))
    dossiers = load_json(Path(source_artifacts["scenario_dossiers_json"]))
    plan = load_json(Path(source_artifacts["investigation_plan_json"]))
    catalog = load_json(Path(source_artifacts["telemetry_catalog_json"]))
    ledger = load_json(Path(source_artifacts["coverage_ledger_json"]))
    code_map = load_json(Path(source_artifacts["code_map_json"]))
    route_evidence = load_json(Path(source_artifacts["route_evidence_verification_json"]))

    scenario_cards = build_scenario_cards(dossiers, plan, catalog)
    focus_domains = build_focus_domains(plan)
    probe_index = build_probe_index(scenario_cards, catalog)
    coverage_limits = build_coverage_limits(ledger)
    source_presence = artifact_presence(source_artifacts)
    output_presence = artifact_presence(output_artifacts)
    output_presence["diagnostic_ai_index_json"] = True
    output_presence["diagnostic_ai_index_report"] = True

    errors: list[str] = []
    for name, present in source_presence.items():
        if not present:
            errors.append(f"missing source artifact: {name}")
    for label, artifact in (
        ("observability run", observability),
        ("scenario dossiers", dossiers),
        ("investigation plan", plan),
        ("telemetry catalog", catalog),
        ("coverage ledger", ledger),
        ("code map", code_map),
        ("route evidence verification", route_evidence),
    ):
        if artifact.get("status") != "passed":
            errors.append(f"{label} status is not passed")

    actionable_cards = [
        card for card in scenario_cards if card.get("role") == "expected_failure_fixture"
    ]
    if not scenario_cards:
        errors.append("scenario_cards is empty")
    if len(actionable_cards) != dossiers.get("actionable_dossier_count"):
        errors.append("actionable scenario card count does not match dossier summary")
    if len(focus_domains) != plan.get("route_count"):
        errors.append("focus domain count does not match investigation plan route_count")
    if coverage_limits.get("only_happy_paths") is not False:
        errors.append("coverage posture did not prove non-happy-path fixtures")
    for card in actionable_cards:
        label = card.get("scenario_id")
        if not card.get("route_id"):
            errors.append(f"{label} is missing a route_id")
        if not card.get("failed_probe_ids"):
            errors.append(f"{label} is missing failed_probe_ids")
        if not as_dict(card.get("start_artifacts")).get("triage_json"):
            errors.append(f"{label} is missing triage_json")
        if not card.get("suggested_commands"):
            errors.append(f"{label} is missing suggested_commands")
    for domain in focus_domains:
        label = domain.get("focus_domain")
        if not domain.get("source_files"):
            errors.append(f"{label} is missing source_files")
        if not domain.get("test_files"):
            errors.append(f"{label} is missing test_files")
        if not domain.get("suggested_commands"):
            errors.append(f"{label} is missing suggested_commands")
    if not probe_index:
        errors.append("probe_index is empty")

    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_index_schema_version": AI_INDEX_SCHEMA_VERSION,
        "generated_at_utc": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(suite_dir),
        "summary": {
            "observability_status": observability.get("status"),
            "route_evidence_status": route_evidence.get("status"),
            "scenario_count": dossiers.get("dossier_count"),
            "healthy_scenario_count": dossiers.get("healthy_dossier_count"),
            "actionable_scenario_count": dossiers.get("actionable_dossier_count"),
            "focus_domain_count": len(focus_domains),
            "route_count": plan.get("route_count"),
            "probe_count": catalog.get("probe_count"),
            "failed_probe_id_count": len(probe_index),
            "signal_family_count": catalog.get("signal_family_count"),
            "event_kind_count": catalog.get("event_kind_count"),
            "coverage_test_count": ledger.get("test_count"),
            "known_gap_count": ledger.get("known_gap_count"),
            "only_happy_paths": coverage_limits.get("only_happy_paths"),
            "top_route_id": as_dict(plan.get("top_route")).get("route_id"),
            "top_route_scenario": as_dict(plan.get("top_route")).get("primary_scenario_id"),
            "top_route_focus_domain": as_dict(plan.get("top_route")).get("focus_domain"),
        },
        "entrypoints": build_entrypoints(source_artifacts, output_artifacts),
        "recommended_workflow": recommended_workflow(),
        "scenario_cards": scenario_cards,
        "focus_domains": focus_domains,
        "probe_index": probe_index,
        "coverage_limits": coverage_limits,
        "source_artifacts": source_artifacts,
        "artifacts": output_artifacts,
        "artifact_presence": {
            "source": source_presence,
            "outputs": output_presence,
        },
        "code_map_status": code_map.get("status"),
        "errors": errors,
        "ai_handoff": [
            "Use this index as the first compact join across dossiers, routes, telemetry catalog, coverage ledger, and route evidence.",
            "Select a scenario_card when you know the scenario id; select a focus_domain when you know the subsystem or likely domain.",
            "Open start_artifacts.primary_artifact before loading telemetry.json.",
            "Run replay_args before editing emulator code so the focused telemetry can be regenerated locally.",
            "Use coverage_limits before making broad compatibility claims from a passing cartridge.",
        ],
    }


def markdown_cell(value: Any) -> str:
    return str(value if value is not None else "-").replace("|", r"\|").replace("\n", " ")


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    lines = [
        "# Diagnostic AI Observability Index",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {summary.get('suite_dir')} |",
        f"| Scenarios | {totals.get('scenario_count')} |",
        f"| Actionable scenarios | {totals.get('actionable_scenario_count')} |",
        f"| Focus domains | {totals.get('focus_domain_count')} |",
        f"| Failed probe ids | {totals.get('failed_probe_id_count')} |",
        f"| Known gaps | {totals.get('known_gap_count')} |",
        f"| Only happy paths | {totals.get('only_happy_paths')} |",
        "",
        "## Recommended Workflow",
        "",
        "| Order | Action | Artifact | Purpose |",
        "| ---: | --- | --- | --- |",
    ]
    for step in as_list(summary.get("recommended_workflow")):
        if not isinstance(step, dict):
            continue
        lines.append(
            f"| {step.get('order')} | {markdown_cell(step.get('action'))} | "
            f"{markdown_cell(step.get('artifact'))} | {markdown_cell(step.get('purpose'))} |"
        )

    lines.extend(
        [
            "",
            "## Focus Domains",
            "",
            "| Rank | Domain | Scenario | Primary artifact | Source files | Tests |",
            "| ---: | --- | --- | --- | --- | --- |",
        ]
    )
    for domain in as_list(summary.get("focus_domains")):
        if not isinstance(domain, dict):
            continue
        lines.append(
            f"| {domain.get('rank')} | {markdown_cell(domain.get('focus_domain'))} | "
            f"{markdown_cell(domain.get('primary_scenario_id'))} | "
            f"{markdown_cell(domain.get('primary_artifact'))} | "
            f"{markdown_cell(', '.join(as_str_list(domain.get('source_files'))))} | "
            f"{markdown_cell(', '.join(as_str_list(domain.get('test_files'))))} |"
        )

    lines.extend(
        [
            "",
            "## Scenario Cards",
            "",
            "| Scenario | Role | Health | Focus domain | Failed probes | Primary artifact |",
            "| --- | --- | --- | --- | --- | --- |",
        ]
    )
    for card in as_list(summary.get("scenario_cards")):
        if not isinstance(card, dict):
            continue
        artifacts = as_dict(card.get("start_artifacts"))
        lines.append(
            f"| {markdown_cell(card.get('scenario_id'))} | {markdown_cell(card.get('role'))} | "
            f"{markdown_cell(card.get('health'))} | {markdown_cell(card.get('focus_domain'))} | "
            f"{markdown_cell(', '.join(as_str_list(card.get('failed_probe_ids'))))} | "
            f"{markdown_cell(artifacts.get('primary_artifact'))} |"
        )

    lines.extend(
        [
            "",
            "## Coverage Limits",
            "",
            "| Field | Value |",
            "| --- | --- |",
        ]
    )
    coverage = as_dict(summary.get("coverage_limits"))
    lines.extend(
        [
            f"| Summary | {markdown_cell(coverage.get('summary'))} |",
            f"| Happy-path scenarios | {len(as_list(coverage.get('happy_path_scenario_ids')))} |",
            f"| Negative fixtures | {len(as_list(coverage.get('negative_fixture_scenario_ids')))} |",
            f"| Known gaps | {coverage.get('known_gap_count')} |",
        ]
    )

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
        required=True,
        type=Path,
        help="Directory produced by scripts/run_diagnostic_e2e.py.",
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write the AI index JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the AI index Markdown report. Defaults inside --suite-dir.",
    )
    parser.add_argument("--json", action="store_true", help="Print the AI index JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    suite_dir = args.suite_dir
    summary_json = args.summary_json or suite_dir / "diagnostic-ai-observability-index.json"
    summary_report = args.summary_report or suite_dir / "diagnostic-ai-observability-index.md"
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)

    summary = build_summary(suite_dir, summary_json, summary_report)
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)

    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        totals = as_dict(summary.get("summary"))
        print(
            "Diagnostic AI observability index "
            f"{summary['status']}: suite={suite_dir} "
            f"summary_json={summary_json} summary_report={summary_report} "
            f"scenarios={totals.get('scenario_count')} "
            f"focus_domains={totals.get('focus_domain_count')} "
            f"failed_probe_ids={totals.get('failed_probe_id_count')} "
            f"only_happy_paths={totals.get('only_happy_paths')}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
