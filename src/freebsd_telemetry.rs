//! FreeBSD-specific telemetry collection
//!
//! This module implements battery, CPU, memory, and temperature telemetry
//! collection using FreeBSD's native tools and interfaces:
//!
//! - `acpiconf` for battery information
//! - `sysctl` for system metrics
//! - Graceful fallbacks when tools are unavailable

use crate::{BatteryError, BatteryInfo, BatteryCapacity, TelemetryError};
use std::process::Command;
use std::str::FromStr;

/// Get battery information using FreeBSD-specific methods
///
/// Priority order:
/// 1. acpiconf -i 0 (ACPI battery interface)
/// 2. sysctl hw.acpi.battery.* (fallback)
/// 3. Return error if no battery found
pub fn get_battery_info() -> Result<BatteryInfo, BatteryError> {
    // Try acpiconf first (most reliable)
    acpiconf_battery()
        .or_else(|_| sysctl_battery())
        .map_err(|_| BatteryError::NotFound)
}

/// Get battery info via acpiconf command
fn acpiconf_battery() -> Result<BatteryInfo, BatteryError> {
    let output = Command::new("acpiconf")
        .args(["-i", "0"])
        .output()
        .map_err(|_| BatteryError::ToolUnavailable {
            tool: "acpiconf".to_string(),
        })?;

    if !output.status.success() {
        return Err(BatteryError::ToolUnavailable {
            tool: "acpiconf".to_string(),
        });
    }

    let info = String::from_utf8_lossy(&output.stdout);

    // Check if battery is charging (may affect measurements)
    if info.lines().any(|line| line.contains("State") && line.contains("charging")) {
        return Err(BatteryError::Charging);
    }

    // Parse remaining capacity percentage
    let percentage = parse_acpiconf_field(&info, "Remaining capacity")?;

    // Parse present rate (in mW) and convert to watts
    let rate_mw = parse_acpiconf_field(&info, "Present rate")
        .unwrap_or(0.0); // Present rate may be 0 when idle

    let watts = if rate_mw > 0.0 { rate_mw / 1000.0 } else { 0.0 };

    Ok(BatteryInfo {
        percentage,
        watts,
        source: "acpiconf".to_string(),
    })
}

/// Parse a numeric field from acpiconf output
fn parse_acpiconf_field(text: &str, field_name: &str) -> Result<f32, BatteryError> {
    text.lines()
        .find(|line| line.contains(field_name))
        .and_then(|line| {
            // Extract the numeric part (handle formats like "85%" or "1250 mW")
            line.split_whitespace()
                .find_map(|word| {
                    // Try to parse as float, removing % suffix if present
                    let clean_word = word.trim_end_matches('%');
                    clean_word.parse::<f32>().ok()
                })
        })
        .ok_or_else(|| BatteryError::ParseError {
            field: field_name.to_string(),
            value: "not found or invalid".to_string(),
        })
}

/// Fallback battery info via sysctl
fn sysctl_battery() -> Result<BatteryInfo, BatteryError> {
    // Try to get battery percentage from sysctl
    let percentage = get_sysctl_f32("hw.acpi.battery.life")
        .map_err(|_| BatteryError::ToolUnavailable {
            tool: "sysctl battery".to_string(),
        })?;

    // Battery rate may not be available via sysctl, default to 0
    let watts = get_sysctl_f32("hw.acpi.battery.rate").unwrap_or(0.0) / 1000.0;

    Ok(BatteryInfo {
        percentage,
        watts,
        source: "sysctl".to_string(),
    })
}

/// Get CPU load average (1-minute) from vm.loadavg
pub fn get_cpu_load() -> Result<f32, TelemetryError> {
    let loadavg = get_sysctl("vm.loadavg")?;

    // Parse "{ 0.15 0.20 0.18 }" format - we want the first number
    loadavg
        .trim_start_matches('{')
        .trim()
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| TelemetryError::ParseError {
            context: "vm.loadavg".to_string(),
            message: format!("Invalid format: {}", loadavg),
        })
}

/// Get memory usage percentage from vm.stats
pub fn get_memory_usage() -> Result<f32, TelemetryError> {
    // Get total pages and free pages
    let total_pages = get_sysctl_u64("vm.stats.vm.v_page_count")?;
    let free_pages = get_sysctl_u64("vm.stats.vm.v_free_count")?;

    if total_pages == 0 {
        return Ok(0.0);
    }

    let used_pages = total_pages.saturating_sub(free_pages);
    let usage_pct = (used_pages as f32 / total_pages as f32) * 100.0;

    Ok(usage_pct)
}

