.PHONY: all build build-ebpf build-app clean install uninstall test help check-deps install-deps build-deb build-rpm prepare-dist

VERSION := $(shell grep '^version' Cargo.toml | head -n1 | cut -d'"' -f2)
TARGET := $(shell rustc -vV | grep host | cut -d' ' -f2)

PREFIX ?= /usr/local
BINDIR := $(PREFIX)/bin
SYSTEMD_DIR := /etc/systemd/system

# Build output directory
DIST_DIR := $(CURDIR)/dist
DEB_DIR := $(DIST_DIR)/deb
RPM_DIR := $(DIST_DIR)/rpm

GREEN := \033[0;32m
YELLOW := \033[0;33m
RED := \033[0;31m
NC := \033[0m

all: build

help:
	@echo "$(GREEN)Process Exporter - Build System$(NC)"
	@echo ""
	@echo "Available targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-20s$(NC) %s\n", $$1, $$2}'

prepare-dist: ## Prepare distribution directories
	@mkdir -p $(DEB_DIR)
	@mkdir -p $(RPM_DIR)

build: build-ebpf build-app ## Build everything (eBPF + application)

build-ebpf: ## Build eBPF program
	@echo "$(GREEN)Building eBPF program...$(NC)"
	cd ebpf && cargo +nightly build --release --target=bpfel-unknown-none -Z build-std=core
	@echo "$(GREEN)✓ eBPF build complete$(NC)"

build-app: ## Build application
	@echo "$(GREEN)Building application...$(NC)"
	cargo build --release
	@echo "$(GREEN)✓ Application build complete$(NC)"
	@echo "$(GREEN)Binary: target/release/process-exporter$(NC)"

build-deb: build prepare-dist check-deps ## Build Debian package
	@echo "$(GREEN)Building Debian package...$(NC)"
	@command -v dpkg-buildpackage >/dev/null 2>&1 || { echo "$(RED)Error: dpkg-buildpackage not found. Install with: apt-get install dpkg-dev$(NC)"; exit 1; }
	@command -v debhelper >/dev/null 2>&1 || { echo "$(RED)Error: debhelper not found. Install with: apt-get install debhelper$(NC)"; exit 1; }

	# Install dependencies if needed
	@echo "$(YELLOW)Checking build dependencies...$(NC)"
	@chmod +x scripts/install-deps.sh

	# Build DEB package
	dpkg-buildpackage -us -uc -b

	# Move packages to dist directory
	@echo "$(GREEN)Moving packages to $(DEB_DIR)...$(NC)"
	@mv ../*.deb $(DEB_DIR)/ 2>/dev/null || true
	@mv ../*.buildinfo $(DEB_DIR)/ 2>/dev/null || true
	@mv ../*.changes $(DEB_DIR)/ 2>/dev/null || true

	@echo "$(GREEN)✓ Debian package created$(NC)"
	@ls -lh $(DEB_DIR)/*.deb

build-rpm: build prepare-dist check-deps ## Build RPM package
	@echo "$(GREEN)Building RPM package...$(NC)"
	@command -v rpmbuild >/dev/null 2>&1 || { echo "$(RED)Error: rpmbuild not found. Install with: dnf install rpm-build$(NC)"; exit 1; }

	# Create rpmbuild directory structure
	@mkdir -p $(HOME)/rpmbuild/{BUILD,RPMS,SOURCES,SPECS,SRPMS}

	# Copy files to rpmbuild SOURCES directory
	@echo "$(YELLOW)Copying source files to rpmbuild/SOURCES...$(NC)"
	@cp target/release/process-exporter $(HOME)/rpmbuild/SOURCES/
	@cp scripts/process-exporter.service $(HOME)/rpmbuild/SOURCES/
	@cp scripts/process-exporter.env $(HOME)/rpmbuild/SOURCES/
	@cp scripts/install-deps.sh $(HOME)/rpmbuild/SOURCES/
	@cp README.md $(HOME)/rpmbuild/SOURCES/
	@if [ -f LICENSE ]; then \
		cp LICENSE $(HOME)/rpmbuild/SOURCES/; \
	else \
		echo "MIT License" > $(HOME)/rpmbuild/SOURCES/LICENSE; \
	fi

	# Copy spec file to SPECS
	@cp rpm/process-exporter.spec $(HOME)/rpmbuild/SPECS/

	# Build RPM
	@echo "$(YELLOW)Building RPM with rpmbuild...$(NC)"
	rpmbuild --define "_version $(VERSION)" \
		-bb $(HOME)/rpmbuild/SPECS/process-exporter.spec

	# Copy RPM to dist directory
	@echo "$(GREEN)Moving packages to $(RPM_DIR)...$(NC)"
	@cp $(HOME)/rpmbuild/RPMS/*/*.rpm $(RPM_DIR)/

	@echo "$(GREEN)✓ RPM package created$(NC)"
	@ls -lh $(RPM_DIR)/*.rpm

install: build ## Install the binary
	@echo "$(GREEN)Installing process-exporter...$(NC)"
	install -d $(DESTDIR)$(BINDIR)
	install -m 755 target/release/process-exporter $(DESTDIR)$(BINDIR)/
	@if [ -d "$(SYSTEMD_DIR)" ]; then \
		echo "$(GREEN)Installing systemd service...$(NC)"; \
		install -d $(DESTDIR)$(SYSTEMD_DIR); \
		install -m 644 scripts/process-exporter.service $(DESTDIR)$(SYSTEMD_DIR)/; \
	fi
	@echo "$(GREEN)✓ Installation complete$(NC)"

uninstall: ## Uninstall the binary
	@echo "$(YELLOW)Uninstalling process-exporter...$(NC)"
	rm -f $(DESTDIR)$(BINDIR)/process-exporter
	rm -f $(DESTDIR)$(SYSTEMD_DIR)/process-exporter.service
	@echo "$(GREEN)✓ Uninstallation complete$(NC)"

clean: ## Clean build artifacts
	@echo "$(YELLOW)Cleaning build artifacts...$(NC)"
	cargo clean
	cd ebpf && cargo clean
	rm -rf debian/process-exporter debian/.debhelper debian/files debian/*.debhelper* debian/*.substvars
	rm -rf $(HOME)/rpmbuild
	rm -rf $(DIST_DIR)
	rm -f ../*.deb ../*.buildinfo ../*.changes
	@echo "$(GREEN)✓ Clean complete$(NC)"

test: ## Run tests
	@echo "$(GREEN)Running tests...$(NC)"
	cargo test
	@echo "$(GREEN)✓ Tests complete$(NC)"

check-deps: ## Check if required dependencies are installed
	@echo "$(GREEN)Checking dependencies...$(NC)"
	@command -v rustc >/dev/null 2>&1 || { echo "$(RED)✗ Rust is not installed. Run: make install-deps$(NC)"; exit 1; }
	@command -v cargo >/dev/null 2>&1 || { echo "$(RED)✗ Cargo is not installed$(NC)"; exit 1; }
	@rustup toolchain list | grep -q nightly || { echo "$(YELLOW)⚠ Nightly toolchain is not installed. Run: make install-deps$(NC)"; exit 1; }
	@command -v bpf-linker >/dev/null 2>&1 || { echo "$(YELLOW)⚠ bpf-linker is not installed. Run: make install-deps$(NC)"; exit 1; }
	@echo "$(GREEN)✓ All dependencies are installed$(NC)"

install-deps: ## Install build dependencies
	@echo "$(GREEN)Installing dependencies...$(NC)"
	@chmod +x scripts/install-deps.sh
	@sudo ./scripts/install-deps.sh --build
	@echo "$(GREEN)✓ Dependencies installed. Please run: source \$$HOME/.cargo/env$(NC)"

dev: ## Build in development mode
	@echo "$(GREEN)Building in development mode...$(NC)"
	cd ebpf && cargo +nightly build --target=bpfel-unknown-none -Z build-std=core
	cargo build
	@echo "$(GREEN)✓ Development build complete$(NC)"

run: dev ## Build and run in development mode
	sudo target/debug/process-exporter

package: build-deb build-rpm ## Build both DEB and RPM packages
	@echo "$(GREEN)========================================$(NC)"
	@echo "$(GREEN)All packages built successfully!$(NC)"
	@echo "$(GREEN)========================================$(NC)"
	@echo ""
	@echo "DEB packages:"
	@ls -lh $(DEB_DIR)/*.deb
	@echo ""
	@echo "RPM packages:"
	@ls -lh $(RPM_DIR)/*.rpm

.DEFAULT_GOAL := help