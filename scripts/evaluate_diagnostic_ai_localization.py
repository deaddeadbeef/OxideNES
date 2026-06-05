#!/usr/bin/env python3
"""Evaluate diagnostic AI localization quality across an accepted scenario suite."""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


AI_LOCALIZATION_EVAL_SCHEMA_VERSION = 1
EXPECTED_SCENARIO_COUNT = 36
EXPECTED_NEGATIVE_FIXTURE_COUNT = 28
EXPECTED_FOCUS_DOMAIN_COUNT = 28


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def as_str_list(value: Any) -> list[str]:
    return [item for item in as_list(value) if isinstance(item, str)]


def load_json(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    value = json.loads(path.read_text(encoding="utf-8"))
    return value if isinstance(value, dict) else {}


def generated_at_utc() -> str:
    return datetime.now(UTC).isoformat().replace("+00:00", "Z")


def markdown_cell(value: Any) -> str:
    if value is None:
        return "-"
    return str(value).replace("|", r"\|").replace("\r", " ").replace("\n", " ")


def row_by_key(rows: Any, key: str) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    for row in as_list(rows):
        if not isinstance(row, dict):
            continue
        value = row.get(key)
        if isinstance(value, str) and value:
            result[value] = row
    return result


def scenario_role(scenario: dict[str, Any], baseline_scenario_id: Any) -> str:
    if scenario.get("id") == baseline_scenario_id:
        return "baseline"
    if scenario.get("expected_passed") is True:
        return "positive_control"
    return "negative_fixture"


def add_check(
    checks: list[dict[str, Any]],
    name: str,
    passed: bool,
    detail: Any,
) -> None:
    checks.append({"name": name, "passed": bool(passed), "detail": detail})


def checks_passed(checks: list[dict[str, Any]]) -> bool:
    return bool(checks) and all(check.get("passed") is True for check in checks)


def check_score(checks: list[dict[str, Any]]) -> float:
    if not checks:
        return 0.0
    return round(
        sum(1 for check in checks if check.get("passed") is True) / len(checks),
        4,
    )


def identity_matches_scenario(identity: dict[str, Any], scenario: dict[str, Any]) -> bool:
    return (
        identity.get("scenario_id") == scenario.get("id")
        and identity.get("focus_domain") == scenario.get("expected_focus_domain")
    )


def build_scenario_eval(
    scenario: dict[str, Any],
    baseline_scenario_id: Any,
    cards_by_scenario: dict[str, dict[str, Any]],
    route_rows_by_id: dict[str, dict[str, Any]],
    packet_rows_by_id: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    scenario_id = scenario.get("id")
    role = scenario_role(scenario, baseline_scenario_id)
    expected_focus_domain = scenario.get("expected_focus_domain")
    actual_focus_domain = scenario.get("actual_focus_domain")
    failed_probe_ids = as_str_list(scenario.get("failed_probe_ids"))
    contract = as_dict(scenario.get("contract"))
    comparison = as_dict(scenario.get("comparison"))
    card = as_dict(cards_by_scenario.get(scenario_id))
    route_id = card.get("route_id")
    route_row = as_dict(route_rows_by_id.get(route_id))
    packet_row = as_dict(packet_rows_by_id.get(route_id))
    route_identity = as_dict(route_row.get("identity"))
    packet_identity = as_dict(packet_row.get("identity"))
    negative = role == "negative_fixture"

    checks: list[dict[str, Any]] = []
    add_check(checks, "expectation_met", scenario.get("expectation_met") is True, scenario.get("expectation_met"))
    add_check(checks, "contract_all_matched", contract.get("all_matched") is True, contract)
    add_check(checks, "passed_state_matches", contract.get("passed_matches") is True, contract.get("actual_passed"))
    add_check(checks, "health_matches", contract.get("health_matches") is True, contract.get("actual_health"))
    add_check(checks, "focus_test_matches", contract.get("focus_test_matches") is True, contract.get("actual_focus_test_id"))
    add_check(
        checks,
        "focus_domain_matches",
        contract.get("focus_domain_matches") is True,
        {"expected": expected_focus_domain, "actual": actual_focus_domain},
    )
    add_check(
        checks,
        "probe_signal_expected",
        bool(failed_probe_ids) if negative else not failed_probe_ids,
        failed_probe_ids,
    )
    if negative:
        add_check(
            checks,
            "comparison_detects_regression",
            comparison.get("passed") is False and int(comparison.get("difference_count") or 0) > 0,
            comparison,
        )
        add_check(checks, "ai_index_card_present", bool(card), scenario_id)
        add_check(
            checks,
            "ai_index_card_focus_matches",
            card.get("focus_domain") == expected_focus_domain,
            {"card": card.get("focus_domain"), "expected": expected_focus_domain},
        )
        add_check(checks, "route_id_present", isinstance(route_id, str) and bool(route_id), route_id)
        add_check(checks, "route_matrix_row_passed", route_row.get("status") == "passed", route_row.get("status"))
        add_check(
            checks,
            "route_identity_matches_scenario",
            identity_matches_scenario(route_identity, scenario),
            route_identity,
        )
        add_check(checks, "route_replay_passed", route_row.get("replay_status") == "passed", route_row.get("replay_status"))
        add_check(checks, "route_tests_passed", route_row.get("tests_status") == "passed", route_row.get("tests_status"))
        add_check(
            checks,
            "source_anchors_present",
            int(route_row.get("source_match_count") or 0) > 0,
            route_row.get("source_match_count"),
        )
        add_check(
            checks,
            "test_anchors_present",
            int(route_row.get("test_match_count") or 0) > 0,
            route_row.get("test_match_count"),
        )
        add_check(
            checks,
            "narrow_commands_present",
            int(route_row.get("narrow_test_command_count") or 0) > 0,
            route_row.get("narrow_test_command_count"),
        )
        add_check(checks, "packet_row_passed", packet_row.get("status") == "passed", packet_row.get("status"))
        add_check(
            checks,
            "packet_identity_matches_scenario",
            identity_matches_scenario(packet_identity, scenario),
            packet_identity,
        )
        add_check(
            checks,
            "packet_self_verification_passed",
            packet_row.get("packet_verification_status") == "passed"
            and int(packet_row.get("packet_verifier_digest_mismatch_count") or 0) == 0
            and int(packet_row.get("packet_verifier_check_count") or 0)
            == int(packet_row.get("packet_verifier_passed_check_count") or 0),
            {
                "status": packet_row.get("packet_verification_status"),
                "checks": packet_row.get("packet_verifier_check_count"),
                "passed_checks": packet_row.get("packet_verifier_passed_check_count"),
                "digest_mismatches": packet_row.get("packet_verifier_digest_mismatch_count"),
            },
        )
        add_check(
            checks,
            "packet_context_present",
            int(packet_row.get("source_window_count") or 0) > 0
            and int(packet_row.get("test_window_count") or 0) > 0,
            {
                "source_windows": packet_row.get("source_window_count"),
                "test_windows": packet_row.get("test_window_count"),
            },
        )
    else:
        add_check(checks, "healthy_route_not_required", route_id in (None, ""), route_id)

    passed = checks_passed(checks)
    return {
        "scenario_id": scenario_id,
        "title": scenario.get("title"),
        "role": role,
        "status": "passed" if passed else "failed",
        "score": check_score(checks),
        "passed_check_count": sum(1 for check in checks if check.get("passed") is True),
        "check_count": len(checks),
        "expected_health": scenario.get("expected_health"),
        "actual_health": scenario.get("actual_health"),
        "expected_focus_domain": expected_focus_domain,
        "actual_focus_domain": actual_focus_domain,
        "expected_focus_test_id": scenario.get("expected_focus_test_id"),
        "actual_focus_test_id": scenario.get("actual_focus_test_id"),
        "failed_probe_ids": failed_probe_ids,
        "failure_kind": scenario.get("failure_kind"),
        "route_id": route_id,
        "route_status": route_row.get("status"),
        "packet_status": packet_row.get("status"),
        "packet_verification_status": packet_row.get("packet_verification_status"),
        "source_match_count": route_row.get("source_match_count"),
        "test_match_count": route_row.get("test_match_count"),
        "packet_source_window_count": packet_row.get("source_window_count"),
        "packet_test_window_count": packet_row.get("test_window_count"),
        "primary_artifact": as_dict(card.get("start_artifacts")).get("primary_artifact")
        if card
        else as_dict(scenario.get("artifacts")).get("triage_json"),
        "replay_args": as_list(scenario.get("replay_args")),
        "checks": checks,
    }


def confusion_rows(scenario_rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    grouped: dict[tuple[Any, Any], list[str]] = defaultdict(list)
    for row in scenario_rows:
        if row.get("role") != "negative_fixture":
            continue
        grouped[(row.get("expected_focus_domain"), row.get("actual_focus_domain"))].append(
            str(row.get("scenario_id"))
        )
    return [
        {
            "expected_focus_domain": expected,
            "actual_focus_domain": actual,
            "matched": expected == actual,
            "count": len(ids),
            "scenario_ids": ids,
        }
        for (expected, actual), ids in sorted(grouped.items(), key=lambda item: str(item[0]))
    ]


def taxonomy_rows(scenario_rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    grouped: dict[str, dict[str, Any]] = {}
    for row in scenario_rows:
        if row.get("role") != "negative_fixture":
            continue
        domain = str(row.get("expected_focus_domain") or "unknown")
        entry = grouped.setdefault(
            domain,
            {
                "focus_domain": domain,
                "scenario_ids": [],
                "failure_kinds": [],
                "failed_probe_ids": [],
            },
        )
        entry["scenario_ids"].append(row.get("scenario_id"))
        if row.get("failure_kind") not in entry["failure_kinds"]:
            entry["failure_kinds"].append(row.get("failure_kind"))
        for probe_id in as_str_list(row.get("failed_probe_ids")):
            if probe_id not in entry["failed_probe_ids"]:
                entry["failed_probe_ids"].append(probe_id)
    return [
        {
            **entry,
            "scenario_count": len(as_list(entry.get("scenario_ids"))),
            "failure_kinds": sorted(str(kind) for kind in as_list(entry.get("failure_kinds")) if kind),
            "failed_probe_ids": sorted(as_str_list(entry.get("failed_probe_ids"))),
        }
        for _, entry in sorted(grouped.items())
    ]


def output_artifacts(summary_json: Path, summary_report: Path) -> dict[str, str]:
    return {
        "diagnostic_ai_localization_eval_json": str(summary_json),
        "diagnostic_ai_localization_eval_report": str(summary_report),
    }


def build_summary(
    suite_dir: Path,
    summary_json: Path,
    summary_report: Path,
) -> dict[str, Any]:
    scenario_suite = load_json(suite_dir / "scenario-suite.json")
    ai_index = load_json(suite_dir / "diagnostic-ai-observability-index.json")
    ai_route_matrix = load_json(suite_dir / "diagnostic-ai-route-matrix.json")
    ai_debug_packet_matrix = load_json(suite_dir / "diagnostic-ai-debug-packet-matrix.json")
    cards_by_scenario = row_by_key(ai_index.get("scenario_cards"), "scenario_id")
    route_rows_by_id = row_by_key(ai_route_matrix.get("routes"), "route_id")
    packet_rows_by_id = row_by_key(ai_debug_packet_matrix.get("routes"), "route_id")
    scenarios = [row for row in as_list(scenario_suite.get("scenarios")) if isinstance(row, dict)]
    scenario_rows = [
        build_scenario_eval(
            scenario,
            scenario_suite.get("baseline_scenario_id"),
            cards_by_scenario,
            route_rows_by_id,
            packet_rows_by_id,
        )
        for scenario in scenarios
    ]
    failed_rows = [row for row in scenario_rows if row.get("status") != "passed"]
    negative_rows = [row for row in scenario_rows if row.get("role") == "negative_fixture"]
    focus_domains = sorted(
        {
            str(row.get("expected_focus_domain"))
            for row in negative_rows
            if row.get("expected_focus_domain")
        }
    )
    scores = [float(row.get("score") or 0.0) for row in scenario_rows]
    errors: list[str] = []
    if scenario_suite.get("passed") is not True:
        errors.append("scenario suite status is not passed")
    if ai_index.get("status") != "passed":
        errors.append("AI index status is not passed")
    if ai_route_matrix.get("status") != "passed":
        errors.append("AI route matrix status is not passed")
    if ai_debug_packet_matrix.get("status") != "passed":
        errors.append("AI debug packet matrix status is not passed")
    if len(scenario_rows) != EXPECTED_SCENARIO_COUNT:
        errors.append(f"expected {EXPECTED_SCENARIO_COUNT} scenarios, found {len(scenario_rows)}")
    if len(negative_rows) != EXPECTED_NEGATIVE_FIXTURE_COUNT:
        errors.append(
            f"expected {EXPECTED_NEGATIVE_FIXTURE_COUNT} negative fixtures, found {len(negative_rows)}"
        )
    if len(focus_domains) != EXPECTED_FOCUS_DOMAIN_COUNT:
        errors.append(
            f"expected {EXPECTED_FOCUS_DOMAIN_COUNT} negative focus domains, found {len(focus_domains)}"
        )
    for row in failed_rows:
        errors.append(f"{row.get('scenario_id')}: localization scorecard failed")

    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_localization_eval_schema_version": AI_LOCALIZATION_EVAL_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(suite_dir),
        "summary": {
            "scenario_count": len(scenario_rows),
            "passed_scenario_count": len(scenario_rows) - len(failed_rows),
            "failed_scenario_count": len(failed_rows),
            "negative_fixture_count": len(negative_rows),
            "healthy_scenario_count": len(scenario_rows) - len(negative_rows),
            "only_happy_paths": len(negative_rows) == 0,
            "expected_focus_domain_count": len(focus_domains),
            "focus_domain_match_count": sum(
                1
                for row in negative_rows
                if row.get("expected_focus_domain") == row.get("actual_focus_domain")
            ),
            "route_ready_count": sum(
                1 for row in negative_rows if row.get("route_status") == "passed"
            ),
            "packet_ready_count": sum(
                1 for row in negative_rows if row.get("packet_status") == "passed"
            ),
            "packet_self_verified_count": sum(
                1
                for row in negative_rows
                if row.get("packet_verification_status") == "passed"
            ),
            "source_anchor_scenario_count": sum(
                1 for row in negative_rows if int(row.get("source_match_count") or 0) > 0
            ),
            "test_anchor_scenario_count": sum(
                1 for row in negative_rows if int(row.get("test_match_count") or 0) > 0
            ),
            "average_score": round(sum(scores) / len(scores), 4) if scores else 0.0,
            "minimum_score": min(scores) if scores else 0.0,
        },
        "artifacts": {
            **output_artifacts(summary_json, summary_report),
            "scenario_suite_json": str(suite_dir / "scenario-suite.json"),
            "diagnostic_ai_index_json": str(suite_dir / "diagnostic-ai-observability-index.json"),
            "diagnostic_ai_route_matrix_json": str(suite_dir / "diagnostic-ai-route-matrix.json"),
            "diagnostic_ai_debug_packet_matrix_json": str(
                suite_dir / "diagnostic-ai-debug-packet-matrix.json"
            ),
        },
        "scenario_scorecards": scenario_rows,
        "focus_domain_confusion": confusion_rows(scenario_rows),
        "failure_taxonomy": taxonomy_rows(scenario_rows),
        "failed_scenarios": [row.get("scenario_id") for row in failed_rows],
        "errors": errors,
        "ai_handoff": [
            "Use this scorecard before trusting automated localization or fix loops.",
            "Each negative fixture must match expected health, focus domain, failed probes, route evidence, source/test anchors, and packet self-verification.",
            "If a scenario row fails, open its primary_artifact, then inspect the failed checks before reading full telemetry.",
        ],
    }


def write_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    lines = [
        "# Diagnostic AI Localization Evaluation",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {markdown_cell(summary.get('suite_dir'))} |",
        f"| Scenarios | {totals.get('passed_scenario_count')}/{totals.get('scenario_count')} |",
        f"| Negative fixtures | {totals.get('negative_fixture_count')} |",
        f"| Only happy paths | {totals.get('only_happy_paths')} |",
        f"| Focus domain matches | {totals.get('focus_domain_match_count')}/{totals.get('negative_fixture_count')} |",
        f"| Route-ready fixtures | {totals.get('route_ready_count')}/{totals.get('negative_fixture_count')} |",
        f"| Packet self-verified fixtures | {totals.get('packet_self_verified_count')}/{totals.get('negative_fixture_count')} |",
        f"| Source anchors | {totals.get('source_anchor_scenario_count')}/{totals.get('negative_fixture_count')} |",
        f"| Test anchors | {totals.get('test_anchor_scenario_count')}/{totals.get('negative_fixture_count')} |",
        f"| Average score | {totals.get('average_score')} |",
        f"| Minimum score | {totals.get('minimum_score')} |",
        "",
        "## Scenario Scorecards",
        "",
        "| Scenario | Role | Health | Focus domain | Score | Route | Packet verify | Status |",
        "| --- | --- | --- | --- | ---: | --- | --- | --- |",
    ]
    for row in as_list(summary.get("scenario_scorecards")):
        if not isinstance(row, dict):
            continue
        lines.append(
            "| {} | {} | {} -> {} | {} -> {} | {} | {} | {} | {} |".format(
                markdown_cell(row.get("scenario_id")),
                markdown_cell(row.get("role")),
                markdown_cell(row.get("expected_health")),
                markdown_cell(row.get("actual_health")),
                markdown_cell(row.get("expected_focus_domain")),
                markdown_cell(row.get("actual_focus_domain")),
                row.get("score"),
                markdown_cell(row.get("route_id")),
                markdown_cell(row.get("packet_verification_status")),
                markdown_cell(row.get("status")),
            )
        )
    lines.extend(
        [
            "",
            "## Focus Domain Confusion",
            "",
            "| Expected | Actual | Matched | Count | Scenarios |",
            "| --- | --- | --- | ---: | --- |",
        ]
    )
    for row in as_list(summary.get("focus_domain_confusion")):
        if isinstance(row, dict):
            lines.append(
                f"| {markdown_cell(row.get('expected_focus_domain'))} | {markdown_cell(row.get('actual_focus_domain'))} | {row.get('matched')} | {row.get('count')} | {markdown_cell(', '.join(as_str_list(row.get('scenario_ids'))))} |"
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
        help="Path to write localization evaluation JSON. Defaults inside --suite-dir.",
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write localization evaluation Markdown. Defaults inside --suite-dir.",
    )
    parser.add_argument("--json", action="store_true", help="Print the localization evaluation JSON.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-localization-eval.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-localization-eval.md"
    summary_json.parent.mkdir(parents=True, exist_ok=True)
    summary_report.parent.mkdir(parents=True, exist_ok=True)
    summary = build_summary(args.suite_dir, summary_json, summary_report)
    summary_json.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(summary_report, summary)
    if args.json:
        print(json.dumps(summary, indent=2, sort_keys=True))
    else:
        totals = as_dict(summary.get("summary"))
        print(
            "Diagnostic AI localization evaluation "
            f"{summary['status']}: suite={args.suite_dir} "
            f"scenarios={totals.get('passed_scenario_count')}/{totals.get('scenario_count')} "
            f"negative_fixtures={totals.get('negative_fixture_count')} "
            f"focus_matches={totals.get('focus_domain_match_count')}/{totals.get('negative_fixture_count')} "
            f"packet_self_verified={totals.get('packet_self_verified_count')}/{totals.get('negative_fixture_count')} "
            f"score={totals.get('average_score')} "
            f"summary_json={summary_json} summary_report={summary_report}"
        )
    return int(summary["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
