#!/bin/sh

# Idle workload - sleep with screen on
# This workload simulates a system at idle while keeping the screen active

# Default values
duration=3600  # Default 1 hour

# Parse command line arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --duration)
            duration="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [--duration SECONDS]"
            echo "  --duration  How long to run (default: 3600 seconds = 1 hour)"
            echo ""
            echo "Example: $0 --duration 1800  # Run for 30 minutes"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

echo "🛌 Running idle workload for $duration seconds ($(($duration / 60)) minutes)..."
echo "⏹️  Press Ctrl+C to stop"

# Prevent system suspension during idle test
prevent_suspension() {
    # Try systemd-inhibit first (most common on Linux)
    if command -v systemd-inhibit >/dev/null 2>&1; then
        echo "🔒 Preventing suspension with systemd-inhibit"
        systemd-inhibit --what=sleep:idle --who=batlab-idle --why="Battery idle test in progress" sleep "$duration" &
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
    exit 0
}

# Set up signal handlers
trap cleanup INT TERM

# Start suspension prevention
prevent_suspension

# Keep screen on and just sleep
# This simulates a system that's idle but with screen active
sleep "$duration"

# Clean up
cleanup

echo "✅ Idle workload completed"
