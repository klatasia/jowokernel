// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// JowoKernel Entry Point

#include <jowokernel/types.h>

extern "C" void lk_main() {
    // 1. Basic CPU/Architecture Initialization
    // arch_early_init();

    // 2. Print Boot Banner
    // We assume a basic early UART or framebuffer console is available.
    // printf("\n==============================\n");
    // printf("        JowoKernel\n");
    // printf(" Optimized Desktop Microkernel\n");
    // printf("==============================\n\n");

    // 3. VM Initialization
    // vm_init();

    // 4. Object/Handle System Initialization
    // object_init();

    // 5. Start the scheduler and first thread (system/core init)
    // sched_start();
    
    // Halt CPU if we ever reach here
    while (true) {
        asm volatile ("hlt");
    }
}
