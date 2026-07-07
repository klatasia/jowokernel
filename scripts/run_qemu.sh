#!/bin/bash
# QEMU Test Script for KLAT OS
# This script launches QEMU with the JowoKernel

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_DIR/build-x86_64"
ISO_DIR="$BUILD_DIR"
KERNEL_BIN="$BUILD_DIR/kernel.bin"
INITRAMFS="$BUILD_DIR/initramfs.cpio.gz"

# Default values
MEMORY="512M"
CPU="qemu-system-x86_64"
DISPLAY="gtk"
SERIAL="stdio"
NETWORK="-net none"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

usage() {
    echo "KLAT OS QEMU Test Script"
    echo ""
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -m, --memory SIZE    Memory size (default: 512M)"
    echo "  -k, --kernel FILE   Kernel binary (default: build-x86_64/kernel.bin)"
    echo "  -i, --initramfs FILE  Initramfs (default: build-x86_64/initramfs.cpio.gz)"
    echo "  -d, --display TYPE  Display type: gtk, vnc, none (default: gtk)"
    echo "  -s, --smp N        Number of CPUs (default: 1)"
    echo "  -h, --help         Show this help"
    echo ""
    echo "Examples:"
    echo "  $0                           # Run with defaults"
    echo "  $0 -m 1G                      # Run with 1GB memory"
    echo "  $0 --display vnc             # Run with VNC display"
    echo "  $0 -s 4                       # Run with 4 CPUs"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -m|--memory)
            MEMORY="$2"
            shift 2
            ;;
        -k|--kernel)
            KERNEL_BIN="$2"
            shift 2
            ;;
        -i|--initramfs)
            INITRAMFS="$2"
            shift 2
            ;;
        -d|--display)
            DISPLAY="$2"
            shift 2
            ;;
        -s|--smp)
            SMP="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

# Check if kernel exists
if [ ! -f "$KERNEL_BIN" ]; then
    echo -e "${RED}Error: Kernel not found at $KERNEL_BIN${NC}"
    echo "Build the kernel first with: make kernel"
    exit 1
fi

# Build QEMU command
CMD="$CPU"

# Add memory
CMD="$CMD -m $MEMORY"

# Add SMP
if [ -n "$SMP" ]; then
    CMD="$CMD -smp $SMP"
fi

# Add display
case $DISPLAY in
    gtk)
        CMD="$CMD -display gtk"
        ;;
    vnc)
        CMD="$CMD -display vnc=:1"
        ;;
    none)
        CMD="$CMD -display none"
        ;;
    *)
        CMD="$CMD -display $DISPLAY"
        ;;
esac

# Serial console
CMD="$CMD -serial $SERIAL"

# Network (disabled by default)
CMD="$CMD $NETWORK"

# Kernel and initramfs
CMD="$CMD -kernel $KERNEL_BIN"
if [ -f "$INITRAMFS" ]; then
    CMD="$CMD -initrd $INITRAMFS"
fi

# Add QEMU monitor
CMD="$CMD -monitor stdio"

# Debugging options
CMD="$CMD -d int,guest_errors"

# Enable KVM if available
if [ -w /dev/kvm ]; then
    CMD="$CMD -enable-kvm"
    echo -e "${GREEN}KVM enabled${NC}"
else
    echo -e "${YELLOW}KVM not available, using TCG${NC}"
fi

# Boot options for multiboot
CMD="$CMD -append \"console=ttyS0 root=/dev/ram0\""

echo ""
echo -e "${GREEN}====================================${NC}"
echo -e "${GREEN}   KLAT OS QEMU Test${NC}"
echo -e "${GREEN}====================================${NC}"
echo ""
echo "Configuration:"
echo "  Kernel:   $KERNEL_BIN"
echo "  Memory:   $MEMORY"
echo "  Display:  $DISPLAY"
[ -n "$SMP" ] && echo "  CPUs:     $SMP"
[ -f "$INITRAMFS" ] && echo "  Initramfs: $INITRAMFS"
echo ""
echo "Starting QEMU..."
echo "Press Ctrl+A, X to exit"
echo ""

# Run QEMU
eval $CMD
