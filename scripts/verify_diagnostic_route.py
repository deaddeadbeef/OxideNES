#!/usr/bin/env python3
"""Validate OxideNES diagnostic route-check and route-matrix artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


EXPECTED_ROUTE_CHECK_SCHEMA = 1
EXPECTED_ROUTE_MATRIX_SCHEMA = 1
EXPECTED_INVESTIGATION_PLAN_SCHEMA = 1
ROUTE_EVIDENCE_VERIFICATION_SCHEMA_VERSION = 1
DEFAULT_VERIFICATION_JSON_NAME = "diagnostic-route-evidence-verification.json"
DEFAULT_VERIFICATION_REPORT_NAME = "diagnostic-route-evidence-verification.md"
REQUIRED_REPLAY_BUNDLE_FILES = {
    "manifest": "manifest.json",
    "triage": "triage.json",
    "telemetry": "telemetry.json",
    "report": "report.md",
    "rom": "diagnostic.nes",
}
MAX_ROUTE_DIR_NAME_LENGTH = 20


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def sanitize_path_component(value: str) -> str:
    cleaned = "".join(char if char.isalnum() or char in "_.-" else "-" for char in value.strip())
    cleaned = cleaned.strip(".-") or "route"
    if len(cleaned) <= MAX_ROUTE_DIR_NAME_LENGTH:
        return cleaned
    digest = hashlib.sha1(cleaned.encode("utf-8")).hexdigest()[:8]
    prefix_length = MAX_ROUTE_DIR_NAME_LENGTH - len(digest) - 1
    prefix = cleaned[:prefix_length].strip(".-") or "route"
    return f"{prefix}-{digest}"


def path_text(value: Any) -> str:
    return value if isinstance(value, str) else ""


def normalized_path(value: Any) -> str:
    return path_text(value).replace("\\", "/")


def utc_now_text() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def markdown_value(value: Any) -> str:
    if value is None:
        return ""
    if isinstance(value, bool):
        return "true" if value else "false"
    return str(value).replace("|", "\\|")


class RouteEvidenceVerifier:
    def __init__(
        self,
        suite_dir: Path,
        matrix_dir: Path | None,
        top_route_dir: Path | None,
        require_matrix: bool,
        require_top_route: bool,
        expect_matrix_tests_skipped: bool,
    ) -> None:
        self.suite_dir = suite_dir
        self.matrix_dir = matrix_dir or suite_dir / "route-replay-matrix"
        self.top_route_dir_arg = top_route_dir
        self.require_matrix = require_matrix
        self.require_top_route = require_top_route
        self.expect_matrix_tests_skipped = expect_matrix_tests_skipped
        self.errors: list[str] = []

    def verify(self) -> dict[str, Any]:
        plan = self.read_json_file(
            self.suite_dir / "diagnostic-investigation-plan.json",
            "diagnostic investigation plan",
        )
        scenarios = self.scenarios_by_id()
        self.expect_equal(
            plan.get("investigation_plan_schema_version"),
            EXPECTED_INVESTIGATION_PLAN_SCHEMA,
            "investigation plan schema version",
        )
        self.expect_equal(plan.get("status"), "passed", "investigation plan status")
        plan_routes = self.investigation_routes(plan)
        if not plan_routes:
            self.errors.append("investigation plan must contain routes")

        verified_matrix = False
        matrix_summary: dict[str, Any] = {}
        matrix_json = self.matrix_dir / "diagnostic-route-matrix.json"
        if self.require_matrix or matrix_json.exists():
            matrix_summary = self.verify_route_matrix(matrix_json, plan_routes, scenarios)
            verified_matrix = True

        verified_top_route = False
        top_route_id = self.top_route_id(plan, plan_routes)
        top_route = next(
            (route for route in plan_routes if route.get("route_id") == top_route_id),
            plan_routes[0] if plan_routes else {},
        )
        top_route_json = self.top_route_dir(top_route_id) / "diagnostic-route-check.json"
        if self.require_top_route or top_route_json.exists():
            scenario = scenarios.get(str(top_route.get("primary_scenario_id") or ""), {})
            self.verify_route_check(
                top_route_json,
                top_route,
                scenario,
                "top route check",
                expect_tests_skipped=False,
            )
            verified_top_route = True

        if not verified_matrix and not verified_top_route:
            self.errors.append(
                "no route evidence found; use --require-matrix or --require-top-route to make the expected artifact explicit"
            )

        return {
            "suite_dir": str(self.suite_dir),
            "route_count": len(plan_routes),
            "matrix_verified": verified_matrix,
            "matrix_route_count": matrix_summary.get("route_count"),
            "matrix_passed_route_count": matrix_summary.get("passed_route_count"),
            "matrix_replay_failure_count": matrix_summary.get("replay_failure_count"),
            "top_route_verified": verified_top_route,
            "top_route_id": top_route_id,
        }

    def verify_route_matrix(
        self,
        matrix_json: Path,
        plan_routes: list[dict[str, Any]],
        scenarios: dict[str, dict[str, Any]],
    ) -> dict[str, Any]:
        matrix = self.read_json_file(matrix_json, "diagnostic route matrix")
        matrix_report = matrix_json.with_name("diagnostic-route-matrix.md")
        self.expect_file(matrix_report, "diagnostic route matrix report")
        if matrix_report.is_file():
            report = matrix_report.read_text(encoding="utf-8")
            for section in ("# Diagnostic Route Matrix", "## Routes", "## AI Handoff"):
                self.expect_contains(report, section, "diagnostic route matrix report")

        self.expect_equal(
            matrix.get("diagnostic_route_matrix_schema_version"),
            EXPECTED_ROUTE_MATRIX_SCHEMA,
            "route matrix schema version",
        )
        self.expect_equal(matrix.get("status"), "passed", "route matrix status")
        self.expect_equal(matrix.get("recommended_exit_code"), 0, "route matrix recommended_exit_code")
        self.expect_equal(matrix.get("route_count"), len(plan_routes), "route matrix route_count")
        self.expect_equal(
            matrix.get("passed_route_count"),
            len(plan_routes),
            "route matrix passed_route_count",
        )
        self.expect_equal(matrix.get("failed_route_count"), 0, "route matrix failed_route_count")
        self.expect_equal(matrix.get("replay_failure_count"), 0, "route matrix replay_failure_count")
        self.expect_equal(matrix.get("test_failure_count"), 0, "route matrix test_failure_count")
        if self.expect_matrix_tests_skipped:
            self.expect_equal(matrix.get("tests_skipped"), True, "route matrix tests_skipped")

        artifacts = as_dict(matrix.get("artifacts"))
        self.expect_path_suffix(
            artifacts.get("route_matrix_json"),
            "diagnostic-route-matrix.json",
            "route matrix route_matrix_json artifact",
        )
        self.expect_path_suffix(
            artifacts.get("route_matrix_report"),
            "diagnostic-route-matrix.md",
            "route matrix route_matrix_report artifact",
        )
        self.expect_nonempty_list(matrix.get("ai_handoff"), "route matrix ai_handoff")

        rows = as_list(matrix.get("routes"))
        self.expect_equal(len(rows), len(plan_routes), "route matrix row count")
        rows_by_id = {row.get("route_id"): row for row in rows if isinstance(row, dict)}
        for route in plan_routes:
            route_id = str(route.get("route_id") or "")
            row = as_dict(rows_by_id.get(route_id))
            if not row:
                self.errors.append(f"route matrix missing row for {route_id}")
                continue
            scenario_id = str(route.get("primary_scenario_id") or "")
            scenario = scenarios.get(scenario_id, {})
            label = f"route matrix {route_id}"
            self.expect_equal(row.get("rank"), route.get("rank"), f"{label} rank")
            self.expect_equal(row.get("focus_domain"), route.get("focus_domain"), f"{label} focus_domain")
            self.expect_equal(
                row.get("primary_scenario_id"),
                scenario_id,
                f"{label} primary_scenario_id",
            )
            self.expect_equal(row.get("status"), "passed", f"{label} status")
            self.expect_equal(row.get("replay_status"), "passed", f"{label} replay_status")
            self.expect_equal(
                row.get("required_artifacts_present"),
                True,
                f"{label} required_artifacts_present",
            )
            self.expect_equal(row.get("actual_health"), scenario.get("expected_health"), f"{label} health")
            self.expect_equal(
                row.get("actual_focus_domain"),
                scenario.get("expected_focus_domain"),
                f"{label} focus domain",
            )
            expected_route_dir = self.matrix_dir / sanitize_path_component(route_id)
            self.expect_path_suffix(
                row.get("route_check_json"),
                f"{sanitize_path_component(route_id)}/diagnostic-route-check.json",
                f"{label} route_check_json",
            )
            self.expect_path_suffix(
                row.get("replay_bundle_triage_json"),
                f"{sanitize_path_component(route_id)}/replay-bundle/triage.json",
                f"{label} replay_bundle_triage_json",
            )
            self.verify_route_check(
                expected_route_dir / "diagnostic-route-check.json",
                route,
                scenario,
                label,
                expect_tests_skipped=bool(matrix.get("tests_skipped")),
            )
        return matrix

    def verify_route_check(
        self,
        route_json: Path,
        expected_route: dict[str, Any],
        scenario: dict[str, Any],
        label: str,
        expect_tests_skipped: bool | None,
    ) -> dict[str, Any]:
        summary = self.read_json_file(route_json, f"{label} JSON")
        route_dir = route_json.parent
        report_path = route_dir / "diagnostic-route-check.md"
        self.expect_file(report_path, f"{label} report")
        if report_path.is_file():
            report = report_path.read_text(encoding="utf-8")
            for section in ("# Diagnostic Route Check", "## Replay", "## Narrow Tests", "## AI Handoff"):
                self.expect_contains(report, section, f"{label} report")

        route_id = str(expected_route.get("route_id") or "")
        scenario_id = str(expected_route.get("primary_scenario_id") or "")
        selection = as_dict(summary.get("selection"))
        replay = as_dict(summary.get("replay"))
        tests = as_dict(summary.get("tests"))
        artifacts = as_dict(summary.get("artifacts"))
        embedded_route = as_dict(summary.get("route"))

        self.expect_equal(
            summary.get("diagnostic_route_check_schema_version"),
            EXPECTED_ROUTE_CHECK_SCHEMA,
            f"{label} schema version",
        )
        self.expect_equal(summary.get("status"), "passed", f"{label} status")
        self.expect_equal(summary.get("recommended_exit_code"), 0, f"{label} recommended_exit_code")
        self.expect_equal(selection.get("route_id"), route_id, f"{label} route_id")
        self.expect_equal(selection.get("rank"), expected_route.get("rank"), f"{label} rank")
        self.expect_equal(
            selection.get("focus_domain"),
            expected_route.get("focus_domain"),
            f"{label} focus_domain",
        )
        self.expect_equal(selection.get("primary_scenario_id"), scenario_id, f"{label} scenario_id")
        self.expect_equal(embedded_route.get("route_id"), route_id, f"{label} embedded route_id")
        self.expect_nonempty_list(summary.get("ai_handoff"), f"{label} ai_handoff")
        self.expect_equal(summary.get("errors"), [], f"{label} errors")

        self.expect_equal(replay.get("status"), "passed", f"{label} replay status")
        for field in (
            "exit_code_matches_expected",
            "health_matches_expected",
            "focus_test_matches_expected",
            "focus_domain_matches_expected",
            "required_artifacts_present",
        ):
            self.expect_equal(replay.get(field), True, f"{label} replay {field}")
        self.expect_equal(
            replay.get("expected_runner_exit_code"),
            scenario.get("expected_runner_exit_code"),
            f"{label} expected runner exit code",
        )
        self.expect_equal(
            as_dict(replay.get("command")).get("exit_code"),
            scenario.get("expected_runner_exit_code"),
            f"{label} actual runner exit code",
        )
        self.expect_equal(replay.get("actual_health"), scenario.get("expected_health"), f"{label} health")
        self.expect_equal(
            replay.get("actual_focus_domain"),
            scenario.get("expected_focus_domain"),
            f"{label} focus domain",
        )
        expected_focus_test_id = scenario.get("expected_focus_test_id")
        if expected_focus_test_id is not None:
            self.expect_equal(
                replay.get("actual_focus_test_id"),
                expected_focus_test_id,
                f"{label} focus test id",
            )

        if expect_tests_skipped is not None:
            self.expect_equal(tests.get("skipped"), expect_tests_skipped, f"{label} tests skipped")
        self.expect_equal(tests.get("status"), "passed", f"{label} tests status")
        commands = as_list(tests.get("commands"))
        if tests.get("skipped"):
            self.expect_equal(tests.get("command_count"), 0, f"{label} skipped test command_count")
        else:
            if tests.get("command_count", 0) < 1:
                self.errors.append(f"{label} must include at least one narrow test command")
            for index, command in enumerate(commands, start=1):
                self.expect_equal(command.get("exit_code"), 0, f"{label} test command {index} exit_code")

        self.verify_replay_bundle(route_dir / "replay-bundle", artifacts, replay, label)
        return summary

    def verify_replay_bundle(
        self,
        bundle_dir: Path,
        artifacts: dict[str, Any],
        replay: dict[str, Any],
        label: str,
    ) -> None:
        self.expect_dir(bundle_dir, f"{label} replay bundle dir")
        for key, filename in REQUIRED_REPLAY_BUNDLE_FILES.items():
            artifact_name = {
                "manifest": "replay_bundle_manifest",
                "triage": "replay_bundle_triage_json",
                "telemetry": "replay_bundle_telemetry_json",
                "report": "replay_bundle_report",
                "rom": "replay_bundle_rom",
            }[key]
            self.expect_path_suffix(
                artifacts.get(artifact_name),
                f"replay-bundle/{filename}",
                f"{label} artifact {artifact_name}",
            )
            self.expect_file(bundle_dir / filename, f"{label} replay bundle {filename}")

        manifest = self.read_json_file(bundle_dir / "manifest.json", f"{label} replay manifest")
        triage = self.read_json_file(bundle_dir / "triage.json", f"{label} replay triage")
        telemetry = self.read_json_file(bundle_dir / "telemetry.json", f"{label} replay telemetry")
        self.expect_equal(triage.get("health"), replay.get("actual_health"), f"{label} triage health")
        self.expect_equal(
            as_dict(triage.get("debug_focus")).get("focus_domain"),
            replay.get("actual_focus_domain"),
            f"{label} triage focus_domain",
        )
        self.expect_equal(
            as_dict(telemetry.get("analysis")).get("health"),
            replay.get("actual_health"),
            f"{label} telemetry analysis health",
        )
        if "passed" not in manifest:
            self.errors.append(f"{label} replay manifest missing passed field")

    def scenarios_by_id(self) -> dict[str, dict[str, Any]]:
        manifest = self.read_json_file(self.suite_dir / "scenario-suite.json", "scenario suite")
        scenarios = {}
        for scenario in as_list(manifest.get("scenarios")):
            if isinstance(scenario, dict) and isinstance(scenario.get("id"), str):
                scenarios[scenario["id"]] = scenario
        return scenarios

    def investigation_routes(self, plan: dict[str, Any]) -> list[dict[str, Any]]:
        routes = [route for route in as_list(plan.get("routes")) if isinstance(route, dict)]
        return sorted(
            routes,
            key=lambda route: (
                route.get("rank") if isinstance(route.get("rank"), int) else 1_000_000,
                str(route.get("route_id") or ""),
            ),
        )

    def top_route_id(self, plan: dict[str, Any], plan_routes: list[dict[str, Any]]) -> str:
        top_route_id = as_dict(plan.get("top_route")).get("route_id")
        if isinstance(top_route_id, str) and top_route_id:
            return top_route_id
        if plan_routes:
            return str(plan_routes[0].get("route_id") or "route")
        return "route"

    def top_route_dir(self, top_route_id: str) -> Path:
        if self.top_route_dir_arg is not None:
            return self.top_route_dir_arg
        return self.suite_dir / "route-checks" / sanitize_path_component(top_route_id)

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

    def expect_file(self, path: Path, label: str) -> None:
        if not path.is_file():
            self.errors.append(f"missing {label}: {path}")

    def expect_dir(self, path: Path, label: str) -> None:
        if not path.is_dir():
            self.errors.append(f"missing {label}: {path}")

    def expect_equal(self, actual: Any, expected: Any, label: str) -> None:
        if actual != expected:
            self.errors.append(f"{label}: expected {expected!r}, got {actual!r}")

    def expect_contains(self, text: str, needle: str, label: str) -> None:
        if needle not in text:
            self.errors.append(f"{label}: missing {needle!r}")

    def expect_nonempty_list(self, value: Any, label: str) -> None:
        if not isinstance(value, list) or not value:
            self.errors.append(f"{label} must be a non-empty array")

    def expect_path_suffix(self, value: Any, suffix: str, label: str) -> None:
        if not isinstance(value, str) or not value:
            self.errors.append(f"{label} must be a non-empty string path")
            return
        expected = suffix.replace("\\", "/")
        if not normalized_path(value).endswith(expected):
            self.errors.append(f"{label}: expected path suffix {expected!r}, got {value!r}")


def summary_paths(args: argparse.Namespace) -> tuple[Path | None, Path | None]:
    if not args.write_summary and args.summary_json is None and args.summary_report is None:
        return None, None
    summary_json = args.summary_json or args.suite_dir / DEFAULT_VERIFICATION_JSON_NAME
    summary_report = args.summary_report or args.suite_dir / DEFAULT_VERIFICATION_REPORT_NAME
    return summary_json, summary_report


def build_summary(
    verifier: RouteEvidenceVerifier,
    verification: dict[str, Any],
    status: str,
    recommended_exit_code: int,
    summary_json: Path | None,
    summary_report: Path | None,
) -> dict[str, Any]:
    top_route_id = str(verification.get("top_route_id") or "")
    top_route_dir = verifier.top_route_dir(top_route_id)
    matrix_json = verifier.matrix_dir / "diagnostic-route-matrix.json"
    matrix_report = matrix_json.with_name("diagnostic-route-matrix.md")
    top_route_json = top_route_dir / "diagnostic-route-check.json"
    top_route_report = top_route_dir / "diagnostic-route-check.md"

    artifacts: dict[str, str] = {
        "route_matrix_json": str(matrix_json),
        "route_matrix_report": str(matrix_report),
        "top_route_check_json": str(top_route_json),
        "top_route_check_report": str(top_route_report),
    }
    if summary_json is not None:
        artifacts["diagnostic_route_evidence_verification_json"] = str(summary_json)
    if summary_report is not None:
        artifacts["diagnostic_route_evidence_verification_report"] = str(summary_report)

    return {
        "diagnostic_route_evidence_verification_schema_version": ROUTE_EVIDENCE_VERIFICATION_SCHEMA_VERSION,
        "generated_at_utc": utc_now_text(),
        "status": status,
        "recommended_exit_code": recommended_exit_code,
        "suite_dir": str(verifier.suite_dir),
        "route_count": verification.get("route_count"),
        "matrix_verified": bool(verification.get("matrix_verified")),
        "matrix_route_count": verification.get("matrix_route_count"),
        "matrix_passed_route_count": verification.get("matrix_passed_route_count"),
        "matrix_replay_failure_count": verification.get("matrix_replay_failure_count"),
        "top_route_verified": bool(verification.get("top_route_verified")),
        "top_route_id": top_route_id,
        "configuration": {
            "matrix_dir": str(verifier.matrix_dir),
            "top_route_dir": str(top_route_dir),
            "require_matrix": verifier.require_matrix,
            "require_top_route": verifier.require_top_route,
            "expect_matrix_tests_skipped": verifier.expect_matrix_tests_skipped,
        },
        "artifacts": artifacts,
        "errors": list(verifier.errors),
        "ai_handoff": [
            "Read this summary first to decide whether diagnostic route evidence is trusted.",
            "If status is failed, triage the errors array before replaying routes.",
            "If status is passed, use route_matrix_json for broad route coverage and top_route_check_json for the full highest-ranked route proof.",
        ],
    }


def render_summary_report(summary: dict[str, Any]) -> str:
    artifacts = as_dict(summary.get("artifacts"))
    configuration = as_dict(summary.get("configuration"))
    errors = [str(error) for error in as_list(summary.get("errors"))]
    handoff = [str(step) for step in as_list(summary.get("ai_handoff"))]

    lines = [
        "# Diagnostic Route Evidence Verification",
        "",
        "## Verdict",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Status | {markdown_value(summary.get('status'))} |",
        f"| Recommended exit code | {markdown_value(summary.get('recommended_exit_code'))} |",
        f"| Generated at UTC | {markdown_value(summary.get('generated_at_utc'))} |",
        f"| Suite directory | `{markdown_value(summary.get('suite_dir'))}` |",
        "",
        "## Route Matrix",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Verified | {markdown_value(summary.get('matrix_verified'))} |",
        f"| Route count | {markdown_value(summary.get('matrix_route_count'))} |",
        f"| Passed route count | {markdown_value(summary.get('matrix_passed_route_count'))} |",
        f"| Replay failure count | {markdown_value(summary.get('matrix_replay_failure_count'))} |",
        f"| Tests skipped expected | {markdown_value(configuration.get('expect_matrix_tests_skipped'))} |",
        f"| Matrix JSON | `{markdown_value(artifacts.get('route_matrix_json'))}` |",
        f"| Matrix report | `{markdown_value(artifacts.get('route_matrix_report'))}` |",
        "",
        "## Top Route",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Verified | {markdown_value(summary.get('top_route_verified'))} |",
        f"| Route ID | {markdown_value(summary.get('top_route_id'))} |",
        f"| Route check JSON | `{markdown_value(artifacts.get('top_route_check_json'))}` |",
        f"| Route check report | `{markdown_value(artifacts.get('top_route_check_report'))}` |",
        "",
        "## Configuration",
        "",
        "| Field | Value |",
        "| --- | --- |",
        f"| Require matrix | {markdown_value(configuration.get('require_matrix'))} |",
        f"| Require top route | {markdown_value(configuration.get('require_top_route'))} |",
        f"| Matrix directory | `{markdown_value(configuration.get('matrix_dir'))}` |",
        f"| Top-route directory | `{markdown_value(configuration.get('top_route_dir'))}` |",
        "",
        "## Artifacts",
        "",
    ]
    for name, path in sorted(artifacts.items()):
        lines.append(f"- `{name}`: `{path}`")

    lines.extend(["", "## AI Handoff", ""])
    lines.extend(f"- {step}" for step in handoff)

    if errors:
        lines.extend(["", "## Errors", ""])
        lines.extend(f"- {error}" for error in errors)

    return "\n".join(lines) + "\n"


def write_summary_files(
    summary: dict[str, Any],
    summary_json: Path | None,
    summary_report: Path | None,
) -> None:
    if summary_json is not None:
        summary_json.parent.mkdir(parents=True, exist_ok=True)
        summary_json.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    if summary_report is not None:
        summary_report.parent.mkdir(parents=True, exist_ok=True)
        summary_report.write_text(render_summary_report(summary), encoding="utf-8")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate OxideNES diagnostic route evidence artifacts."
    )
    parser.add_argument(
        "--suite-dir",
        required=True,
        type=Path,
        help="Directory produced by scripts/run_diagnostic_observability.py plus route checks.",
    )
    parser.add_argument(
        "--matrix-dir",
        type=Path,
        help="Directory containing diagnostic-route-matrix.json. Defaults to <suite-dir>/route-replay-matrix.",
    )
    parser.add_argument(
        "--top-route-dir",
        type=Path,
        help="Directory containing the top-route diagnostic-route-check.json. Defaults to <suite-dir>/route-checks/<top-route>.",
    )
    parser.add_argument(
        "--require-matrix",
        action="store_true",
        help="Fail if the all-route matrix artifact is missing.",
    )
    parser.add_argument(
        "--require-top-route",
        action="store_true",
        help="Fail if the top-route full route-check artifact is missing.",
    )
    parser.add_argument(
        "--expect-matrix-tests-skipped",
        action="store_true",
        help="Require the all-route matrix to have skipped narrow tests.",
    )
    parser.add_argument(
        "--write-summary",
        action="store_true",
        help=(
            "Write diagnostic-route-evidence-verification.json and "
            "diagnostic-route-evidence-verification.md under --suite-dir."
        ),
    )
    parser.add_argument(
        "--summary-json",
        type=Path,
        help=(
            "Path for the machine-readable verification summary. "
            "Implies --write-summary for this file."
        ),
    )
    parser.add_argument(
        "--summary-report",
        type=Path,
        help=(
            "Path for the Markdown verification summary. "
            "Implies --write-summary for this file."
        ),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    verifier = RouteEvidenceVerifier(
        args.suite_dir,
        args.matrix_dir,
        args.top_route_dir,
        args.require_matrix,
        args.require_top_route,
        args.expect_matrix_tests_skipped,
    )
    summary = verifier.verify()
    summary_json, summary_report = summary_paths(args)
    status = "failed" if verifier.errors else "passed"
    recommended_exit_code = 1 if verifier.errors else 0
    verification_summary = build_summary(
        verifier,
        summary,
        status,
        recommended_exit_code,
        summary_json,
        summary_report,
    )
    write_summary_files(verification_summary, summary_json, summary_report)
    if verifier.errors:
        print("Diagnostic route evidence verification failed:", file=sys.stderr)
        for error in verifier.errors:
            print(f"- {error}", file=sys.stderr)
        if summary_json is not None or summary_report is not None:
            print(
                "Diagnostic route evidence verification summary written: "
                f"json={summary_json} report={summary_report}",
                file=sys.stderr,
            )
        return 1
    print(
        "Diagnostic route evidence verification passed: "
        f"routes={summary['route_count']} "
        f"matrix={summary['matrix_verified']}:{summary.get('matrix_passed_route_count')} "
        f"replay_failures={summary.get('matrix_replay_failure_count')} "
        f"top_route={summary['top_route_verified']}:{summary.get('top_route_id')}"
    )
    if summary_json is not None or summary_report is not None:
        print(
            "Diagnostic route evidence verification summary written: "
            f"json={summary_json} report={summary_report}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
