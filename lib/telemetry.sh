#!/bin/sh
# Telemetry collection library for battery, CPU, memory, temperature

# Get current timestamp in ISO format
get_timestamp() {
    date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u '+%Y-%m-%dT%H:%M:%SZ'
}

# Get battery percentage and power draw
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

get_battery_linux() {
    # Try upower first
    if command -v upower >/dev/null 2>&1; then
        battery=$(upower -e | grep 'BAT' | head -1)
        if [ -n "$battery" ]; then
            info=$(upower -i "$battery" 2>/dev/null)
            pct=$(echo "$info" | grep percentage | grep -o '[0-9]*' | head -1)
            watts=$(echo "$info" | grep energy-rate | grep -o '[0-9.]*' | head -1)

            if [ -n "$pct" ] && [ -n "$watts" ]; then
                echo "pct:${pct},watts:${watts},src:upower"
                return
            fi
        fi
    fi

    # Try sysfs fallback
    for bat in /sys/class/power_supply/BAT*; do
        if [ -d "$bat" ]; then
            if [ -f "$bat/capacity" ]; then
                pct=$(cat "$bat/capacity" 2>/dev/null || echo "0")
            fi

            watts="0"
            if [ -f "$bat/power_now" ]; then
                power_uw=$(cat "$bat/power_now" 2>/dev/null || echo "0")
                watts=$(echo "scale=2; $power_uw / 1000000" | bc 2>/dev/null || echo "0")
            fi

            echo "pct:${pct:-0},watts:${watts},src:sysfs"
            return
        fi
    done

    echo "pct:0,watts:0,src:unavailable"
}

get_battery_freebsd() {
    if command -v acpiconf >/dev/null 2>&1; then
        info=$(acpiconf -i 0 2>/dev/null)
        pct=$(echo "$info" | grep "Remaining capacity" | grep -o '[0-9]*' | head -1)
        rate_mw=$(echo "$info" | grep "Present rate" | grep -o '[0-9]*' | head -1)

        if [ -n "$rate_mw" ] && [ "$rate_mw" -gt 0 ]; then
            watts=$(echo "scale=3; $rate_mw / 1000" | bc 2>/dev/null || echo "0")
        else
            watts="0"
        fi

        echo "pct:${pct:-0},watts:${watts},src:acpiconf"
    else
        echo "pct:0,watts:0,src:unavailable"
    fi
}

# Get CPU load (1-minute average)
get_cpu_load() {
    os_type="$1"

    case "$os_type" in
        Linux)
            if [ -f /proc/loadavg ]; then
                cut -d' ' -f1 /proc/loadavg
            else
                echo "0"
            fi
            ;;
        FreeBSD)
            sysctl -n vm.loadavg 2>/dev/null | cut -d' ' -f2 || echo "0"
            ;;
        *)
            echo "0"
            ;;
    esac
}

# Get memory usage percentage
get_memory_usage() {
    os_type="$1"

    case "$os_type" in
        Linux)
            if [ -f /proc/meminfo ]; then
                awk '/MemTotal:/ {total=$2} /MemAvailable:/ {avail=$2} END {print int((total-avail)*100/total)}' /proc/meminfo 2>/dev/null || echo "0"
            else
                echo "0"
            fi
            ;;
        FreeBSD)
            # Calculate from vm.stats sysctls
            total=$(sysctl -n vm.stats.vm.v_page_count 2>/dev/null || echo "0")
            free=$(sysctl -n vm.stats.vm.v_free_count 2>/dev/null || echo "0")
            if [ "$total" -gt 0 ]; then
                echo "$total $free" | awk '{print int(($1-$2)*100/$1)}'
            else
                echo "0"
            fi
            ;;
        *)
            echo "0"
            ;;
    esac
}

# Get temperature (first available thermal sensor)
get_temperature() {
    os_type="$1"

    case "$os_type" in
        Linux)
            # Try thermal zones
            for zone in /sys/class/thermal/thermal_zone*/temp; do
                if [ -f "$zone" ]; then
                    temp_millic=$(cat "$zone" 2>/dev/null)
                    if [ -n "$temp_millic" ] && [ "$temp_millic" -gt 0 ]; then
                        echo "scale=1; $temp_millic / 1000" | bc 2>/dev/null || echo "0"
                        return
                    fi
                fi
            done

            # Try hwmon
            for hwmon in /sys/class/hwmon/hwmon*/temp*_input; do
                if [ -f "$hwmon" ]; then
                    temp_millic=$(cat "$hwmon" 2>/dev/null)
                    if [ -n "$temp_millic" ] && [ "$temp_millic" -gt 0 ]; then
                        echo "scale=1; $temp_millic / 1000" | bc 2>/dev/null || echo "0"
                        return
                    fi
                fi
            done

            echo "0"
            ;;
        FreeBSD)
            # Try CPU temperature
            temp=$(sysctl -n dev.cpu.0.temperature 2>/dev/null | sed 's/C//' || echo "")
            if [ -n "$temp" ]; then
                echo "$temp"
                return
            fi

            # Try ACPI thermal zones
            for tz in $(sysctl -N hw.acpi.thermal 2>/dev/null | grep temperature | head -1); do
                temp=$(sysctl -n "$tz" 2>/dev/null | sed 's/C//' || echo "")
                if [ -n "$temp" ]; then
                    echo "$temp"
                    return
                fi
            done

            echo "0"
            ;;
        *)
            echo "0"
            ;;
    esac
}

# Sample all telemetry and output JSON line
sample_telemetry() {
    os_type=$(uname -s | grep -q FreeBSD && echo "FreeBSD" || echo "Linux")
    timestamp=$(get_timestamp)

    # Get battery info
    battery_info=$(get_battery_info "$os_type")
    pct=$(echo "$battery_info" | cut -d, -f1 | cut -d: -f2)
    watts=$(echo "$battery_info" | cut -d, -f2 | cut -d: -f2)
    src=$(echo "$battery_info" | cut -d, -f3 | cut -d: -f2)

    # Get system metrics
    cpu_load=$(get_cpu_load "$os_type")
    ram_pct=$(get_memory_usage "$os_type")
    temp_c=$(get_temperature "$os_type")

    # Output JSON line
    cat << EOJSON
{"t":"$timestamp","pct":$pct,"watts":$watts,"cpu_load":$cpu_load,"ram_pct":$ram_pct,"temp_c":$temp_c,"src":"$src"}
EOJSON
}
