// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Event IPC Object Implementation

#include <jowokernel/event.h>
#include <jowokernel/wait_queue.h>
#include <jowokernel/spinlock.h>

namespace object {

// Event is a simple signaling mechanism allowing threads to wait until
// a specific condition (signal) is triggered by another thread.
class Event {
public:
    Event() : signaled_(false) {}
    ~Event() {}

    // Trigger the event, waking up any waiting threads
    void Signal() {
        kernel::AutoSpinlock guard(lock_);
        if (!signaled_) {
            signaled_ = true;
            wait_queue_.WakeAll();
        }
    }

    // Reset the event back to unsignaled state
    void Reset() {
        kernel::AutoSpinlock guard(lock_);
        signaled_ = false;
    }

    // Wait for the event to be signaled
    void Wait() {
        lock_.Acquire();
        if (signaled_) {
            lock_.Release();
            return;
        }
        
        // WaitQueue::Block() typically releases the lock and puts thread to sleep
        // For simplicity, we assume WaitQueue handles the sleep state
        wait_queue_.Block();
        lock_.Release();
    }

private:
    bool signaled_;
    kernel::Spinlock lock_;
    kernel::WaitQueue wait_queue_;
};

} // namespace object
