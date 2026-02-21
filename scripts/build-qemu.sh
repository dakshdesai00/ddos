#!/bin/bash

# ============================================================================
# build-qemu.sh - Build and Run DDOS in QEMU
# ============================================================================
#
# PURPOSE:
# Complete build pipeline for QEMU development:
# 1. Clean previous build artifacts
# 2. Build kernel for QEMU (RPi3 model with 0x3F000000 peripheral base)
# 3. Launch in QEMU emulator
#
# USAGE:
# ./scripts/build-qemu.sh
#
# REQUIREMENTS:
# - cargo (Rust toolchain)
# - qemu-system-aarch64
# - Raspberry Pi 3 QEMU image support
#
# EXIT CONTROLS:
# - To exit QEMU: Press Ctrl+A then X
#

set -e  # Exit on any error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'  # No Color

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}DDOS Kernel - QEMU Build & Run${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${YELLOW}[1/3]${NC} Cleaning previous build..."
cd "$PROJECT_ROOT"
cargo clean
echo -e "${GREEN}✓${NC} Clean complete"
echo ""

echo -e "${YELLOW}[2/3]${NC} Building kernel for QEMU (RPi3 model)..."
cargo build \
    --features qemu \
    --target aarch64-unknown-none-softfloat
echo -e "${GREEN}✓${NC} Build complete"
echo ""

KERNEL_BINARY="target/aarch64-unknown-none-softfloat/debug/ddos"

if [ ! -f "$KERNEL_BINARY" ]; then
    echo -e "${RED}✗ FATAL: Kernel binary not found at $KERNEL_BINARY${NC}"
    exit 1
fi

echo -e "${YELLOW}[3/3]${NC} Launching QEMU..."
echo -e "${YELLOW}========================================${NC}"
echo -e "To exit QEMU: Press ${RED}Ctrl+A${NC} then ${RED}X${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""

# Launch QEMU with:
# -M raspi3b: Emulate Raspberry Pi 3B
# -serial stdio: Redirect serial output/input to console
# -kernel: Path to kernel ELF binary
qemu-system-aarch64 \
    -M raspi3b \
    -serial stdio \
    -kernel "$KERNEL_BINARY"
