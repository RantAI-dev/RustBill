#!/usr/bin/env bash
# RustBill Quick Installer
# Downloads and runs the Rust TUI installer binary.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/RantAI-dev/RustBill/main/scripts/install.sh | sudo bash

set -e

REPO="RantAI-dev/RustBill"
BINARY_NAME="rustbill-installer"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
DIM='\033[0;90m'
NC='\033[0m'

echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  RustBill Installer                            ║${NC}"
echo -e "${GREEN}║  Billing, Subscriptions & License Management   ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════╝${NC}"
echo ""

# Detect architecture
ARCH=$(uname -m)
case "${ARCH}" in
    x86_64)  TARGET="x86_64-linux-musl" ;;
    aarch64) TARGET="aarch64-linux-musl" ;;
    *)
        echo -e "${RED}Error: Unsupported architecture: ${ARCH}${NC}"
        echo "Supported: x86_64, aarch64"
        exit 1
        ;;
esac

INSTALLER_URL="https://github.com/${REPO}/releases/latest/download/${BINARY_NAME}-${TARGET}"

echo -e "${BLUE}Downloading installer for ${ARCH}...${NC}"
echo -e "${DIM}  ${INSTALLER_URL}${NC}"
echo ""

# Download installer
if ! curl -fsSL "${INSTALLER_URL}" -o "/tmp/${BINARY_NAME}" 2>/tmp/rustbill-dl.err; then
    echo -e "${RED}Error: Failed to download installer.${NC}"
    echo ""
    echo "This could mean:"
    echo "  1. No release has been published yet"
    echo "  2. Network connectivity issue"
    echo "  3. GitHub is temporarily unavailable"
    echo ""
    echo "To build and run the installer from source:"
    echo "  git clone https://github.com/${REPO}.git"
    echo "  cd RustBill/rustbill/installer"
    echo "  cargo build --release"
    echo "  sudo ./target/release/${BINARY_NAME} install"
    echo ""
    if [ -f /tmp/rustbill-dl.err ]; then
        echo -e "${DIM}$(cat /tmp/rustbill-dl.err)${NC}"
        rm -f /tmp/rustbill-dl.err
    fi
    exit 1
fi
rm -f /tmp/rustbill-dl.err

# Verify file is not empty
if [ ! -s "/tmp/${BINARY_NAME}" ]; then
    echo -e "${RED}Error: Downloaded file is empty.${NC}"
    rm -f "/tmp/${BINARY_NAME}"
    exit 1
fi

chmod +x "/tmp/${BINARY_NAME}"

echo -e "${GREEN}Starting installer...${NC}"
echo ""

# Run installer — use sudo if not already root
if [ "$(id -u)" -eq 0 ]; then
    exec "/tmp/${BINARY_NAME}" install "$@"
else
    exec sudo "/tmp/${BINARY_NAME}" install "$@"
fi
