#!/bin/sh

describe() {
    echo "Idle workload - sleep with screen on"
}

run() {
    duration="3600"  # Default 1 hour

    # Parse arguments
    while [ $# -gt 0 ]; do
        case "$1" in
            --duration)
                duration="$2"
                shift 2
                ;;
            *)
                echo "Unknown option: $1" >&2
                return 1
                ;;
        esac
    done

    echo "Running idle workload for $duration seconds..."
    echo "Press Ctrl+C to stop"

    # Keep screen on and just sleep
    sleep "$duration"
}
