// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Kernel Spinlock Implementation

#include <jowokernel/spinlock.h>

namespace kernel {

void Spinlock::Acquire() {
    // 1. Save interrupt state and disable interrupts (CLI)
    u64 flags;
    asm volatile (
        "pushfq\n"
        "pop %0\n"
        "cli\n"
        : "=r"(flags) : : "memory"
    );
    saved_flags_ = flags;

    // 2. Spin until we acquire the lock (using atomic exchange/xchg)
    int expected = 1;
    while (true) {
        int current = 1;
        asm volatile (
            "xchg %0, %1\n"
            : "+m"(locked_), "+r"(current)
            : : "memory"
        );
        
        if (current == 0) {
            // We got the lock
            break;
        }

        // Pause to reduce bus contention (intel 'pause' instruction)
        asm volatile ("pause" : : : "memory");
    }
}

void Spinlock::Release() {
    // 1. Release the lock
    asm volatile (
        "movl $0, %0\n"
        : "=m"(locked_) : : "memory"
    );

    // 2. Restore previous interrupt state
    if (saved_flags_ & (1 << 9)) { // Bit 9 is the Interrupt Flag (IF) in RFLAGS
        asm volatile ("sti" : : : "memory");
    }
}

} // namespace kernel
