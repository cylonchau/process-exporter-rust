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

# Install for Debian/Ubuntu
install_debian() {
    echo -e "${YELLOW}Installing dependencies for Debian/Ubuntu...${NC}"

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

# Install for Rocky Linux/RHEL/Fedora
install_redhat() {
    echo -e "${YELLOW}Installing dependencies for Rocky Linux/RHEL/Fedora...${NC}"

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

    # Install build dependencies
    $PKG_MGR install -y \
        curl \
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
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable

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

    if [ "$RUST_INSTALLED" = false ]; then
        echo -e "${YELLOW}IMPORTANT: Run the following command to configure your current shell:${NC}"
        echo -e "  ${GREEN}source \$HOME/.cargo/env${NC}"
        echo ""
        echo -e "Or restart your shell to apply changes."
        echo ""
    fi

    echo -e "${GREEN}You can now build the project with:${NC}"
    echo -e "  ${GREEN}make build${NC}"
    echo ""
}

# Main installation flow
main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-system)
                SKIP_SYSTEM_DEPS=1
                shift
                ;;
            --rust-only)
                RUST_ONLY=1
                shift
                ;;
            -h|--help)
                echo "Usage: $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --skip-system    Skip system package installation (useful for CI)"
                echo "  --rust-only      Only install Rust and related tools"
                echo "  -h, --help       Show this help message"
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
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
                install_debian
                ;;
            rocky|rhel|centos|fedora|almalinux)
                install_redhat
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

    # Install Rust (can be done as regular user)
    install_rust

    # Install Rust nightly and eBPF tools
    install_rust_nightly

    # Print instructions
    print_instructions
}

# Run main function
main "$@"