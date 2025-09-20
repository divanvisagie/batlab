# batlab - Battery Test Harness

A cross-platform C tool for measuring and comparing battery life between FreeBSD and Linux configurations on laptops.

![batlab logo](docs/logo-256.png)

## Purpose

FreeBSD laptop users often report poor battery life compared to Linux, but lack systematic data on properly tuned configurations. This tool enables researchers to:

- **Measure battery efficiency** under different FreeBSD power management settings
- **Compare against Linux baselines** on the same hardware
- **Identify optimal FreeBSD configurations** that approach or exceed Linux battery life
- **Build a dataset** of real-world laptop power management performance

**Target hardware:** Any laptop
**Primary focus:** FreeBSD power management research vs Linux

## Quick Start

```bash
# 1. Build and initialize
make
./batlab init

# 2. Manually configure your system power management
# (Set CPU governors, powerd settings, C-states, etc.)

# 3. Start logging (Terminal 1)
./batlab log freebsd-powerd-aggressive

# 4. Run workload (Terminal 2)
./batlab run idle

# 5. Stop both with Ctrl+C when done

# 6. View results
./batlab report

# 7. Generate HTML reports (optional)
./scripts/batlab-report --all
# Open docs/index.html in browser
```

## How It Works

1. **You configure** your system power management manually
2. **Logger samples** battery %, power draw, CPU load, temperature at 1Hz
3. **Workload runs** independently in separate terminal
4. **Data collected** until battery dies or you stop manually
5. **Results compared** across different configurations and operating systems

## Installation

### From Source (Recommended)

```bash
git clone <repository>
cd batlab
make
./batlab init
```

### System Requirements
- Battery-powered laptop
- FreeBSD or Linux (macOS partially supported for development)
- C99 compiler (gcc, clang, or cc)
- Standard C library and math library (libm)
- Shell access for workload scripts

**Recommended for suspension prevention:**
- Linux: `systemd` (systemd-inhibit) or `caffeine` package
- macOS: Built-in `caffeinate` (automatic)
- FreeBSD: Manual power management configuration

**FreeBSD users:** System compiler available by default (`cc`)
**Linux users:** Install build tools: `apt install build-essential` or `yum groupinstall "Development Tools"`

## Usage

### Basic Commands

```bash
make                             # Build the binary (creates bin/batlab + symlink)
./batlab init                    # Set up directories
./batlab log <config-name>       # Start logging (Terminal 1)
./batlab run <workload>          # Run workload (Terminal 2)
./batlab report                  # View text results
./scripts/batlab-report          # Generate HTML report
./scripts/batlab-report --all    # Generate all HTML reports
./batlab export --csv data.csv   # Export for analysis
./batlab list workloads          # See available workloads
```

### Example Research Workflow

**Linux baseline:**
```bash
# Configure Linux with default power management
./batlab log linux-default
./batlab run idle    # In second terminal, run until low battery
```

**FreeBSD comparison:**
```bash
# Configure FreeBSD power management manually
sysctl hw.acpi.cpu.cx_lowest=C8
powerd_flags="-a adaptive -b minimum"
service powerd restart

./batlab log freebsd-c8-minimum
./batlab run idle    # Same workload, compare results
```

**Analysis:**
```bash
./batlab report --group-by config
./scripts/batlab-report --all    # Generate HTML reports
./batlab export --csv comparison.csv
# Open docs/index.html to view detailed comparisons
```

### Configuration Names

You choose descriptive names for your manual configurations:

- `freebsd-default` - Stock FreeBSD installation
- `freebsd-powerd-aggressive` - Minimum power settings
- `freebsd-c8-states` - Deep CPU sleep states
- `linux-baseline` - Default Linux distribution
- `linux-tlp-optimized` - TLP power optimization

The tool records your configuration name with complete system telemetry data.

### Available Workloads

- `idle` - System idle with screen on
- `stress` - CPU stress test

Add custom workloads by creating scripts in `workload/` directory following the standard interface.

## Data Output

Results stored in `data/` directory:
- `<run-id>.jsonl` - Per-second measurements (battery %, watts, CPU, temp)
- `<run-id>.meta.json` - System info and configuration metadata

### Viewing Results

**Text reports:** `batlab report` - Quick command-line summary
**HTML reports:** `./scripts/batlab-report` - Comprehensive web reports with graphs
**CSV export:** `batlab export --csv` - For spreadsheet analysis

HTML reports include:
- Interactive battery drain graphs (4-panel analysis)
- Detailed system metadata and test statistics
- Professional report format suitable for research
- Index page linking all device reports

## Research Applications

- **FreeBSD vs Linux comparison** on identical hardware
- **Power management optimization** - test different FreeBSD settings
- **Workload analysis** - how different tasks affect battery life
- **Hardware characterization** - build database across laptop models
- **Community contributions** - share results to improve FreeBSD power management

## Contributing

This tool is designed for the research community:

- **Test different hardware** - submit results from your laptop model
- **Add workloads** - create new test scenarios
- **Improve FreeBSD support** - enhance power management detection
- **Share configurations** - document effective power settings

## Building and Development

The tool is implemented in C for maximum compatibility and performance:

```bash
# Quick build (creates bin/batlab and ./batlab symlink)
make

# Debug build
make debug

# Install system-wide
sudo make install

# Clean build artifacts (removes bin/ directory)
make clean

# Run tests
make test
```

See `src/README.md` for detailed technical implementation.

## License

This project is licensed under the BSD 3-Clause License - see the [LICENSE](LICENSE) file for details.
