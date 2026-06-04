use oxidenes::diagnostic::{
    build_diagnostic_cartridge, compare_diagnostic_to_baseline,
    format_diagnostic_comparison_report, format_diagnostic_report, run_diagnostic,
    DiagnosticComparisonSeverity, DiagnosticConfig, DiagnosticFailureKind,
    DiagnosticFaultInjection, DiagnosticHealth, DiagnosticProbeStatus, DiagnosticSubsystem,
    TestTimelineEndReason, TestTimelineOutcome, DIAGNOSTIC_PROVENANCE,
    DIAGNOSTIC_RENDER_FRAME_EXPECTED_CHECKSUM, DIAGNOSTIC_RENDER_FRAME_EXPECTED_NONZERO_PIXELS,
    DIAGNOSTIC_RENDER_FRAME_EXPECTED_UNIQUE_COLORS, DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION,
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
    assert!(telemetry
        .analysis
        .coverage_gaps
        .iter()
        .any(|gap| { gap.id == "mapper_banking_runtime" && gap.subsystem == "cartridge" }));
    assert_eq!(telemetry.analysis.failing_subsystem, None);
    assert_eq!(telemetry.analysis.failing_test, None);
    assert_eq!(telemetry.analysis.first_failure_domain, None);
    assert_eq!(
        telemetry.analysis.debug_focus.health,
        DiagnosticHealth::Healthy
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 28);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_test_name,
        Some("ppu_scroll_seam_matrix")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_domain, None);
    assert_eq!(telemetry.analysis.debug_focus.failure_kind, None);
    assert_eq!(telemetry.analysis.debug_focus.failed_probe_ids.len(), 0);
    assert_eq!(telemetry.analysis.debug_focus.skipped_probe_count, 0);
    assert!(telemetry
        .analysis
        .debug_focus
        .terminal_instruction
        .as_ref()
        .and_then(|instruction| instruction.instruction.as_deref())
        .is_some_and(|text| text.starts_with("JMP 0x")));
    assert_eq!(
        telemetry
            .analysis
            .debug_focus
            .terminal_instruction
            .as_ref()
            .and_then(|instruction| instruction.current_test_name),
        Some("ppu_scroll_seam_matrix")
    );
    assert_eq!(
        telemetry
            .analysis
            .debug_focus
            .terminal_instruction
            .as_ref()
            .and_then(|instruction| instruction.symbol.as_deref()),
        Some("hang")
    );
    assert!(telemetry.analysis.summary.contains("diagnostic passed"));
    assert!(telemetry
        .analysis
        .coverage
        .subsystem_summary
        .iter()
        .any(|entry| entry.subsystem == DiagnosticSubsystem::Cpu && entry.total == 6));
    assert!(telemetry
        .analysis
        .coverage
        .subsystem_summary
        .iter()
        .any(|entry| entry.subsystem == DiagnosticSubsystem::Ppu && entry.total == 10));
    assert!(telemetry
        .analysis
        .coverage
        .subsystem_summary
        .iter()
        .any(|entry| entry.subsystem == DiagnosticSubsystem::Joypad && entry.total == 6));
    assert!(telemetry
        .analysis
        .coverage
        .subsystem_summary
        .iter()
        .any(|entry| entry.subsystem == DiagnosticSubsystem::Dma && entry.total == 2));
    assert!(telemetry
        .analysis
        .coverage
        .subsystem_summary
        .iter()
        .any(|entry| entry.subsystem == DiagnosticSubsystem::Cartridge && entry.total == 2));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0x70
            && failure.test_name == Some("joypad_strobe_shift")
            && failure.likely_domain == "joypad.strobe_shift"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0xB0
            && failure.test_name == Some("cpu_zero_page_index_wrap")
            && failure.likely_domain == "cpu.addressing.zero_page_x_wrap"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0xC0
            && failure.test_name == Some("cpu_indirect_jmp_page_wrap")
            && failure.likely_domain == "cpu.control_flow.indirect_jmp_page_wrap"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0xB3
            && failure.test_name == Some("cpu_addressing_mode_matrix")
            && failure.likely_domain == "cpu.addressing.page_cross_load"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0xD0
            && failure.test_name == Some("ppu_vram_read_buffer")
            && failure.likely_domain == "ppu.registers.ppudata_buffer"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0xF1
            && failure.test_name == Some("mapper2_prg_bank_switch")
            && failure.likely_domain == "mapper.uxrom.prg_bank_switch"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0xF5
            && failure.test_name == Some("mapper2_prg_ram_roundtrip")
            && failure.likely_domain == "mapper.uxrom.prg_ram"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0xE0
            && failure.test_name == Some("ppu_horizontal_nametable_mirroring")
            && failure.likely_domain == "ppu.nametables.horizontal_mirroring"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0x78
            && failure.test_name == Some("joypad_strobe_reset_midstream")
            && failure.likely_domain == "joypad.strobe_reset"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0x7A
            && failure.test_name == Some("ppu_vram_increment_32")
            && failure.likely_domain == "ppu.registers.ppudata_increment_32"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0x7C
            && failure.test_name == Some("ppu_status_latch_reset")
            && failure.likely_domain == "ppu.registers.status_latch_reset"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0x7D
            && failure.test_name == Some("joypad_strobe_high_hold")
            && failure.likely_domain == "joypad.strobe_high_hold"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0x9B
            && failure.test_name == Some("input_port_serial_matrix")
            && failure.likely_domain == "joypad.input_port_matrix"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0x83
            && failure.test_name == Some("oam_dma_phase_matrix")
            && failure.likely_domain == "dma.oam_phase_matrix"
    }));
    assert!(telemetry.suite.failure_catalog.iter().any(|failure| {
        failure.code == 0x85
            && failure.test_name == Some("ppu_sprite_zero_hit")
            && failure.likely_domain == "ppu.sprite_zero_hit"
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
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "joypad2_strobe_shift" && test.intent.contains("joypad-2") && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "cpu_zero_page_index_wrap"
            && test.intent.contains("zero-page indexed")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "cpu_indirect_jmp_page_wrap" && test.intent.contains("JMP") && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "ppu_vram_read_buffer" && test.intent.contains("PPUDATA") && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "mapper2_prg_bank_switch" && test.intent.contains("Mapper 2") && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "mapper2_prg_ram_roundtrip" && test.intent.contains("PRG RAM") && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "ppu_horizontal_nametable_mirroring"
            && test.intent.contains("horizontal nametable")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "joypad_strobe_reset_midstream"
            && test.intent.contains("strobe-high/strobe-low")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "ppu_vram_increment_32" && test.intent.contains("PPUCTRL bit 2") && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "ppu_status_latch_reset"
            && test.intent.contains("PPUSTATUS resets")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "joypad_strobe_high_hold"
            && test.intent.contains("strobe is high")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "cpu_addressing_mode_matrix"
            && test.intent.contains("absolute,X")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "input_port_serial_matrix"
            && test.intent.contains("both input ports")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "oam_dma_phase_matrix"
            && test.intent.contains("odd and even start-phase")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "ppu_sprite_zero_hit"
            && test.intent.contains("sprite/background overlap")
            && test.passed
    }));
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "ppu_scroll_seam_matrix" && test.intent.contains("vertical") && test.passed
    }));
    assert!(telemetry.cpu_addressing_matrix.passed);
    assert_eq!(telemetry.cpu_addressing_matrix.observed_case_count, 3);
    assert_eq!(
        telemetry.cpu_addressing_matrix.abs_x_no_cross_result_hex,
        "0x34"
    );
    assert_eq!(
        telemetry.cpu_addressing_matrix.abs_x_page_cross_result_hex,
        "0x56"
    );
    assert_eq!(
        telemetry
            .cpu_addressing_matrix
            .indirect_y_page_cross_result_hex,
        "0x56"
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cpu.addressing_matrix.results"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "cpu.addressing.page_cross_load"
    }));
    assert!(telemetry.input_port_matrix.passed);
    assert_eq!(telemetry.input_port_matrix.observed_case_count, 24);
    assert_eq!(telemetry.input_port_matrix.joypad1_high_first_hex, "0x01");
    assert_eq!(telemetry.input_port_matrix.joypad1_high_second_hex, "0x01");
    assert_eq!(telemetry.input_port_matrix.joypad2_high_first_hex, "0x00");
    assert_eq!(telemetry.input_port_matrix.joypad2_high_second_hex, "0x00");
    assert_eq!(
        telemetry.input_port_matrix.joypad1_overread_first_hex,
        "0x01"
    );
    assert_eq!(
        telemetry.input_port_matrix.joypad1_overread_second_hex,
        "0x01"
    );
    assert_eq!(
        telemetry.input_port_matrix.joypad2_overread_first_hex,
        "0x01"
    );
    assert_eq!(
        telemetry.input_port_matrix.joypad2_overread_second_hex,
        "0x01"
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "joypad.input_port_matrix.results"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "joypad.input_port_matrix"
    }));
    assert!(telemetry.apu_status_matrix.passed);
    assert_eq!(telemetry.apu_status_matrix.expected_mask_hex, "0x0F");
    assert_eq!(telemetry.apu_status_matrix.observed_mask_hex, "0x0F");
    assert_eq!(telemetry.apu_status_matrix.expected_case_count, 4);
    assert_eq!(telemetry.apu_status_matrix.observed_case_count, 4);
    assert!(telemetry.apu_status_matrix.pulse1_status_bit);
    assert!(telemetry.apu_status_matrix.pulse2_status_bit);
    assert!(telemetry.apu_status_matrix.triangle_status_bit);
    assert!(telemetry.apu_status_matrix.noise_status_bit);
    assert!(telemetry.apu_dmc_status.passed);
    assert_eq!(telemetry.apu_dmc_status.expected_bit_hex, "0x10");
    assert_eq!(telemetry.apu_dmc_status.observed_bit_hex, "0x10");
    assert_eq!(telemetry.apu_dmc_status.expected_case_count, 1);
    assert_eq!(telemetry.apu_dmc_status.observed_case_count, 1);
    assert!(telemetry.apu_dmc_status.dmc_status_bit);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "apu.status_matrix"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "apu.status_matrix"
            && probe.observed.contains("mask 0x0F")
            && probe.observed.contains("cases 4/4")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "apu.dmc_status"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "apu.dmc_status"
            && probe.observed.contains("bit 0x10")
            && probe.observed.contains("cases 1/1")
    }));
    assert!(telemetry.ppu_vblank_timing.passed);
    assert_eq!(telemetry.ppu_vblank_timing.test_id, 10);
    assert_eq!(
        telemetry.ppu_vblank_timing.test_name,
        Some("ppu_nmi_and_render_frame")
    );
    assert_eq!(telemetry.ppu_vblank_timing.observed_nmi_count, 2);
    assert!(telemetry.ppu_vblank_timing.wait_loop_start_cycle.is_some());
    assert!(telemetry
        .ppu_vblank_timing
        .first_nmi_latency_cycles
        .is_some_and(|cycles| {
            (telemetry.ppu_vblank_timing.first_nmi_expected_min_cycles
                ..=telemetry.ppu_vblank_timing.first_nmi_expected_max_cycles)
                .contains(&cycles)
        }));
    assert!(telemetry
        .ppu_vblank_timing
        .inter_nmi_cycles
        .is_some_and(|cycles| {
            (telemetry.ppu_vblank_timing.inter_nmi_expected_min_cycles
                ..=telemetry.ppu_vblank_timing.inter_nmi_expected_max_cycles)
                .contains(&cycles)
        }));
    assert!(telemetry.ppu_vblank_timing.nmi_window_passed);
    assert!(telemetry.ppu_vblank_timing.edge_passed);
    assert_eq!(telemetry.ppu_vblank_timing.edge_expected_set_scanline, 241);
    assert_eq!(telemetry.ppu_vblank_timing.edge_expected_set_dot, 1);
    assert_eq!(telemetry.ppu_vblank_timing.edge_expected_clear_scanline, -1);
    assert_eq!(telemetry.ppu_vblank_timing.edge_expected_clear_dot, 1);
    assert_eq!(telemetry.ppu_vblank_timing.edge_set_count, 2);
    assert_eq!(telemetry.ppu_vblank_timing.edge_clear_count, 1);
    assert_eq!(telemetry.ppu_vblank_timing.edge_nmi_trigger_count, 2);
    assert_eq!(
        telemetry.ppu_vblank_timing.edge_first_set_ppu_scanline,
        Some(241)
    );
    assert_eq!(telemetry.ppu_vblank_timing.edge_first_set_ppu_dot, Some(1));
    assert!(telemetry
        .ppu_vblank_timing
        .edge_first_set_cpu_cycle
        .is_some());
    assert_eq!(
        telemetry.ppu_vblank_timing.edge_first_clear_ppu_scanline,
        Some(-1)
    );
    assert_eq!(
        telemetry.ppu_vblank_timing.edge_first_clear_ppu_dot,
        Some(1)
    );
    assert_eq!(
        telemetry.ppu_vblank_timing.edge_second_set_ppu_scanline,
        Some(241)
    );
    assert_eq!(telemetry.ppu_vblank_timing.edge_second_set_ppu_dot, Some(1));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.vblank_timing.nmi_window"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "ppu.vblank_timing"
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.vblank_timing.edge_dots"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "ppu.vblank_timing"
            && probe.observed.contains("first_set=241:1")
            && probe.observed.contains("first_clear=-1:1")
            && probe.observed.contains("second_set=241:1")
    }));
    assert_eq!(
        telemetry.frame.checksum,
        DIAGNOSTIC_RENDER_FRAME_EXPECTED_CHECKSUM
    );
    assert_eq!(
        telemetry.frame.expected_checksum,
        DIAGNOSTIC_RENDER_FRAME_EXPECTED_CHECKSUM
    );
    assert_eq!(telemetry.frame.checksum_hex, "0x00BDFEF60DAB16A5");
    assert_eq!(telemetry.frame.expected_checksum_hex, "0x00BDFEF60DAB16A5");
    assert!(telemetry.frame.checksum_matches_expected);
    assert!(telemetry.frame.checksum_validation_enabled);
    assert_eq!(
        telemetry.frame.checksum_validation_reason,
        "enabled: canonical default diagnostic fixture"
    );
    assert_eq!(
        telemetry.frame.unique_colors,
        DIAGNOSTIC_RENDER_FRAME_EXPECTED_UNIQUE_COLORS
    );
    assert_eq!(
        telemetry.frame.expected_unique_colors,
        DIAGNOSTIC_RENDER_FRAME_EXPECTED_UNIQUE_COLORS
    );
    assert!(telemetry.frame.unique_colors_match_expected);
    assert_eq!(
        telemetry.frame.nonzero_pixels,
        DIAGNOSTIC_RENDER_FRAME_EXPECTED_NONZERO_PIXELS
    );
    assert_eq!(
        telemetry.frame.expected_nonzero_pixels,
        DIAGNOSTIC_RENDER_FRAME_EXPECTED_NONZERO_PIXELS
    );
    assert!(telemetry.frame.nonzero_pixels_match_expected);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.frame_checksum"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "ppu.rendering.frame_signature"
            && probe.expected.contains("0x00BDFEF60DAB16A5")
            && probe.observed.contains("0x00BDFEF60DAB16A5")
    }));
    assert!(telemetry.ppu_sprite_zero_hit.passed);
    assert_eq!(
        telemetry.ppu_sprite_zero_hit.observed_status_bit_hex,
        "0x40"
    );
    assert_eq!(telemetry.ppu_sprite_zero_hit.observed_case_count, 1);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.sprite_zero_hit.status"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "ppu.sprite_zero_hit"
    }));
    assert!(telemetry.ppu_sprite_overflow.passed);
    assert_eq!(
        telemetry.ppu_sprite_overflow.observed_status_bit_hex,
        "0x20"
    );
    assert_eq!(
        telemetry
            .ppu_sprite_overflow
            .false_positive_observed_status_bit_hex,
        "0x20"
    );
    assert_eq!(
        telemetry
            .ppu_sprite_overflow
            .false_negative_observed_status_bit_hex,
        "0x00"
    );
    assert!(telemetry.ppu_sprite_overflow.hardware_bug_matrix_passed);
    assert_eq!(telemetry.ppu_sprite_overflow.observed_case_count, 3);
    assert_eq!(telemetry.ppu_sprite_overflow.restored_oam_byte_count, 256);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.sprite_overflow.status"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "ppu.sprite_overflow"
            && probe.observed.contains("false_positive 0x20")
            && probe.observed.contains("false_negative 0x00")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.sprite_overflow.hardware_bug_matrix"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "ppu.sprite_overflow.hardware_bug"
            && probe.expected.contains("false_negative=0x00")
            && probe.observed.contains("true_positive=0x20")
            && probe.observed.contains("false_positive=0x20")
            && probe.observed.contains("false_negative=0x00")
    }));
    assert!(telemetry.ppu_sprite_priority.passed);
    assert_eq!(telemetry.ppu_sprite_priority.observed_case_count, 2);
    assert_eq!(
        telemetry.ppu_sprite_priority.front_observed_color_hex,
        "0xB53120"
    );
    assert_eq!(
        telemetry.ppu_sprite_priority.behind_observed_color_hex,
        "0x64B0FF"
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.sprite_priority.samples"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "ppu.sprite_priority"
    }));
    assert!(telemetry.ppu_scroll_seam.passed);
    assert_eq!(telemetry.ppu_scroll_seam.observed_case_count, 6);
    assert_eq!(telemetry.ppu_scroll_seam.scroll_x, 4);
    assert_eq!(telemetry.ppu_scroll_seam.coarse_scroll_x, 8);
    assert_eq!(telemetry.ppu_scroll_seam.scroll_y, 4);
    assert_eq!(
        telemetry.ppu_scroll_seam.left_observed_color_hex,
        "0x64B0FF"
    );
    assert_eq!(
        telemetry.ppu_scroll_seam.right_observed_color_hex,
        "0xB53120"
    );
    assert_eq!(
        telemetry.ppu_scroll_seam.coarse_left_observed_color_hex,
        "0xB53120"
    );
    assert_eq!(
        telemetry.ppu_scroll_seam.coarse_right_observed_color_hex,
        "0x64B0FF"
    );
    assert_eq!(
        telemetry.ppu_scroll_seam.nametable_wrap_mirroring,
        "vertical"
    );
    assert_eq!(telemetry.ppu_scroll_seam.nametable_wrap_scroll_x, 248);
    assert_eq!(telemetry.ppu_scroll_seam.nametable_wrap_scroll_y, 0);
    assert_eq!(
        telemetry
            .ppu_scroll_seam
            .nametable_wrap_left_observed_color_hex,
        "0xB53120"
    );
    assert_eq!(
        telemetry
            .ppu_scroll_seam
            .nametable_wrap_right_observed_color_hex,
        "0x64B0FF"
    );
    assert!(telemetry.ppu_scroll_seam.nametable_wrap_frames >= 2);
    assert!(telemetry.ppu_scroll_seam.nametable_wrap_cycles > 0);
    assert!(telemetry.ppu_scroll_seam.nametable_wrap_passed);
    assert!(telemetry.ppu_scroll_seam.nametable_wrap_error.is_none());
    assert_eq!(telemetry.ppu_scroll_seam.top_observed_color_hex, "0x64B0FF");
    assert_eq!(
        telemetry.ppu_scroll_seam.bottom_observed_color_hex,
        "0xB53120"
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.scroll_seam.samples"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "ppu.scroll_seam"
    }));
    assert_eq!(telemetry.input.joypad1_expected_mask_hex, "0x81");
    assert_eq!(telemetry.input.joypad2_mask_hex, "0x28");
    assert_eq!(telemetry.input.joypad2_expected_mask_hex, "0x28");
    assert_eq!(telemetry.input.fault_injection, None);
    assert_eq!(telemetry.input.fault_injection_label, None);
    assert!(telemetry.dma.oam_dma_observed);
    assert!(telemetry.dma.oam_dma_completed);
    assert!((513..=514).contains(&telemetry.dma.oam_dma_active_cycles));
    assert!(matches!(
        telemetry.dma.oam_dma_first_active_cycle_parity,
        Some("even") | Some("odd")
    ));
    assert_eq!(telemetry.dma.oam_dma_start_test, Some(5));
    assert_eq!(
        telemetry.dma.oam_dma_start_test_name,
        Some("oam_dma_transfer")
    );
    assert_eq!(telemetry.dma.oam_dma_transfer_count, 3);
    assert_eq!(telemetry.dma.oam_dma_phase_matrix_test_transfer_count, 2);
    assert_eq!(
        telemetry.dma.oam_dma_phase_matrix_expected_test_transfers,
        2
    );
    assert!(telemetry.dma.oam_dma_phase_matrix_has_even_start);
    assert!(telemetry.dma.oam_dma_phase_matrix_has_odd_start);
    assert!(telemetry.dma.oam_dma_phase_matrix_passed);
    assert_eq!(telemetry.dma.oam_dma_active_cycle_buckets.len(), 3);
    assert!(telemetry
        .dma
        .oam_dma_active_cycle_buckets
        .iter()
        .all(|cycles| (513..=514).contains(cycles)));
    assert!(telemetry.dma.dmc_dma_fetches_observed >= 2);
    assert!(telemetry.dma.dmc_dma_fetches_during_oam_dma >= 1);
    assert!(telemetry.dma.dmc_dma_oam_overlap_observed);
    assert_eq!(
        telemetry.dma.dmc_dma_three_cycle_fetches + telemetry.dma.dmc_dma_four_cycle_fetches,
        telemetry.dma.dmc_dma_fetches_observed
    );
    assert!(telemetry
        .dma
        .dmc_dma_first_fetch_stall_cycles
        .is_some_and(|cycles| (3..=4).contains(&cycles)));
    assert!(telemetry
        .dma
        .dmc_dma_first_oam_overlap_stall_cycles
        .is_some_and(|cycles| (3..=4).contains(&cycles)));
    assert_eq!(telemetry.dma.dmc_dma_first_oam_overlap_test, Some(5));
    assert_eq!(
        telemetry.dma.dmc_dma_first_oam_overlap_test_name,
        Some("oam_dma_transfer")
    );
    assert_eq!(
        telemetry.dma.dmc_dma_stall_cycles_after_oam_dma,
        telemetry
            .dma
            .dmc_dma_first_oam_overlap_stall_cycles
            .expect("overlap stall bucket should be observed") as u64
    );
    assert!(telemetry.audio.sample_count_passed);
    assert!((telemetry.audio.expected_min_sample_count
        ..=telemetry.audio.expected_max_sample_count)
        .contains(&telemetry.audio.sample_count));
    assert!(telemetry.audio.peak_abs_passed);
    assert!(telemetry.audio.rms_abs_passed);
    assert!(telemetry.audio.mean_abs_passed);
    assert!(telemetry.audio.passed);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "apu.sample_count"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "apu.frame_output"
            && probe.expected.contains("12000..=13000")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "apu.output_envelope"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "apu.output_envelope"
            && probe.observed.contains("peak")
            && probe.observed.contains("rms")
            && probe.observed.contains("mean")
    }));
    assert_eq!(telemetry.instruction_trace.retention_limit, 64);
    assert_eq!(
        telemetry.instruction_trace.retained_instruction_count,
        telemetry.instruction_trace.tail.len()
    );
    assert!(telemetry.instruction_trace.captured_instruction_count > telemetry.events.len() as u64);
    assert!(telemetry.instruction_trace.truncated);
    assert!(telemetry
        .instruction_trace
        .tail
        .iter()
        .all(|entry| entry.cpu.pc == entry.pc));
    assert!(telemetry
        .instruction_trace
        .tail
        .iter()
        .any(|entry| entry.diagnostic_ram.signature_hex == "0xA5"));
    let last_instruction = telemetry
        .instruction_trace
        .tail
        .last()
        .expect("diagnostic should retain an instruction trace tail");
    assert!(last_instruction.sequence <= telemetry.instruction_trace.captured_instruction_count);
    assert!(last_instruction.pc >= 0x8000);
    assert!(last_instruction.opcode_hex.is_some());
    assert!(last_instruction
        .instruction
        .as_ref()
        .is_some_and(|instruction| instruction.mnemonic == "JMP"
            && instruction.addressing_mode == "absolute"
            && instruction.text.starts_with("JMP ")));
    assert!(last_instruction.symbol.as_ref().is_some_and(|symbol| {
        symbol.name == "hang" && symbol.offset == 0 && symbol.address_hex == last_instruction.pc_hex
    }));
    assert!(telemetry
        .events
        .iter()
        .any(|event| event.current_test_name == Some("joypad_overread_returns_one")));
    assert!(telemetry
        .events
        .iter()
        .any(|event| event.note == "oam_dma_started"
            && event.current_test_name == Some("oam_dma_transfer")));
    assert!(telemetry
        .events
        .iter()
        .any(|event| event.note == "oam_dma_completed"
            && event.current_test_name == Some("oam_dma_transfer")));
    assert!(telemetry
        .events
        .iter()
        .any(|event| event.note == "dmc_dma_oam_overlap"
            && event.current_test_name == Some("oam_dma_transfer")));
    assert!(telemetry
        .events
        .iter()
        .any(|event| event.note == "oam_dma_started"
            && event.current_test_name == Some("oam_dma_phase_matrix")));
    assert!(telemetry
        .events
        .iter()
        .any(|event| event.note == "oam_dma_completed"
            && event.current_test_name == Some("oam_dma_phase_matrix")));
    assert!(telemetry
        .events
        .iter()
        .any(|event| event.current_test_name == Some("joypad2_strobe_shift")));
    assert!(telemetry
        .events
        .iter()
        .all(|event| event.cpu.pc == event.pc));
    assert!(telemetry.events.iter().any(|event| {
        event.current_test_name == Some("oam_dma_transfer")
            && event.diagnostic_ram.signature == 0xA5
            && event.diagnostic_ram.current_result_addr == Some(0x0204)
    }));
    let final_event = telemetry
        .events
        .last()
        .expect("diagnostic should emit at least one event");
    assert_eq!(final_event.diagnostic_ram.failure_code_hex, "0x00");
    assert_eq!(
        final_event.diagnostic_ram.current_result_hex.as_deref(),
        Some("0x01")
    );
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
    assert_eq!(
        telemetry.analysis.probe_summary.total_probes,
        telemetry.probes.len()
    );
    assert!(telemetry.analysis.probe_summary.total_probes > DIAGNOSTIC_TESTS.len());
    assert_eq!(
        telemetry.analysis.probe_summary.passed_probes,
        telemetry.analysis.probe_summary.total_probes
    );
    assert_eq!(telemetry.analysis.probe_summary.failed_probes, 0);
    assert_eq!(telemetry.analysis.probe_summary.skipped_probes, 0);
    assert!(telemetry
        .probes
        .iter()
        .all(|probe| probe.status == DiagnosticProbeStatus::Passed));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ram.signature"
            && probe.expected.contains("0xA5")
            && probe.observed.contains("0xA5")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.10.result"
            && probe.test_name == Some("ppu_nmi_and_render_frame")
            && probe.status == DiagnosticProbeStatus::Passed
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.vblank_timing.nmi_window"
            && probe.test_name == Some("ppu_nmi_and_render_frame")
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.observed.contains("inter_nmi=")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.vblank_timing.edge_dots"
            && probe.test_name == Some("ppu_nmi_and_render_frame")
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.expected.contains("scanline 241 dot 1")
            && probe.observed.contains("first_set=241:1")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "dma.oam_active_cycles"
            && probe.test_name == Some("oam_dma_transfer")
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.observed.contains("active cycles")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "dma.oam_phase_matrix"
            && probe.test_name == Some("oam_dma_phase_matrix")
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.observed.contains("phase transfers")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "dma.dmc_oam_overlap"
            && probe.test_name == Some("oam_dma_transfer")
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.observed.contains("overlapping fetches")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "dma.dmc_stall_phase"
            && probe.test_name == Some("oam_dma_transfer")
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.observed.contains("first overlap bucket")
    }));
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
    assert!(report.contains("## Input Configuration"));
    assert!(report.contains("| Joypad 2 mask / expected | 0x28 / 0x28 |"));
    assert!(report.contains("## Debug Focus"));
    assert!(report.contains("| Focus test | ppu_scroll_seam_matrix (28) |"));
    assert!(report.contains("| Terminal instruction | seq "));
    assert!(report.contains("## Coverage"));
    assert!(report.contains("## Known Coverage Gaps"));
    assert!(report.contains("| mapper_banking_runtime | cartridge |"));
    assert!(report.contains("## PPU Pixel Pipeline"));
    assert!(report.contains("| First NMI cycle / latency |"));
    assert!(report.contains("| Second NMI cycle / inter-NMI cycles |"));
    assert!(report.contains(
        "| Vblank edge expected set/clear dots | set scanline 241 dot 1, clear scanline -1 dot 1 |"
    ));
    assert!(report.contains("| Vblank edge observed counts | set 2, clear 1, NMI triggers 2 |"));
    assert!(report.contains("| Vblank first set edge CPU/frame/PPU/phase |"));
    assert!(report.contains("/ 241:1 /"));
    assert!(report.contains("| Vblank first clear edge CPU/frame/PPU/phase |"));
    assert!(report.contains("/ -1:1 /"));
    assert!(report.contains("| Vblank second set edge CPU/frame/PPU/phase |"));
    assert!(report.contains("| Vblank timing passed | true |"));
    assert!(report.contains(
        "| Render-frame checksum / expected | 0x00BDFEF60DAB16A5 / 0x00BDFEF60DAB16A5 |"
    ));
    assert!(report.contains("| Render-frame checksum passed | true |"));
    assert!(report.contains(
        "| Render-frame checksum validation | enabled: canonical default diagnostic fixture |"
    ));
    assert!(report.contains("| Render-frame colors / expected | 3 / 3 |"));
    assert!(report.contains("| Render-frame nonzero pixels / expected | 61440 / 61440 |"));
    assert!(report.contains("| Sprite-zero-hit status bit / expected | 0x40 / 0x40 |"));
    assert!(report.contains("| Sprite-overflow status bit / expected | 0x20 / 0x20 |"));
    assert!(report.contains("| Sprite-overflow false-positive bit / expected | 0x20 / 0x20 |"));
    assert!(report.contains("| Sprite-overflow false-negative bit / expected | 0x00 / 0x00 |"));
    assert!(report.contains("| Sprite-overflow cases / expected | 3 / 3 |"));
    assert!(report.contains("| Sprite-overflow restored OAM bytes | 256 |"));
    assert!(report.contains("| Sprite-overflow hardware-bug matrix passed | true |"));
    assert!(report
        .contains("| Sprite-priority front sample / expected | (18, 18) 0xB53120 / 0xB53120 |"));
    assert!(report
        .contains("| Sprite-priority behind sample / expected | (42, 18) 0x64B0FF / 0x64B0FF |"));
    assert!(report.contains("| Sprite-priority cases / expected | 2 / 2 |"));
    assert!(report.contains("| Scroll-seam left sample / expected | (2, 18) 0x64B0FF / 0x64B0FF |"));
    assert!(
        report.contains("| Scroll-seam right sample / expected | (10, 18) 0xB53120 / 0xB53120 |")
    );
    assert!(report
        .contains("| Scroll-seam coarse-left sample / expected | (2, 18) 0xB53120 / 0xB53120 |"));
    assert!(report
        .contains("| Scroll-seam coarse-right sample / expected | (10, 18) 0x64B0FF / 0x64B0FF |"));
    assert!(report.contains(
        "| Scroll-seam nametable-wrap left sample / expected | (2, 18) 0xB53120 / 0xB53120 |"
    ));
    assert!(report.contains(
        "| Scroll-seam nametable-wrap right sample / expected | (10, 18) 0x64B0FF / 0x64B0FF |"
    ));
    assert!(
        report.contains("| Scroll-seam nametable-wrap scroll X/Y/mirroring | 248 / 0 / vertical |")
    );
    assert!(report.contains("| Scroll-seam nametable-wrap error | none |"));
    assert!(report.contains("| Scroll-seam top sample / expected | (2, 12) 0x64B0FF / 0x64B0FF |"));
    assert!(
        report.contains("| Scroll-seam bottom sample / expected | (2, 20) 0xB53120 / 0xB53120 |")
    );
    assert!(report.contains("| Scroll-seam cases / expected | 6 / 6 |"));
    assert!(report.contains("## DMA Timing"));
    assert!(report.contains("| OAM DMA completed | true |"));
    assert!(report.contains("| Active cycles / expected |"));
    assert!(report.contains("| Transfer count / total active cycles | 3 /"));
    assert!(report.contains("| Phase matrix transfers / expected | 2 / 2 |"));
    assert!(report.contains("| Phase matrix parity coverage | even=true odd=true"));
    assert!(report.contains("| DMC fetches / overlapping fetches |"));
    assert!(report.contains("| DMC overlap test | oam_dma_transfer |"));
    assert!(report.contains("| DMC overlap parity / stall bucket |"));
    assert!(report.contains("| DMC 3-cycle / 4-cycle fetches |"));
    assert!(report.contains("## APU Audio Output"));
    assert!(report.contains("| Sample count / expected |"));
    assert!(report.contains("| Peak abs / expected |"));
    assert!(report.contains("| RMS abs / expected |"));
    assert!(report.contains("| Mean abs / expected |"));
    assert!(report.contains(
        "| Audio envelope passed | sample_count=true peak=true rms=true mean=true overall=true |"
    ));
    assert!(report.contains("| DMC status bit / expected | 0x10 / 0x10 |"));
    assert!(report.contains("| DMC status active / cases / passed | true / 1/1 / true |"));
    assert!(report.contains("## Timing"));
    assert!(report.contains("## Observation Probes"));
    assert!(report.contains("| Passed probes |"));
    assert!(report.contains("| passed | ram.signature | host_observation | bus | none |"));
    assert!(report.contains("| Slowest test |"));
    assert!(report.contains("| 10 | ppu_nmi_and_render_frame | ppu | integration | passed |"));
    assert!(report.contains("| 11 | joypad2_strobe_shift | joypad | integration | passed |"));
    assert!(report.contains("| 12 | cpu_zero_page_index_wrap | cpu | edge_case | passed |"));
    assert!(report.contains("| 13 | cpu_indirect_jmp_page_wrap | cpu | edge_case | passed |"));
    assert!(report.contains("| 14 | ppu_vram_read_buffer | ppu | edge_case | passed |"));
    assert!(report.contains("| 15 | mapper2_prg_bank_switch | cartridge | integration | passed |"));
    assert!(
        report.contains("| 16 | mapper2_prg_ram_roundtrip | cartridge | integration | passed |")
    );
    assert!(
        report.contains("| 17 | ppu_horizontal_nametable_mirroring | ppu | integration | passed |")
    );
    assert!(report.contains("| 18 | joypad_strobe_reset_midstream | joypad | edge_case | passed |"));
    assert!(report.contains("| 19 | ppu_vram_increment_32 | ppu | edge_case | passed |"));
    assert!(report.contains("| 20 | ppu_status_latch_reset | ppu | edge_case | passed |"));
    assert!(report.contains("| 21 | joypad_strobe_high_hold | joypad | edge_case | passed |"));
    assert!(report.contains("| 22 | cpu_addressing_mode_matrix | cpu | edge_case | passed |"));
    assert!(report.contains("| 23 | input_port_serial_matrix | joypad | edge_case | passed |"));
    assert!(report.contains("| 24 | oam_dma_phase_matrix | dma | edge_case | passed |"));
    assert!(report.contains("| 25 | ppu_sprite_zero_hit | ppu | edge_case | passed |"));
    assert!(report.contains("| 26 | ppu_sprite_overflow | ppu | edge_case | passed |"));
    assert!(report.contains("| 27 | ppu_sprite_priority_mux | ppu | edge_case | passed |"));
    assert!(report.contains("| 28 | ppu_scroll_seam_matrix | ppu | edge_case | passed |"));
    assert!(report.contains("## Instruction Trace Tail"));
    assert!(report.contains(
        "| Seq | Cycle | Frame | Test | PC | Instruction | Symbol | CPU A/X/Y | SP/P | Result |"
    ));
    assert!(report.contains("| JMP 0x"));
    assert!(report.contains("| hang |"));
    assert!(report.contains("## Event Tail"));
    assert!(report.contains("| CPU A/X/Y | SP/P | Result | Failure |"));
    assert!(telemetry.cycles > 0);
    assert!(telemetry.frames >= 2);
}

