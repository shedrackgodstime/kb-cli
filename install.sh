#!/bin/bash
set -euo pipefail

# kb install script
# Usage: curl -fsSL https://raw.githubusercontent.com/shedrackgodstime/kb-cli/master/install.sh | bash

REPO="shedrackgodstime/kb-cli"
BINARY="kb"
INSTALL_DIR="${KB_INSTALL_DIR:-$HOME/.local/bin}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}✓${NC} $1"; }
warn()  { echo -e "${YELLOW}!${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1" >&2; exit 1; }

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux";;
        Darwin*) echo "darwin";;
        MINGW*|MSYS*|CYGWIN*) echo "windows";;
        *) error "Unsupported OS: $(uname -s)";;
    esac
}

# Detect architecture
detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)   echo "x86_64";;
        aarch64|arm64)   echo "aarch64";;
        *) error "Unsupported architecture: $(uname -m)";;
    esac
}

# Get latest version from GitHub
get_latest_version() {
    local version
    version=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null | grep '"tag_name"' | head -1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
    if [ -z "$version" ]; then
        error "Failed to fetch latest version. Check your network connection."
    fi
    echo "$version"
}

# Download and install
install_kb() {
    local os arch version tarball url

    os=$(detect_os)
    arch=$(detect_arch)
    version=$(get_latest_version)

    # Map to release asset names
    case "$os" in
        linux)
            tarball="${BINARY}-${version}-${arch}-unknown-linux-gnu.tar.gz"
            ;;
        darwin)
            tarball="${BINARY}-${version}-${arch}-apple-darwin.tar.gz"
            ;;
        windows)
            error "Use install.ps1 for Windows: iwr -useb https://raw.githubusercontent.com/$REPO/master/install.ps1 | iex"
            ;;
    esac

    url="https://github.com/$REPO/releases/download/${version}/${tarball}"

    echo "Installing kb ${version} (${os}/${arch})..."

    # Download to temp
    local tmpdir
    tmpdir=$(mktemp -d)
    trap "rm -rf $tmpdir" EXIT

    echo "Downloading from $url..."
    if ! curl -fsSL "$url" -o "$tmpdir/$tarball"; then
        error "Download failed. Check if version $version exists for your platform."
    fi

    # Extract
    echo "Extracting..."
    tar -xzf "$tmpdir/$tarball" -C "$tmpdir"

    # Create install dir
    mkdir -p "$INSTALL_DIR"

    # Move binary
    mv "$tmpdir/$BINARY" "$INSTALL_DIR/$BINARY"
    chmod +x "$INSTALL_DIR/$BINARY"

    info "Installed $BINARY to $INSTALL_DIR/$BINARY"

    # Check if in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            info "$INSTALL_DIR is in PATH"
            ;;
        *)
            warn "$INSTALL_DIR is not in your PATH"
            echo ""
            echo "  Add it to your shell profile:"
            echo ""
            echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
            echo ""
            echo "  Or run the binary directly:"
            echo ""
            echo "    $INSTALL_DIR/$BINARY --help"
            echo ""
            ;;
    esac
}

install_kb
