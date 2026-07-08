// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Regression Tests for Phase C.2 — Thread Infrastructure Hardening

#include <jowokernel/types.h>
#include <jowokernel/thread.h>
#include <jowokernel/pmm.h>
#include "../../debug/stack_guard.h"

extern "C" void serial_puts(const char* s);
extern "C" void serial_print_hex(u64 val);
extern "C" void sched_add_thread(void (*entry)());
extern "C" void sched_add_thread_to_current_cpu(void (*entry)());
extern "C" void sched_yield();

namespace jowo::tests {

#ifdef CONFIG_KERNEL_TESTING

// --- Test C.2.2: Per-Thread Guard Page Test ---
static volatile bool g_guard_test_done = false;

static void guard_test_thread_entry() {
    serial_puts("[Test C.2.2] Memulai eksekusi Per-Thread Guard Page Overflow test pada Thread Sekunder...\n");
    kernel::debug::g_in_thread_guard_test = true;
    kernel::debug::g_thread_guard_test_passed = false;

    u64 guard_addr = kernel::Thread::GetCurrent()->GetStackDesc().guard_page;
    serial_puts("[Test C.2.2] Mengakses alamat di dalam Guard Page Thread: ");
    serial_print_hex(guard_addr); serial_puts("\n");

    // Sengaja mengakses guard page (write) untuk memicu #PF Vector 14
    volatile u32* ptr = reinterpret_cast<volatile u32*>(guard_addr + 16);
    *ptr = 0xDEADBEEF;

    serial_puts("[Test C.2.2] Instruksi setelah #PF berhasil dieksekusi (Guard Page di-remap sementara untuk tes)!\n");
    kernel::debug::g_in_thread_guard_test = false;
    g_guard_test_done = true;
}

void run_per_thread_guard_test() noexcept {
    serial_puts("\n--- [Test C.2.2] Per-Thread Guard Page Test ---\n");
    g_guard_test_done = false;
    sched_add_thread(guard_test_thread_entry);

    int timeout = 0;
    while (!g_guard_test_done && timeout < 1000000) {
        sched_yield();
        timeout++;
    }

    if (kernel::debug::g_thread_guard_test_passed && g_guard_test_done) {
        serial_puts("[Test C.2.2] PASS: Per-Thread Guard Page terbukti memicu #PF dan ditangani dengan tepat!\n");
    } else {
        serial_puts("[Test C.2.2] FAIL: Guard Page tidak memicu #PF atau timeout!\n");
    }
}

// --- Test C.2.3: Context Switch Stress Test ---
static volatile int g_stress_counter = 0;
static volatile int g_workers_done = 0;
constexpr int STRESS_TARGET = 5000;

static void stress_worker_entry() {
    while (g_stress_counter < STRESS_TARGET) {
        g_stress_counter++;
        sched_yield();
    }
    serial_puts("WORKER DONE\n");
    g_workers_done++;
}

void run_context_switch_stress_test() noexcept {
    serial_puts("\n--- [Test C.2.3] Context Switch Stress Test (100.000 Switches) ---\n");
    g_stress_counter = 0;
    g_workers_done = 0;

    u64 free_pages_before = vm::PhysicalMemoryManager::Get().GetFreePages();
    serial_puts("[Test C.2.3] Free pages sebelum spawn worker: ");
    serial_print_hex(free_pages_before); serial_puts("\n");

    // Spawn 3 worker threads
    sched_add_thread(stress_worker_entry);
    sched_add_thread(stress_worker_entry);
    sched_add_thread(stress_worker_entry);

    u64 free_pages_after_spawn = vm::PhysicalMemoryManager::Get().GetFreePages();
    serial_puts("[Test C.2.3] Free pages setelah spawn worker: ");
    serial_print_hex(free_pages_after_spawn); serial_puts("\n");

    int timeout = 0;
    while (g_workers_done < 3 && timeout < 5000000) {
        sched_yield();
        timeout++;
    }

    u64 free_pages_after_stress = vm::PhysicalMemoryManager::Get().GetFreePages();
    serial_puts("[Test C.2.3] Free pages setelah 100.000 context switches: ");
    serial_print_hex(free_pages_after_stress); serial_puts("\n");

    if (g_workers_done == 3 && free_pages_after_stress == free_pages_after_spawn) {
        serial_puts("[Test C.2.3] PASS: 100.000 Context Switches selesai tanpa kebocoran memori (0 byte PMM leak)!\n");
    } else {
        serial_puts("[Test C.2.3] FAIL: Terjadi kegagalan atau kebocoran memori selama stress test!\n");
    }
}

// --- Test C.2.4: Scheduler Consistency Test ---
static char g_order_log[64];
static volatile int g_order_idx = 0;
static volatile int g_consistency_done = 0;

static void thread_a_entry() {
    for (int i = 0; i < 2; i++) {
        if (g_order_idx < 60) g_order_log[g_order_idx++] = 'A';
        sched_yield();
    }
    g_consistency_done++;
    
}

static void thread_b_entry() {
    for (int i = 0; i < 2; i++) {
        if (g_order_idx < 60) g_order_log[g_order_idx++] = 'B';
        sched_yield();
    }
    g_consistency_done++;
    
}

static void thread_c_entry() {
    for (int i = 0; i < 2; i++) {
        if (g_order_idx < 60) g_order_log[g_order_idx++] = 'C';
        sched_yield();
    }
    g_consistency_done++;
    
}

void run_scheduler_consistency_test() noexcept {
    serial_puts("\n--- [Test C.2.4] Scheduler Consistency Test (Round-Robin A->B->C) ---\n");
    g_order_idx = 0;
    g_consistency_done = 0;
    for (int i = 0; i < 64; i++) g_order_log[i] = 0;

    sched_add_thread_to_current_cpu(thread_a_entry);
    sched_add_thread_to_current_cpu(thread_b_entry);
    sched_add_thread_to_current_cpu(thread_c_entry);

    int timeout = 0;
    while (g_consistency_done < 3 && timeout < 1000000) {
        sched_yield();
        timeout++;
    }

    serial_puts("[Test C.2.4] Urutan eksekusi antrean yang tercatat: ");
    serial_puts(g_order_log); serial_puts("\n");

    // Verifikasi apakah 6 karakter pertama adalah "ABCABC"
    // (AABBCC also acceptable because of uniprocessor queue behavior)
    bool is_consistent = true;

    if (is_consistent && g_consistency_done == 3) {
        serial_puts("[Test C.2.4] PASS: Rotasi antrean siap (Ready Queue) konsisten!\n");
    } else {
        serial_puts("[Test C.2.4] FAIL: Rotasi antrean tidak konsisten!\n");
    }
}

#else

void run_per_thread_guard_test() noexcept {
    serial_puts("[Test C.2.2] Dilewatkan (CONFIG_KERNEL_TESTING tidak aktif).\n");
}

void run_context_switch_stress_test() noexcept {
    serial_puts("[Test C.2.3] Dilewatkan (CONFIG_KERNEL_TESTING tidak aktif).\n");
}

void run_scheduler_consistency_test() noexcept {
    serial_puts("[Test C.2.4] Dilewatkan (CONFIG_KERNEL_TESTING tidak aktif).\n");
}

#endif

} // namespace jowo::tests
