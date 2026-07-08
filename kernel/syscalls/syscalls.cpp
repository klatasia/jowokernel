// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Kernel System Call Handler

#include <jowokernel/syscalls.h>
#include <jowokernel/types.h>
#include <jowokernel/thread.h>
#include <jowokernel/process.h>

extern "C" void serial_puts(const char* s);
extern "C" void sched_yield();

namespace kernel {
namespace syscalls {

void syscall_trace_hex(u64 val) {
    char buf[20];
    int pos = 19;
    buf[pos] = '\0';
    pos--;
    
    if (val == 0) {
        buf[pos] = '0';
        pos--;
    } else {
        while (val > 0) {
            int rem = val % 16;
            if (rem < 10) buf[pos] = '0' + rem;
            else buf[pos] = 'A' + (rem - 10);
            val /= 16;
            pos--;
        }
    }
    buf[pos] = 'x';
    buf[pos-1] = '0';
    serial_puts(&buf[pos-1]);
}

// Basic copy_from_user implementation with naive boundary checking
static bool copy_from_user(void* dest, const void* user_src, u64 len) {
    if (len == 0) return true;
    u64 start = (u64)user_src;
    u64 end = start + len;
    
    // Check if the range is in the lower half (user space on x86_64 usually < 0x00007FFFFFFFFFFF)
    // We just check if it's < 0xFFFF800000000000
    if (start >= 0xFFFF800000000000ULL || end > 0xFFFF800000000000ULL || end < start) {
        return false; // EFAULT
    }
    
    // Copy safely (we are in ring 0, assuming #PF is handled correctly or SMAP is disabled)
    char* d = (char*)dest;
    const char* s = (const char*)user_src;
    for (u64 i = 0; i < len; i++) {
        d[i] = s[i];
    }
    return true;
}

// This function is called by the x86_64 architecture-specific
// trap handler via syscall_entry. RDI points to the InterruptFrame.
extern "C" u64 handle_syscall(InterruptFrame* frame) {
    u64 syscall_num = frame->rax;
    u64 arg1 = frame->rdi; // Note: Linux ABI uses rdi for arg1, rsi for arg2, etc.
    u64 arg2 = frame->rsi;
    u64 arg3 = frame->rdx;
    u64 arg4 = frame->r10;
    u64 arg5 = frame->r8;
    u64 arg6 = frame->r9;

    Thread* curr_thread = Thread::GetCurrent();
    Process* curr_proc = curr_thread ? curr_thread->GetProcess() : nullptr;
    
    if (syscall_num != SYS_WRITE && syscall_num != SYS_YIELD) {
        serial_puts("[Syscall] NUM: ");
        syscall_trace_hex(syscall_num);
        serial_puts(" ARG1: ");
        syscall_trace_hex(arg1);
        serial_puts("\n");
    }

    switch (syscall_num) {
        case SYS_EXIT:
        case 231: { // SYS_EXIT_GROUP in Linux ABI
            serial_puts("[Syscall] SYS_EXIT dipanggil dengan status: ");
            syscall_trace_hex(arg1);
            serial_puts("\n");
            
            if (curr_proc) {
                curr_proc->Exit(static_cast<int>(arg1));
            }
            
            // Loop until scheduler switches context
            while(1) {
                sched_yield();
            }
            return 0;
        }

        case SYS_WRITE: {
            const char* user_buf = reinterpret_cast<const char*>(arg2);
            u64 count = arg3;
            
            if (count > 0x10000) count = 0x10000;
            
            if (arg1 == 1 || arg1 == 2) {
                char kbuf[256];
                u64 bytes_written = 0;
                while (bytes_written < count) {
                    u64 chunk = count - bytes_written;
                    if (chunk > sizeof(kbuf) - 1) chunk = sizeof(kbuf) - 1;
                    
                    if (!copy_from_user(kbuf, user_buf + bytes_written, chunk)) {
                        return (u64)-14; // EFAULT
                    }
                    kbuf[chunk] = '\0';
                    serial_puts(kbuf);
                    bytes_written += chunk;
                }
            }
            return count;
        }

        case SYS_YIELD: {
            sched_yield();
            return 0;
        }
        
        case SYS_GETPID: {
            if (curr_proc) return curr_proc->GetId();
            return 0;
        }

        default:
            serial_puts("[Syscall] Unknown Syscall: ");
            syscall_trace_hex(syscall_num);
            serial_puts("\n");
            return (u64)-38; // ENOSYS
    }
}

} // namespace syscalls
} // namespace kernel
