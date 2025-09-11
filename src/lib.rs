//! # batlab - Battery Test Harness
//!
//! A cross-platform battery efficiency measurement tool for FreeBSD vs Linux research.
//!
//! This library provides robust telemetry collection for battery, CPU, memory, and
//! temperature metrics with platform-specific implementations for FreeBSD and Linux.
//!
//! ## Architecture
//!
//! The library is organized into platform-specific modules that are conditionally
//! compiled based on the target OS:
//!
//! - `freebsd_telemetry`: Uses acpiconf, sysctl for FreeBSD systems
//! - `linux_telemetry`: Uses upower, sysfs for Linux systems
//!
//! ## Example
//!
//! ```rust
//! use batlab::{collect_telemetry, TelemetrySample};
//!
//! let sample = collect_telemetry()?;
//! println!("Battery: {}% at {:.2}W", sample.battery.percentage, sample.battery.watts);
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Platform-specific modules
#[cfg(target_os = "freebsd")]
mod freebsd_telemetry;
#[cfg(target_os = "linux")]
mod linux_telemetry;
#[cfg(not(any(target_os = "freebsd", target_os = "linux")))]
mod unsupported_telemetry;

// Re-export platform-specific implementations
#[cfg(target_os = "freebsd")]
pub use freebsd_telemetry::*;
#[cfg(target_os = "linux")]
pub use linux_telemetry::*;
#[cfg(not(any(target_os = "freebsd", target_os = "linux")))]
pub use unsupported_telemetry::*;



/// Battery telemetry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    /// Battery charge percentage (0.0-100.0)
    pub percentage: f32,
    /// Current power draw in watts
    pub watts: f32,
    /// Data source (e.g., "acpiconf", "upower", "sysfs")
    pub source: String,
}

/// Complete telemetry sample containing all system metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySample {
    /// Timestamp when sample was taken
    #[serde(rename = "t")]
    pub timestamp: DateTime<Utc>,
    /// Battery charge percentage
    #[serde(rename = "pct")]
    pub percentage: f32,
    /// Current power draw in watts
    pub watts: f32,
    /// CPU load average (1-minute)
    pub cpu_load: f32,
    /// RAM usage percentage
    pub ram_pct: f32,
    /// Temperature in Celsius
    pub temp_c: f32,
    /// Battery data source
    #[serde(rename = "src")]
    pub source: String,
}

/// Comprehensive error type for all telemetry operations
#[derive(Error, Debug)]
pub enum TelemetryError {
    /// Battery-related errors
    #[error("Battery error: {0}")]
    Battery(#[from] BatteryError),

    /// System command execution errors
    #[error("Command failed: {command} - {message}")]
    CommandFailed { command: String, message: String },

    /// Data parsing errors
    #[error("Parse error in {context}: {message}")]
    ParseError { context: String, message: String },

    /// Permission denied accessing system resources
    #[error("Permission denied: {resource}")]
    PermissionDenied { resource: String },

    /// IO errors (file access, etc.)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// System resource not available
    #[error("Resource unavailable: {resource}")]
    Unavailable { resource: String },
}

/// Battery-specific error types
#[derive(Error, Debug)]
pub enum BatteryError {
    /// No battery found on the system
    #[error("Battery not found")]
    NotFound,

    /// Battery is charging (may affect measurements)
    #[error("Battery is charging")]
    Charging,

    /// Failed to parse battery information
    #[error("Failed to parse {field}: {value}")]
    ParseError { field: String, value: String },

    /// Permission denied accessing battery information
    #[error("Permission denied accessing battery via {tool}")]
    PermissionDenied { tool: String },

    /// Battery command/tool not available
    #[error("Battery tool not available: {tool}")]
    ToolUnavailable { tool: String },
}

/// System information for metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Hostname
    pub hostname: String,
    /// Operating system name and version
    pub os: String,
    /// Kernel version
    pub kernel: String,
    /// CPU model
    pub cpu: String,
    /// Machine/hardware model
    pub machine: String,
}

/// Run metadata for experiment tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetadata {
    /// Unique run identifier
    pub run_id: String,
    /// System information
    #[serde(flatten)]
    pub system: SystemInfo,
    /// User-defined configuration name
    pub config: String,
    /// Workload name
    pub workload: Option<String>,
    /// When the run started
    pub start_time: DateTime<Utc>,
    /// Sampling frequency in Hz
    pub sampling_hz: f32,
    /// Battery capacity information
    pub battery_capacity: Option<BatteryCapacity>,
}

/// Battery capacity information for efficiency calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryCapacity {
    /// Design capacity in Wh
    pub design_wh: Option<f32>,
    /// Current full capacity in Wh
    pub full_wh: Option<f32>,
}

