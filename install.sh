#!/bin/sh
set -e

REPO="kamilmac/timecop"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# Detect OS and architecture
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Darwin)
            OS="darwin"
            ;;
        Linux)
            OS="linux"
            ;;
        *)
            echo "Unsupported OS: $OS"
            exit 1
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
            echo "Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac

    PLATFORM="${OS}-${ARCH}"
}

# Get latest release version
get_latest_version() {
    curl -sL "https://api.github.com/repos/${REPO}/releases/latest" | \
        grep '"tag_name":' | \
        sed -E 's/.*"([^"]+)".*/\1/'
}

main() {
    detect_platform

    echo "Detected platform: $PLATFORM"

    VERSION="${VERSION:-$(get_latest_version)}"
    if [ -z "$VERSION" ]; then
        echo "Could not determine latest version"
        exit 1
    fi

    echo "Installing timecop $VERSION..."

    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/timecop-${PLATFORM}.tar.gz"

    # Create temp directory
    TMP_DIR="$(mktemp -d)"
    trap "rm -rf $TMP_DIR" EXIT

    # Download and extract
    echo "Downloading from $DOWNLOAD_URL"
    curl -sL "$DOWNLOAD_URL" | tar -xz -C "$TMP_DIR"

    # Ensure install directory exists
    if [ ! -d "$INSTALL_DIR" ]; then
        echo "Creating $INSTALL_DIR (requires sudo)"
        sudo mkdir -p "$INSTALL_DIR"
    fi

    # Install binary
    if [ -w "$INSTALL_DIR" ]; then
        mv "$TMP_DIR/timecop" "$INSTALL_DIR/timecop"
    else
        echo "Installing to $INSTALL_DIR (requires sudo)"
        sudo mv "$TMP_DIR/timecop" "$INSTALL_DIR/timecop"
    fi

    chmod +x "$INSTALL_DIR/timecop"

    echo "timecop installed successfully to $INSTALL_DIR/timecop"
    echo ""
    echo "Run 'timecop' in any git repository to start"
}

main
