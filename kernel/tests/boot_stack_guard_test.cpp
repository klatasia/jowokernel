// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Regression Test for KERNEL-001 (Boot Stack Overflow Guard)

#include "../debug/stack_guard.h"

extern "C" void serial_puts(const char* s);

namespace jowo::tests {

void run_boot_stack_guard_test() noexcept {
    serial_puts("[Regression Test] Memulai verifikasi KERNEL-001 (Boot Stack Guard)...\n");
    if (kernel::debug::stack_guard_check()) {
        serial_puts("[Regression Test] PASS: Stack Guard Canary utuh pasca stress test beban tinggi!\n\n");
    } else {
        serial_puts("[Regression Test] FAIL: Stack Guard Canary terdeteksi rusak/tertimpa!\n");
    }
}

} // namespace jowo::tests
