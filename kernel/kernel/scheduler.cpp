// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Core Microkernel Scheduler

#include <jowokernel/types.h>

namespace kernel {

// Basic Thread structure
struct Thread {
    u64 rsp;           // Stack pointer
    int id;            // Thread ID
    int priority;      // Priority level
    Thread* next;      // Next thread in run queue
};

class Scheduler {
public:
    Scheduler() : current_thread_(nullptr), ready_queue_(nullptr) {}

    void Init() {
        // Initialize the scheduler structures
    }

    void AddThread(Thread* t) {
        // Add to the end of the ready queue
        if (!ready_queue_) {
            ready_queue_ = t;
            t->next = nullptr;
        } else {
            Thread* curr = ready_queue_;
            while (curr->next) {
                curr = curr->next;
            }
            curr->next = t;
            t->next = nullptr;
        }
    }

    void Schedule() {
        // Simple Round Robin implementation
        if (!ready_queue_) return;

        Thread* old_thread = current_thread_;
        Thread* next_thread = ready_queue_;

        // Rotate queue
        ready_queue_ = ready_queue_->next;
        if (old_thread) {
            AddThread(old_thread);
        }

        current_thread_ = next_thread;

        if (old_thread != next_thread && old_thread != nullptr) {
            // arch_context_switch(old_thread, next_thread);
        }
    }

private:
    Thread* current_thread_;
    Thread* ready_queue_;
};

} // namespace kernel
