#!/bin/sh

describe() {
    echo "CPU stress test workload"
}

run() {
    intensity="50"
    duration="3600"

    # Parse arguments
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
            *)
                echo "Unknown option: $1" >&2
                return 1
                ;;
        esac
    done

    echo "Running CPU stress at $intensity% for $duration seconds..."

    # Simple CPU stress using dd and compression
    ncpu=$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo "1")

    i=0
    while [ $i -lt "$ncpu" ]; do
        (
            end_time=$(($(date +%s) + duration))
            while [ $(date +%s) -lt $end_time ]; do
                dd if=/dev/zero bs=1M count=1 2>/dev/null | gzip >/dev/null
                sleep 0.1
            done
        ) &
        i=$((i + 1))
    done

    wait
}
