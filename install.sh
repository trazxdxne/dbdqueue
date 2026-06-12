#!/bin/sh
set -e

# Configuration
REPO="trazxdxne/dbdqueue" # Change this to the actual GitHub username/repo
BINARY_NAME="dbdqueue"

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0;37m' # No Color

echo "${BLUE}==>${NC} Installing dbdqueue..."

# 1. Detect OS
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
if [ "$OS" != "linux" ]; then
    echo "${RED}Error:${NC} dbdqueue is currently only supported on Linux (for region-locking features)."
    exit 1
fi

# 2. Detect Arch
ARCH=$(uname -m)
case "$ARCH" in
    x86_64|amd64)
        SUFFIX="x86_64"
        ;;
    aarch64|arm64)
        SUFFIX="aarch64"
        ;;
    *)
        echo "${RED}Error:${NC} Unsupported architecture: $ARCH"
        exit 1
        ;;
esac

# 3. Get latest release version
echo "${BLUE}==>${NC} Checking latest version from GitHub..."
LATEST_RELEASE=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep -oP '"tag_name": "\K[^"]+')

if [ -z "$LATEST_RELEASE" ]; then
    # Fallback to scraping releases if GitHub API limits are hit
    LATEST_RELEASE=$(curl -s "https://github.com/$REPO/releases" | grep -oP 'releases/tag/\K[a-zA-Z0-9.-]+' | head -n 1)
fi

if [ -z "$LATEST_RELEASE" ]; then
    echo "${RED}Error:${NC} Could not determine the latest release version."
    exit 1
fi

echo "${BLUE}==>${NC} Found version $LATEST_RELEASE"

# 4. Download binary
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_RELEASE/dbdqueue-linux-$SUFFIX"
TEMP_FILE=$(mktemp)

echo "${BLUE}==>${NC} Downloading binary from $DOWNLOAD_URL..."
if ! curl -sSfL -o "$TEMP_FILE" "$DOWNLOAD_URL"; then
    echo "${RED}Error:${NC} Failed to download binary for $OS-$SUFFIX."
    rm -f "$TEMP_FILE"
    exit 1
fi

chmod +x "$TEMP_FILE"

# 5. Determine installation folder
INSTALL_DIR="/usr/local/bin"
USE_SUDO=""

# If /usr/local/bin is not writable by current user, we check if we have sudo or if we should use ~/.local/bin
if [ ! -w "$INSTALL_DIR" ]; then
    if [ "$(id -u)" -ne 0 ] && command -v sudo >/dev/null 2>&1; then
        USE_SUDO="sudo"
    else
        # Fallback to user-local bin
        INSTALL_DIR="$HOME/.local/bin"
        mkdir -p "$INSTALL_DIR"
    fi
fi

echo "${BLUE}==>${NC} Installing to $INSTALL_DIR/$BINARY_NAME..."
$USE_SUDO mv "$TEMP_FILE" "$INSTALL_DIR/$BINARY_NAME"

echo "${GREEN}==>${NC} dbdqueue has been installed successfully!"
echo "You can now run it by typing: ${YELLOW}dbdqueue${NC}"

# Verify if PATH contains the directory
case ":$PATH:" in
    *:"$INSTALL_DIR":*) ;;
    *)
        echo "${YELLOW}Warning:${NC} $INSTALL_DIR is not in your PATH. You may need to add it to your shell config."
        ;;
esac
