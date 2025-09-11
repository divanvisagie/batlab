//! Linux-specific telemetry collection
//!
//! This module implements battery, CPU, memory, and temperature telemetry
//! collection using Linux-specific interfaces:
//!
//! - `upower` for battery information (preferred)
//! - `/sys/class/power_supply/` for direct sysfs access (fallback)
//! - `/proc/` filesystem for system metrics
//! - `/sys/class/thermal/` for temperature sensors

use crate::{BatteryCapacity, BatteryError, BatteryInfo, TelemetryError};
use std::fs;
use std::process::Command;

/// Get battery information using Linux-specific methods
///
/// Priority order:
/// 1. upower (most user-friendly, handles multiple batteries)
/// 2. sysfs /sys/class/power_supply/BAT* (direct kernel interface)
/// 3. Return error if no battery found
pub fn get_battery_info() -> Result<BatteryInfo, BatteryError> {
    // Try upower first (most reliable and handles multiple batteries)
    upower_battery()
        .or_else(|err| {
            // Preserve Charging error, try sysfs for other errors
            match err {
                BatteryError::Charging => Err(err),
                _ => sysfs_battery(),
            }
        })
        .map_err(|err| {
            // Preserve Charging error, convert others to NotFound
            match err {
                BatteryError::Charging => err,
                _ => BatteryError::NotFound,
            }
        })
}

/// Get battery info via upower command
fn upower_battery() -> Result<BatteryInfo, BatteryError> {
    // First, find battery devices
    let devices_output =
        Command::new("upower")
            .arg("-e")
            .output()
            .map_err(|_| BatteryError::ToolUnavailable {
                tool: "upower".to_string(),
            })?;

    if !devices_output.status.success() {
        return Err(BatteryError::ToolUnavailable {
            tool: "upower".to_string(),
        });
    }

    let devices = String::from_utf8_lossy(&devices_output.stdout);
    let battery_path = devices
        .lines()
        .find(|line| line.contains("BAT"))
        .ok_or(BatteryError::NotFound)?;

    // Get detailed battery information
    let info_output = Command::new("upower")
        .args(["-i", battery_path])
        .output()
        .map_err(|_| BatteryError::ToolUnavailable {
            tool: "upower".to_string(),
        })?;

    if !info_output.status.success() {
        return Err(BatteryError::ToolUnavailable {
            tool: "upower".to_string(),
        });
    }

    let info = String::from_utf8_lossy(&info_output.stdout);

    // Check if battery is charging (look for the specific state line, not history)
    // Must check for exact "charging" word, not substring (to avoid matching "discharging")
    if info.lines().any(|line| {
        line.trim().starts_with("state:") && line.split_whitespace().any(|word| word == "charging")
    }) {
        return Err(BatteryError::Charging);
    }

    // Parse percentage
    let percentage = parse_upower_field(&info, "percentage")?;

    // Parse energy rate (watts)
    let watts = parse_upower_field(&info, "energy-rate").unwrap_or(0.0);

    Ok(BatteryInfo {
        percentage,
        watts,
        source: "upower".to_string(),
    })
}

/// Parse a numeric field from upower output
fn parse_upower_field(text: &str, field_name: &str) -> Result<f32, BatteryError> {
    text.lines()
        .find(|line| line.trim_start().starts_with(field_name))
        .and_then(|line| {
            // Format is typically "  field_name:          value unit"
            line.split(':')
                .nth(1)?
                .trim()
                .split_whitespace()
                .next()?
                .trim_end_matches('%')
                .parse()
                .ok()
        })
        .ok_or_else(|| BatteryError::ParseError {
            field: field_name.to_string(),
            value: "not found or invalid".to_string(),
        })
}

/// Fallback battery info via sysfs
fn sysfs_battery() -> Result<BatteryInfo, BatteryError> {
    // Find battery directories
    let power_supply_dir = "/sys/class/power_supply";
    let entries = fs::read_dir(power_supply_dir).map_err(|_| BatteryError::NotFound)?;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if name.starts_with("BAT") {
            if let Ok(info) = sysfs_battery_info(&path) {
                return Ok(info);
            }
        }
    }

    Err(BatteryError::NotFound)
}

/// Get battery info from a specific sysfs battery path
fn sysfs_battery_info(battery_path: &std::path::Path) -> Result<BatteryInfo, BatteryError> {
    // Get percentage
    let capacity_path = battery_path.join("capacity");
    let percentage = fs::read_to_string(&capacity_path)
        .map_err(|_| BatteryError::PermissionDenied {
            tool: capacity_path.display().to_string(),
        })?
        .trim()
        .parse::<f32>()
        .map_err(|_| BatteryError::ParseError {
            field: "capacity".to_string(),
            value: "invalid number".to_string(),
        })?;

    // Check if charging
    let status_path = battery_path.join("status");
    if let Ok(status) = fs::read_to_string(&status_path) {
        if status.trim().to_lowercase().contains("charging") {
            return Err(BatteryError::Charging);
        }
    }

    // Get power consumption (watts)
    let watts = sysfs_get_power_watts(battery_path).unwrap_or(0.0);

    Ok(BatteryInfo {
        percentage,
        watts,
        source: "sysfs".to_string(),
    })
}

