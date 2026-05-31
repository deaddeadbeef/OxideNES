use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn diagnostic_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_oxidenes-diagnostic"))
}

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oxidenes-diagnostic-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("json file should be readable"))
        .expect("json file should parse")
}

#[test]
fn diagnostic_cli_writes_ai_ready_bundle() {
    let bundle_dir = temp_dir("bundle-pass");

    let status = Command::new(diagnostic_bin())
        .arg("--bundle-dir")
        .arg(&bundle_dir)
        .arg("--no-stdout")
        .status()
        .expect("diagnostic command should run");

    assert!(status.success());
    assert_bundle_artifacts(&bundle_dir, false, true);

    fs::remove_dir_all(&bundle_dir).expect("bundle temp dir should be removable");
}

#[test]
fn diagnostic_cli_bundle_includes_baseline_comparison() {
    let root = temp_dir("bundle-comparison");
    let baseline = root.join("baseline.json");
    let bundle_dir = root.join("bundle");
    fs::create_dir_all(&root).expect("temp dir should be created");

    let baseline_status = Command::new(diagnostic_bin())
        .arg("--json")
        .arg(&baseline)
        .arg("--no-stdout")
        .status()
        .expect("baseline diagnostic command should run");
    assert!(baseline_status.success());

    let bundle_status = Command::new(diagnostic_bin())
        .arg("--bundle-dir")
        .arg(&bundle_dir)
        .arg("--baseline-json")
        .arg(&baseline)
        .arg("--no-stdout")
        .status()
        .expect("comparison bundle diagnostic command should run");

    assert!(bundle_status.success());
    assert_bundle_artifacts(&bundle_dir, true, true);
    let triage = read_json(&bundle_dir.join("triage.json"));
    assert_eq!(triage["comparison"]["passed"], Value::Bool(true));
    assert_eq!(triage["comparison"]["difference_count"], Value::from(0));
    let comparison = read_json(&bundle_dir.join("comparison.json"));
    assert_eq!(comparison["passed"], Value::Bool(true));
    assert_eq!(comparison["difference_count"], Value::from(0));

    fs::remove_dir_all(&root).expect("bundle temp dir should be removable");
}

