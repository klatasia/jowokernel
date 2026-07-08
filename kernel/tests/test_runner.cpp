// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Unified Kernel Runtime Self-Test Runner Implementation

#include <jowokernel/tests/test_runner.h>
#include <jowokernel/tests/graphics_validation.h>
#include <jowokernel/debug/memory_audit.h>

extern "C" void serial_puts(const char* s);

namespace jowo::tests {

void run_batch1_subsystems_tests() noexcept;
void run_batch2_subsystems_tests() noexcept;
void run_batch3_subsystems_tests() noexcept;
void run_batch4_subsystems_tests() noexcept;
void run_boot_stack_guard_test() noexcept;
void run_boot_guard_page_test() noexcept;
void run_vmo_leak_test() noexcept;
void run_per_thread_guard_test() noexcept;
void run_context_switch_stress_test() noexcept;
void run_scheduler_consistency_test() noexcept;

namespace kernel_fpu {
    void TestFpuContextSwitch();
}

void run_boot_tests() noexcept {
    serial_puts("\n=== [Self-Test] Boot & Subsystems Validation ===\n");
    run_batch1_subsystems_tests();
    run_batch2_subsystems_tests();
    run_batch3_subsystems_tests();
    run_batch4_subsystems_tests();
    run_boot_stack_guard_test();
    run_boot_guard_page_test();
}

void run_memory_tests() noexcept {
    serial_puts("\n=== [Self-Test] Memory Architecture Audit (Fase B) ===\n");
#ifdef CONFIG_DEBUG_MEMORY_AUDIT
    kernel::debug::run_memory_audit();
#else
    serial_puts("[Memory Audit] Dilewatkan (Bukan Build Debug).\n");
#endif
    run_vmo_leak_test();
}

void run_scheduler_tests() noexcept {
    serial_puts("\n=== [Self-Test] Thread Infrastructure Hardening (Fase C.2) ===\n");
    run_per_thread_guard_test();
    run_context_switch_stress_test();
    run_scheduler_consistency_test();
    kernel_fpu::TestFpuContextSwitch();
}

void run_graphics_tests() noexcept {
    serial_puts("\n=== [Self-Test] Graphics Validation ===\n");
    graphics::RunAllGraphicsTests();
}

namespace userspace {
    extern void run_elf_loader_test();
    extern void run_m32_ring3_test();
    extern void run_m33_syscall_test();
}

void run_userspace_tests() noexcept {
    serial_puts("\n=== [Self-Test] Userspace Foundation (M3) ===\n");
    userspace::run_elf_loader_test();
    userspace::run_m32_ring3_test();
    userspace::run_m33_syscall_test();
}

void run_kernel_self_tests() noexcept {
    serial_puts("\n==================================================\n");
    serial_puts("       MEMULAI JOWOKERNEL RUNTIME SELF-TESTS      \n");
    serial_puts("==================================================\n");

    run_boot_tests();
    run_memory_tests();
    
    serial_puts("\n[Tests] Boot & Memory Tests Selesai. Menunggu Scheduler...\n");
}

void run_post_scheduler_tests() noexcept {
    serial_puts("\n==================================================\n");
    serial_puts("      MEMULAI POST-SCHEDULER RUNTIME TESTS        \n");
    serial_puts("==================================================\n");

    run_scheduler_tests();
    run_graphics_tests();
    run_userspace_tests();

    serial_puts("\n==================================================\n");
    serial_puts("    SELURUH KERNEL RUNTIME SELF-TESTS SUKSES!     \n");
    serial_puts("==================================================\n\n");
}

} // namespace jowo::tests
