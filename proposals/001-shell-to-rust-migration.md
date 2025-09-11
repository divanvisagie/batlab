# Proposal 001: Migration from POSIX Shell to Rust Implementation

**Author:** Assistant  
**Date:** 2025-01-15  
**Status:** Approved  
**Target Version:** 2.0.0  

## Problem Statement

The current POSIX shell implementation of `batlab` has several maintainability and robustness issues that are becoming apparent as the codebase grows:

### 1. Cross-Platform Code Complexity

The shell approach to handling FreeBSD vs Linux differences is verbose and error-prone:

```sh
get_battery_info() {
    os_type="$1"
    case "$os_type" in
        Linux)
            get_battery_linux
            ;;
        FreeBSD)
            get_battery_freebsd
            ;;
        *)
            echo "pct:0,watts:0,src:unsupported"
            ;;
    esac
}
```

This pattern is repeated throughout the codebase, leading to:
- Runtime OS detection overhead on every function call
- Duplicate code paths that are difficult to maintain
- Complex fallback chains that are hard to debug

### 2. String Processing Fragility

Battery telemetry parsing relies on brittle shell string manipulation:

```sh
pct=$(echo "$info" | grep percentage | grep -o '[0-9]*' | head -1)
watts=$(echo "$info" | grep energy-rate | grep -o '[0-9.]*' | head -1)
```

This creates multiple failure points:
- Pipeline failures can produce empty values
- Regex patterns may not match variant output formats
- No validation of parsed numeric values
- Dependency on external tools (`bc` for floating point arithmetic)

### 3. Error Handling Limitations

Shell error handling is primitive:
- No structured error types
- Limited ability to propagate error context
- Difficult to distinguish between different failure modes
- Silent failures in pipelines can corrupt data

### 4. JSON Generation Issues

Manual JSON string construction is error-prone:

```sh
cat << EOJSON
{"t":"$timestamp","pct":$pct,"watts":$watts,"cpu_load":$cpu_load,"ram_pct":$ram_pct,"temp_c":$temp_c,"src":"$src"}
EOJSON
```

Problems include:
- No escaping of special characters
- Risk of malformed JSON with null/empty values
- Difficult to extend with new fields

## Proposed Solution: Migration to Rust

### Overview

Migrate the core telemetry collection and data processing components from POSIX shell to Rust, while maintaining the CLI interface and overall architecture.

### Benefits

#### 1. Elegant Cross-Platform Handling

Use Rust's `cfg` attributes for compile-time platform selection:

```rust
#[cfg(target_os = "freebsd")]
mod freebsd_telemetry;
#[cfg(target_os = "linux")]
mod linux_telemetry;

#[cfg(target_os = "freebsd")]
pub use freebsd_telemetry::*;
#[cfg(target_os = "linux")]
pub use linux_telemetry::*;

pub fn sample_battery() -> Result<BatteryInfo, BatteryError> {
    get_battery_info() // Resolved at compile time
}
```

This approach provides:
- Zero runtime overhead for OS detection
- Clear separation of platform-specific code
- Single point of maintenance for each platform
- Compile-time verification of platform support

#### 2. Robust Data Processing

Replace fragile string parsing with type-safe Rust structures:

```rust
#[derive(Debug, Serialize)]
pub struct BatteryInfo {
    pub percentage: f32,
    pub watts: f32,
    pub source: String,
}

#[derive(Debug, thiserror::Error)]
pub enum BatteryError {
    #[error("Battery not found")]
    NotFound,
    #[error("Permission denied accessing {source}")]
    PermissionDenied { source: String },
    #[error("Failed to parse {field}: {value}")]
    ParseError { field: String, value: String },
}

fn parse_acpiconf_output(output: &str) -> Result<BatteryInfo, BatteryError> {
    let percentage = parse_acpiconf_field(output, "Remaining capacity")?;
    let rate_mw = parse_acpiconf_field(output, "Present rate")?;
    let watts = rate_mw / 1000.0; // Convert mW to W
    
    Ok(BatteryInfo {
        percentage,
        watts,
        source: "acpiconf".to_string(),
    })
}
```

#### 3. Superior Error Handling

Rust's `Result<T, E>` system with rich error context:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelemetryError {
    #[error("Battery error: {0}")]
    Battery(#[from] BatteryError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Command failed: {command}")]
    CommandFailed { command: String },
}

// Forces explicit error handling
pub fn collect_telemetry() -> Result<TelemetrySample, TelemetryError> {
    let battery = get_battery_info()?;
    let cpu_load = get_cpu_load()?;
    // Compiler ensures all errors are handled
}
```

#### 4. Type-Safe JSON Generation

Use serde for guaranteed valid JSON output:

```rust
#[derive(Serialize)]
struct TelemetrySample {
    #[serde(rename = "t")]
    timestamp: String,
    #[serde(rename = "pct")]
    percentage: f32,
    watts: f32,
    cpu_load: f32,
    ram_pct: f32,
    temp_c: f32,
    #[serde(rename = "src")]
    source: String,
}