/// Collect a complete telemetry sample
///
/// This is the main entry point for telemetry collection. It gathers battery,
/// CPU, memory, and temperature data from the appropriate platform sources.
///
/// # Errors
///
/// Returns `TelemetryError` if any telemetry collection fails. Individual
/// metric failures may be handled gracefully with default values depending
/// on the platform implementation.
///
/// # Example
///
/// ```rust
/// use batlab::collect_telemetry;
///
/// match collect_telemetry() {
///     Ok(sample) => println!("{}", serde_json::to_string(&sample)?),
///     Err(e) => eprintln!("Telemetry failed: {}", e),
/// }
/// ```
pub fn collect_telemetry() -> Result<TelemetrySample, TelemetryError> {
    let timestamp = Utc::now();

    // Get battery information (required)
    let battery = get_battery_info()?;

    // Get system metrics (with graceful fallbacks)
    let cpu_load = get_cpu_load().unwrap_or(0.0);
    let ram_pct = get_memory_usage().unwrap_or(0.0);
    let temp_c = get_temperature().unwrap_or(0.0);

    Ok(TelemetrySample {
        timestamp,
        percentage: battery.percentage,
        watts: battery.watts,
        cpu_load,
        ram_pct,
        temp_c,
        source: battery.source,
    })
}

/// Get system information for metadata
///
/// Collects hostname, OS version, kernel, CPU model, and machine type
/// for inclusion in run metadata.
pub fn get_system_info() -> Result<SystemInfo, TelemetryError> {
    let hostname = get_command_output("hostname", &[])?;
    let kernel = get_command_output("uname", &["-r"])?;

    // Platform-specific system info
    #[cfg(target_os = "freebsd")]
    let (os, cpu, machine) = {
        let os = get_command_output("uname", &["-sr"])?;
        let cpu = get_sysctl("hw.model")?;
        let machine = get_sysctl("hw.machine")
            .or_else(|_| get_command_output("uname", &["-m"]))?;
        (os, cpu, machine)
    };

    #[cfg(target_os = "linux")]
    let (os, cpu, machine) = {
        let os = std::fs::read_to_string("/etc/os-release")
            .unwrap_or_default()
            .lines()
            .find(|line| line.starts_with("PRETTY_NAME="))
            .and_then(|line| line.split('=').nth(1))
            .map(|s| s.trim_matches('"').to_string())
            .unwrap_or_else(|| get_command_output("uname", &["-sr"]).unwrap_or_default());

        let cpu = std::fs::read_to_string("/proc/cpuinfo")
            .unwrap_or_default()
            .lines()
            .find(|line| line.starts_with("model name"))
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let machine = get_command_output("uname", &["-m"])?;
        (os, cpu, machine)
    };

    #[cfg(not(any(target_os = "freebsd", target_os = "linux")))]
    let (os, cpu, machine) = {
        let os = get_command_output("uname", &["-sr"]).unwrap_or_else(|_| "Unknown".to_string());
        let cpu = "Unknown".to_string();
        let machine = get_command_output("uname", &["-m"]).unwrap_or_else(|_| "Unknown".to_string());
        (os, cpu, machine)
    };

    Ok(SystemInfo {
        hostname,
        os,
        kernel,
        cpu,
        machine,
    })
}

/// Helper function to execute command and get output
fn get_command_output(cmd: &str, args: &[&str]) -> Result<String, TelemetryError> {
    let output = std::process::Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| TelemetryError::CommandFailed {
            command: format!("{} {}", cmd, args.join(" ")),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(TelemetryError::CommandFailed {
            command: format!("{} {}", cmd, args.join(" ")),
            message: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Helper function for FreeBSD sysctl access
#[cfg(target_os = "freebsd")]
fn get_sysctl(name: &str) -> Result<String, TelemetryError> {
    get_command_output("sysctl", &["-n", name])
}

/// Generate a unique run ID
///
/// Format: `YYYY-MM-DDTHH:MM:SSZ_hostname_os_config`
pub fn generate_run_id(config: &str, workload: Option<&str>) -> String {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let hostname = get_command_output("hostname", &[])
        .unwrap_or_else(|_| "unknown".to_string());

    #[cfg(target_os = "freebsd")]
    let os = "FreeBSD";
    #[cfg(target_os = "linux")]
    let os = "Linux";
    #[cfg(not(any(target_os = "freebsd", target_os = "linux")))]
    let os = "Unknown";

    match workload {
        Some(w) => format!("{}_{}_{}_{}_{}",  timestamp, hostname, os, config, w),
        None => format!("{}_{}_{}_{}",  timestamp, hostname, os, config),
    }
}

// Platform-specific functions are re-exported from the platform modules above

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_id_generation() {
        let config = "test-config";
        let workload = Some("idle");

        let run_id = generate_run_id(config, workload);

        // Should contain all expected components
        assert!(run_id.contains(config));
        assert!(run_id.contains("idle"));

        // Should be valid format (basic check)
        let parts: Vec<&str> = run_id.split('_').collect();
        assert!(parts.len() >= 4);
    }

    #[test]
    fn test_telemetry_sample_serialization() {
        let sample = TelemetrySample {
            timestamp: Utc::now(),
            percentage: 85.5,
            watts: 12.3,
            cpu_load: 0.25,
            ram_pct: 45.0,
            temp_c: 42.5,
            source: "test".to_string(),
        };

        // Should serialize to valid JSON
        let json = serde_json::to_string(&sample).expect("Serialization failed");
        assert!(json.contains("\"pct\":85.5"));
        assert!(json.contains("\"watts\":12.3"));

        // Should deserialize back correctly
        let deserialized: TelemetrySample = serde_json::from_str(&json)
            .expect("Deserialization failed");
        assert_eq!(deserialized.percentage, sample.percentage);
        assert_eq!(deserialized.watts, sample.watts);
    }
}
