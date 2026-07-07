// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Wait Queue

#pragma once

#include <jowokernel/types.h>
#include <jowokernel/spinlock.h>

namespace kernel {

struct Thread; // Forward declaration

// A WaitQueue allows threads to block until a specific condition is met 
// (e.g., waiting for IPC message or a Mutex).
class WaitQueue {
public:
    WaitQueue();
    ~WaitQueue();

    // Block the current thread on this queue
    void Block();

    // Wake up one thread blocked on this queue
    void WakeOne();

    // Wake up all threads blocked on this queue
    void WakeAll();

private:
    Spinlock lock_;
    Thread* head_;
    Thread* tail_;
};

} // namespace kernel
