//! batlab - Battery Test Harness
//!
//! Cross-platform battery efficiency measurement for FreeBSD vs Linux research.
//! Manual configuration approach - user configures system, tool records data.

use batlab::{
    collect_telemetry, generate_run_id, get_battery_info, get_system_info, BatteryError,
    RunMetadata, TelemetryError, TelemetrySample,
};
use chrono::Utc;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const VERSION: &str = "2.0.0";

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Table,
    Csv,
    Json,
}

#[derive(Parser)]
#[command(name = "batlab")]
#[command(version = VERSION)]
#[command(about = "Battery Test Harness for FreeBSD vs Linux Research")]
#[command(
    long_about = "Cross-platform battery efficiency measurement for FreeBSD vs Linux research.\n\
Manual configuration approach - user configures system, tool records data.\n\n\
WORKFLOW:\n\
1. Manually configure your system power management\n\
2. Terminal 1: batlab log my-config-name\n\
3. Terminal 2: batlab run workload-name\n\
4. Stop both with Ctrl+C when done"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize directories and check system capabilities
    Init,
    /// Start telemetry logging with user-defined configuration name
    Log {
        /// Configuration name (letters, numbers, hyphens, underscores only)
        config_name: String,
        /// Sampling frequency in Hz (0.01-10.0)
        #[arg(long, default_value = "0.0167")]
        hz: f32,
        /// Output file for logging (default: auto-generated)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Run workload (use in separate terminal while logging)
    Run {
        /// Workload name
        workload: String,
        /// Additional workload arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Analyze collected data and display results
    Report {
        /// Group results by field (config, os, workload)
        #[arg(long, default_value = "config")]
        group_by: String,
        /// Output format
        #[arg(long, value_enum, default_value = "table")]
        format: OutputFormat,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
        /// Baseline configuration for efficiency comparison
        #[arg(long)]
        baseline: Option<String>,
        /// Minimum samples required for valid run
        #[arg(long, default_value = "10")]
        min_samples: usize,
    },
    /// Export summary data for external analysis
    Export {
        /// Output format
        #[arg(long, value_enum, default_value = "csv")]
        format: OutputFormat,
        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// List available workloads
    List {
        /// What to list
        #[arg(default_value = "workloads")]
        item: String,
    },
    /// Collect a single telemetry sample (for testing)
    Sample,
    /// Show system metadata
    Metadata,
}

#[derive(Debug, Serialize)]
struct RunSummary {
    run_id: String,
    config: String,
    os: String,
    workload: Option<String>,
    duration_s: f32,
    samples_total: usize,
    samples_valid: usize,
    avg_watts: f32,
    median_watts: f32,
    p95_watts: f32,
    avg_cpu_load: f32,
    avg_ram_pct: f32,
    avg_temp_c: f32,
    pct_drop: Option<f32>,
    start_pct: Option<f32>,
    end_pct: Option<f32>,
}

#[derive(Debug, Serialize)]
struct ComparisonReport {
    summaries: Vec<RunSummary>,
    grouped_stats: HashMap<String, GroupedStats>,
}

#[derive(Debug, Serialize)]
struct GroupedStats {
    group_name: String,
    run_count: usize,
    avg_watts_mean: f32,
    avg_watts_stddev: f32,
    efficiency_vs_baseline: Option<f32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Determine project directory (current working directory)
    let project_dir = std::env::current_dir()?;
    let data_dir = project_dir.join("data");
    let workload_dir = project_dir.join("workload");

    match cli.command {
        Commands::Init => cmd_init(&project_dir, &data_dir, &workload_dir),
        Commands::Log {
            config_name,
            hz,
            output,
        } => cmd_log(&config_name, hz, output.as_deref(), &data_dir),
        Commands::Run { workload, args } => cmd_run(&workload, &args, &workload_dir),
        Commands::Report {
            group_by,
            format,
            output,
            baseline,
            min_samples,
        } => cmd_report(
            &data_dir,
            &group_by,
            &format,
            output.as_deref(),
            baseline.as_deref(),
            min_samples,
        ),
        Commands::Export { format, output } => cmd_export(&data_dir, &format, output.as_deref()),
        Commands::List { item } => cmd_list(&item, &workload_dir),
        Commands::Sample => cmd_sample(),
        Commands::Metadata => cmd_metadata(),
    }
}

/// Initialize project directories and check capabilities
fn cmd_init(
    project_dir: &Path,
    data_dir: &Path,
    workload_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔋 Initializing batlab battery test harness...");

    // Create directories
    for dir in [data_dir, workload_dir, &project_dir.join("report")] {
        if !dir.exists() {
            fs::create_dir_all(dir)?;
            println!("📁 Created directory: {}", dir.display());
        }
    }

    // Create example workloads
    create_example_workloads(workload_dir)?;

    // Detect OS and capabilities
    println!("🔍 Detecting system capabilities...");

    let system_info = get_system_info()?;
    println!("💻 Detected: {} system", system_info.os);

    // Check battery access based on OS
    check_battery_capabilities(&system_info.os);

    println!("✅ Initialization complete!");
    println!("📋 Next steps:");
    println!("   1. Manually configure your system power management");
    println!("   2. Run: batlab log <config-name> (in terminal 1)");
    println!("   3. Run: batlab run <workload> (in terminal 2)");

    Ok(())
}

fn check_battery_capabilities(os: &str) {
    match os.to_lowercase().as_str() {
        os if os.contains("linux") => {
            if which::which("upower").is_ok() {
                println!("✅ upower available for battery telemetry");
            } else if std::path::Path::new("/sys/class/power_supply").exists() {
                println!("✅ sysfs power_supply available for battery telemetry");
            } else {
                println!("⚠️  No battery telemetry sources found");
            }
        }
        os if os.contains("freebsd") => {
            if which::which("acpiconf").is_ok() {
                println!("✅ acpiconf available for battery telemetry");
            } else {
                println!("⚠️  acpiconf not found - install it for battery telemetry");
            }
        }
        _ => {
            println!("⚠️  Unsupported OS: {} - some features may not work", os);
        }
    }
}

fn create_example_workloads(workload_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let idle_content = r#"#!/bin/sh

describe() {
    echo "Idle workload - sleep with screen on"
}

run() {
    duration="3600"  # Default 1 hour

    # Parse arguments
    while [ $# -gt 0 ]; do
        case "$1" in
            --duration)
                duration="$2"
                shift 2
                ;;
            *)
                echo "Unknown option: $1" >&2
                return 1
                ;;
        esac
    done

    echo "Running idle workload for $duration seconds..."
    echo "Press Ctrl+C to stop"

    # Keep screen on and just sleep
    sleep "$duration"
}
"#;

    let stress_content = r#"#!/bin/sh

describe() {
    echo "CPU stress test workload"
}

run() {
    intensity="50"
    duration="3600"

    # Parse arguments
    while [ $# -gt 0 ]; do
        case "$1" in
            --intensity)
                intensity="$2"
                shift 2
                ;;
            --duration)
                duration="$2"
                shift 2
                ;;
            *)
                echo "Unknown option: $1" >&2
                return 1
                ;;
        esac
    done

    echo "Running CPU stress at $intensity% for $duration seconds..."

    # Simple CPU stress using dd and compression
    ncpu=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "1")

    i=0
    while [ $i -lt "$ncpu" ]; do
        (
            end_time=$(($(date +%s) + duration))
            while [ $(date +%s) -lt $end_time ]; do
                dd if=/dev/zero bs=1M count=1 2>/dev/null | gzip >/dev/null
                sleep 0.1
            done
        ) &
        i=$((i + 1))
    done

    wait
}
"#;

    let idle_workload = workload_dir.join("idle.sh");
    if !idle_workload.exists() {
        fs::write(&idle_workload, idle_content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&idle_workload)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&idle_workload, perms)?;
        }
        println!("📄 Created workload: idle.sh");
    }

    let stress_workload = workload_dir.join("stress.sh");
    if !stress_workload.exists() {
        fs::write(&stress_workload, stress_content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&stress_workload)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&stress_workload, perms)?;
        }
        println!("📄 Created workload: stress.sh");
    }

    Ok(())
}

