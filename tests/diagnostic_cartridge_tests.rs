use oxidenes::diagnostic::{
    build_diagnostic_cartridge, run_diagnostic, DiagnosticConfig, DiagnosticFailureKind,
    DiagnosticHealth, DiagnosticSubsystem, DIAGNOSTIC_PROVENANCE,
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
    assert_eq!(telemetry.analysis.health, DiagnosticHealth::Healthy);
    assert_eq!(
        telemetry.analysis.coverage.total_tests,
        DIAGNOSTIC_TESTS.len()
    );
    assert_eq!(
        telemetry.analysis.coverage.passed_tests,
        DIAGNOSTIC_TESTS.len()
    );
    assert_eq!(telemetry.analysis.coverage.failed_tests, 0);
    assert_eq!(telemetry.analysis.failing_subsystem, None);
    assert_eq!(telemetry.analysis.failing_test, None);
    assert_eq!(telemetry.analysis.first_failure_domain, None);
    assert!(telemetry.analysis.summary.contains("diagnostic passed"));
    assert!(telemetry
        .analysis
        .coverage
        .subsystem_summary
        .iter()
        .any(|entry| entry.subsystem == DiagnosticSubsystem::Cpu && entry.total == 3));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0x70
            && failure.test_name == Some("joypad_strobe_shift")
            && failure.likely_domain == "joypad.strobe_shift"
    }));
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
fn generated_diagnostic_cartridge_localizes_intentional_joypad_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        joypad1_mask: 0x00,
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(telemetry.verdict.current_test, 7);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("joypad_strobe_shift")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x70);
    assert_eq!(telemetry.verdict.host_failures.len(), 1);
    assert!(telemetry.verdict.host_failures[0].contains("test 7"));

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include structured failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 7);
    assert_eq!(failure.test_name, Some("joypad_strobe_shift"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Joypad));
    assert_eq!(failure.failure_code_hex, "0x70");
    assert_eq!(failure.likely_domain, "joypad.strobe_shift");
    assert!(failure.assertion.contains("A button"));
    assert!(failure.remediation_hint.contains("joypad strobe"));

    assert_eq!(
        telemetry.analysis.health,
        DiagnosticHealth::CartridgeAssertionFailed
    );
    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Joypad)
    );
    assert_eq!(telemetry.analysis.failing_test, Some("joypad_strobe_shift"));
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("joypad.strobe_shift")
    );
    assert!(telemetry
        .analysis
        .summary
        .contains("diagnostic failed at joypad_strobe_shift"));
    assert!(telemetry
        .analysis
        .next_actions
        .iter()
        .any(|action| action.contains("joypad strobe")));
}

#[test]
fn generated_diagnostic_cartridge_localizes_timeout() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        max_cpu_cycles: 1,
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should return timeout telemetry");

    assert!(!telemetry.verdict.passed);
    assert!(telemetry.verdict.timeout);
    assert!(telemetry
        .verdict
        .host_failures
        .iter()
        .any(|failure| failure.contains("timed out")));

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("timeout should include structured failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::Timeout);
    assert_eq!(failure.likely_domain, "emulator.progress_or_infinite_loop");
    assert!(failure.observed.contains("status was"));

    assert_eq!(telemetry.analysis.health, DiagnosticHealth::TimedOut);
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("emulator.progress_or_infinite_loop")
    );
    assert!(telemetry.analysis.summary.contains("diagnostic timed out"));
    assert!(telemetry
        .analysis
        .next_actions
        .iter()
        .any(|action| action.contains("CPU PC")));
}

#[test]
fn generated_diagnostic_cartridge_has_no_bundled_rom_provenance() {
    let rom = build_diagnostic_cartridge().expect("diagnostic cartridge should build");

    assert_eq!(&rom[0..4], b"NES\x1A");
    assert_eq!(rom.len(), 16 + 2 * 16 * 1024 + 8 * 1024);
    assert!(DIAGNOSTIC_PROVENANCE.contains("Generated OxideNES diagnostic"));
    assert!(DIAGNOSTIC_PROVENANCE.contains("no ROM content"));
}
