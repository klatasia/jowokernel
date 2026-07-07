// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Kernel System Call Handler

#include <jowokernel/syscalls.h>
#include <jowokernel/types.h>

namespace kernel {
namespace syscalls {

// This function is called by the x86_64 architecture-specific
// trap handler (e.g. triggered via the 'syscall' instruction).
extern "C" u64 handle_syscall(u64 syscall_num, u64 arg1, u64 arg2, u64 arg3) {
    switch (syscall_num) {
        case SYS_EXIT:
            // Terminate current thread
            return 0;

        case SYS_WRITE:
            // Placeholder: Write to standard output (early console)
            // Example: fd = arg1, buf = arg2, count = arg3
            return arg3; // Return bytes written

        case SYS_READ:
            // Placeholder: Read from standard input
            return 0;

        case SYS_YIELD:
            // Yield CPU to another thread
            // Scheduler::Get()->Schedule();
            return 0;

        case SYS_HANDLE_CLS:
            // Close an object handle
            return 0;

        case SYS_CHANNEL_CREATE:
            // arg1 = out_handle0, arg2 = out_handle1
            // object::ChannelDispatcher::Create(...)
            return 0;

        case SYS_CHANNEL_WRITE:
            // Handle write to IPC channel
            return 0;

        case SYS_CHANNEL_READ:
            // Handle read from IPC channel
            return 0;

        case SYS_VMO_CREATE:
            // Create a Virtual Memory Object
            // vm::VmObject(arg1, arg2)
            return 0;

        default:
            // Unknown syscall
            return (u64)-1;
    }
}

} // namespace syscalls
} // namespace kernel