#[test]
fn generated_diagnostic_cartridge_runs_configured_input_mask_matrix_to_pass() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        joypad1_mask: 0xAA,
        expected_joypad1_mask: 0xAA,
        joypad2_mask: 0x55,
        expected_joypad2_mask: 0x55,
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run with alternate expected input masks");

    assert!(telemetry.verdict.passed);
    assert_eq!(telemetry.analysis.health, DiagnosticHealth::Healthy);
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 28);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_test_name,
        Some("ppu_scroll_seam_matrix")
    );
    assert_eq!(telemetry.input.joypad1_mask_hex, "0xAA");
    assert_eq!(telemetry.input.joypad1_expected_mask_hex, "0xAA");
    assert_eq!(telemetry.input.joypad2_mask_hex, "0x55");
    assert_eq!(telemetry.input.joypad2_expected_mask_hex, "0x55");
    assert_eq!(telemetry.frame.checksum_hex, "0x89D7CBABF407D845");
    assert!(!telemetry.frame.checksum_matches_expected);
    assert!(!telemetry.frame.checksum_validation_enabled);
    assert_eq!(
        telemetry.frame.checksum_validation_reason,
        "disabled: non-default input timing fixture"
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.frame_checksum"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.likely_domain == "ppu.rendering.frame_signature"
            && probe.expected.contains("validation disabled")
            && probe.observed.contains("0x89D7CBABF407D845")
    }));
    assert!(telemetry.input_port_matrix.passed);
    assert_eq!(telemetry.input_port_matrix.joypad1_high_first_hex, "0x00");
    assert_eq!(telemetry.input_port_matrix.joypad2_high_first_hex, "0x01");
    assert_eq!(
        telemetry.analysis.coverage.passed_tests,
        DIAGNOSTIC_TESTS.len()
    );
    assert!(telemetry.tests.iter().any(|test| {
        test.name == "joypad_strobe_shift"
            && test
                .expected_observations
                .iter()
                .any(|observation| observation.contains("configured joypad-1 expected mask"))
            && test.passed
    }));
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
    assert_eq!(
        telemetry.analysis.debug_focus.health,
        DiagnosticHealth::CartridgeAssertionFailed
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 7);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_test_name,
        Some("joypad_strobe_shift")
    );
    assert_eq!(
        telemetry.analysis.debug_focus.focus_subsystem,
        Some(DiagnosticSubsystem::Joypad)
    );
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("joypad.strobe_shift")
    );
    assert_eq!(
        telemetry.analysis.debug_focus.failure_kind,
        Some(DiagnosticFailureKind::CartridgeAssertion)
    );
    assert_eq!(telemetry.analysis.debug_focus.failure_code_hex, "0x70");
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .iter()
        .any(|id| id == "cartridge.status.pass"));
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .iter()
        .any(|id| id == "cartridge.test.7.result"));
    assert!(telemetry.analysis.debug_focus.skipped_probe_count > 0);
    assert_eq!(
        telemetry
            .analysis
            .debug_focus
            .last_test_instruction
            .as_ref()
            .and_then(|instruction| instruction.current_test_name),
        Some("joypad_strobe_shift")
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
    assert_eq!(
        telemetry
            .analysis
            .probe_summary
            .first_failed_probe
            .as_deref(),
        Some("cartridge.status.pass")
    );
    assert_eq!(telemetry.analysis.probe_summary.failed_probes, 2);
    assert!(telemetry.analysis.probe_summary.skipped_probes > 0);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.7.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(7)
            && probe.likely_domain == "joypad.strobe_shift"
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.8.result" && probe.status == DiagnosticProbeStatus::Skipped
    }));
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| {
        entry
            .symbol
            .as_ref()
            .is_some_and(|symbol| symbol.name.starts_with("test_07_joypad_strobe_shift"))
    }));

    assert_eq!(telemetry.analysis.timing.started_tests, 7);
    assert_eq!(telemetry.analysis.timing.ended_tests, 7);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
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
    assert!(report.contains("## Debug Focus"));
    assert!(report.contains("| Focus test | joypad_strobe_shift (7) |"));
    assert!(report.contains("| Focus domain | joypad.strobe_shift |"));
    assert!(report.contains("cartridge.test.7.result"));
    assert!(report.contains("## Failure Localization"));
    assert!(report.contains("| Likely domain | joypad.strobe_shift |"));
    assert!(report.contains("| Remediation hint | Inspect joypad strobe"));
    assert!(report.contains("| 7 | joypad_strobe_shift | joypad | smoke | failed |"));
    assert!(report.contains("| 8 | cpu_branch_page_crossing | cpu | edge_case | not_started |"));
    assert!(report.contains(
        "| failed | cartridge.test.7.result | cartridge_result | joypad | joypad_strobe_shift |"
    ));
    assert!(report.contains(
        "| skipped | cartridge.test.8.result | cartridge_result | cpu | cpu_branch_page_crossing |"
    ));
    assert!(report.contains("## Host Failures"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_joypad2_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        joypad2_mask: 0x00,
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported player-2 failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(telemetry.verdict.current_test, 11);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("joypad2_strobe_shift")
    );
    assert_eq!(telemetry.verdict.failure_code, 0xA3);
    assert_eq!(telemetry.input.joypad2_mask_hex, "0x00");
    assert_eq!(telemetry.input.joypad2_expected_mask_hex, "0x28");

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include player-2 failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 11);
    assert_eq!(failure.test_name, Some("joypad2_strobe_shift"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Joypad));
    assert_eq!(failure.failure_code_hex, "0xA3");
    assert_eq!(failure.likely_domain, "joypad2.strobe_shift");
    assert!(failure.assertion.contains("Start button"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Joypad)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("joypad2_strobe_shift")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("joypad2.strobe_shift")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 11);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("joypad2.strobe_shift")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.11.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(11)
            && probe.likely_domain == "joypad2.strobe_shift"
    }));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_dma_oam_transfer_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::DmaOamTransfer),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported DMA host-observation failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("dma_oam_transfer")
    );
    assert_eq!(telemetry.verdict.status, 0x80);
    assert_eq!(telemetry.verdict.current_test, 28);
    assert_eq!(telemetry.verdict.failure_code, 0x00);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("host-observation failure should include failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::HostValidation);
    assert_eq!(failure.test_id, 5);
    assert_eq!(failure.test_name, Some("oam_dma_transfer"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Dma));
    assert_eq!(failure.likely_domain, "dma.oam_transfer");
    assert!(failure.assertion.contains("PPU OAM contents"));
    assert!(failure.expected.contains("OAM checksum"));
    assert!(failure.observed.contains("OAM checksum"));

    assert_eq!(
        telemetry.analysis.health,
        DiagnosticHealth::HostValidationFailed
    );
    assert_eq!(telemetry.analysis.failing_test, Some("oam_dma_transfer"));
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("dma.oam_transfer")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 5);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_test_name,
        Some("oam_dma_transfer")
    );
    assert_eq!(
        telemetry.analysis.debug_focus.focus_subsystem,
        Some(DiagnosticSubsystem::Dma)
    );
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("dma.oam_transfer")
    );
    assert_eq!(
        telemetry.analysis.debug_focus.failure_kind,
        Some(DiagnosticFailureKind::HostValidation)
    );
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .contains(&"oam.dma_checksum".to_string()));

    assert_eq!(
        telemetry.analysis.coverage.passed_tests,
        DIAGNOSTIC_TESTS.len()
    );
    assert!(telemetry.dma.oam_dma_completed);
    assert!((telemetry.dma.oam_dma_expected_min_cycles
        ..=telemetry.dma.oam_dma_expected_max_cycles)
        .contains(&telemetry.dma.oam_dma_active_cycles));
    assert_ne!(telemetry.oam.checksum, telemetry.oam.expected_checksum);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "oam.dma_checksum"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(5)
            && probe.likely_domain == "dma.oam_transfer"
    }));
    assert!(telemetry
        .verdict
        .host_failures
        .iter()
        .any(|failure| failure.contains("OAM DMA checksum mismatch")));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_dma_phase_matrix_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::DmaPhaseMatrix),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported DMA phase-matrix failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("dma_phase_matrix")
    );
    assert_eq!(telemetry.verdict.current_test, 24);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("oam_dma_phase_matrix")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x83);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include DMA phase-matrix localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 24);
    assert_eq!(failure.test_name, Some("oam_dma_phase_matrix"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Dma));
    assert_eq!(failure.failure_code_hex, "0x83");
    assert_eq!(failure.likely_domain, "dma.oam_phase_matrix");
    assert!(failure
        .assertion
        .contains("second cartridge-triggered transfer"));

    assert_eq!(
        telemetry.analysis.health,
        DiagnosticHealth::CartridgeAssertionFailed
    );
    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Dma)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("oam_dma_phase_matrix")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("dma.oam_phase_matrix")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 24);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("dma.oam_phase_matrix")
    );
    assert_eq!(telemetry.dma.oam_dma_phase_matrix_test_transfer_count, 1);
    assert!(!telemetry.dma.oam_dma_phase_matrix_passed);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.24.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(24)
            && probe.likely_domain == "dma.oam_phase_matrix"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 24);
    assert_eq!(telemetry.analysis.timing.ended_tests, 24);
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "oam_dma_phase_matrix_before_second_dma")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | oam_dma_phase_matrix (24) |"));
    assert!(report.contains("| Focus domain | dma.oam_phase_matrix |"));
    assert!(report.contains("| Likely domain | dma.oam_phase_matrix |"));
    assert!(report.contains("| 24 | oam_dma_phase_matrix | dma | edge_case | failed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_ppu_sprite_zero_hit_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::PpuSpriteZeroHit),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported PPU sprite-zero-hit failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("ppu_sprite_zero_hit")
    );
    assert_eq!(telemetry.verdict.current_test, 25);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("ppu_sprite_zero_hit")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x85);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include PPU sprite-zero-hit localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 25);
    assert_eq!(failure.test_name, Some("ppu_sprite_zero_hit"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Ppu));
    assert_eq!(failure.failure_code_hex, "0x85");
    assert_eq!(failure.likely_domain, "ppu.sprite_zero_hit");
    assert!(failure.assertion.contains("sprite-zero-hit"));

    assert_eq!(
        telemetry.analysis.health,
        DiagnosticHealth::CartridgeAssertionFailed
    );
    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(telemetry.analysis.failing_test, Some("ppu_sprite_zero_hit"));
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("ppu.sprite_zero_hit")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 25);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("ppu.sprite_zero_hit")
    );
    assert_eq!(
        telemetry.ppu_sprite_zero_hit.observed_status_bit_hex,
        "0x00"
    );
    assert_eq!(telemetry.ppu_sprite_zero_hit.observed_case_count, 0);
    assert!(!telemetry.ppu_sprite_zero_hit.passed);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.25.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(25)
            && probe.likely_domain == "ppu.sprite_zero_hit"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 25);
    assert_eq!(telemetry.analysis.timing.ended_tests, 25);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | ppu_sprite_zero_hit (25) |"));
    assert!(report.contains("| Focus domain | ppu.sprite_zero_hit |"));
    assert!(report.contains("| Likely domain | ppu.sprite_zero_hit |"));
    assert!(report.contains("| Sprite-zero-hit status bit / expected | 0x00 / 0x40 |"));
    assert!(report.contains("| 25 | ppu_sprite_zero_hit | ppu | edge_case | failed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_ppu_sprite_overflow_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::PpuSpriteOverflow),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported PPU sprite-overflow failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("ppu_sprite_overflow")
    );
    assert_eq!(telemetry.verdict.current_test, 26);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("ppu_sprite_overflow")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x86);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include PPU sprite-overflow localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 26);
    assert_eq!(failure.test_name, Some("ppu_sprite_overflow"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Ppu));
    assert_eq!(failure.failure_code_hex, "0x86");
    assert_eq!(failure.likely_domain, "ppu.sprite_overflow");
    assert!(failure.assertion.contains("sprite overflow"));

    assert_eq!(
        telemetry.analysis.health,
        DiagnosticHealth::CartridgeAssertionFailed
    );
    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(telemetry.analysis.failing_test, Some("ppu_sprite_overflow"));
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("ppu.sprite_overflow")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 26);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("ppu.sprite_overflow")
    );
    assert_eq!(
        telemetry.ppu_sprite_overflow.observed_status_bit_hex,
        "0x00"
    );
    assert_eq!(
        telemetry
            .ppu_sprite_overflow
            .false_positive_observed_status_bit_hex,
        "0x20"
    );
    assert_eq!(
        telemetry
            .ppu_sprite_overflow
            .false_negative_observed_status_bit_hex,
        "0x00"
    );
    assert_eq!(telemetry.ppu_sprite_overflow.observed_case_count, 0);
    assert!(!telemetry.ppu_sprite_overflow.hardware_bug_matrix_passed);
    assert!(!telemetry.ppu_sprite_overflow.passed);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.26.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(26)
            && probe.likely_domain == "ppu.sprite_overflow"
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.sprite_overflow.status"
            && probe.status == DiagnosticProbeStatus::Skipped
            && probe.test_id == Some(26)
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.sprite_overflow.hardware_bug_matrix"
            && probe.status == DiagnosticProbeStatus::Skipped
            && probe.test_id == Some(26)
    }));
    assert_eq!(telemetry.oam.checksum, telemetry.oam.expected_checksum);
    assert_eq!(telemetry.analysis.timing.started_tests, 26);
    assert_eq!(telemetry.analysis.timing.ended_tests, 26);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | ppu_sprite_overflow (26) |"));
    assert!(report.contains("| Focus domain | ppu.sprite_overflow |"));
    assert!(report.contains("| Likely domain | ppu.sprite_overflow |"));
    assert!(report.contains("| Sprite-overflow status bit / expected | 0x00 / 0x20 |"));
    assert!(report.contains("| Sprite-overflow false-positive bit / expected | 0x20 / 0x20 |"));
    assert!(report.contains("| Sprite-overflow false-negative bit / expected | 0x00 / 0x00 |"));
    assert!(report.contains("| Sprite-overflow cases / expected | 0 / 3 |"));
    assert!(report.contains("| Sprite-overflow hardware-bug matrix passed | false |"));
    assert!(report.contains("| 26 | ppu_sprite_overflow | ppu | edge_case | failed |"));
    assert!(report.contains("| 27 | ppu_sprite_priority_mux | ppu | edge_case | not_started |"));
    assert!(report.contains("| 28 | ppu_scroll_seam_matrix | ppu | edge_case | not_started |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_ppu_sprite_priority_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::PpuSpritePriority),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported PPU sprite-priority failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("ppu_sprite_priority")
    );
    assert_eq!(telemetry.verdict.status, 0x80);
    assert_eq!(telemetry.verdict.current_test, 28);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("ppu_scroll_seam_matrix")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x00);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include PPU sprite-priority localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::HostValidation);
    assert_eq!(failure.test_id, 27);
    assert_eq!(failure.test_name, Some("ppu_sprite_priority_mux"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Ppu));
    assert_eq!(failure.failure_code_hex, "0x00");
    assert_eq!(failure.likely_domain, "ppu.sprite_priority");
    assert!(failure.assertion.contains("frame pixels"));
    assert!(failure.expected.contains("front sample"));
    assert!(failure.observed.contains("front sample 0x64B0FF"));

    assert_eq!(
        telemetry.analysis.health,
        DiagnosticHealth::HostValidationFailed
    );
    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("ppu_sprite_priority_mux")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("ppu.sprite_priority")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 27);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("ppu.sprite_priority")
    );
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .contains(&"ppu.sprite_priority.samples".to_string()));
    assert_eq!(
        telemetry.ppu_sprite_priority.front_observed_color_hex,
        "0x64B0FF"
    );
    assert_eq!(
        telemetry.ppu_sprite_priority.behind_observed_color_hex,
        "0xB53120"
    );
    assert_eq!(telemetry.ppu_sprite_priority.observed_case_count, 2);
    assert!(!telemetry.ppu_sprite_priority.passed);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.sprite_priority.samples"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(27)
            && probe.likely_domain == "ppu.sprite_priority"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 28);
    assert_eq!(telemetry.analysis.timing.ended_tests, 28);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | ppu_sprite_priority_mux (27) |"));
    assert!(report.contains("| Focus domain | ppu.sprite_priority |"));
    assert!(report.contains("| Likely domain | ppu.sprite_priority |"));
    assert!(report
        .contains("| Sprite-priority front sample / expected | (18, 18) 0x64B0FF / 0xB53120 |"));
    assert!(report
        .contains("| Sprite-priority behind sample / expected | (42, 18) 0xB53120 / 0x64B0FF |"));
    assert!(report.contains("| 27 | ppu_sprite_priority_mux | ppu | edge_case | passed |"));
    assert!(report.contains("| 28 | ppu_scroll_seam_matrix | ppu | edge_case | passed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_ppu_scroll_seam_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::PpuScrollSeam),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported PPU scroll-seam failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("ppu_scroll_seam")
    );
    assert_eq!(telemetry.verdict.status, 0x80);
    assert_eq!(telemetry.verdict.current_test, 28);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("ppu_scroll_seam_matrix")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x00);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include PPU scroll-seam localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::HostValidation);
    assert_eq!(failure.test_id, 28);
    assert_eq!(failure.test_name, Some("ppu_scroll_seam_matrix"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Ppu));
    assert_eq!(failure.failure_code_hex, "0x00");
    assert_eq!(failure.likely_domain, "ppu.scroll_seam");
    assert!(failure
        .assertion
        .contains("fine-X, coarse-X, coarse-X nametable-wrap, and vertical scroll"));
    assert!(failure.expected.contains("right sample"));
    assert!(failure.observed.contains("right sample 0x64B0FF"));

    assert_eq!(
        telemetry.analysis.health,
        DiagnosticHealth::HostValidationFailed
    );
    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("ppu_scroll_seam_matrix")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("ppu.scroll_seam")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 28);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("ppu.scroll_seam")
    );
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .contains(&"ppu.scroll_seam.samples".to_string()));
    assert_eq!(
        telemetry.ppu_scroll_seam.left_observed_color_hex,
        "0x64B0FF"
    );
    assert_eq!(
        telemetry.ppu_scroll_seam.right_observed_color_hex,
        "0x64B0FF"
    );
    assert_eq!(
        telemetry.ppu_scroll_seam.right_expected_color_hex,
        "0xB53120"
    );
    assert_eq!(
        telemetry.ppu_scroll_seam.coarse_left_observed_color_hex,
        "0x64B0FF"
    );
    assert_eq!(
        telemetry.ppu_scroll_seam.coarse_left_expected_color_hex,
        "0xB53120"
    );
    assert!(telemetry.ppu_scroll_seam.nametable_wrap_passed);
    assert_eq!(
        telemetry
            .ppu_scroll_seam
            .nametable_wrap_left_observed_color_hex,
        "0xB53120"
    );
    assert_eq!(
        telemetry
            .ppu_scroll_seam
            .nametable_wrap_right_observed_color_hex,
        "0x64B0FF"
    );
    assert_eq!(telemetry.ppu_scroll_seam.observed_case_count, 6);
    assert!(!telemetry.ppu_scroll_seam.passed);
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.scroll_seam.samples"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(28)
            && probe.likely_domain == "ppu.scroll_seam"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 28);
    assert_eq!(telemetry.analysis.timing.ended_tests, 28);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | ppu_scroll_seam_matrix (28) |"));
    assert!(report.contains("| Focus domain | ppu.scroll_seam |"));
    assert!(report.contains("| Likely domain | ppu.scroll_seam |"));
    assert!(
        report.contains("| Scroll-seam right sample / expected | (10, 18) 0x64B0FF / 0xB53120 |")
    );
    assert!(report
        .contains("| Scroll-seam coarse-left sample / expected | (2, 18) 0x64B0FF / 0xB53120 |"));
    assert!(report.contains("| Status matrix mask / expected | 0x0F / 0x0F |"));
    assert!(report
        .contains("| Status matrix channels | pulse1=true pulse2=true triangle=true noise=true |"));
    assert!(report.contains("| Status matrix cases / expected | 4 / 4 |"));
    assert!(report.contains("| Status matrix passed | true |"));
    assert!(report.contains("| 28 | ppu_scroll_seam_matrix | ppu | edge_case | passed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_apu_status_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::ApuStatusRegister),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported APU failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("apu_status_register")
    );
    assert_eq!(telemetry.verdict.current_test, 6);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("apu_status_register")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x61);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include APU failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 6);
    assert_eq!(failure.test_name, Some("apu_status_register"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Apu));
    assert_eq!(failure.likely_domain, "apu.status");
    assert_eq!(
        telemetry.analysis.health,
        DiagnosticHealth::CartridgeAssertionFailed
    );
    assert_eq!(telemetry.analysis.failing_test, Some("apu_status_register"));
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("apu.status")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 6);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("apu.status")
    );
    assert_eq!(
        telemetry.analysis.debug_focus.failure_kind,
        Some(DiagnosticFailureKind::CartridgeAssertion)
    );
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .contains(&"cartridge.test.6.result".to_string()));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.6.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(6)
            && probe.likely_domain == "apu.status"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 6);
    assert_eq!(telemetry.analysis.timing.ended_tests, 6);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "apu_status_register_before_status_read")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | apu_status_register (6) |"));
    assert!(report.contains("| Focus domain | apu.status |"));
    assert!(report.contains("| Likely domain | apu.status |"));
    assert!(report.contains("| 6 | apu_status_register | apu | smoke | failed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_cpu_zero_page_wrap_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::CpuZeroPageIndexWrap),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported CPU failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("cpu_zero_page_index_wrap")
    );
    assert_eq!(telemetry.verdict.current_test, 12);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("cpu_zero_page_index_wrap")
    );
    assert_eq!(telemetry.verdict.failure_code, 0xB0);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include CPU failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 12);
    assert_eq!(failure.test_name, Some("cpu_zero_page_index_wrap"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Cpu));
    assert_eq!(failure.failure_code_hex, "0xB0");
    assert_eq!(failure.likely_domain, "cpu.addressing.zero_page_x_wrap");
    assert!(failure.assertion.contains("Zero-page indexed LDA"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Cpu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("cpu_zero_page_index_wrap")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("cpu.addressing.zero_page_x_wrap")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 12);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("cpu.addressing.zero_page_x_wrap")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.12.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(12)
            && probe.likely_domain == "cpu.addressing.zero_page_x_wrap"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 12);
    assert_eq!(telemetry.analysis.timing.ended_tests, 12);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "cpu_zero_page_index_wrap_before_read")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | cpu_zero_page_index_wrap (12) |"));
    assert!(report.contains("| Focus domain | cpu.addressing.zero_page_x_wrap |"));
    assert!(report.contains("| Likely domain | cpu.addressing.zero_page_x_wrap |"));
    assert!(report.contains("| 12 | cpu_zero_page_index_wrap | cpu | edge_case | failed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_cpu_indirect_jmp_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::CpuIndirectJmpPageWrap),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported CPU control-flow failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("cpu_indirect_jmp_page_wrap")
    );
    assert_eq!(telemetry.verdict.current_test, 13);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("cpu_indirect_jmp_page_wrap")
    );
    assert_eq!(telemetry.verdict.failure_code, 0xC0);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include CPU control-flow failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 13);
    assert_eq!(failure.test_name, Some("cpu_indirect_jmp_page_wrap"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Cpu));
    assert_eq!(failure.failure_code_hex, "0xC0");
    assert_eq!(
        failure.likely_domain,
        "cpu.control_flow.indirect_jmp_page_wrap"
    );
    assert!(failure.assertion.contains("Indirect JMP pointer"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Cpu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("cpu_indirect_jmp_page_wrap")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("cpu.control_flow.indirect_jmp_page_wrap")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 13);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("cpu.control_flow.indirect_jmp_page_wrap")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.13.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(13)
            && probe.likely_domain == "cpu.control_flow.indirect_jmp_page_wrap"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 13);
    assert_eq!(telemetry.analysis.timing.ended_tests, 13);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "cpu_indirect_jmp_page_wrap_before_jump")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | cpu_indirect_jmp_page_wrap (13) |"));
    assert!(report.contains("| Focus domain | cpu.control_flow.indirect_jmp_page_wrap |"));
    assert!(report.contains("| Likely domain | cpu.control_flow.indirect_jmp_page_wrap |"));
    assert!(report.contains("| 13 | cpu_indirect_jmp_page_wrap | cpu | edge_case | failed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_cpu_addressing_matrix_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::CpuAddressingModeMatrix),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported CPU addressing-matrix failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("cpu_addressing_mode_matrix")
    );
    assert_eq!(telemetry.verdict.current_test, 22);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("cpu_addressing_mode_matrix")
    );
    assert_eq!(telemetry.verdict.failure_code, 0xB3);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include CPU addressing-matrix localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 22);
    assert_eq!(failure.test_name, Some("cpu_addressing_mode_matrix"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Cpu));
    assert_eq!(failure.failure_code_hex, "0xB3");
    assert_eq!(failure.likely_domain, "cpu.addressing.page_cross_load");
    assert!(failure.assertion.contains("Absolute,X page-crossing"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Cpu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("cpu_addressing_mode_matrix")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("cpu.addressing.page_cross_load")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 22);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("cpu.addressing.page_cross_load")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.22.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(22)
            && probe.likely_domain == "cpu.addressing.page_cross_load"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 22);
    assert_eq!(telemetry.analysis.timing.ended_tests, 22);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "cpu_addressing_matrix_before_page_cross_read")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | cpu_addressing_mode_matrix (22) |"));
    assert!(report.contains("| Focus domain | cpu.addressing.page_cross_load |"));
    assert!(report.contains("| Likely domain | cpu.addressing.page_cross_load |"));
    assert!(report.contains("| 22 | cpu_addressing_mode_matrix | cpu | edge_case | failed |"));
    assert!(report.contains("| 23 | input_port_serial_matrix | joypad | edge_case | not_started |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_input_port_matrix_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::InputPortMatrix),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported input-port matrix failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("input_port_matrix")
    );
    assert_eq!(telemetry.verdict.current_test, 23);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("input_port_serial_matrix")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x9B);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include input-port matrix localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 23);
    assert_eq!(failure.test_name, Some("input_port_serial_matrix"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Joypad));
    assert_eq!(failure.failure_code_hex, "0x9B");
    assert_eq!(failure.likely_domain, "joypad.input_port_matrix");
    assert!(failure
        .assertion
        .contains("Joypad 2 input-port matrix serial reads"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Joypad)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("input_port_serial_matrix")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("joypad.input_port_matrix")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 23);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("joypad.input_port_matrix")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.23.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(23)
            && probe.likely_domain == "joypad.input_port_matrix"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 23);
    assert_eq!(telemetry.analysis.timing.ended_tests, 23);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "input_port_matrix_before_serial_reads")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | input_port_serial_matrix (23) |"));
    assert!(report.contains("| Focus domain | joypad.input_port_matrix |"));
    assert!(report.contains("| Likely domain | joypad.input_port_matrix |"));
    assert!(report.contains("| 23 | input_port_serial_matrix | joypad | edge_case | failed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_ppu_read_buffer_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::PpuVramReadBuffer),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported PPU failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("ppu_vram_read_buffer")
    );
    assert_eq!(telemetry.verdict.current_test, 14);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("ppu_vram_read_buffer")
    );
    assert_eq!(telemetry.verdict.failure_code, 0xD0);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include PPU failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 14);
    assert_eq!(failure.test_name, Some("ppu_vram_read_buffer"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Ppu));
    assert_eq!(failure.failure_code_hex, "0xD0");
    assert_eq!(failure.likely_domain, "ppu.registers.ppudata_buffer");
    assert!(failure.assertion.contains("buffered VRAM byte"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("ppu_vram_read_buffer")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("ppu.registers.ppudata_buffer")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 14);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("ppu.registers.ppudata_buffer")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.14.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(14)
            && probe.likely_domain == "ppu.registers.ppudata_buffer"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 14);
    assert_eq!(telemetry.analysis.timing.ended_tests, 14);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "ppu_vram_read_buffer_before_first_read")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | ppu_vram_read_buffer (14) |"));
    assert!(report.contains("| Focus domain | ppu.registers.ppudata_buffer |"));
    assert!(report.contains("| Likely domain | ppu.registers.ppudata_buffer |"));
    assert!(report.contains("| 14 | ppu_vram_read_buffer | ppu | edge_case | failed |"));
    assert!(
        report.contains("| 15 | mapper2_prg_bank_switch | cartridge | integration | not_started |")
    );
    assert!(report
        .contains("| 16 | mapper2_prg_ram_roundtrip | cartridge | integration | not_started |"));
    assert!(report
        .contains("| 17 | ppu_horizontal_nametable_mirroring | ppu | integration | not_started |"));
    assert!(report
        .contains("| 18 | joypad_strobe_reset_midstream | joypad | edge_case | not_started |"));
    assert!(report.contains("| 19 | ppu_vram_increment_32 | ppu | edge_case | not_started |"));
    assert!(report.contains("| 20 | ppu_status_latch_reset | ppu | edge_case | not_started |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_mapper2_bank_switch_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::Mapper2PrgBankSwitch),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported mapper failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("mapper2_prg_bank_switch")
    );
    assert_eq!(telemetry.verdict.current_test, 15);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("mapper2_prg_bank_switch")
    );
    assert_eq!(telemetry.verdict.failure_code, 0xF1);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include mapper failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 15);
    assert_eq!(failure.test_name, Some("mapper2_prg_bank_switch"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Cartridge));
    assert_eq!(failure.failure_code_hex, "0xF1");
    assert_eq!(failure.likely_domain, "mapper.uxrom.prg_bank_switch");
    assert!(failure.assertion.contains("bank 1"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Cartridge)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("mapper2_prg_bank_switch")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("mapper.uxrom.prg_bank_switch")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 15);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("mapper.uxrom.prg_bank_switch")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.15.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(15)
            && probe.likely_domain == "mapper.uxrom.prg_bank_switch"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 15);
    assert_eq!(telemetry.analysis.timing.ended_tests, 15);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "mapper2_prg_bank_switch_before_read")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | mapper2_prg_bank_switch (15) |"));
    assert!(report.contains("| Focus domain | mapper.uxrom.prg_bank_switch |"));
    assert!(report.contains("| Likely domain | mapper.uxrom.prg_bank_switch |"));
    assert!(report.contains("| 15 | mapper2_prg_bank_switch | cartridge | integration | failed |"));
    assert!(report
        .contains("| 16 | mapper2_prg_ram_roundtrip | cartridge | integration | not_started |"));
    assert!(report
        .contains("| 17 | ppu_horizontal_nametable_mirroring | ppu | integration | not_started |"));
    assert!(report
        .contains("| 18 | joypad_strobe_reset_midstream | joypad | edge_case | not_started |"));
    assert!(report.contains("| 19 | ppu_vram_increment_32 | ppu | edge_case | not_started |"));
    assert!(report.contains("| 20 | ppu_status_latch_reset | ppu | edge_case | not_started |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_mapper2_prg_ram_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::Mapper2PrgRam),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported mapper PRG RAM failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("mapper2_prg_ram")
    );
    assert_eq!(telemetry.verdict.current_test, 16);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("mapper2_prg_ram_roundtrip")
    );
    assert_eq!(telemetry.verdict.failure_code, 0xF5);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include mapper PRG RAM failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 16);
    assert_eq!(failure.test_name, Some("mapper2_prg_ram_roundtrip"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Cartridge));
    assert_eq!(failure.failure_code_hex, "0xF5");
    assert_eq!(failure.likely_domain, "mapper.uxrom.prg_ram");
    assert!(failure.assertion.contains("upper boundary"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Cartridge)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("mapper2_prg_ram_roundtrip")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("mapper.uxrom.prg_ram")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 16);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("mapper.uxrom.prg_ram")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.16.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(16)
            && probe.likely_domain == "mapper.uxrom.prg_ram"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 16);
    assert_eq!(telemetry.analysis.timing.ended_tests, 16);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "mapper2_prg_ram_roundtrip_before_high_read")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | mapper2_prg_ram_roundtrip (16) |"));
    assert!(report.contains("| Focus domain | mapper.uxrom.prg_ram |"));
    assert!(report.contains("| Likely domain | mapper.uxrom.prg_ram |"));
    assert!(
        report.contains("| 16 | mapper2_prg_ram_roundtrip | cartridge | integration | failed |")
    );
    assert!(report
        .contains("| 17 | ppu_horizontal_nametable_mirroring | ppu | integration | not_started |"));
    assert!(report
        .contains("| 18 | joypad_strobe_reset_midstream | joypad | edge_case | not_started |"));
    assert!(report.contains("| 19 | ppu_vram_increment_32 | ppu | edge_case | not_started |"));
    assert!(report.contains("| 20 | ppu_status_latch_reset | ppu | edge_case | not_started |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_ppu_nametable_mirroring_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::PpuNametableMirroring),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported PPU nametable mirroring failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("ppu_nametable_mirroring")
    );
    assert_eq!(telemetry.verdict.current_test, 17);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("ppu_horizontal_nametable_mirroring")
    );
    assert_eq!(telemetry.verdict.failure_code, 0xE0);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include PPU nametable mirroring localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 17);
    assert_eq!(
        failure.test_name,
        Some("ppu_horizontal_nametable_mirroring")
    );
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Ppu));
    assert_eq!(failure.failure_code_hex, "0xE0");
    assert_eq!(failure.likely_domain, "ppu.nametables.horizontal_mirroring");
    assert!(failure.assertion.contains("$2000"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("ppu_horizontal_nametable_mirroring")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("ppu.nametables.horizontal_mirroring")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 17);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("ppu.nametables.horizontal_mirroring")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.17.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(17)
            && probe.likely_domain == "ppu.nametables.horizontal_mirroring"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 17);
    assert_eq!(telemetry.analysis.timing.ended_tests, 17);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(
            |symbol| symbol.name == "ppu_horizontal_nametable_mirroring_before_first_mirror_read"
        )));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | ppu_horizontal_nametable_mirroring (17) |"));
    assert!(report.contains("| Focus domain | ppu.nametables.horizontal_mirroring |"));
    assert!(report.contains("| Likely domain | ppu.nametables.horizontal_mirroring |"));
    assert!(
        report.contains("| 17 | ppu_horizontal_nametable_mirroring | ppu | integration | failed |")
    );
    assert!(report
        .contains("| 18 | joypad_strobe_reset_midstream | joypad | edge_case | not_started |"));
    assert!(report.contains("| 19 | ppu_vram_increment_32 | ppu | edge_case | not_started |"));
    assert!(report.contains("| 20 | ppu_status_latch_reset | ppu | edge_case | not_started |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_joypad_strobe_reset_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::JoypadStrobeReset),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported joypad strobe-reset failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("joypad_strobe_reset")
    );
    assert_eq!(telemetry.verdict.current_test, 18);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("joypad_strobe_reset_midstream")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x78);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include joypad strobe-reset localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 18);
    assert_eq!(failure.test_name, Some("joypad_strobe_reset_midstream"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Joypad));
    assert_eq!(failure.failure_code_hex, "0x78");
    assert_eq!(failure.likely_domain, "joypad.strobe_reset");
    assert!(failure.assertion.contains("A button bit"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Joypad)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("joypad_strobe_reset_midstream")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("joypad.strobe_reset")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 18);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("joypad.strobe_reset")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.18.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(18)
            && probe.likely_domain == "joypad.strobe_reset"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 18);
    assert_eq!(telemetry.analysis.timing.ended_tests, 18);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "joypad_strobe_reset_before_reset_read")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | joypad_strobe_reset_midstream (18) |"));
    assert!(report.contains("| Focus domain | joypad.strobe_reset |"));
    assert!(report.contains("| Likely domain | joypad.strobe_reset |"));
    assert!(report.contains("| 18 | joypad_strobe_reset_midstream | joypad | edge_case | failed |"));
    assert!(report.contains("| 19 | ppu_vram_increment_32 | ppu | edge_case | not_started |"));
    assert!(report.contains("| 20 | ppu_status_latch_reset | ppu | edge_case | not_started |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_joypad_strobe_high_hold_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::JoypadStrobeHighHold),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported joypad strobe-high hold failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("joypad_strobe_high_hold")
    );
    assert_eq!(telemetry.verdict.current_test, 21);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("joypad_strobe_high_hold")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x7D);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include joypad strobe-high localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 21);
    assert_eq!(failure.test_name, Some("joypad_strobe_high_hold"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Joypad));
    assert_eq!(failure.failure_code_hex, "0x7D");
    assert_eq!(failure.likely_domain, "joypad.strobe_high_hold");
    assert!(failure.assertion.contains("strobe-high read"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Joypad)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("joypad_strobe_high_hold")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("joypad.strobe_high_hold")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 21);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("joypad.strobe_high_hold")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.21.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(21)
            && probe.likely_domain == "joypad.strobe_high_hold"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 21);
    assert_eq!(telemetry.analysis.timing.ended_tests, 21);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "joypad_strobe_high_hold_before_reads")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | joypad_strobe_high_hold (21) |"));
    assert!(report.contains("| Focus domain | joypad.strobe_high_hold |"));
    assert!(report.contains("| Likely domain | joypad.strobe_high_hold |"));
    assert!(report.contains("| 21 | joypad_strobe_high_hold | joypad | edge_case | failed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_ppu_vram_increment_32_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::PpuVramIncrement32),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported PPUDATA increment-32 failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("ppu_vram_increment_32")
    );
    assert_eq!(telemetry.verdict.current_test, 19);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("ppu_vram_increment_32")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x7A);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include PPUDATA increment-32 localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 19);
    assert_eq!(failure.test_name, Some("ppu_vram_increment_32"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Ppu));
    assert_eq!(failure.failure_code_hex, "0x7A");
    assert_eq!(failure.likely_domain, "ppu.registers.ppudata_increment_32");
    assert!(failure.assertion.contains("auto-increments by 32"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("ppu_vram_increment_32")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("ppu.registers.ppudata_increment_32")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 19);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("ppu.registers.ppudata_increment_32")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.19.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(19)
            && probe.likely_domain == "ppu.registers.ppudata_increment_32"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 19);
    assert_eq!(telemetry.analysis.timing.ended_tests, 19);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "ppu_vram_increment_32_before_stride_read")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | ppu_vram_increment_32 (19) |"));
    assert!(report.contains("| Focus domain | ppu.registers.ppudata_increment_32 |"));
    assert!(report.contains("| Likely domain | ppu.registers.ppudata_increment_32 |"));
    assert!(report.contains("| 19 | ppu_vram_increment_32 | ppu | edge_case | failed |"));
    assert!(report.contains("| 20 | ppu_status_latch_reset | ppu | edge_case | not_started |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_ppu_status_latch_reset_failure() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::PpuStatusLatchReset),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a reported PPUSTATUS latch-reset failure");

    assert!(!telemetry.verdict.passed);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("ppu_status_latch_reset")
    );
    assert_eq!(telemetry.verdict.current_test, 20);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("ppu_status_latch_reset")
    );
    assert_eq!(telemetry.verdict.failure_code, 0x7C);

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("failed run should include PPUSTATUS latch-reset localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::CartridgeAssertion);
    assert_eq!(failure.test_id, 20);
    assert_eq!(failure.test_name, Some("ppu_status_latch_reset"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Ppu));
    assert_eq!(failure.failure_code_hex, "0x7C");
    assert_eq!(failure.likely_domain, "ppu.registers.status_latch_reset");
    assert!(failure.assertion.contains("PPUSTATUS read resets"));

    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("ppu_status_latch_reset")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("ppu.registers.status_latch_reset")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 20);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("ppu.registers.status_latch_reset")
    );
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.20.result"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(20)
            && probe.likely_domain == "ppu.registers.status_latch_reset"
    }));
    assert_eq!(telemetry.analysis.timing.started_tests, 20);
    assert_eq!(telemetry.analysis.timing.ended_tests, 20);
    assert_eq!(
        telemetry.analysis.timing.not_started_tests,
        DIAGNOSTIC_TESTS.len() - telemetry.analysis.timing.started_tests
    );
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "ppu_status_latch_reset_before_address_write")));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | ppu_status_latch_reset (20) |"));
    assert!(report.contains("| Focus domain | ppu.registers.status_latch_reset |"));
    assert!(report.contains("| Likely domain | ppu.registers.status_latch_reset |"));
    assert!(report.contains("| 20 | ppu_status_latch_reset | ppu | edge_case | failed |"));
}

