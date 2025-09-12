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

# Keep screen on and just sleep
# This simulates a system that's idle but with screen active
sleep "$duration"

echo "✅ Idle workload completed"
