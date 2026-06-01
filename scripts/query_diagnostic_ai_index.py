#!/usr/bin/env python3
"""Query an OxideNES diagnostic AI observability index."""

from __future__ import annotations

import argparse
import json
from datetime import UTC, datetime
from pathlib import Path
from typing import Any


QUERY_RESULT_SCHEMA_VERSION = 1
QUERY_SMOKE_SCHEMA_VERSION = 1


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


def index_path(args: argparse.Namespace) -> Path:
    return args.index_json or args.suite_dir / "diagnostic-ai-observability-index.json"


def load_index(args: argparse.Namespace) -> tuple[Path, dict[str, Any], list[str]]:
    path = index_path(args)
    index = load_json(path)
    errors: list[str] = []
    if not index:
        errors.append(f"missing or invalid AI index: {path}")
    elif index.get("status") != "passed":
        errors.append(f"AI index status is {index.get('status')!r}, expected 'passed'")
    return path, index, errors


def scenario_cards(index: dict[str, Any]) -> list[dict[str, Any]]:
    return [card for card in as_list(index.get("scenario_cards")) if isinstance(card, dict)]


def focus_domains(index: dict[str, Any]) -> list[dict[str, Any]]:
    return [domain for domain in as_list(index.get("focus_domains")) if isinstance(domain, dict)]


def probe_rows(index: dict[str, Any]) -> list[dict[str, Any]]:
    return [row for row in as_list(index.get("probe_index")) if isinstance(row, dict)]


def find_by_key(rows: list[dict[str, Any]], key: str, value: str) -> dict[str, Any] | None:
    for row in rows:
        if row.get(key) == value:
            return row
    return None


def list_result(index: dict[str, Any], kind: str) -> dict[str, Any]:
    if kind == "scenarios":
        ids = [card.get("scenario_id") for card in scenario_cards(index)]
    elif kind == "focus-domains":
        ids = [domain.get("focus_domain") for domain in focus_domains(index)]
    else:
        ids = [row.get("probe_id") for row in probe_rows(index)]
    values = [value for value in ids if isinstance(value, str)]
    return {
        "kind": kind,
        "count": len(values),
        "values": values,
    }


def top_route_result(index: dict[str, Any]) -> dict[str, Any]:
    summary = as_dict(index.get("summary"))
    top_route_id = summary.get("top_route_id")
    top_scenario = summary.get("top_route_scenario")
    top_domain = summary.get("top_route_focus_domain")
    domain = (
        find_by_key(focus_domains(index), "focus_domain", top_domain)
        if isinstance(top_domain, str)
        else None
    )
    scenario = (
        find_by_key(scenario_cards(index), "scenario_id", top_scenario)
        if isinstance(top_scenario, str)
        else None
    )
    return {
        "route_id": top_route_id,
        "focus_domain": top_domain,
        "scenario_id": top_scenario,
        "domain": domain,
        "scenario": scenario,
    }


def query_result(
    args: argparse.Namespace,
    query: dict[str, Any],
    result: Any,
    errors: list[str],
) -> dict[str, Any]:
    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_query_result_schema_version": QUERY_RESULT_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(args.suite_dir),
        "index_json": str(index_path(args)),
        "query": query,
        "result": result,
        "errors": errors,
    }


def run_summary(args: argparse.Namespace, index: dict[str, Any], errors: list[str]) -> dict[str, Any]:
    result = {
        "summary": as_dict(index.get("summary")),
        "entrypoints": as_list(index.get("entrypoints")),
        "recommended_workflow": as_list(index.get("recommended_workflow")),
        "ai_handoff": as_list(index.get("ai_handoff")),
    }
    return query_result(args, {"type": "summary"}, result, errors)


def run_top_route(args: argparse.Namespace, index: dict[str, Any], errors: list[str]) -> dict[str, Any]:
    result = top_route_result(index)
    if not result.get("route_id"):
        errors.append("top route is missing")
    if not result.get("domain"):
        errors.append("top route focus domain row is missing")
    if not result.get("scenario"):
        errors.append("top route scenario card is missing")
    return query_result(args, {"type": "top-route"}, result, errors)