/// Calculate power consumption from sysfs values
fn sysfs_get_power_watts(battery_path: &std::path::Path) -> Result<f32, BatteryError> {
    // Try power_now first (microwatts)
    let power_now_path = battery_path.join("power_now");
    if let Ok(power_uw) = fs::read_to_string(&power_now_path) {
        if let Ok(power) = power_uw.trim().parse::<f32>() {
            return Ok(power / 1_000_000.0); // Convert µW to W
        }
    }

    // Fallback: calculate from voltage_now and current_now
    let voltage_path = battery_path.join("voltage_now");
    let current_path = battery_path.join("current_now");

    if let (Ok(voltage_str), Ok(current_str)) = (
        fs::read_to_string(&voltage_path),
        fs::read_to_string(&current_path),
    ) {
        if let (Ok(voltage_uv), Ok(current_ua)) = (
            voltage_str.trim().parse::<f32>(),
            current_str.trim().parse::<f32>(),
        ) {
            // P = V * I (voltage in µV, current in µA)
            let power_watts = (voltage_uv * current_ua) / 1_000_000_000_000.0;
            return Ok(power_watts);
        }
    }

    // No power information available
    Ok(0.0)
}

/// Get CPU load average (1-minute) from /proc/loadavg
pub fn get_cpu_load() -> Result<f32, TelemetryError> {
    let loadavg = fs::read_to_string("/proc/loadavg").map_err(|e| TelemetryError::Io(e))?;

    // Parse "0.15 0.20 0.18 1/123 456" format - we want the first number
    loadavg
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| TelemetryError::ParseError {
            context: "/proc/loadavg".to_string(),
            message: format!("Invalid format: {}", loadavg.trim()),
        })
}

/// Get memory usage percentage from /proc/meminfo
pub fn get_memory_usage() -> Result<f32, TelemetryError> {
    let meminfo = fs::read_to_string("/proc/meminfo").map_err(|e| TelemetryError::Io(e))?;

    let mut mem_total = None;
    let mut mem_available = None;

    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            mem_total = parse_meminfo_kb(line);
        } else if line.starts_with("MemAvailable:") {
            mem_available = parse_meminfo_kb(line);
        }

        // Break early if we have both values
        if mem_total.is_some() && mem_available.is_some() {
            break;
        }
    }

    let total = mem_total.ok_or_else(|| TelemetryError::ParseError {
        context: "/proc/meminfo".to_string(),
        message: "MemTotal not found".to_string(),
    })?;

    let available = mem_available.ok_or_else(|| TelemetryError::ParseError {
        context: "/proc/meminfo".to_string(),
        message: "MemAvailable not found".to_string(),
    })?;

    if total == 0 {
        return Ok(0.0);
    }

    let used = total.saturating_sub(available);
    let usage_pct = (used as f32 / total as f32) * 100.0;

    Ok(usage_pct)
}

/// Parse memory value in kB from meminfo line
fn parse_meminfo_kb(line: &str) -> Option<u64> {
    // Format: "MemTotal:       16384000 kB"
    line.split_whitespace().nth(1)?.parse().ok()
}

/// Get system temperature from available thermal sensors
pub fn get_temperature() -> Result<f32, TelemetryError> {
    // Try thermal zones first
    if let Ok(temp) = get_thermal_zone_temperature() {
        return Ok(temp);
    }

    // Try hwmon sensors as fallback
    get_hwmon_temperature()
}

/// Get temperature from /sys/class/thermal/thermal_zone*
fn get_thermal_zone_temperature() -> Result<f32, TelemetryError> {
    let thermal_dir = "/sys/class/thermal";
    let entries = fs::read_dir(thermal_dir).map_err(|e| TelemetryError::Io(e))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if name.starts_with("thermal_zone") {
            let temp_path = path.join("temp");
            if let Ok(temp_str) = fs::read_to_string(&temp_path) {
                if let Ok(temp_millic) = temp_str.trim().parse::<f32>() {
                    if temp_millic > 0.0 {
                        return Ok(temp_millic / 1000.0); // Convert millicelsius to celsius
                    }
                }
            }
        }
    }

    Err(TelemetryError::Unavailable {
        resource: "thermal zones".to_string(),
    })
}

/// Get temperature from /sys/class/hwmon/hwmon*
fn get_hwmon_temperature() -> Result<f32, TelemetryError> {
    let hwmon_dir = "/sys/class/hwmon";
    let entries = fs::read_dir(hwmon_dir).map_err(|e| TelemetryError::Io(e))?;

    for entry in entries.flatten() {
        let hwmon_path = entry.path();

        // Look for temp*_input files
        if let Ok(hwmon_entries) = fs::read_dir(&hwmon_path) {
            for hwmon_entry in hwmon_entries.flatten() {
                let file_path = hwmon_entry.path();
                let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

                if file_name.starts_with("temp") && file_name.ends_with("_input") {
                    if let Ok(temp_str) = fs::read_to_string(&file_path) {
                        if let Ok(temp_millic) = temp_str.trim().parse::<f32>() {
                            if temp_millic > 0.0 {
                                return Ok(temp_millic / 1000.0); // Convert millicelsius to celsius
                            }
                        }
                    }
                }
            }
        }
    }

    Err(TelemetryError::Unavailable {
        resource: "hwmon sensors".to_string(),
    })
}

