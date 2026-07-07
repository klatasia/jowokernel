// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Init: The First User-Space Process

#include <jowokernel/syscalls.h>

int main(int argc, char** argv) {
    const char* msg = "JowoKernel Init Process Started!\n";
    
    // Send message to early console via syscall
    // File descriptor 1 is conventionally stdout
    sys_write(1, msg, 33);

    // Enter a basic event loop or spawn child services
    while (true) {
        // Yield CPU to avoid locking up
        sys_yield();
    }

    return 0;
}