def run_scenario(args: argparse.Namespace, index: dict[str, Any], errors: list[str]) -> dict[str, Any]:
    result = find_by_key(scenario_cards(index), "scenario_id", args.scenario_id)
    if result is None:
        errors.append(f"scenario not found: {args.scenario_id}")
        result = {}
    return query_result(
        args,
        {"type": "scenario", "scenario_id": args.scenario_id},
        result,
        errors,
    )


def run_focus_domain(
    args: argparse.Namespace, index: dict[str, Any], errors: list[str]
) -> dict[str, Any]:
    result = find_by_key(focus_domains(index), "focus_domain", args.focus_domain)
    if result is None:
        errors.append(f"focus domain not found: {args.focus_domain}")
        result = {}
    return query_result(
        args,
        {"type": "focus-domain", "focus_domain": args.focus_domain},
        result,
        errors,
    )


def run_probe(args: argparse.Namespace, index: dict[str, Any], errors: list[str]) -> dict[str, Any]:
    result = find_by_key(probe_rows(index), "probe_id", args.probe_id)
    if result is None:
        errors.append(f"probe not found: {args.probe_id}")
        result = {}
    return query_result(args, {"type": "probe", "probe_id": args.probe_id}, result, errors)


def run_coverage_gaps(
    args: argparse.Namespace, index: dict[str, Any], errors: list[str]
) -> dict[str, Any]:
    coverage = as_dict(index.get("coverage_limits"))
    result = {
        "only_happy_paths": coverage.get("only_happy_paths"),
        "summary": coverage.get("summary"),
        "known_gap_count": coverage.get("known_gap_count"),
        "coverage_gaps": as_list(coverage.get("coverage_gaps")),
    }
    if result["only_happy_paths"] is not False:
        errors.append("coverage posture does not prove non-happy-path fixtures")
    if not result["coverage_gaps"]:
        errors.append("coverage gaps are missing")
    return query_result(args, {"type": "coverage-gaps"}, result, errors)


def run_list(args: argparse.Namespace, index: dict[str, Any], errors: list[str]) -> dict[str, Any]:
    result = list_result(index, args.kind)
    if not result["values"]:
        errors.append(f"no values found for list kind {args.kind}")
    return query_result(args, {"type": "list", "kind": args.kind}, result, errors)


def smoke_artifact_paths(args: argparse.Namespace) -> dict[str, str]:
    summary_json = args.summary_json or args.suite_dir / "diagnostic-ai-query-smoke.json"
    summary_report = args.summary_report or args.suite_dir / "diagnostic-ai-query-smoke.md"
    return {
        "diagnostic_ai_query_smoke_json": str(summary_json),
        "diagnostic_ai_query_smoke_report": str(summary_report),
    }