// Guaranteed valid JSON - no manual string construction
let json = serde_json::to_string(&sample)?;
println!("{}", json);
```

### Implementation Strategy

#### Phase 1: Core Telemetry Module
- Implement `src/telemetry.rs` with platform-specific modules
- Create `src/freebsd_telemetry.rs` and `src/linux_telemetry.rs`
- Add comprehensive error handling with `Result<T, E>`
- Implement type-safe JSON output with serde

#### Phase 2: CLI Integration
- Keep main `batlab` script in shell for CLI parsing
- Replace telemetry library calls with Rust binary invocation
- Maintain backward compatibility for existing workflows

#### Phase 3: Full Migration (Optional)
- Migrate remaining shell components if benefits are clear
- Consider keeping workload system in shell for flexibility

#### FreeBSD-First with Rust

Rust is an excellent choice for FreeBSD-first development:

- **Official Tier-2 support**: FreeBSD/amd64 with host tools
- **Package availability**: `pkg install rust` or via ports collection
- **Memory safety**: Eliminates entire classes of system-level bugs
- **Performance**: Zero-cost abstractions, efficient 1Hz sampling
- **Cross-platform**: Superior `cfg` attribute system vs C preprocessor
- **Ecosystem**: Rich crates for JSON, error handling, CLI parsing

The single-package dependency is justified by the significant safety and maintainability improvements for a research tool.

### Compatibility and Migration

#### Backward Compatibility
- Maintain existing CLI interface
- Keep same data formats (JSONL, metadata JSON)
- Preserve configuration and workflow

#### Deployment
- Single statically linked binary for each platform
- Fallback to shell implementation for unsupported platforms
- Build system using standard Makefile (no external build tools)

## Trade-offs and Considerations

### Advantages
- ✅ **Cleaner cross-platform code** with `cfg` attributes  
- ✅ **Superior error handling** with `Result<T, E>` types
- ✅ **Memory safety** eliminates buffer overflows and use-after-free
- ✅ **Type safety** prevents parsing and conversion errors
- ✅ **Guaranteed valid JSON** with serde serialization
- ✅ **FreeBSD-friendly** with official Tier-2 support
- ✅ **Modern tooling** with cargo, built-in testing, documentation

### Disadvantages  
- ❌ **Rust installation required** (`pkg install rust` on FreeBSD)
- ❌ **Compilation required** instead of single script deployment
- ❌ **Steeper learning curve** for contributors unfamiliar with Rust
- ❌ **Platform-specific binaries** needed for distribution

### Risks
- **Breaking existing workflows** during migration
- **Platform-specific compilation issues**
- **Increased complexity** for simple operations

## Implementation Plan

### Deliverables

1. **Core telemetry Rust library** (`src/lib.rs`, `src/telemetry.rs`)
2. **Platform-specific modules** (`src/freebsd_telemetry.rs`, `src/linux_telemetry.rs`)  
3. **CLI wrapper** (keep shell for argument parsing, invoke Rust binary for telemetry)
4. **Build system** (`Cargo.toml` with platform-specific dependencies)
5. **Migration guide** for existing users
6. **Comprehensive testing** on both FreeBSD and Linux with `cargo test`

### Timeline

- **Week 1-2**: Implement core telemetry module with FreeBSD support
- **Week 3**: Add Linux support and cross-platform testing
- **Week 4**: CLI integration and backward compatibility testing
- **Week 5**: Documentation and migration guide

### Success Criteria

- [ ] Maintains identical data output formats
- [ ] Passes existing test scenarios on FreeBSD and Linux
- [ ] Improves telemetry sampling reliability by >95%
- [ ] Reduces cross-platform code duplication by >60%
- [ ] Preserves zero-dependency FreeBSD base system compatibility

## Decision: Rust Implementation

**Proceeding with Rust migration** for the core telemetry components while maintaining shell for CLI and workload management. This approach provides the benefits of memory-safe system-level programming with superior cross-platform handling, while preserving the simplicity and flexibility of shell scripting for user-facing components.

Rust provides the best balance of safety, maintainability, and cross-platform elegance. The single `pkg install rust` dependency is justified by the significant improvements in robustness and developer experience. The cross-platform benefits and memory safety will be essential as the project scales to support more diverse hardware and research scenarios.

## Alternatives Considered

### C Implementation
- **Pros**: Zero dependencies (in FreeBSD base system), familiar to system programmers, lightweight binaries
- **Cons**: Manual memory management, basic error handling, preprocessor macros less elegant than Rust cfg attributes
- **Verdict**: Solid choice but Rust's safety benefits outweigh the single-package dependency

### Improved Shell Implementation  
- **Pros**: No migration cost, maintains current simplicity
- **Cons**: Doesn't address fundamental string parsing fragility and cross-platform verbosity
- **Verdict**: Technical debt would continue to accumulate

### Go/Python Implementation  
- **Pros**: Modern language features, good cross-platform support
- **Cons**: Runtime dependencies, conflicts with minimal dependency goals
- **Verdict**: Not aligned with project philosophy

## Next Steps

With Rust selected as the implementation language:

1. **Set up Rust project structure** with `cargo init`
2. **Design core telemetry API** with proper error types
3. **Implement FreeBSD telemetry module** using acpiconf and sysctl
4. **Add Linux telemetry module** with upower and sysfs support  
5. **Create CLI binary** that maintains shell script compatibility
6. **Comprehensive testing** on both platforms
7. **Migration documentation** for existing users

The Rust implementation will provide a solid foundation for reliable, cross-platform battery research while maintaining the tool's FreeBSD-first philosophy.