/// Get system temperature from available thermal sensors
pub fn get_temperature() -> Result<f32, TelemetryError> {
    // Try CPU temperature first
    if let Ok(temp) = get_cpu_temperature() {
        return Ok(temp);
    }

    // Try ACPI thermal zones
    get_acpi_thermal_temperature()
}

/// Get CPU temperature from dev.cpu.0.temperature
fn get_cpu_temperature() -> Result<f32, TelemetryError> {
    let temp_str = get_sysctl("dev.cpu.0.temperature")?;

    // Parse "45.0C" format
    temp_str
        .trim_end_matches('C')
        .parse()
        .map_err(|_| TelemetryError::ParseError {
            context: "dev.cpu.0.temperature".to_string(),
            message: format!("Invalid temperature format: {}", temp_str),
        })
}

/// Get temperature from ACPI thermal zones
fn get_acpi_thermal_temperature() -> Result<f32, TelemetryError> {
    // Try common thermal zone names
    let thermal_sysctls = [
        "hw.acpi.thermal.tz0.temperature",
        "hw.acpi.thermal.tz1.temperature",
        "dev.acpi_tz.0.temperature",
    ];

    for sysctl_name in &thermal_sysctls {
        if let Ok(temp_str) = get_sysctl(sysctl_name) {
            if let Ok(temp) = temp_str.trim_end_matches('C').parse::<f32>() {
                return Ok(temp);
            }
        }
    }

    Err(TelemetryError::Unavailable {
        resource: "thermal sensors".to_string(),
    })
}

/// Get battery capacity information if available
pub fn get_battery_capacity() -> Result<Option<BatteryCapacity>, BatteryError> {
    // Try to get battery capacity information from acpiconf
    let output = Command::new("acpiconf")
        .args(["-i", "0"])
        .output()
        .map_err(|_| BatteryError::ToolUnavailable {
            tool: "acpiconf".to_string(),
        })?;

    if !output.status.success() {
        return Ok(None);
    }

    let info = String::from_utf8_lossy(&output.stdout);

    let design_capacity = parse_acpiconf_field(&info, "Design capacity").ok();
    let last_full_capacity = parse_acpiconf_field(&info, "Last full capacity").ok();

    // Convert from mWh to Wh if values are available
    let design_wh = design_capacity.map(|mwh| mwh / 1000.0);
    let full_wh = last_full_capacity.map(|mwh| mwh / 1000.0);

    if design_wh.is_some() || full_wh.is_some() {
        Ok(Some(BatteryCapacity { design_wh, full_wh }))
    } else {
        Ok(None)
    }
}

/// Helper function to get sysctl value as string
fn get_sysctl(name: &str) -> Result<String, TelemetryError> {
    let output = Command::new("sysctl")
        .args(["-n", name])
        .output()
        .map_err(|e| TelemetryError::CommandFailed {
            command: format!("sysctl -n {}", name),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        return Err(TelemetryError::Unavailable {
            resource: name.to_string(),
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Helper function to get sysctl value as f32
fn get_sysctl_f32(name: &str) -> Result<f32, TelemetryError> {
    let value_str = get_sysctl(name)?;
    value_str.parse().map_err(|_| TelemetryError::ParseError {
        context: name.to_string(),
        message: format!("Cannot parse as f32: {}", value_str),
    })
}

/// Helper function to get sysctl value as u64
fn get_sysctl_u64(name: &str) -> Result<u64, TelemetryError> {
    let value_str = get_sysctl(name)?;
    value_str.parse().map_err(|_| TelemetryError::ParseError {
        context: name.to_string(),
        message: format!("Cannot parse as u64: {}", value_str),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_acpiconf_field() {
        let sample_output = r#"
Design capacity:        57040 mWh
Last full capacity:     53200 mWh
Technology:             Lithium Ion
Design voltage:         11400 mV
Remaining capacity:     85%
Remaining time:         3:45
Present rate:           12500 mW
Present voltage:        11800 mV
        "#;

        assert_eq!(
            parse_acpiconf_field(sample_output, "Remaining capacity").unwrap(),
            85.0
        );
        assert_eq!(
            parse_acpiconf_field(sample_output, "Present rate").unwrap(),
            12500.0
        );
        assert_eq!(
            parse_acpiconf_field(sample_output, "Design capacity").unwrap(),
            57040.0
        );
    }

    #[test]
    fn test_parse_acpiconf_field_missing() {
        let sample_output = "Some other output";
        assert!(parse_acpiconf_field(sample_output, "Nonexistent field").is_err());
    }

    #[test]
    fn test_loadavg_parsing() {
        // Test typical FreeBSD loadavg format
        let loadavg = "{ 0.15 0.20 0.18 }";
        let result = loadavg
            .trim_start_matches('{')
            .trim()
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<f32>().ok());

        assert_eq!(result, Some(0.15));
    }
}
