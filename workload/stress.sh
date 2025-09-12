#!/bin/sh

# CPU stress test workload
# This workload creates CPU load to test battery drain under compute workloads

# Default values
intensity=25    # CPU load percentage (1-100)
duration=36000   # Default 10 hours

# Parse command line arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --intensity)
            intensity="$2"
            shift 2
            ;;
        --duration)
            duration="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [--intensity PERCENT] [--duration SECONDS]"
            echo "  --intensity  CPU load percentage (1-100, default: 50)"
            echo "  --duration   How long to run (default: 3600 seconds = 1 hour)"
            echo ""
            echo "Examples:"
            echo "  $0 --intensity 75 --duration 1800  # 75% CPU for 30 minutes"
            echo "  $0 --intensity 25                  # 25% CPU for 1 hour"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Validate intensity
if [ "$intensity" -lt 1 ] || [ "$intensity" -gt 100 ]; then
    echo "Error: Intensity must be between 1 and 100" >&2
    exit 1
fi

echo "🔥 Running CPU stress at ${intensity}% intensity for $duration seconds ($(($duration / 60)) minutes)..."
echo "⏹️  Press Ctrl+C to stop"

# Prevent system suspension during stress test
prevent_suspension() {
    # Try systemd-inhibit first (most common on Linux)
    if command -v systemd-inhibit >/dev/null 2>&1; then
        echo "🔒 Preventing suspension with systemd-inhibit"
        systemd-inhibit --what=sleep:idle --who=batlab-stress --why="Battery stress test in progress" sleep "$duration" &
        inhibit_pid=$!
        return 0
    fi

    # Try caffeine as fallback
    if command -v caffeine >/dev/null 2>&1; then
        echo "🔒 Preventing suspension with caffeine"
        caffeine &
        caffeine_pid=$!
        return 0
    fi

    # Try pmset on macOS
    if command -v pmset >/dev/null 2>&1; then
        echo "🔒 Preventing suspension with pmset"
        caffeinate -i &
        caffeinate_pid=$!
        return 0
    fi

    echo "⚠️  No suspension prevention tool found - system may suspend during test"
    return 1
}

# Cleanup function
cleanup() {
    echo "🔓 Re-enabling system suspension"
    [ -n "$inhibit_pid" ] && kill "$inhibit_pid" 2>/dev/null
    [ -n "$caffeine_pid" ] && kill "$caffeine_pid" 2>/dev/null
    [ -n "$caffeinate_pid" ] && kill "$caffeinate_pid" 2>/dev/null

    # Kill all stress worker processes
    jobs -p | xargs -r kill 2>/dev/null
    exit 0
}

# Set up signal handlers
trap cleanup INT TERM

# Start suspension prevention
prevent_suspension

# Get number of CPU cores
ncpu=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "1")
echo "📊 Using $ncpu CPU cores"

# Calculate work and sleep ratios based on intensity
# For 50% intensity: work 0.5s, sleep 0.5s per second
work_time=$(echo "scale=3; $intensity / 100" | bc 2>/dev/null || awk "BEGIN {print $intensity/100}")
sleep_time=$(echo "scale=3; 1 - $work_time" | bc 2>/dev/null || awk "BEGIN {print 1 - $work_time}")

echo "📈 Work ratio: ${work_time}s work, ${sleep_time}s sleep per second"

# Start CPU stress workers
i=0
while [ $i -lt "$ncpu" ]; do
    (
        end_time=$(($(date +%s) + duration))
        while [ $(date +%s) -lt $end_time ]; do
            # Do CPU-intensive work for work_time seconds
            timeout "${work_time}s" sh -c 'while true; do echo "stress" | sha256sum >/dev/null 2>&1; done' 2>/dev/null || true

            # Sleep for sleep_time seconds (if any)
            if [ "$sleep_time" != "0" ]; then
                sleep "$sleep_time" 2>/dev/null || true
            fi
        done
    ) &
    i=$((i + 1))
done

# Wait for all background jobs to complete
wait

# Clean up
cleanup

echo "✅ CPU stress workload completed"
