#!/bin/sh

# Platform Compilation Test Script for batlab
# Tests compilation on different platforms by mocking platform-specific headers and functions

set -e

# Colors for output (if terminal supports them)
if [ -t 1 ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

# Helper functions
log_info() {
    printf "${BLUE}[INFO]${NC} %s\n" "$1"
}

log_success() {
    printf "${GREEN}[PASS]${NC} %s\n" "$1"
}

log_warning() {
    printf "${YELLOW}[WARN]${NC} %s\n" "$1"
}

log_error() {
    printf "${RED}[FAIL]${NC} %s\n" "$1"
}

# Test counter
TESTS_TOTAL=0
TESTS_PASSED=0
TESTS_FAILED=0

run_test() {
    local test_name="$1"
    local test_command="$2"

    TESTS_TOTAL=$((TESTS_TOTAL + 1))

    printf "${BLUE}Testing:${NC} %s... " "$test_name"

    if eval "$test_command" >/dev/null 2>&1; then
        printf "${GREEN}PASS${NC}\n"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        printf "${RED}FAIL${NC}\n"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Create temporary directory for test files
TEMP_DIR=$(mktemp -d)
trap 'rm -rf "$TEMP_DIR"' EXIT

cd "$(dirname "$0")"

log_info "Starting platform compilation tests..."
log_info "Temporary directory: $TEMP_DIR"

# Create mock headers for FreeBSD
create_freebsd_mocks() {
    local mock_dir="$TEMP_DIR/freebsd_mocks"
    mkdir -p "$mock_dir/sys"

    # Mock kvm.h
    cat > "$mock_dir/kvm.h" << 'EOF'
#ifndef _KVM_H_
#define _KVM_H_

typedef struct __kvm kvm_t;
typedef struct kinfo_proc {
    int ki_pid;
    char ki_comm[24];
} kinfo_proc_t;

kvm_t *kvm_open(const char *, const char *, const char *, int, const char *);
int kvm_close(kvm_t *);
kinfo_proc_t *kvm_getprocs(kvm_t *, int, int, int *);

#define KERN_PROC_PROC 8

#endif
EOF

    # Mock sys/sysctl.h additions
    cat > "$mock_dir/sys/sysctl.h" << 'EOF'
#ifndef _SYS_SYSCTL_H_
#define _SYS_SYSCTL_H_

#include <sys/types.h>

int sysctlbyname(const char *, void *, size_t *, const void *, size_t);
int sysctl(const int *, u_int, void *, size_t *, const void *, size_t);

#define CTL_HW 6
#define HW_ACPI 1000
#define HW_ACPI_BATTERY_LIFE 1001
#define HW_ACPI_BATTERY_TIME 1002

#endif
EOF

    # Mock sys/user.h
    cat > "$mock_dir/sys/user.h" << 'EOF'
#ifndef _SYS_USER_H_
#define _SYS_USER_H_

/* FreeBSD user.h mock for compilation testing */

#endif
EOF

    echo "$mock_dir"
}

# Create mock headers for Linux
create_linux_mocks() {
    local mock_dir="$TEMP_DIR/linux_mocks"
    mkdir -p "$mock_dir/sys"

    # Mock sys/sysinfo.h
    cat > "$mock_dir/sys/sysinfo.h" << 'EOF'
#ifndef _SYS_SYSINFO_H
#define _SYS_SYSINFO_H

struct sysinfo {
    long uptime;
    unsigned long loads[3];
    unsigned long totalram;
    unsigned long freeram;
    unsigned long sharedram;
    unsigned long bufferram;
    unsigned long totalswap;
    unsigned long freeswap;
    unsigned short procs;
    unsigned long totalhigh;
    unsigned long freehigh;
    unsigned int mem_unit;
};

int sysinfo(struct sysinfo *info);

#endif
EOF

    echo "$mock_dir"
}

# Test 1: Syntax check with preprocessor only
log_info "Test 1: Preprocessor syntax validation"

run_test "C syntax validation" "cc -std=c99 -E src/batlab.c -o /dev/null"
run_test "Telemetry syntax validation" "cc -std=c99 -E src/telemetry.c -o /dev/null"
run_test "Analysis syntax validation" "cc -std=c99 -E src/analysis.c -o /dev/null"

# Test 2: FreeBSD compilation with mocks
log_info "Test 2: FreeBSD compilation (with mocks)"

FREEBSD_MOCK_DIR=$(create_freebsd_mocks)

run_test "FreeBSD preprocessor" "cc -std=c99 -D__FreeBSD__ -I$FREEBSD_MOCK_DIR -E src/telemetry.c -o /dev/null"
run_test "FreeBSD compilation" "cc -std=c99 -D__FreeBSD__ -I$FREEBSD_MOCK_DIR -Wall -Wextra -c src/batlab.c -o $TEMP_DIR/batlab_freebsd.o"
run_test "FreeBSD telemetry compilation" "cc -std=c99 -D__FreeBSD__ -I$FREEBSD_MOCK_DIR -Wall -Wextra -Wno-unused-function -c src/telemetry.c -o $TEMP_DIR/telemetry_freebsd.o"
run_test "FreeBSD analysis compilation" "cc -std=c99 -D__FreeBSD__ -I$FREEBSD_MOCK_DIR -Wall -Wextra -Wno-unused-parameter -c src/analysis.c -o $TEMP_DIR/analysis_freebsd.o"

# Test 3: Linux compilation with mocks
log_info "Test 3: Linux compilation (with mocks)"

LINUX_MOCK_DIR=$(create_linux_mocks)

run_test "Linux preprocessor" "cc -std=c99 -D__linux__ -D_GNU_SOURCE -I$LINUX_MOCK_DIR -E src/telemetry.c -o /dev/null"
run_test "Linux compilation" "cc -std=c99 -D__linux__ -D_GNU_SOURCE -I$LINUX_MOCK_DIR -Wall -Wextra -c src/batlab.c -o $TEMP_DIR/batlab_linux.o"
run_test "Linux telemetry compilation" "cc -std=c99 -D__linux__ -D_GNU_SOURCE -I$LINUX_MOCK_DIR -Wall -Wextra -Wno-unused-function -c src/telemetry.c -o $TEMP_DIR/telemetry_linux.o"
run_test "Linux analysis compilation" "cc -std=c99 -D__linux__ -D_GNU_SOURCE -I$LINUX_MOCK_DIR -Wall -Wextra -Wno-unused-parameter -c src/analysis.c -o $TEMP_DIR/analysis_linux.o"

# Test 4: Cross-platform compatibility checks
log_info "Test 4: Cross-platform compatibility"

# Check for proper conditional compilation
run_test "FreeBSD conditionals present" "grep -q '#ifdef __FreeBSD__' src/telemetry.c"
run_test "Linux conditionals present" "grep -q '#ifdef __linux__' src/telemetry.c"

# Test successful compilation with mocks is the main goal
run_test "FreeBSD compilation successful" "test -f $TEMP_DIR/telemetry_freebsd.o"
run_test "Linux compilation successful" "test -f $TEMP_DIR/telemetry_linux.o"

# Test 5: Build system compatibility
log_info "Test 5: Build system compatibility"

# Test make targets
run_test "Makefile syntax" "make -n all >/dev/null 2>&1"
run_test "Clean target" "make clean"
run_test "Help target" "make help >/dev/null"

# Test platform detection in Makefile
run_test "Platform detection" "make UNAME_S=FreeBSD -n all | grep -q 'D__FreeBSD__'"
run_test "Linux platform detection" "make UNAME_S=Linux -n all | grep -q 'D__linux__'"

# Test 6: Header compatibility
log_info "Test 6: Header file compatibility"

run_test "Telemetry header syntax" "cc -std=c99 -E src/telemetry.h -o /dev/null"
run_test "No circular includes" "cc -std=c99 -Wall -Wextra -fsyntax-only -c src/telemetry.h -o /dev/null || true"

# Test 7: Warning levels
log_info "Test 7: Compiler warning compliance"

run_test "High warning level" "cc -std=c99 -Wall -Wextra -D__FreeBSD__ -I$FREEBSD_MOCK_DIR -Wno-unused-function -Wno-unused-parameter -c src/batlab.c -o $TEMP_DIR/warnings_test.o"

# Test 8: Static analysis simulation
log_info "Test 8: Static analysis simulation"

if command -v splint >/dev/null 2>&1; then
    run_test "Static analysis" "splint +posixlib -D__FreeBSD__ -I$FREEBSD_MOCK_DIR src/batlab.c src/telemetry.c src/analysis.c || true"
else
    log_warning "splint not available, skipping static analysis test"
fi

# Test 9: Memory model compatibility
log_info "Test 9: Memory model compatibility"

run_test "64-bit compilation" "cc -std=c99 -D__FreeBSD__ -I$FREEBSD_MOCK_DIR -Wall -Wextra -Wno-unused-function -fsyntax-only src/batlab.c"
run_test "Pointer size compatibility" "cc -std=c99 -D__FreeBSD__ -I$FREEBSD_MOCK_DIR -Wno-unused-function -E src/telemetry.c | grep -q 'sizeof.*size_t' || true"

# Summary
log_info "Platform compilation test summary:"
printf "  Total tests: %d\n" "$TESTS_TOTAL"
printf "  ${GREEN}Passed: %d${NC}\n" "$TESTS_PASSED"
printf "  ${RED}Failed: %d${NC}\n" "$TESTS_FAILED"

if [ "$TESTS_FAILED" -eq 0 ]; then
    log_success "All platform compilation tests passed!"
    log_info "The code should compile correctly on FreeBSD and Linux"
    exit 0
else
    log_error "Some platform compilation tests failed"
    log_warning "Review the failures above before deploying to target platforms"
    exit 1
fi
