# Makefile for batlab - Battery Test Harness (C version)
#
# Cross-platform build system supporting FreeBSD and Linux

CC = cc
CFLAGS = -std=c99 -Wall -Wextra -O2 -g
LDFLAGS = -lm

# Platform-specific flags
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),FreeBSD)
    CFLAGS += -D__FreeBSD__
    LDFLAGS += -lkvm
endif
ifeq ($(UNAME_S),Linux)
    CFLAGS += -D__linux__ -D_GNU_SOURCE
endif

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

# Compile source files
$(BINDIR)/%.o: $(SRCDIR)/%.c $(HEADERS) | $(BINDIR)
	$(CC) $(CFLAGS) -c $< -o $@

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
debug: CFLAGS += -DDEBUG -g3 -O0
debug: $(TARGET)

# Static analysis
lint:
	@which splint >/dev/null 2>&1 && splint +posixlib $(SOURCES) || echo "splint not available"

# Test the build
test: $(TARGET)
	@echo "Testing basic functionality..."
	$(TARGET) --help >/dev/null && echo "✅ Help command works" || echo "❌ Help command failed"
	$(TARGET) metadata >/dev/null && echo "✅ Metadata command works" || echo "❌ Metadata command failed"
	@echo "Basic tests complete"

# Format code (if clang-format is available)
format:
	@which clang-format >/dev/null 2>&1 && \
		clang-format -i $(SOURCES) $(HEADERS) && \
		echo "Code formatted" || \
		echo "clang-format not available"

# Check for memory leaks (if valgrind is available)
memcheck: $(TARGET)
	@which valgrind >/dev/null 2>&1 && \
		valgrind --leak-check=full --show-reachable=yes ./$(TARGET) metadata || \
		echo "valgrind not available"

# Cross-compile for different architectures (example for amd64/arm64)
cross-amd64:
	$(MAKE) CC=clang CFLAGS="$(CFLAGS) -target x86_64-unknown-freebsd"

cross-arm64:
	$(MAKE) CC=clang CFLAGS="$(CFLAGS) -target aarch64-unknown-freebsd"

# Package for distribution
package: clean $(TARGET)
	@VERSION=$$($(TARGET) --version 2>&1 | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' | head -1); \
	tar -czf batlab-$$VERSION-$(UNAME_S)-$$(uname -m).tar.gz \
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

# Declare phony targets
.PHONY: all install uninstall clean distclean debug lint test format memcheck cross-amd64 cross-arm64 package help

# Create a convenience symlink to the binary in the root
batlab: $(TARGET)
	@ln -sf $(TARGET) batlab

# Default make without arguments shows help
.DEFAULT_GOAL := all
