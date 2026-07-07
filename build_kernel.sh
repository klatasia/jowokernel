#!/bin/bash
# Script Setup & Kompilasi JowoKernel di Lingkungan Ubuntu (WSL/Linux)

set -e

echo "=== [1/3] Menginstal Toolchain (Clang, LLD, Make, QEMU, NASM) ==="
# Meminta akses sudo untuk instalasi paket
sudo apt-get update
sudo apt-get install -y clang lld make qemu-system-x86 mtools xorriso nasm gcc-multilib build-essential

echo "=== [2/3] Mengkompilasi JowoKernel ==="
# Memastikan direktori build bersih
make clean || true

# Menjalankan makefile
make

echo "=== [3/3] Menjalankan Simulator QEMU ==="
if [ -f "build/jowokernel.bin" ]; then
    echo "Kompilasi sukses! Meluncurkan QEMU..."
    # Karena QEMU berjalan di dalam WSL, pastikan WSL2 GUI (WSLg) aktif,
    # atau jalankan QEMU di mode text (ncurses)
    qemu-system-x86_64 -kernel build/jowokernel.bin -m 512M -serial stdio -display none
else
    echo "GAGAL: File build/jowokernel.bin tidak ditemukan."
    exit 1
fi