/// Get battery capacity information if available
pub fn get_battery_capacity() -> Result<Option<BatteryCapacity>, BatteryError> {
    // Try upower first for comprehensive capacity info
    if let Ok(capacity) = upower_battery_capacity() {
        return Ok(Some(capacity));
    }

    // Try sysfs fallback
    sysfs_battery_capacity().map(Some)
}

/// Get battery capacity via upower
fn upower_battery_capacity() -> Result<BatteryCapacity, BatteryError> {
    let devices_output =
        Command::new("upower")
            .arg("-e")
            .output()
            .map_err(|_| BatteryError::ToolUnavailable {
                tool: "upower".to_string(),
            })?;

    let devices = String::from_utf8_lossy(&devices_output.stdout);
    let battery_path = devices
        .lines()
        .find(|line| line.contains("BAT"))
        .ok_or(BatteryError::NotFound)?;

    let info_output = Command::new("upower")
        .args(["-i", battery_path])
        .output()
        .map_err(|_| BatteryError::ToolUnavailable {
            tool: "upower".to_string(),
        })?;

    let info = String::from_utf8_lossy(&info_output.stdout);

    let design_wh = parse_upower_field(&info, "energy-full-design").ok();
    let full_wh = parse_upower_field(&info, "energy-full").ok();

    Ok(BatteryCapacity { design_wh, full_wh })
}

/// Get battery capacity via sysfs
fn sysfs_battery_capacity() -> Result<BatteryCapacity, BatteryError> {
    let power_supply_dir = "/sys/class/power_supply";
    let entries = fs::read_dir(power_supply_dir).map_err(|_| BatteryError::NotFound)?;

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if name.starts_with("BAT") {
            return sysfs_battery_capacity_info(&path);
        }
    }

    Err(BatteryError::NotFound)
}

/// Get capacity info from specific sysfs battery path
fn sysfs_battery_capacity_info(
    battery_path: &std::path::Path,
) -> Result<BatteryCapacity, BatteryError> {
    let mut design_wh = None;
    let mut full_wh = None;

    // Try energy_full_design (in µWh)
    let design_path = battery_path.join("energy_full_design");
    if let Ok(design_str) = fs::read_to_string(&design_path) {
        if let Ok(design_uwh) = design_str.trim().parse::<f32>() {
            design_wh = Some(design_uwh / 1_000_000.0); // Convert µWh to Wh
        }
    }

    // Try energy_full (in µWh)
    let full_path = battery_path.join("energy_full");
    if let Ok(full_str) = fs::read_to_string(&full_path) {
        if let Ok(full_uwh) = full_str.trim().parse::<f32>() {
            full_wh = Some(full_uwh / 1_000_000.0); // Convert µWh to Wh
        }
    }

    Ok(BatteryCapacity { design_wh, full_wh })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_upower_field() {
        let sample_output = r#"
Device: /org/freedesktop/UPower/devices/battery_BAT0
  native-path:          BAT0
  vendor:               LGC
  model:                LNV-45N1
  serial:               1234
  power supply:         yes
  updated:              Tue 15 Jan 2025 10:30:45 AM EST (62 seconds ago)
  has history:          yes
  has statistics:       yes
  battery
    present:             yes
    rechargeable:        yes
    state:               discharging
    warning-level:       none
    energy:              45.67 Wh
    energy-empty:        0 Wh
    energy-full:         50.12 Wh
    energy-full-design:  57.72 Wh
    energy-rate:         8.45 W
    voltage:             11.4 V
    percentage:          85%
    capacity:            86.8%
    technology:          lithium-ion
        "#;

        assert_eq!(
            parse_upower_field(sample_output, "percentage").unwrap(),
            85.0
        );
        assert_eq!(
            parse_upower_field(sample_output, "energy-rate").unwrap(),
            8.45
        );
        assert_eq!(
            parse_upower_field(sample_output, "energy-full-design").unwrap(),
            57.72
        );
    }

    #[test]
    fn test_parse_meminfo_kb() {
        assert_eq!(
            parse_meminfo_kb("MemTotal:       16384000 kB"),
            Some(16384000)
        );
        assert_eq!(
            parse_meminfo_kb("MemAvailable:    8192000 kB"),
            Some(8192000)
        );
        assert_eq!(parse_meminfo_kb("Invalid line"), None);
    }

    #[test]
    fn test_loadavg_parsing() {
        let loadavg = "0.15 0.20 0.18 1/123 456";
        let result = loadavg
            .split_whitespace()
            .next()
            .and_then(|s| s.parse::<f32>().ok());

        assert_eq!(result, Some(0.15));
    }
}
