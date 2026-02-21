#!/bin/bash

# ============================================================================
# build-rpi4.sh - Build DDOS for Real Raspberry Pi 4
# ============================================================================
#
# PURPOSE:
# Build kernel for flashing onto real Raspberry Pi 4 hardware.
# Uses the RPi4 peripheral base address (0xFE000000).
#
# USAGE:
# ./scripts/build-rpi4.sh
#
# OUTPUT:
# Kernel binary at: target/aarch64-unknown-none-softfloat/debug/ddos
#
# TO FLASH TO SD CARD:
# ============================================================================
#
# 1. Find your SD card device:
#    diskutil list
#    
#    Look for a disk without "hardware" in the description. Usually /dev/diskX
#    where X is a number.
#
# 2. Unmount the SD card:
#    diskutil unmountDisk /dev/diskX
#    (Replace X with your disk number)
#
# 3. Flash the kernel:
#    sudo dd if=target/aarch64-unknown-none-softfloat/debug/ddos \
#            of=/dev/rdiskX bs=4m
#    (Note: rdisk is faster than disk; replace X with your number)
#    (Note: dd will not show progress; this is normal. Wait 1-2 minutes)
#
# 4. Eject safely:
#    diskutil ejectDisk /dev/diskX
#
# 5. Insert SD card into RPi4 and power on!
#
# ============================================================================
# WARNING: dd is powerful and dangerous!
# ============================================================================
# Wrong disk = DATA LOSS on your computer
# Verify the disk number carefully!
# Better: Use Balena Etcher (slower but safer GUI)
# ============================================================================

set -e  # Exit on any error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'  # No Color

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}DDOS Kernel - Raspberry Pi 4 Build${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo -e "${YELLOW}[1/2]${NC} Cleaning previous build..."
cd "$PROJECT_ROOT"
cargo clean
echo -e "${GREEN}✓${NC} Clean complete"
echo ""

echo -e "${YELLOW}[2/2]${NC} Building kernel for Raspberry Pi 4..."
echo -e "Using: --no-default-features --features rpi4 (peripheral base: 0xFE000000)"
echo ""
cargo build \
    --no-default-features \
    --features rpi4 \
    --target aarch64-unknown-none-softfloat
echo -e "${GREEN}✓${NC} Build complete"
echo ""

KERNEL_BINARY="target/aarch64-unknown-none-softfloat/debug/ddos"

if [ ! -f "$KERNEL_BINARY" ]; then
    echo -e "${RED}✗ FATAL: Kernel binary not found at $KERNEL_BINARY${NC}"
    exit 1
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Build Successful!${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Kernel binary at:"
echo -e "  ${GREEN}$KERNEL_BINARY${NC}"
echo ""
echo -e "${YELLOW}Next Steps: Flash to SD Card${NC}"
echo ""
echo -e "1. Find your SD card device:"
echo -e "   ${YELLOW}diskutil list${NC}"
echo ""
echo -e "2. Unmount SD card (replace 'X' with disk number):"
echo -e "   ${YELLOW}diskutil unmountDisk /dev/diskX${NC}"
echo ""
echo -e "3. Flash kernel (replace 'X' with disk number):"
echo -e "   ${YELLOW}sudo dd if=$KERNEL_BINARY of=/dev/rdiskX bs=4m${NC}"
echo ""
echo -e "4. Eject SD card:"
echo -e "   ${YELLOW}diskutil ejectDisk /dev/diskX${NC}"
echo ""
echo -e "5. Insert SD card into RPi4 and power on!"
echo ""
echo -e "${RED}⚠️  WARNING: Use 'rdisk' not 'disk' for speed!${NC}"
echo -e "${RED}⚠️  WARNING: Wrong disk = DATA LOSS on your computer!${NC}"
echo ""
