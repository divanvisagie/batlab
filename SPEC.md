# Battery Test Harness — Technical Specification

**Project codename:** `batlab`
**Target hardware:** Laptops (any model/vendor)
**Target OSes:** FreeBSD 14.3+ (primary), Linux (secondary)
**Design ethos:** FreeBSD-first, POSIX shell, minimal dependencies

## 1. Technical Requirements

A POSIX shell-based tool for systematic battery efficiency measurement across FreeBSD and Linux configurations.

**Core Requirements:**
- Cross-platform telemetry collection (battery, CPU, memory, temperature)
- Manual configuration approach (user controls system state)
- Structured data output for analysis
- Extensible workload system
- FreeBSD base system compatibility

---

## 2. Goals & Non‑Goals

### 2.1 Goals

* Cross‑OS harness that:

  * Records manually applied **configurations** (user-defined names for system states)
  * Runs **workloads** in separate process while logging telemetry
  * Samples battery telemetry and system state at a fixed cadence
  * Emits structured logs with configuration metadata
  * Produces comparable metrics across manual configurations
* Minimal dependencies; pure shell where possible.
* Manual configuration approach - user sets up system state.
* Easy export (CSV/JSON) and comparison across runs.

### 2.2 Non‑Goals

* Not a general battery diagnostics tool.
* Not a background agent or automated scheduler.
* No kernel patching or vendor firmware juggling.
* No automatic system configuration - user manually configures system.

---

## 3. High‑Level Architecture

```
batlab.sh (CLI)
  ├─ workload/ (workload scripts - any commands/scenarios)  
  ├─ lib/telemetry.sh (OS-specific readers + system metrics)
  ├─ data/ (logs/.jsonl/.csv per run)
  └─ report/ (generated summaries & plots)
```

* **Logger**: samples telemetry and system metrics continuously, records to JSONL
* **Runner**: executes workloads in separate process while logger runs
* **Telemetry**: reads power/charge from OS sources; collects system metrics; falls back to %/time slope if needed.
* **Configuration**: user manually configures system, tool records configuration name only.
* **Reporter**: parses logs → CSV → aggregate stats → optional plots.

---

## 4. Command Line Interface (CLI)

```
batlab init
batlab log <config-name>
batlab run <workload> [args...]
batlab report [--group-by field] [--format table|csv|json]
batlab export [--csv file] [--json file]
batlab list workloads
```

**Command Specifications:**

- `init`: Initialize directories, probe capabilities
- `log <config-name>`: Start telemetry logging with user-defined configuration label
- `run <workload>`: Execute workload (separate process from logger)
- `report`: Analyze collected data, generate summary statistics
- `export`: Export structured data for external analysis
- `list`: Enumerate available workloads

---

## 5. Telemetry Sources & Fallbacks

### 5.1 FreeBSD (Primary Platform)

Priority order:

