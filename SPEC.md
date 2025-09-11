# Battery Test Harness — SPEC

**Project codename:** `batlab`
**Target hardware:** Laptops (any model/vendor)
**Target OSes:** FreeBSD 14.3+ (first-class), Linux (modern distros)
**Design ethos:** FreeBSD-first, POSIX shell, small, boring. One shell script, a couple of helpers. No daemons, no GUIs, no bash-isms.

---

## 1. Problem Statement

We want to systematically measure and improve FreeBSD battery life on laptops, specifically targeting the gap between FreeBSD and Linux power efficiency. The research addresses the hypothesis that properly tuned FreeBSD can achieve competitive battery life, but lacks systematic measurement and optimal configuration guidance.

The tool must capture telemetry while running workloads under manually configured power management settings, and produce comparable metrics to quantify the actual battery life gap between FreeBSD configurations and Linux baselines.

**Primary research question:** Which FreeBSD configuration approaches Linux battery efficiency most closely, and under what workloads?

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
batlab.sh init
batlab.sh log <config-name>
batlab.sh run <workload> [args...]
batlab.sh report [--group-by os,config,workload] [--format table|csv|json]
batlab.sh export [--csv data/summary.csv] [--json data/summary.json]
batlab.sh list workloads
```

### 4.1 `init`

* Create the data directories; probe telemetry capabilities; print checklist.

### 4.2 `log <config-name>`

* Start continuous telemetry logging with user-provided configuration name.
* Record current system snapshot (OS, kernel, hardware, system settings).
* Sample at 1 Hz by default (configurable).
* Run until manually stopped (Ctrl+C) or battery dies.
* Configuration name is user-defined label for manual system configuration.

### 4.3 `run <workload> [args...]`

* Run workload in separate terminal while logger runs.
* Workload runs until completion or manual interrupt.
* No automatic coordination with logger - user manages both processes.

### 4.4 `report`

* Scan `data/*.jsonl` and print a table with averages and confidence bands (mean/median/p95), grouped as requested.
* Support filtering by any metadata fields.

### 4.5 `list workloads`

* List available workloads with descriptions.

### 4.6 `export`

* Emit CSV/JSON summaries for external tools.

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

## 8. Workloads (extensible)

The workload system supports any executable workload. Built-in workloads include:

* `idle`: configurable sleep with screen on.
* `web_idle`: loop rendering text pages with configurable cadence.
* `compile`: compile projects with configurable parallelism and iterations.
* `video_playback`: play media files with configurable parameters.
* `wifi_throughput`: network throughput tests with configurable patterns.
* `stress`: CPU/memory stress testing with configurable intensity.

Each workload resides in `workload/<name>.sh`, implements standard interface:
* `describe()` - return description
* `run(args...)` - execute workload until completion or interrupt
* Must exit cleanly when signaled (Ctrl+C).
* Can define custom parameters and validation.

**Usage pattern**: Run `batlab.sh log <config-name>` in one terminal, then `batlab.sh run <workload>` in another.

---

## 9. Manual Configuration System

### 9.1 Configuration Approach

**User responsibility**: All system configuration is done manually by the researcher.

**Tool responsibility**: Record the user-provided configuration name and current system state.

### 9.2 Workflow

1. **Manual setup**: User configures system power management manually (governors, C-states, services, etc.)
2. **Start logging**: `batlab.sh log my-config-name` - begins telemetry collection
3. **Run workload**: `batlab.sh run workload-name` (in separate terminal)
4. **Stop when done**: Ctrl+C to stop logging, or let battery drain completely
5. **Repeat**: Change configuration manually, repeat with new config name

### 9.3 Configuration Examples

Example configuration names (user-defined):
* `freebsd-default` - Stock FreeBSD installation
* `freebsd-powerd-min` - powerd with minimum power settings
* `freebsd-c8-states` - Aggressive C-state configuration  
* `linux-baseline` - Default Linux distribution settings
* `linux-tlp-optimized` - Linux with TLP power optimization

---

## 10. Error Handling & Edge Cases

* Telemetry unavailable → warn once, switch to slope fallback.
* Sampling gaps → mark sample invalid, do not impute.
* Workload crash → still finalize run and compute metrics on available window.
* Permission denied on sysfs → print remedial hints (group membership or `sudo`), mark controls as best‑effort.

---

## 11. Security & Safety

* No persistent root requirements; only brief `sudo` for specific steps when needed. Refuse to continue if invoked as root for entire run unless `--i‑am‑sure` set.
* Profiles must log every mutation they make to system settings.

---

## 12. Configuration

Simple `.env` file read by `batlab.sh`:

```
SAMPLING_HZ=1
RUN_DURATION_S=600
STOP_ON_PCT_DROP=5
```

CLI flags override env.

---

## 13. Reporting Details

* Aggregation uses only valid `watts` samples; if <50% valid, mark run as "low confidence".
* Print table with aligned columns; provide `--csv`/`--json` outputs.
* Optional: generate plots (PNG) of watts over time and boxplots per group. (Implemented in a separate `report.py` to keep shell pure.)

---

## 14. Directory Layout

```
.
├── batlab.sh
├── lib/
│   └── telemetry.sh

├── workload/
│   ├── idle.sh
│   ├── web_idle.sh
│   ├── compile.sh
│   ├── video_playback.sh
│   └── [unlimited custom workloads]
├── wifi_throughput.sh
│   └── stress.sh
├── data/
├── report/
└── SPEC.md
```

---

## 15. Acceptance Criteria

* **Telemetry sanity**: On Linux with upower, `avg_watts` is non‑NaN for a 5‑minute idle run; on FreeBSD with acpiconf, same.
* **Repeatability**: Two consecutive `idle` runs under the same configuration produce `avg_watts` within ±10%.
* **Comparability**: `report` groups and shows deltas between OS/config/workload combinations.
* **Portability**: Runs natively on FreeBSD 14.3+ base system; supports mainstream Linux distributions.
* **POSIX Compliance**: Pure POSIX shell - no bash-isms or GNU-specific tools required.
* **Research value**: Provides actionable data for FreeBSD power management improvement.

---

## 16. Nice‑to‑Have (later)

* Optional `turbostat`/`pmcstat` channels recorded to sidecar files.
* HTML report with sparkline charts.

---

## 17. Risks

* Firmware/EC quirks may lie about `Present rate` under low draw.
* iGPU power states differ across kernels; results may reflect driver behavior more than configurations.
* Ambient temperature variance can skew idle results.

---

## 18. License & Contribution

* 3-clause BSD. PRs must maintain POSIX shell compatibility and FreeBSD-first approach. No bash-isms, no GNU-specific tools. Test on FreeBSD base system first. Also: never suggest `nano`.
