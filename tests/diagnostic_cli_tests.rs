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
fn diagnostic_cli_can_replay_named_fault_fixture() {
    let bundle_dir = temp_dir("bundle-fault-replay");

    let status = Command::new(diagnostic_bin())
        .arg("--bundle-dir")
        .arg(&bundle_dir)
        .arg("--fault-injection")
        .arg("joypad_strobe_high_hold")
        .arg("--no-stdout")
        .status()
        .expect("fault replay diagnostic command should run");

    assert_eq!(status.code(), Some(1));
    assert_bundle_artifacts_with_config(
        &bundle_dir,
        false,
        false,
        "0x28",
        Some("joypad_strobe_high_hold"),
    );
    let triage = read_json(&bundle_dir.join("triage.json"));
    assert_eq!(
        triage["input"]["fault_injection"],
        Value::String("joypad_strobe_high_hold".to_string())
    );
    assert_eq!(
        triage["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(triage["debug_focus"]["focus_test_id"], Value::from(21));
    assert_eq!(
        triage["debug_focus"]["focus_domain"],
        Value::String("joypad.strobe_high_hold".to_string())
    );
    assert_eq!(
        triage["failure"]["likely_domain"],
        Value::String("joypad.strobe_high_hold".to_string())
    );

    fs::remove_dir_all(&bundle_dir).expect("bundle temp dir should be removable");
}

#[test]
fn diagnostic_cli_rejects_unknown_fault_fixture() {
    let output = Command::new(diagnostic_bin())
        .arg("--fault-injection")
        .arg("not_a_fault")
        .arg("--no-stdout")
        .output()
        .expect("invalid fault diagnostic command should run");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("invalid --fault-injection value: not_a_fault"));
    assert!(stderr.contains("joypad_strobe_high_hold"));
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
    assert_eq!(manifest["scenario_suite_schema_version"], Value::from(11));
    assert_eq!(manifest["telemetry_schema_version"], Value::from(60));
    assert_eq!(manifest["triage_schema_version"], Value::from(6));
    assert_eq!(manifest["bundle_schema_version"], Value::from(3));
    assert_eq!(manifest["passed"], Value::Bool(true));
    assert_eq!(manifest["recommended_exit_code"], Value::from(0));
    assert_eq!(
        manifest["baseline_scenario_id"],
        Value::String("pass".to_string())
    );
    assert_eq!(manifest["scenario_count"], Value::from(34));
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
    assert_eq!(manifest["analysis"]["scenario_count"], Value::from(34));
    assert_eq!(
        manifest["analysis"]["expectation_met_count"],
        Value::from(34)
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
        Value::from(26)
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
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("ppu_nmi_timeout_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("mapper2_bank_switch_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("mapper2_prg_ram_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("ppu_nametable_mirroring_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("joypad_strobe_reset_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("joypad_strobe_high_hold_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("ppu_vram_increment_32_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("ppu_status_latch_reset_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("cpu_addressing_matrix_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("cpu_rmw_matrix_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("cpu_rmw_addressing_matrix_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("input_port_matrix_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("cpu_ram_mirroring_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("dma_phase_matrix_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("ppu_sprite_zero_hit_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("ppu_sprite_overflow_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("ppu_sprite_priority_fault".to_string())));
    assert!(manifest["analysis"]["known_divergence_scenario_ids"]
        .as_array()
        .expect("known divergence scenario ids should be an array")
        .iter()
        .any(|id| id == &Value::String("ppu_scroll_seam_fault".to_string())));
    let attention_queue = manifest["analysis"]["attention_queue"]
        .as_array()
        .expect("attention queue should be an array");
    assert_eq!(attention_queue.len(), 26);
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
    assert!(
        timeout_attention["comparison_difference_count"]
            .as_u64()
            .expect("timeout attention difference count should be numeric")
            > 0
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
    let bus_attention = find_attention_item(attention_queue, "cpu_ram_mirroring_fault");
    assert_eq!(
        bus_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        bus_attention["focus_domain"],
        Value::String("bus.cpu_ram_mirroring".to_string())
    );
    assert_eq!(
        bus_attention["next_artifact"],
        Value::String("cpu_ram_mirroring_fault/comparison.json".to_string())
    );
    let ppu_mirroring_attention =
        find_attention_item(attention_queue, "ppu_nametable_mirroring_fault");
    assert_eq!(
        ppu_mirroring_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        ppu_mirroring_attention["focus_domain"],
        Value::String("ppu.nametables.horizontal_mirroring".to_string())
    );
    assert_eq!(
        ppu_mirroring_attention["next_artifact"],
        Value::String("ppu_nametable_mirroring_fault/comparison.json".to_string())
    );
    let joypad_reset_attention = find_attention_item(attention_queue, "joypad_strobe_reset_fault");
    assert_eq!(
        joypad_reset_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        joypad_reset_attention["focus_domain"],
        Value::String("joypad.strobe_reset".to_string())
    );
    assert_eq!(
        joypad_reset_attention["next_artifact"],
        Value::String("joypad_strobe_reset_fault/comparison.json".to_string())
    );
    let joypad_hold_attention =
        find_attention_item(attention_queue, "joypad_strobe_high_hold_fault");
    assert_eq!(
        joypad_hold_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        joypad_hold_attention["focus_domain"],
        Value::String("joypad.strobe_high_hold".to_string())
    );
    assert_eq!(
        joypad_hold_attention["next_artifact"],
        Value::String("joypad_strobe_high_hold_fault/comparison.json".to_string())
    );
    let ppu_increment_32_attention =
        find_attention_item(attention_queue, "ppu_vram_increment_32_fault");
    assert_eq!(
        ppu_increment_32_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        ppu_increment_32_attention["focus_domain"],
        Value::String("ppu.registers.ppudata_increment_32".to_string())
    );
    assert_eq!(
        ppu_increment_32_attention["next_artifact"],
        Value::String("ppu_vram_increment_32_fault/comparison.json".to_string())
    );
    let ppu_status_latch_attention =
        find_attention_item(attention_queue, "ppu_status_latch_reset_fault");
    assert_eq!(
        ppu_status_latch_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        ppu_status_latch_attention["focus_domain"],
        Value::String("ppu.registers.status_latch_reset".to_string())
    );
    assert_eq!(
        ppu_status_latch_attention["next_artifact"],
        Value::String("ppu_status_latch_reset_fault/comparison.json".to_string())
    );
    let mapper_attention = find_attention_item(attention_queue, "mapper2_bank_switch_fault");
    assert_eq!(
        mapper_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        mapper_attention["focus_domain"],
        Value::String("mapper.uxrom.prg_bank_switch".to_string())
    );
    assert_eq!(
        mapper_attention["next_artifact"],
        Value::String("mapper2_bank_switch_fault/comparison.json".to_string())
    );
    let mapper_ram_attention = find_attention_item(attention_queue, "mapper2_prg_ram_fault");
    assert_eq!(
        mapper_ram_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        mapper_ram_attention["focus_domain"],
        Value::String("mapper.uxrom.prg_ram".to_string())
    );
    assert_eq!(
        mapper_ram_attention["next_artifact"],
        Value::String("mapper2_prg_ram_fault/comparison.json".to_string())
    );
    let ppu_nmi_attention = find_attention_item(attention_queue, "ppu_nmi_timeout_fault");
    assert_eq!(
        ppu_nmi_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        ppu_nmi_attention["focus_domain"],
        Value::String("ppu.nmi".to_string())
    );
    assert_eq!(
        ppu_nmi_attention["actual_health"],
        Value::String("timed_out".to_string())
    );
    assert_eq!(
        ppu_nmi_attention["next_artifact"],
        Value::String("ppu_nmi_timeout_fault/comparison.json".to_string())
    );
    let ppu_priority_attention = find_attention_item(attention_queue, "ppu_sprite_priority_fault");
    assert_eq!(
        ppu_priority_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        ppu_priority_attention["focus_domain"],
        Value::String("ppu.sprite_priority".to_string())
    );
    assert_eq!(
        ppu_priority_attention["actual_health"],
        Value::String("host_validation_failed".to_string())
    );
    assert_eq!(
        ppu_priority_attention["next_artifact"],
        Value::String("ppu_sprite_priority_fault/comparison.json".to_string())
    );
    let ppu_scroll_attention = find_attention_item(attention_queue, "ppu_scroll_seam_fault");
    assert_eq!(
        ppu_scroll_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        ppu_scroll_attention["focus_domain"],
        Value::String("ppu.scroll_seam".to_string())
    );
    assert_eq!(
        ppu_scroll_attention["actual_health"],
        Value::String("host_validation_failed".to_string())
    );
    assert_eq!(
        ppu_scroll_attention["next_artifact"],
        Value::String("ppu_scroll_seam_fault/comparison.json".to_string())
    );
    let dma_attention = find_attention_item(attention_queue, "dma_oam_transfer_fault");
    assert_eq!(
        dma_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        dma_attention["focus_domain"],
        Value::String("dma.oam_transfer".to_string())
    );
    assert_eq!(
        dma_attention["next_artifact"],
        Value::String("dma_oam_transfer_fault/comparison.json".to_string())
    );
    let apu_attention = find_attention_item(attention_queue, "apu_status_fault");
    assert_eq!(
        apu_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        apu_attention["focus_domain"],
        Value::String("apu.status".to_string())
    );
    assert_eq!(
        apu_attention["next_artifact"],
        Value::String("apu_status_fault/comparison.json".to_string())
    );
    let cpu_attention = find_attention_item(attention_queue, "cpu_zero_page_wrap_fault");
    assert_eq!(
        cpu_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        cpu_attention["focus_domain"],
        Value::String("cpu.addressing.zero_page_x_wrap".to_string())
    );
    assert_eq!(
        cpu_attention["next_artifact"],
        Value::String("cpu_zero_page_wrap_fault/comparison.json".to_string())
    );
    let jmp_attention = find_attention_item(attention_queue, "cpu_indirect_jmp_fault");
    assert_eq!(
        jmp_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        jmp_attention["focus_domain"],
        Value::String("cpu.control_flow.indirect_jmp_page_wrap".to_string())
    );
    assert_eq!(
        jmp_attention["next_artifact"],
        Value::String("cpu_indirect_jmp_fault/comparison.json".to_string())
    );
    let addressing_attention = find_attention_item(attention_queue, "cpu_addressing_matrix_fault");
    assert_eq!(
        addressing_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        addressing_attention["focus_domain"],
        Value::String("cpu.addressing.page_cross_load".to_string())
    );
    assert_eq!(
        addressing_attention["next_artifact"],
        Value::String("cpu_addressing_matrix_fault/comparison.json".to_string())
    );
    let rmw_attention = find_attention_item(attention_queue, "cpu_rmw_matrix_fault");
    assert_eq!(
        rmw_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        rmw_attention["focus_domain"],
        Value::String("cpu.rmw.asl".to_string())
    );
    assert_eq!(
        rmw_attention["next_artifact"],
        Value::String("cpu_rmw_matrix_fault/comparison.json".to_string())
    );
    let rmw_addressing_attention =
        find_attention_item(attention_queue, "cpu_rmw_addressing_matrix_fault");
    assert_eq!(
        rmw_addressing_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        rmw_addressing_attention["focus_domain"],
        Value::String("cpu.rmw.absolute_asl".to_string())
    );
    assert_eq!(
        rmw_addressing_attention["next_artifact"],
        Value::String("cpu_rmw_addressing_matrix_fault/comparison.json".to_string())
    );
    let input_port_attention = find_attention_item(attention_queue, "input_port_matrix_fault");
    assert_eq!(
        input_port_attention["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        input_port_attention["focus_domain"],
        Value::String("joypad.input_port_matrix".to_string())
    );
    assert_eq!(
        input_port_attention["next_artifact"],
        Value::String("input_port_matrix_fault/comparison.json".to_string())
    );
    assert!(manifest["ai_handoff"]
        .as_array()
        .expect("scenario ai_handoff should be an array")
        .iter()
        .any(|entry| entry
            .as_str()
            .is_some_and(|text| text.contains("scenario-suite-observer.json"))));

    let observer = read_json(&suite_dir.join("scenario-suite-observer.json"));
    assert_eq!(observer["observer_schema_version"], Value::from(2));
    assert_eq!(observer["scenario_suite_schema_version"], Value::from(11));
    assert_eq!(observer["telemetry_schema_version"], Value::from(60));
    assert_eq!(observer["triage_schema_version"], Value::from(6));
    assert_eq!(observer["bundle_schema_version"], Value::from(3));
    assert_eq!(observer["status"], Value::String("passed".to_string()));
    assert_eq!(observer["recommended_exit_code"], Value::from(0));
    assert_eq!(observer["scenario_count"], Value::from(34));
    assert_eq!(observer["contract_mismatch_count"], Value::from(0));
    assert_eq!(observer["baseline_divergence_count"], Value::from(26));
    let observer_actions = observer["next_actions"]
        .as_array()
        .expect("observer next_actions should be an array");
    assert_eq!(observer_actions.len(), 26);
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
        .any(|entry| entry
            .as_str()
            .is_some_and(|text| text.starts_with("comparison_difference_count="))));
    let bus_action = find_observer_action(observer_actions, "cpu_ram_mirroring_fault");
    assert_eq!(
        bus_action["priority"],
        Value::String("known_divergence".to_string())
    );
    assert_eq!(
        bus_action["action_type"],
        Value::String("inspect_known_divergence".to_string())
    );
    assert_eq!(
        bus_action["primary_artifact"],
        Value::String("cpu_ram_mirroring_fault/comparison.json".to_string())
    );
    assert!(bus_action["evidence"]
        .as_array()
        .expect("bus action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=bus.cpu_ram_mirroring".to_string())));
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
    let ppu_mirroring_action =
        find_observer_action(observer_actions, "ppu_nametable_mirroring_fault");
    assert_eq!(
        ppu_mirroring_action["primary_artifact"],
        Value::String("ppu_nametable_mirroring_fault/comparison.json".to_string())
    );
    assert!(ppu_mirroring_action["evidence"]
        .as_array()
        .expect("PPU mirroring action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String("focus_domain=ppu.nametables.horizontal_mirroring".to_string())));
    assert!(ppu_mirroring_action["evidence"]
        .as_array()
        .expect("PPU mirroring action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.17.result".to_string()
            )));
    let ppu_sprite_action = find_observer_action(observer_actions, "ppu_sprite_zero_hit_fault");
    assert_eq!(
        ppu_sprite_action["primary_artifact"],
        Value::String("ppu_sprite_zero_hit_fault/comparison.json".to_string())
    );
    assert!(ppu_sprite_action["evidence"]
        .as_array()
        .expect("PPU sprite-zero-hit action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=ppu.sprite_zero_hit".to_string())));
    assert!(ppu_sprite_action["evidence"]
        .as_array()
        .expect("PPU sprite-zero-hit action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.25.result".to_string()
            )));
    let ppu_overflow_action = find_observer_action(observer_actions, "ppu_sprite_overflow_fault");
    assert_eq!(
        ppu_overflow_action["primary_artifact"],
        Value::String("ppu_sprite_overflow_fault/comparison.json".to_string())
    );
    assert!(ppu_overflow_action["evidence"]
        .as_array()
        .expect("PPU sprite-overflow action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=ppu.sprite_overflow".to_string())));
    assert!(ppu_overflow_action["evidence"]
        .as_array()
        .expect("PPU sprite-overflow action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.26.result".to_string()
            )));
    let ppu_priority_action = find_observer_action(observer_actions, "ppu_sprite_priority_fault");
    assert_eq!(
        ppu_priority_action["primary_artifact"],
        Value::String("ppu_sprite_priority_fault/comparison.json".to_string())
    );
    assert!(ppu_priority_action["evidence"]
        .as_array()
        .expect("PPU sprite-priority action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=ppu.sprite_priority".to_string())));
    assert!(ppu_priority_action["evidence"]
        .as_array()
        .expect("PPU sprite-priority action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String("failed_probe_ids=ppu.sprite_priority.samples".to_string())));
    let ppu_scroll_action = find_observer_action(observer_actions, "ppu_scroll_seam_fault");
    assert_eq!(
        ppu_scroll_action["primary_artifact"],
        Value::String("ppu_scroll_seam_fault/comparison.json".to_string())
    );
    assert!(ppu_scroll_action["evidence"]
        .as_array()
        .expect("PPU scroll-seam action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=ppu.scroll_seam".to_string())));
    assert!(ppu_scroll_action["evidence"]
        .as_array()
        .expect("PPU scroll-seam action evidence should be an array")
        .iter()
        .any(
            |entry| entry == &Value::String("failed_probe_ids=ppu.scroll_seam.samples".to_string())
        ));
    let joypad_reset_action = find_observer_action(observer_actions, "joypad_strobe_reset_fault");
    assert_eq!(
        joypad_reset_action["primary_artifact"],
        Value::String("joypad_strobe_reset_fault/comparison.json".to_string())
    );
    assert!(joypad_reset_action["evidence"]
        .as_array()
        .expect("joypad reset action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=joypad.strobe_reset".to_string())));
    assert!(joypad_reset_action["evidence"]
        .as_array()
        .expect("joypad reset action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.18.result".to_string()
            )));
    let joypad_hold_action =
        find_observer_action(observer_actions, "joypad_strobe_high_hold_fault");
    assert_eq!(
        joypad_hold_action["primary_artifact"],
        Value::String("joypad_strobe_high_hold_fault/comparison.json".to_string())
    );
    assert!(joypad_hold_action["evidence"]
        .as_array()
        .expect("joypad strobe-high action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=joypad.strobe_high_hold".to_string())));
    assert!(joypad_hold_action["evidence"]
        .as_array()
        .expect("joypad strobe-high action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.21.result".to_string()
            )));
    assert_replay_args_contains(&joypad_hold_action["replay_args"], "--fault-injection");
    assert_replay_args_contains(
        &joypad_hold_action["replay_args"],
        "joypad_strobe_high_hold",
    );
    assert_replay_args_contains(
        &joypad_hold_action["replay_args"],
        "target/diagnostics/replay/joypad_strobe_high_hold_fault",
    );
    let ppu_increment_32_action =
        find_observer_action(observer_actions, "ppu_vram_increment_32_fault");
    assert_eq!(
        ppu_increment_32_action["primary_artifact"],
        Value::String("ppu_vram_increment_32_fault/comparison.json".to_string())
    );
    assert!(ppu_increment_32_action["evidence"]
        .as_array()
        .expect("PPU increment-32 action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String("focus_domain=ppu.registers.ppudata_increment_32".to_string())));
    assert!(ppu_increment_32_action["evidence"]
        .as_array()
        .expect("PPU increment-32 action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.19.result".to_string()
            )));
    let ppu_status_latch_action =
        find_observer_action(observer_actions, "ppu_status_latch_reset_fault");
    assert_eq!(
        ppu_status_latch_action["primary_artifact"],
        Value::String("ppu_status_latch_reset_fault/comparison.json".to_string())
    );
    assert!(ppu_status_latch_action["evidence"]
        .as_array()
        .expect("PPU status latch action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String("focus_domain=ppu.registers.status_latch_reset".to_string())));
    assert!(ppu_status_latch_action["evidence"]
        .as_array()
        .expect("PPU status latch action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.20.result".to_string()
            )));
    let mapper_action = find_observer_action(observer_actions, "mapper2_bank_switch_fault");
    assert_eq!(
        mapper_action["primary_artifact"],
        Value::String("mapper2_bank_switch_fault/comparison.json".to_string())
    );
    assert!(mapper_action["evidence"]
        .as_array()
        .expect("mapper action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String("focus_domain=mapper.uxrom.prg_bank_switch".to_string())));
    assert!(mapper_action["evidence"]
        .as_array()
        .expect("mapper action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.15.result".to_string()
            )));
    let mapper_ram_action = find_observer_action(observer_actions, "mapper2_prg_ram_fault");
    assert_eq!(
        mapper_ram_action["primary_artifact"],
        Value::String("mapper2_prg_ram_fault/comparison.json".to_string())
    );
    assert!(mapper_ram_action["evidence"]
        .as_array()
        .expect("mapper PRG RAM action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=mapper.uxrom.prg_ram".to_string())));
    assert!(mapper_ram_action["evidence"]
        .as_array()
        .expect("mapper PRG RAM action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.16.result".to_string()
            )));
    let ppu_nmi_action = find_observer_action(observer_actions, "ppu_nmi_timeout_fault");
    assert_eq!(
        ppu_nmi_action["primary_artifact"],
        Value::String("ppu_nmi_timeout_fault/comparison.json".to_string())
    );
    assert!(ppu_nmi_action["evidence"]
        .as_array()
        .expect("PPU NMI action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("health=timed_out".to_string())));
    assert!(ppu_nmi_action["evidence"]
        .as_array()
        .expect("PPU NMI action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=ppu.nmi".to_string())));
    assert!(ppu_nmi_action["evidence"]
        .as_array()
        .expect("PPU NMI action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=runtime.completed,cartridge.status.pass,cartridge.test.10.result,ppu.nmi_count,ppu.vblank_timing.nmi_window"
                    .to_string()
            )));
    let dma_action = find_observer_action(observer_actions, "dma_oam_transfer_fault");
    assert_eq!(
        dma_action["primary_artifact"],
        Value::String("dma_oam_transfer_fault/comparison.json".to_string())
    );
    assert!(dma_action["evidence"]
        .as_array()
        .expect("DMA action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=dma.oam_transfer".to_string())));
    assert!(dma_action["evidence"]
        .as_array()
        .expect("DMA action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("failed_probe_ids=oam.dma_checksum".to_string())));
    let apu_action = find_observer_action(observer_actions, "apu_status_fault");
    assert_eq!(
        apu_action["primary_artifact"],
        Value::String("apu_status_fault/comparison.json".to_string())
    );
    assert!(apu_action["evidence"]
        .as_array()
        .expect("APU action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=apu.status".to_string())));
    assert!(apu_action["evidence"]
        .as_array()
        .expect("APU action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.6.result".to_string()
            )));
    let cpu_action = find_observer_action(observer_actions, "cpu_zero_page_wrap_fault");
    assert_eq!(
        cpu_action["primary_artifact"],
        Value::String("cpu_zero_page_wrap_fault/comparison.json".to_string())
    );
    assert!(cpu_action["evidence"]
        .as_array()
        .expect("CPU action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String("focus_domain=cpu.addressing.zero_page_x_wrap".to_string())));
    assert!(cpu_action["evidence"]
        .as_array()
        .expect("CPU action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.12.result".to_string()
            )));
    let jmp_action = find_observer_action(observer_actions, "cpu_indirect_jmp_fault");
    assert_eq!(
        jmp_action["primary_artifact"],
        Value::String("cpu_indirect_jmp_fault/comparison.json".to_string())
    );
    assert!(jmp_action["evidence"]
        .as_array()
        .expect("indirect JMP action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String("focus_domain=cpu.control_flow.indirect_jmp_page_wrap".to_string())));
    assert!(jmp_action["evidence"]
        .as_array()
        .expect("indirect JMP action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.13.result".to_string()
            )));
    let addressing_action = find_observer_action(observer_actions, "cpu_addressing_matrix_fault");
    assert_eq!(
        addressing_action["primary_artifact"],
        Value::String("cpu_addressing_matrix_fault/comparison.json".to_string())
    );
    assert!(addressing_action["evidence"]
        .as_array()
        .expect("CPU addressing action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String("focus_domain=cpu.addressing.page_cross_load".to_string())));
    assert!(addressing_action["evidence"]
        .as_array()
        .expect("CPU addressing action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.22.result".to_string()
            )));
    let rmw_action = find_observer_action(observer_actions, "cpu_rmw_matrix_fault");
    assert_eq!(
        rmw_action["primary_artifact"],
        Value::String("cpu_rmw_matrix_fault/comparison.json".to_string())
    );
    assert!(rmw_action["evidence"]
        .as_array()
        .expect("CPU RMW action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=cpu.rmw.asl".to_string())));
    assert!(rmw_action["evidence"]
        .as_array()
        .expect("CPU RMW action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.37.result".to_string()
            )));
    let rmw_addressing_action =
        find_observer_action(observer_actions, "cpu_rmw_addressing_matrix_fault");
    assert_eq!(
        rmw_addressing_action["primary_artifact"],
        Value::String("cpu_rmw_addressing_matrix_fault/comparison.json".to_string())
    );
    assert!(rmw_addressing_action["evidence"]
        .as_array()
        .expect("CPU RMW addressing action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=cpu.rmw.absolute_asl".to_string())));
    assert!(rmw_addressing_action["evidence"]
        .as_array()
        .expect("CPU RMW addressing action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.38.result".to_string()
            )));
    let input_port_action = find_observer_action(observer_actions, "input_port_matrix_fault");
    assert_eq!(
        input_port_action["primary_artifact"],
        Value::String("input_port_matrix_fault/comparison.json".to_string())
    );
    assert!(input_port_action["evidence"]
        .as_array()
        .expect("input-port action evidence should be an array")
        .iter()
        .any(|entry| entry == &Value::String("focus_domain=joypad.input_port_matrix".to_string())));
    assert!(input_port_action["evidence"]
        .as_array()
        .expect("input-port action evidence should be an array")
        .iter()
        .any(|entry| entry
            == &Value::String(
                "failed_probe_ids=cartridge.status.pass,cartridge.test.23.result".to_string()
            )));

    let observations = observer["observations"]
        .as_array()
        .expect("observer observations should be an array");
    assert_eq!(observations.len(), 34);
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
    let input_matrix_observation =
        find_observer_observation(observations, "input_mask_matrix_pass");
    assert_eq!(
        input_matrix_observation["role"],
        Value::String("expected_pass_fixture".to_string())
    );
    assert_eq!(
        input_matrix_observation["outcome"],
        Value::String("matches_baseline".to_string())
    );
    assert_eq!(
        input_matrix_observation["health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        input_matrix_observation["next_artifact"],
        Value::String("input_mask_matrix_pass/triage.json".to_string())
    );
    assert_replay_args_contains(&input_matrix_observation["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_matrix_observation["replay_args"], "0xAA");
    assert_replay_args_contains(&input_matrix_observation["replay_args"], "--expect-joypad2");
    assert_replay_args_contains(&input_matrix_observation["replay_args"], "0x55");
    assert_replay_args_contains(
        &input_matrix_observation["replay_args"],
        "target/diagnostics/replay/input_mask_matrix_pass",
    );
    let input_all_released_observation =
        find_observer_observation(observations, "input_mask_all_released_pass");
    assert_eq!(
        input_all_released_observation["role"],
        Value::String("expected_pass_fixture".to_string())
    );
    assert_eq!(
        input_all_released_observation["outcome"],
        Value::String("matches_baseline".to_string())
    );
    assert_eq!(
        input_all_released_observation["health"],
        Value::String("healthy".to_string())
    );
    assert_replay_args_contains(&input_all_released_observation["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_all_released_observation["replay_args"], "0x00");
    assert_replay_args_contains(
        &input_all_released_observation["replay_args"],
        "target/diagnostics/replay/input_mask_all_released_pass",
    );
    let input_all_pressed_observation =
        find_observer_observation(observations, "input_mask_all_pressed_pass");
    assert_eq!(
        input_all_pressed_observation["role"],
        Value::String("expected_pass_fixture".to_string())
    );
    assert_eq!(
        input_all_pressed_observation["outcome"],
        Value::String("matches_baseline".to_string())
    );
    assert_eq!(
        input_all_pressed_observation["health"],
        Value::String("healthy".to_string())
    );
    assert_replay_args_contains(&input_all_pressed_observation["replay_args"], "--joypad2");
    assert_replay_args_contains(&input_all_pressed_observation["replay_args"], "0xFF");
    assert_replay_args_contains(
        &input_all_pressed_observation["replay_args"],
        "target/diagnostics/replay/input_mask_all_pressed_pass",
    );
    let input_joypad1_pressed_observation =
        find_observer_observation(observations, "input_mask_joypad1_pressed_pass");
    assert_eq!(
        input_joypad1_pressed_observation["role"],
        Value::String("expected_pass_fixture".to_string())
    );
    assert_eq!(
        input_joypad1_pressed_observation["outcome"],
        Value::String("matches_baseline".to_string())
    );
    assert_eq!(
        input_joypad1_pressed_observation["health"],
        Value::String("healthy".to_string())
    );
    assert_replay_args_contains(
        &input_joypad1_pressed_observation["replay_args"],
        "--joypad1",
    );
    assert_replay_args_contains(&input_joypad1_pressed_observation["replay_args"], "0xFF");
    assert_replay_args_contains(
        &input_joypad1_pressed_observation["replay_args"],
        "--joypad2",
    );
    assert_replay_args_contains(&input_joypad1_pressed_observation["replay_args"], "0x00");
    assert_replay_args_contains(
        &input_joypad1_pressed_observation["replay_args"],
        "target/diagnostics/replay/input_mask_joypad1_pressed_pass",
    );
    let input_joypad2_pressed_observation =
        find_observer_observation(observations, "input_mask_joypad2_pressed_pass");
    assert_eq!(
        input_joypad2_pressed_observation["role"],
        Value::String("expected_pass_fixture".to_string())
    );
    assert_eq!(
        input_joypad2_pressed_observation["outcome"],
        Value::String("matches_baseline".to_string())
    );
    assert_eq!(
        input_joypad2_pressed_observation["health"],
        Value::String("healthy".to_string())
    );
    assert_replay_args_contains(
        &input_joypad2_pressed_observation["replay_args"],
        "--joypad1",
    );
    assert_replay_args_contains(&input_joypad2_pressed_observation["replay_args"], "0x00");
    assert_replay_args_contains(
        &input_joypad2_pressed_observation["replay_args"],
        "--joypad2",
    );
    assert_replay_args_contains(&input_joypad2_pressed_observation["replay_args"], "0xFF");
    assert_replay_args_contains(
        &input_joypad2_pressed_observation["replay_args"],
        "target/diagnostics/replay/input_mask_joypad2_pressed_pass",
    );
    let input_sparse_bits_observation =
        find_observer_observation(observations, "input_mask_sparse_bits_pass");
    assert_eq!(
        input_sparse_bits_observation["role"],
        Value::String("expected_pass_fixture".to_string())
    );
    assert_eq!(
        input_sparse_bits_observation["outcome"],
        Value::String("matches_baseline".to_string())
    );
    assert_eq!(
        input_sparse_bits_observation["health"],
        Value::String("healthy".to_string())
    );
    assert_replay_args_contains(&input_sparse_bits_observation["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_sparse_bits_observation["replay_args"], "0x81");
    assert_replay_args_contains(&input_sparse_bits_observation["replay_args"], "--joypad2");
    assert_replay_args_contains(&input_sparse_bits_observation["replay_args"], "0x18");
    assert_replay_args_contains(
        &input_sparse_bits_observation["replay_args"],
        "target/diagnostics/replay/input_mask_sparse_bits_pass",
    );
    let input_nibble_split_observation =
        find_observer_observation(observations, "input_mask_nibble_split_pass");
    assert_eq!(
        input_nibble_split_observation["role"],
        Value::String("expected_pass_fixture".to_string())
    );
    assert_eq!(
        input_nibble_split_observation["outcome"],
        Value::String("matches_baseline".to_string())
    );
    assert_eq!(
        input_nibble_split_observation["health"],
        Value::String("healthy".to_string())
    );
    assert_replay_args_contains(&input_nibble_split_observation["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_nibble_split_observation["replay_args"], "0x0F");
    assert_replay_args_contains(&input_nibble_split_observation["replay_args"], "--joypad2");
    assert_replay_args_contains(&input_nibble_split_observation["replay_args"], "0xF0");
    assert_replay_args_contains(
        &input_nibble_split_observation["replay_args"],
        "target/diagnostics/replay/input_mask_nibble_split_pass",
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
    assert!(
        timeout_observation["comparison_difference_count"]
            .as_u64()
            .expect("timeout observation difference count should be numeric")
            > 0
    );
    assert_eq!(
        timeout_observation["next_artifact"],
        Value::String("timeout_cycle_limit/comparison.json".to_string())
    );
    let dma_observation = find_observer_observation(observations, "dma_oam_transfer_fault");
    assert_eq!(
        dma_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        dma_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        dma_observation["health"],
        Value::String("host_validation_failed".to_string())
    );
    assert_eq!(
        dma_observation["focus_domain"],
        Value::String("dma.oam_transfer".to_string())
    );
    assert_eq!(
        dma_observation["next_artifact"],
        Value::String("dma_oam_transfer_fault/comparison.json".to_string())
    );
    let apu_observation = find_observer_observation(observations, "apu_status_fault");
    assert_eq!(
        apu_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        apu_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        apu_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        apu_observation["focus_domain"],
        Value::String("apu.status".to_string())
    );
    assert_eq!(
        apu_observation["next_artifact"],
        Value::String("apu_status_fault/comparison.json".to_string())
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
    let ppu_mirroring_observation =
        find_observer_observation(observations, "ppu_nametable_mirroring_fault");
    assert_eq!(
        ppu_mirroring_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        ppu_mirroring_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        ppu_mirroring_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        ppu_mirroring_observation["focus_domain"],
        Value::String("ppu.nametables.horizontal_mirroring".to_string())
    );
    assert_eq!(
        ppu_mirroring_observation["next_artifact"],
        Value::String("ppu_nametable_mirroring_fault/comparison.json".to_string())
    );
    let joypad_reset_observation =
        find_observer_observation(observations, "joypad_strobe_reset_fault");
    assert_eq!(
        joypad_reset_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        joypad_reset_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        joypad_reset_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        joypad_reset_observation["focus_domain"],
        Value::String("joypad.strobe_reset".to_string())
    );
    assert_eq!(
        joypad_reset_observation["next_artifact"],
        Value::String("joypad_strobe_reset_fault/comparison.json".to_string())
    );
    let joypad_hold_observation =
        find_observer_observation(observations, "joypad_strobe_high_hold_fault");
    assert_eq!(
        joypad_hold_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        joypad_hold_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        joypad_hold_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        joypad_hold_observation["focus_domain"],
        Value::String("joypad.strobe_high_hold".to_string())
    );
    assert_eq!(
        joypad_hold_observation["next_artifact"],
        Value::String("joypad_strobe_high_hold_fault/comparison.json".to_string())
    );
    assert_replay_args_contains(&joypad_hold_observation["replay_args"], "--fault-injection");
    assert_replay_args_contains(
        &joypad_hold_observation["replay_args"],
        "joypad_strobe_high_hold",
    );
    let ppu_increment_32_observation =
        find_observer_observation(observations, "ppu_vram_increment_32_fault");
    assert_eq!(
        ppu_increment_32_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        ppu_increment_32_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        ppu_increment_32_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        ppu_increment_32_observation["focus_domain"],
        Value::String("ppu.registers.ppudata_increment_32".to_string())
    );
    assert_eq!(
        ppu_increment_32_observation["next_artifact"],
        Value::String("ppu_vram_increment_32_fault/comparison.json".to_string())
    );
    let ppu_status_latch_observation =
        find_observer_observation(observations, "ppu_status_latch_reset_fault");
    assert_eq!(
        ppu_status_latch_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        ppu_status_latch_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        ppu_status_latch_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        ppu_status_latch_observation["focus_domain"],
        Value::String("ppu.registers.status_latch_reset".to_string())
    );
    assert_eq!(
        ppu_status_latch_observation["next_artifact"],
        Value::String("ppu_status_latch_reset_fault/comparison.json".to_string())
    );
    let mapper_observation = find_observer_observation(observations, "mapper2_bank_switch_fault");
    assert_eq!(
        mapper_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        mapper_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        mapper_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        mapper_observation["focus_domain"],
        Value::String("mapper.uxrom.prg_bank_switch".to_string())
    );
    assert_eq!(
        mapper_observation["next_artifact"],
        Value::String("mapper2_bank_switch_fault/comparison.json".to_string())
    );
    let mapper_ram_observation = find_observer_observation(observations, "mapper2_prg_ram_fault");
    assert_eq!(
        mapper_ram_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        mapper_ram_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        mapper_ram_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        mapper_ram_observation["focus_domain"],
        Value::String("mapper.uxrom.prg_ram".to_string())
    );
    assert_eq!(
        mapper_ram_observation["next_artifact"],
        Value::String("mapper2_prg_ram_fault/comparison.json".to_string())
    );
    let ppu_nmi_observation = find_observer_observation(observations, "ppu_nmi_timeout_fault");
    assert_eq!(
        ppu_nmi_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        ppu_nmi_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        ppu_nmi_observation["health"],
        Value::String("timed_out".to_string())
    );
    assert_eq!(
        ppu_nmi_observation["focus_domain"],
        Value::String("ppu.nmi".to_string())
    );
    assert_eq!(
        ppu_nmi_observation["next_artifact"],
        Value::String("ppu_nmi_timeout_fault/comparison.json".to_string())
    );
    let cpu_observation = find_observer_observation(observations, "cpu_zero_page_wrap_fault");
    assert_eq!(
        cpu_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        cpu_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        cpu_observation["focus_domain"],
        Value::String("cpu.addressing.zero_page_x_wrap".to_string())
    );
    assert_eq!(
        cpu_observation["next_artifact"],
        Value::String("cpu_zero_page_wrap_fault/comparison.json".to_string())
    );
    let jmp_observation = find_observer_observation(observations, "cpu_indirect_jmp_fault");
    assert_eq!(
        jmp_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        jmp_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        jmp_observation["focus_domain"],
        Value::String("cpu.control_flow.indirect_jmp_page_wrap".to_string())
    );
    assert_eq!(
        jmp_observation["next_artifact"],
        Value::String("cpu_indirect_jmp_fault/comparison.json".to_string())
    );
    let addressing_observation =
        find_observer_observation(observations, "cpu_addressing_matrix_fault");
    assert_eq!(
        addressing_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        addressing_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        addressing_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        addressing_observation["focus_domain"],
        Value::String("cpu.addressing.page_cross_load".to_string())
    );
    assert_eq!(
        addressing_observation["next_artifact"],
        Value::String("cpu_addressing_matrix_fault/comparison.json".to_string())
    );
    let rmw_observation = find_observer_observation(observations, "cpu_rmw_matrix_fault");
    assert_eq!(
        rmw_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        rmw_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        rmw_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        rmw_observation["focus_domain"],
        Value::String("cpu.rmw.asl".to_string())
    );
    assert_eq!(
        rmw_observation["next_artifact"],
        Value::String("cpu_rmw_matrix_fault/comparison.json".to_string())
    );
    let rmw_addressing_observation =
        find_observer_observation(observations, "cpu_rmw_addressing_matrix_fault");
    assert_eq!(
        rmw_addressing_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        rmw_addressing_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        rmw_addressing_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        rmw_addressing_observation["focus_domain"],
        Value::String("cpu.rmw.absolute_asl".to_string())
    );
    assert_eq!(
        rmw_addressing_observation["next_artifact"],
        Value::String("cpu_rmw_addressing_matrix_fault/comparison.json".to_string())
    );
    let input_port_observation = find_observer_observation(observations, "input_port_matrix_fault");
    assert_eq!(
        input_port_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        input_port_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        input_port_observation["health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(
        input_port_observation["focus_domain"],
        Value::String("joypad.input_port_matrix".to_string())
    );
    assert_eq!(
        input_port_observation["next_artifact"],
        Value::String("input_port_matrix_fault/comparison.json".to_string())
    );
    let ppu_priority_observation =
        find_observer_observation(observations, "ppu_sprite_priority_fault");
    assert_eq!(
        ppu_priority_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        ppu_priority_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        ppu_priority_observation["health"],
        Value::String("host_validation_failed".to_string())
    );
    assert_eq!(
        ppu_priority_observation["focus_domain"],
        Value::String("ppu.sprite_priority".to_string())
    );
    assert_eq!(
        ppu_priority_observation["next_artifact"],
        Value::String("ppu_sprite_priority_fault/comparison.json".to_string())
    );
    let ppu_scroll_observation = find_observer_observation(observations, "ppu_scroll_seam_fault");
    assert_eq!(
        ppu_scroll_observation["role"],
        Value::String("expected_failure_fixture".to_string())
    );
    assert_eq!(
        ppu_scroll_observation["outcome"],
        Value::String("expected_baseline_divergence".to_string())
    );
    assert_eq!(
        ppu_scroll_observation["health"],
        Value::String("host_validation_failed".to_string())
    );
    assert_eq!(
        ppu_scroll_observation["focus_domain"],
        Value::String("ppu.scroll_seam".to_string())
    );
    assert_eq!(
        ppu_scroll_observation["next_artifact"],
        Value::String("ppu_scroll_seam_fault/comparison.json".to_string())
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
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | dma_oam_transfer_fault | dma_oam_transfer_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | apu_status_fault | apu_status_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_read_buffer_fault | ppu_read_buffer_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_nametable_mirroring_fault | ppu_nametable_mirroring_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_sprite_zero_hit_fault | ppu_sprite_zero_hit_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_sprite_overflow_fault | ppu_sprite_overflow_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_sprite_priority_fault | ppu_sprite_priority_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_scroll_seam_fault | ppu_scroll_seam_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | joypad_strobe_reset_fault | joypad_strobe_reset_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | joypad_strobe_high_hold_fault | joypad_strobe_high_hold_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_vram_increment_32_fault | ppu_vram_increment_32_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_status_latch_reset_fault | ppu_status_latch_reset_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | mapper2_bank_switch_fault | mapper2_bank_switch_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | mapper2_prg_ram_fault | mapper2_prg_ram_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | ppu_nmi_timeout_fault | ppu_nmi_timeout_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | cpu_indirect_jmp_fault | cpu_indirect_jmp_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | cpu_addressing_matrix_fault | cpu_addressing_matrix_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | cpu_rmw_matrix_fault | cpu_rmw_matrix_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | cpu_rmw_addressing_matrix_fault | cpu_rmw_addressing_matrix_fault/comparison.json |"));
    assert!(observer_report.contains("| known_divergence | inspect_known_divergence | input_port_matrix_fault | input_port_matrix_fault/comparison.json |"));
    assert!(observer_report.contains("top_difference_path=dma.oam_dma_observed"));
    assert!(observer_report.contains("## Observations"));
    assert!(observer_report
        .contains("| pass | baseline | matches_baseline | healthy | - | 0 | pass/triage.json |"));
    assert!(observer_report.contains(
        "| input_mask_matrix_pass | expected_pass_fixture | matches_baseline | healthy | - |"
    ));
    assert!(observer_report.contains(
        "| input_mask_all_released_pass | expected_pass_fixture | matches_baseline | healthy | - |"
    ));
    assert!(observer_report.contains(
        "| input_mask_all_pressed_pass | expected_pass_fixture | matches_baseline | healthy | - |"
    ));
    assert!(observer_report.contains(
        "| input_mask_joypad1_pressed_pass | expected_pass_fixture | matches_baseline | healthy | - |"
    ));
    assert!(observer_report.contains(
        "| input_mask_joypad2_pressed_pass | expected_pass_fixture | matches_baseline | healthy | - |"
    ));
    assert!(observer_report.contains(
        "| input_mask_sparse_bits_pass | expected_pass_fixture | matches_baseline | healthy | - |"
    ));
    assert!(observer_report.contains(
        "| input_mask_nibble_split_pass | expected_pass_fixture | matches_baseline | healthy | - |"
    ));
    assert!(observer_report.contains("| dma_phase_matrix_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | dma.oam_phase_matrix |"));
    assert!(observer_report.contains("| ppu_sprite_zero_hit_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | ppu.sprite_zero_hit |"));
    assert!(observer_report.contains("| ppu_sprite_overflow_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | ppu.sprite_overflow |"));
    assert!(observer_report.contains("| ppu_sprite_priority_fault | expected_failure_fixture | expected_baseline_divergence | host_validation_failed | ppu.sprite_priority |"));
    assert!(observer_report.contains("| ppu_scroll_seam_fault | expected_failure_fixture | expected_baseline_divergence | host_validation_failed | ppu.scroll_seam |"));
    assert!(observer_report.contains("| timeout_cycle_limit | expected_failure_fixture | expected_baseline_divergence | timed_out | emulator.progress_or_infinite_loop |"));
    assert!(observer_report.contains("| dma_oam_transfer_fault | expected_failure_fixture | expected_baseline_divergence | host_validation_failed | dma.oam_transfer |"));
    assert!(observer_report.contains("| apu_status_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | apu.status |"));
    assert!(observer_report.contains("| ppu_read_buffer_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | ppu.registers.ppudata_buffer |"));
    assert!(observer_report.contains("| ppu_nametable_mirroring_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | ppu.nametables.horizontal_mirroring |"));
    assert!(observer_report.contains("| joypad_strobe_reset_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | joypad.strobe_reset |"));
    assert!(observer_report.contains("| joypad_strobe_high_hold_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | joypad.strobe_high_hold |"));
    assert!(observer_report.contains("| ppu_vram_increment_32_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | ppu.registers.ppudata_increment_32 |"));
    assert!(observer_report.contains("| ppu_status_latch_reset_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | ppu.registers.status_latch_reset |"));
    assert!(observer_report.contains("| mapper2_bank_switch_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | mapper.uxrom.prg_bank_switch |"));
    assert!(observer_report.contains("| mapper2_prg_ram_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | mapper.uxrom.prg_ram |"));
    assert!(observer_report.contains("| ppu_nmi_timeout_fault | expected_failure_fixture | expected_baseline_divergence | timed_out | ppu.nmi |"));
    assert!(observer_report.contains("| cpu_ram_mirroring_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | bus.cpu_ram_mirroring |"));
    assert!(observer_report.contains("| cpu_indirect_jmp_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | cpu.control_flow.indirect_jmp_page_wrap |"));
    assert!(observer_report.contains("| cpu_addressing_matrix_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | cpu.addressing.page_cross_load |"));
    assert!(observer_report.contains("| cpu_rmw_matrix_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | cpu.rmw.asl |"));
    assert!(observer_report.contains("| cpu_rmw_addressing_matrix_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | cpu.rmw.absolute_asl |"));
    assert!(observer_report.contains("| input_port_matrix_fault | expected_failure_fixture | expected_baseline_divergence | cartridge_assertion_failed | joypad.input_port_matrix |"));
    assert!(observer_report.contains("## Replay Commands"));
    assert!(observer_report.contains("target/diagnostics/replay/joypad_strobe_high_hold_fault"));
    assert!(observer_report.contains("## Artifact Hints"));
    assert!(observer_report.contains("scenario-suite.json"));

    let suite_report = fs::read_to_string(suite_dir.join("scenario-suite.md"))
        .expect("scenario suite report should be readable");
    assert!(suite_report.contains("# Diagnostic Scenario Suite"));
    assert!(suite_report.contains("## Suite Analysis"));
    assert!(suite_report.contains("| Status | passed |"));
    assert!(suite_report.contains("| Baseline divergences | 26 |"));
    assert!(suite_report.contains("## Attention Queue"));
    assert!(suite_report.contains("| known_divergence | timeout_cycle_limit | scenario_diverges_from_pass_baseline | timed_out | emulator.progress_or_infinite_loop |"));
    assert!(suite_report.contains("| known_divergence | dma_oam_transfer_fault | scenario_diverges_from_pass_baseline | host_validation_failed | dma.oam_transfer |"));
    assert!(suite_report.contains("| known_divergence | dma_phase_matrix_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | dma.oam_phase_matrix |"));
    assert!(suite_report.contains("| known_divergence | apu_status_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | apu.status |"));
    assert!(suite_report.contains("| known_divergence | cpu_ram_mirroring_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | bus.cpu_ram_mirroring |"));
    assert!(suite_report.contains("| known_divergence | ppu_read_buffer_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | ppu.registers.ppudata_buffer |"));
    assert!(suite_report.contains("| known_divergence | ppu_nametable_mirroring_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | ppu.nametables.horizontal_mirroring |"));
    assert!(suite_report.contains("| known_divergence | ppu_sprite_zero_hit_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | ppu.sprite_zero_hit |"));
    assert!(suite_report.contains("| known_divergence | ppu_sprite_overflow_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | ppu.sprite_overflow |"));
    assert!(suite_report.contains("| known_divergence | ppu_sprite_priority_fault | scenario_diverges_from_pass_baseline | host_validation_failed | ppu.sprite_priority | 9 | ppu_sprite_priority_fault/comparison.json |"));
    assert!(suite_report.contains("| known_divergence | ppu_scroll_seam_fault | scenario_diverges_from_pass_baseline | host_validation_failed | ppu.scroll_seam |"));
    assert!(suite_report.contains("| known_divergence | joypad_strobe_reset_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | joypad.strobe_reset |"));
    assert!(suite_report.contains("| known_divergence | joypad_strobe_high_hold_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | joypad.strobe_high_hold |"));
    assert!(suite_report.contains("| known_divergence | ppu_vram_increment_32_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | ppu.registers.ppudata_increment_32 |"));
    assert!(suite_report.contains("| known_divergence | ppu_status_latch_reset_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | ppu.registers.status_latch_reset |"));
    assert!(suite_report.contains("| known_divergence | mapper2_bank_switch_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | mapper.uxrom.prg_bank_switch |"));
    assert!(suite_report.contains("| known_divergence | mapper2_prg_ram_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | mapper.uxrom.prg_ram |"));
    assert!(suite_report.contains("| known_divergence | ppu_nmi_timeout_fault | scenario_diverges_from_pass_baseline | timed_out | ppu.nmi |"));
    assert!(suite_report.contains("| known_divergence | cpu_zero_page_wrap_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | cpu.addressing.zero_page_x_wrap |"));
    assert!(suite_report.contains("| known_divergence | cpu_indirect_jmp_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | cpu.control_flow.indirect_jmp_page_wrap |"));
    assert!(suite_report.contains("| known_divergence | cpu_addressing_matrix_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | cpu.addressing.page_cross_load |"));
    assert!(suite_report.contains("| known_divergence | cpu_rmw_matrix_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | cpu.rmw.asl |"));
    assert!(suite_report.contains("| known_divergence | cpu_rmw_addressing_matrix_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | cpu.rmw.absolute_asl |"));
    assert!(suite_report.contains("| known_divergence | input_port_matrix_fault | scenario_diverges_from_pass_baseline | cartridge_assertion_failed | joypad.input_port_matrix |"));
    assert!(suite_report.contains("| Scenario | Expected pass | Actual pass |"));
    assert!(
        suite_report.contains("| input_mask_matrix_pass | true | true | true | healthy | 38 | - |")
    );
    assert!(suite_report
        .contains("| input_mask_all_released_pass | true | true | true | healthy | 38 | - |"));
    assert!(suite_report
        .contains("| input_mask_all_pressed_pass | true | true | true | healthy | 38 | - |"));
    assert!(suite_report
        .contains("| input_mask_joypad1_pressed_pass | true | true | true | healthy | 38 | - |"));
    assert!(suite_report
        .contains("| input_mask_joypad2_pressed_pass | true | true | true | healthy | 38 | - |"));
    assert!(suite_report
        .contains("| input_mask_sparse_bits_pass | true | true | true | healthy | 38 | - |"));
    assert!(suite_report
        .contains("| input_mask_nibble_split_pass | true | true | true | healthy | 38 | - |"));
    assert!(suite_report.contains("| joypad1_mismatch | false | false | true |"));
    assert!(suite_report.contains("| dma_oam_transfer_fault | false | false | true | host_validation_failed | 5 | dma.oam_transfer |"));
    assert!(suite_report.contains(
        "| apu_status_fault | false | false | true | cartridge_assertion_failed | 6 | apu.status |"
    ));
    assert!(suite_report.contains("| cpu_zero_page_wrap_fault | false | false | true | cartridge_assertion_failed | 12 | cpu.addressing.zero_page_x_wrap |"));
    assert!(suite_report.contains("| cpu_indirect_jmp_fault | false | false | true | cartridge_assertion_failed | 13 | cpu.control_flow.indirect_jmp_page_wrap |"));
    assert!(suite_report.contains("| ppu_read_buffer_fault | false | false | true | cartridge_assertion_failed | 14 | ppu.registers.ppudata_buffer |"));
    assert!(suite_report.contains("| ppu_nametable_mirroring_fault | false | false | true | cartridge_assertion_failed | 17 | ppu.nametables.horizontal_mirroring |"));
    assert!(suite_report.contains("| joypad_strobe_reset_fault | false | false | true | cartridge_assertion_failed | 18 | joypad.strobe_reset |"));
    assert!(suite_report.contains("| joypad_strobe_high_hold_fault | false | false | true | cartridge_assertion_failed | 21 | joypad.strobe_high_hold |"));
    assert!(suite_report.contains("| cpu_addressing_matrix_fault | false | false | true | cartridge_assertion_failed | 22 | cpu.addressing.page_cross_load |"));
    assert!(suite_report.contains("| cpu_rmw_matrix_fault | false | false | true | cartridge_assertion_failed | 37 | cpu.rmw.asl |"));
    assert!(suite_report.contains("| cpu_rmw_addressing_matrix_fault | false | false | true | cartridge_assertion_failed | 38 | cpu.rmw.absolute_asl |"));
    assert!(suite_report.contains("| input_port_matrix_fault | false | false | true | cartridge_assertion_failed | 23 | joypad.input_port_matrix |"));
    assert!(suite_report.contains("| ppu_sprite_priority_fault | false | false | true | host_validation_failed | 27 | ppu.sprite_priority |"));
    assert!(suite_report.contains("| ppu_scroll_seam_fault | false | false | true | host_validation_failed | 28 | ppu.scroll_seam |"));
    assert!(suite_report.contains("| ppu_vram_increment_32_fault | false | false | true | cartridge_assertion_failed | 19 | ppu.registers.ppudata_increment_32 |"));
    assert!(suite_report.contains("| ppu_status_latch_reset_fault | false | false | true | cartridge_assertion_failed | 20 | ppu.registers.status_latch_reset |"));
    assert!(suite_report.contains("| mapper2_bank_switch_fault | false | false | true | cartridge_assertion_failed | 15 | mapper.uxrom.prg_bank_switch |"));
    assert!(suite_report.contains("| mapper2_prg_ram_fault | false | false | true | cartridge_assertion_failed | 16 | mapper.uxrom.prg_ram |"));
    assert!(suite_report
        .contains("| ppu_nmi_timeout_fault | false | false | true | timed_out | 10 | ppu.nmi |"));
    assert!(suite_report.contains("cartridge.test.7.result"));
    assert!(suite_report.contains("| timeout_cycle_limit | false | false | true | timed_out | 0 | emulator.progress_or_infinite_loop |"));
    assert!(suite_report.contains("runtime.completed"));
    assert!(suite_report.contains("## Contract Matrix"));
    assert!(suite_report.contains("| joypad1_mismatch | true | true | true | true | true |"));
    assert!(suite_report.contains("| dma_oam_transfer_fault | true | true | true | true | true |"));
    assert!(suite_report.contains("| apu_status_fault | true | true | true | true | true |"));
    assert!(
        suite_report.contains("| cpu_zero_page_wrap_fault | true | true | true | true | true |")
    );
    assert!(suite_report.contains("| cpu_indirect_jmp_fault | true | true | true | true | true |"));
    assert!(
        suite_report.contains("| cpu_addressing_matrix_fault | true | true | true | true | true |")
    );
    assert!(suite_report.contains("| cpu_rmw_matrix_fault | true | true | true | true | true |"));
    assert!(suite_report
        .contains("| cpu_rmw_addressing_matrix_fault | true | true | true | true | true |"));
    assert!(suite_report.contains("| input_port_matrix_fault | true | true | true | true | true |"));
    assert!(suite_report.contains("| ppu_read_buffer_fault | true | true | true | true | true |"));
    assert!(suite_report
        .contains("| ppu_nametable_mirroring_fault | true | true | true | true | true |"));
    assert!(
        suite_report.contains("| joypad_strobe_reset_fault | true | true | true | true | true |")
    );
    assert!(suite_report
        .contains("| joypad_strobe_high_hold_fault | true | true | true | true | true |"));
    assert!(
        suite_report.contains("| ppu_vram_increment_32_fault | true | true | true | true | true |")
    );
    assert!(suite_report
        .contains("| ppu_status_latch_reset_fault | true | true | true | true | true |"));
    assert!(
        suite_report.contains("| mapper2_bank_switch_fault | true | true | true | true | true |")
    );
    assert!(suite_report.contains("| mapper2_prg_ram_fault | true | true | true | true | true |"));
    assert!(suite_report.contains("| ppu_nmi_timeout_fault | true | true | true | true | true |"));
    assert!(suite_report.contains("| ppu_scroll_seam_fault | true | true | true | true | true |"));
    assert!(suite_report.contains("| timeout_cycle_limit | true | true | true | true | true |"));
    assert!(suite_report.contains("## AI Drilldown"));
    assert!(suite_report.contains("## Replay Commands"));
    assert!(suite_report.contains("target/diagnostics/replay/input_mask_matrix_pass"));
    assert!(suite_report.contains("target/diagnostics/replay/input_mask_all_released_pass"));
    assert!(suite_report.contains("target/diagnostics/replay/input_mask_all_pressed_pass"));
    assert!(suite_report.contains("target/diagnostics/replay/input_mask_joypad1_pressed_pass"));
    assert!(suite_report.contains("target/diagnostics/replay/input_mask_joypad2_pressed_pass"));
    assert!(suite_report.contains("target/diagnostics/replay/input_mask_sparse_bits_pass"));
    assert!(suite_report.contains("target/diagnostics/replay/input_mask_nibble_split_pass"));
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
    assert_eq!(pass["actual_focus_test_id"], Value::from(38));
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
    assert_eq!(pass["contract"]["expected_focus_test_id"], Value::from(38));
    assert_eq!(pass["contract"]["actual_focus_test_id"], Value::from(38));
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
    assert_replay_args_contains(&pass["replay_args"], "target/diagnostics/replay/pass");
    assert_replay_args_contains(&pass["replay_args"], "--bundle-dir");
    assert_replay_args_contains(&pass["replay_args"], "--no-stdout");
    assert_bundle_artifacts(&suite_dir.join("pass"), true, true);

    let input_matrix = find_scenario(scenarios, "input_mask_matrix_pass");
    assert_eq!(input_matrix["expected_passed"], Value::Bool(true));
    assert_eq!(input_matrix["actual_passed"], Value::Bool(true));
    assert_eq!(input_matrix["expectation_met"], Value::Bool(true));
    assert_eq!(
        input_matrix["actual_health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(input_matrix["actual_focus_test_id"], Value::from(38));
    assert_eq!(input_matrix["comparison"]["passed"], Value::Bool(true));
    assert_eq!(
        input_matrix["config"]["joypad1_mask_hex"],
        Value::String("0xAA".to_string())
    );
    assert_eq!(
        input_matrix["config"]["expected_joypad1_mask_hex"],
        Value::String("0xAA".to_string())
    );
    assert_eq!(
        input_matrix["config"]["joypad2_mask_hex"],
        Value::String("0x55".to_string())
    );
    assert_eq!(
        input_matrix["config"]["expected_joypad2_mask_hex"],
        Value::String("0x55".to_string())
    );
    assert_replay_args_contains(&input_matrix["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_matrix["replay_args"], "0xAA");
    assert_replay_args_contains(&input_matrix["replay_args"], "--expect-joypad2");
    assert_replay_args_contains(&input_matrix["replay_args"], "0x55");
    assert_bundle_artifacts_with_config(
        &suite_dir.join("input_mask_matrix_pass"),
        true,
        true,
        "0x55",
        None,
    );

    let input_all_released = find_scenario(scenarios, "input_mask_all_released_pass");
    assert_eq!(input_all_released["expected_passed"], Value::Bool(true));
    assert_eq!(input_all_released["actual_passed"], Value::Bool(true));
    assert_eq!(input_all_released["expectation_met"], Value::Bool(true));
    assert_eq!(
        input_all_released["actual_health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        input_all_released["config"]["joypad1_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        input_all_released["config"]["expected_joypad1_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        input_all_released["config"]["joypad2_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        input_all_released["config"]["expected_joypad2_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        input_all_released["comparison"]["passed"],
        Value::Bool(true)
    );
    assert_replay_args_contains(&input_all_released["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_all_released["replay_args"], "0x00");
    assert_bundle_artifacts_with_config(
        &suite_dir.join("input_mask_all_released_pass"),
        true,
        true,
        "0x00",
        None,
    );

    let input_all_pressed = find_scenario(scenarios, "input_mask_all_pressed_pass");
    assert_eq!(input_all_pressed["expected_passed"], Value::Bool(true));
    assert_eq!(input_all_pressed["actual_passed"], Value::Bool(true));
    assert_eq!(input_all_pressed["expectation_met"], Value::Bool(true));
    assert_eq!(
        input_all_pressed["actual_health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        input_all_pressed["config"]["joypad1_mask_hex"],
        Value::String("0xFF".to_string())
    );
    assert_eq!(
        input_all_pressed["config"]["expected_joypad1_mask_hex"],
        Value::String("0xFF".to_string())
    );
    assert_eq!(
        input_all_pressed["config"]["joypad2_mask_hex"],
        Value::String("0xFF".to_string())
    );
    assert_eq!(
        input_all_pressed["config"]["expected_joypad2_mask_hex"],
        Value::String("0xFF".to_string())
    );
    assert_eq!(input_all_pressed["comparison"]["passed"], Value::Bool(true));
    assert_replay_args_contains(&input_all_pressed["replay_args"], "--joypad2");
    assert_replay_args_contains(&input_all_pressed["replay_args"], "0xFF");
    assert_bundle_artifacts_with_config(
        &suite_dir.join("input_mask_all_pressed_pass"),
        true,
        true,
        "0xFF",
        None,
    );

    let input_joypad1_pressed = find_scenario(scenarios, "input_mask_joypad1_pressed_pass");
    assert_eq!(input_joypad1_pressed["expected_passed"], Value::Bool(true));
    assert_eq!(input_joypad1_pressed["actual_passed"], Value::Bool(true));
    assert_eq!(input_joypad1_pressed["expectation_met"], Value::Bool(true));
    assert_eq!(
        input_joypad1_pressed["actual_health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        input_joypad1_pressed["config"]["joypad1_mask_hex"],
        Value::String("0xFF".to_string())
    );
    assert_eq!(
        input_joypad1_pressed["config"]["expected_joypad1_mask_hex"],
        Value::String("0xFF".to_string())
    );
    assert_eq!(
        input_joypad1_pressed["config"]["joypad2_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        input_joypad1_pressed["config"]["expected_joypad2_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        input_joypad1_pressed["comparison"]["passed"],
        Value::Bool(true)
    );
    assert_replay_args_contains(&input_joypad1_pressed["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_joypad1_pressed["replay_args"], "0xFF");
    assert_replay_args_contains(&input_joypad1_pressed["replay_args"], "--expect-joypad2");
    assert_replay_args_contains(&input_joypad1_pressed["replay_args"], "0x00");
    assert_bundle_artifacts_with_config(
        &suite_dir.join("input_mask_joypad1_pressed_pass"),
        true,
        true,
        "0x00",
        None,
    );

    let input_joypad2_pressed = find_scenario(scenarios, "input_mask_joypad2_pressed_pass");
    assert_eq!(input_joypad2_pressed["expected_passed"], Value::Bool(true));
    assert_eq!(input_joypad2_pressed["actual_passed"], Value::Bool(true));
    assert_eq!(input_joypad2_pressed["expectation_met"], Value::Bool(true));
    assert_eq!(
        input_joypad2_pressed["actual_health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        input_joypad2_pressed["config"]["joypad1_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        input_joypad2_pressed["config"]["expected_joypad1_mask_hex"],
        Value::String("0x00".to_string())
    );
    assert_eq!(
        input_joypad2_pressed["config"]["joypad2_mask_hex"],
        Value::String("0xFF".to_string())
    );
    assert_eq!(
        input_joypad2_pressed["config"]["expected_joypad2_mask_hex"],
        Value::String("0xFF".to_string())
    );
    assert_eq!(
        input_joypad2_pressed["comparison"]["passed"],
        Value::Bool(true)
    );
    assert_replay_args_contains(&input_joypad2_pressed["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_joypad2_pressed["replay_args"], "0x00");
    assert_replay_args_contains(&input_joypad2_pressed["replay_args"], "--expect-joypad2");
    assert_replay_args_contains(&input_joypad2_pressed["replay_args"], "0xFF");
    assert_bundle_artifacts_with_config(
        &suite_dir.join("input_mask_joypad2_pressed_pass"),
        true,
        true,
        "0xFF",
        None,
    );

    let input_sparse_bits = find_scenario(scenarios, "input_mask_sparse_bits_pass");
    assert_eq!(input_sparse_bits["expected_passed"], Value::Bool(true));
    assert_eq!(input_sparse_bits["actual_passed"], Value::Bool(true));
    assert_eq!(input_sparse_bits["expectation_met"], Value::Bool(true));
    assert_eq!(
        input_sparse_bits["actual_health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        input_sparse_bits["config"]["joypad1_mask_hex"],
        Value::String("0x81".to_string())
    );
    assert_eq!(
        input_sparse_bits["config"]["expected_joypad1_mask_hex"],
        Value::String("0x81".to_string())
    );
    assert_eq!(
        input_sparse_bits["config"]["joypad2_mask_hex"],
        Value::String("0x18".to_string())
    );
    assert_eq!(
        input_sparse_bits["config"]["expected_joypad2_mask_hex"],
        Value::String("0x18".to_string())
    );
    assert_eq!(input_sparse_bits["comparison"]["passed"], Value::Bool(true));
    assert_replay_args_contains(&input_sparse_bits["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_sparse_bits["replay_args"], "0x81");
    assert_replay_args_contains(&input_sparse_bits["replay_args"], "--expect-joypad2");
    assert_replay_args_contains(&input_sparse_bits["replay_args"], "0x18");
    assert_bundle_artifacts_with_config(
        &suite_dir.join("input_mask_sparse_bits_pass"),
        true,
        true,
        "0x18",
        None,
    );

    let input_nibble_split = find_scenario(scenarios, "input_mask_nibble_split_pass");
    assert_eq!(input_nibble_split["expected_passed"], Value::Bool(true));
    assert_eq!(input_nibble_split["actual_passed"], Value::Bool(true));
    assert_eq!(input_nibble_split["expectation_met"], Value::Bool(true));
    assert_eq!(
        input_nibble_split["actual_health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        input_nibble_split["config"]["joypad1_mask_hex"],
        Value::String("0x0F".to_string())
    );
    assert_eq!(
        input_nibble_split["config"]["expected_joypad1_mask_hex"],
        Value::String("0x0F".to_string())
    );
    assert_eq!(
        input_nibble_split["config"]["joypad2_mask_hex"],
        Value::String("0xF0".to_string())
    );
    assert_eq!(
        input_nibble_split["config"]["expected_joypad2_mask_hex"],
        Value::String("0xF0".to_string())
    );
    assert_eq!(
        input_nibble_split["comparison"]["passed"],
        Value::Bool(true)
    );
    assert_replay_args_contains(&input_nibble_split["replay_args"], "--joypad1");
    assert_replay_args_contains(&input_nibble_split["replay_args"], "0x0F");
    assert_replay_args_contains(&input_nibble_split["replay_args"], "--expect-joypad2");
    assert_replay_args_contains(&input_nibble_split["replay_args"], "0xF0");
    assert_bundle_artifacts_with_config(
        &suite_dir.join("input_mask_nibble_split_pass"),
        true,
        true,
        "0xF0",
        None,
    );

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

    let dma = find_scenario(scenarios, "dma_oam_transfer_fault");
    assert_eq!(
        dma["actual_health"],
        Value::String("host_validation_failed".to_string())
    );
    assert_eq!(dma["actual_focus_test_id"], Value::from(5));
    assert_eq!(
        dma["actual_focus_domain"],
        Value::String("dma.oam_transfer".to_string())
    );
    assert_eq!(dma["expectation_met"], Value::Bool(true));
    assert_eq!(dma["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        dma["contract"]["expected_focus_domain"],
        Value::String("dma.oam_transfer".to_string())
    );
    assert!(dma["failed_probe_ids"]
        .as_array()
        .expect("DMA failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("oam.dma_checksum".to_string())));
    assert_eq!(dma["comparison"]["passed"], Value::Bool(false));
    assert!(
        dma["comparison"]["difference_count"]
            .as_u64()
            .expect("DMA comparison difference_count should be numeric")
            > 0
    );
    let dma_triage = read_json(&suite_dir.join("dma_oam_transfer_fault").join("triage.json"));
    assert_eq!(
        dma_triage["input"]["fault_injection"],
        Value::String("dma_oam_transfer".to_string())
    );
    assert_eq!(
        dma_triage["debug_focus"]["failure_kind"],
        Value::String("host_validation".to_string())
    );
    assert_eq!(
        dma_triage["failure"]["likely_domain"],
        Value::String("dma.oam_transfer".to_string())
    );
    assert!(dma_triage["probes"]["failed"]
        .as_array()
        .expect("DMA triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("oam.dma_checksum".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("dma_oam_transfer_fault"),
        true,
        false,
        "0x28",
        Some("dma_oam_transfer"),
    );

    let apu = find_scenario(scenarios, "apu_status_fault");
    assert_eq!(
        apu["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(apu["actual_focus_test_id"], Value::from(6));
    assert_eq!(
        apu["actual_focus_domain"],
        Value::String("apu.status".to_string())
    );
    assert_eq!(apu["expectation_met"], Value::Bool(true));
    assert_eq!(apu["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        apu["contract"]["expected_focus_domain"],
        Value::String("apu.status".to_string())
    );
    assert!(apu["failed_probe_ids"]
        .as_array()
        .expect("APU failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.6.result".to_string())));
    assert_eq!(apu["comparison"]["passed"], Value::Bool(false));
    assert!(
        apu["comparison"]["difference_count"]
            .as_u64()
            .expect("APU comparison difference_count should be numeric")
            > 0
    );
    let apu_triage = read_json(&suite_dir.join("apu_status_fault").join("triage.json"));
    assert_eq!(
        apu_triage["input"]["fault_injection"],
        Value::String("apu_status_register".to_string())
    );
    assert_eq!(
        apu_triage["failure"]["likely_domain"],
        Value::String("apu.status".to_string())
    );
    assert!(apu_triage["probes"]["failed"]
        .as_array()
        .expect("APU triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.6.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("apu_status_fault"),
        true,
        false,
        "0x28",
        Some("apu_status_register"),
    );

    let cpu = find_scenario(scenarios, "cpu_zero_page_wrap_fault");
    assert_eq!(
        cpu["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(cpu["actual_focus_test_id"], Value::from(12));
    assert_eq!(
        cpu["actual_focus_domain"],
        Value::String("cpu.addressing.zero_page_x_wrap".to_string())
    );
    assert_eq!(cpu["expectation_met"], Value::Bool(true));
    assert_eq!(cpu["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        cpu["contract"]["expected_focus_domain"],
        Value::String("cpu.addressing.zero_page_x_wrap".to_string())
    );
    assert!(cpu["failed_probe_ids"]
        .as_array()
        .expect("CPU failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.12.result".to_string())));
    assert_eq!(cpu["comparison"]["passed"], Value::Bool(false));
    assert!(
        cpu["comparison"]["difference_count"]
            .as_u64()
            .expect("CPU comparison difference_count should be numeric")
            > 0
    );
    let cpu_triage = read_json(
        &suite_dir
            .join("cpu_zero_page_wrap_fault")
            .join("triage.json"),
    );
    assert_eq!(
        cpu_triage["input"]["fault_injection"],
        Value::String("cpu_zero_page_index_wrap".to_string())
    );
    assert_bundle_artifacts_with_config(
        &suite_dir.join("cpu_zero_page_wrap_fault"),
        true,
        false,
        "0x28",
        Some("cpu_zero_page_index_wrap"),
    );

    let jmp = find_scenario(scenarios, "cpu_indirect_jmp_fault");
    assert_eq!(
        jmp["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(jmp["actual_focus_test_id"], Value::from(13));
    assert_eq!(
        jmp["actual_focus_domain"],
        Value::String("cpu.control_flow.indirect_jmp_page_wrap".to_string())
    );
    assert_eq!(jmp["expectation_met"], Value::Bool(true));
    assert_eq!(jmp["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        jmp["contract"]["expected_focus_domain"],
        Value::String("cpu.control_flow.indirect_jmp_page_wrap".to_string())
    );
    assert!(jmp["failed_probe_ids"]
        .as_array()
        .expect("indirect JMP failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.13.result".to_string())));
    assert_eq!(jmp["comparison"]["passed"], Value::Bool(false));
    assert!(
        jmp["comparison"]["difference_count"]
            .as_u64()
            .expect("indirect JMP comparison difference_count should be numeric")
            > 0
    );
    let jmp_triage = read_json(&suite_dir.join("cpu_indirect_jmp_fault").join("triage.json"));
    assert_eq!(
        jmp_triage["input"]["fault_injection"],
        Value::String("cpu_indirect_jmp_page_wrap".to_string())
    );
    assert_bundle_artifacts_with_config(
        &suite_dir.join("cpu_indirect_jmp_fault"),
        true,
        false,
        "0x28",
        Some("cpu_indirect_jmp_page_wrap"),
    );

    let addressing = find_scenario(scenarios, "cpu_addressing_matrix_fault");
    assert_eq!(
        addressing["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(addressing["actual_focus_test_id"], Value::from(22));
    assert_eq!(
        addressing["actual_focus_domain"],
        Value::String("cpu.addressing.page_cross_load".to_string())
    );
    assert_eq!(addressing["expectation_met"], Value::Bool(true));
    assert_eq!(addressing["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        addressing["contract"]["expected_focus_domain"],
        Value::String("cpu.addressing.page_cross_load".to_string())
    );
    assert!(addressing["failed_probe_ids"]
        .as_array()
        .expect("CPU addressing failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.22.result".to_string())));
    assert_eq!(addressing["comparison"]["passed"], Value::Bool(false));
    assert!(
        addressing["comparison"]["difference_count"]
            .as_u64()
            .expect("CPU addressing comparison difference_count should be numeric")
            > 0
    );
    let addressing_triage = read_json(
        &suite_dir
            .join("cpu_addressing_matrix_fault")
            .join("triage.json"),
    );
    assert_eq!(
        addressing_triage["input"]["fault_injection"],
        Value::String("cpu_addressing_mode_matrix".to_string())
    );
    assert_eq!(
        addressing_triage["failure"]["likely_domain"],
        Value::String("cpu.addressing.page_cross_load".to_string())
    );
    assert!(addressing_triage["probes"]["failed"]
        .as_array()
        .expect("CPU addressing triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.22.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("cpu_addressing_matrix_fault"),
        true,
        false,
        "0x28",
        Some("cpu_addressing_mode_matrix"),
    );

    let rmw = find_scenario(scenarios, "cpu_rmw_matrix_fault");
    assert_eq!(
        rmw["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(rmw["actual_focus_test_id"], Value::from(37));
    assert_eq!(
        rmw["actual_focus_domain"],
        Value::String("cpu.rmw.asl".to_string())
    );
    assert_eq!(rmw["expectation_met"], Value::Bool(true));
    assert_eq!(rmw["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        rmw["contract"]["expected_focus_domain"],
        Value::String("cpu.rmw.asl".to_string())
    );
    assert!(rmw["failed_probe_ids"]
        .as_array()
        .expect("CPU RMW failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.37.result".to_string())));
    assert_eq!(rmw["comparison"]["passed"], Value::Bool(false));
    assert!(
        rmw["comparison"]["difference_count"]
            .as_u64()
            .expect("CPU RMW comparison difference_count should be numeric")
            > 0
    );
    let rmw_triage = read_json(&suite_dir.join("cpu_rmw_matrix_fault").join("triage.json"));
    assert_eq!(
        rmw_triage["input"]["fault_injection"],
        Value::String("cpu_rmw_matrix".to_string())
    );
    assert_eq!(
        rmw_triage["failure"]["likely_domain"],
        Value::String("cpu.rmw.asl".to_string())
    );
    assert!(rmw_triage["probes"]["failed"]
        .as_array()
        .expect("CPU RMW triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.37.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("cpu_rmw_matrix_fault"),
        true,
        false,
        "0x28",
        Some("cpu_rmw_matrix"),
    );

    let rmw_addressing = find_scenario(scenarios, "cpu_rmw_addressing_matrix_fault");
    assert_eq!(
        rmw_addressing["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(rmw_addressing["actual_focus_test_id"], Value::from(38));
    assert_eq!(
        rmw_addressing["actual_focus_domain"],
        Value::String("cpu.rmw.absolute_asl".to_string())
    );
    assert_eq!(rmw_addressing["expectation_met"], Value::Bool(true));
    assert_eq!(rmw_addressing["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        rmw_addressing["contract"]["expected_focus_domain"],
        Value::String("cpu.rmw.absolute_asl".to_string())
    );
    assert!(rmw_addressing["failed_probe_ids"]
        .as_array()
        .expect("CPU RMW addressing failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.38.result".to_string())));
    assert_eq!(rmw_addressing["comparison"]["passed"], Value::Bool(false));
    assert!(
        rmw_addressing["comparison"]["difference_count"]
            .as_u64()
            .expect("CPU RMW addressing comparison difference_count should be numeric")
            > 0
    );
    let rmw_addressing_triage = read_json(
        &suite_dir
            .join("cpu_rmw_addressing_matrix_fault")
            .join("triage.json"),
    );
    assert_eq!(
        rmw_addressing_triage["input"]["fault_injection"],
        Value::String("cpu_rmw_addressing_matrix".to_string())
    );
    assert_eq!(
        rmw_addressing_triage["debug_focus"]["focus_test_name"],
        Value::String("cpu_rmw_addressing_matrix".to_string())
    );
    assert_eq!(
        rmw_addressing_triage["failure"]["likely_domain"],
        Value::String("cpu.rmw.absolute_asl".to_string())
    );
    assert!(rmw_addressing_triage["probes"]["failed"]
        .as_array()
        .expect("CPU RMW addressing triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.38.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("cpu_rmw_addressing_matrix_fault"),
        true,
        false,
        "0x28",
        Some("cpu_rmw_addressing_matrix"),
    );

    let input_port = find_scenario(scenarios, "input_port_matrix_fault");
    assert_eq!(
        input_port["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(input_port["actual_focus_test_id"], Value::from(23));
    assert_eq!(
        input_port["actual_focus_domain"],
        Value::String("joypad.input_port_matrix".to_string())
    );
    assert_eq!(input_port["expectation_met"], Value::Bool(true));
    assert_eq!(input_port["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        input_port["contract"]["expected_focus_domain"],
        Value::String("joypad.input_port_matrix".to_string())
    );
    assert!(input_port["failed_probe_ids"]
        .as_array()
        .expect("input-port failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.23.result".to_string())));
    assert_eq!(input_port["comparison"]["passed"], Value::Bool(false));
    assert!(
        input_port["comparison"]["difference_count"]
            .as_u64()
            .expect("input-port comparison difference_count should be numeric")
            > 0
    );
    let input_port_triage = read_json(
        &suite_dir
            .join("input_port_matrix_fault")
            .join("triage.json"),
    );
    assert_eq!(
        input_port_triage["input"]["fault_injection"],
        Value::String("input_port_matrix".to_string())
    );
    assert_eq!(
        input_port_triage["failure"]["likely_domain"],
        Value::String("joypad.input_port_matrix".to_string())
    );
    assert!(input_port_triage["probes"]["failed"]
        .as_array()
        .expect("input-port triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.23.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("input_port_matrix_fault"),
        true,
        false,
        "0x28",
        Some("input_port_matrix"),
    );

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

    let mapper = find_scenario(scenarios, "mapper2_bank_switch_fault");
    assert_eq!(
        mapper["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(mapper["actual_focus_test_id"], Value::from(15));
    assert_eq!(
        mapper["actual_focus_domain"],
        Value::String("mapper.uxrom.prg_bank_switch".to_string())
    );
    assert_eq!(mapper["expectation_met"], Value::Bool(true));
    assert_eq!(mapper["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        mapper["contract"]["expected_focus_domain"],
        Value::String("mapper.uxrom.prg_bank_switch".to_string())
    );
    assert!(mapper["failed_probe_ids"]
        .as_array()
        .expect("mapper failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.15.result".to_string())));
    assert_eq!(mapper["comparison"]["passed"], Value::Bool(false));
    assert!(
        mapper["comparison"]["difference_count"]
            .as_u64()
            .expect("mapper comparison difference_count should be numeric")
            > 0
    );
    let mapper_triage = read_json(
        &suite_dir
            .join("mapper2_bank_switch_fault")
            .join("triage.json"),
    );
    assert_eq!(
        mapper_triage["input"]["fault_injection"],
        Value::String("mapper2_prg_bank_switch".to_string())
    );
    assert_eq!(
        mapper_triage["failure"]["likely_domain"],
        Value::String("mapper.uxrom.prg_bank_switch".to_string())
    );
    assert!(mapper_triage["probes"]["failed"]
        .as_array()
        .expect("mapper triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.15.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("mapper2_bank_switch_fault"),
        true,
        false,
        "0x28",
        Some("mapper2_prg_bank_switch"),
    );

    let mapper_ram = find_scenario(scenarios, "mapper2_prg_ram_fault");
    assert_eq!(
        mapper_ram["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(mapper_ram["actual_focus_test_id"], Value::from(16));
    assert_eq!(
        mapper_ram["actual_focus_domain"],
        Value::String("mapper.uxrom.prg_ram".to_string())
    );
    assert_eq!(mapper_ram["expectation_met"], Value::Bool(true));
    assert_eq!(mapper_ram["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        mapper_ram["contract"]["expected_focus_domain"],
        Value::String("mapper.uxrom.prg_ram".to_string())
    );
    assert!(mapper_ram["failed_probe_ids"]
        .as_array()
        .expect("mapper PRG RAM failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.16.result".to_string())));
    assert_eq!(mapper_ram["comparison"]["passed"], Value::Bool(false));
    assert!(
        mapper_ram["comparison"]["difference_count"]
            .as_u64()
            .expect("mapper PRG RAM comparison difference_count should be numeric")
            > 0
    );
    let mapper_ram_triage = read_json(&suite_dir.join("mapper2_prg_ram_fault").join("triage.json"));
    assert_eq!(
        mapper_ram_triage["input"]["fault_injection"],
        Value::String("mapper2_prg_ram".to_string())
    );
    assert_eq!(
        mapper_ram_triage["failure"]["likely_domain"],
        Value::String("mapper.uxrom.prg_ram".to_string())
    );
    assert!(mapper_ram_triage["probes"]["failed"]
        .as_array()
        .expect("mapper PRG RAM triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.16.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("mapper2_prg_ram_fault"),
        true,
        false,
        "0x28",
        Some("mapper2_prg_ram"),
    );

    let ppu_mirroring = find_scenario(scenarios, "ppu_nametable_mirroring_fault");
    assert_eq!(
        ppu_mirroring["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(ppu_mirroring["actual_focus_test_id"], Value::from(17));
    assert_eq!(
        ppu_mirroring["actual_focus_domain"],
        Value::String("ppu.nametables.horizontal_mirroring".to_string())
    );
    assert_eq!(ppu_mirroring["expectation_met"], Value::Bool(true));
    assert_eq!(ppu_mirroring["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        ppu_mirroring["contract"]["expected_focus_domain"],
        Value::String("ppu.nametables.horizontal_mirroring".to_string())
    );
    assert!(ppu_mirroring["failed_probe_ids"]
        .as_array()
        .expect("PPU mirroring failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.17.result".to_string())));
    assert_eq!(ppu_mirroring["comparison"]["passed"], Value::Bool(false));
    assert!(
        ppu_mirroring["comparison"]["difference_count"]
            .as_u64()
            .expect("PPU mirroring comparison difference_count should be numeric")
            > 0
    );
    let ppu_mirroring_triage = read_json(
        &suite_dir
            .join("ppu_nametable_mirroring_fault")
            .join("triage.json"),
    );
    assert_eq!(
        ppu_mirroring_triage["input"]["fault_injection"],
        Value::String("ppu_nametable_mirroring".to_string())
    );
    assert_eq!(
        ppu_mirroring_triage["failure"]["likely_domain"],
        Value::String("ppu.nametables.horizontal_mirroring".to_string())
    );
    assert!(ppu_mirroring_triage["probes"]["failed"]
        .as_array()
        .expect("PPU mirroring triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.17.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("ppu_nametable_mirroring_fault"),
        true,
        false,
        "0x28",
        Some("ppu_nametable_mirroring"),
    );

    let ppu_sprite_zero_hit = find_scenario(scenarios, "ppu_sprite_zero_hit_fault");
    assert_eq!(
        ppu_sprite_zero_hit["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(ppu_sprite_zero_hit["actual_focus_test_id"], Value::from(25));
    assert_eq!(
        ppu_sprite_zero_hit["actual_focus_domain"],
        Value::String("ppu.sprite_zero_hit".to_string())
    );
    assert_eq!(ppu_sprite_zero_hit["expectation_met"], Value::Bool(true));
    assert_eq!(
        ppu_sprite_zero_hit["contract"]["all_matched"],
        Value::Bool(true)
    );
    assert_eq!(
        ppu_sprite_zero_hit["contract"]["expected_focus_domain"],
        Value::String("ppu.sprite_zero_hit".to_string())
    );
    assert!(ppu_sprite_zero_hit["failed_probe_ids"]
        .as_array()
        .expect("PPU sprite-zero-hit failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.25.result".to_string())));
    assert_eq!(
        ppu_sprite_zero_hit["comparison"]["passed"],
        Value::Bool(false)
    );
    assert!(
        ppu_sprite_zero_hit["comparison"]["difference_count"]
            .as_u64()
            .expect("PPU sprite-zero-hit comparison difference_count should be numeric")
            > 0
    );
    let ppu_sprite_zero_hit_triage = read_json(
        &suite_dir
            .join("ppu_sprite_zero_hit_fault")
            .join("triage.json"),
    );
    assert_eq!(
        ppu_sprite_zero_hit_triage["input"]["fault_injection"],
        Value::String("ppu_sprite_zero_hit".to_string())
    );
    assert_eq!(
        ppu_sprite_zero_hit_triage["failure"]["likely_domain"],
        Value::String("ppu.sprite_zero_hit".to_string())
    );
    assert!(ppu_sprite_zero_hit_triage["probes"]["failed"]
        .as_array()
        .expect("PPU sprite-zero-hit triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.25.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("ppu_sprite_zero_hit_fault"),
        true,
        false,
        "0x28",
        Some("ppu_sprite_zero_hit"),
    );

    let ppu_sprite_overflow = find_scenario(scenarios, "ppu_sprite_overflow_fault");
    assert_eq!(
        ppu_sprite_overflow["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(ppu_sprite_overflow["actual_focus_test_id"], Value::from(26));
    assert_eq!(
        ppu_sprite_overflow["actual_focus_domain"],
        Value::String("ppu.sprite_overflow".to_string())
    );
    assert_eq!(ppu_sprite_overflow["expectation_met"], Value::Bool(true));
    assert_eq!(
        ppu_sprite_overflow["contract"]["all_matched"],
        Value::Bool(true)
    );
    assert_eq!(
        ppu_sprite_overflow["contract"]["expected_focus_domain"],
        Value::String("ppu.sprite_overflow".to_string())
    );
    assert!(ppu_sprite_overflow["failed_probe_ids"]
        .as_array()
        .expect("PPU sprite-overflow failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.26.result".to_string())));
    assert_eq!(
        ppu_sprite_overflow["comparison"]["passed"],
        Value::Bool(false)
    );
    assert!(
        ppu_sprite_overflow["comparison"]["difference_count"]
            .as_u64()
            .expect("PPU sprite-overflow comparison difference_count should be numeric")
            > 0
    );
    let ppu_sprite_overflow_triage = read_json(
        &suite_dir
            .join("ppu_sprite_overflow_fault")
            .join("triage.json"),
    );
    assert_eq!(
        ppu_sprite_overflow_triage["input"]["fault_injection"],
        Value::String("ppu_sprite_overflow".to_string())
    );
    assert_eq!(
        ppu_sprite_overflow_triage["failure"]["likely_domain"],
        Value::String("ppu.sprite_overflow".to_string())
    );
    assert!(ppu_sprite_overflow_triage["probes"]["failed"]
        .as_array()
        .expect("PPU sprite-overflow triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.26.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("ppu_sprite_overflow_fault"),
        true,
        false,
        "0x28",
        Some("ppu_sprite_overflow"),
    );

    let ppu_sprite_priority = find_scenario(scenarios, "ppu_sprite_priority_fault");
    assert_eq!(
        ppu_sprite_priority["actual_health"],
        Value::String("host_validation_failed".to_string())
    );
    assert_eq!(ppu_sprite_priority["actual_focus_test_id"], Value::from(27));
    assert_eq!(
        ppu_sprite_priority["actual_focus_domain"],
        Value::String("ppu.sprite_priority".to_string())
    );
    assert_eq!(ppu_sprite_priority["expectation_met"], Value::Bool(true));
    assert_eq!(
        ppu_sprite_priority["contract"]["all_matched"],
        Value::Bool(true)
    );
    assert_eq!(
        ppu_sprite_priority["contract"]["expected_focus_domain"],
        Value::String("ppu.sprite_priority".to_string())
    );
    assert!(ppu_sprite_priority["failed_probe_ids"]
        .as_array()
        .expect("PPU sprite-priority failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("ppu.sprite_priority.samples".to_string())));
    assert_eq!(
        ppu_sprite_priority["comparison"]["passed"],
        Value::Bool(false)
    );
    assert_eq!(
        ppu_sprite_priority["comparison"]["difference_count"],
        Value::from(9)
    );
    let ppu_sprite_priority_triage = read_json(
        &suite_dir
            .join("ppu_sprite_priority_fault")
            .join("triage.json"),
    );
    assert_eq!(
        ppu_sprite_priority_triage["input"]["fault_injection"],
        Value::String("ppu_sprite_priority".to_string())
    );
    assert_eq!(
        ppu_sprite_priority_triage["debug_focus"]["failure_kind"],
        Value::String("host_validation".to_string())
    );
    assert_eq!(
        ppu_sprite_priority_triage["failure"]["likely_domain"],
        Value::String("ppu.sprite_priority".to_string())
    );
    assert!(ppu_sprite_priority_triage["probes"]["failed"]
        .as_array()
        .expect("PPU sprite-priority triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("ppu.sprite_priority.samples".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("ppu_sprite_priority_fault"),
        true,
        false,
        "0x28",
        Some("ppu_sprite_priority"),
    );

    let ppu_scroll_seam = find_scenario(scenarios, "ppu_scroll_seam_fault");
    assert_eq!(
        ppu_scroll_seam["actual_health"],
        Value::String("host_validation_failed".to_string())
    );
    assert_eq!(ppu_scroll_seam["actual_focus_test_id"], Value::from(28));
    assert_eq!(
        ppu_scroll_seam["actual_focus_domain"],
        Value::String("ppu.scroll_seam".to_string())
    );
    assert_eq!(ppu_scroll_seam["expectation_met"], Value::Bool(true));
    assert_eq!(
        ppu_scroll_seam["contract"]["all_matched"],
        Value::Bool(true)
    );
    assert_eq!(
        ppu_scroll_seam["contract"]["expected_focus_domain"],
        Value::String("ppu.scroll_seam".to_string())
    );
    assert!(ppu_scroll_seam["failed_probe_ids"]
        .as_array()
        .expect("PPU scroll-seam failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("ppu.scroll_seam.samples".to_string())));
    assert_eq!(ppu_scroll_seam["comparison"]["passed"], Value::Bool(false));
    assert!(
        ppu_scroll_seam["comparison"]["difference_count"]
            .as_u64()
            .expect("PPU scroll-seam comparison difference_count should be numeric")
            > 0
    );
    let ppu_scroll_seam_triage =
        read_json(&suite_dir.join("ppu_scroll_seam_fault").join("triage.json"));
    assert_eq!(
        ppu_scroll_seam_triage["input"]["fault_injection"],
        Value::String("ppu_scroll_seam".to_string())
    );
    assert_eq!(
        ppu_scroll_seam_triage["debug_focus"]["failure_kind"],
        Value::String("host_validation".to_string())
    );
    assert_eq!(
        ppu_scroll_seam_triage["failure"]["likely_domain"],
        Value::String("ppu.scroll_seam".to_string())
    );
    assert!(ppu_scroll_seam_triage["probes"]["failed"]
        .as_array()
        .expect("PPU scroll-seam triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("ppu.scroll_seam.samples".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("ppu_scroll_seam_fault"),
        true,
        false,
        "0x28",
        Some("ppu_scroll_seam"),
    );

    let joypad_reset = find_scenario(scenarios, "joypad_strobe_reset_fault");
    assert_eq!(
        joypad_reset["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(joypad_reset["actual_focus_test_id"], Value::from(18));
    assert_eq!(
        joypad_reset["actual_focus_domain"],
        Value::String("joypad.strobe_reset".to_string())
    );
    assert_eq!(joypad_reset["expectation_met"], Value::Bool(true));
    assert_eq!(joypad_reset["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        joypad_reset["contract"]["expected_focus_domain"],
        Value::String("joypad.strobe_reset".to_string())
    );
    assert!(joypad_reset["failed_probe_ids"]
        .as_array()
        .expect("joypad reset failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.18.result".to_string())));
    assert_eq!(joypad_reset["comparison"]["passed"], Value::Bool(false));
    assert!(
        joypad_reset["comparison"]["difference_count"]
            .as_u64()
            .expect("joypad reset comparison difference_count should be numeric")
            > 0
    );
    let joypad_reset_triage = read_json(
        &suite_dir
            .join("joypad_strobe_reset_fault")
            .join("triage.json"),
    );
    assert_eq!(
        joypad_reset_triage["input"]["fault_injection"],
        Value::String("joypad_strobe_reset".to_string())
    );
    assert_eq!(
        joypad_reset_triage["failure"]["likely_domain"],
        Value::String("joypad.strobe_reset".to_string())
    );
    assert!(joypad_reset_triage["probes"]["failed"]
        .as_array()
        .expect("joypad reset triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.18.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("joypad_strobe_reset_fault"),
        true,
        false,
        "0x28",
        Some("joypad_strobe_reset"),
    );

    let ppu_increment_32 = find_scenario(scenarios, "ppu_vram_increment_32_fault");
    assert_eq!(
        ppu_increment_32["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(ppu_increment_32["actual_focus_test_id"], Value::from(19));
    assert_eq!(
        ppu_increment_32["actual_focus_domain"],
        Value::String("ppu.registers.ppudata_increment_32".to_string())
    );
    assert_eq!(ppu_increment_32["expectation_met"], Value::Bool(true));
    assert_eq!(
        ppu_increment_32["contract"]["all_matched"],
        Value::Bool(true)
    );
    assert_eq!(
        ppu_increment_32["contract"]["expected_focus_domain"],
        Value::String("ppu.registers.ppudata_increment_32".to_string())
    );
    assert!(ppu_increment_32["failed_probe_ids"]
        .as_array()
        .expect("PPU increment-32 failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.19.result".to_string())));
    assert_eq!(ppu_increment_32["comparison"]["passed"], Value::Bool(false));
    assert!(
        ppu_increment_32["comparison"]["difference_count"]
            .as_u64()
            .expect("PPU increment-32 comparison difference_count should be numeric")
            > 0
    );
    let ppu_increment_32_triage = read_json(
        &suite_dir
            .join("ppu_vram_increment_32_fault")
            .join("triage.json"),
    );
    assert_eq!(
        ppu_increment_32_triage["input"]["fault_injection"],
        Value::String("ppu_vram_increment_32".to_string())
    );
    assert_eq!(
        ppu_increment_32_triage["failure"]["likely_domain"],
        Value::String("ppu.registers.ppudata_increment_32".to_string())
    );
    assert!(ppu_increment_32_triage["probes"]["failed"]
        .as_array()
        .expect("PPU increment-32 triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.19.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("ppu_vram_increment_32_fault"),
        true,
        false,
        "0x28",
        Some("ppu_vram_increment_32"),
    );

    let ppu_status_latch = find_scenario(scenarios, "ppu_status_latch_reset_fault");
    assert_eq!(
        ppu_status_latch["actual_health"],
        Value::String("cartridge_assertion_failed".to_string())
    );
    assert_eq!(ppu_status_latch["actual_focus_test_id"], Value::from(20));
    assert_eq!(
        ppu_status_latch["actual_focus_domain"],
        Value::String("ppu.registers.status_latch_reset".to_string())
    );
    assert_eq!(ppu_status_latch["expectation_met"], Value::Bool(true));
    assert_eq!(
        ppu_status_latch["contract"]["all_matched"],
        Value::Bool(true)
    );
    assert_eq!(
        ppu_status_latch["contract"]["expected_focus_domain"],
        Value::String("ppu.registers.status_latch_reset".to_string())
    );
    assert!(ppu_status_latch["failed_probe_ids"]
        .as_array()
        .expect("PPU status latch failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("cartridge.test.20.result".to_string())));
    assert_eq!(ppu_status_latch["comparison"]["passed"], Value::Bool(false));
    assert!(
        ppu_status_latch["comparison"]["difference_count"]
            .as_u64()
            .expect("PPU status latch comparison difference_count should be numeric")
            > 0
    );
    let ppu_status_latch_triage = read_json(
        &suite_dir
            .join("ppu_status_latch_reset_fault")
            .join("triage.json"),
    );
    assert_eq!(
        ppu_status_latch_triage["input"]["fault_injection"],
        Value::String("ppu_status_latch_reset".to_string())
    );
    assert_eq!(
        ppu_status_latch_triage["failure"]["likely_domain"],
        Value::String("ppu.registers.status_latch_reset".to_string())
    );
    assert!(ppu_status_latch_triage["probes"]["failed"]
        .as_array()
        .expect("PPU status latch triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("cartridge.test.20.result".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("ppu_status_latch_reset_fault"),
        true,
        false,
        "0x28",
        Some("ppu_status_latch_reset"),
    );

    let ppu_nmi = find_scenario(scenarios, "ppu_nmi_timeout_fault");
    assert_eq!(
        ppu_nmi["actual_health"],
        Value::String("timed_out".to_string())
    );
    assert_eq!(ppu_nmi["actual_focus_test_id"], Value::from(10));
    assert_eq!(
        ppu_nmi["actual_focus_domain"],
        Value::String("ppu.nmi".to_string())
    );
    assert_eq!(ppu_nmi["expectation_met"], Value::Bool(true));
    assert_eq!(ppu_nmi["contract"]["all_matched"], Value::Bool(true));
    assert_eq!(
        ppu_nmi["contract"]["expected_focus_domain"],
        Value::String("ppu.nmi".to_string())
    );
    assert!(ppu_nmi["failed_probe_ids"]
        .as_array()
        .expect("PPU NMI failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("ppu.nmi_count".to_string())));
    assert!(ppu_nmi["failed_probe_ids"]
        .as_array()
        .expect("PPU NMI failed probes should be an array")
        .iter()
        .any(|probe| probe == &Value::String("ppu.vblank_timing.nmi_window".to_string())));
    assert_eq!(ppu_nmi["comparison"]["passed"], Value::Bool(false));
    assert!(
        ppu_nmi["comparison"]["difference_count"]
            .as_u64()
            .expect("PPU NMI comparison difference_count should be numeric")
            > 0
    );
    let ppu_nmi_triage = read_json(&suite_dir.join("ppu_nmi_timeout_fault").join("triage.json"));
    assert_eq!(
        ppu_nmi_triage["input"]["fault_injection"],
        Value::String("ppu_nmi_timeout".to_string())
    );
    assert_eq!(
        ppu_nmi_triage["health"],
        Value::String("timed_out".to_string())
    );
    assert_eq!(
        ppu_nmi_triage["debug_focus"]["focus_test_name"],
        Value::String("ppu_nmi_and_render_frame".to_string())
    );
    assert_eq!(
        ppu_nmi_triage["debug_focus"]["focus_domain"],
        Value::String("ppu.nmi".to_string())
    );
    assert_eq!(
        ppu_nmi_triage["failure"]["likely_domain"],
        Value::String("ppu.nmi".to_string())
    );
    assert!(ppu_nmi_triage["probes"]["failed"]
        .as_array()
        .expect("PPU NMI triage failed probes should be an array")
        .iter()
        .any(|probe| probe["id"] == Value::String("ppu.nmi_count".to_string())));
    assert_bundle_artifacts_with_config(
        &suite_dir.join("ppu_nmi_timeout_fault"),
        true,
        false,
        "0x28",
        Some("ppu_nmi_timeout"),
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
    assert_eq!(triage["telemetry_schema_version"], Value::from(60));
    assert_eq!(triage["passed"], Value::Bool(true));
    assert_eq!(
        triage["debug_focus"]["health"],
        Value::String("healthy".to_string())
    );
    assert_eq!(
        triage["debug_focus"]["focus_test_name"],
        Value::String("cpu_rmw_addressing_matrix".to_string())
    );
    assert_eq!(
        triage["debug_focus"]["terminal_instruction"]["symbol"],
        Value::String("hang".to_string())
    );
    assert!(triage["debug_focus"]["terminal_instruction"]["instruction"]
        .as_str()
        .is_some_and(|instruction| instruction.starts_with("JMP 0x")));
    assert_eq!(triage["coverage"]["passed_tests"], Value::from(30));
    assert_eq!(triage["dma"]["oam_dma_completed"], Value::Bool(true));
    assert_eq!(
        triage["dma"]["oam_dma_phase_matrix_passed"],
        Value::Bool(true)
    );
    assert_eq!(
        triage["dma"]["oam_dma_phase_matrix_test_transfer_count"],
        Value::from(5)
    );
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
    assert!(
        triage["dma"]["dmc_dma_oam_overlap_offsets"]
            .as_array()
            .expect("DMC/OAM overlap offsets should be an array")
            .len()
            >= 3
    );
    assert!(
        triage["dma"]["dmc_dma_oam_overlap_phase_matrix_transfer_indices"]
            .as_array()
            .expect("DMC/OAM phase-matrix transfer indices should be an array")
            .len()
            >= 3
    );
    assert!(
        triage["dma"]["dmc_dma_oam_overlap_phase_matrix_distinct_transfer_count"]
            .as_u64()
            .is_some_and(|count| count >= 3)
    );
    assert_eq!(
        triage["dma"]["dmc_dma_oam_overlap_expected_min_phase_matrix_transfers"],
        Value::from(3)
    );
    assert_eq!(
        triage["dma"]["dmc_dma_oam_overlap_burst_train_passed"],
        Value::Bool(true)
    );
    assert_eq!(
        triage["dma"]["dmc_dma_oam_overlap_expected_position_buckets"],
        serde_json::json!(["beginning", "middle", "end"])
    );
    assert_eq!(
        triage["dma"]["dmc_dma_oam_overlap_missing_position_buckets"],
        serde_json::json!([])
    );
    assert_eq!(
        triage["dma"]["dmc_dma_oam_overlap_position_matrix_passed"],
        Value::Bool(true)
    );
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
    assert_eq!(manifest["bundle_schema_version"], Value::from(3));
    assert_eq!(manifest["telemetry_schema_version"], Value::from(60));
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

fn assert_replay_args_contains(args: &Value, expected: &str) {
    assert!(
        args.as_array()
            .expect("replay_args should be an array")
            .iter()
            .any(|arg| arg == &Value::String(expected.to_string())),
        "replay_args should contain {expected}"
    );
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
