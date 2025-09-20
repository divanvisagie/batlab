# Makefile for batlab - Battery Test Harness
#
# Clean Unix-style organization:
# - bin/ contains all executables
# - man/ contains manual pages
# - lib/ contains supporting libraries
# - Minimal dependencies, maximum compatibility

# Installation directories
PREFIX = /usr/local
BINDIR = $(PREFIX)/bin
MANDIR = $(PREFIX)/man/man1

# Executables
BATLAB_BIN = bin/batlab
BATLAB_GRAPH = bin/batlab-graph
BATLAB_REPORT = bin/batlab-report

# Manual pages
MAN_PAGES = man/batlab.1 man/batlab-graph.1 man/batlab-report.1

# Default target
all: ready

# Verify everything is ready to use
ready:
	@echo "batlab - Battery Test Harness"
	@echo ""
	@echo "No compilation needed! All tools ready to use:"
	@echo "  $(BATLAB_BIN)      - Main battery testing tool"
	@echo "  $(BATLAB_GRAPH)    - PNG graph generator"
	@echo "  $(BATLAB_REPORT)   - HTML report generator"
	@echo ""
	@echo "Quick start:"
	@echo "  $(BATLAB_BIN) init"
	@echo "  $(BATLAB_BIN) --help"
	@echo "  man batlab"
	@echo ""
	@echo "Platform support: FreeBSD, OpenBSD, NetBSD, Linux, macOS"
	@chmod +x $(BATLAB_BIN) $(BATLAB_GRAPH) $(BATLAB_REPORT)

# Install everything
install: ready
	@echo "Installing batlab tools to $(BINDIR)..."
	install -d $(BINDIR)
	install -m 755 $(BATLAB_BIN) $(BINDIR)/batlab
	install -m 755 $(BATLAB_GRAPH) $(BINDIR)/batlab-graph
	install -m 755 $(BATLAB_REPORT) $(BINDIR)/batlab-report
	@echo "Installing manual pages to $(MANDIR)..."
	install -d $(MANDIR)
	install -m 644 $(MAN_PAGES) $(MANDIR)/
	@echo ""
	@echo "Installation complete!"
	@echo "Run 'batlab init' to get started"
	@echo "Run 'man batlab' for documentation"

# Uninstall
uninstall:
	@echo "Removing batlab tools..."
	rm -f $(BINDIR)/batlab $(BINDIR)/batlab-graph $(BINDIR)/batlab-report
	rm -f $(MANDIR)/batlab.1 $(MANDIR)/batlab-graph.1 $(MANDIR)/batlab-report.1
	@echo "Uninstall complete"

# Test all tools
test: ready
	@echo "Testing batlab tools..."
	@if $(BATLAB_BIN) --help >/dev/null 2>&1; then \
		echo "batlab: OK"; \
	else \
		echo "batlab: FAILED"; \
	fi
	@if $(BATLAB_GRAPH) --help >/dev/null 2>&1; then \
		echo "batlab-graph: OK"; \
	else \
		echo "batlab-graph: FAILED"; \
	fi
	@if $(BATLAB_REPORT) --help >/dev/null 2>&1; then \
		echo "batlab-report: OK"; \
	else \
		echo "batlab-report: FAILED"; \
	fi
	@echo "Tool tests complete"

# Check shell syntax
check: ready
	@echo "Checking shell syntax..."
	@if command -v shellcheck >/dev/null 2>&1; then \
		echo "Running shellcheck..."; \
		shellcheck $(BATLAB_BIN) $(BATLAB_GRAPH) $(BATLAB_REPORT); \
		echo "Syntax check complete"; \
	else \
		echo "shellcheck not available - using basic syntax check"; \
		sh -n $(BATLAB_BIN) && echo "batlab: syntax OK"; \
		sh -n $(BATLAB_GRAPH) && echo "batlab-graph: syntax OK"; \
		sh -n $(BATLAB_REPORT) && echo "batlab-report: syntax OK"; \
	fi

# View manual pages (requires installation or MANPATH setup)
man:
	@echo "Manual pages:"
	@echo "  man batlab         - Main tool documentation"
	@echo "  man batlab-graph   - Graph generation"
	@echo "  man batlab-report  - HTML report generation"
	@echo ""
	@echo "To view without installation:"
	@echo "  man -l man/batlab.1"

# Create convenience symlink in root
batlab: ready
	@ln -sf $(BATLAB_BIN) batlab
	@echo "Created symlink: ./batlab -> $(BATLAB_BIN)"

# Package for distribution
package: ready
	@VERSION=`$(BATLAB_BIN) --version 2>&1 | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' | head -1` || VERSION="2.0.0"; \
	ARCH=`uname -m`; \
	UNAME_S=`uname -s`; \
	PKGNAME="batlab-$$VERSION-$$UNAME_S-$$ARCH"; \
	echo "Creating package $$PKGNAME.tar.gz..."; \
	tar -czf $$PKGNAME.tar.gz \
		bin/ man/ workload/ templates/ \
		README.md LICENSE Makefile \
		--exclude='*.bak' --exclude='*~' || \
	tar -czf batlab-$$VERSION.tar.gz \
		bin/ man/ workload/ README.md LICENSE Makefile; \
	echo "Package created"

# Clean temporary files
clean:
	rm -f *~ *.bak *.tmp
	rm -f batlab  # Remove symlink
	find . -name '*.bak' -delete 2>/dev/null || true
	find . -name '*~' -delete 2>/dev/null || true

# Show system info
info:
	@echo "System Information:"
	@echo "  OS: `uname -s`"
	@echo "  Architecture: `uname -m`"
	@echo "  Kernel: `uname -r`"
	@echo "  Shell: $$SHELL"
	@echo "  batlab version: `$(BATLAB_BIN) --version 2>/dev/null || echo 'Not available'`"

# Help
help:
	@echo "batlab - Battery Test Harness"
	@echo ""
	@echo "MAIN TARGETS:"
	@echo "  all (ready)   - Verify tools are ready (default)"
	@echo "  install       - Install to $(PREFIX)"
	@echo "  uninstall     - Remove from $(PREFIX)"
	@echo "  test          - Test all tools"
	@echo "  check         - Check shell syntax"
	@echo ""
	@echo "UTILITIES:"
	@echo "  batlab        - Create convenience symlink"
	@echo "  man           - Show manual page info"
	@echo "  package       - Create distribution package"
	@echo "  clean         - Remove temporary files"
	@echo "  info          - Show system information"
	@echo "  help          - Show this help"
	@echo ""
	@echo "DOCUMENTATION:"
	@echo "  man batlab           - Main tool manual"
	@echo "  man batlab-graph     - Graph generation manual"
	@echo "  man batlab-report    - Report generation manual"
	@echo ""
	@echo "QUICK START:"
	@echo "  $(BATLAB_BIN) init"
	@echo "  $(BATLAB_BIN) log"
	@echo "  $(BATLAB_BIN) run idle"

# Declare phony targets
.PHONY: all ready install uninstall test check man batlab package clean info help
