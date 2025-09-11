# batlab - Battery Test Harness

A cross-platform battery efficiency measurement tool for comparing FreeBSD and Linux power management on laptop hardware.

## Research Purpose

This tool was created to systematically measure and improve FreeBSD battery life on laptops, specifically targeting the gap between FreeBSD and Linux power efficiency. The goal is to:

- **Establish baseline measurements** comparing default FreeBSD vs Linux configurations
- **Test FreeBSD power management configurations** to find optimal settings
- **Identify the best FreeBSD setup** that approaches or exceeds Linux battery life
- **Fill the research gap** - many complaints about FreeBSD laptop battery life lack proper power tuning

**Target hardware:** Laptops (any model/vendor)  
**Focus:** FreeBSD 14.3+ power management vs mainstream Linux distributions

The research addresses the hypothesis that properly tuned FreeBSD can achieve competitive battery life, but lacks systematic measurement and optimal configuration guidance.

## How It Works

`batlab` measures power consumption, system load, and temperatures while you manually configure systems and run workloads. This enables direct comparison between your FreeBSD configurations and Linux baselines to quantify the actual battery life gap and track improvements.

**Primary comparison:** Manual FreeBSD configurations vs Linux baseline  
**Supported platforms:** FreeBSD (first-class), Linux  
**Design philosophy:** POSIX shell, FreeBSD-first, research-focused - you control the system, tool records data

## Quick Start

```bash
# Initialize the test environment
./batlab.sh init

# List available workloads
./batlab.sh list workloads

# Terminal 1: Start logging with your config name
./batlab.sh log freebsd-powerd-min

# Terminal 2: Run workload (while logging runs)
./batlab.sh run idle

# Stop both with Ctrl+C when done, then view results
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
- POSIX shell (/bin/sh - FreeBSD base system compatible)
- Battery-powered laptop
- Sudo access for system configuration changes

## Usage

### Basic Workflow

1. **Manually configure system** - Set up power management yourself
2. **Start telemetry logging** - Record with your config name  
3. **Run workload in separate terminal** - While logging continues
4. **Stop and analyze results** - Compare across your configurations

### Commands

#### `batlab.sh init`
Initialize directories and check system capabilities.

#### `batlab.sh log <config-name>`
Start continuous telemetry logging with your configuration name.

```bash
./batlab.sh log freebsd-powerd-aggressive
./batlab.sh log linux-baseline-tlp
```

#### `batlab.sh run <workload> [args...]`
Run a workload in separate terminal while logging continues.

```bash
# Basic idle test  
./batlab.sh run idle

# Video playback test
./batlab.sh run video_playback --file ~/test.mp4

# Compile workload
./batlab.sh run compile --project ~/src/myproject --iterations 3
```

**Usage pattern:** Run logger in one terminal, workload in another. Stop both with Ctrl+C when done.

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

#### `batlab.sh list workloads`
List available workloads with descriptions.

### Manual Configuration

You manually configure your system power management settings:

**FreeBSD examples:**
```bash
# Configure powerd for aggressive power saving
sysctl hw.acpi.cpu.cx_lowest=C8
powerd_flags="-a adaptive -b minimum -n minimum"  
service powerd restart

# Enable WiFi power saving
ifconfig wlan0 powersave
```

**Linux examples:**
```bash
# Set CPU governor  
echo powersave | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Enable laptop mode
echo 1 | sudo tee /proc/sys/vm/laptop_mode
```

Then use a descriptive name when logging: `./batlab.sh log freebsd-c8-powersave`

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
# Boot into Linux, manually configure baseline settings
# Terminal 1: Start logging
./batlab.sh log linux-baseline

# Terminal 2: Run tests
./batlab.sh run idle       # Let run until battery low
./batlab.sh run compile    # Restart logging between tests  
./batlab.sh run video_playback

# Boot into FreeBSD, configure default settings
# Terminal 1: Start logging  
./batlab.sh log freebsd-default

# Terminal 2: Run same tests
./batlab.sh run idle
./batlab.sh run compile
./batlab.sh run video_playback

# Configure FreeBSD for power saving, repeat
./batlab.sh log freebsd-powersave
./batlab.sh run idle
# ... etc

# Compare results
./batlab.sh report --group-by os,config
echo "How close did FreeBSD get to Linux efficiency?"
```

