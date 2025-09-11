# batlab - Battery Test Harness

A cross-platform battery efficiency measurement tool for comparing FreeBSD and Linux power management on laptop hardware.

## Research Purpose

This tool was created to systematically measure and improve FreeBSD battery life on laptops, specifically targeting the gap between FreeBSD and Linux power efficiency. The goal is to:

- **Establish baseline measurements** comparing default FreeBSD vs Linux configurations
- **Test FreeBSD power management configurations** to find optimal settings
- **Identify the best FreeBSD setup** that approaches or exceeds Linux battery life
- **Fill the research gap** - many complaints about FreeBSD laptop battery life lack proper power tuning

**Target hardware:** Lenovo ThinkPad X1 Carbon Gen 9  
**Focus:** FreeBSD 14.3+ power management vs mainstream Linux distributions

The research addresses the hypothesis that properly tuned FreeBSD can achieve competitive battery life, but lacks systematic measurement and optimal configuration guidance.

## How It Works

`batlab` applies system power management configurations, runs standardized workloads, and measures power consumption, system load, and temperatures. This enables direct comparison between FreeBSD configurations and Linux baselines to quantify the actual battery life gap and track improvements.

**Primary comparison:** FreeBSD configurations vs Linux baseline  
**Supported platforms:** Linux, FreeBSD  
**Design philosophy:** Simple, extensible, research-focused

## Quick Start

```bash
# Initialize the test environment
./batlab.sh init

# List available configurations and workloads
./batlab.sh list configs
./batlab.sh list workloads

# Apply a configuration
./batlab.sh config apply linux-baseline

# Run a test (10 minutes, stop on 5% battery drop)
./batlab.sh run linux-baseline -- idle --duration 600

# View results
./batlab.sh report
./batlab.sh export --csv results.csv
```

## Installation

Clone the repository and ensure you're running on battery power:

```bash
git clone <repository>
cd batlab
./batlab.sh init
```

**Prerequisites:**
- Shell access (bash/zsh)
- Battery-powered laptop
- Sudo access for system configuration changes

## Usage

### Basic Workflow

1. **Create/select a configuration** - System settings to test
2. **Apply the configuration** - Modify system power management  
3. **Run workload** - Execute test while measuring power
4. **Analyze results** - Compare across configurations

### Commands

#### `batlab.sh init`
Initialize directories and check system capabilities.

#### `batlab.sh config apply <name>`
Apply a system configuration from `config/<name>.sh`.

```bash
./batlab.sh config apply fbsd-powersave
./batlab.sh config apply linux-baseline
```

#### `batlab.sh run <config> -- <workload> [args...]`
Run a workload under a configuration and collect telemetry.

```bash
# Basic idle test
./batlab.sh run linux-baseline -- idle --duration 300

# Video playback test  
./batlab.sh run fbsd-powersave -- video_playback --file ~/test.mp4 --duration 600

# Compile workload
./batlab.sh run fbsd-maxperf -- compile --project ~/src/myproject --iterations 3
```

**Stop conditions:**
- Workload completes naturally
- `--duration` seconds elapsed  
- Battery drops by configured percentage
- Manual interrupt (Ctrl+C)

#### `batlab.sh report [options]`
Analyze collected data and display results.

```bash
# Basic report
./batlab.sh report

# Group by configuration
./batlab.sh report --group-by config

# Group by OS and workload
./batlab.sh report --group-by os,workload

# Different output formats
./batlab.sh report --format csv
./batlab.sh report --format json
```

#### `batlab.sh export --csv <file> [--json <file>]`
Export summary data for external analysis.

#### `batlab.sh list [configs|workloads]`
List available configurations or workloads with descriptions.

### Configuration Files

Create custom configurations in `config/<name>.sh`:

```bash
#!/bin/sh

describe() {
    echo "Custom FreeBSD power saving configuration"
}

apply() {
    # Apply system changes
    sysctl hw.acpi.cpu.cx_lowest=C3
    powerd_flags="-a adaptive -b minimum -n minimum"
    service powerd restart
    
    # Log changes for reporting
    echo "Applied C3 C-states and minimum power policy"
}

revert() {  # optional
    # Restore previous settings
    sysctl hw.acpi.cpu.cx_lowest=C1
    service powerd onerestart
}
```

### Workload Files

Create custom workloads in `workload/<name>.sh`:

```bash
#!/bin/sh

describe() {
    echo "CPU stress test with configurable intensity"
}

run() {
    local duration="$1"
    local intensity="${2:-50}"  # Default 50% CPU
    
    # Validate parameters
    [ "$duration" -gt 0 ] || { echo "Invalid duration"; return 1; }
    
    # Run workload
    stress-ng --cpu $(nproc) --cpu-load $intensity --timeout ${duration}s
}
```

## Example Workflows

### Core Research Workflow: FreeBSD vs Linux Comparison

```bash
# Establish Linux baseline on your hardware
./batlab.sh config apply linux-baseline
./batlab.sh run linux-baseline -- idle --duration 600
./batlab.sh run linux-baseline -- compile --duration 600
./batlab.sh run linux-baseline -- video_playback --duration 600

# Test default FreeBSD (likely poor performance)
./batlab.sh config apply fbsd-default  
./batlab.sh run fbsd-default -- idle --duration 600
./batlab.sh run fbsd-default -- compile --duration 600
./batlab.sh run fbsd-default -- video_playback --duration 600

# Test aggressive FreeBSD power saving
./batlab.sh config apply fbsd-powersave
./batlab.sh run fbsd-powersave -- idle --duration 600
./batlab.sh run fbsd-powersave -- compile --duration 600  
./batlab.sh run fbsd-powersave -- video_playback --duration 600

# Compare and find the gap
./batlab.sh report --group-by os,config
echo "How close did FreeBSD get to Linux efficiency?"
```