def build_smoke_summary(
    args: argparse.Namespace, index: dict[str, Any], base_errors: list[str]
) -> dict[str, Any]:
    errors = list(base_errors)
    summary = as_dict(index.get("summary"))
    top = top_route_result(index)
    top_scenario_id = top.get("scenario_id")
    top_domain = top.get("focus_domain")
    top_probe_id = None
    top_domain_row = as_dict(top.get("domain"))
    failed_probe_ids = as_str_list(top_domain_row.get("failed_probe_ids"))
    if failed_probe_ids:
        top_probe_id = "ppu.nmi_count" if "ppu.nmi_count" in failed_probe_ids else failed_probe_ids[0]

    checks: list[dict[str, Any]] = []

    def add_check(name: str, passed: bool, detail: Any) -> None:
        checks.append({"name": name, "passed": passed, "detail": detail})
        if not passed:
            errors.append(f"smoke check failed: {name}")

    add_check("index_status", index.get("status") == "passed", index.get("status"))
    add_check("scenario_cards", summary.get("scenario_count") == 18, summary.get("scenario_count"))
    add_check(
        "actionable_scenarios",
        summary.get("actionable_scenario_count") == 16,
        summary.get("actionable_scenario_count"),
    )
    add_check("focus_domains", summary.get("focus_domain_count") == 16, summary.get("focus_domain_count"))
    add_check(
        "top_route",
        bool(top.get("route_id") and top.get("domain") and top.get("scenario")),
        {
            "route_id": top.get("route_id"),
            "scenario_id": top_scenario_id,
            "focus_domain": top_domain,
        },
    )
    add_check(
        "top_route_scenario_query",
        isinstance(top_scenario_id, str)
        and find_by_key(scenario_cards(index), "scenario_id", top_scenario_id) is not None,
        top_scenario_id,
    )
    add_check(
        "top_route_focus_domain_query",
        isinstance(top_domain, str)
        and find_by_key(focus_domains(index), "focus_domain", top_domain) is not None,
        top_domain,
    )
    add_check(
        "top_route_probe_query",
        isinstance(top_probe_id, str)
        and find_by_key(probe_rows(index), "probe_id", top_probe_id) is not None,
        top_probe_id,
    )
    add_check(
        "coverage_not_only_happy_paths",
        as_dict(index.get("coverage_limits")).get("only_happy_paths") is False,
        as_dict(index.get("coverage_limits")).get("only_happy_paths"),
    )
    add_check(
        "coverage_gaps_present",
        bool(as_list(as_dict(index.get("coverage_limits")).get("coverage_gaps"))),
        as_dict(index.get("coverage_limits")).get("known_gap_count"),
    )

    artifacts = smoke_artifact_paths(args)
    status = "passed" if not errors else "failed"
    return {
        "diagnostic_ai_query_smoke_schema_version": QUERY_SMOKE_SCHEMA_VERSION,
        "generated_at_utc": generated_at_utc(),
        "status": status,
        "recommended_exit_code": 0 if status == "passed" else 1,
        "suite_dir": str(args.suite_dir),
        "index_json": str(index_path(args)),
        "artifacts": artifacts,
        "checks": checks,
        "summary": {
            "scenario_count": summary.get("scenario_count"),
            "actionable_scenario_count": summary.get("actionable_scenario_count"),
            "focus_domain_count": summary.get("focus_domain_count"),
            "failed_probe_id_count": summary.get("failed_probe_id_count"),
            "top_route_id": top.get("route_id"),
            "top_route_scenario": top_scenario_id,
            "top_route_focus_domain": top_domain,
            "top_route_probe": top_probe_id,
            "only_happy_paths": as_dict(index.get("coverage_limits")).get("only_happy_paths"),
            "known_gap_count": as_dict(index.get("coverage_limits")).get("known_gap_count"),
        },
        "errors": errors,
        "ai_handoff": [
            "Use query smoke as proof that the AI index supports deterministic route, scenario, probe, and coverage queries.",
            "Use the query CLI in JSON mode when automating diagnostic triage from uploaded artifacts.",
        ],
    }