#[test]
fn generated_diagnostic_cartridge_localizes_intentional_ppu_nmi_timeout() {
    let telemetry = run_diagnostic(DiagnosticConfig {
        fault_injection: Some(DiagnosticFaultInjection::PpuNmiTimeout),
        ..DiagnosticConfig::default()
    })
    .expect("diagnostic should run to a PPU NMI timeout");

    assert!(!telemetry.verdict.passed);
    assert!(telemetry.verdict.timeout);
    assert_eq!(
        telemetry.input.fault_injection_label,
        Some("ppu_nmi_timeout")
    );
    assert_eq!(telemetry.verdict.current_test, 10);
    assert_eq!(
        telemetry.verdict.current_test_name,
        Some("ppu_nmi_and_render_frame")
    );

    let failure = telemetry
        .verdict
        .failure
        .as_ref()
        .expect("PPU NMI timeout should include failure localization");
    assert_eq!(failure.kind, DiagnosticFailureKind::Timeout);
    assert_eq!(failure.test_id, 10);
    assert_eq!(failure.test_name, Some("ppu_nmi_and_render_frame"));
    assert_eq!(failure.subsystem, Some(DiagnosticSubsystem::Ppu));
    assert_eq!(failure.likely_domain, "ppu.nmi");

    assert_eq!(telemetry.analysis.health, DiagnosticHealth::TimedOut);
    assert_eq!(
        telemetry.analysis.failing_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(
        telemetry.analysis.failing_test,
        Some("ppu_nmi_and_render_frame")
    );
    assert_eq!(
        telemetry.analysis.first_failure_domain.as_deref(),
        Some("ppu.nmi")
    );
    assert_eq!(telemetry.analysis.debug_focus.focus_test_id, 10);
    assert_eq!(
        telemetry.analysis.debug_focus.focus_test_name,
        Some("ppu_nmi_and_render_frame")
    );
    assert_eq!(
        telemetry.analysis.debug_focus.focus_subsystem,
        Some(DiagnosticSubsystem::Ppu)
    );
    assert_eq!(
        telemetry.analysis.debug_focus.focus_domain.as_deref(),
        Some("ppu.nmi")
    );
    assert_eq!(
        telemetry.analysis.debug_focus.failure_kind,
        Some(DiagnosticFailureKind::Timeout)
    );
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .contains(&"runtime.completed".to_string()));
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .contains(&"cartridge.test.10.result".to_string()));
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .contains(&"ppu.nmi_count".to_string()));
    assert!(telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .contains(&"ppu.vblank_timing.nmi_window".to_string()));
    assert!(!telemetry
        .analysis
        .debug_focus
        .failed_probe_ids
        .contains(&"ppu.vblank_timing.edge_dots".to_string()));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.nmi_count"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(10)
            && probe.likely_domain == "ppu.nmi"
            && probe.observed.contains("NMI count 0")
    }));
    assert!(!telemetry.ppu_vblank_timing.passed);
    assert!(!telemetry.ppu_vblank_timing.nmi_window_passed);
    assert!(telemetry.ppu_vblank_timing.edge_passed);
    assert_eq!(telemetry.ppu_vblank_timing.observed_nmi_count, 0);
    assert!(telemetry.ppu_vblank_timing.edge_set_count >= 2);
    assert!(telemetry.ppu_vblank_timing.edge_clear_count >= 1);
    assert!(telemetry
        .ppu_vblank_timing
        .first_nmi_latency_cycles
        .is_none());
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.vblank_timing.nmi_window"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.test_id == Some(10)
            && probe.likely_domain == "ppu.vblank_timing"
            && probe.observed.contains("nmi_count=0")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "ppu.vblank_timing.edge_dots"
            && probe.status == DiagnosticProbeStatus::Passed
            && probe.test_id == Some(10)
            && probe.likely_domain == "ppu.vblank_timing"
            && probe.observed.contains("first_set=241:1")
            && probe.observed.contains("first_clear=-1:1")
    }));
    assert!(telemetry.instruction_trace.tail.iter().any(|entry| entry
        .symbol
        .as_ref()
        .is_some_and(|symbol| symbol.name == "ppu_nmi_render_frame_after_enable")));

    let nmi_timeline = telemetry
        .timeline
        .iter()
        .find(|test| test.test_id == 10)
        .expect("PPU NMI test should have timeline telemetry");
    assert_eq!(nmi_timeline.outcome, TestTimelineOutcome::TimedOut);
    assert_eq!(
        nmi_timeline.end_reason,
        Some(TestTimelineEndReason::Timeout)
    );
    assert!(nmi_timeline
        .duration_cycles
        .is_some_and(|duration| duration > 0));
    assert!(telemetry
        .timeline
        .iter()
        .filter(|test| test.test_id > 10)
        .all(|test| test.outcome == TestTimelineOutcome::NotStarted));

    let report = format_diagnostic_report(&telemetry);
    assert!(report.contains("| Focus test | ppu_nmi_and_render_frame (10) |"));
    assert!(report.contains("| Focus domain | ppu.nmi |"));
    assert!(report.contains("| Likely domain | ppu.nmi |"));
    assert!(report.contains("| 10 | ppu_nmi_and_render_frame | ppu | integration | timed_out |"));
    assert!(report.contains("ppu.nmi_count"));
    assert!(report.contains("ppu.vblank_timing.nmi_window"));
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
        telemetry.analysis.debug_focus.health,
        DiagnosticHealth::TimedOut
    );
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
    assert!(telemetry.analysis.probe_summary.failed_probes >= 2);
    assert!(telemetry.analysis.probe_summary.skipped_probes >= DIAGNOSTIC_TESTS.len());
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "runtime.completed"
            && probe.status == DiagnosticProbeStatus::Failed
            && probe.observed.contains("timeout=true")
    }));
    assert!(telemetry.probes.iter().any(|probe| {
        probe.id == "cartridge.test.1.result" && probe.status == DiagnosticProbeStatus::Skipped
    }));
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
    assert!(report.contains("| Not started tests | 28 |"));
    assert!(report.contains("| Slowest test | none |"));
    assert!(report.contains("| failed | runtime.completed | host_observation | none | none |"));
}

