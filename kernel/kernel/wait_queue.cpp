// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Wait Queue Implementation

#include <jowokernel/wait_queue.h>

namespace kernel {

WaitQueue::WaitQueue() : head_(nullptr), tail_(nullptr) {}

WaitQueue::~WaitQueue() {
    // Ideally, should panic if destroyed while threads are still waiting
}

void WaitQueue::Block() {
    AutoSpinlock guard(lock_);
    // In a full implementation, we would:
    // 1. Get the current_thread
    // 2. Add it to the linked list (head_/tail_)
    // 3. Set thread state to BLOCKED
    // 4. Yield the CPU via Scheduler::Get().Schedule()
}

void WaitQueue::WakeOne() {
    AutoSpinlock guard(lock_);
    if (!head_) return;

    // In a full implementation, we would:
    // 1. Pop the first thread from the list
    // 2. Set its state to READY
    // 3. Add it back to the Scheduler's ready queue
}

void WaitQueue::WakeAll() {
    AutoSpinlock guard(lock_);
    while (head_) {
        // Pop all threads and wake them
        head_ = nullptr; // Simplified
        tail_ = nullptr;
    }
}

} // namespace kernel
