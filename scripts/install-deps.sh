#!/bin/bash
set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Process Exporter - Dependency Installer ===${NC}"
echo ""

# Detect OS
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
        OS_LIKE=$ID_LIKE
    elif [ -f /etc/debian_version ]; then
        OS="debian"
        OS_VERSION=$(cat /etc/debian_version)
    elif [ -f /etc/redhat-release ]; then
        OS="rhel"
        OS_VERSION=$(cat /etc/redhat-release | grep -oP '\d+' | head -1)
    else
        echo -e "${RED}Error: Unable to detect operating system${NC}"
        exit 1
    fi

    echo -e "${GREEN}Detected OS: ${OS} ${OS_VERSION}${NC}"
}

# Install runtime dependencies for Debian/Ubuntu
install_runtime_debian() {
    echo -e "${YELLOW}Installing runtime dependencies for Debian/Ubuntu...${NC}"

    apt-get update
    apt-get install -y \
        libelf1 \
        ca-certificates

    # Install kernel headers if available
    apt-get install -y linux-headers-$(uname -r) 2>/dev/null || \
        apt-get install -y linux-headers-generic 2>/dev/null || \
        echo -e "${YELLOW}Warning: Could not install kernel headers (eBPF may not work)${NC}"

    echo -e "${GREEN}✓ Runtime dependencies installed${NC}"
}

# Install runtime dependencies for Rocky Linux/RHEL/Fedora
install_runtime_redhat() {
    echo -e "${YELLOW}Installing runtime dependencies for Rocky Linux/RHEL/Fedora...${NC}"

    # Determine package manager
    if command -v dnf &> /dev/null; then
        PKG_MGR="dnf"
    elif command -v yum &> /dev/null; then
        PKG_MGR="yum"
    else
        echo -e "${RED}Error: No package manager found${NC}"
        exit 1
    fi

    $PKG_MGR update -y
    $PKG_MGR install -y \
        elfutils-libelf \
        ca-certificates

    # Install kernel headers and devel packages
    $PKG_MGR install -y kernel-devel-$(uname -r) kernel-headers-$(uname -r) 2>/dev/null || \
        $PKG_MGR install -y kernel-devel kernel-headers 2>/dev/null || \
        echo -e "${YELLOW}Warning: Could not install kernel headers (eBPF may not work)${NC}"

    echo -e "${GREEN}✓ Runtime dependencies installed${NC}"
}

# Install build dependencies for Debian/Ubuntu
install_build_debian() {
    echo -e "${YELLOW}Installing build dependencies for Debian/Ubuntu...${NC}"

    # Update package list
    apt-get update

    # Install build dependencies
    apt-get install -y \
        curl \
        build-essential \
        pkg-config \
        libssl-dev \
        clang \
        llvm \
        libelf-dev \
        gcc-multilib \
        git \
        ca-certificates

    # Install kernel headers if available
    apt-get install -y linux-headers-$(uname -r) || \
        apt-get install -y linux-headers-generic || \
        echo -e "${YELLOW}Warning: Could not install kernel headers${NC}"

    echo -e "${GREEN}✓ Debian/Ubuntu dependencies installed${NC}"
}

# Install build dependencies for Rocky Linux/RHEL/Fedora
install_build_redhat() {
    echo -e "${YELLOW}Installing build dependencies for Rocky Linux/RHEL/Fedora...${NC}"

    # Determine package manager
    if command -v dnf &> /dev/null; then
        PKG_MGR="dnf"
    elif command -v yum &> /dev/null; then
        PKG_MGR="yum"
    else
        echo -e "${RED}Error: No package manager found${NC}"
        exit 1
    fi

    # Install EPEL if needed (for RHEL/Rocky)
    if [[ "$OS" == "rhel" || "$OS" == "rocky" || "$OS" == "centos" ]]; then
        if ! rpm -q epel-release &> /dev/null; then
            echo -e "${YELLOW}Installing EPEL repository...${NC}"
            $PKG_MGR install -y epel-release || true
        fi
    fi

    # Update package list
    $PKG_MGR update -y

    # Check if curl-minimal is installed and remove it if necessary
    if rpm -q curl-minimal &> /dev/null; then
        echo -e "${YELLOW}Replacing curl-minimal with full curl package...${NC}"
        $PKG_MGR install -y curl --allowerasing
    fi

    # Install build dependencies
    # NOTE: 'curl' is removed from the list if it causes conflicts
    $PKG_MGR install -y \
        gcc \
        gcc-c++ \
        make \
        pkg-config \
        openssl-devel \
        clang \
        llvm \
        elfutils-libelf-devel \
        git \
        ca-certificates

    # Try to install curl if not present (after removing curl-minimal)
    if ! command -v curl &> /dev/null; then
        $PKG_MGR install --allowerasing -y curl || echo -e "${YELLOW}Warning: Could not install curl${NC}"
    fi

    # Install kernel headers and devel packages
    $PKG_MGR install -y kernel-devel-$(uname -r) kernel-headers-$(uname -r) || \
        $PKG_MGR install -y kernel-devel kernel-headers || \
        echo -e "${YELLOW}Warning: Could not install kernel headers${NC}"

    echo -e "${GREEN}✓ Rocky Linux/RHEL/Fedora dependencies installed${NC}"
}

