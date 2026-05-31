use std::fs;
use std::path::PathBuf;

use oxidenes::diagnostic::{
    build_diagnostic_cartridge, compare_diagnostic_to_baseline,
    format_diagnostic_comparison_report, format_diagnostic_report, run_diagnostic,
    DiagnosticConfig, DIAGNOSTIC_PROVENANCE,
};

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

    if let Some(path) = dump_rom_path {
        let rom = build_diagnostic_cartridge()?;
        write_file(&path, &rom)?;
    }

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

    if print_stdout {
        println!("{json}");
    }

    Ok(telemetry.verdict.passed
        && comparison
            .as_ref()
            .is_none_or(|comparison| comparison.passed))
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

fn write_file(path: &PathBuf, data: &[u8]) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    fs::write(path, data).map_err(|err| format!("failed to write {}: {err}", path.display()))
}