#[test]
fn generated_diagnostic_cartridge_compares_against_matching_baseline() {
    let baseline = run_diagnostic(DiagnosticConfig::default()).expect("baseline should run");
    let baseline_json = serde_json::to_string(&baseline).expect("baseline should serialize");
    let current = run_diagnostic(DiagnosticConfig::default()).expect("current should run");

    let comparison =
        compare_diagnostic_to_baseline(&current, &baseline_json).expect("comparison should run");

    assert!(comparison.passed);
    assert_eq!(comparison.difference_count, 0);
    assert_eq!(comparison.failure_count, 0);
    assert_eq!(comparison.warning_count, 0);
    assert_eq!(
        comparison.current_schema_version,
        DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION
    );
    assert_eq!(
        comparison.baseline_schema_version,
        Some(DIAGNOSTIC_TELEMETRY_SCHEMA_VERSION as u64)
    );
    let report = format_diagnostic_comparison_report(&comparison);
    assert!(report.contains("# OxideNES Diagnostic Baseline Comparison"));
    assert!(report.contains("| Result | pass |"));
    assert!(report.contains("No baseline differences detected."));
}

#[test]
fn generated_diagnostic_cartridge_comparison_warns_on_execution_state_drift() {
    let telemetry = run_diagnostic(DiagnosticConfig::default()).expect("diagnostic should run");
    let mut baseline = serde_json::to_value(&telemetry).expect("telemetry should serialize");
    baseline["cpu"]["pc"] = serde_json::Value::from(0);
    baseline["ram"]["checksum"] = serde_json::Value::from(0);
    baseline["instruction_trace"]["captured_instruction_count"] = serde_json::Value::from(1);
    let baseline_json = serde_json::to_string(&baseline).expect("baseline should serialize");

    let comparison =
        compare_diagnostic_to_baseline(&telemetry, &baseline_json).expect("comparison should run");

    assert!(comparison.passed);
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Warning
            && difference.category == "state"
            && difference.path == "cpu.pc"
    }));
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Warning
            && difference.category == "state"
            && difference.path == "ram.checksum"
    }));
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Warning
            && difference.category == "trace"
            && difference.path == "instruction_trace.captured_instruction_count"
    }));
}

