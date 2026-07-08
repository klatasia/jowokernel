// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Regression Test for KERNEL-002 (VMO Multi-Page Physical Memory Leak)

#include <jowokernel/vmo.h>
#include <jowokernel/pmm.h>
#include <jowokernel/string.h>

extern "C" void serial_puts(const char* s);
extern "C" void serial_print_hex(u64 n);

namespace jowo::tests {

void run_vmo_leak_test() noexcept {
    serial_puts("\n[Regression Test] Memulai verifikasi KERNEL-002 (VMO Multi-Page Leak)...\n");
    u64 free_before = vm::PhysicalMemoryManager::Get().GetFreePages();
    serial_puts("  PMM Free Pages Awal : 0x"); serial_print_hex(free_before); serial_puts("\n");

    serial_puts("  Stress test alokasi & destruksi 50 VMO (@ 64 KB / 16 pages)...\n");
    for (int i = 0; i < 50; i++) {
        vm::VmObject* vmo = vm::CreateVmObject(16 * 4096, 0);
        delete vmo;
    }

    u64 free_after = vm::PhysicalMemoryManager::Get().GetFreePages();
    serial_puts("  PMM Free Pages Akhir: 0x"); serial_print_hex(free_after); serial_puts("\n");

    if (free_before == free_after) {
        serial_puts("  [Regression Test] PASS: Tidak ada kebocoran halaman fisik (KERNEL-002 FIXED)!\n\n");
    } else {
        serial_puts("  [Regression Test] FAIL: Terdeteksi kebocoran memori fisik!\n\n");
    }
}

} // namespace jowo::tests