def write_smoke_markdown(path: Path, summary: dict[str, Any]) -> None:
    totals = as_dict(summary.get("summary"))
    lines = [
        "# Diagnostic AI Query Smoke",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {summary.get('status')} |",
        f"| Generated at UTC | {summary.get('generated_at_utc')} |",
        f"| Suite dir | {summary.get('suite_dir')} |",
        f"| Top route | {totals.get('top_route_id')} |",
        f"| Top scenario | {totals.get('top_route_scenario')} |",
        f"| Top focus domain | {totals.get('top_route_focus_domain')} |",
        f"| Top probe | {totals.get('top_route_probe')} |",
        f"| Only happy paths | {totals.get('only_happy_paths')} |",
        "",
        "## Checks",
        "",
        "| Check | Passed | Detail |",
        "| --- | --- | --- |",
    ]
    for check in as_list(summary.get("checks")):
        if not isinstance(check, dict):
            continue
        detail = str(check.get("detail")).replace("|", r"\|").replace("\n", " ")
        lines.append(f"| {check.get('name')} | {check.get('passed')} | {detail} |")
    lines.extend(["", "## AI Handoff", ""])
    for instruction in as_list(summary.get("ai_handoff")):
        lines.append(f"- {instruction}")
    if summary.get("errors"):
        lines.extend(["", "## Errors", ""])
        for error in as_list(summary.get("errors")):
            lines.append(f"- {error}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def run_smoke(args: argparse.Namespace, index: dict[str, Any], errors: list[str]) -> dict[str, Any]:
    summary = build_smoke_summary(args, index, errors)
    artifacts = smoke_artifact_paths(args)
    json_path = Path(artifacts["diagnostic_ai_query_smoke_json"])
    report_path = Path(artifacts["diagnostic_ai_query_smoke_report"])
    json_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_smoke_markdown(report_path, summary)
    return summary


def print_human(payload: dict[str, Any]) -> None:
    query = as_dict(payload.get("query"))
    if query:
        print(
            "Diagnostic AI index query "
            f"{payload.get('status')}: type={query.get('type')} "
            f"suite={payload.get('suite_dir')}"
        )
        return
    summary = as_dict(payload.get("summary"))
    print(
        "Diagnostic AI query smoke "
        f"{payload.get('status')}: suite={payload.get('suite_dir')} "
        f"top_route={summary.get('top_route_id')} "
        f"scenario={summary.get('top_route_scenario')} "
        f"probe={summary.get('top_route_probe')}"
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--suite-dir",
        required=True,
        type=Path,
        help="Directory containing diagnostic-ai-observability-index.json.",
    )
    parser.add_argument(
        "--index-json",
        type=Path,
        help="Explicit diagnostic AI index JSON path. Defaults inside --suite-dir.",
    )
    parser.add_argument("--json", action="store_true", help="Print JSON result.")

    def add_json_flag(subparser: argparse.ArgumentParser) -> None:
        subparser.add_argument(
            "--json",
            action="store_true",
            default=argparse.SUPPRESS,
            help="Print JSON result.",
        )

    subparsers = parser.add_subparsers(dest="command", required=True)
    summary = subparsers.add_parser("summary", help="Print the compact index summary.")
    add_json_flag(summary)
    top_route = subparsers.add_parser(
        "top-route", help="Print the top route and joined scenario/domain rows."
    )
    add_json_flag(top_route)

    scenario = subparsers.add_parser("scenario", help="Query one scenario card.")
    scenario.add_argument("scenario_id")
    add_json_flag(scenario)

    focus_domain = subparsers.add_parser("focus-domain", help="Query one focus-domain route.")
    focus_domain.add_argument("focus_domain")
    add_json_flag(focus_domain)

    probe = subparsers.add_parser("probe", help="Query one failed probe row.")
    probe.add_argument("probe_id")
    add_json_flag(probe)

    coverage_gaps = subparsers.add_parser(
        "coverage-gaps", help="Print known coverage limits and gaps."
    )
    add_json_flag(coverage_gaps)

    list_parser = subparsers.add_parser("list", help="List available ids.")
    list_parser.add_argument("kind", choices=["scenarios", "focus-domains", "probes"])
    add_json_flag(list_parser)

    smoke = subparsers.add_parser("smoke", help="Run deterministic query smoke checks.")
    smoke.add_argument(
        "--summary-json",
        type=Path,
        help="Path to write the smoke JSON. Defaults inside --suite-dir.",
    )
    smoke.add_argument(
        "--summary-report",
        type=Path,
        help="Path to write the smoke Markdown report. Defaults inside --suite-dir.",
    )
    add_json_flag(smoke)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    _, index, errors = load_index(args)
    command = args.command
    if command == "summary":
        payload = run_summary(args, index, errors)
    elif command == "top-route":
        payload = run_top_route(args, index, errors)
    elif command == "scenario":
        payload = run_scenario(args, index, errors)
    elif command == "focus-domain":
        payload = run_focus_domain(args, index, errors)
    elif command == "probe":
        payload = run_probe(args, index, errors)
    elif command == "coverage-gaps":
        payload = run_coverage_gaps(args, index, errors)
    elif command == "list":
        payload = run_list(args, index, errors)
    else:
        payload = run_smoke(args, index, errors)

    if args.json:
        print(json.dumps(payload, indent=2, sort_keys=True))
    else:
        print_human(payload)
    return int(payload["recommended_exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
