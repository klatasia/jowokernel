// m33_test.cpp
// M3.3 Userspace Syscall Tests
// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT

#include <jowokernel/tests/test_runner.h>
#include "../../../subsystems/24_userspace/elf_loader.h"
#include <jowokernel/process.h>
#include <jowokernel/vfs.h>
#include <jowokernel/vmo.h>
#include <jowokernel/string.h>
#include <jowokernel/thread.h>

extern "C" void serial_puts(const char* s);
extern "C" void serial_print_hex(u64 val);
extern "C" void sched_wake(kernel::Thread* t);
extern "C" void sched_yield();

#include "test_syscall_bin.h"
#include "test_yield_bin.h"

namespace jowo::tests {
namespace userspace {

// Helper: Print separator for test output
static void print_test_separator(const char* test_name) {
    serial_puts("\n");
    serial_puts("========================================\n");
    serial_puts("[M3.3] ");
    serial_puts(test_name);
    serial_puts("\n");
    serial_puts("========================================\n");
}

// Helper: Launch a userspace ELF and report results
static bool launch_elf_test(const char* test_name,
                            const char* elf_data,
                            u64 elf_len,
                            const char* expected_output) {
    serial_puts("[M3.3] Loading: ");
    serial_puts(test_name);
    serial_puts("\n");

    // Create process
    kernel::Process* proc = new kernel::Process(
        kernel::ProcessTable::Get().AllocatePid(), test_name);

    if (!proc) {
        serial_puts("[M3.3] FAIL: Cannot create process\n");
        return false;
    }

    // Load ELF
    fs::MemoryFileVNode* vnode = new fs::MemoryFileVNode();
    vnode->LoadData(elf_data, elf_len);

    u64 out_rsp = 0;
    u64 rip = kernel::userspace::ElfLoader::Load(vnode, proc, &out_rsp);

    if (rip == 0 || out_rsp == 0) {
        serial_puts("[M3.3] FAIL: ElfLoader failed\n");
        delete vnode;
        delete proc;
        return false;
    }

    serial_puts("[M3.3] ELF loaded at virt 0x");
    serial_print_hex(rip);
    serial_puts("\n");

    // Create and wake thread
    kernel::Thread* thread = thread_create_user(proc, rip, out_rsp, 0);
    if (!thread) {
        serial_puts("[M3.3] FAIL: Cannot create thread\n");
        delete vnode;
        delete proc;
        return false;
    }

    serial_puts("[M3.3] Thread created, waking...\n");
    sched_wake(thread);

    // Yield to let the test run
    sched_yield();
    sched_yield();

    serial_puts("[M3.3] Test thread ran (check output above)\n");
    return true;
}

// ============================================================
// Test 1: Basic SYS_WRITE and SYS_EXIT
// ============================================================
void test_m33_basic_syscall() {
    print_test_separator("Basic Syscall Test (SYS_WRITE, SYS_EXIT)");

    launch_elf_test(
        "test_syscall",
        reinterpret_cast<const char*>(test_syscall_elf),
        test_syscall_elf_len,
        "[Test M3.3] Hello from userspace via Syscall!"
    );

    serial_puts("[M3.3] Expected: 'Hello from userspace via Syscall!'\n");
    serial_puts("[M3.3] Expected: Process exit with status 0\n");
}

// ============================================================
// Test 2: SYS_YIELD - Scheduler invoked from Ring 3
// ============================================================
void test_m33_yield() {
    print_test_separator("Yield Test (SYS_YIELD)");

    launch_elf_test(
        "test_yield",
        reinterpret_cast<const char*>(test_yield_elf),
        test_yield_elf_len,
        "[Test M3.3] Yield Test A"
    );

    serial_puts("[M3.3] Expected output:\n");
    serial_puts("  [Test M3.3] Yield Test A\n");
    serial_puts("  [Scheduler] Context switch\n");
    serial_puts("  [Test M3.3] Yield Test A lagi sesudah yield!\n");
}

// ============================================================
// Test 3: EFAULT - Invalid pointer handling
// ============================================================
void test_m33_fault() {
    print_test_separator("EFAULT Test (Invalid Pointer)");

    // Check if test_fault_bin.h exists
#ifdef TEST_FAULT_BIN_H
    launch_elf_test(
        "test_fault",
        reinterpret_cast<const char*>(test_fault_elf),
        test_fault_elf_len,
        "[M3.3] EFAULT returned correctly!"
    );
#else
    serial_puts("[M3.3] test_fault.elf not available\n");
    serial_puts("[M3.3] Note: copy_from_user() should return -14 (EFAULT)\n");
#endif
}

// ============================================================
// Test 4: Stress Test - Many syscalls
// ============================================================
void test_m33_stress() {
    print_test_separator("Stress Test (1000 syscalls)");

#ifdef TEST_STRESS_BIN_H
    launch_elf_test(
        "test_stress",
        reinterpret_cast<const char*>(test_stress_elf),
        test_stress_elf_len,
        "[M3.3] Starting stress test"
    );
#else
    serial_puts("[M3.3] test_stress.elf not available\n");
#endif
}

// ============================================================
// Main entry point for all M3.3 tests
// ============================================================
void run_m33_syscall_test() {
    serial_puts("\n");
    serial_puts("################################################\n");
    serial_puts("#           M3.3 USERYSPACE SYSCALL TESTS        #\n");
    serial_puts("################################################\n");

    // Run all tests
    test_m33_basic_syscall();
    test_m33_yield();
    test_m33_fault();
    test_m33_stress();

    serial_puts("\n");
    serial_puts("################################################\n");
    serial_puts("#           M3.3 TESTS COMPLETE                #\n");
    serial_puts("################################################\n");
    serial_puts("\n");
    serial_puts("[M3.3] If all tests passed:\n");
    serial_puts("  - No #PF, #GP, or panic occurred\n");
    serial_puts("  - SYS_WRITE output visible above\n");
    serial_puts("  - Process exited cleanly\n");
    serial_puts("\n");
}

} // namespace userspace
} // namespace jowo::tests