#[test]
fn generated_diagnostic_cartridge_comparison_fails_on_assertion_regression() {
    let baseline = run_diagnostic(DiagnosticConfig::default()).expect("baseline should run");
    let baseline_json = serde_json::to_string(&baseline).expect("baseline should serialize");
    let current = run_diagnostic(DiagnosticConfig {
        joypad1_mask: 0x00,
        ..DiagnosticConfig::default()
    })
    .expect("current should run to reported failure");

    let comparison =
        compare_diagnostic_to_baseline(&current, &baseline_json).expect("comparison should run");

    assert!(!comparison.passed);
    assert!(comparison.failure_count >= 1);
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Failure
            && difference.path == "verdict.passed"
    }));
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Failure
            && difference.path == "timeline[7].outcome"
            && difference.current.as_deref() == Some("failed")
    }));
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Failure
            && difference.path == "analysis.probe_summary.failed_probes"
    }));
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Failure
            && difference.path == "probes[cartridge.test.7.result].status"
            && difference.current.as_deref() == Some("failed")
    }));
    let report = format_diagnostic_comparison_report(&comparison);
    assert!(report.contains("| Result | fail |"));
    assert!(report.contains("diagnostic comparison failed"));
    assert!(report.contains("| failure | timeline | timeline[7].outcome | passed | failed |"));
}

