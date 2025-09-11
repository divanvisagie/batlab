# batlab Architecture

Technical implementation details for the battery test harness.

## Overview

`batlab` is a POSIX shell-based tool designed for cross-platform battery efficiency measurement with FreeBSD as the primary platform.

## Design Principles

- **FreeBSD-first**: Native support using FreeBSD base system tools
- **POSIX compliance**: Pure POSIX shell, no bash-isms or GNU-specific tools
- **Manual configuration**: User controls system state, tool records data
- **Two-process model**: Separate logging and workload execution
- **Research-focused**: Extensible, transparent, auditable

## Architecture Components

```
batlab (CLI)
├── lib/telemetry.sh (cross-platform data collection)
├── workload/ (extensible workload scripts)
├── data/ (structured logs and metadata)
└── report/ (analysis and export)
```

### Core Components

#### 1. CLI Interface (`batlab`)
- Command routing and argument parsing
- System capability detection
- Process coordination
- Error handling and logging

#### 2. Telemetry Library (`lib/telemetry.sh`)
- Cross-platform data collection
- Battery, CPU, memory, temperature sampling
- OS-specific source prioritization
- JSON output formatting

#### 3. Workload System (`workload/`)
- Standardized interface: `describe()` and `run()` functions
- Signal handling for clean interruption
- Extensible - users can add custom workloads

#### 4. Data Storage (`data/`)
- Per-sample JSONL files (`.jsonl`)
- Run metadata files (`.meta.json`)
- Structured for easy analysis

## Data Collection Strategy

### FreeBSD (Primary Platform)

**Battery telemetry priority:**
1. `acpiconf -i 0` - Present rate (mW), remaining capacity (%)
2. `sysctl hw.acpi.battery.*` - Validation and supplemental data
3. Slope fallback - Percentage-based power estimation

**System metrics:**
- CPU load: `sysctl vm.loadavg`
- Memory usage: `sysctl vm.stats.vm.v_*`
- Temperature: `sysctl dev.cpu.*.temperature` or `sysctl hw.acpi.thermal.tz*`

### Linux (Secondary Platform)

**Battery telemetry priority:**
1. `upower` - energy-rate, percentage, time-to-empty
2. sysfs `/sys/class/power_supply/BAT*/*` - Direct kernel interface
3. Slope fallback - Energy delta over time

**System metrics:**
- CPU load: `/proc/loadavg`
- Memory usage: `/proc/meminfo`
- Temperature: `/sys/class/thermal/thermal_zone*/temp` or hwmon

## Sampling and Timing

- **Default frequency**: 1Hz (configurable 0.5-2Hz)
- **Precision**: 1-second intervals (adequate for battery research)
- **Duration**: User-controlled (manual stop or battery depletion)
- **Outlier handling**: Hampel filter for >60W spikes

## Data Format

### Per-Sample JSONL
```json
{"t":"2025-01-15T10:30:45Z","pct":85.2,"watts":6.8,"cpu_load":0.15,"ram_pct":45.2,"temp_c":42.5,"src":"acpiconf"}
```

### Run Metadata
```json
{
  "run_id": "timestamp_hostname_os_config_workload",
  "host": "hostname",
  "machine": "Hardware Model",
  "cpu": "CPU Model",
  "os": "FreeBSD 14.3-RELEASE",
  "config": "user-defined-name",
  "workload": "workload-name",
  "sampling_hz": 1,
  "battery": {"design_wh": 57.0, "full_wh": 53.2}
}
```

## Process Model

### Two-Process Workflow
1. **Logger process** (`batlab log config-name`)
   - Continuous telemetry sampling
   - Signal handling for clean shutdown
   - Real-time data writing to JSONL

2. **Workload process** (`batlab run workload`)
   - Independent execution
   - No coordination with logger
   - User manages both processes manually

### Benefits
- **Isolation**: Workload crashes don't affect data collection
- **Flexibility**: Can start/stop workloads independently
- **Transparency**: Clear separation of measurement and load
- **Debugging**: Easy to trace issues to specific component

## Extensibility

### Adding Workloads
Create `workload/name.sh` with:
```bash
#!/bin/sh
describe() { echo "Workload description"; }
run() { 
    # Workload implementation
    # Accept arguments, handle signals
}
```

### Adding Telemetry Sources
Extend `lib/telemetry.sh` functions:
- `get_battery_info()` - New battery data sources
- `get_temperature()` - Additional thermal sensors
- `sample_telemetry()` - Custom metrics in `sys` field

### Cross-Platform Considerations
- Feature detection with graceful fallbacks
- OS-specific conditional logic
- POSIX compliance for portability
- FreeBSD base system compatibility priority

## Error Handling

- **Telemetry unavailable**: Switch to fallback methods, continue sampling
- **Sampling gaps**: Mark invalid samples, don't interpolate
- **Permission denied**: Provide remediation hints, continue best-effort
- **Workload failure**: Complete data collection on available samples
- **Signal handling**: Clean shutdown preserves collected data

## Security Model

- **Minimal privileges**: Brief `sudo` only when required for specific operations
- **No persistent root**: Refuse to run as root without explicit flag
- **Transparent operations**: All system changes logged for auditability
- **User responsibility**: Manual configuration means user controls system changes

## Performance Characteristics

- **Memory usage**: Minimal - streaming JSONL output
- **CPU overhead**: <1% for 1Hz sampling
- **Disk I/O**: Sequential writes, one line per second
- **Network**: None (local measurement only)
- **Battery impact**: Negligible measurement overhead

## POSIX Compliance Details

### Avoided GNU-isms
- No `readlink -f` (use `cd $(dirname) && pwd`)
- No bash arrays or associative arrays
- No `local` variables (use function scope carefully)
- No `nproc` without fallback to `sysctl hw.ncpu`
- No `[[` constructs (use `[` tests)

### FreeBSD Base System Compatibility
- Pure `/bin/sh` scripting
- Native utilities: `sysctl`, `acpiconf`, `service`
- No external package dependencies
- Tested on FreeBSD base system first

## Future Considerations

### Potential Enhancements
- HTML report generation (separate Python script to maintain shell purity)
- Additional workload types (network, storage, graphics)
- More thermal sensor sources
- Integration with FreeBSD power management frameworks

### Scalability
- Current design handles hours of continuous sampling
- JSONL format enables streaming processing of large datasets
- Metadata structure supports batch analysis across multiple runs