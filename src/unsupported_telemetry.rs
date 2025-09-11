//! Unsupported platform telemetry module
//!
//! This module provides stub implementations for platforms not officially
//! supported by batlab (e.g., macOS, Windows). It allows the code to compile
//! and provides basic testing capabilities without actual telemetry collection.

use crate::{BatteryError, BatteryInfo, BatteryCapacity, TelemetryError};

/// Stub battery info for unsupported platforms
pub fn get_battery_info() -> Result<BatteryInfo, BatteryError> {
    Err(BatteryError::NotFound)
}

/// Stub CPU load for unsupported platforms
pub fn get_cpu_load() -> Result<f32, TelemetryError> {
    Err(TelemetryError::Unavailable {
        resource: "CPU load on unsupported platform".to_string(),
    })
}

/// Stub memory usage for unsupported platforms
pub fn get_memory_usage() -> Result<f32, TelemetryError> {
    Err(TelemetryError::Unavailable {
        resource: "Memory usage on unsupported platform".to_string(),
    })
}

/// Stub temperature for unsupported platforms
pub fn get_temperature() -> Result<f32, TelemetryError> {
    Err(TelemetryError::Unavailable {
        resource: "Temperature sensors on unsupported platform".to_string(),
    })
}

/// Stub battery capacity for unsupported platforms
pub fn get_battery_capacity() -> Result<Option<BatteryCapacity>, BatteryError> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unsupported_functions_return_errors() {
        assert!(get_battery_info().is_err());
        assert!(get_cpu_load().is_err());
        assert!(get_memory_usage().is_err());
        assert!(get_temperature().is_err());

        // Battery capacity should return None, not error
        assert_eq!(get_battery_capacity().unwrap(), None);
    }
}
