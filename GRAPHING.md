# Battery Data Visualization

This guide explains how to generate graphs and reports from `batlab` battery data.

## Quick Start

After collecting battery data with `batlab log <config>`, you have two visualization options:

**Simple PNG graphs:**
```bash
# Auto-named PNG from latest data
./scripts/batlab-graph

# Custom filename
./scripts/batlab-graph my_battery_test.png
```

**Comprehensive HTML reports:**
```bash
# HTML report from latest data
./scripts/batlab-report

# Generate all HTML reports + index
./scripts/batlab-report --all

# View in browser: docs/index.html
```

## Requirements

Install gnuplot and jq:

```bash
# Ubuntu/Debian
sudo apt install jq gnuplot

# FreeBSD
pkg install jq gnuplot

# macOS
brew install jq gnuplot
```

## What You Get

The generated PNG contains a 4-panel analysis:

1. **Battery Drain** - Battery percentage over time
2. **Power Consumption** - Watts consumed over time  
3. **CPU Load** - Processor utilization over time
4. **Temperature** - System temperature over time

## Examples

### Basic Usage
```bash
# Collect some data first
batlab log freebsd-test
batlab run idle
# ... let it run for a while, then Ctrl+C

# Generate graph
./scripts/batlab-graph
# Creates: battery_freebsd-test.png
```

### Compare Configurations
```bash
# Test different power settings
batlab log freebsd-powerd-aggressive  
# ... run test ...
./scripts/batlab-graph freebsd_aggressive.png  # generate while data is latest

batlab log freebsd-default
# ... run test ...
./scripts/batlab-graph freebsd_default.png     # generate while data is latest
```

**Note:** The tool always uses the latest `.jsonl` file in `data/`, so generate graphs immediately after each test for best workflow.

### For Research/Reports
```bash
# Use descriptive filenames
./scripts/batlab-graph "FreeBSD_vs_Linux_Battery_Comparison.png"
./scripts/batlab-graph "ThinkPad_X1_Power_Management_Analysis.png"
```

## Understanding the Data

### Healthy Battery Drain
- Smooth, consistent downward slope in battery percentage
- Stable power consumption during similar activities
- CPU load correlates with power spikes
- Temperature stays reasonable

### Poor Power Management
- Erratic power consumption patterns
- High power draw during "idle" periods
- Frequent power spikes without corresponding CPU activity

### Key Insights to Look For
- **Battery vs Power correlation**: Does high power consumption correspond to faster battery drain?
- **CPU vs Power relationship**: Are power spikes explained by CPU activity?
- **Temperature patterns**: Does the system manage heat efficiently?
- **Idle efficiency**: How low does power consumption go during quiet periods?

## Troubleshooting

**"No JSONL files found"**
- Run `batlab log <config>` first to collect data
- Check that the `data/` directory exists and contains `.jsonl` files

**"Missing required tools"**
- Install jq and gnuplot as shown above
- Verify installation: `which jq gnuplot`

**Empty or bad graphs**
- Ensure you have multiple data points (run tests for at least 10+ minutes)
- Check that the battery was actually draining (AC adapter unplugged)

**Graph quality issues**
- Generated PNGs are 1200×800 pixels at high quality
- For presentations, the 4-panel layout works well
- Configuration name is extracted from metadata for the title

## Report Options

### HTML Reports (Recommended)
For comprehensive analysis with embedded graphs and detailed statistics:

```bash
./scripts/batlab-report --all    # Generate all reports
# Open docs/index.html in browser
```

Features:
- Professional HTML layout with embedded graphs
- Detailed system metadata and statistics  
- Index dashboard for all device reports
- Self-contained files suitable for sharing

### PNG Graphs (Simple)
For quick graph generation:

```bash
./scripts/batlab-graph my_test.png
```

### Text Reports (Basic)
If you can't install gnuplot, use batlab's built-in text reports:

```bash
batlab report
```

This provides basic statistics and summaries with no additional dependencies.

## Integration with Research Workflow

### FreeBSD Power Management Research
```bash
# Document your system setup
echo "FreeBSD 14.1, ThinkPad X1, powerd aggressive mode" > notes.txt

# Collect baseline
batlab log baseline-freebsd
batlab run idle

# Test optimization
sysctl hw.acpi.cpu.cx_lowest=C8
batlab log freebsd-c8-optimization  
batlab run idle

# Generate comprehensive reports
./scripts/batlab-report --all

# Or individual graphs if preferred
./scripts/batlab-graph baseline_freebsd.png
./scripts/batlab-graph optimized_freebsd.png

# View results: open docs/index.html
```

### Presentation Materials
**HTML Reports:**
- Professional layout suitable for research presentations
- Self-contained files with embedded graphs
- Detailed statistics and system metadata
- Index dashboard for comprehensive overviews

**PNG Graphs:**
- 4-panel layout shows comprehensive system behavior
- 1200×800 resolution at high DPI is suitable for most uses
- Configuration names from metadata appear in graph titles  
- Automatic time axis formatting (hours from test start)

## Tips

1. **Consistent test conditions**: Same workload, similar starting battery percentage
2. **Descriptive names**: Use meaningful configuration names in `batlab log`
3. **Sufficient duration**: Run tests for at least 30-60 minutes for meaningful trends
4. **Document settings**: Keep notes on system configuration alongside graphs
5. **Multiple runs**: Consider averaging results from multiple test runs

The goal is to make battery analysis as simple as possible while providing the insights needed for FreeBSD power management research.