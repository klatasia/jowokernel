// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// FPU Context Switch Validation Test

#include <jowokernel/tests/test_runner.h>
#include <jowokernel/debug/assert.h>

extern "C" void serial_puts(const char* s);
extern "C" void sched_add_thread(void (*entry)());
extern "C" void sched_yield();

namespace jowo {
namespace tests {
namespace kernel_fpu {

static volatile bool thread_a_done = false;
static volatile bool thread_b_done = false;
static volatile bool test_failed = false;

static void ThreadA_Func() {
    double val_a = 1.2345;
    asm volatile("movsd %0, %%xmm0" : : "m"(val_a) : "xmm0");
    
    double st0_a = 123.45;
    asm volatile("fldl %0" : : "m"(st0_a));

    sched_yield();

    double val_check;
    asm volatile("movsd %%xmm0, %0" : "=m"(val_check));
    if (val_check != val_a) {
        serial_puts("[FPU Test] Thread A FAILED: xmm0 changed!\n");
        test_failed = true;
    }

    double st0_check;
    asm volatile("fstpl %0" : "=m"(st0_check));
    if (st0_check != st0_a) {
        serial_puts("[FPU Test] Thread A FAILED: st0 changed!\n");
        test_failed = true;
    }

    thread_a_done = true;
    
}

static void ThreadB_Func() {
    double val_b = 9.8765;
    asm volatile("movsd %0, %%xmm0" : : "m"(val_b) : "xmm0");

    double st0_b = 999.99;
    asm volatile("fldl %0" : : "m"(st0_b));

    sched_yield();

    double dump;
    asm volatile("fstpl %0" : "=m"(dump));

    thread_b_done = true;
    
}

void TestFpuContextSwitch() {
#ifdef CONFIG_KERNEL_TESTING
    serial_puts("\n--- [Test C.2.5] Context Switch Validation (FPU/SSE) ---\n");
    
    thread_a_done = false;
    thread_b_done = false;
    test_failed = false;

    sched_add_thread(ThreadA_Func);
    sched_add_thread(ThreadB_Func);

    int timeout = 0;
    while ((!thread_a_done || !thread_b_done) && timeout < 1000000) {
        sched_yield();
        timeout++;
    }

    if (test_failed || !thread_a_done || !thread_b_done) {
        serial_puts("[Test C.2.5] FAIL: FPU/SSE state corrupted across context switch!\n");
    } else {
        serial_puts("[Test C.2.5] PASS: xmm0 and ST(0) preserved across context switch!\n");
    }
#else
    serial_puts("[Test C.2.5] Dilewatkan (CONFIG_KERNEL_TESTING tidak aktif).\n");
#endif
}

} // namespace kernel_fpu
} // namespace tests
} // namespace jowo
