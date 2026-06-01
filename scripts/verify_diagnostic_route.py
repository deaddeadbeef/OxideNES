#!/usr/bin/env python3
"""Validate OxideNES diagnostic route-check and route-matrix artifacts."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


EXPECTED_ROUTE_CHECK_SCHEMA = 1
EXPECTED_ROUTE_MATRIX_SCHEMA = 1
EXPECTED_INVESTIGATION_PLAN_SCHEMA = 1
REQUIRED_REPLAY_BUNDLE_FILES = {
    "manifest": "manifest.json",
    "triage": "triage.json",
    "telemetry": "telemetry.json",
    "report": "report.md",
    "rom": "diagnostic.nes",
}


def as_dict(value: Any) -> dict[str, Any]:
    return value if isinstance(value, dict) else {}


def as_list(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def sanitize_path_component(value: str) -> str:
    cleaned = "".join(char if char.isalnum() or char in "_.-" else "-" for char in value.strip())
    return cleaned.strip(".-") or "route"


def path_text(value: Any) -> str:
    return value if isinstance(value, str) else ""


def normalized_path(value: Any) -> str:
    return path_text(value).replace("\\", "/")


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
    if verifier.errors:
        print("Diagnostic route evidence verification failed:", file=sys.stderr)
        for error in verifier.errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print(
        "Diagnostic route evidence verification passed: "
        f"routes={summary['route_count']} "
        f"matrix={summary['matrix_verified']}:{summary.get('matrix_passed_route_count')} "
        f"replay_failures={summary.get('matrix_replay_failure_count')} "
        f"top_route={summary['top_route_verified']}:{summary.get('top_route_id')}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
