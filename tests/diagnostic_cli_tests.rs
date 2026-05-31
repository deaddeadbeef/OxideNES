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
        telemetry["analysis"]["probe_summary"]["first_failed_probe"],
        Value::String("cartridge.status.pass".to_string())
    );
    let triage = read_json(&bundle_dir.join("triage.json"));
    assert_eq!(triage["passed"], Value::Bool(false));
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
    assert_eq!(triage["triage_schema_version"], Value::from(1));
    assert_eq!(triage["telemetry_schema_version"], Value::from(5));
    assert_eq!(triage["passed"], Value::Bool(true));
    assert_eq!(triage["coverage"]["passed_tests"], Value::from(10));
    assert_eq!(triage["probes"]["failed_probes"], Value::from(0));
    assert!(triage["artifact_hints"]
        .as_array()
        .expect("artifact hints should be an array")
        .iter()
        .any(|hint| hint["path"] == Value::String("telemetry.json".to_string())));

    fs::remove_dir_all(&root).expect("triage temp dir should be removable");
}

fn assert_bundle_artifacts(bundle_dir: &Path, includes_comparison: bool, passed: bool) {
    let manifest = read_json(&bundle_dir.join("manifest.json"));
    assert_eq!(manifest["bundle_schema_version"], Value::from(1));
    assert_eq!(manifest["telemetry_schema_version"], Value::from(5));
    assert_eq!(manifest["passed"], Value::Bool(passed));
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
