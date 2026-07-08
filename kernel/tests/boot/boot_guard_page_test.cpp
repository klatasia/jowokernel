// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Regression Test for Fase C.1 (Boot Stack Guard Page via Paging)

#include "../../debug/stack_guard.h"
#include <jowokernel/types.h>

extern "C" void serial_puts(const char* s);
extern "C" u8 boot_stack_guard_page[];

namespace arch::x86 {
    extern void UnmapPageRaw(u64 pml4_phys, u64 virtual_address);
}

namespace jowo::tests {

void run_boot_guard_page_test() noexcept {
#ifdef CONFIG_KERNEL_TESTING
    serial_puts("[Regression Test] Memulai verifikasi Fase C.1 (Boot Stack Guard Page via Paging)...\n");
    
    // Set bendera tes agar handler #PF tahu ini adalah pengujian sengaja dan tidak memicu panic
    kernel::debug::g_in_boot_guard_test = true;
    kernel::debug::g_guard_test_passed = false;
    
    // Akses alamat di dalam boot_stack_guard_page (sengaja memicu Page Fault)
    volatile u32* guard_ptr = reinterpret_cast<volatile u32*>(boot_stack_guard_page + 2048);
    serial_puts("[Regression Test] Mencoba menulis ke dalam Boot Stack Guard Page...\n");
    *guard_ptr = 0xDEADBEEF; // Instruksi ini akan memicu #PF (Vector 14)
    
    // Matikan bendera tes
    kernel::debug::g_in_boot_guard_test = false;
    
    if (kernel::debug::g_guard_test_passed) {
        serial_puts("[Regression Test] PASS: Mekanisme Guard Page berfungsi sempurna & menghasilkan diagnostik lengkap!\n\n");
    } else {
        serial_puts("[Regression Test] FAIL: Akses ke Guard Page tidak memicu Page Fault!\n\n");
    }
    
    // Unmap kembali guard page untuk menjaga keamanan kernel selanjutnya
    u64 cr3;
    asm volatile("mov %%cr3, %0" : "=r"(cr3));
    arch::x86::UnmapPageRaw(cr3 & ~0xFFFULL, reinterpret_cast<u64>(boot_stack_guard_page));
#else
    serial_puts("[Regression Test] Dilewatkan (Bukan Build Testing - Guard Page Fatal Mode).\n\n");
#endif
}

} // namespace jowo::tests
