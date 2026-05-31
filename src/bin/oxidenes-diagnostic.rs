use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use oxidenes::diagnostic::{
    build_diagnostic_cartridge, compare_diagnostic_to_baseline,
    format_diagnostic_comparison_report, format_diagnostic_report, run_diagnostic,
    DiagnosticComparisonTelemetry, DiagnosticConfig, DiagnosticDebugEventFocusTelemetry,
    DiagnosticDebugInstructionFocusTelemetry, DiagnosticHealth, DiagnosticProbeStatus,
    DiagnosticTelemetry, DIAGNOSTIC_PROVENANCE,
};
use oxidenes::recording::sha256;
use serde::Serialize;

const DIAGNOSTIC_BUNDLE_SCHEMA_VERSION: u16 = 1;
const DIAGNOSTIC_TRIAGE_SCHEMA_VERSION: u16 = 5;
const DIAGNOSTIC_SCENARIO_SUITE_SCHEMA_VERSION: u16 = 6;
const DIAGNOSTIC_SCENARIO_OBSERVER_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Serialize)]
struct DiagnosticBundleManifest {
    bundle_schema_version: u16,
    telemetry_schema_version: u16,
    suite_name: String,
    suite_version: String,
    passed: bool,
    recommended_exit_code: u8,
    config: DiagnosticBundleConfig,
    comparison_included: bool,
    artifacts: Vec<DiagnosticBundleArtifact>,
    ai_handoff: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DiagnosticBundleConfig {
    max_cpu_cycles: u64,
    joypad1_mask: u8,
    joypad1_mask_hex: String,
    joypad2_mask: u8,
    joypad2_mask_hex: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteManifest {
    scenario_suite_schema_version: u16,
    telemetry_schema_version: u16,
    triage_schema_version: u16,
    bundle_schema_version: u16,
    suite_name: String,
    suite_version: String,
    baseline_scenario_id: &'static str,
    scenario_count: usize,
    passed: bool,
    recommended_exit_code: u8,
    analysis: DiagnosticScenarioSuiteAnalysis,
    artifacts: DiagnosticScenarioSuiteRootArtifacts,
    scenarios: Vec<DiagnosticScenarioSuiteEntry>,
    ai_handoff: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteAnalysis {
    status: &'static str,
    summary: String,
    scenario_count: usize,
    expectation_met_count: usize,
    expectation_mismatch_count: usize,
    contract_mismatch_count: usize,
    baseline_divergence_count: usize,
    critical_scenario_ids: Vec<String>,
    known_divergence_scenario_ids: Vec<String>,
    attention_queue: Vec<DiagnosticScenarioSuiteAttentionItem>,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteAttentionItem {
    scenario_id: String,
    priority: &'static str,
    reason: &'static str,
    next_artifact: String,
    expectation_met: bool,
    contract_all_matched: bool,
    comparison_passed: bool,
    actual_health: String,
    focus_domain: Option<String>,
    failed_probe_ids: Vec<String>,
    comparison_difference_count: usize,
    top_difference_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteRootArtifacts {
    scenario_suite_json: &'static str,
    scenario_suite_report: &'static str,
    scenario_suite_observer_json: &'static str,
    scenario_suite_observer_report: &'static str,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteObserverReport {
    observer_schema_version: u16,
    scenario_suite_schema_version: u16,
    telemetry_schema_version: u16,
    triage_schema_version: u16,
    bundle_schema_version: u16,
    suite_name: String,
    suite_version: String,
    status: &'static str,
    summary: String,
    recommended_exit_code: u8,
    scenario_count: usize,
    contract_mismatch_count: usize,
    baseline_divergence_count: usize,
    critical_scenario_ids: Vec<String>,
    known_divergence_scenario_ids: Vec<String>,
    next_actions: Vec<DiagnosticScenarioSuiteObserverAction>,
    observations: Vec<DiagnosticScenarioSuiteObserverObservation>,
    artifact_hints: Vec<DiagnosticScenarioSuiteObserverArtifactHint>,
    ai_handoff: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteObserverAction {
    priority: &'static str,
    action_type: &'static str,
    scenario_id: String,
    summary: String,
    primary_artifact: String,
    supporting_artifacts: Vec<String>,
    evidence: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteObserverObservation {
    scenario_id: String,
    title: String,
    role: &'static str,
    outcome: &'static str,
    health: String,
    expected_passed: bool,
    actual_passed: bool,
    contract_all_matched: bool,
    comparison_passed: bool,
    focus_test_id: u8,
    focus_domain: Option<String>,
    failed_probe_ids: Vec<String>,
    comparison_difference_count: usize,
    top_difference_path: Option<String>,
    next_artifact: String,
    bundle_manifest: String,
    triage_json: String,
    comparison_json: String,
    telemetry_json: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteObserverArtifactHint {
    path: &'static str,
    kind: &'static str,
    purpose: &'static str,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteEntry {
    id: &'static str,
    title: &'static str,
    purpose: &'static str,
    directory: String,
    expected_runner_exit_code: u8,
    expected_passed: bool,
    actual_passed: bool,
    expected_health: String,
    actual_health: String,
    expected_focus_test_id: Option<u8>,
    actual_focus_test_id: u8,
    actual_focus_test_name: Option<&'static str>,
    expected_focus_domain: Option<&'static str>,
    actual_focus_domain: Option<String>,
    failure_kind: Option<String>,
    failure_code_hex: String,
    failed_probe_ids: Vec<String>,
    comparison: DiagnosticTriageComparison,
    contract: DiagnosticScenarioSuiteContract,
    expectation_met: bool,
    config: DiagnosticBundleConfig,
    artifacts: DiagnosticScenarioSuiteArtifacts,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteContract {
    expected_passed: bool,
    actual_passed: bool,
    passed_matches: bool,
    expected_health: String,
    actual_health: String,
    health_matches: bool,
    expected_focus_test_id: Option<u8>,
    actual_focus_test_id: u8,
    focus_test_matches: bool,
    expected_focus_domain: Option<&'static str>,
    actual_focus_domain: Option<String>,
    focus_domain_matches: bool,
    all_matched: bool,
}

#[derive(Debug, Serialize)]
struct DiagnosticScenarioSuiteArtifacts {
    bundle_manifest: String,
    triage_json: String,
    telemetry_json: String,
    report_md: String,
    comparison_json: String,
    comparison_report: String,
    diagnostic_rom: String,
}

struct DiagnosticScenarioSpec {
    id: &'static str,
    title: &'static str,
    purpose: &'static str,
    config: DiagnosticConfig,
    expected_passed: bool,
    expected_health: DiagnosticHealth,
    expected_focus_test_id: Option<u8>,
    expected_focus_domain: Option<&'static str>,
}

struct DiagnosticScenarioSuiteWriteResult {
    passed: bool,
    json: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticBundleArtifact {
    path: String,
    kind: &'static str,
    bytes: usize,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageReport {
    triage_schema_version: u16,
    telemetry_schema_version: u16,
    suite_name: String,
    suite_version: String,
    passed: bool,
    recommended_exit_code: u8,
    health: String,
    summary: String,
    current_test: DiagnosticTriageCurrentTest,
    debug_focus: DiagnosticTriageDebugFocus,
    input: DiagnosticTriageInput,
    failure: Option<DiagnosticTriageFailure>,
    coverage: DiagnosticTriageCoverage,
    coverage_gaps: Vec<DiagnosticTriageCoverageGap>,
    dma: DiagnosticTriageDma,
    probes: DiagnosticTriageProbeSummary,
    timing: DiagnosticTriageTiming,
    instruction_trace: DiagnosticTriageInstructionTrace,
    comparison: Option<DiagnosticTriageComparison>,
    next_actions: Vec<String>,
    artifact_hints: Vec<DiagnosticTriageArtifactHint>,
    event_tail: Vec<DiagnosticTriageEvent>,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageCurrentTest {
    id: u8,
    name: Option<&'static str>,
    status_hex: String,
    failure_code_hex: String,
    timed_out: bool,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageDebugFocus {
    health: String,
    focus_test_id: u8,
    focus_test_name: Option<&'static str>,
    focus_subsystem: Option<String>,
    focus_domain: Option<String>,
    failure_kind: Option<String>,
    failure_code_hex: String,
    failed_probe_ids: Vec<String>,
    skipped_probe_count: usize,
    last_event: Option<DiagnosticTriageDebugEventFocus>,
    terminal_instruction: Option<DiagnosticTriageDebugInstructionFocus>,
    last_test_instruction: Option<DiagnosticTriageDebugInstructionFocus>,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageDebugEventFocus {
    kind: String,
    cycle: u64,
    frame: u64,
    status_hex: String,
    current_test: u8,
    current_test_name: Option<&'static str>,
    pc_hex: String,
    note: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageDebugInstructionFocus {
    sequence: u64,
    cycle: u64,
    frame: u64,
    current_test: u8,
    current_test_name: Option<&'static str>,
    pc_hex: String,
    instruction: Option<String>,
    symbol: Option<String>,
    status_hex: String,
    current_result_hex: Option<String>,
    failure_code_hex: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageInput {
    joypad1_mask_hex: String,
    joypad1_expected_mask_hex: String,
    joypad2_mask_hex: String,
    joypad2_expected_mask_hex: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageFailure {
    kind: String,
    test_id: u8,
    test_name: Option<&'static str>,
    subsystem: Option<String>,
    tier: Option<String>,
    failure_code_hex: String,
    assertion: String,
    expected: String,
    observed: String,
    likely_domain: String,
    remediation_hint: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageCoverage {
    total_tests: usize,
    passed_tests: usize,
    failed_tests: usize,
    subsystems: Vec<DiagnosticTriageSubsystemCoverage>,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageSubsystemCoverage {
    subsystem: String,
    passed: usize,
    total: usize,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageCoverageGap {
    id: &'static str,
    subsystem: &'static str,
    risk: &'static str,
    current_coverage: &'static str,
    missing_coverage: &'static str,
    suggested_next_test: &'static str,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageDma {
    oam_dma_observed: bool,
    oam_dma_completed: bool,
    oam_dma_active_cycles: u64,
    oam_dma_expected_min_cycles: u64,
    oam_dma_expected_max_cycles: u64,
    oam_dma_start_cycle: Option<u64>,
    oam_dma_end_cycle: Option<u64>,
    oam_dma_first_active_cycle: Option<u64>,
    oam_dma_first_active_cycle_parity: Option<&'static str>,
    oam_dma_start_test_name: Option<&'static str>,
    oam_dma_end_test_name: Option<&'static str>,
    dmc_dma_fetches_observed: u64,
    dmc_dma_fetches_during_oam_dma: u64,
    dmc_dma_expected_min_oam_overlap_fetches: u64,
    dmc_dma_oam_overlap_observed: bool,
    dmc_dma_first_fetch_cycle: Option<u64>,
    dmc_dma_first_fetch_address: Option<u16>,
    dmc_dma_first_fetch_cpu_cycle_parity: Option<&'static str>,
    dmc_dma_first_fetch_stall_cycles: Option<u8>,
    dmc_dma_first_oam_overlap_cycle: Option<u64>,
    dmc_dma_first_oam_overlap_test_name: Option<&'static str>,
    dmc_dma_first_oam_overlap_cpu_cycle_parity: Option<&'static str>,
    dmc_dma_first_oam_overlap_stall_cycles: Option<u8>,
    dmc_dma_three_cycle_fetches: u64,
    dmc_dma_four_cycle_fetches: u64,
    dmc_dma_expected_min_stall_cycles: u8,
    dmc_dma_expected_max_stall_cycles: u8,
    dmc_dma_stall_cycles: u64,
    dmc_dma_stall_cycles_after_oam_dma: u64,
    dmc_dma_queued_during_oam_dma_cycles: u64,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageProbeSummary {
    total_probes: usize,
    passed_probes: usize,
    failed_probes: usize,
    skipped_probes: usize,
    first_failed_probe: Option<String>,
    failed: Vec<DiagnosticTriageProbe>,
    skipped: Vec<DiagnosticTriageProbe>,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageProbe {
    id: String,
    source: String,
    subsystem: Option<String>,
    test_id: Option<u8>,
    test_name: Option<&'static str>,
    expected: String,
    observed: String,
    likely_domain: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageTiming {
    started_tests: usize,
    ended_tests: usize,
    not_started_tests: usize,
    timed_out_tests: usize,
    slowest_test: Option<DiagnosticTriageSlowestTest>,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageSlowestTest {
    test_id: u8,
    test_name: &'static str,
    subsystem: String,
    tier: String,
    duration_cycles: u64,
    duration_frames: u64,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageInstructionTrace {
    captured_instruction_count: u64,
    retained_instruction_count: usize,
    retention_limit: usize,
    truncated: bool,
    triage_tail_count: usize,
    tail: Vec<DiagnosticTriageInstructionTraceEntry>,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageInstructionTraceEntry {
    sequence: u64,
    cycle: u64,
    frame: u64,
    current_test: u8,
    current_test_name: Option<&'static str>,
    pc_hex: String,
    opcode_hex: Option<String>,
    instruction: Option<String>,
    mnemonic: Option<&'static str>,
    addressing_mode: Option<&'static str>,
    symbol: Option<String>,
    symbol_offset_hex: Option<String>,
    cpu_a_hex: String,
    cpu_x_hex: String,
    cpu_y_hex: String,
    cpu_sp_hex: String,
    cpu_status_hex: String,
    current_result_hex: Option<String>,
    failure_code_hex: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageComparison {
    passed: bool,
    summary: String,
    difference_count: usize,
    failure_count: usize,
    warning_count: usize,
    info_count: usize,
    top_differences: Vec<DiagnosticTriageDifference>,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageDifference {
    severity: String,
    category: &'static str,
    path: String,
    baseline: Option<String>,
    current: Option<String>,
    note: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageArtifactHint {
    path: &'static str,
    kind: &'static str,
    purpose: &'static str,
}

#[derive(Debug, Serialize)]
struct DiagnosticTriageEvent {
    kind: String,
    cycle: u64,
    frame: u64,
    status_hex: String,
    current_test: u8,
    current_test_name: Option<&'static str>,
    pc_hex: String,
    cpu_a_hex: String,
    cpu_x_hex: String,
    cpu_y_hex: String,
    cpu_sp_hex: String,
    cpu_status_hex: String,
    cpu_pending_cycles: u8,
    current_result_addr_hex: Option<String>,
    current_result_hex: Option<String>,
    failure_code_hex: String,
    signature_hex: String,
    nmi_count: u8,
    note: String,
}

fn main() {
    match run() {
        Ok(passed) => {
            if !passed {
                std::process::exit(1);
            }
        }
        Err(err) => {
            eprintln!("oxidenes-diagnostic: {err}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<bool, String> {
    let mut config = DiagnosticConfig::default();
    let mut json_path: Option<PathBuf> = None;
    let mut report_path: Option<PathBuf> = None;
    let mut baseline_json_path: Option<PathBuf> = None;
    let mut comparison_json_path: Option<PathBuf> = None;
    let mut comparison_report_path: Option<PathBuf> = None;
    let mut triage_json_path: Option<PathBuf> = None;
    let mut dump_rom_path: Option<PathBuf> = None;
    let mut bundle_dir: Option<PathBuf> = None;
    let mut scenario_suite_dir: Option<PathBuf> = None;
    let mut config_overridden = false;
    let mut print_stdout = true;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return Ok(true);
            }
            "--json" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--json requires a file path".to_string())?;
                json_path = Some(PathBuf::from(path));
            }
            "--report" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--report requires a file path".to_string())?;
                report_path = Some(PathBuf::from(path));
            }
            "--baseline-json" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--baseline-json requires a file path".to_string())?;
                baseline_json_path = Some(PathBuf::from(path));
            }
            "--comparison-json" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--comparison-json requires a file path".to_string())?;
                comparison_json_path = Some(PathBuf::from(path));
            }
            "--comparison-report" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--comparison-report requires a file path".to_string())?;
                comparison_report_path = Some(PathBuf::from(path));
            }
            "--triage-json" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--triage-json requires a file path".to_string())?;
                triage_json_path = Some(PathBuf::from(path));
            }
            "--dump-rom" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--dump-rom requires a file path".to_string())?;
                dump_rom_path = Some(PathBuf::from(path));
            }
            "--bundle-dir" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--bundle-dir requires a directory path".to_string())?;
                bundle_dir = Some(PathBuf::from(path));
            }
            "--scenario-suite-dir" => {
                let path = args
                    .next()
                    .ok_or_else(|| "--scenario-suite-dir requires a directory path".to_string())?;
                scenario_suite_dir = Some(PathBuf::from(path));
            }
            "--max-cycles" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--max-cycles requires a number".to_string())?;
                config.max_cpu_cycles = value
                    .parse()
                    .map_err(|_| format!("invalid --max-cycles value: {value}"))?;
                config_overridden = true;
            }
            "--joypad1" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--joypad1 requires a byte mask".to_string())?;
                config.joypad1_mask = parse_byte(&value)?;
                config_overridden = true;
            }
            "--joypad2" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--joypad2 requires a byte mask".to_string())?;
                config.joypad2_mask = parse_byte(&value)?;
                config_overridden = true;
            }
            "--no-stdout" => {
                print_stdout = false;
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    if let Some(path) = scenario_suite_dir {
        let single_run_outputs_requested = json_path.is_some()
            || report_path.is_some()
            || baseline_json_path.is_some()
            || comparison_json_path.is_some()
            || comparison_report_path.is_some()
            || triage_json_path.is_some()
            || dump_rom_path.is_some()
            || bundle_dir.is_some();
        if single_run_outputs_requested || config_overridden {
            return Err(
                "--scenario-suite-dir uses fixed scenario configs and cannot be combined with single-run output or config override options"
                    .to_string(),
            );
        }

        let result = write_scenario_suite(&path)?;
        if print_stdout {
            println!("{}", result.json);
        }
        return Ok(result.passed);
    }

    let rom = if dump_rom_path.is_some() || bundle_dir.is_some() {
        Some(build_diagnostic_cartridge()?)
    } else {
        None
    };

    if let Some(path) = dump_rom_path {
        let rom = rom
            .as_ref()
            .expect("ROM should be available when --dump-rom was requested");
        write_file(&path, rom)?;
    }

    let config_manifest = diagnostic_bundle_config(&config);
    let telemetry = run_diagnostic(config)?;
    let json = serde_json::to_string_pretty(&telemetry)
        .map_err(|err| format!("failed to serialize telemetry: {err}"))?;

    if let Some(path) = json_path {
        write_file(&path, json.as_bytes())?;
    }

    if let Some(path) = report_path {
        let report = format_diagnostic_report(&telemetry);
        write_file(&path, report.as_bytes())?;
    }

    let comparison = if let Some(path) = baseline_json_path {
        let baseline_json = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        Some(compare_diagnostic_to_baseline(&telemetry, &baseline_json)?)
    } else {
        None
    };

    if comparison.is_none() && (comparison_json_path.is_some() || comparison_report_path.is_some())
    {
        return Err(
            "--comparison-json and --comparison-report require --baseline-json".to_string(),
        );
    }

    if let Some(comparison) = &comparison {
        if let Some(path) = comparison_json_path {
            let json = serde_json::to_string_pretty(comparison)
                .map_err(|err| format!("failed to serialize comparison telemetry: {err}"))?;
            write_file(&path, json.as_bytes())?;
        }
        if let Some(path) = comparison_report_path {
            let report = format_diagnostic_comparison_report(comparison);
            write_file(&path, report.as_bytes())?;
        }
    }

    if let Some(path) = triage_json_path {
        let triage_json = diagnostic_triage_json(&telemetry, comparison.as_ref())?;
        write_file(&path, triage_json.as_bytes())?;
    }

    let passed = telemetry.verdict.passed
        && comparison
            .as_ref()
            .is_none_or(|comparison| comparison.passed);

    if let Some(path) = bundle_dir {
        let rom = rom
            .as_ref()
            .expect("ROM should be available when --bundle-dir was requested");
        write_bundle(
            &path,
            DiagnosticBundleInput {
                telemetry: &telemetry,
                telemetry_json: &json,
                diagnostic_rom: rom,
                comparison: comparison.as_ref(),
                passed,
                config: config_manifest,
            },
        )?;
    }

    if print_stdout {
        println!("{json}");
    }

    Ok(passed)
}

fn print_help() {
    println!("OxideNES headless diagnostic cartridge runner");
    println!();
    println!("{DIAGNOSTIC_PROVENANCE}");
    println!();
    println!("USAGE:");
    println!("    cargo run --bin oxidenes-diagnostic -- [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --json <FILE>        Write telemetry JSON to a file");
    println!("    --report <FILE>      Write a Markdown diagnostic report to a file");
    println!(
        "    --baseline-json <FILE>       Compare current telemetry with a prior JSON baseline"
    );
    println!("    --comparison-json <FILE>     Write baseline comparison JSON to a file");
    println!(
        "    --comparison-report <FILE>   Write a Markdown baseline comparison report to a file"
    );
    println!("    --triage-json <FILE>         Write compact AI triage JSON to a file");
    println!("    --dump-rom <FILE>    Generate the diagnostic .nes cartridge at runtime");
    println!("    --bundle-dir <DIR>   Write an AI-ready diagnostic artifact bundle");
    println!(
        "    --scenario-suite-dir <DIR>   Write an AI-ready pass/fail corpus with observer reports"
    );
    println!("    --max-cycles <N>     Override the CPU-cycle timeout");
    println!("    --joypad1 <BYTE>     Override joypad-1 mask, decimal or 0x-prefixed hex");
    println!("    --joypad2 <BYTE>     Override joypad-2 mask, decimal or 0x-prefixed hex");
    println!("    --no-stdout          Do not print telemetry JSON to stdout");
    println!("    -h, --help           Show this help");
}

fn parse_byte(value: &str) -> Result<u8, String> {
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u8::from_str_radix(hex, 16).map_err(|_| format!("invalid byte mask: {value}"))
    } else {
        value
            .parse()
            .map_err(|_| format!("invalid byte mask: {value}"))
    }
}

struct DiagnosticBundleInput<'a> {
    telemetry: &'a DiagnosticTelemetry,
    telemetry_json: &'a str,
    diagnostic_rom: &'a [u8],
    comparison: Option<&'a DiagnosticComparisonTelemetry>,
    passed: bool,
    config: DiagnosticBundleConfig,
}

fn write_scenario_suite(path: &Path) -> Result<DiagnosticScenarioSuiteWriteResult, String> {
    fs::create_dir_all(path)
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;

    let specs = diagnostic_scenario_specs();
    let baseline_spec = specs
        .first()
        .ok_or_else(|| "diagnostic scenario suite has no baseline scenario".to_string())?;
    let diagnostic_rom = build_diagnostic_cartridge()?;
    let baseline_telemetry = run_diagnostic(baseline_spec.config.clone())?;
    let telemetry_schema_version = baseline_telemetry.schema_version;
    let suite_name = baseline_telemetry.suite.name.to_string();
    let suite_version = baseline_telemetry.suite.version.to_string();
    let baseline_json = serde_json::to_string_pretty(&baseline_telemetry)
        .map_err(|err| format!("failed to serialize baseline scenario telemetry: {err}"))?;

    let mut scenarios = Vec::new();
    scenarios.push(write_scenario_bundle(
        path,
        baseline_spec,
        baseline_telemetry,
        &baseline_json,
        &diagnostic_rom,
    )?);

    for spec in specs.iter().skip(1) {
        let telemetry = run_diagnostic(spec.config.clone())?;
        scenarios.push(write_scenario_bundle(
            path,
            spec,
            telemetry,
            &baseline_json,
            &diagnostic_rom,
        )?);
    }

    let passed = scenarios.iter().all(|scenario| scenario.expectation_met);
    let analysis = scenario_suite_analysis(&scenarios);
    let manifest = DiagnosticScenarioSuiteManifest {
        scenario_suite_schema_version: DIAGNOSTIC_SCENARIO_SUITE_SCHEMA_VERSION,
        telemetry_schema_version,
        triage_schema_version: DIAGNOSTIC_TRIAGE_SCHEMA_VERSION,
        bundle_schema_version: DIAGNOSTIC_BUNDLE_SCHEMA_VERSION,
        suite_name,
        suite_version,
        baseline_scenario_id: baseline_spec.id,
        scenario_count: scenarios.len(),
        passed,
        recommended_exit_code: if passed { 0 } else { 1 },
        analysis,
        artifacts: DiagnosticScenarioSuiteRootArtifacts {
            scenario_suite_json: "scenario-suite.json",
            scenario_suite_report: "scenario-suite.md",
            scenario_suite_observer_json: "scenario-suite-observer.json",
            scenario_suite_observer_report: "scenario-suite-observer.md",
        },
        scenarios,
        ai_handoff: scenario_suite_ai_handoff(),
    };
    let observer = scenario_suite_observer(&manifest);
    let report = format_scenario_suite_report(&manifest);
    write_file(&path.join("scenario-suite.md"), report.as_bytes())?;
    let observer_report = format_scenario_suite_observer_report(&observer);
    write_file(
        &path.join("scenario-suite-observer.md"),
        observer_report.as_bytes(),
    )?;
    let observer_json = serde_json::to_string_pretty(&observer)
        .map_err(|err| format!("failed to serialize diagnostic scenario observer: {err}"))?;
    write_file(
        &path.join("scenario-suite-observer.json"),
        observer_json.as_bytes(),
    )?;
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|err| format!("failed to serialize diagnostic scenario suite: {err}"))?;
    write_file(&path.join("scenario-suite.json"), json.as_bytes())?;

    Ok(DiagnosticScenarioSuiteWriteResult { passed, json })
}

fn scenario_suite_observer(
    manifest: &DiagnosticScenarioSuiteManifest,
) -> DiagnosticScenarioSuiteObserverReport {
    DiagnosticScenarioSuiteObserverReport {
        observer_schema_version: DIAGNOSTIC_SCENARIO_OBSERVER_SCHEMA_VERSION,
        scenario_suite_schema_version: manifest.scenario_suite_schema_version,
        telemetry_schema_version: manifest.telemetry_schema_version,
        triage_schema_version: manifest.triage_schema_version,
        bundle_schema_version: manifest.bundle_schema_version,
        suite_name: manifest.suite_name.clone(),
        suite_version: manifest.suite_version.clone(),
        status: manifest.analysis.status,
        summary: manifest.analysis.summary.clone(),
        recommended_exit_code: manifest.recommended_exit_code,
        scenario_count: manifest.scenario_count,
        contract_mismatch_count: manifest.analysis.contract_mismatch_count,
        baseline_divergence_count: manifest.analysis.baseline_divergence_count,
        critical_scenario_ids: manifest.analysis.critical_scenario_ids.clone(),
        known_divergence_scenario_ids: manifest.analysis.known_divergence_scenario_ids.clone(),
        next_actions: observer_next_actions(manifest),
        observations: observer_observations(manifest),
        artifact_hints: scenario_suite_observer_artifact_hints(),
        ai_handoff: scenario_suite_observer_ai_handoff(),
    }
}

fn observer_next_actions(
    manifest: &DiagnosticScenarioSuiteManifest,
) -> Vec<DiagnosticScenarioSuiteObserverAction> {
    let actions = manifest
        .analysis
        .attention_queue
        .iter()
        .filter_map(|item| {
            manifest
                .scenarios
                .iter()
                .find(|scenario| scenario.id == item.scenario_id)
                .map(|scenario| observer_action(item, scenario))
        })
        .collect::<Vec<_>>();

    if actions.is_empty() {
        return vec![DiagnosticScenarioSuiteObserverAction {
            priority: "info",
            action_type: "archive_clean_suite",
            scenario_id: manifest.baseline_scenario_id.to_string(),
            summary: "All scenario contracts and baseline comparisons matched; archive this suite as a passing observability baseline.".to_string(),
            primary_artifact: "scenario-suite.json".to_string(),
            supporting_artifacts: vec![
                "scenario-suite.md".to_string(),
                "scenario-suite-observer.md".to_string(),
            ],
            evidence: vec![format!(
                "scenario_count={}; contract_mismatch_count=0; baseline_divergence_count=0",
                manifest.scenario_count
            )],
        }];
    }

    actions
}

fn observer_action(
    item: &DiagnosticScenarioSuiteAttentionItem,
    scenario: &DiagnosticScenarioSuiteEntry,
) -> DiagnosticScenarioSuiteObserverAction {
    let (action_type, summary) = if item.priority == "critical" {
        (
            "investigate_contract_mismatch",
            format!(
                "Open {} first; {} no longer matches its expected diagnostic contract.",
                item.next_artifact, item.scenario_id
            ),
        )
    } else {
        (
            "inspect_known_divergence",
            format!(
                "Use {} to confirm {} is still an intentional fixture divergence from pass/.",
                item.next_artifact, item.scenario_id
            ),
        )
    };

    let mut supporting_artifacts = vec![
        scenario.artifacts.triage_json.clone(),
        scenario.artifacts.telemetry_json.clone(),
    ];
    if item.next_artifact != scenario.artifacts.comparison_json {
        supporting_artifacts.push(scenario.artifacts.comparison_json.clone());
    }

    DiagnosticScenarioSuiteObserverAction {
        priority: item.priority,
        action_type,
        scenario_id: item.scenario_id.clone(),
        summary,
        primary_artifact: item.next_artifact.clone(),
        supporting_artifacts,
        evidence: observer_action_evidence(item),
    }
}

fn observer_action_evidence(item: &DiagnosticScenarioSuiteAttentionItem) -> Vec<String> {
    let mut evidence = vec![
        format!("health={}", item.actual_health),
        format!("contract_all_matched={}", item.contract_all_matched),
        format!("comparison_passed={}", item.comparison_passed),
        format!(
            "comparison_difference_count={}",
            item.comparison_difference_count
        ),
    ];

    if let Some(domain) = &item.focus_domain {
        evidence.push(format!("focus_domain={domain}"));
    }
    if !item.failed_probe_ids.is_empty() {
        evidence.push(format!(
            "failed_probe_ids={}",
            item.failed_probe_ids.join(",")
        ));
    }
    if let Some(path) = &item.top_difference_path {
        evidence.push(format!("top_difference_path={path}"));
    }

    evidence
}

fn observer_observations(
    manifest: &DiagnosticScenarioSuiteManifest,
) -> Vec<DiagnosticScenarioSuiteObserverObservation> {
    manifest
        .scenarios
        .iter()
        .map(|scenario| DiagnosticScenarioSuiteObserverObservation {
            scenario_id: scenario.id.to_string(),
            title: scenario.title.to_string(),
            role: observer_scenario_role(manifest, scenario),
            outcome: observer_scenario_outcome(scenario),
            health: scenario.actual_health.clone(),
            expected_passed: scenario.expected_passed,
            actual_passed: scenario.actual_passed,
            contract_all_matched: scenario.contract.all_matched,
            comparison_passed: scenario.comparison.passed,
            focus_test_id: scenario.actual_focus_test_id,
            focus_domain: scenario.actual_focus_domain.clone(),
            failed_probe_ids: scenario.failed_probe_ids.clone(),
            comparison_difference_count: scenario.comparison.difference_count,
            top_difference_path: scenario
                .comparison
                .top_differences
                .first()
                .map(|difference| difference.path.clone()),
            next_artifact: observer_scenario_next_artifact(scenario),
            bundle_manifest: scenario.artifacts.bundle_manifest.clone(),
            triage_json: scenario.artifacts.triage_json.clone(),
            comparison_json: scenario.artifacts.comparison_json.clone(),
            telemetry_json: scenario.artifacts.telemetry_json.clone(),
        })
        .collect()
}

fn observer_scenario_role(
    manifest: &DiagnosticScenarioSuiteManifest,
    scenario: &DiagnosticScenarioSuiteEntry,
) -> &'static str {
    if scenario.id == manifest.baseline_scenario_id {
        "baseline"
    } else if scenario.expected_passed {
        "expected_pass_fixture"
    } else {
        "expected_failure_fixture"
    }
}

fn observer_scenario_outcome(scenario: &DiagnosticScenarioSuiteEntry) -> &'static str {
    if !scenario.contract.all_matched {
        "unexpected_contract_mismatch"
    } else if !scenario.comparison.passed {
        "expected_baseline_divergence"
    } else {
        "matches_baseline"
    }
}

fn observer_scenario_next_artifact(scenario: &DiagnosticScenarioSuiteEntry) -> String {
    if !scenario.contract.all_matched {
        scenario.artifacts.triage_json.clone()
    } else if !scenario.comparison.passed {
        scenario.artifacts.comparison_json.clone()
    } else {
        scenario.artifacts.triage_json.clone()
    }
}

fn scenario_suite_observer_artifact_hints() -> Vec<DiagnosticScenarioSuiteObserverArtifactHint> {
    vec![
        DiagnosticScenarioSuiteObserverArtifactHint {
            path: "scenario-suite-observer.json",
            kind: "observer_json",
            purpose:
                "Machine-readable suite-level next actions, scenario observations, and evidence pointers.",
        },
        DiagnosticScenarioSuiteObserverArtifactHint {
            path: "scenario-suite-observer.md",
            kind: "observer_report",
            purpose: "Compact human-readable observer briefing for CI and local debugging.",
        },
        DiagnosticScenarioSuiteObserverArtifactHint {
            path: "scenario-suite.json",
            kind: "scenario_suite_manifest",
            purpose: "Full root suite manifest with contract details and attention queue.",
        },
        DiagnosticScenarioSuiteObserverArtifactHint {
            path: "scenario-suite.md",
            kind: "scenario_suite_report",
            purpose: "Full Markdown suite report with matrices and artifact map.",
        },
    ]
}

fn scenario_suite_observer_ai_handoff() -> Vec<String> {
    vec![
        "Start with next_actions; critical items are regressions, known_divergence items are intentional fixture drilldowns.".to_string(),
        "Use observations to identify the scenario role, outcome, health, focus domain, failed probes, and first artifact to open.".to_string(),
        "Open primary_artifact before telemetry_json unless the action evidence says raw events or instruction trace context is required.".to_string(),
    ]
}

fn scenario_suite_analysis(
    scenarios: &[DiagnosticScenarioSuiteEntry],
) -> DiagnosticScenarioSuiteAnalysis {
    let scenario_count = scenarios.len();
    let expectation_met_count = scenarios
        .iter()
        .filter(|scenario| scenario.expectation_met)
        .count();
    let expectation_mismatch_count = scenario_count.saturating_sub(expectation_met_count);
    let contract_mismatch_count = scenarios
        .iter()
        .filter(|scenario| !scenario.contract.all_matched)
        .count();
    let baseline_divergence_count = scenarios
        .iter()
        .filter(|scenario| !scenario.comparison.passed)
        .count();
    let critical_scenario_ids = scenarios
        .iter()
        .filter(|scenario| !scenario.contract.all_matched)
        .map(|scenario| scenario.id.to_string())
        .collect();
    let known_divergence_scenario_ids = scenarios
        .iter()
        .filter(|scenario| scenario.contract.all_matched && !scenario.comparison.passed)
        .map(|scenario| scenario.id.to_string())
        .collect();
    let mut attention_queue = scenarios
        .iter()
        .filter_map(scenario_attention_item)
        .collect::<Vec<_>>();
    attention_queue.sort_by(|left, right| {
        attention_priority_rank(left.priority)
            .cmp(&attention_priority_rank(right.priority))
            .then_with(|| {
                right
                    .comparison_difference_count
                    .cmp(&left.comparison_difference_count)
            })
            .then_with(|| left.scenario_id.cmp(&right.scenario_id))
    });

    let status = if contract_mismatch_count == 0 {
        "passed"
    } else {
        "attention_required"
    };
    let summary = format!(
        "{expectation_met_count}/{scenario_count} scenario contracts matched; {contract_mismatch_count} contract mismatches; {baseline_divergence_count} baseline divergences indexed for drilldown."
    );

    DiagnosticScenarioSuiteAnalysis {
        status,
        summary,
        scenario_count,
        expectation_met_count,
        expectation_mismatch_count,
        contract_mismatch_count,
        baseline_divergence_count,
        critical_scenario_ids,
        known_divergence_scenario_ids,
        attention_queue,
    }
}

fn scenario_attention_item(
    scenario: &DiagnosticScenarioSuiteEntry,
) -> Option<DiagnosticScenarioSuiteAttentionItem> {
    let (priority, reason, next_artifact) = if !scenario.contract.all_matched {
        (
            "critical",
            "scenario_contract_mismatch",
            format!("{}/triage.json", scenario.id),
        )
    } else if !scenario.comparison.passed {
        (
            "known_divergence",
            "scenario_diverges_from_pass_baseline",
            format!("{}/comparison.json", scenario.id),
        )
    } else {
        return None;
    };

    Some(DiagnosticScenarioSuiteAttentionItem {
        scenario_id: scenario.id.to_string(),
        priority,
        reason,
        next_artifact,
        expectation_met: scenario.expectation_met,
        contract_all_matched: scenario.contract.all_matched,
        comparison_passed: scenario.comparison.passed,
        actual_health: scenario.actual_health.clone(),
        focus_domain: scenario.actual_focus_domain.clone(),
        failed_probe_ids: scenario.failed_probe_ids.clone(),
        comparison_difference_count: scenario.comparison.difference_count,
        top_difference_path: scenario
            .comparison
            .top_differences
            .first()
            .map(|difference| difference.path.clone()),
    })
}

fn attention_priority_rank(priority: &str) -> u8 {
    match priority {
        "critical" => 0,
        "known_divergence" => 1,
        _ => 2,
    }
}

fn format_scenario_suite_report(manifest: &DiagnosticScenarioSuiteManifest) -> String {
    let mut report = String::new();
    writeln!(report, "# Diagnostic Scenario Suite").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(report, "| Suite | {} |", manifest.suite_name).expect("write report");
    writeln!(report, "| Version | {} |", manifest.suite_version).expect("write report");
    writeln!(
        report,
        "| Scenario suite schema | {} |",
        manifest.scenario_suite_schema_version
    )
    .expect("write report");
    writeln!(
        report,
        "| Telemetry schema | {} |",
        manifest.telemetry_schema_version
    )
    .expect("write report");
    writeln!(
        report,
        "| Triage schema | {} |",
        manifest.triage_schema_version
    )
    .expect("write report");
    writeln!(
        report,
        "| Baseline scenario | {} |",
        manifest.baseline_scenario_id
    )
    .expect("write report");
    writeln!(report, "| Scenario count | {} |", manifest.scenario_count).expect("write report");
    writeln!(report, "| Passed | {} |", manifest.passed).expect("write report");
    writeln!(
        report,
        "| Recommended exit code | {} |",
        manifest.recommended_exit_code
    )
    .expect("write report");

    writeln!(report).expect("write report");
    writeln!(report, "## Suite Analysis").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(report, "| Status | {} |", manifest.analysis.status).expect("write report");
    writeln!(
        report,
        "| Summary | {} |",
        markdown_cell(&manifest.analysis.summary)
    )
    .expect("write report");
    writeln!(
        report,
        "| Contract mismatches | {} |",
        manifest.analysis.contract_mismatch_count
    )
    .expect("write report");
    writeln!(
        report,
        "| Baseline divergences | {} |",
        manifest.analysis.baseline_divergence_count
    )
    .expect("write report");

    writeln!(report).expect("write report");
    writeln!(report, "## Attention Queue").expect("write report");
    writeln!(report).expect("write report");
    if manifest.analysis.attention_queue.is_empty() {
        writeln!(report, "No attention items.").expect("write report");
    } else {
        writeln!(
            report,
            "| Priority | Scenario | Reason | Health | Focus domain | Differences | Next artifact |"
        )
        .expect("write report");
        writeln!(report, "| --- | --- | --- | --- | --- | --- | --- |").expect("write report");
        for item in &manifest.analysis.attention_queue {
            writeln!(
                report,
                "| {} | {} | {} | {} | {} | {} | {} |",
                item.priority,
                item.scenario_id,
                item.reason,
                item.actual_health,
                item.focus_domain.as_deref().unwrap_or("-"),
                item.comparison_difference_count,
                item.next_artifact
            )
            .expect("write report");
        }
    }

    writeln!(report).expect("write report");
    writeln!(report, "## Scenario Matrix").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "| Scenario | Expected pass | Actual pass | Expectation met | Health | Focus test | Focus domain | Comparison | Failed probes | Bundle |"
    )
    .expect("write report");
    writeln!(
        report,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    )
    .expect("write report");
    for scenario in &manifest.scenarios {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            scenario.id,
            scenario.expected_passed,
            scenario.actual_passed,
            scenario.expectation_met,
            scenario.actual_health,
            scenario.actual_focus_test_id,
            scenario.actual_focus_domain.as_deref().unwrap_or("-"),
            format_comparison_cell(&scenario.comparison),
            format_failed_probe_ids(&scenario.failed_probe_ids),
            scenario.directory
        )
        .expect("write report");
    }

    writeln!(report).expect("write report");
    writeln!(report, "## Contract Matrix").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "| Scenario | Passed | Health | Focus test | Focus domain | All |"
    )
    .expect("write report");
    writeln!(report, "| --- | --- | --- | --- | --- | --- |").expect("write report");
    for scenario in &manifest.scenarios {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} |",
            scenario.id,
            scenario.contract.passed_matches,
            scenario.contract.health_matches,
            scenario.contract.focus_test_matches,
            scenario.contract.focus_domain_matches,
            scenario.contract.all_matched
        )
        .expect("write report");
    }

    writeln!(report).expect("write report");
    writeln!(report, "## AI Drilldown").expect("write report");
    writeln!(report).expect("write report");
    for instruction in &manifest.ai_handoff {
        writeln!(report, "- {instruction}").expect("write report");
    }
    writeln!(
        report,
        "- Use each root scenario `comparison` summary to decide whether to inspect `<scenario>/comparison.json` before raw telemetry."
    )
    .expect("write report");
    writeln!(
        report,
        "- For each scenario, inspect `<scenario>/triage.json`, then `<scenario>/comparison.json`, then `<scenario>/telemetry.json` when raw events are needed."
    )
    .expect("write report");

    writeln!(report).expect("write report");
    writeln!(report, "## Baseline Comparison Matrix").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "| Scenario | Passed | Differences | Failures | Warnings | Info | Top difference |"
    )
    .expect("write report");
    writeln!(report, "| --- | --- | --- | --- | --- | --- | --- |").expect("write report");
    for scenario in &manifest.scenarios {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} | {} |",
            scenario.id,
            scenario.comparison.passed,
            scenario.comparison.difference_count,
            scenario.comparison.failure_count,
            scenario.comparison.warning_count,
            scenario.comparison.info_count,
            markdown_cell(&format_top_difference(&scenario.comparison.top_differences))
        )
        .expect("write report");
    }

    writeln!(report).expect("write report");
    writeln!(report, "## Artifact Map").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "| Scenario | Manifest | Triage | Telemetry | Report | Comparison |"
    )
    .expect("write report");
    writeln!(report, "| --- | --- | --- | --- | --- | --- |").expect("write report");
    for scenario in &manifest.scenarios {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} |",
            scenario.id,
            scenario.artifacts.bundle_manifest,
            scenario.artifacts.triage_json,
            scenario.artifacts.telemetry_json,
            scenario.artifacts.report_md,
            scenario.artifacts.comparison_json
        )
        .expect("write report");
    }
    report
}

fn format_scenario_suite_observer_report(
    observer: &DiagnosticScenarioSuiteObserverReport,
) -> String {
    let mut report = String::new();
    writeln!(report, "# Diagnostic Scenario Suite Observer").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Field | Value |").expect("write report");
    writeln!(report, "| --- | --- |").expect("write report");
    writeln!(report, "| Suite | {} |", observer.suite_name).expect("write report");
    writeln!(report, "| Version | {} |", observer.suite_version).expect("write report");
    writeln!(
        report,
        "| Observer schema | {} |",
        observer.observer_schema_version
    )
    .expect("write report");
    writeln!(
        report,
        "| Scenario suite schema | {} |",
        observer.scenario_suite_schema_version
    )
    .expect("write report");
    writeln!(report, "| Status | {} |", observer.status).expect("write report");
    writeln!(report, "| Summary | {} |", markdown_cell(&observer.summary)).expect("write report");
    writeln!(
        report,
        "| Recommended exit code | {} |",
        observer.recommended_exit_code
    )
    .expect("write report");

    writeln!(report).expect("write report");
    writeln!(report, "## Next Actions").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "| Priority | Action | Scenario | Primary artifact | Evidence |"
    )
    .expect("write report");
    writeln!(report, "| --- | --- | --- | --- | --- |").expect("write report");
    for action in &observer.next_actions {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} |",
            action.priority,
            action.action_type,
            action.scenario_id,
            action.primary_artifact,
            markdown_cell(&action.evidence.join("<br>"))
        )
        .expect("write report");
    }

    writeln!(report).expect("write report");
    writeln!(report, "## Observations").expect("write report");
    writeln!(report).expect("write report");
    writeln!(
        report,
        "| Scenario | Role | Outcome | Health | Focus domain | Differences | Next artifact |"
    )
    .expect("write report");
    writeln!(report, "| --- | --- | --- | --- | --- | --- | --- |").expect("write report");
    for observation in &observer.observations {
        writeln!(
            report,
            "| {} | {} | {} | {} | {} | {} | {} |",
            observation.scenario_id,
            observation.role,
            observation.outcome,
            observation.health,
            observation.focus_domain.as_deref().unwrap_or("-"),
            observation.comparison_difference_count,
            observation.next_artifact
        )
        .expect("write report");
    }

    writeln!(report).expect("write report");
    writeln!(report, "## Artifact Hints").expect("write report");
    writeln!(report).expect("write report");
    writeln!(report, "| Path | Kind | Purpose |").expect("write report");
    writeln!(report, "| --- | --- | --- |").expect("write report");
    for hint in &observer.artifact_hints {
        writeln!(
            report,
            "| {} | {} | {} |",
            hint.path,
            hint.kind,
            markdown_cell(hint.purpose)
        )
        .expect("write report");
    }

    writeln!(report).expect("write report");
    writeln!(report, "## AI Handoff").expect("write report");
    writeln!(report).expect("write report");
    for instruction in &observer.ai_handoff {
        writeln!(report, "- {instruction}").expect("write report");
    }

    report
}

fn format_failed_probe_ids(ids: &[String]) -> String {
    if ids.is_empty() {
        "-".to_string()
    } else {
        markdown_cell(&ids.join("<br>"))
    }
}

fn format_comparison_cell(comparison: &DiagnosticTriageComparison) -> String {
    let status = if comparison.passed { "pass" } else { "fail" };
    format!("{}; {} differences", status, comparison.difference_count)
}

fn format_top_difference(differences: &[DiagnosticTriageDifference]) -> String {
    differences
        .first()
        .map(|difference| {
            format!(
                "{} {} {}",
                difference.severity, difference.category, difference.path
            )
        })
        .unwrap_or_else(|| "-".to_string())
}

fn markdown_cell(value: &str) -> String {
    value.replace('|', r"\|").replace(['\r', '\n'], " ")
}

fn write_scenario_bundle(
    suite_dir: &Path,
    spec: &DiagnosticScenarioSpec,
    telemetry: DiagnosticTelemetry,
    baseline_json: &str,
    diagnostic_rom: &[u8],
) -> Result<DiagnosticScenarioSuiteEntry, String> {
    let telemetry_json = serde_json::to_string_pretty(&telemetry)
        .map_err(|err| format!("failed to serialize {} scenario telemetry: {err}", spec.id))?;
    let comparison = compare_diagnostic_to_baseline(&telemetry, baseline_json)?;
    let scenario_passed = telemetry.verdict.passed && comparison.passed;
    let bundle_dir = suite_dir.join(spec.id);
    let config = diagnostic_bundle_config(&spec.config);

    write_bundle(
        &bundle_dir,
        DiagnosticBundleInput {
            telemetry: &telemetry,
            telemetry_json: &telemetry_json,
            diagnostic_rom,
            comparison: Some(&comparison),
            passed: scenario_passed,
            config: config.clone(),
        },
    )?;

    let focus = &telemetry.analysis.debug_focus;
    let actual_health = json_label(&telemetry.analysis.health)?;
    let expected_health = json_label(&spec.expected_health)?;
    let failure_kind = focus
        .failure_kind
        .map(|kind| json_label(&kind))
        .transpose()?;
    let focus_test_matches = spec
        .expected_focus_test_id
        .is_none_or(|test_id| focus.focus_test_id == test_id);
    let focus_domain_matches = spec
        .expected_focus_domain
        .is_none_or(|domain| focus.focus_domain.as_deref() == Some(domain));
    let passed_matches = telemetry.verdict.passed == spec.expected_passed;
    let health_matches = telemetry.analysis.health == spec.expected_health;
    let expectation_met =
        passed_matches && health_matches && focus_test_matches && focus_domain_matches;
    let comparison_summary = triage_comparison(&comparison)?;
    let contract = DiagnosticScenarioSuiteContract {
        expected_passed: spec.expected_passed,
        actual_passed: telemetry.verdict.passed,
        passed_matches,
        expected_health: expected_health.clone(),
        actual_health: actual_health.clone(),
        health_matches,
        expected_focus_test_id: spec.expected_focus_test_id,
        actual_focus_test_id: focus.focus_test_id,
        focus_test_matches,
        expected_focus_domain: spec.expected_focus_domain,
        actual_focus_domain: focus.focus_domain.clone(),
        focus_domain_matches,
        all_matched: expectation_met,
    };

    Ok(DiagnosticScenarioSuiteEntry {
        id: spec.id,
        title: spec.title,
        purpose: spec.purpose,
        directory: spec.id.to_string(),
        expected_runner_exit_code: if spec.expected_passed { 0 } else { 1 },
        expected_passed: spec.expected_passed,
        actual_passed: telemetry.verdict.passed,
        expected_health,
        actual_health,
        expected_focus_test_id: spec.expected_focus_test_id,
        actual_focus_test_id: focus.focus_test_id,
        actual_focus_test_name: focus.focus_test_name,
        expected_focus_domain: spec.expected_focus_domain,
        actual_focus_domain: focus.focus_domain.clone(),
        failure_kind,
        failure_code_hex: focus.failure_code_hex.clone(),
        failed_probe_ids: focus.failed_probe_ids.clone(),
        comparison: comparison_summary,
        contract,
        expectation_met,
        config,
        artifacts: DiagnosticScenarioSuiteArtifacts {
            bundle_manifest: format!("{}/manifest.json", spec.id),
            triage_json: format!("{}/triage.json", spec.id),
            telemetry_json: format!("{}/telemetry.json", spec.id),
            report_md: format!("{}/report.md", spec.id),
            comparison_json: format!("{}/comparison.json", spec.id),
            comparison_report: format!("{}/comparison.md", spec.id),
            diagnostic_rom: format!("{}/diagnostic.nes", spec.id),
        },
    })
}

fn diagnostic_scenario_specs() -> Vec<DiagnosticScenarioSpec> {
    let default = DiagnosticConfig::default();
    vec![
        DiagnosticScenarioSpec {
            id: "pass",
            title: "Known-good generated cartridge pass",
            purpose: "Baseline diagnostic bundle for comparison and healthy debug-focus shape.",
            config: default.clone(),
            expected_passed: true,
            expected_health: DiagnosticHealth::Healthy,
            expected_focus_test_id: Some(13),
            expected_focus_domain: None,
        },
        DiagnosticScenarioSpec {
            id: "joypad1_mismatch",
            title: "Intentional joypad-1 assertion failure",
            purpose: "Failure-localization fixture for $4016 strobe/shift regressions.",
            config: DiagnosticConfig {
                joypad1_mask: 0x00,
                ..default.clone()
            },
            expected_passed: false,
            expected_health: DiagnosticHealth::CartridgeAssertionFailed,
            expected_focus_test_id: Some(7),
            expected_focus_domain: Some("joypad.strobe_shift"),
        },
        DiagnosticScenarioSpec {
            id: "joypad2_mismatch",
            title: "Intentional joypad-2 assertion failure",
            purpose: "Failure-localization fixture for $4017 player-2 strobe/shift regressions.",
            config: DiagnosticConfig {
                joypad2_mask: 0x00,
                ..default.clone()
            },
            expected_passed: false,
            expected_health: DiagnosticHealth::CartridgeAssertionFailed,
            expected_focus_test_id: Some(11),
            expected_focus_domain: Some("joypad2.strobe_shift"),
        },
        DiagnosticScenarioSpec {
            id: "timeout_cycle_limit",
            title: "Intentional one-cycle timeout",
            purpose: "Progress watchdog fixture for runs that fail before the cartridge can start a test.",
            config: DiagnosticConfig {
                max_cpu_cycles: 1,
                ..default
            },
            expected_passed: false,
            expected_health: DiagnosticHealth::TimedOut,
            expected_focus_test_id: None,
            expected_focus_domain: Some("emulator.progress_or_infinite_loop"),
        },
    ]
}

fn scenario_suite_ai_handoff() -> Vec<String> {
    vec![
        "Start with scenario-suite-observer.json next_actions, then use scenario-suite.json analysis.attention_queue when full scenario contract detail is needed.".to_string(),
        "Use each scenario contract object to see whether pass state, health, focus test, or focus domain caused an expectation mismatch.".to_string(),
        "Use pass/ as the known-good baseline bundle; every scenario bundle includes comparison.json against that baseline.".to_string(),
        "For failures, open each scenario triage.json debug_focus before loading telemetry.json, report.md, or comparison.json.".to_string(),
    ]
}

fn diagnostic_bundle_config(config: &DiagnosticConfig) -> DiagnosticBundleConfig {
    DiagnosticBundleConfig {
        max_cpu_cycles: config.max_cpu_cycles,
        joypad1_mask: config.joypad1_mask,
        joypad1_mask_hex: hex_byte(config.joypad1_mask),
        joypad2_mask: config.joypad2_mask,
        joypad2_mask_hex: hex_byte(config.joypad2_mask),
    }
}

fn write_bundle(path: &Path, input: DiagnosticBundleInput<'_>) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;

    let mut artifacts = Vec::new();
    let triage_json = diagnostic_triage_json(input.telemetry, input.comparison)?;
    artifacts.push(write_bundle_artifact(
        path,
        "triage.json",
        "ai_triage_json",
        triage_json.as_bytes(),
    )?);

    artifacts.push(write_bundle_artifact(
        path,
        "telemetry.json",
        "telemetry_json",
        input.telemetry_json.as_bytes(),
    )?);

    let report = format_diagnostic_report(input.telemetry);
    artifacts.push(write_bundle_artifact(
        path,
        "report.md",
        "diagnostic_report",
        report.as_bytes(),
    )?);

    artifacts.push(write_bundle_artifact(
        path,
        "diagnostic.nes",
        "diagnostic_cartridge",
        input.diagnostic_rom,
    )?);

    if let Some(comparison) = input.comparison {
        let comparison_json = serde_json::to_string_pretty(comparison)
            .map_err(|err| format!("failed to serialize comparison telemetry: {err}"))?;
        artifacts.push(write_bundle_artifact(
            path,
            "comparison.json",
            "comparison_json",
            comparison_json.as_bytes(),
        )?);

        let comparison_report = format_diagnostic_comparison_report(comparison);
        artifacts.push(write_bundle_artifact(
            path,
            "comparison.md",
            "comparison_report",
            comparison_report.as_bytes(),
        )?);
    }

    let manifest = DiagnosticBundleManifest {
        bundle_schema_version: DIAGNOSTIC_BUNDLE_SCHEMA_VERSION,
        telemetry_schema_version: input.telemetry.schema_version,
        suite_name: input.telemetry.suite.name.to_string(),
        suite_version: input.telemetry.suite.version.to_string(),
        passed: input.passed,
        recommended_exit_code: if input.passed { 0 } else { 1 },
        config: input.config,
        comparison_included: input.comparison.is_some(),
        artifacts,
        ai_handoff: bundle_ai_handoff(input.telemetry.verdict.passed, input.comparison),
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|err| format!("failed to serialize bundle manifest: {err}"))?;
    write_file(&path.join("manifest.json"), manifest_json.as_bytes())
}

fn bundle_ai_handoff(
    telemetry_passed: bool,
    comparison: Option<&DiagnosticComparisonTelemetry>,
) -> Vec<String> {
    let mut handoff = vec![
        "Start with manifest.json to verify artifact hashes and bundle result.".to_string(),
        "Read triage.json debug_focus first for the compact failing test, probes, last event, and instruction anchors before loading full telemetry.".to_string(),
        "Read report.md for human triage and telemetry.json for exact probe, timeline, event, instruction trace, and execution-snapshot data.".to_string(),
    ];
    if comparison.is_some() {
        handoff.push(
            "Read comparison.json before raw telemetry when a baseline was supplied.".to_string(),
        );
    }
    if !telemetry_passed {
        handoff.push(
            "Prioritize analysis.debug_focus, verdict.failure, analysis.probe_summary, failed probes, instruction_trace.tail, and the event tail."
                .to_string(),
        );
    }
    handoff
}

fn diagnostic_triage_json(
    telemetry: &DiagnosticTelemetry,
    comparison: Option<&DiagnosticComparisonTelemetry>,
) -> Result<String, String> {
    let triage = diagnostic_triage_report(telemetry, comparison)?;
    serde_json::to_string_pretty(&triage)
        .map_err(|err| format!("failed to serialize diagnostic triage JSON: {err}"))
}

fn diagnostic_triage_report(
    telemetry: &DiagnosticTelemetry,
    comparison: Option<&DiagnosticComparisonTelemetry>,
) -> Result<DiagnosticTriageReport, String> {
    let passed = telemetry.verdict.passed
        && comparison
            .as_ref()
            .is_none_or(|comparison| comparison.passed);

    Ok(DiagnosticTriageReport {
        triage_schema_version: DIAGNOSTIC_TRIAGE_SCHEMA_VERSION,
        telemetry_schema_version: telemetry.schema_version,
        suite_name: telemetry.suite.name.to_string(),
        suite_version: telemetry.suite.version.to_string(),
        passed,
        recommended_exit_code: if passed { 0 } else { 1 },
        health: json_label(&telemetry.analysis.health)?,
        summary: telemetry.analysis.summary.clone(),
        current_test: DiagnosticTriageCurrentTest {
            id: telemetry.verdict.current_test,
            name: telemetry.verdict.current_test_name,
            status_hex: hex_byte(telemetry.verdict.status),
            failure_code_hex: hex_byte(telemetry.verdict.failure_code),
            timed_out: telemetry.verdict.timeout,
        },
        debug_focus: triage_debug_focus(telemetry)?,
        input: DiagnosticTriageInput {
            joypad1_mask_hex: telemetry.input.joypad1_mask_hex.clone(),
            joypad1_expected_mask_hex: telemetry.input.joypad1_expected_mask_hex.clone(),
            joypad2_mask_hex: telemetry.input.joypad2_mask_hex.clone(),
            joypad2_expected_mask_hex: telemetry.input.joypad2_expected_mask_hex.clone(),
        },
        failure: triage_failure(telemetry)?,
        coverage: triage_coverage(telemetry)?,
        coverage_gaps: triage_coverage_gaps(telemetry),
        dma: DiagnosticTriageDma {
            oam_dma_observed: telemetry.dma.oam_dma_observed,
            oam_dma_completed: telemetry.dma.oam_dma_completed,
            oam_dma_active_cycles: telemetry.dma.oam_dma_active_cycles,
            oam_dma_expected_min_cycles: telemetry.dma.oam_dma_expected_min_cycles,
            oam_dma_expected_max_cycles: telemetry.dma.oam_dma_expected_max_cycles,
            oam_dma_start_cycle: telemetry.dma.oam_dma_start_cycle,
            oam_dma_end_cycle: telemetry.dma.oam_dma_end_cycle,
            oam_dma_first_active_cycle: telemetry.dma.oam_dma_first_active_cycle,
            oam_dma_first_active_cycle_parity: telemetry.dma.oam_dma_first_active_cycle_parity,
            oam_dma_start_test_name: telemetry.dma.oam_dma_start_test_name,
            oam_dma_end_test_name: telemetry.dma.oam_dma_end_test_name,
            dmc_dma_fetches_observed: telemetry.dma.dmc_dma_fetches_observed,
            dmc_dma_fetches_during_oam_dma: telemetry.dma.dmc_dma_fetches_during_oam_dma,
            dmc_dma_expected_min_oam_overlap_fetches: telemetry
                .dma
                .dmc_dma_expected_min_oam_overlap_fetches,
            dmc_dma_oam_overlap_observed: telemetry.dma.dmc_dma_oam_overlap_observed,
            dmc_dma_first_fetch_cycle: telemetry.dma.dmc_dma_first_fetch_cycle,
            dmc_dma_first_fetch_address: telemetry.dma.dmc_dma_first_fetch_address,
            dmc_dma_first_fetch_cpu_cycle_parity: telemetry
                .dma
                .dmc_dma_first_fetch_cpu_cycle_parity,
            dmc_dma_first_fetch_stall_cycles: telemetry.dma.dmc_dma_first_fetch_stall_cycles,
            dmc_dma_first_oam_overlap_cycle: telemetry.dma.dmc_dma_first_oam_overlap_cycle,
            dmc_dma_first_oam_overlap_test_name: telemetry.dma.dmc_dma_first_oam_overlap_test_name,
            dmc_dma_first_oam_overlap_cpu_cycle_parity: telemetry
                .dma
                .dmc_dma_first_oam_overlap_cpu_cycle_parity,
            dmc_dma_first_oam_overlap_stall_cycles: telemetry
                .dma
                .dmc_dma_first_oam_overlap_stall_cycles,
            dmc_dma_three_cycle_fetches: telemetry.dma.dmc_dma_three_cycle_fetches,
            dmc_dma_four_cycle_fetches: telemetry.dma.dmc_dma_four_cycle_fetches,
            dmc_dma_expected_min_stall_cycles: telemetry.dma.dmc_dma_expected_min_stall_cycles,
            dmc_dma_expected_max_stall_cycles: telemetry.dma.dmc_dma_expected_max_stall_cycles,
            dmc_dma_stall_cycles: telemetry.dma.dmc_dma_stall_cycles,
            dmc_dma_stall_cycles_after_oam_dma: telemetry.dma.dmc_dma_stall_cycles_after_oam_dma,
            dmc_dma_queued_during_oam_dma_cycles: telemetry
                .dma
                .dmc_dma_queued_during_oam_dma_cycles,
        },
        probes: triage_probe_summary(telemetry)?,
        timing: triage_timing(telemetry)?,
        instruction_trace: triage_instruction_trace(telemetry),
        comparison: comparison.map(triage_comparison).transpose()?,
        next_actions: telemetry.analysis.next_actions.clone(),
        artifact_hints: triage_artifact_hints(comparison.is_some()),
        event_tail: triage_event_tail(telemetry)?,
    })
}

fn triage_debug_focus(
    telemetry: &DiagnosticTelemetry,
) -> Result<DiagnosticTriageDebugFocus, String> {
    let focus = &telemetry.analysis.debug_focus;
    Ok(DiagnosticTriageDebugFocus {
        health: json_label(&focus.health)?,
        focus_test_id: focus.focus_test_id,
        focus_test_name: focus.focus_test_name,
        focus_subsystem: focus
            .focus_subsystem
            .map(|subsystem| json_label(&subsystem))
            .transpose()?,
        focus_domain: focus.focus_domain.clone(),
        failure_kind: focus
            .failure_kind
            .map(|kind| json_label(&kind))
            .transpose()?,
        failure_code_hex: focus.failure_code_hex.clone(),
        failed_probe_ids: focus.failed_probe_ids.clone(),
        skipped_probe_count: focus.skipped_probe_count,
        last_event: focus
            .last_event
            .as_ref()
            .map(triage_debug_event_focus)
            .transpose()?,
        terminal_instruction: focus
            .terminal_instruction
            .as_ref()
            .map(triage_debug_instruction_focus),
        last_test_instruction: focus
            .last_test_instruction
            .as_ref()
            .map(triage_debug_instruction_focus),
    })
}

fn triage_debug_event_focus(
    event: &DiagnosticDebugEventFocusTelemetry,
) -> Result<DiagnosticTriageDebugEventFocus, String> {
    Ok(DiagnosticTriageDebugEventFocus {
        kind: json_label(&event.kind)?,
        cycle: event.cycle,
        frame: event.frame,
        status_hex: event.status_hex.clone(),
        current_test: event.current_test,
        current_test_name: event.current_test_name,
        pc_hex: event.pc_hex.clone(),
        note: event.note.clone(),
    })
}

fn triage_debug_instruction_focus(
    instruction: &DiagnosticDebugInstructionFocusTelemetry,
) -> DiagnosticTriageDebugInstructionFocus {
    DiagnosticTriageDebugInstructionFocus {
        sequence: instruction.sequence,
        cycle: instruction.cycle,
        frame: instruction.frame,
        current_test: instruction.current_test,
        current_test_name: instruction.current_test_name,
        pc_hex: instruction.pc_hex.clone(),
        instruction: instruction.instruction.clone(),
        symbol: instruction.symbol.clone(),
        status_hex: instruction.status_hex.clone(),
        current_result_hex: instruction.current_result_hex.clone(),
        failure_code_hex: instruction.failure_code_hex.clone(),
    }
}

fn triage_failure(
    telemetry: &DiagnosticTelemetry,
) -> Result<Option<DiagnosticTriageFailure>, String> {
    telemetry
        .verdict
        .failure
        .as_ref()
        .map(|failure| {
            Ok(DiagnosticTriageFailure {
                kind: json_label(&failure.kind)?,
                test_id: failure.test_id,
                test_name: failure.test_name,
                subsystem: failure
                    .subsystem
                    .map(|subsystem| json_label(&subsystem))
                    .transpose()?,
                tier: failure.tier.map(|tier| json_label(&tier)).transpose()?,
                failure_code_hex: failure.failure_code_hex.clone(),
                assertion: failure.assertion.clone(),
                expected: failure.expected.clone(),
                observed: failure.observed.clone(),
                likely_domain: failure.likely_domain.clone(),
                remediation_hint: failure.remediation_hint.clone(),
            })
        })
        .transpose()
}

fn triage_coverage(telemetry: &DiagnosticTelemetry) -> Result<DiagnosticTriageCoverage, String> {
    let coverage = &telemetry.analysis.coverage;
    Ok(DiagnosticTriageCoverage {
        total_tests: coverage.total_tests,
        passed_tests: coverage.passed_tests,
        failed_tests: coverage.failed_tests,
        subsystems: coverage
            .subsystem_summary
            .iter()
            .map(|entry| {
                Ok(DiagnosticTriageSubsystemCoverage {
                    subsystem: json_label(&entry.subsystem)?,
                    passed: entry.passed,
                    total: entry.total,
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

fn triage_coverage_gaps(telemetry: &DiagnosticTelemetry) -> Vec<DiagnosticTriageCoverageGap> {
    telemetry
        .analysis
        .coverage_gaps
        .iter()
        .map(|gap| DiagnosticTriageCoverageGap {
            id: gap.id,
            subsystem: gap.subsystem,
            risk: gap.risk,
            current_coverage: gap.current_coverage,
            missing_coverage: gap.missing_coverage,
            suggested_next_test: gap.suggested_next_test,
        })
        .collect()
}

fn triage_probe_summary(
    telemetry: &DiagnosticTelemetry,
) -> Result<DiagnosticTriageProbeSummary, String> {
    let summary = &telemetry.analysis.probe_summary;
    Ok(DiagnosticTriageProbeSummary {
        total_probes: summary.total_probes,
        passed_probes: summary.passed_probes,
        failed_probes: summary.failed_probes,
        skipped_probes: summary.skipped_probes,
        first_failed_probe: summary.first_failed_probe.clone(),
        failed: telemetry
            .probes
            .iter()
            .filter(|probe| probe.status == DiagnosticProbeStatus::Failed)
            .take(8)
            .map(triage_probe)
            .collect::<Result<Vec<_>, String>>()?,
        skipped: telemetry
            .probes
            .iter()
            .filter(|probe| probe.status == DiagnosticProbeStatus::Skipped)
            .take(8)
            .map(triage_probe)
            .collect::<Result<Vec<_>, String>>()?,
    })
}

fn triage_probe(
    probe: &oxidenes::diagnostic::DiagnosticProbeTelemetry,
) -> Result<DiagnosticTriageProbe, String> {
    Ok(DiagnosticTriageProbe {
        id: probe.id.clone(),
        source: json_label(&probe.source)?,
        subsystem: probe
            .subsystem
            .map(|subsystem| json_label(&subsystem))
            .transpose()?,
        test_id: probe.test_id,
        test_name: probe.test_name,
        expected: probe.expected.clone(),
        observed: probe.observed.clone(),
        likely_domain: probe.likely_domain.clone(),
    })
}

fn triage_timing(telemetry: &DiagnosticTelemetry) -> Result<DiagnosticTriageTiming, String> {
    let timing = &telemetry.analysis.timing;
    Ok(DiagnosticTriageTiming {
        started_tests: timing.started_tests,
        ended_tests: timing.ended_tests,
        not_started_tests: timing.not_started_tests,
        timed_out_tests: timing.timed_out_tests,
        slowest_test: timing
            .slowest_test
            .as_ref()
            .map(|test| {
                Ok::<DiagnosticTriageSlowestTest, String>(DiagnosticTriageSlowestTest {
                    test_id: test.test_id,
                    test_name: test.test_name,
                    subsystem: json_label(&test.subsystem)?,
                    tier: json_label(&test.tier)?,
                    duration_cycles: test.duration_cycles,
                    duration_frames: test.duration_frames,
                })
            })
            .transpose()?,
    })
}

fn triage_instruction_trace(telemetry: &DiagnosticTelemetry) -> DiagnosticTriageInstructionTrace {
    let start = telemetry.instruction_trace.tail.len().saturating_sub(16);
    let tail = telemetry.instruction_trace.tail[start..]
        .iter()
        .map(|entry| DiagnosticTriageInstructionTraceEntry {
            sequence: entry.sequence,
            cycle: entry.cycle,
            frame: entry.frame,
            current_test: entry.diagnostic_ram.current_test,
            current_test_name: entry.diagnostic_ram.current_test_name,
            pc_hex: entry.pc_hex.clone(),
            opcode_hex: entry.opcode_hex.clone(),
            instruction: entry
                .instruction
                .as_ref()
                .map(|instruction| instruction.text.clone()),
            mnemonic: entry
                .instruction
                .as_ref()
                .map(|instruction| instruction.mnemonic),
            addressing_mode: entry
                .instruction
                .as_ref()
                .map(|instruction| instruction.addressing_mode),
            symbol: entry.symbol.as_ref().map(format_symbol),
            symbol_offset_hex: entry
                .symbol
                .as_ref()
                .map(|symbol| symbol.offset_hex.clone()),
            cpu_a_hex: hex_byte(entry.cpu.a),
            cpu_x_hex: hex_byte(entry.cpu.x),
            cpu_y_hex: hex_byte(entry.cpu.y),
            cpu_sp_hex: hex_byte(entry.cpu.sp),
            cpu_status_hex: hex_byte(entry.cpu.status),
            current_result_hex: entry.diagnostic_ram.current_result_hex.clone(),
            failure_code_hex: entry.diagnostic_ram.failure_code_hex.clone(),
        })
        .collect();

    DiagnosticTriageInstructionTrace {
        captured_instruction_count: telemetry.instruction_trace.captured_instruction_count,
        retained_instruction_count: telemetry.instruction_trace.retained_instruction_count,
        retention_limit: telemetry.instruction_trace.retention_limit,
        truncated: telemetry.instruction_trace.truncated,
        triage_tail_count: telemetry.instruction_trace.tail[start..].len(),
        tail,
    }
}

fn triage_comparison(
    comparison: &DiagnosticComparisonTelemetry,
) -> Result<DiagnosticTriageComparison, String> {
    Ok(DiagnosticTriageComparison {
        passed: comparison.passed,
        summary: comparison.summary.clone(),
        difference_count: comparison.difference_count,
        failure_count: comparison.failure_count,
        warning_count: comparison.warning_count,
        info_count: comparison.info_count,
        top_differences: comparison
            .differences
            .iter()
            .take(8)
            .map(|difference| {
                Ok(DiagnosticTriageDifference {
                    severity: json_label(&difference.severity)?,
                    category: difference.category,
                    path: difference.path.clone(),
                    baseline: difference.baseline.clone(),
                    current: difference.current.clone(),
                    note: difference.note.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
    })
}

fn triage_artifact_hints(comparison_included: bool) -> Vec<DiagnosticTriageArtifactHint> {
    let mut hints = vec![
        DiagnosticTriageArtifactHint {
            path: "manifest.json",
            kind: "bundle_manifest",
            purpose: "Verify bundle result, artifact hashes, and handoff guidance.",
        },
        DiagnosticTriageArtifactHint {
            path: "triage.json",
            kind: "ai_triage_json",
            purpose: "Compact machine-readable debug focus, failure summary, probes, trace anchors, and next actions.",
        },
        DiagnosticTriageArtifactHint {
            path: "report.md",
            kind: "diagnostic_report",
            purpose: "Human-readable triage report.",
        },
        DiagnosticTriageArtifactHint {
            path: "telemetry.json",
            kind: "telemetry_json",
            purpose:
                "Full-fidelity telemetry for exact probe, timeline, event, instruction trace, and host state analysis.",
        },
        DiagnosticTriageArtifactHint {
            path: "diagnostic.nes",
            kind: "diagnostic_cartridge",
            purpose: "Generated IP-safe cartridge used for the run.",
        },
    ];

    if comparison_included {
        hints.push(DiagnosticTriageArtifactHint {
            path: "comparison.json",
            kind: "comparison_json",
            purpose: "Machine-readable baseline comparison result.",
        });
        hints.push(DiagnosticTriageArtifactHint {
            path: "comparison.md",
            kind: "comparison_report",
            purpose: "Human-readable baseline comparison report.",
        });
    }

    hints
}

fn triage_event_tail(
    telemetry: &DiagnosticTelemetry,
) -> Result<Vec<DiagnosticTriageEvent>, String> {
    let start = telemetry.events.len().saturating_sub(8);
    telemetry.events[start..]
        .iter()
        .map(|event| {
            Ok(DiagnosticTriageEvent {
                kind: json_label(&event.kind)?,
                cycle: event.cycle,
                frame: event.frame,
                status_hex: hex_byte(event.status),
                current_test: event.current_test,
                current_test_name: event.current_test_name,
                pc_hex: format!("0x{:04X}", event.pc),
                cpu_a_hex: hex_byte(event.cpu.a),
                cpu_x_hex: hex_byte(event.cpu.x),
                cpu_y_hex: hex_byte(event.cpu.y),
                cpu_sp_hex: hex_byte(event.cpu.sp),
                cpu_status_hex: hex_byte(event.cpu.status),
                cpu_pending_cycles: event.cpu.pending_cycles,
                current_result_addr_hex: event.diagnostic_ram.current_result_addr_hex.clone(),
                current_result_hex: event.diagnostic_ram.current_result_hex.clone(),
                failure_code_hex: event.diagnostic_ram.failure_code_hex.clone(),
                signature_hex: event.diagnostic_ram.signature_hex.clone(),
                nmi_count: event.diagnostic_ram.nmi_count,
                note: event.note.clone(),
            })
        })
        .collect()
}

fn json_label<T: Serialize>(value: &T) -> Result<String, String> {
    let value =
        serde_json::to_value(value).map_err(|err| format!("failed to serialize label: {err}"))?;
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("expected serialized label to be a string, got {value}"))
}

fn write_bundle_artifact(
    bundle_dir: &Path,
    file_name: &'static str,
    kind: &'static str,
    data: &[u8],
) -> Result<DiagnosticBundleArtifact, String> {
    let path = bundle_dir.join(file_name);
    write_file(&path, data)?;
    Ok(DiagnosticBundleArtifact {
        path: file_name.to_string(),
        kind,
        bytes: data.len(),
        sha256: sha256_hex(data),
    })
}

fn write_file(path: &Path, data: &[u8]) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(path, data).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn sha256_hex(data: &[u8]) -> String {
    sha256(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_byte(value: u8) -> String {
    format!("0x{value:02X}")
}

fn format_symbol(symbol: &oxidenes::diagnostic::DiagnosticSymbolTelemetry) -> String {
    if symbol.offset == 0 {
        symbol.name.clone()
    } else {
        format!("{}+{}", symbol.name, symbol.offset_hex)
    }
}
