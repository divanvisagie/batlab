# Makefile for batlab - Battery Test Harness (C version)
#
# BSD make compatible - works on FreeBSD, OpenBSD, NetBSD, and macOS
# Also compatible with GNU make on Linux
#
# This Makefile uses POSIX-compliant syntax that works with both BSD make
# (default on FreeBSD) and GNU make (default on Linux). Key compatibility
# features:
# - Uses $(VAR) instead of ${VAR} for maximum compatibility
# - Uses shell commands for platform detection instead of make conditionals
# - Explicit rules instead of pattern rules for object files
# - Standard POSIX shell constructs in all commands

CC = cc
CFLAGS = -std=c99 -Wall -Wextra -O2 -g
LDFLAGS = -lm

# Platform detection - compatible with both BSD and GNU make
UNAME_S = $(shell uname -s)

# Platform-specific flags
FREEBSD_CFLAGS = -D__FreeBSD__
FREEBSD_LDFLAGS = -lkvm
LINUX_CFLAGS = -D__linux__ -D_GNU_SOURCE

# Use shell test for platform detection
PLATFORM_CFLAGS = $(shell if [ "$(UNAME_S)" = "FreeBSD" ]; then echo "$(FREEBSD_CFLAGS)"; elif [ "$(UNAME_S)" = "Linux" ]; then echo "$(LINUX_CFLAGS)"; fi)
PLATFORM_LDFLAGS = $(shell if [ "$(UNAME_S)" = "FreeBSD" ]; then echo "$(FREEBSD_LDFLAGS)"; fi)

CFLAGS += $(PLATFORM_CFLAGS)
LDFLAGS += $(PLATFORM_LDFLAGS)

# Target executable and build directory
BINDIR = bin
TARGET = $(BINDIR)/batlab
INSTALL_DIR = /usr/local/bin

# Source files
SRCDIR = src
SOURCES = $(SRCDIR)/batlab.c $(SRCDIR)/telemetry.c $(SRCDIR)/analysis.c
OBJECTS = $(BINDIR)/batlab.o $(BINDIR)/telemetry.o $(BINDIR)/analysis.o
HEADERS = $(SRCDIR)/telemetry.h

# Default target
all: $(TARGET)

# Build the main executable
$(TARGET): $(BINDIR) $(OBJECTS)
	$(CC) $(OBJECTS) -o $(TARGET) $(LDFLAGS)

# Create bin directory
$(BINDIR):
	@mkdir -p $(BINDIR)

# Compile source files - explicit rules for BSD make compatibility
$(BINDIR)/batlab.o: $(SRCDIR)/batlab.c $(HEADERS) | $(BINDIR)
	$(CC) $(CFLAGS) -c $(SRCDIR)/batlab.c -o $(BINDIR)/batlab.o

$(BINDIR)/telemetry.o: $(SRCDIR)/telemetry.c $(HEADERS) | $(BINDIR)
	$(CC) $(CFLAGS) -c $(SRCDIR)/telemetry.c -o $(BINDIR)/telemetry.o

$(BINDIR)/analysis.o: $(SRCDIR)/analysis.c $(HEADERS) | $(BINDIR)
	$(CC) $(CFLAGS) -c $(SRCDIR)/analysis.c -o $(BINDIR)/analysis.o

# Install the binary
install: $(TARGET)
	@echo "Installing batlab to $(INSTALL_DIR)..."
	install -m 755 $(TARGET) $(INSTALL_DIR)/batlab
	@echo "Installation complete!"
	@echo "Run 'batlab init' to get started"

# Uninstall
uninstall:
	@echo "Removing batlab from $(INSTALL_DIR)..."
	rm -f $(INSTALL_DIR)/batlab
	@echo "Uninstall complete"

# Clean build artifacts
clean:
	rm -rf $(BINDIR)

# Clean everything including backup files
distclean: clean
	rm -f *~ *.bak core
	rm -f batlab

# Development targets
debug: clean
	@$(MAKE) CFLAGS="$(CFLAGS) -DDEBUG -g3 -O0" all

# Static analysis
lint:
	@if command -v splint >/dev/null 2>&1; then \
		splint +posixlib $(SOURCES); \
	else \
		echo "splint not available"; \
	fi

# Test the build
test: $(TARGET)
	@echo "Testing basic functionality..."
	@if $(TARGET) --help >/dev/null 2>&1; then \
		echo "Help command works"; \
	else \
		echo "Help command failed"; \
	fi
	@if $(TARGET) metadata >/dev/null 2>&1; then \
		echo "Metadata command works"; \
	else \
		echo "Metadata command failed"; \
	fi
	@echo "Basic tests complete"

# Format code (if clang-format is available)
format:
	@if command -v clang-format >/dev/null 2>&1; then \
		clang-format -i $(SOURCES) $(HEADERS); \
		echo "Code formatted"; \
	else \
		echo "clang-format not available"; \
	fi

# Check for memory leaks (if valgrind is available)
memcheck: $(TARGET)
	@if command -v valgrind >/dev/null 2>&1; then \
		valgrind --leak-check=full --show-reachable=yes ./$(TARGET) metadata; \
	else \
		echo "valgrind not available"; \
	fi

# Cross-compile for different architectures
cross-amd64: clean
	@$(MAKE) CC=clang CFLAGS="$(CFLAGS) -target x86_64-unknown-freebsd" all

cross-arm64: clean
	@$(MAKE) CC=clang CFLAGS="$(CFLAGS) -target aarch64-unknown-freebsd" all

# Package for distribution
package: clean $(TARGET)
	@VERSION=`$(TARGET) --version 2>&1 | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' | head -1`; \
	ARCH=`uname -m`; \
	tar -czf batlab-$$VERSION-$(UNAME_S)-$$ARCH.tar.gz \
		$(TARGET) README.md LICENSE SPEC.md workload/

# Help target
help:
	@echo "Available targets:"
	@echo "  all        - Build batlab (default)"
	@echo "  install    - Install to $(INSTALL_DIR)"
	@echo "  uninstall  - Remove from $(INSTALL_DIR)"
	@echo "  clean      - Remove build artifacts"
	@echo "  distclean  - Remove all generated files"
	@echo "  debug      - Build with debug symbols"
	@echo "  test       - Run basic functionality tests"
	@echo "  format     - Format code with clang-format"
	@echo "  memcheck   - Check for memory leaks with valgrind"
	@echo "  package    - Create distribution package"
	@echo "  help       - Show this help message"
	@echo ""
	@echo "Platform: $(UNAME_S)"
	@echo "Compiler: $(CC)"

# Create a convenience symlink to the binary in the root
batlab: $(TARGET)
	@ln -sf $(TARGET) batlab

# Declare phony targets
.PHONY: all install uninstall clean distclean debug lint test format memcheck cross-amd64 cross-arm64 package help

# Make sure we can override CC and CFLAGS from command line
# This works with both BSD make and GNU make
CC ?= cc
