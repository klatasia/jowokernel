// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel Syscall Dispatcher
// Routes syscalls to appropriate handlers

#pragma once

#include "../contracts/types.h"
#include "../contracts/process.h"
#include "../contracts/memory.h"
#include "../contracts/filesystem.h"
#include "../contracts/ipc.h"

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// Syscall Numbers
// ============================================================

// Linux x86_64 ABI syscalls
#define SYSCALL_READ           0
#define SYSCALL_WRITE          1
#define SYSCALL_OPEN           2
#define SYSCALL_CLOSE          3
#define SYSCALL_STAT           4
#define SYSCALL_FSTAT          5
#define SYSCALL_POLL           7
#define SYSCALL_LSEEK          8
#define SYSCALL_MMAP           9
#define SYSCALL_MUNMAP         11
#define SYSCALL_BRK            12
#define SYSCALL_RT_SIGACTION   13
#define SYSCALL_RT_SIGRETURN   15
#define SYSCALL_IOCTL          16
#define SYSCALL_YIELD          24
#define SYSCALL_DUP            32
#define SYSCALL_DUP2           33
#define SYSCALL_NANOSLEEP      35
#define SYSCALL_GETPID         39
#define SYSCALL_CLONE          56
#define SYSCALL_FORK           57
#define SYSCALL_EXECVE         59
#define SYSCALL_EXIT           60
#define SYSCALL_WAIT4          61
#define SYSCALL_KILL           62
#define SYSCALL_ARCH_PRCTL     158
#define SYSCALL_FUTEX          202
#define SYSCALL_CLOCK_GETTIME  228
#define SYSCALL_PIPE2          293

// Extended syscalls
#define SYSCALL_CHANNEL_CREATE     400
#define SYSCALL_CHANNEL_WRITE      401
#define SYSCALL_CHANNEL_READ       402
#define SYSCALL_VMO_CREATE_PHYSICAL 403
#define SYSCALL_IOPORT_WRITE16     404
#define SYSCALL_IOPORT_READ16      405
#define SYSCALL_HANDLE_CLOSE       406
#define SYSCALL_VMO_CREATE         407
#define SYSCALL_OBJECT_WAIT        408
#define SYSCALL_INTERRUPT_BIND     409
#define SYSCALL_GET_TICKS          410
#define SYSCALL_OBJECT_WAIT_MANY   411
#define SYSCALL_GET_TIME           412

// ============================================================
// Syscall Invocation (userspace stub)
// ============================================================

// Low-level syscall invocation
// This is implemented in assembly for actual system calls
// These are weak stubs that can be overridden by the actual syscall entry
extern uint64_t __syscall(uint64_t nr, uint64_t a1, uint64_t a2, uint64_t a3,
                           uint64_t a4, uint64_t a5, uint64_t a6);

// Syscall wrapper macro
#define SYSCALL_INVOKE(nr, a1, a2, a3, a4, a5, a6) \
    __syscall((uint64_t)(nr), (uint64_t)(a1), (uint64_t)(a2), (uint64_t)(a3), \
              (uint64_t)(a4), (uint64_t)(a5), (uint64_t)(a6))

// ============================================================
// High-level Syscall Wrappers
// ============================================================

// Read syscall wrapper
static inline ssize_t syscall_read(int fd, void* buf, size_t count) {
    return (ssize_t)SYSCALL_INVOKE(SYSCALL_READ, fd, (uint64_t)buf, count, 0, 0, 0);
}

// Write syscall wrapper
static inline ssize_t syscall_write(int fd, const void* buf, size_t count) {
    return (ssize_t)SYSCALL_INVOKE(SYSCALL_WRITE, fd, (uint64_t)buf, count, 0, 0, 0);
}

// Open syscall wrapper
static inline int syscall_open(const char* path, int flags, int mode) {
    return (int)SYSCALL_INVOKE(SYSCALL_OPEN, (uint64_t)path, flags, mode, 0, 0, 0);
}

// Close syscall wrapper
static inline int syscall_close(int fd) {
    return (int)SYSCALL_INVOKE(SYSCALL_CLOSE, fd, 0, 0, 0, 0, 0);
}

// Mmap syscall wrapper
static inline uint64_t syscall_mmap(uint64_t addr, size_t length, int prot, int flags, int fd, uint64_t offset) {
    return SYSCALL_INVOKE(SYSCALL_MMAP, addr, length, prot, flags, fd, offset);
}

// Exit syscall wrapper
static inline void syscall_exit(int status) {
    SYSCALL_INVOKE(SYSCALL_EXIT, status, 0, 0, 0, 0, 0);
    __builtin_unreachable();
}

// Yield syscall wrapper
static inline void syscall_yield(void) {
    SYSCALL_INVOKE(SYSCALL_YIELD, 0, 0, 0, 0, 0, 0);
}

// Getpid syscall wrapper
static inline int syscall_getpid(void) {
    return (int)SYSCALL_INVOKE(SYSCALL_GETPID, 0, 0, 0, 0, 0, 0);
}

// ============================================================
// Clock/Time
// ============================================================

#define CLOCK_REALTIME      0
#define CLOCK_MONOTONIC     1
#define CLOCK_PROCESS_CPUTIME 2
#define CLOCK_THREAD_CPUTIME 3

struct timespec {
    long tv_sec;
    long tv_nsec;
};

static inline int syscall_clock_gettime(int clock_id, struct timespec* ts) {
    return (int)SYSCALL_INVOKE(SYSCALL_CLOCK_GETTIME, clock_id, (uint64_t)ts, 0, 0, 0, 0);
}

#ifdef __cplusplus
}
#endif
