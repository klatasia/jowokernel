#!/bin/bash
# Create initramfs for KLAT OS
# This script packages userspace binaries into a cpio archive

set -e

# Directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
USERSpace_DIR="$SCRIPT_DIR"
BUILD_DIR="$USERSpace_DIR/build-x86_64"
INITRAMFS_DIR="$BUILD_DIR/initramfs"

# Clean and create
rm -rf "$INITRAMFS_DIR"
mkdir -p "$INITRAMFS_DIR/bin"
mkdir -p "$INITRAMFS_DIR/sbin"
mkdir -p "$INITRAMFS_DIR/etc"
mkdir -p "$INITRAMFS_DIR/dev"
mkdir -p "$INITRAMFS_DIR/proc"
mkdir -p "$INITRAMFS_DIR/sys"
mkdir -p "$INITRAMFS_DIR/root"
mkdir -p "$INITRAMFS_DIR/home"
mkdir -p "$INITRAMFS_DIR/tmp"
mkdir -p "$INITRAMFS_DIR/lib"
mkdir -p "$INITRAMFS_DIR/usr/bin"
mkdir -p "$INITRAMFS_DIR/usr/sbin"

echo "Creating initramfs structure..."

# Copy or create placeholder binaries if they don't exist
if [ -f "$BUILD_DIR/init/init.elf" ]; then
    cp "$BUILD_DIR/init/init.elf" "$INITRAMFS_DIR/sbin/init"
    echo "  Copied init"
else
    echo "  init not found (build with 'make' in userspace/)"
fi

if [ -f "$BUILD_DIR/shell/shell.elf" ]; then
    cp "$BUILD_DIR/shell/shell.elf" "$INITRAMFS_DIR/bin/sh"
    ln -sf /bin/sh "$INITRAMFS_DIR/bin/shell"
    echo "  Copied shell -> sh"
else
    echo "  shell not found"
fi

# Create device nodes (using mknod would require root)
# For now, we'll use a minimal /dev with console

# Create basic symlinks
ln -sf /bin/sh "$INITRAMFS_DIR/bin/sh" 2>/dev/null || true
ln -sf /proc/mounts "$INITRAMFS_DIR/etc/mtab" 2>/dev/null || true

# Create init script that will be run by the kernel
cat > "$INITRAMFS_DIR/init" << 'INITEOF'
#!/bin/sh
# Init script for KLAT OS

echo ""
echo "===================================="
echo "   KLAT OS v1.0.0"
echo "   JowoKernel - Desktop Microkernel"
echo "===================================="
echo ""

echo "[Init] Mounting filesystems..."
mount -t proc none /proc 2>/dev/null || true
mount -t sysfs none /sys 2>/dev/null || true
mount -t devpts none /dev/pts 2>/dev/null || true

echo "[Init] Starting shell..."
exec /bin/sh

# If shell exits, halt
echo "[Init] Shell exited. Halting..."
INITEOF

chmod +x "$INITRAMFS_DIR/init"

# Create cpio archive
cd "$INITRAMFS_DIR"
echo "Creating cpio archive..."
find . -print0 | cpio -ov --null > "$BUILD_DIR/initramfs.cpio" 2>/dev/null || true

# Create gzip compressed archive
if command -v gzip &> /dev/null; then
    gzip -c "$BUILD_DIR/initramfs.cpio" > "$BUILD_DIR/initramfs.cpio.gz"
    echo "Created initramfs.cpio.gz"
fi

echo ""
echo "Initramfs created successfully!"
echo "Location: $BUILD_DIR/initramfs"
echo "Archive: $BUILD_DIR/initramfs.cpio"
[ -f "$BUILD_DIR/initramfs.cpio.gz" ] && echo "Compressed: $BUILD_DIR/initramfs.cpio.gz"
