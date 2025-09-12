# HTML Reports System

This guide explains how to generate comprehensive HTML reports from `batlab` battery test data.

## Overview

The HTML reports system creates professional, web-based reports that include:

- **Interactive graphs** - 4-panel battery analysis charts with FreeBSD red color scheme
- **Detailed statistics** - Power consumption, efficiency metrics
- **System metadata** - Complete test environment information
- **Index dashboard** - Central hub for all device reports with embedded batlab logo
- **Embedded images** - Self-contained HTML files
- **FreeBSD styling** - Clean white background with red accents and monospaced fonts

## Quick Start

```bash
# Generate report from latest test data
./scripts/batlab-report

# Generate reports for all test data
./scripts/batlab-report --all

# Update index page only
./scripts/batlab-report --index

# Generate report for specific test
./scripts/batlab-report my-test-name
```

## Generated Structure

```
docs/
├── index.html              # Main dashboard
└── reports/
    ├── debian-default.html # Individual test reports
    ├── debian-default.png  # Graph images
    ├── freebsd-test.html
    └── freebsd-test.png
```

## Report Features

### Individual Test Reports

Each HTML report contains:

**Test Information Section:**
- Configuration name and host system
- Operating system details
- Test start time and duration
- Sampling rate and run ID

**Statistics Dashboard:**
- Test duration with sample count
- Battery performance (start → end percentage)
- Power consumption (average, min, max)
- System load (CPU and temperature)

**Interactive Graph:**
- Battery drain over time
- Power consumption patterns
- CPU load correlation
- Temperature monitoring
- Time axis in hours from test start

**Data Insights:**
- Key observations and efficiency metrics
- Performance analysis summary
- Thermal management assessment

### Index Dashboard

The main index page provides:

**Statistics Overview:**
- Total number of reports
- Unique devices tested
- Current year tracking

**Report Grid:**
- Visual cards for each test report
- Configuration name and host information
- Test date and report ID
- Direct links to detailed reports

**Quick Start Guide:**
- Command examples
- Usage instructions

## Usage Examples

### Single Report Generation

```bash
# After collecting data
batlab log freebsd-optimized
batlab run idle
# ... let test run ...

# Generate HTML report
./scripts/batlab-report
# Creates: docs/reports/freebsd-optimized.html
```

### Batch Report Generation

```bash
# Generate reports for all existing data
./scripts/batlab-report --all

# Open the dashboard
firefox docs/index.html
# or
open docs/index.html  # macOS
```

### Research Workflow

```bash
# Test multiple configurations
batlab log linux-baseline && batlab run idle
./scripts/batlab-report linux-baseline

batlab log freebsd-default && batlab run idle  
./scripts/batlab-report freebsd-default

batlab log freebsd-optimized && batlab run idle
./scripts/batlab-report freebsd-optimized

# Generate comprehensive comparison
./scripts/batlab-report --all
```

## Report Content Details

### Graphs

Four-panel analysis showing:

1. **Battery Drain** - Percentage over time (FreeBSD red line)
2. **Power Consumption** - Watts consumed (dark red line)  
3. **CPU Load** - Processor utilization (maroon line)
4. **Temperature** - System thermal state (light red line)

All graphs share the same time axis (hours from test start) for easy correlation.

### Statistics

**Duration Metrics:**
- Total test runtime in hours
- Number of data samples collected
- Sampling frequency

**Battery Performance:**
- Starting and ending battery percentage
- Total battery drain amount
- Drain rate (percentage per hour)

**Power Analysis:**
- Average power consumption
- Minimum and maximum power draw
- Power consumption range

**System Load:**
- Average CPU utilization
- Average system temperature
- Peak values for both metrics

### Insights Generation

Each report includes automatically generated insights:

- Battery efficiency assessment
- Power consumption profile analysis
- System activity correlation
- Thermal management effectiveness

## Technical Details

### Image Embedding

Graphs are embedded as base64-encoded PNG images directly in the HTML, making reports self-contained and portable.

### Responsive Design

Reports use responsive CSS with FreeBSD-themed styling for optimal viewing on:
- Desktop browsers
- Tablet displays  
- Mobile devices (with horizontal scrolling for graphs)
- Clean white backgrounds with red accents
- Monospaced fonts throughout for technical readability

### Browser Compatibility

HTML reports work in all modern browsers:
- Chrome/Chromium
- Firefox
- Safari
- Edge

### File Organization

Reports are organized by configuration name extracted from the data filename. The system automatically:
- Creates the `docs/reports/` directory
- Generates unique filenames
- Updates the index page with embedded batlab logo
- Maintains consistent FreeBSD red theme across all pages
- Uses monospaced fonts for technical accuracy

## Advanced Usage

### Custom Report Names

```bash
# Use descriptive names for better organization
./scripts/batlab-report "ThinkPad_X1_FreeBSD_Optimized"
./scripts/batlab-report "Dell_XPS_Linux_Baseline"
```

### Integration with Research

```bash
# Document your methodology
echo "FreeBSD 14.1, powerd aggressive, C8 states" > notes.txt

# Generate reports with context
./scripts/batlab-report freebsd-research-v1
./scripts/batlab-report linux-comparison-baseline

# Create presentation materials
./scripts/batlab-report --all
# Use docs/index.html as research dashboard
```

### Batch Processing

```bash
# Process all historical data
find data/ -name "*.jsonl" | while read file; do
    config=$(basename "$file" .jsonl | sed 's/.*_//')
    ./scripts/batlab-report "$config"
done
```

## Troubleshooting

### Missing Dependencies

```bash
# Install required tools
sudo apt install jq gnuplot        # Ubuntu/Debian
pkg install jq gnuplot             # FreeBSD  
brew install jq gnuplot            # macOS
```

### No Data Files

```bash
# Ensure you have collected data first
ls data/*.jsonl
# If empty, run: batlab log <config> && batlab run <workload>
```

### Report Generation Errors

```bash
# Check file permissions
chmod +x scripts/batlab-report

# Verify data file format
head -1 data/*.jsonl | jq .
```

### Graph Display Issues

- Ensure browser supports base64 images
- Check that PNG files are generated in `docs/reports/`
- Verify gnuplot installation and version

## Integration Tips

### Research Publications

- HTML reports provide publication-ready graphs
- Professional styling suitable for academic use
- Self-contained files easy to archive and share
- Consistent formatting across all reports

### Team Collaboration

- Share `docs/` directory via web server
- Use index page as team dashboard
- Archive complete report sets for reproducibility
- Include report URLs in research documentation

### Automation

```bash
# Add to test scripts
run_battery_test() {
    local config="$1"
    batlab log "$config"
    batlab run idle
    ./scripts/batlab-report "$config"
    echo "Report: file://$PWD/docs/reports/${config}.html"
}

# Usage
run_battery_test "freebsd-experiment-1"
```

## Best Practices

1. **Consistent Naming:** Use descriptive, consistent configuration names
2. **Regular Generation:** Generate reports immediately after tests
3. **Documentation:** Include methodology notes with reports
4. **Archival:** Back up complete `docs/` directories for research records
5. **Sharing:** Use index page as central hub for all stakeholders

## Future Enhancements

The HTML reporting system is designed for extensibility:

- Additional graph types (scatter plots, histograms)
- Comparative analysis between configurations
- Export options (PDF, PNG, SVG)
- Real-time report updates
- Custom styling and branding options

## Requirements

- `jq` - JSON processing
- `gnuplot` - Graph generation
- Modern web browser for viewing
- Bash shell environment

The system builds on the existing `batlab-graph` script and maintains compatibility with all current data formats.