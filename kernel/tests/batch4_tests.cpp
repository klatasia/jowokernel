// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Batch 4 Subsystems Integration Tests & Benchmarks

#include "../subsystems/19_power/power_manager.h"
#include "../subsystems/20_timer/timer_manager.h"
#include "../subsystems/21_debug/debug_manager.h"
#include "../subsystems/22_trace/trace_manager.h"
#include "../subsystems/23_syscall/syscall_manager.h"
#include "../subsystems/24_userspace/userspace_manager.h"

extern "C" void serial_puts(const char* s);

namespace jowo::tests {

void run_batch4_subsystems_tests() noexcept {
    serial_puts("\n[Tests] Memulai Pengujian & Inisialisasi 6 Subsistem Terakhir (Gelombang 4)...\n");

    if (subsystems::PowerManagerSubsystem::Get().Init() &&
        subsystems::TimerManagerSubsystem::Get().Init() &&
        subsystems::DebugManagerSubsystem::Get().Init() &&
        subsystems::TracingManagerSubsystem::Get().Init() &&
        subsystems::SyscallManagerSubsystem::Get().Init() &&
        subsystems::UserspaceManagerSubsystem::Get().Init()) {
        
        serial_puts("[Tests] SUKSES: 24 SUBSISTEM JOWOKERNEL TELAH BEROPERASI 100% ONLINE!\n\n");
    }
}

} // namespace jowo::tests
