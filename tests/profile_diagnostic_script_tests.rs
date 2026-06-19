use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

fn diagnostic_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_oxidenes-diagnostic"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "oxidenes-profile-script-{name}-{}-{nanos}",
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
    panic!("python or python3 should be available for diagnostic script tests");
}

fn diagnostic_binary_name() -> &'static str {
    if cfg!(windows) {
        "oxidenes-diagnostic.exe"
    } else {
        "oxidenes-diagnostic"
    }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("json file should be readable"))
        .expect("json file should parse")
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .expect("copied diagnostic binary metadata should be readable")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("copied diagnostic binary should be executable");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

#[test]
fn profile_script_uses_cargo_target_dir_for_skip_build_binary() {
    let root = temp_dir("target-dir");
    let cargo_target_dir = root.join("cargo-target");
    let bin_dir = cargo_target_dir.join("debug");
    fs::create_dir_all(&bin_dir).expect("fake cargo target dir should be created");
    let copied_binary = bin_dir.join(diagnostic_binary_name());
    fs::copy(diagnostic_bin(), &copied_binary).expect("diagnostic binary should copy");
    make_executable(&copied_binary);

    let output_dir = root.join("profile");
    let output = Command::new(python_command())
        .current_dir(repo_root())
        .env("CARGO_TARGET_DIR", &cargo_target_dir)
        .arg("scripts/profile_diagnostic_cartridge.py")
        .arg("--output-dir")
        .arg(&output_dir)
        .arg("--profile")
        .arg("debug")
        .arg("--samples")
        .arg("1")
        .arg("--warmups")
        .arg("0")
        .arg("--skip-build")
        .output()
        .expect("profile script should run");

    assert!(
        output.status.success(),
        "profile script failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let profile = read_json(&output_dir.join("diagnostic-cartridge-profile.json"));
    assert_eq!(profile["status"], Value::String("passed".to_string()));
    assert_eq!(
        profile["diagnostic_cartridge_profile_schema_version"],
        Value::from(2)
    );
    assert_eq!(
        PathBuf::from(profile["config"]["binary"].as_str().unwrap())
            .canonicalize()
            .expect("profile binary should exist"),
        copied_binary
            .canonicalize()
            .expect("copied binary should exist")
    );
    assert_eq!(
        PathBuf::from(profile["config"]["target_dir"].as_str().unwrap())
            .canonicalize()
            .expect("profile target dir should exist"),
        cargo_target_dir
            .canonicalize()
            .expect("cargo target dir should exist")
    );
    assert_eq!(
        profile["build_metadata"]["version"],
        Value::String(env!("OXIDENES_VERSION").to_string())
    );
    assert_eq!(
        profile["build_metadata"]["build_type"],
        Value::String(env!("OXIDENES_BUILD_TYPE").to_string())
    );
    assert_eq!(
        profile["build_metadata"]["package_version"],
        Value::String(env!("CARGO_PKG_VERSION").to_string())
    );

    fs::remove_dir_all(&root).expect("profile temp dir should be removable");
}

#[test]
fn profile_script_reports_missing_build_metadata_fields() {
    let script = repo_root().join("scripts/profile_diagnostic_cartridge.py");
    let output = Command::new(python_command())
        .env("PROFILE_SCRIPT", script)
        .arg("-c")
        .arg(
            r#"
import importlib.util
import os

script = os.environ["PROFILE_SCRIPT"]
spec = importlib.util.spec_from_file_location("profile_diagnostic_cartridge", script)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

assert module.missing_build_metadata_fields({}) == ["version", "build_type", "package_version"]
assert module.missing_build_metadata_fields({"version": "0.3.40-dev"}) == ["build_type", "package_version"]
assert module.missing_build_metadata_fields({"version": "0.3.40-dev", "build_type": "dev", "package_version": "0.3.40"}) == []
"#,
        )
        .output()
        .expect("python metadata validator check should run");

    assert!(
        output.status.success(),
        "metadata validator check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
