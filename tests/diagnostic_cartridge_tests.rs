use oxidenes::diagnostic::{
    build_diagnostic_cartridge, run_diagnostic, DiagnosticConfig, DIAGNOSTIC_PROVENANCE,
    DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION, DIAGNOSTIC_TESTS,
};

#[test]
fn generated_diagnostic_cartridge_runs_headlessly_to_pass() {
    let telemetry = run_diagnostic(DiagnosticConfig::default()).expect("diagnostic should run");

    assert!(
        telemetry.verdict.passed,
        "diagnostic failed with host failures: {:?}",
        telemetry.verdict.host_failures
    );
    assert_eq!(
        telemetry.schema_version,
        DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION
    );
    assert_eq!(telemetry.suite.test_count, DIAGNOSTIC_TESTS.len());
    assert_eq!(telemetry.tests.len(), DIAGNOSTIC_TESTS.len());
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "cpu_branch_page_crossing"
            && test.intent.contains("page boundary")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "joypad_overread_returns_one"
            && test.intent.contains("eighth latched button")
            && test.passed
    }));
    assert!(telemetry
        .events
        .iter()
        .any(|event| event.current_test_name == Some("joypad_overread_returns_one")));
    assert!(telemetry.cycles > 0);
    assert!(telemetry.frames >= 2);
}

#[test]
fn generated_diagnostic_cartridge_has_no_bundled_rom_provenance() {
    let rom = build_diagnostic_cartridge().expect("diagnostic cartridge should build");

    assert_eq!(&rom[0..4], b"NES\x1A");
    assert_eq!(rom.len(), 16 + 2 * 16 * 1024 + 8 * 1024);
    assert!(DIAGNOSTIC_PROVENANCE.contains("Generated OxideNES diagnostic"));
    assert!(DIAGNOSTIC_PROVENANCE.contains("no ROM content"));
}
