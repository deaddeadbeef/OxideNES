use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn oxidenes_binary() -> &'static str {
    env!("CARGO_BIN_EXE_oxidenes")
}

fn expected_version() -> &'static str {
    env!("OXIDENES_VERSION")
}

fn expected_build_type() -> &'static str {
    env!("OXIDENES_BUILD_TYPE")
}

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "oxidenes-cli-{name}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("temp dir should be creatable");
    dir
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

#[test]
fn import_roms_cli_copies_into_default_library_and_updates_config() {
    let root = temp_dir("import-copy");
    let home = root.join("home");
    let source = root.join("source");
    fs::create_dir_all(&home).expect("home dir should be creatable");
    fs::create_dir_all(&source).expect("source dir should be creatable");
    fs::write(source.join("alpha.nes"), b"alpha").expect("source ROM should be writable");
    fs::write(source.join("BETA.NES"), b"beta").expect("source ROM should be writable");
    fs::write(source.join("notes.txt"), b"not a ROM").expect("non-ROM should be writable");

    let output = Command::new(oxidenes_binary())
        .arg("--import-roms")
        .arg(&source)
        .arg("--import-mode")
        .arg("copy")
        .env("USERPROFILE", &home)
        .env("HOME", &home)
        .output()
        .expect("run oxidenes --import-roms");

    assert!(
        output.status.success(),
        "--import-roms failed: status={:?} stdout={} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("import stdout is utf-8");
    assert!(stdout.contains("Imported 2 NES ROM(s) with copy mode"));
    assert!(stdout.contains("Skipped 1 non-NES entry(s)"));

    let library = home.join(".nes-emulator").join("roms");
    assert_eq!(fs::read(library.join("alpha.nes")).unwrap(), b"alpha");
    assert_eq!(fs::read(library.join("BETA.NES")).unwrap(), b"beta");
    assert!(!library.join("notes.txt").exists());

    let config: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(home.join(".nes-emulator").join("config.json")).unwrap(),
    )
    .expect("config json should parse");
    assert_eq!(
        config["rom_directory"].as_str(),
        Some(library.to_string_lossy().as_ref())
    );

    fs::remove_dir_all(root).expect("temp dir should be removable");
}

#[test]
fn import_roms_cli_rejects_invalid_mode_without_creating_library() {
    let root = temp_dir("import-invalid-mode");
    let home = root.join("home");
    let source = root.join("source");
    fs::create_dir_all(&home).expect("home dir should be creatable");
    fs::create_dir_all(&source).expect("source dir should be creatable");
    fs::write(source.join("game.nes"), b"game").expect("source ROM should be writable");

    let output = Command::new(oxidenes_binary())
        .arg("--import-roms")
        .arg(&source)
        .arg("--import-mode")
        .arg("move")
        .env("USERPROFILE", &home)
        .env("HOME", &home)
        .output()
        .expect("run oxidenes --import-roms with invalid mode");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).expect("import stderr is utf-8");
    assert!(stderr.contains("unsupported --import-mode: move"));
    assert!(!home.join(".nes-emulator").join("roms").exists());

    fs::remove_dir_all(root).expect("temp dir should be removable");
}
