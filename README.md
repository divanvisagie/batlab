# batlab - Battery Test Harness

Cross-platform battery testing tool for Unix systems. Measures and compares battery life between different OS configurations on laptops.

## Quick Start

```bash
# Initialize
bin/batlab init

# Start logging (terminal 1)
bin/batlab log my-config

# Run workload (terminal 2)
bin/batlab run idle

# Stop both with Ctrl+C, then view results
bin/batlab report
```

## Installation

```bash
make install        # Install to /usr/local
man batlab          # View documentation
```

Or run directly from source:
```bash
bin/batlab --help
```

## Tools

- **batlab** - Main battery testing tool
- **batlab-graph** - Generate PNG graphs
- **batlab-report** - Generate HTML reports

## Platform Support

- FreeBSD (acpiconf, sysctl)
- OpenBSD (apm)
- NetBSD (envstat)
- Linux (upower, /sys)
- macOS (ioreg, pmset)

## Documentation

```bash
man batlab          # Main tool
man batlab-graph    # Graph generation
man batlab-report   # HTML reports
```

## Dependencies

- POSIX shell
- Standard Unix tools (awk, sed, grep)
- gnuplot (for batlab-graph)

No compilation required.

## Data Format

Telemetry stored as JSONL in `data/` directory:
```json
{"t": "2024-01-20T10:30:45Z", "pct": 85, "watts": 12.5, "cpu_load": 0.45}
```

## Research Workflow

1. Configure system power management
2. Run test: `batlab log config-name` + `batlab run workload`
3. Analyze: `batlab report` or `batlab-report --all`
4. Compare different configurations

## License

See LICENSE file.