# Install Rust
install_rust() {
    if command -v rustc &> /dev/null; then
        echo -e "${GREEN}✓ Rust is already installed ($(rustc --version))${NC}"
        RUST_INSTALLED=true
    else
        echo -e "${YELLOW}Installing Rust...${NC}"

        # Check if curl is available, if not try wget
        if command -v curl &> /dev/null; then
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        elif command -v wget &> /dev/null; then
            wget -qO- https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
        else
            echo -e "${RED}Error: Neither curl nor wget is available${NC}"
            exit 1
        fi

        # Source cargo env for current session
        if [ -f "$HOME/.cargo/env" ]; then
            . "$HOME/.cargo/env"
        fi

        export PATH="$HOME/.cargo/bin:$PATH"
        RUST_INSTALLED=false
        echo -e "${GREEN}✓ Rust installed successfully${NC}"
    fi
}

# Install Rust nightly and components
install_rust_nightly() {
    echo -e "${YELLOW}Installing Rust nightly toolchain and components...${NC}"

    # Ensure cargo is in PATH
    if [ -f "$HOME/.cargo/env" ]; then
        . "$HOME/.cargo/env"
    fi
    export PATH="$HOME/.cargo/bin:$PATH"

    # Install nightly toolchain
    rustup toolchain install nightly

    # Add rust-src component for nightly
    rustup component add rust-src --toolchain nightly

    # Install bpf-linker if not present
    if ! command -v bpf-linker &> /dev/null; then
        echo -e "${YELLOW}Installing bpf-linker...${NC}"
        cargo install bpf-linker
        echo -e "${GREEN}✓ bpf-linker installed${NC}"
    else
        echo -e "${GREEN}✓ bpf-linker is already installed${NC}"
    fi

    echo -e "${GREEN}✓ Rust nightly and components installed${NC}"
}

# Check if running as root (required for package installation)
check_root() {
    if [ "$EUID" -ne 0 ] && [ "$SKIP_SYSTEM_DEPS" != "1" ]; then
        echo -e "${RED}Error: This script must be run as root (or with sudo)${NC}"
        echo -e "${YELLOW}Or set SKIP_SYSTEM_DEPS=1 to skip system package installation${NC}"
        exit 1
    fi
}

# Print post-installation instructions
print_instructions() {
    echo ""
    echo -e "${GREEN}=== Installation Complete ===${NC}"
    echo ""

    if [ "$RUST_INSTALLED" = false ] && [ "$RUNTIME_ONLY" != "1" ]; then
        echo -e "${YELLOW}IMPORTANT: Run the following command to configure your current shell:${NC}"
        echo -e "  ${GREEN}source \$HOME/.cargo/env${NC}"
        echo ""
        echo -e "Or restart your shell to apply changes."
        echo ""
    fi

    if [ "$RUNTIME_ONLY" = "1" ]; then
        echo -e "${GREEN}Runtime dependencies installed. The service should now work properly.${NC}"
        echo ""
        echo -e "Start the service with:${NC}"
        echo -e "  ${GREEN}sudo systemctl start process-exporter${NC}"
        echo ""
    else
        echo -e "${GREEN}You can now build the project with:${NC}"
        echo -e "  ${GREEN}make build${NC}"
        echo ""
    fi
}

# Show usage
show_usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
  --runtime        Install only runtime dependencies (for end users)
  --build          Install build dependencies (for developers)
  --skip-system    Skip system package installation
  --rust-only      Only install Rust and related tools
  -h, --help       Show this help message

Examples:
  # For end users (after installing the package)
  sudo $0 --runtime

  # For developers (building from source)
  sudo $0 --build

  # In CI environment
  sudo $0 --build
EOF
}

# Main installation flow
main() {
    # Default to build mode
    MODE="build"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --runtime)
                MODE="runtime"
                RUNTIME_ONLY=1
                shift
                ;;
            --build)
                MODE="build"
                shift
                ;;
            --skip-system)
                SKIP_SYSTEM_DEPS=1
                shift
                ;;
            --rust-only)
                RUST_ONLY=1
                shift
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                show_usage
                exit 1
                ;;
        esac
    done

    # Detect operating system
    detect_os

    # Install system dependencies unless skipped
    if [ "$SKIP_SYSTEM_DEPS" != "1" ] && [ "$RUST_ONLY" != "1" ]; then
        check_root

        case $OS in
            ubuntu|debian|linuxmint)
                if [ "$MODE" = "runtime" ]; then
                    install_runtime_debian
                else
                    install_build_debian
                fi
                ;;
            rocky|rhel|centos|fedora|almalinux)
                if [ "$MODE" = "runtime" ]; then
                    install_runtime_redhat
                else
                    install_build_redhat
                fi
                ;;
            *)
                echo -e "${RED}Error: Unsupported operating system: $OS${NC}"
                echo -e "${YELLOW}Supported: Ubuntu, Debian, Rocky Linux, RHEL, CentOS, Fedora${NC}"
                exit 1
                ;;
        esac
    else
        echo -e "${YELLOW}Skipping system package installation${NC}"
    fi

    # Install Rust only for build mode
    if [ "$MODE" = "build" ] || [ "$RUST_ONLY" = "1" ]; then
        install_rust
        install_rust_nightly
    fi

    # Print instructions
    print_instructions
}

# Run main function
main "$@"