/*
 * Copyright (c) 2026 JowoKernel Team
 * SPDX-License-Identifier: MIT
 *
 * Minimal libc startup stub
 */

#include <jowokernel/syscalls.h>

// Forward declaration of the main function provided by the application
extern int main(int argc, char** argv);

extern "C" void _start() {
    // Basic setup for user-space application (e.g. stack alignment, TLS)

    char* argv[] = { nullptr };
    int argc = 0;

    // Call the application's main function
    int exit_code = main(argc, argv);

    // Terminate the process cleanly using system call
    sys_exit(exit_code);

    // Should never reach here
    while (true) {}
}

// Minimal implementation of syscall wrappers via x86_64 assembly
extern "C" void sys_yield() {
    asm volatile (
        "mov $4, %%rax\n" // SYS_YIELD = 4
        "syscall\n"
        : : : "rax", "rcx", "r11"
    );
}

extern "C" void sys_exit(int status) {
    asm volatile (
        "mov $1, %%rax\n" // SYS_EXIT = 1
        "mov %0, %%rdi\n"
        "syscall\n"
        : : "r"(status) : "rax", "rcx", "r11"
    );
}

extern "C" i64 sys_write(int fd, const void* buf, u64 count) {
    i64 ret;
    asm volatile (
        "mov $2, %%rax\n" // SYS_WRITE = 2
        "syscall\n"
        : "=a"(ret)
        : "a"(2), "D"(fd), "S"(buf), "d"(count)
        : "rcx", "r11", "memory"
    );
    return ret;
}
