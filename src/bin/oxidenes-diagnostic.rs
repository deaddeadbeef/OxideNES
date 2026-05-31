use std::fs;
use std::path::{Path, PathBuf};

use oxidenes::diagnostic::{
    build_diagnostic_cartridge, compare_diagnostic_to_baseline,
    format_diagnostic_comparison_report, format_diagnostic_report, run_diagnostic,
    DiagnosticConfig, DIAGNOSTIC_PROVENANCE,
};
use oxidenes::recording::sha256;
use serde::Serialize;

const DIAGNOSTIC_BUNDLE_SCHEMA_VERSION: u16 = 1;

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

#[derive(Debug, Serialize)]
struct DiagnosticBundleConfig {
    max_cpu_cycles: u64,
    joypad1_mask: u8,
    joypad1_mask_hex: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticBundleArtifact {
    path: String,
    kind: &'static str,
    bytes: usize,
    sha256: String,
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
    let mut dump_rom_path: Option<PathBuf> = None;
    let mut bundle_dir: Option<PathBuf> = None;
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
            "--max-cycles" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--max-cycles requires a number".to_string())?;
                config.max_cpu_cycles = value
                    .parse()
                    .map_err(|_| format!("invalid --max-cycles value: {value}"))?;
            }
            "--joypad1" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--joypad1 requires a byte mask".to_string())?;
                config.joypad1_mask = parse_byte(&value)?;
            }
            "--no-stdout" => {
                print_stdout = false;
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
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

    let config_manifest = DiagnosticBundleConfig {
        max_cpu_cycles: config.max_cpu_cycles,
        joypad1_mask: config.joypad1_mask,
        joypad1_mask_hex: hex_byte(config.joypad1_mask),
    };
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
    println!("    --dump-rom <FILE>    Generate the diagnostic .nes cartridge at runtime");
    println!("    --bundle-dir <DIR>   Write an AI-ready diagnostic artifact bundle");
    println!("    --max-cycles <N>     Override the CPU-cycle timeout");
    println!("    --joypad1 <BYTE>     Override joypad-1 mask, decimal or 0x-prefixed hex");
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
    telemetry: &'a oxidenes::diagnostic::DiagnosticTelemetry,
    telemetry_json: &'a str,
    diagnostic_rom: &'a [u8],
    comparison: Option<&'a oxidenes::diagnostic::DiagnosticComparisonTelemetry>,
    passed: bool,
    config: DiagnosticBundleConfig,
}

fn write_bundle(path: &Path, input: DiagnosticBundleInput<'_>) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;

    let mut artifacts = Vec::new();
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
    comparison: Option<&oxidenes::diagnostic::DiagnosticComparisonTelemetry>,
) -> Vec<String> {
    let mut handoff = vec![
        "Start with manifest.json to verify artifact hashes and bundle result.".to_string(),
        "Read report.md for human triage and telemetry.json for exact probe, timeline, and event data.".to_string(),
    ];
    if comparison.is_some() {
        handoff.push(
            "Read comparison.json before raw telemetry when a baseline was supplied.".to_string(),
        );
    }
    if !telemetry_passed {
        handoff.push(
            "Prioritize verdict.failure, analysis.probe_summary, failed probes, and the event tail."
                .to_string(),
        );
    }
    handoff
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
