use oxidenes::diagnostic::{
    build_diagnostic_cartridge, format_diagnostic_report, run_diagnostic, DiagnosticConfig,
    DiagnosticFailureKind, DiagnosticHealth, DiagnosticSubsystem, TestTimelineEndReason,
    TestTimelineOutcome, DIAGNOSTIC_PROVENANCE, DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION,
    DIAGNOSTIC_TESTS,
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
    assert_eq!(telemetry.timeline.len(), DIAGNOSTIC_TESTS.len());
    assert_eq!(
        telemetry.analysis.timing.started_tests,
        DIAGNOSTIC_TESTS.len()
    );
    assert_eq!(
        telemetry.analysis.timing.ended_tests,
        DIAGNOSTIC_TESTS.len()
    );
    assert_eq!(telemetry.analysis.timing.not_started_tests, 0);
    assert_eq!(telemetry.analysis.timing.timed_out_tests, 0);
    assert!(telemetry
        .analysis
        .timing
        .slowest_test
        .as_ref()
        .is_some_and(|test| test.duration_cycles > 0));
    assert!(telemetry.timeline.iter().all(|test| {
        test.outcome == TestTimelineOutcome::Passed
            && test.started
            && test.ended
            && test.duration_cycles.is_some()
    }));
    assert_eq!(
        telemetry.timeline.last().map(|test| test.end_reason),
        Some(Some(TestTimelineEndReason::CartridgePassed))
    );
    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("# OxideNES Diagnostic Report"));
    assert!(report.contains("| Result | pass |"));
    assert!(report.contains("| Health | healthy |"));
    assert!(report.contains("## Coverage"));
    assert!(report.contains("## Timing"));
    assert!(report.contains("| Slowest test | ppu_nmi_and_render_frame"));
    assert!(report.contains("| 10 | ppu_nmi_and_render_frame | ppu | integration | passed |"));
    assert!(report.contains("## Event Tail"));
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

    assert_eq!(telemetry.analysis.timing.started_tests, 7);
    assert_eq!(telemetry.analysis.timing.ended_tests, 7);
    assert_eq!(telemetry.analysis.timing.not_started_tests, 3);
    let failing_timeline = telemetry
        .timeline
        .iter()
        .find(|test| test.test_id == 7)
        .expect("failing test should have timeline telemetry");
    assert_eq!(failing_timeline.test_name, "joypad_strobe_shift");
    assert_eq!(failing_timeline.outcome, TestTimelineOutcome::Failed);
    assert_eq!(
        failing_timeline.end_reason,
        Some(TestTimelineEndReason::CartridgeFailed)
    );
    assert_eq!(failing_timeline.terminal_status, Some(0xE0));
    assert!(failing_timeline
        .duration_cycles
        .is_some_and(|duration| duration > 0));
    assert!(telemetry
        .timeline
        .iter()
        .filter(|test| test.test_id > 7)
        .all(|test| test.outcome == TestTimelineOutcome::NotStarted));
    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Result | fail |"));
    assert!(report.contains("| Health | cartridge_assertion_failed |"));
    assert!(report.contains("## Failure Localization"));
    assert!(report.contains("| Likely domain | joypad.strobe_shift |"));
    assert!(report.contains("| Remediation hint | Inspect joypad strobe"));
    assert!(report.contains("| 7 | joypad_strobe_shift | joypad | smoke | failed |"));
    assert!(report.contains("| 8 | cpu_branch_page_crossing | cpu | edge_case | not_started |"));
    assert!(report.contains("## Host Failures"));
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
    assert_eq!(telemetry.analysis.timing.started_tests, 0);
    assert_eq!(telemetry.analysis.timing.ended_tests, 0);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len()
    );
    assert!(telemetry.analysis.timing.slowest_test.is_none());
    assert!(telemetry
        .timeline
        .iter()
        .all(|test| test.outcome == TestTimelineOutcome::NotStarted));
    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Health | timed_out |"));
    assert!(report.contains("| First failure domain | emulator.progress_or_infinite_loop |"));
    assert!(report.contains("| Not started tests | 10 |"));
    assert!(report.contains("| Slowest test | none |"));
}

#[test]
fn generated_diagnostic_cartridge_has_no_bundled_rom_provenance() {
    let rom = build_diagnostic_cartridge().expect("diagnostic cartridge should build");

    assert_eq!(&rom[0..4], b"NES\x1A");
    assert_eq!(rom.len(), 16 + 2 * 16 * 1024 + 8 * 1024);
    assert!(DIAGNOSTIC_PROVENANCE.contains("Generated OxideNES diagnostic"));
    assert!(DIAGNOSTIC_PROVENANCE.contains("no ROM content"));
}
