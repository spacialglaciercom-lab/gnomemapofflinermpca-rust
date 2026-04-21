#!/bin/sh
# setup-freebsd.sh - Automated setup for rmpca on FreeBSD
#
# Usage: ./setup-freebsd.sh [options]
#   --minimal    Install only minimal requirements (no HTTP features)
#   --system     Install system-wide (/usr/local/bin)
#   --help       Show this help message

set -e

# Configuration
INSTALLATION_MODE="user"
INSTALL_HTTP="true"
PROJECT_DIR="$(pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse command line arguments
while [ $# -gt 0 ]; do
    case "$1" in
        --minimal)
            INSTALL_HTTP="false"
            shift
            ;;
        --system)
            INSTALLATION_MODE="system"
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "  --minimal    Install only minimal requirements (no HTTP features)"
            echo "  --system     Install system-wide (/usr/local/bin)"
            echo "  --help       Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

# Functions
log_info() {
    echo -e "${BLUE}INFO:${NC} $1"
}

log_success() {
    echo -e "${GREEN}SUCCESS:${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}WARNING:${NC} $1"
}

log_error() {
    echo -e "${RED}ERROR:${NC} $1"
}

check_command() {
    if ! command -v "$1" >/dev/null 2>&1; then
        return 1
    else
        return 0
    fi
}

print_header() {
    echo ""
    echo -e "${BLUE}========================================${NC}"
    echo -e "${BLUE}  rmpca FreeBSD Setup Script${NC}"
    echo -e "${BLUE}========================================${NC}"
    echo ""
}

print_section() {
    echo ""
    echo -e "${YELLOW}>>> $1${NC}"
    echo ""
}

