#!/bin/bash

# ============================================================================
# build-rpi5.sh - Build DDOS for Real Raspberry Pi 5
# ============================================================================
#
# PURPOSE:
# Build kernel for real Raspberry Pi 5 and generate firmware-loadable SD files.
# Produces kernel_2712.img + config.txt for the SD boot partition.
#
# USAGE:
# ./scripts/build-rpi5.sh [BOOT_MOUNT_PATH]
# Example: ./scripts/build-rpi5.sh /Volumes/bootfs
#
# OUTPUT:
# - ELF: target/aarch64-unknown-none-softfloat/debug/ddos
# - SD files: sdcard/rpi5-boot/{kernel_2712.img,config.txt}
# - Optional copy to BOOT_MOUNT_PATH
# ============================================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}DDOS Kernel - Raspberry Pi 5 Build${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BOOT_MOUNT_PATH="$1"
OUT_DIR="$PROJECT_ROOT/sdcard/rpi5-boot"

find_objcopy() {
    if command -v llvm-objcopy >/dev/null 2>&1; then
        echo "llvm-objcopy"
        return
    fi

    if command -v rust-objcopy >/dev/null 2>&1; then
        echo "rust-objcopy"
        return
    fi

    if command -v aarch64-none-elf-objcopy >/dev/null 2>&1; then
        echo "aarch64-none-elf-objcopy"
        return
    fi

    if command -v objcopy >/dev/null 2>&1; then
        echo "objcopy"
        return
    fi

    echo ""
}

echo -e "${YELLOW}[1/4]${NC} Cleaning previous build..."
cd "$PROJECT_ROOT"
cargo clean
echo -e "${GREEN}✓${NC} Clean complete"
echo ""

echo -e "${YELLOW}[2/4]${NC} Building kernel for Raspberry Pi 5..."
echo -e "Using: --no-default-features --features rpi5 (peripheral base: 0x1F000000)"
echo ""
cargo build \
    --no-default-features \
    --features rpi5 \
    --target aarch64-unknown-none-softfloat
echo -e "${GREEN}✓${NC} Build complete"
echo ""

KERNEL_BINARY="target/aarch64-unknown-none-softfloat/debug/ddos"
KERNEL_IMAGE="$OUT_DIR/kernel_2712.img"
CONFIG_TXT="$OUT_DIR/config.txt"

if [ ! -f "$KERNEL_BINARY" ]; then
    echo -e "${RED}✗ FATAL: Kernel binary not found at $KERNEL_BINARY${NC}"
    exit 1
fi

OBJCOPY_CMD="$(find_objcopy)"
if [ -z "$OBJCOPY_CMD" ]; then
    echo -e "${RED}✗ FATAL: objcopy tool not found.${NC}"
    echo -e "Install one of: llvm-objcopy, rust-objcopy, or aarch64-none-elf-objcopy"
    exit 1
fi

echo -e "${YELLOW}[3/4]${NC} Creating SD-loadable kernel image..."
mkdir -p "$OUT_DIR"
"$OBJCOPY_CMD" -O binary "$KERNEL_BINARY" "$KERNEL_IMAGE"
echo -e "${GREEN}✓${NC} Created $KERNEL_IMAGE"
echo ""

echo -e "${YELLOW}[4/4]${NC} Writing Raspberry Pi 5 boot config..."
cat > "$CONFIG_TXT" <<EOF
arm_64bit=1
kernel=kernel_2712.img
enable_uart=1
dtparam=uart0_console
dtoverlay=disable-bt
enable_rp1_uart=1
pciex4_reset=0
uart_2ndstage=0
disable_commandline_tags=1
EOF
echo -e "${GREEN}✓${NC} Created $CONFIG_TXT"
echo ""

if [ -n "$BOOT_MOUNT_PATH" ]; then
    if [ ! -d "$BOOT_MOUNT_PATH" ]; then
        echo -e "${RED}✗ FATAL: Boot mount path not found: $BOOT_MOUNT_PATH${NC}"
        exit 1
    fi

    cp "$KERNEL_IMAGE" "$BOOT_MOUNT_PATH/kernel_2712.img"
    cp "$CONFIG_TXT" "$BOOT_MOUNT_PATH/config.txt"

    echo -e "${GREEN}✓${NC} Copied kernel_2712.img and config.txt to $BOOT_MOUNT_PATH"
    echo ""
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Build Successful!${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "Kernel binary at:"
echo -e "  ${GREEN}$KERNEL_BINARY${NC}"
echo ""
echo -e "Firmware-loadable SD files at:"
echo -e "  ${GREEN}$OUT_DIR${NC}"
echo ""
echo -e "${YELLOW}Next Steps${NC}"
echo -e "- Preferred: run with boot mount path to auto-copy:"
echo -e "  ${YELLOW}./scripts/build-rpi5.sh /Volumes/bootfs${NC}"
echo -e "- Manual copy to mounted FAT boot partition:"
echo -e "  ${YELLOW}cp $KERNEL_IMAGE /Volumes/bootfs/kernel_2712.img${NC}"
echo -e "  ${YELLOW}cp $CONFIG_TXT /Volumes/bootfs/config.txt${NC}"
echo ""
echo -e "${RED}⚠️  Note: This updates files on the SD boot partition; no raw dd needed.${NC}"
echo ""