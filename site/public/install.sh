#!/usr/bin/env bash
# Cassette CLI Installation Script
# https://cassette.fm
set -e

# Configuration
REPO_OWNER="sandwichfarm"
REPO_NAME="cassette"
BINARY_NAME="cassette"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case "$OS" in
        darwin)
            PLATFORM="macOS"
            ;;
        linux)
            PLATFORM="Linux"
            ;;
        mingw*|msys*|cygwin*)
            PLATFORM="Windows"
            ;;
        *)
            error "Unsupported operating system: $OS"
            ;;
    esac
    
    case "$ARCH" in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            ;;
        *)
            error "Unsupported architecture: $ARCH"
            ;;
    esac
    
    PLATFORM_NAME="${PLATFORM}-${ARCH}"
    info "Detected platform: $PLATFORM_NAME"
}

# Get the latest release version from GitHub
get_latest_version() {
    info "Fetching latest version..."
    LATEST_VERSION=$(curl -s "https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    
    if [ -z "$LATEST_VERSION" ]; then
        error "Failed to fetch latest version"
    fi
    
    info "Latest version: $LATEST_VERSION"
}

# Check if cassette is already installed and get version
check_existing_installation() {
    if command -v cassette >/dev/null 2>&1; then
        CURRENT_VERSION=$(cassette --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
        CASSETTE_PATH=$(which cassette)
        info "Found existing installation at $CASSETTE_PATH (version: $CURRENT_VERSION)"
        
        # Compare versions if possible
        if [ "$CURRENT_VERSION" != "unknown" ] && [ "$CURRENT_VERSION" != "${LATEST_VERSION#v}" ]; then
            echo ""
            warning "Current version ($CURRENT_VERSION) differs from latest (${LATEST_VERSION#v})"
            read -p "Do you want to upgrade? (y/N) " -n 1 -r
            echo ""
            if [[ ! $REPLY =~ ^[Yy]$ ]]; then
                info "Installation cancelled"
                exit 0
            fi
        elif [ "$CURRENT_VERSION" = "${LATEST_VERSION#v}" ]; then
            success "Cassette is already up to date (version: $CURRENT_VERSION)"
            exit 0
        fi
    else
        info "No existing installation found"
    fi
}

# Determine installation directory
determine_install_dir() {
    # Try common user-writable directories in PATH
    for dir in "$HOME/.local/bin" "$HOME/bin" "/usr/local/bin"; do
        if [[ ":$PATH:" == *":$dir:"* ]] && [ -d "$dir" ] && [ -w "$dir" ]; then
            INSTALL_DIR="$dir"
            info "Will install to: $INSTALL_DIR"
            return
        fi
    done
    
    # If no writable directory in PATH, create ~/.local/bin
    INSTALL_DIR="$HOME/.local/bin"
    if [ ! -d "$INSTALL_DIR" ]; then
        info "Creating directory: $INSTALL_DIR"
        mkdir -p "$INSTALL_DIR"
    fi
    
    # Check if ~/.local/bin is in PATH
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
        warning "$INSTALL_DIR is not in PATH"
        echo ""
        echo "Add the following to your shell configuration file (.bashrc, .zshrc, etc.):"
        echo ""
        echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
        echo ""
        read -p "Continue with installation? (y/N) " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            info "Installation cancelled"
            exit 0
        fi
    fi
}

# Download and install cassette
download_and_install() {
    # Construct download URL
    if [ "$PLATFORM" = "Windows" ]; then
        ARCHIVE_EXT="zip"
        BINARY_FILE="cassette.exe"
    else
        ARCHIVE_EXT="tar.gz"
        BINARY_FILE="cassette"
    fi
    
    DOWNLOAD_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${LATEST_VERSION}/cassette-${PLATFORM_NAME}.${ARCHIVE_EXT}"
    CHECKSUM_URL="${DOWNLOAD_URL}.sha256"
    
    # Create temporary directory
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf $TEMP_DIR" EXIT
    
    info "Downloading Cassette ${LATEST_VERSION}..."
    curl -L -o "$TEMP_DIR/cassette-archive.${ARCHIVE_EXT}" "$DOWNLOAD_URL" || error "Failed to download binary"
    
    # Download and verify checksum (optional)
    if curl -L -o "$TEMP_DIR/cassette-archive.${ARCHIVE_EXT}.sha256" "$CHECKSUM_URL" 2>/dev/null; then
        info "Verifying checksum..."
        cd "$TEMP_DIR"
        if command -v shasum >/dev/null 2>&1; then
            shasum -a 256 -c "cassette-archive.${ARCHIVE_EXT}.sha256" || warning "Checksum verification failed"
        else
            warning "shasum not found, skipping checksum verification"
        fi
        cd - >/dev/null
    fi
    
    # Extract archive
    info "Extracting archive..."
    cd "$TEMP_DIR"
    if [ "$ARCHIVE_EXT" = "tar.gz" ]; then
        tar -xzf "cassette-archive.${ARCHIVE_EXT}"
    else
        unzip -q "cassette-archive.${ARCHIVE_EXT}"
    fi
    
    # Make binary executable
    if [ -f "$BINARY_FILE" ]; then
        chmod +x "$BINARY_FILE"
    else
        error "Binary not found in archive"
    fi
    
    # Move to installation directory
    info "Installing to $INSTALL_DIR..."
    mv "$BINARY_FILE" "$INSTALL_DIR/$BINARY_NAME"
    
    cd - >/dev/null
}

# Verify installation
verify_installation() {
    if command -v cassette >/dev/null 2>&1; then
        VERSION=$(cassette --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "unknown")
        success "Cassette $VERSION has been installed successfully!"
        echo ""
        echo "Get started with:"
        echo "  cassette --help"
        echo ""
        echo "Visit https://cassette.fm for documentation"
    else
        error "Installation verification failed"
    fi
}

# Main installation flow
main() {
    echo "🎵 Cassette CLI Installer"
    echo "========================"
    echo ""
    
    detect_platform
    get_latest_version
    check_existing_installation
    determine_install_dir
    download_and_install
    verify_installation
}

# Run main function
main "$@"