### Test FreeBSD Power Management Hypotheses

```bash
# Test deep C-states hypothesis
cat > config/fbsd-deep-cstates.sh << 'EOF'
describe() { echo "FreeBSD with aggressive C-state policy"; }
apply() {
    sysctl hw.acpi.cpu.cx_lowest=C8
    powerd_flags="-a adaptive -b minimum -n minimum"
    service powerd restart
}
EOF

# Test CPU frequency scaling hypothesis  
cat > config/fbsd-freq-scaling.sh << 'EOF'
describe() { echo "FreeBSD with aggressive CPU frequency scaling"; }
apply() {
    # Set minimum available frequency
    sysctl dev.cpu.0.freq=$(sysctl -n dev.cpu.0.freq_levels | cut -d/ -f2)
    sysctl hw.acpi.cpu.cx_lowest=C3
}
EOF

# Test WiFi power saving hypothesis
cat > config/fbsd-wifi-powersave.sh << 'EOF'
describe() { echo "FreeBSD with WiFi power management"; }
apply() {
    ifconfig wlan0 powersave
    sysctl net.wlan.power_save=1
}
EOF

# Run systematic test of each hypothesis
for config in fbsd-deep-cstates fbsd-freq-scaling fbsd-wifi-powersave; do
    ./batlab.sh config apply "$config"
    ./batlab.sh run "$config" -- idle --duration 600
done

./batlab.sh report --group-by config
echo "Which hypothesis showed the most improvement?"
```

### Complete FreeBSD Battery Life Research Suite

```bash
#!/bin/sh
# Comprehensive FreeBSD vs Linux battery life comparison

# Linux baselines
linux_configs="linux-baseline linux-tlp-optimized"

# FreeBSD configurations to test
freebsd_configs="fbsd-default fbsd-powersave fbsd-aggressive fbsd-laptop-mode"

# Representative workloads
workloads="idle web_idle compile video_playback"

echo "=== FreeBSD Battery Life Research Suite ==="
echo "Comparing FreeBSD configurations against Linux baselines"
echo

# Test all combinations
for config in $linux_configs $freebsd_configs; do
    for workload in $workloads; do
        echo "Testing $config with $workload..."
        ./batlab.sh config apply "$config"
        ./batlab.sh run "$config" -- "$workload" --duration 600
        sleep 30  # Let system stabilize between tests
    done
done

# Generate comparison report
./batlab.sh report --group-by os,config --format table
./batlab.sh export --csv freebsd_vs_linux_battery.csv

echo 
echo "=== Results Summary ==="
echo "CSV exported to: freebsd_vs_linux_battery.csv"
echo "Analysis: How close did the best FreeBSD config get to Linux baseline?"
```

## Data Files

Results are stored in `data/`:

- `<run_id>.jsonl` - Per-second telemetry samples
- `<run_id>.meta.json` - Run metadata and system state
- `summary.csv` - Aggregate metrics across all runs

### Sample telemetry data:
```json
{
  "t": "2025-01-15T10:30:45Z",
  "pct": 85.2,
  "watts": 6.8,
  "cpu_load": 0.15,
  "ram_pct": 45.2,
  "temp_c": 42.5,
  "src": "upower",
  "sys": {
    "cpu_freq": 2400,
    "gpu_freq": 350
  }
}
```

## Metrics Collected

**Per sample (1Hz):**
- Battery percentage and power draw (watts)
- CPU load and RAM usage percentage  
- System temperature (CPU/thermal zones)
- Custom system metrics (extensible)

**Per run summary:**
- Average, median, 95th percentile power consumption
- Average CPU load, RAM usage, temperature
- Battery percentage drop over run
- Estimated battery life based on current consumption

## Configuration

Optional `.env` file:

```bash
SAMPLING_HZ=1                # Sample rate (0.5-2 Hz)
RUN_DURATION_S=600          # Default run duration  
STOP_ON_PCT_DROP=5          # Stop when battery drops this %
```

CLI flags override environment variables.

## Platform Notes

### Linux
- Uses `upower` or sysfs for battery data
- Temperature from thermal zones or hwmon
- Supports TLP, powertop, and governor modifications

### FreeBSD  
- Uses `acpiconf` and ACPI sysctls for battery data
- Temperature from CPU sensors and ACPI thermal zones
- Supports `powerd` configuration and sysctl power management

## Troubleshooting

**Permission errors accessing battery/temperature:**
```bash
# Add user to required groups (Linux)
sudo usermod -a -G power,adm $USER

# Or run specific commands with sudo
sudo ./batlab.sh run config -- workload
```

**No power data available:**
- Ensure running on battery (AC unplugged)
- Check `./batlab.sh init` output for capability detection
- Tool will fall back to percentage-based power estimation

**Temperature not collected:**
- Non-critical; tests continue without temperature data
- Check thermal sensor availability with `sensors` (Linux) or `sysctl hw.acpi.thermal` (FreeBSD)

## Contributing

Configurations and workloads are modular - add new ones by creating files in `config/` and `workload/` directories following the standard interface.

**Requirements:**
- POSIX shell compatibility
- Idempotent configuration scripts where possible  
- Proper error handling and logging

## License

3-clause BSD