# Main installation flow
main() {
    print_header

    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        log_error "Cargo.toml not found. Please run this script from the rmpca-rust directory."
        exit 1
    fi

    # Step 1: Check system requirements
    print_section "Step 1: Checking system requirements"

    # Check FreeBSD version
    if [ ! -f /etc/freebsd-release ]; then
        log_error "This script is designed for FreeBSD. Exiting."
        exit 1
    fi

    log_info "Detected FreeBSD: $(freebsd-version -u)"

    # Check for root if system installation
    if [ "$INSTALLATION_MODE" = "system" ] && [ "$(id -u)" -ne 0 ]; then
        log_error "System-wide installation requires root privileges."
        log_error "Please run with sudo or use --user flag."
        exit 1
    fi

    # Step 2: Install Rust toolchain
    print_section "Step 2: Installing Rust toolchain"

    if check_command rustc && check_command cargo; then
        log_success "Rust is already installed: $(rustc --version)"
    else
        log_info "Rust not found. Installing..."

        if check_command pkg; then
            log_info "Installing Rust via pkg (FreeBSD packages)..."
            sudo pkg install -y rust
        elif check_command curl; then
            log_info "Installing Rust via rustup..."
            curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
        else
            log_error "Neither pkg nor curl is available. Cannot install Rust."
            exit 1
        fi

        # Verify installation
        if ! check_command rustc; then
            log_error "Rust installation failed. Please install manually."
            exit 1
        fi

        log_success "Rust installed: $(rustc --version)"
    fi

    # Step 3: Install system dependencies
    if [ "$INSTALL_HTTP" = "true" ]; then
        print_section "Step 3: Installing system dependencies"

        if check_command pkgconf && check_command pkg-info; then
            log_success "System dependencies already installed"
        else
            log_info "Installing pkgconf and openssl..."
            sudo pkg install -y pkgconf openssl || {
                log_warning "pkg install failed. Trying without sudo..."
                pkg install -y pkgconf openssl || {
                    log_error "Failed to install system dependencies."
                    log_info "Continuing without HTTP features..."
                    INSTALL_HTTP="false"
                }
            }

            if [ "$INSTALL_HTTP" = "true" ]; then
                log_success "System dependencies installed"
            fi
        fi
    else
        log_info "Skipping system dependencies (--minimal mode)"
    fi

    # Step 4: Build rmpca
    print_section "Step 4: Building rmpca"

    log_info "Building release binary (this may take a few minutes)..."

    BUILD_ARGS=""
    if [ "$INSTALL_HTTP" = "false" ]; then
        BUILD_ARGS="--no-default-features"
        log_info "Building without default features (no HTTP client)"
    fi

    cargo build --release $BUILD_ARGS

    if [ $? -eq 0 ]; then
        log_success "Build completed successfully"
    else
        log_error "Build failed. Please check error messages above."
        exit 1
    fi

    # Step 5: Install rmpca
    print_section "Step 5: Installing rmpca"

    BINARY_PATH="./target/release/rmpca"

    if [ ! -f "$BINARY_PATH" ]; then
        log_error "Binary not found at: $BINARY_PATH"
        exit 1
    fi

    if [ "$INSTALLATION_MODE" = "system" ]; then
        log_info "Installing system-wide to /usr/local/bin/"
        sudo cp "$BINARY_PATH" /usr/local/bin/
        sudo chmod +x /usr/local/bin/rmpca
        INSTALL_PATH="/usr/local/bin/rmpca"
    else
        # Create user's bin directory
        mkdir -p "$HOME/.local/bin"
        log_info "Installing to user directory: $HOME/.local/bin/"
        cp "$BINARY_PATH" "$HOME/.local/bin/rmpca"
        chmod +x "$HOME/.local/bin/rmpca"
        INSTALL_PATH="$HOME/.local/bin/rmpca"

        # Add to PATH if not already there
        if ! echo "$PATH" | grep -q "$HOME/.local/bin"; then
            log_info "Adding $HOME/.local/bin to PATH..."

            # Try different shell configurations
            if [ -n "$ZSH_VERSION" ]; then
                echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
                log_info "Added to ~/.zshrc. Run: source ~/.zshrc"
            elif [ -n "$BASH_VERSION" ]; then
                echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
                log_info "Added to ~/.bashrc. Run: source ~/.bashrc"
            elif [ -f ~/.profile ]; then
                echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.profile
                log_info "Added to ~/.profile. Run: source ~/.profile"
            else
                log_warning "Could not determine shell. Please add manually:"
                log_warning "export PATH=\"$HOME/.local/bin:\$PATH\""
            fi
        fi
    fi

    log_success "rmpca installed to: $INSTALL_PATH"

    # Step 6: Create configuration directory
    print_section "Step 6: Setting up configuration"

    CONFIG_DIR="$HOME/.config"
    CACHE_DIR="$HOME/.cache/rmpca"

    mkdir -p "$CONFIG_DIR"
    mkdir -p "$CACHE_DIR"

    log_info "Configuration directory: $CONFIG_DIR"
    log_info "Cache directory: $CACHE_DIR"

    if [ -f "RouteMaster.toml.example" ]; then
        if [ ! -f "$CONFIG_DIR/RouteMaster.toml" ]; then
            log_info "Creating example configuration..."
            cp RouteMaster.toml.example "$CONFIG_DIR/RouteMaster.toml"
            log_success "Example configuration created: $CONFIG_DIR/RouteMaster.toml"
            log_info "Edit this file to customize rmpca behavior"
        else
            log_info "Configuration file already exists: $CONFIG_DIR/RouteMaster.toml"
        fi
    fi

    # Step 7: Run verification tests
    print_section "Step 7: Running verification tests"

    if check_command "$INSTALL_PATH"; then
        VERSION=$("$INSTALL_PATH" --version 2>&1 || echo "version check failed")
        log_success "Installation verified: $VERSION"
    else
        log_warning "PATH may not be updated. Try: export PATH=\"$HOME/.local/bin:\$PATH\""
    fi

    # Step 8: Summary
    print_section "Installation Complete!"

    echo ""
    echo -e "${GREEN}✓${NC} Rust toolchain installed"
    if [ "$INSTALL_HTTP" = "true" ]; then
        echo -e "${GREEN}✓${NC} System dependencies installed"
    else
        echo -e "${YELLOW}○${NC} System dependencies skipped (--minimal mode)"
    fi
    echo -e "${GREEN}✓${NC} rmpca binary built and installed"
    echo -e "${GREEN}✓${NC} Configuration directories created"
    echo ""

    echo -e "${BLUE}Quick Start:${NC}"
    echo "  Check version:"
    echo "    $INSTALL_PATH --version"
    echo ""
    echo "  Show help:"
    echo "    $INSTALL_PATH --help"
    echo ""
    echo "  Check jail status:"
    echo "    $INSTALL_PATH status --health"
    echo ""
    echo "  Compile a map:"
    echo "    $INSTALL_PATH compile-map input.geojson -o input.rmp"
    echo ""
    echo "  Optimize with cache:"
    echo "    $INSTALL_PATH optimize --cache input.rmp roads.geojson -o route.gpx"
    echo ""

    if [ "$INSTALLATION_MODE" = "user" ]; then
        echo -e "${YELLOW}Note:${NC} You may need to update your PATH or restart your shell."
        echo "Run: source ~/.profile (or source ~/.bashrc, source ~/.zshrc)"
    fi

    echo ""
    echo -e "${BLUE}For more information:${NC}"
    echo "  Configuration: $CONFIG_DIR/RouteMaster.toml"
    echo "  Documentation: FREEBSD_SETUP.md"
    echo "  README: README.md"
    echo ""
}

# Run main function
main