/// Start telemetry logging
fn cmd_log(
    config_name: &str,
    hz: f32,
    output_file: Option<&str>,
    data_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate config name
    if config_name.is_empty() || config_name.chars().any(|c| c.is_whitespace()) {
        eprintln!("❌ Configuration name cannot be empty or contain whitespace");
        process::exit(1);
    }

    // Validate frequency
    if !(0.01..=10.0).contains(&hz) {
        eprintln!("❌ Sampling frequency must be between 0.01 and 10.0 Hz");
        process::exit(1);
    }

    // Wait for battery to be available and not charging
    wait_for_battery_ready()?;

    // Create data directory if it doesn't exist
    if !data_dir.exists() {
        fs::create_dir_all(data_dir)?;
    }

    // Generate run ID and file paths
    let run_id = generate_run_id(config_name, None);
    let jsonl_file = match output_file {
        Some(file) => PathBuf::from(file),
        None => data_dir.join(format!("{}.jsonl", run_id)),
    };
    let meta_file = jsonl_file.with_extension("meta.json");

    println!("🔋 Starting telemetry logging...");
    println!("⚙️  Configuration: {}", config_name);
    println!("📊 Run ID: {}", run_id);
    println!("📁 Output: {}", jsonl_file.display());
    println!("🔄 Sampling at {:.1} Hz", hz);
    println!("⏹️  Press Ctrl+C to stop logging");

    // Create metadata
    let system_info = get_system_info()?;
    let metadata = serde_json::json!({
        "run_id": run_id,
        "host": system_info.hostname,
        "os": system_info.os,
        "config": config_name,
        "start_time": Utc::now().to_rfc3339(),
        "sampling_hz": hz
    });

    fs::write(&meta_file, serde_json::to_string_pretty(&metadata)?)?;

    // Set up signal handler for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    ctrlc::set_handler(move || {
        eprintln!("\n⏹️  Received interrupt signal, stopping telemetry...");
        running_clone.store(false, Ordering::SeqCst);
    })?;

    // Set up output writer
    let mut writer: Box<dyn Write> = Box::new(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&jsonl_file)?,
    );

    // Calculate sleep duration
    let sleep_duration = Duration::from_secs_f32(1.0 / hz);
    let mut sample_count = 0u64;
    let mut error_count = 0u64;

    println!("🚀 Logging started - run workload in another terminal");

    // Main sampling loop
    while running.load(Ordering::SeqCst) {
        match collect_telemetry() {
            Ok(sample) => {
                match serde_json::to_string(&sample) {
                    Ok(json) => {
                        if writeln!(writer, "{}", json).is_ok() {
                            sample_count += 1;

                            // Flush output periodically
                            if sample_count % 10 == 0 {
                                let _ = writer.flush();
                            }
                        } else {
                            eprintln!("⚠️  Warning: Failed to write sample to output");
                            error_count += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️  Warning: JSON serialization failed: {}", e);
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️  Warning: Telemetry collection failed: {}", e);
                error_count += 1;

                // Exit if too many consecutive errors during startup
                if error_count > 10 && sample_count == 0 {
                    eprintln!("❌ Too many failures during startup, exiting");
                    process::exit(1);
                }
            }
        }

        thread::sleep(sleep_duration);
    }

    // Final summary
    let _ = writer.flush();
    println!("\n📊 Telemetry logging stopped");
    println!("📈 Samples collected: {}", sample_count);
    if error_count > 0 {
        println!("⚠️  Errors encountered: {}", error_count);
    }

    Ok(())
}

/// Run workload
fn cmd_run(
    workload_name: &str,
    args: &[String],
    workload_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let workload_file = workload_dir.join(format!("{}.sh", workload_name));

    if !workload_file.exists() {
        eprintln!("❌ Workload not found: {}", workload_file.display());
        eprintln!("📋 Available workloads:");
        let _ = cmd_list("workloads", workload_dir);
        process::exit(1);
    }

    println!("🏃 Running workload: {}", workload_name);

    // Execute the workload script
    let mut cmd = process::Command::new("sh");
    cmd.arg(&workload_file);

    // Add the "run" function call and any arguments
    cmd.arg("-c");
    let script_content = format!(
        "source '{}' && run {}",
        workload_file.display(),
        args.join(" ")
    );
    cmd.arg(&script_content);

    let status = cmd.status()?;

    if status.success() {
        println!("✅ Workload completed successfully");
        Ok(())
    } else {
        eprintln!(
            "❌ Workload failed with exit code: {}",
            status.code().unwrap_or(-1)
        );
        process::exit(1);
    }
}

/// Generate report from collected data
fn cmd_report(
    data_dir: &Path,
    group_by: &str,
    format: &OutputFormat,
    output_file: Option<&str>,
    baseline: Option<&str>,
    min_samples: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Load and analyze data
    let summaries = load_run_summaries(data_dir, min_samples)?;

    if summaries.is_empty() {
        eprintln!("❌ No valid runs found in {}", data_dir.display());
        eprintln!("💡 Make sure you have collected some telemetry data first:");
        eprintln!("   batlab log <config-name>");
        return Ok(());
    }

    // Generate grouped statistics
    let grouped_stats = generate_grouped_stats(&summaries, group_by, baseline);

    let report = ComparisonReport {
        summaries,
        grouped_stats,
    };

    // Output report
    let output = match format {
        OutputFormat::Table => generate_table_report(&report),
        OutputFormat::Csv => generate_csv_report(&report),
        OutputFormat::Json => serde_json::to_string_pretty(&report)?,
    };

    match output_file {
        Some(file_path) => fs::write(file_path, output)?,
        None => println!("{}", output),
    }

    Ok(())
}

/// Export data
fn cmd_export(
    data_dir: &Path,
    format: &OutputFormat,
    output_file: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    cmd_report(data_dir, "config", format, output_file, None, 1)
}

/// List workloads
fn cmd_list(item: &str, workload_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    match item {
        "workloads" => {
            println!("📋 Available workloads:");
            if workload_dir.exists() {
                for entry in fs::read_dir(workload_dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if let Some(extension) = path.extension() {
                        if extension == "sh" {
                            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                                // Try to get description from the workload file
                                let description = get_workload_description(&path)
                                    .unwrap_or_else(|| "No description".to_string());

                                println!("  📄 {:<20} {}", name, description);
                            }
                        }
                    }
                }
            } else {
                println!("⚠️  No workloads directory found");
                println!("💡 Run 'batlab init' to create example workloads");
            }
        }
        _ => {
            eprintln!("❌ Usage: batlab list workloads");
            process::exit(1);
        }
    }

    Ok(())
}

fn get_workload_description(workload_path: &Path) -> Option<String> {
    if let Ok(content) = fs::read_to_string(workload_path) {
        // Look for describe() function and extract the echo statement
        for line in content.lines() {
            if line.trim().starts_with("echo ") && content.contains("describe()") {
                let desc = line
                    .trim()
                    .strip_prefix("echo ")?
                    .trim_matches('"')
                    .trim_matches('\'');
                return Some(desc.to_string());
            }
        }
    }
    None
}

/// Handle single telemetry sample collection
fn cmd_sample() -> Result<(), Box<dyn std::error::Error>> {
    // Wait for battery to be available and not charging
    wait_for_battery_ready()?;

    match collect_telemetry() {
        Ok(sample) => {
            println!("{}", serde_json::to_string_pretty(&sample)?);
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Telemetry collection failed: {}", e);

            // Provide helpful error messages based on error type
            match &e {
                TelemetryError::Battery(battery_err) => match battery_err {
                    batlab::BatteryError::NotFound => {
                        eprintln!("💡 Hint: Make sure you're running on a laptop with a battery");
                        #[cfg(target_os = "freebsd")]
                        eprintln!("        Try: pkg install acpi (for acpiconf command)");
                        #[cfg(target_os = "linux")]
                        eprintln!("        Try: which upower (check if upower is installed)");
                    }
                    batlab::BatteryError::Charging => {
                        eprintln!("💡 Hint: Unplug AC adapter for accurate battery measurements");
                    }
                    batlab::BatteryError::PermissionDenied { tool } => {
                        eprintln!("💡 Hint: Permission denied accessing {}", tool);
                        eprintln!("        You may need to run with appropriate permissions");
                    }
                    _ => {}
                },
                TelemetryError::Unavailable { resource } => {
                    eprintln!("💡 Hint: {} not available on this system", resource);
                }
                _ => {}
            }

            process::exit(1);
        }
    }
}

/// Wait for battery to be ready (available and not charging)
/// This function will loop until the battery is detected and not charging,
/// prompting the user to unplug AC adapter when needed.
fn wait_for_battery_ready() -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match get_battery_info() {
            Ok(_) => {
                // Battery is available and not charging
                println!("✅ Battery detected and ready for measurements");
                return Ok(());
            }
            Err(BatteryError::NotFound) => {
                eprintln!("❌ No battery found on this system");
                eprintln!("💡 Hint: Make sure you're running on a laptop with a battery");
                #[cfg(target_os = "freebsd")]
                eprintln!("        Try: pkg install acpi (for acpiconf command)");
                #[cfg(target_os = "linux")]
                eprintln!("        Try: which upower (check if upower is installed)");
                process::exit(1);
            }
            Err(BatteryError::Charging) => {
                println!("🔌 Battery is currently charging");
                println!(
                    "⚠️  For accurate battery life measurements, the AC adapter must be unplugged"
                );
                println!("📋 Please unplug your AC adapter and press Enter to continue...");

                // Wait for user input
                let stdin = io::stdin();
                let _ = stdin.lock().read_line(&mut String::new())?;

                println!("🔄 Checking battery status...");
                // Continue the loop to check again
            }
            Err(BatteryError::PermissionDenied { tool }) => {
                eprintln!("❌ Permission denied accessing {}", tool);
                eprintln!("💡 Hint: You may need to run with appropriate permissions");
                process::exit(1);
            }
            Err(other) => {
                eprintln!("❌ Battery error: {}", other);
                process::exit(1);
            }
        }
    }
}

