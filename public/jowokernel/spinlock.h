// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Kernel Spinlock

#pragma once

#include <jowokernel/types.h>

namespace kernel {

// A basic Spinlock for kernel synchronization in an SMP environment.
// It disables local interrupts while held to prevent deadlocks with interrupt handlers.
class Spinlock {
public:
    Spinlock() : locked_(0) {}
    ~Spinlock() {}

    void Acquire();
    void Release();

private:
    // 0 = unlocked, 1 = locked
    volatile int locked_;
    u64 saved_flags_;
};

// RAII helper for Spinlock
class AutoSpinlock {
public:
    AutoSpinlock(Spinlock& lock) : lock_(lock) { lock_.Acquire(); }
    ~AutoSpinlock() { lock_.Release(); }
private:
    Spinlock& lock_;
};

} // namespace kernel