#[test]
fn diagnostic_cli_writes_ai_ready_scenario_suite() {
    let suite_dir = temp_dir("scenario-suite");

    let status = Command::new(diagnostic_bin())
        .arg("--scenario-suite-dir")
        .arg(&suite_dir)
        .arg("--no-stdout")
        .status()
        .expect("scenario suite diagnostic command should run");

    assert!(status.success());
    let manifest = read_json(&suite_dir.join("scenario-suite.json"));
    assert_eq!(manifest["scenario_suite_schema_version"], Value::from(6));
    assert_eq!(manifest["telemetry_schema_version"], Value::from(18));
    assert_eq!(manifest["triage_schema_version"], Value::from(6));
    assert_eq!(manifest["bundle_schema_version"], Value::from(2));
    assert_eq!(manifest["passed"], Value::Bool(true));
    assert_eq!(manifest["recommended_exit_code"], Value::from(0));
    assert_eq!(
        manifest["baseline_scenario_id"],
        Value::String("pass".to_string())
    );
    assert_eq!(manifest["scenario_count"], Value::from(5));
    assert_eq!(
        manifest["artifacts"]["scenario_suite_json"],
        Value::String("scenario-suite.json".to_string())
    );
    assert_eq!(
        manifest["artifacts"]["scenario_suite_report"],
        Value::String("scenario-suite.md".to_string())
    );
    assert_eq!(
        manifest["artifacts"]["scenario_suite_observer_json"],
        Value::String("scenario-suite-observer.json".to_string())
    );
    assert_eq!(
        manifest["artifacts"]["scenario_suite_observer_report"],
        Value::String("scenario-suite-observer.md".to_string())
    );
    assert_eq!(
        manifest["analysis"]["status"],
        Value::String("passed".to_string())
    );
    assert_eq!(manifest["analysis"]["scenario_count"], Value::from(5));
    assert_eq!(
        manifest["analysis"]["expectation_met_count"],
        Value::from(5)
    );
    assert_eq!(
        manifest["analysis"]["expectation_mismatch_count"],
        Value::from(0)
    );
    assert_eq!(
        manifest["analysis"]["contract_mismatch_count"],
        Value::from(0)
    );
    assert_eq!(
        manifest["analysis"]["baseline_divergence_count"],
        Value::from(4)
    );
    assert_eq!(
        manifest["analysis"]["critical_scenario_ids"],
        Value::Array(Vec::new())
    );
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("timeout_cycle_limit".to_string())));
    let attention_queue = manifest["analysis"]["attention_queue"]
        .as_array()
        .expect("attention queue should be an array");
    assert_eq!(attention_queue.len(), 4);
    let timeout_attention = find_attention_item(attention_queue, "timeout_cycle_limit");
    assert_eq!(
        timeout_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        timeout_attention["reason"],
        Value::String("scenario_diverges_from_pass_baseline".to_string())
    );
    assert_eq!(
        timeout_attention["next_artifact"],
        Value::String("timeout_cycle_limit/comparison.json".to_string())
    );
    assert_eq!(
        timeout_attention["comparison_difference_count"],
        Value::from(92)
    );
    let ppu_attention = find_attention_item(attention_queue, "ppu_read_buffer_fault");
    assert_eq!(
        ppu_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        ppu_attention["focus_domain"],
        Value::String("ppu.registers.ppudata_buffer".to_string())
    );
    assert_eq!(
        ppu_attention["next_artifact"],
        Value::String("ppu_read_buffer_fault/comparison.json".to_string())
    );
    assert!(manifest["ai_handoff"]
        .as_array()
        .expect("scenario ai_handoff should be an array")
        .iter()
        .any(|entry| entry
            .as_str()
            .is_some_and(|text| text.contains("scenario-suite-observer.json"))));

    let observer = read_json(&suite_dir.join("scenario-suite-observer.json"));
    assert_eq!(observer["observer_schema_version"], Value::from(1));
    assert_eq!(observer["scenario_suite_schema_version"], Value::from(6));
    assert_eq!(observer["telemetry_schema_version"], Value::from(18));
    assert_eq!(observer["triage_schema_version"], Value::from(6));
    assert_eq!(observer["bundle_schema_version"], Value::from(2));
    assert_eq!(observer["status"], Value::String("passed".to_string()));
    assert_eq!(observer["recommended_exit_code"], Value::from(0));
    assert_eq!(observer["scenario_count"], Value::from(5));
    assert_eq!(observer["contract_mismatch_count"], Value::from(0));
    assert_eq!(observer["baseline_divergence_count"], Value::from(4));
    let observer_actions = observer["next_actions"]
        .as_array()
        .expect("observer next_actions should be an array");
    assert_eq!(observer_actions.len(), 4);
    let timeout_action = find_observer_action(observer_actions, "timeout_cycle_limit");
    assert_eq!(
        timeout_action["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        timeout_action["action_type"],
        Value::String("inspect_known_divergence".to_string())
    );
    assert_eq!(
        timeout_action["primary_artifact"],
        Value::String("timeout_cycle_limit/comparison.json".to_string())
    );
    assert!(timeout_action["supporting_artifacts"]
        .as_array()
        .expect("timeout supporting artifacts should be an array")
        .iter()
        .any(|artifact| artifact == &Value::String("timeout_cycle_limit/triage.json".to_string())));
    assert!(timeout_action["evidence"]
        .as_array()
        .expect("timeout action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("comparison_difference_count=92".to_string())));
    assert!(timeout_action["evidence"]
        .as_array()
        .expect("timeout action evidence should be an array")
        .iter()
        .any(
            |entry| entry == &Value::String("top_difference_path=dma.oam_dma_observed".to_string())
        ));
    let ppu_action = find_observer_action(observer_actions, "ppu_read_buffer_fault");
    assert_eq!(
        ppu_action["primary_artifact"],
        Value::String("ppu_read_buffer_fault/comparison.json".to_string())
    );
    assert!(ppu_action["evidence"]
        .as_array()
        .expect("PPU action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String("focus_domain=ppu.registers.ppudata_buffer".to_string())));
    assert!(ppu_action["evidence"]
        .as_array()
        .expect("PPU action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.14.result".to_string()
            )));

    let observations = observer["observations"]
        .as_array()
        .expect("observer observations should be an array");
    assert_eq!(observations.len(), 5);
    let pass_observation = find_observer_observation(observations, "pass");
    assert_eq!(
        pass_observation["role"],
        Value::String("baseline".to_string())
    );
    assert_eq!(
        pass_observation["outcome"],
        Value::String("matches_baseline".to_string())
    );
    assert_eq!(
        pass_observation["next_artifact"],
        Value::String("pass/triage.json".to_string())
    );
    let timeout_observation = find_observer_observation(observations, "timeout_cycle_limit");
    assert_eq!(
        timeout_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        timeout_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        timeout_observation["focus_domain"],
        Value::String("emulator.progress_or_infinite_loop".to_string())
    );
    assert_eq!(
        timeout_observation["comparison_difference_count"],
        Value::from(92)
    );
    assert_eq!(
        timeout_observation["next_artifact"],
        Value::String("timeout_cycle_limit/comparison.json".to_string())
    );
    let ppu_observation = find_observer_observation(observations, "ppu_read_buffer_fault");
    assert_eq!(
        ppu_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        ppu_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        ppu_observation["focus_domain"],
        Value::String("ppu.registers.ppudata_buffer".to_string())
    );
    assert_eq!(
        ppu_observation["next_artifact"],
        Value::String("ppu_read_buffer_fault/comparison.json".to_string())
    );
    assert!(observer["artifact_hints"]
        .as_array()
        .expect("observer artifact hints should be an array")
        .iter()
        .any(|hint| hint["path"] == Value::String("scenario-suite.json".to_string())));

    let observer_report = fs::read_to_string(suite_dir.join("scenario-suite-observer.md"))
        .expect("scenario suite observer report should be readable");
    assert!(observer_report.contains("# Diagnostic Scenario Suite Observer"));
    assert!(observer_report.contains("## Next Actions"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | timeout_cycle_limit | timeout_cycle_limit/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_read_buffer_fault | ppu_read_buffer_fault/comparison.json |"));
    assert!(observer_report.contains("top_difference_path=dma.oam_dma_observed"));
    assert!(observer_report.contains("## Observations"));
    assert!(observer_report
        .contains("| pass | baseline | matches_baseline | healthy | - | 0 | pass/triage.json |"));
    assert!(observer_report.contains("| timeout_cycle_limit | expected_failure_fixture | expected_baseline_divergence | timed_out | emulator.progress_or_infinite_loop | 92 | timeout_cycle_limit/comparison.json |"));
    assert!(observer_report.contains("| ppu_read_buffer_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | ppu.registers.ppudata_buffer |"));
    assert!(observer_report.contains("## Artifact Hints"));
    assert!(observer_report.contains("scenario-suite.json"));

    let suite_report = fs::read_to_string(suite_dir.join("scenario-suite.md"))
        .expect("scenario suite report should be readable");
    assert!(suite_report.contains("# Diagnostic Scenario Suite"));
    assert!(suite_report.contains("## Suite Analysis"));
    assert!(suite_report.contains("| Status | passed |"));
    assert!(suite_report.contains("| Baseline divergences | 4 |"));
    assert!(suite_report.contains("## Attention Queue"));
    assert!(suite_report.contains("| known_divergence | timeout_cycle_limit | scenario_diverges_from_pass_baseline | timed_out | emulator.progress_or_infinite_loop | 92 | timeout_cycle_limit/comparison.json |"));
    assert!(suite_report.contains("| known_divergence | ppu_read_buffer_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | ppu.registers.ppudata_buffer |"));
    assert!(suite_report.contains("| Scenario | Expected pass | Actual pass |"));
    assert!(suite_report.contains("| joypad1_mismatch | false | false | true |"));
    assert!(suite_report.contains("| ppu_read_buffer_fault | false | false | true | cartridge_assertion_failed | 14 | ppu.registers.ppudata_buffer |"));
    assert!(suite_report.contains("cartridge.test.7.result"));
    assert!(suite_report.contains("| timeout_cycle_limit | false | false | true | timed_out | 0 | emulator.progress_or_infinite_loop |"));
    assert!(suite_report.contains("runtime.completed"));
    assert!(suite_report.contains("## Contract Matrix"));
    assert!(suite_report.contains("| joypad1_mismatch | true | true | true | true | true |"));
    assert!(suite_report.contains("| ppu_read_buffer_fault | true | true | true | true | true |"));
    assert!(suite_report.contains("| timeout_cycle_limit | true | true | true | true | true |"));
    assert!(suite_report.contains("## AI Drilldown"));
    assert!(suite_report.contains("## Baseline Comparison Matrix"));
    assert!(suite_report.contains("| pass | true | 0 | 0 | 0 | 0 | - |"));
    assert!(suite_report.contains("| joypad1_mismatch | false |"));
    assert!(suite_report.contains("pass/triage.json"));
    assert!(suite_report.contains("joypad2_mismatch/comparison.json"));

    let scenarios = manifest["scenarios"]
        .as_array()
        .expect("scenario manifest should list scenarios");
    let pass = find_scenario(scenarios, "pass");
    assert_eq!(pass["expected_passed"], Value::Bool(true));
    assert_eq!(pass["actual_passed"], Value::Bool(true));
    assert_eq!(pass["expectation_met"], Value::Bool(true));
    assert_eq!(pass["actual_health"], Value::String("healthy".to_string()));
    assert_eq!(pass["actual_focus_test_id"], Value::from(14));
    assert_eq!(pass["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(pass["contract"]["passed_matches"], Value::Bool(true));
    assert_eq!(pass["contract"]["health_matches"], Value::Bool(true));
    assert_eq!(pass["contract"]["focus_test_matches"], Value::Bool(true));
    assert_eq!(pass["contract"]["focus_domain_matches"], Value::Bool(true));
    assert_eq!(pass["contract"]["expected_passed"], Value::Bool(true));
    assert_eq!(pass["contract"]["actual_passed"], Value::Bool(true));
    assert_eq!(
        pass["contract"]["expected_health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        pass["contract"]["actual_health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(pass["contract"]["expected_focus_test_id"], Value::from(14));
    assert_eq!(pass["contract"]["actual_focus_test_id"], Value::from(14));
    assert_eq!(pass["comparison"]["passed"], Value::Bool(true));
    assert_eq!(pass["comparison"]["difference_count"], Value::from(0));
    assert_eq!(pass["comparison"]["failure_count"], Value::from(0));
    assert_eq!(
        pass["comparison"]["top_differences"],
        Value::Array(Vec::new())
    );
    assert_eq!(
        pass["artifacts"]["triage_json"],
        Value::String("pass/triage.json".to_string())
    );
    assert_bundle_artifacts(&suite_dir.join("pass"), true, true);

    let joypad1 = find_scenario(scenarios, "joypad1_mismatch");
    assert_eq!(joypad1["expected_runner_exit_code"], Value::from(1));
    assert_eq!(joypad1["expected_passed"], Value::Bool(false));
    assert_eq!(joypad1["actual_passed"], Value::Bool(false));
    assert_eq!(joypad1["expectation_met"], Value::Bool(true));
    assert_eq!(
        joypad1["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(joypad1["actual_focus_test_id"], Value::from(7));
    assert_eq!(
        joypad1["actual_focus_domain"],
        Value::String("joypad.strobe_shift".to_string())
    );
    assert_eq!(joypad1["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(joypad1["contract"]["passed_matches"], Value::Bool(true));
    assert_eq!(joypad1["contract"]["health_matches"], Value::Bool(true));
    assert_eq!(joypad1["contract"]["focus_test_matches"], Value::Bool(true));
    assert_eq!(
        joypad1["contract"]["focus_domain_matches"],
        Value::Bool(true)
    );
    assert_eq!(
        joypad1["contract"]["expected_focus_domain"],
        Value::String("joypad.strobe_shift".to_string())
    );
    assert_eq!(
        joypad1["contract"]["actual_focus_domain"],
        Value::String("joypad.strobe_shift".to_string())
    );
    assert!(joypad1["failed_probe_ids"]
        .as_array()
        .expect("joypad1 failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.7.result".to_string())));
    assert_eq!(joypad1["comparison"]["passed"], Value::Bool(false));
    assert!(
        joypad1["comparison"]["difference_count"]
            .as_u64()
            .expect("joypad1 comparison difference_count should be numeric")
            > 0
    );
    assert!(joypad1["comparison"]["top_differences"]
        .as_array()
        .expect("joypad1 top differences should be an array")
        .iter()
        .any(|difference| difference["path"]
            .as_str()
            .is_some_and(|path| path.contains("joypad1") || path.contains("verdict"))));
    assert_bundle_artifacts(&suite_dir.join("joypad1_mismatch"), true, false);

    let joypad2 = find_scenario(scenarios, "joypad2_mismatch");
    assert_eq!(joypad2["actual_focus_test_id"], Value::from(11));
    assert_eq!(
        joypad2["actual_focus_domain"],
        Value::String("joypad2.strobe_shift".to_string())
    );
    assert_eq!(joypad2["comparison"]["passed"], Value::Bool(false));
    assert!(
        joypad2["comparison"]["difference_count"]
            .as_u64()
            .expect("joypad2 comparison difference_count should be numeric")
            > 0
    );
    assert_bundle_artifacts_with_joypad2(&suite_dir.join("joypad2_mismatch"), true, false, "0x00");

    let ppu = find_scenario(scenarios, "ppu_read_buffer_fault");
    assert_eq!(
        ppu["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(ppu["actual_focus_test_id"], Value::from(14));
    assert_eq!(
        ppu["actual_focus_domain"],
        Value::String("ppu.registers.ppudata_buffer".to_string())
    );
    assert_eq!(ppu["expectation_met"], Value::Bool(true));
    assert_eq!(ppu["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        ppu["contract"]["expected_focus_domain"],
        Value::String("ppu.registers.ppudata_buffer".to_string())
    );
    assert!(ppu["failed_probe_ids"]
        .as_array()
        .expect("PPU failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.14.result".to_string())));
    assert_eq!(ppu["comparison"]["passed"], Value::Bool(false));
    assert!(
        ppu["comparison"]["difference_count"]
            .as_u64()
            .expect("PPU comparison difference_count should be numeric")
            > 0
    );
    let ppu_triage = read_json(&suite_dir.join("ppu_read_buffer_fault").join("triage.json"));
    assert_eq!(
        ppu_triage["input"]["fault_injection"],
        Value::String("ppu_vram_read_buffer".to_string())
    );
    assert_bundle_artifacts_with_config(
        &suite_dir.join("ppu_read_buffer_fault"),
        true,
        false,
        "0x28",
        Some("ppu_vram_read_buffer"),
    );

    let timeout = find_scenario(scenarios, "timeout_cycle_limit");
    assert_eq!(
        timeout["actual_health"],
        Value::String("timed_out".to_string())
    );
    assert_eq!(
        timeout["actual_focus_domain"],
        Value::String("emulator.progress_or_infinite_loop".to_string())
    );
    assert_eq!(timeout["expectation_met"], Value::Bool(true));
    assert_eq!(timeout["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(timeout["contract"]["expected_focus_test_id"], Value::Null);
    assert_eq!(timeout["contract"]["actual_focus_test_id"], Value::from(0));
    assert_eq!(timeout["contract"]["focus_test_matches"], Value::Bool(true));
    assert_eq!(
        timeout["contract"]["expected_focus_domain"],
        Value::String("emulator.progress_or_infinite_loop".to_string())
    );
    assert_eq!(timeout["comparison"]["passed"], Value::Bool(false));
    assert!(
        timeout["comparison"]["difference_count"]
            .as_u64()
            .expect("timeout comparison difference_count should be numeric")
            > 0
    );
    assert_bundle_artifacts(&suite_dir.join("timeout_cycle_limit"), true, false);

    fs::remove_dir_all(&suite_dir).expect("scenario suite temp dir should be removable");
}

#[test]
fn diagnostic_cli_writes_bundle_before_failure_exit() {
    let bundle_dir = temp_dir("bundle-fail");

    let status = Command::new(diagnostic_bin())
        .arg("--bundle-dir")
        .arg(&bundle_dir)
        .arg("--joypad1")
        .arg("0x00")
        .arg("--no-stdout")
        .status()
        .expect("failing diagnostic command should run");

    assert_eq!(status.code(), Some(1));
    assert_bundle_artifacts(&bundle_dir, false, false);
    let telemetry = read_json(&bundle_dir.join("telemetry.json"));
    assert_eq!(telemetry["verdict"]["passed"], Value::Bool(false));
    assert_eq!(
        telemetry["input"]["joypad1_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        telemetry["analysis"]["probe_summary"]["first_failed_probe"],
        Value::String("cartridge.status.pass".to_string())
    );
    let triage = read_json(&bundle_dir.join("triage.json"));
    assert_eq!(triage["passed"], Value::Bool(false));
    assert_eq!(triage["debug_focus"]["focus_test_id"], Value::from(7));
    assert_eq!(
        triage["debug_focus"]["focus_domain"],
        Value::String("joypad.strobe_shift".to_string())
    );
    assert!(triage["debug_focus"]["failed_probe_ids"]
        .as_array()
        .expect("debug focus failed probe ids should be an array")
        .iter()
        .any(|id| id == &Value::String("cartridge.test.7.result".to_string())));
    assert_eq!(
        triage["debug_focus"]["last_test_instruction"]["current_test_name"],
        Value::String("joypad_strobe_shift".to_string())
    );
    assert_eq!(
        triage["failure"]["likely_domain"],
        Value::String("joypad.strobe_shift".to_string())
    );
    assert_eq!(
        triage["probes"]["first_failed_probe"],
        Value::String("cartridge.status.pass".to_string())
    );
    assert!(triage["probes"]["failed"]
        .as_array()
        .expect("triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.7.result".to_string())));

    fs::remove_dir_all(&bundle_dir).expect("bundle temp dir should be removable");
}

#[test]
fn diagnostic_cli_can_localize_joypad2_override_failures() {
    let bundle_dir = temp_dir("bundle-fail-joypad2");

    let status = Command::new(diagnostic_bin())
        .arg("--bundle-dir")
        .arg(&bundle_dir)
        .arg("--joypad2")
        .arg("0x00")
        .arg("--no-stdout")
        .status()
        .expect("failing joypad2 diagnostic command should run");

    assert_eq!(status.code(), Some(1));
    assert_bundle_artifacts_with_joypad2(&bundle_dir, false, false, "0x00");
    let telemetry = read_json(&bundle_dir.join("telemetry.json"));
    assert_eq!(telemetry["verdict"]["current_test"], Value::from(11));
    assert_eq!(
        telemetry["input"]["joypad2_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        telemetry["input"]["joypad2_expected_mask_hex"],
        Value::String("0x28".to_string())
    );
    let triage = read_json(&bundle_dir.join("triage.json"));
    assert_eq!(
        triage["input"]["joypad2_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        triage["input"]["joypad2_expected_mask_hex"],
        Value::String("0x28".to_string())
    );
    assert_eq!(
        triage["failure"]["likely_domain"],
        Value::String("joypad2.strobe_shift".to_string())
    );
    assert_eq!(triage["debug_focus"]["focus_test_id"], Value::from(11));
    assert_eq!(
        triage["debug_focus"]["focus_domain"],
        Value::String("joypad2.strobe_shift".to_string())
    );
    assert!(triage["probes"]["failed"]
        .as_array()
        .expect("triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.11.result".to_string())));

    fs::remove_dir_all(&bundle_dir).expect("bundle temp dir should be removable");
}

#[test]
fn diagnostic_cli_writes_standalone_triage_json() {
    let root = temp_dir("triage-standalone");
    let triage_path = root.join("triage.json");
    fs::create_dir_all(&root).expect("temp dir should be created");

    let status = Command::new(diagnostic_bin())
        .arg("--triage-json")
        .arg(&triage_path)
        .arg("--no-stdout")
        .status()
        .expect("diagnostic command should run");

    assert!(status.success());
    let triage = read_json(&triage_path);
    assert_eq!(triage["triage_schema_version"], Value::from(6));
    assert_eq!(triage["telemetry_schema_version"], Value::from(18));
    assert_eq!(triage["passed"], Value::Bool(true));
    assert_eq!(
        triage["debug_focus"]["health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        triage["debug_focus"]["focus_test_name"],
        Value::String("ppu_vram_read_buffer".to_string())
    );
    assert_eq!(
        triage["debug_focus"]["terminal_instruction"]["symbol"],
        Value::String("hang".to_string())
    );
    assert!(triage["debug_focus"]["terminal_instruction"]["instruction"]
        .as_str()
        .is_some_and(|instruction| instruction.starts_with("JMP 0x")));
    assert_eq!(triage["coverage"]["passed_tests"], Value::from(14));
    assert_eq!(triage["dma"]["oam_dma_completed"], Value::Bool(true));
    assert!(triage["dma"]["oam_dma_active_cycles"]
        .as_u64()
        .is_some_and(|cycles| (513..=514).contains(&cycles)));
    assert_eq!(
        triage["dma"]["dmc_dma_oam_overlap_observed"],
        Value::Bool(true)
    );
    assert!(triage["dma"]["dmc_dma_fetches_during_oam_dma"]
        .as_u64()
        .is_some_and(|fetches| fetches >= 1));
    assert!(triage["dma"]["dmc_dma_first_oam_overlap_stall_cycles"]
        .as_u64()
        .is_some_and(|cycles| (3..=4).contains(&cycles)));
    assert!(triage["instruction_trace"]["captured_instruction_count"]
        .as_u64()
        .is_some_and(|count| count > 0));
    assert_eq!(
        triage["instruction_trace"]["retention_limit"],
        Value::from(64)
    );
    assert!(triage["instruction_trace"]["tail"]
        .as_array()
        .expect("instruction trace tail should be an array")
        .iter()
        .any(|entry| entry["opcode_hex"].as_str().is_some()
            && entry["instruction"].as_str().is_some()
            && entry["symbol"].as_str().is_some()));
    assert!(triage["coverage_gaps"]
        .as_array()
        .expect("coverage gaps should be an array")
        .iter()
        .any(|gap| gap["id"] == Value::String("ppu_pixel_pipeline".to_string())));
    assert_eq!(triage["probes"]["failed_probes"], Value::from(0));
    assert!(triage["artifact_hints"]
        .as_array()
        .expect("artifact hints should be an array")
        .iter()
        .any(|hint| hint["path"] == Value::String("telemetry.json".to_string())));
    assert!(triage["event_tail"]
        .as_array()
        .expect("event tail should be an array")
        .iter()
        .any(|event| event["cpu_status_hex"].as_str().is_some()
            && event["signature_hex"] == Value::String("0xA5".to_string())));

    fs::remove_dir_all(&root).expect("triage temp dir should be removable");
}

fn assert_bundle_artifacts(bundle_dir: &Path, includes_comparison: bool, passed: bool) {
    assert_bundle_artifacts_with_config(bundle_dir, includes_comparison, passed, "0x28", None);
}

fn assert_bundle_artifacts_with_joypad2(
    bundle_dir: &Path,
    includes_comparison: bool,
    passed: bool,
    expected_joypad2_mask_hex: &str,
) {
    assert_bundle_artifacts_with_config(
        bundle_dir,
        includes_comparison,
        passed,
        expected_joypad2_mask_hex,
        None,
    );
}

fn assert_bundle_artifacts_with_config(
    bundle_dir: &Path,
    includes_comparison: bool,
    passed: bool,
    expected_joypad2_mask_hex: &str,
    expected_fault_injection: Option<&str>,
) {
    let manifest = read_json(&bundle_dir.join("manifest.json"));
    assert_eq!(manifest["bundle_schema_version"], Value::from(2));
    assert_eq!(manifest["telemetry_schema_version"], Value::from(18));
    assert_eq!(manifest["passed"], Value::Bool(passed));
    assert_eq!(
        manifest["config"]["joypad2_mask_hex"],
        Value::String(expected_joypad2_mask_hex.to_string())
    );
    assert_eq!(
        manifest["config"]["fault_injection"],
        expected_fault_injection.map_or(Value::Null, |fault| Value::String(fault.to_string()))
    );
    assert_eq!(
        manifest["comparison_included"],
        Value::Bool(includes_comparison)
    );
    assert!(manifest["ai_handoff"]
        .as_array()
        .expect("ai_handoff should be an array")
        .iter()
        .any(|entry| entry
            .as_str()
            .is_some_and(|text| text.contains("triage.json"))));

    let artifacts = manifest["artifacts"]
        .as_array()
        .expect("artifacts should be an array");
    let expected_count = if includes_comparison { 6 } else { 4 };
    assert_eq!(artifacts.len(), expected_count);

    assert_manifest_artifact(bundle_dir, artifacts, "triage.json", "ai_triage_json");
    assert_manifest_artifact(bundle_dir, artifacts, "telemetry.json", "telemetry_json");
    assert_manifest_artifact(bundle_dir, artifacts, "report.md", "diagnostic_report");
    assert_manifest_artifact(
        bundle_dir,
        artifacts,
        "diagnostic.nes",
        "diagnostic_cartridge",
    );
    if includes_comparison {
        assert_manifest_artifact(bundle_dir, artifacts, "comparison.json", "comparison_json");
        assert_manifest_artifact(bundle_dir, artifacts, "comparison.md", "comparison_report");
    }
}

fn assert_manifest_artifact(bundle_dir: &Path, artifacts: &[Value], path: &str, kind: &str) {
    let artifact = artifacts
        .iter()
        .find(|artifact| artifact["path"] == Value::String(path.to_string()))
        .unwrap_or_else(|| panic!("missing bundle artifact {path}"));
    assert_eq!(artifact["kind"], Value::String(kind.to_string()));
    let data = fs::read(bundle_dir.join(path)).expect("artifact file should be readable");
    assert_eq!(artifact["bytes"], Value::from(data.len()));
    let digest = artifact["sha256"]
        .as_str()
        .expect("sha256 should be a string");
    assert_eq!(digest.len(), 64);
    assert!(digest.chars().all(|ch| ch.is_ascii_hexdigit()));
}

fn find_scenario<'a>(scenarios: &'a [Value], id: &str) -> &'a Value {
    scenarios
        .iter()
        .find(|scenario| scenario["id"] == Value::String(id.to_string()))
        .unwrap_or_else(|| panic!("missing diagnostic scenario {id}"))
}

fn find_attention_item<'a>(items: &'a [Value], scenario_id: &str) -> &'a Value {
    items
        .iter()
        .find(|item| item["scenario_id"] == Value::String(scenario_id.to_string()))
        .unwrap_or_else(|| panic!("missing diagnostic attention item {scenario_id}"))
}

fn find_observer_action<'a>(items: &'a [Value], scenario_id: &str) -> &'a Value {
    items
        .iter()
        .find(|item| item["scenario_id"] == Value::String(scenario_id.to_string()))
        .unwrap_or_else(|| panic!("missing diagnostic observer action {scenario_id}"))
}

fn find_observer_observation<'a>(items: &'a [Value], scenario_id: &str) -> &'a Value {
    items
        .iter()
        .find(|item| item["scenario_id"] == Value::String(scenario_id.to_string()))
        .unwrap_or_else(|| panic!("missing diagnostic observer observation {scenario_id}"))
}