### Test FreeBSD Power Management Hypotheses

```bash
# Test deep C-states hypothesis
sysctl hw.acpi.cpu.cx_lowest=C8
powerd_flags="-a adaptive -b minimum -n minimum"
service powerd restart
./batlab.sh log freebsd-deep-cstates
./batlab.sh run idle  # In second terminal

# Test CPU frequency scaling hypothesis
sysctl dev.cpu.0.freq=$(sysctl -n dev.cpu.0.freq_levels | cut -d/ -f2)
sysctl hw.acpi.cpu.cx_lowest=C3  
./batlab.sh log freebsd-freq-scaling
./batlab.sh run idle

# Test WiFi power saving hypothesis
ifconfig wlan0 powersave
sysctl net.wlan.power_save=1
./batlab.sh log freebsd-wifi-powersave  
./batlab.sh run idle

# Compare results
./batlab.sh report --group-by config
echo "Which hypothesis showed the most improvement?"
```

### Complete FreeBSD Battery Life Research Suite

```bash
#!/bin/sh
# Comprehensive FreeBSD vs Linux battery life comparison

# Configuration names to test manually
configs="linux-baseline linux-tlp freebsd-default freebsd-powersave freebsd-aggressive"

# Representative workloads
workloads="idle web_idle compile video_playback"

echo "=== FreeBSD Battery Life Research Suite ==="
echo "Manual testing protocol:"
echo

for config in $configs; do
    echo "1. Configure system manually for: $config"
    echo "2. For each workload:"
    for workload in $workloads; do
        echo "   - Terminal 1: ./batlab.sh log $config-$workload"
        echo "   - Terminal 2: ./batlab.sh run $workload"
        echo "   - Let run until battery dies or sufficient data collected"
    done
    echo "3. Reboot/switch OS for next configuration"
    echo
done

echo "After all tests:"
echo "./batlab.sh report --group-by config --format table"
echo "./batlab.sh export --csv freebsd_vs_linux_battery.csv"
echo
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
```

No automatic stop conditions - you manually stop logging and workloads with Ctrl+C.

## Platform Notes

### FreeBSD (Primary Platform)
- Uses `acpiconf` and ACPI sysctls for battery data
- Temperature from CPU sensors and ACPI thermal zones
- Full `powerd` configuration and sysctl power management support
- Pure POSIX shell - no external dependencies beyond base system
- Native support for FreeBSD power management features

### Linux
- Uses `upower` or sysfs for battery data
- Temperature from thermal zones or hwmon
- Supports TLP, powertop, and governor modifications

## Troubleshooting

**Permission errors accessing battery/temperature:**
```bash
# Add user to required groups (Linux)
sudo usermod -a -G power,adm $USER

# Or run with sudo if needed
sudo ./batlab.sh log config-name
```

**No power data available:**
- Ensure running on battery (AC unplugged)
- Check `./batlab.sh init` output for capability detection
- Tool will fall back to percentage-based power estimation

**Temperature not collected:**
- Non-critical; tests continue without temperature data
- Check thermal sensor availability with `sensors` (Linux) or `sysctl hw.acpi.thermal` (FreeBSD)

## Contributing

Workloads are modular - add new ones by creating files in `workload/` directory following the standard interface.

### Hardware Diversity

This tool is designed to work on any laptop hardware. The specific hardware details (model, CPU, battery specs) are automatically captured in the data files. We encourage testing on different hardware:

- **Different laptop models**: ThinkPads, MacBooks, Dell XPS, Framework, etc.
- **Different CPUs**: Intel, AMD, ARM-based systems
- **Different battery technologies**: Various capacities and chemistries

Submit your results via pull requests to build a comprehensive dataset across hardware platforms. Each test run captures full hardware specifications, enabling hardware-specific analysis and recommendations.

**Requirements:**
- POSIX shell compatibility (FreeBSD /bin/sh first)
- No bash-isms or GNU-specific tools
- Workloads must handle interruption gracefully
- Proper error handling and logging

## License

3-clause BSD