use std::process::Command;

fn oxidenes_binary() -> &'static str {
    env!("CARGO_BIN_EXE_oxidenes")
}

fn expected_version() -> &'static str {
    env!("OXIDENES_VERSION")
}

fn expected_build_type() -> &'static str {
    env!("OXIDENES_BUILD_TYPE")
}

#[test]
fn build_metadata_version_matches_build_type() {
    match expected_build_type() {
        "dev" => assert_eq!(
            expected_version(),
            format!("{}-dev", env!("CARGO_PKG_VERSION"))
        ),
        "release" => assert_eq!(expected_version(), env!("CARGO_PKG_VERSION")),
        other => panic!("unexpected OxideNES build type: {other}"),
    }
}

#[test]
fn version_cli_reports_build_metadata_version() {
    let output = Command::new(oxidenes_binary())
        .arg("--version")
        .output()
        .expect("run oxidenes --version");

    assert!(
        output.status.success(),
        "--version failed: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("version output is utf-8");
    assert_eq!(stdout.trim(), format!("oxidenes {}", expected_version()));
}

#[test]
fn help_banner_reports_build_metadata_version() {
    let output = Command::new(oxidenes_binary())
        .arg("--help")
        .output()
        .expect("run oxidenes --help");

    assert!(
        output.status.success(),
        "--help failed: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("help output is utf-8");
    let expected = format!("OxideNES v{}", expected_version());
    assert_eq!(stdout.lines().next(), Some(expected.as_str()));
}