/// Show system metadata
fn cmd_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let system_info = get_system_info()?;
    println!("{}", serde_json::to_string_pretty(&system_info)?);
    Ok(())
}

/// Load and summarize all runs from the data directory
fn load_run_summaries(
    data_dir: &Path,
    min_samples: usize,
) -> Result<Vec<RunSummary>, Box<dyn std::error::Error>> {
    let mut summaries = Vec::new();

    if !data_dir.exists() {
        return Ok(summaries);
    }

    for entry in fs::read_dir(data_dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(extension) = path.extension() {
            if extension == "jsonl" {
                match analyze_run(&path, min_samples) {
                    Ok(Some(summary)) => summaries.push(summary),
                    Ok(None) => {
                        eprintln!("⚠️  Skipping {} (insufficient samples)", path.display());
                    }
                    Err(e) => {
                        eprintln!("⚠️  Warning: Failed to analyze {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    // Sort by run ID (which includes timestamp)
    summaries.sort_by(|a, b| a.run_id.cmp(&b.run_id));

    Ok(summaries)
}

/// Analyze a single run file and generate summary
fn analyze_run(
    jsonl_path: &Path,
    min_samples: usize,
) -> Result<Option<RunSummary>, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(jsonl_path)?;
    let samples: Vec<TelemetrySample> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    if samples.len() < min_samples {
        return Ok(None);
    }

    // Load metadata if available
    let run_id = jsonl_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let metadata_path = jsonl_path.with_extension("meta.json");
    let (config, workload, os) = if metadata_path.exists() {
        match fs::read_to_string(&metadata_path) {
            Ok(meta_content) => {
                if let Ok(metadata) = serde_json::from_str::<RunMetadata>(&meta_content) {
                    (metadata.config, metadata.workload, metadata.system.os)
                } else {
                    parse_run_id_fallback(&run_id)
                }
            }
            Err(_) => parse_run_id_fallback(&run_id),
        }
    } else {
        parse_run_id_fallback(&run_id)
    };

    // Calculate statistics
    let valid_samples: Vec<&TelemetrySample> = samples
        .iter()
        .filter(|s| s.watts >= 0.0 && s.percentage >= 0.0 && s.percentage <= 100.0)
        .collect();

    let samples_total = samples.len();
    let samples_valid = valid_samples.len();

    if samples_valid == 0 {
        return Ok(None);
    }

    // Calculate duration
    let duration_s = if let (Some(first), Some(last)) = (samples.first(), samples.last()) {
        (last.timestamp - first.timestamp).num_seconds() as f32
    } else {
        0.0
    };

    // Power statistics
    let mut watts_values: Vec<f32> = valid_samples.iter().map(|s| s.watts).collect();
    watts_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let avg_watts = watts_values.iter().sum::<f32>() / watts_values.len() as f32;
    let median_watts = percentile(&watts_values, 0.5);
    let p95_watts = percentile(&watts_values, 0.95);

    // System metrics
    let avg_cpu_load = valid_samples.iter().map(|s| s.cpu_load).sum::<f32>() / samples_valid as f32;
    let avg_ram_pct = valid_samples.iter().map(|s| s.ram_pct).sum::<f32>() / samples_valid as f32;
    let avg_temp_c = valid_samples.iter().map(|s| s.temp_c).sum::<f32>() / samples_valid as f32;

    // Battery percentage drop
    let (start_pct, end_pct, pct_drop) = if valid_samples.len() >= 2 {
        let start = valid_samples.first().unwrap().percentage;
        let end = valid_samples.last().unwrap().percentage;
        let drop = if start > end { Some(start - end) } else { None };
        (Some(start), Some(end), drop)
    } else {
        (None, None, None)
    };

    Ok(Some(RunSummary {
        run_id,
        config,
        os,
        workload,
        duration_s,
        samples_total,
        samples_valid,
        avg_watts,
        median_watts,
        p95_watts,
        avg_cpu_load,
        avg_ram_pct,
        avg_temp_c,
        pct_drop,
        start_pct,
        end_pct,
    }))
}

/// Parse config, workload, and OS from run ID as fallback
fn parse_run_id_fallback(run_id: &str) -> (String, Option<String>, String) {
    let parts: Vec<&str> = run_id.split('_').collect();

    match parts.len() {
        4 => (parts[3].to_string(), None, parts[2].to_string()),
        5 => (
            parts[3].to_string(),
            Some(parts[4].to_string()),
            parts[2].to_string(),
        ),
        _ => ("unknown".to_string(), None, "unknown".to_string()),
    }
}

/// Calculate percentile from sorted data
fn percentile(sorted_data: &[f32], p: f32) -> f32 {
    if sorted_data.is_empty() {
        return 0.0;
    }

    let index = (p * (sorted_data.len() - 1) as f32) as usize;
    sorted_data[index.min(sorted_data.len() - 1)]
}

/// Generate grouped statistics
fn generate_grouped_stats(
    summaries: &[RunSummary],
    group_by: &str,
    baseline: Option<&str>,
) -> HashMap<String, GroupedStats> {
    let mut groups: HashMap<String, Vec<&RunSummary>> = HashMap::new();

    // Group summaries
    for summary in summaries {
        let group_key = match group_by {
            "config" => &summary.config,
            "os" => &summary.os,
            "workload" => summary.workload.as_deref().unwrap_or("none"),
            _ => &summary.config,
        };

        groups
            .entry(group_key.to_string())
            .or_default()
            .push(summary);
    }

    // Calculate baseline average watts for comparison
    let baseline_watts = baseline.and_then(|baseline_name| {
        groups.get(baseline_name).map(|group| {
            let total_watts: f32 = group.iter().map(|s| s.avg_watts).sum();
            total_watts / group.len() as f32
        })
    });

    // Generate stats for each group
    let mut grouped_stats = HashMap::new();

    for (group_name, group_summaries) in groups {
        let watts_values: Vec<f32> = group_summaries.iter().map(|s| s.avg_watts).collect();
        let count = watts_values.len();
        let mean = watts_values.iter().sum::<f32>() / count as f32;

        let variance = watts_values.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / count as f32;
        let stddev = variance.sqrt();

        let efficiency_vs_baseline = baseline_watts.map(|baseline| {
            ((baseline - mean) / baseline) * 100.0 // Positive = more efficient than baseline
        });

        let stats = GroupedStats {
            group_name: group_name.clone(),
            run_count: count,
            avg_watts_mean: mean,
            avg_watts_stddev: stddev,
            efficiency_vs_baseline,
        };

        grouped_stats.insert(group_name, stats);
    }

    grouped_stats
}

/// Generate table format report
fn generate_table_report(report: &ComparisonReport) -> String {
    let mut output = String::new();

    // Individual runs table
    output.push_str("INDIVIDUAL RUNS\n");
    output.push_str(&format!(
        "{:<30} {:<15} {:<10} {:<10} {:<8} {:<8} {:<8} {:<8} {:<8}\n",
        "RUN_ID", "CONFIG", "OS", "WORKLOAD", "SAMPLES", "AVG_W", "MED_W", "CPU%", "TEMP°C"
    ));
    output.push_str(&"-".repeat(120));
    output.push('\n');

    for summary in &report.summaries {
        let run_id = if summary.run_id.len() > 30 {
            summary.run_id[..27].to_string() + "..."
        } else {
            summary.run_id.clone()
        };

        let workload = summary.workload.as_deref().unwrap_or("-");

        output.push_str(&format!(
            "{:<30} {:<15} {:<10} {:<10} {:<8} {:<8.2} {:<8.2} {:<8.1} {:<8.1}\n",
            &run_id,
            &summary.config[..summary.config.len().min(15)],
            &summary.os[..summary.os.len().min(10)],
            &workload[..workload.len().min(10)],
            summary.samples_valid,
            summary.avg_watts,
            summary.median_watts,
            summary.avg_cpu_load * 100.0,
            summary.avg_temp_c,
        ));
    }

    output.push('\n');

    // Grouped statistics table
    output.push_str("GROUPED STATISTICS\n");
    output.push_str(&format!(
        "{:<20} {:<8} {:<12} {:<12} {:<15}\n",
        "GROUP", "RUNS", "AVG_WATTS", "STDDEV", "VS_BASELINE%"
    ));
    output.push_str(&"-".repeat(70));
    output.push('\n');

    let mut groups: Vec<_> = report.grouped_stats.values().collect();
    groups.sort_by(|a, b| a.group_name.cmp(&b.group_name));

    for stats in groups {
        let vs_baseline = stats
            .efficiency_vs_baseline
            .map(|x| format!("{:+.1}", x))
            .unwrap_or_else(|| "-".to_string());

        output.push_str(&format!(
            "{:<20} {:<8} {:<12.2} {:<12.2} {:<15}\n",
            &stats.group_name[..stats.group_name.len().min(20)],
            stats.run_count,
            stats.avg_watts_mean,
            stats.avg_watts_stddev,
            vs_baseline,
        ));
    }

    output
}

/// Generate CSV format report
fn generate_csv_report(report: &ComparisonReport) -> String {
    let mut output = String::new();

    // CSV header
    output.push_str("run_id,config,os,workload,duration_s,samples_total,samples_valid,avg_watts,median_watts,p95_watts,avg_cpu_load,avg_ram_pct,avg_temp_c,pct_drop\n");

    // CSV data
    for summary in &report.summaries {
        let workload = summary.workload.as_deref().unwrap_or("");
        let pct_drop = summary.pct_drop.map(|x| x.to_string()).unwrap_or_default();

        output.push_str(&format!(
            "{},{},{},{},{},{},{},{:.3},{:.3},{:.3},{:.3},{:.1},{:.1},{}\n",
            summary.run_id,
            summary.config,
            summary.os,
            workload,
            summary.duration_s,
            summary.samples_total,
            summary.samples_valid,
            summary.avg_watts,
            summary.median_watts,
            summary.p95_watts,
            summary.avg_cpu_load,
            summary.avg_ram_pct,
            summary.avg_temp_c,
            pct_drop,
        ));
    }

    output
}