#[test]
fn generated_diagnostic_cartridge_comparison_warns_on_timing_regression() {
    let telemetry = run_diagnostic(DiagnosticConfig::default()).expect("diagnostic should run");
    let mut baseline = serde_json::to_value(&telemetry).expect("telemetry should serialize");
    let timeline = baseline["timeline"]
        .as_array_mut()
        .expect("baseline should include timeline");
    let test = timeline
        .iter_mut()
        .find(|entry| entry["test_id"].as_u64() == Some(10))
        .expect("baseline should include test 10");
    test["duration_cycles"] = serde_json::Value::from(1);
    let baseline_json = serde_json::to_string(&baseline).expect("baseline should serialize");

    let comparison =
        compare_diagnostic_to_baseline(&telemetry, &baseline_json).expect("comparison should run");

    assert!(comparison.passed);
    assert_eq!(comparison.failure_count, 0);
    assert!(comparison.warning_count >= 1);
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Warning
            && difference.category == "timing"
            && difference.path == "timeline[10].duration_cycles"
    }));
}

#[test]
fn generated_diagnostic_cartridge_comparison_warns_on_dma_timing_drift() {
    let telemetry = run_diagnostic(DiagnosticConfig::default()).expect("diagnostic should run");
    let mut baseline = serde_json::to_value(&telemetry).expect("telemetry should serialize");
    baseline["dma"]["oam_dma_active_cycles"] = serde_json::Value::from(1);
    baseline["dma"]["dmc_dma_fetches_during_oam_dma"] = serde_json::Value::from(0);
    baseline["dma"]["dmc_dma_first_oam_overlap_stall_cycles"] = serde_json::Value::from(9);
    let baseline_json = serde_json::to_string(&baseline).expect("baseline should serialize");

    let comparison =
        compare_diagnostic_to_baseline(&telemetry, &baseline_json).expect("comparison should run");

    assert!(comparison.passed);
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Warning
            && difference.category == "dma"
            && difference.path == "dma.oam_dma_active_cycles"
    }));
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Warning
            && difference.category == "dma"
            && difference.path == "dma.dmc_dma_fetches_during_oam_dma"
    }));
    assert!(comparison.differences.iter().any(|difference| {
        difference.severity == DiagnosticComparisonSeverity::Warning
            && difference.category == "dma"
            && difference.path == "dma.dmc_dma_first_oam_overlap_stall_cycles"
    }));
}

#[test]
fn generated_diagnostic_cartridge_has_no_bundled_rom_provenance() {
    let rom = build_diagnostic_cartridge().expect("diagnostic cartridge should build");

    assert_eq!(&rom[0..4], b"NES\x1A");
    assert_eq!(rom[4], 4);
    assert_eq!(rom[6], 0x20);
    assert_eq!(rom[7], 0x00);
    assert_eq!(rom.len(), 16 + 4 * 16 * 1024 + 8 * 1024);
    assert!(DIAGNOSTIC_PROVENANCE.contains("Generated OxideNES diagnostic"));
    assert!(DIAGNOSTIC_PROVENANCE.contains("no ROM content"));
}
