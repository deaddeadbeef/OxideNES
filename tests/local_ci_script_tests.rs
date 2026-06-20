use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oxidenes-local-ci-script-{name}-{}-{nanos}",
        std::process::id()
    ))
}

fn python_command() -> String {
    for candidate in ["python", "python3"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            return candidate.to_string();
        }
    }
    panic!("python or python3 should be available for local CI script tests");
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("json file should be readable"))
        .expect("json file should parse")
}

#[test]
fn local_ci_script_dry_run_writes_ai_ready_report() {
    let root = temp_dir("dry-run");
    let output_dir = root.join("local-ci");
    let suite_dir = root.join("scenario-suite");

    let output = Command::new(python_command())
        .current_dir(repo_root())
        .arg("scripts/run_local_ci.py")
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--suite-dir")
        .arg(&suite_dir)
        .arg("--target")
        .arg("x86_64-pc-windows-msvc")
        .arg("--dry-run")
        .output()
        .expect("local CI script dry-run should start");

    assert!(
        output.status.success(),
        "local CI dry-run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report_path = output_dir.join("local-ci-report.json");
    let report = read_json(&report_path);
    assert_eq!(report["local_ci_schema_version"], Value::from(1));
    assert_eq!(report["status"], Value::String("planned".to_string()));
    assert_eq!(report["dry_run"], Value::Bool(true));
    assert_eq!(
        report["artifacts"]["diagnostic_scenario_suite_dir"],
        Value::String(suite_dir.to_string_lossy().to_string())
    );

    let names: Vec<_> = report["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .map(|command| command["name"].as_str().expect("command should have name"))
        .collect();
    for expected in [
        "fmt",
        "ip-compliance",
        "security-audit",
        "diagnostic-e2e",
        "verify-diagnostic-observability",
        "verify-diagnostic-suite",
        "diagnostic-profile",
        "build",
        "test",
        "smoke-binary",
        "clippy",
    ] {
        assert!(
            names.contains(&expected),
            "missing expected local CI step {expected}"
        );
    }

    let security_audit = report["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .find(|command| command["name"] == Value::String("security-audit".to_string()))
        .expect("security-audit step should be present");
    assert_eq!(
        security_audit["argv"],
        serde_json::json!(["cargo", "audit", "--no-fetch", "--stale"])
    );

    assert!(report["commands"]
        .as_array()
        .expect("commands should be an array")
        .iter()
        .all(
            |command| command["status"] == Value::String("skipped".to_string())
                && command["skip_reason"] == Value::String("dry-run".to_string())
        ));

    let markdown = fs::read_to_string(output_dir.join("local-ci-report.md"))
        .expect("markdown report should be readable");
    assert!(markdown.contains("# OxideNES Local CI Report"));
    assert!(markdown.contains("| diagnostic-e2e | skipped |"));
    assert!(markdown.contains("diagnostic_scenario_suite_dir"));

    fs::remove_dir_all(&root).expect("local CI temp dir should be removable");
}