1. **acpiconf -i 0**: `Present rate` (mW), `Remaining capacity` (%), `Remaining time`.
2. **hw\.acpi.battery\` sysctls**: as supplemental/validation.
3. **Slope fallback**: `% drop` across samples.

Additional metrics:

* **CPU load**: read from `sysctl vm.loadavg` (1-minute average)
* **RAM usage**: read from `sysctl vm.stats.vm.v_*` sysctls (calculate used percentage)
* **Temperature**: read from `sysctl dev.cpu.*.temperature` or `sysctl hw.acpi.thermal.tz*`

Native FreeBSD implementation using base system tools only - no external dependencies.

### 5.2 Linux (Secondary Platform)

Priority order:

1. **upower**: `energy-rate` (W), `percentage`, `time-to-empty` when discharging.
2. **sysfs** `/sys/class/power_supply/BAT*/{power_now,voltage_now,current_now,energy_now}`; compute W if possible.
3. **Slope fallback**: if instantaneous watts unavailable or erratic, compute W via `Δ(energy)/Δt` from `energy_now` or estimate via `% drop` and design/full energy when reported.

Additional metrics:

* **CPU load**: read from `/proc/loadavg` (1-minute average)
* **RAM usage**: read from `/proc/meminfo` (calculate used percentage)
* **Temperature**: read from `/sys/class/thermal/thermal_zone*/temp` or `/sys/class/hwmon/hwmon*/temp*_input`

**Sampling cadence**: 1 Hz default; configurable 0.5–2 Hz. Reporter will aggregate to 1‑second bins.

**Outlier handling**: Hampel filter over a 15‑sec window; discard negative or >60 W spikes for this platform unless corroborated by multiple sources.

---

## 6. Metrics

Per run:

* `avg_watts` (arithmetic mean of valid samples)
* `median_watts`, `p95_watts`
* `avg_cpu_load` (arithmetic mean of CPU load samples)
* `avg_ram_pct` (arithmetic mean of RAM usage percentage)
* `avg_temp_c` (arithmetic mean of temperature samples)
* `est_battery_life_hours` = `full_energy_Wh / avg_watts` if `full_energy_Wh` known
* `pct_drop` over run window
* `samples_ok` / `samples_total`

Across runs (grouped):

* Mean ± stdev of `avg_watts`
* Delta vs. Linux baseline (primary research metric)
* FreeBSD efficiency gap analysis

---

## 7. Data Model & Files

### 7.1 Per‑sample JSONL (`data/<run_id>.jsonl`)

Each line:

```json
{
  "t": "2025-09-11T12:34:56Z",
  "pct": 83.0,
  "watts": 5.8,
  "cpu_load": 0.15,
  "ram_pct": 42.3,
  "src": "upower|acpiconf|sysfs|slope",
  "temp_c": 42.5,
  "sys": {
    "cpu_freq": 2400,
    "gpu_freq": 350,
    "custom_metric_1": "value1",
    "custom_metric_2": 42
  },
  "notes": "optional"
}
```

### 7.2 Run manifest (`data/<run_id>.meta.json`)

```json
{
  "run_id": "2025-09-11T12:30:00Z_hostname_FreeBSD_custom-config_web_idle",
  "host": "$(hostname)",
  "machine": "$(dmidecode -s system-product-name)", 
  "cpu": "$(sysctl -n hw.model)",
  "os": "FreeBSD 14.1-RELEASE",
  "kernel": "$(uname -r)",
  "config": "custom-config",
  "workload": "web_idle",
  "duration_s": 600,
  "sampling_hz": 1,
  "system_state": {
    "sysctls": {...},
    "hardware": {...},
    "processes": {...}
  },
  "battery": { "design_wh": 57.0, "full_wh": 53.2 },
  "custom_fields": {}
}
```

### 7.3 Summary CSV (`data/summary.csv`)

Columns: `run_id, os, config, workload, duration_s, avg_watts, median_watts, p95_watts, avg_cpu_load, avg_ram_pct, avg_temp_c, pct_drop, samples_ok, samples_total` plus any custom metrics from system state collection.

---

## 8. Workload System

**Interface Requirements:**
- Each workload in `workload/<name>.sh`
- Standard functions: `describe()`, `run(args...)`
- Signal handling for clean interruption
- Parameter validation and error handling

**Built-in Workloads:**
- `idle` - System idle with screen active
- `stress` - Configurable CPU/memory stress test

**Execution Model:**
- Independent process from telemetry logger
- User manages both logger and workload processes
- No automatic coordination between processes

## 9. Configuration System

**Manual Configuration Approach:**
- User configures system power management manually
- Tool records user-provided configuration name only
- No automated system changes by tool

**Configuration Workflow:**
1. Manual system configuration (powerd, governors, C-states, etc.)
2. Start telemetry: `batlab log <config-name>`
3. Execute workload: `batlab run <workload>` (separate terminal)
4. Manual termination or battery depletion

---

## 10. Error Handling

- **Telemetry unavailable**: Graceful fallback to alternative sources
- **Sampling gaps**: Mark invalid samples, continue collection
- **Workload failure**: Complete data collection on available samples
- **Permission errors**: Provide remediation guidance, continue best-effort

## 11. Security Model

- **Minimal privileges**: Brief `sudo` only when required
- **No persistent root**: Refuse root execution without explicit override
- **User responsibility**: Manual configuration means user controls system changes

## 12. Configuration Options

Environment variables in optional `.env` file:
```
SAMPLING_HZ=1    # Sample rate (0.5-2 Hz)
```

## 13. Output Formats

- **Real-time**: JSONL streaming during collection
- **Summary**: Tabular reports with statistical aggregation  
- **Export**: CSV/JSON for external analysis tools
- **Data validation**: Mark low-confidence runs (<50% valid samples)

## 14. Directory Structure

```
├── batlab                 # Main CLI interface
├── lib/telemetry.sh       # Cross-platform data collection
├── workload/              # Extensible workload scripts
├── data/                  # Output data files
├── ARCHITECTURE.md        # Implementation details
└── SPEC.md               # This specification
```

## 15. Acceptance Criteria

- **Cross-platform**: Native FreeBSD operation, Linux compatibility
- **POSIX compliance**: Pure shell, no bash-isms or GNU tools
- **Data integrity**: Consistent telemetry collection across platforms
- **Repeatability**: <±10% variance for identical configurations
- **Extensibility**: Simple addition of new workloads and telemetry sources

## 16. Implementation Notes

See `ARCHITECTURE.md` for detailed technical implementation, cross-platform considerations, and development guidelines.

## 17. License

3-clause BSD. Contributions must maintain POSIX compatibility and FreeBSD-first approach.
