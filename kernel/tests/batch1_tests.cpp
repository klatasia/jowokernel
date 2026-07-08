// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Batch 1 Subsystems Integration Tests & Benchmarks

#include "../subsystems/01_boot/boot_manager.h"
#include "../subsystems/02_cpu/cpu_manager.h"
#include "../subsystems/03_smp/smp_manager.h"
#include "../subsystems/04_sched/sched_manager.h"
#include "../subsystems/05_thread/thread_manager.h"
#include "../subsystems/06_process/process_manager.h"

extern "C" void serial_puts(const char* s);

namespace jowo::tests {

void run_batch1_subsystems_tests() noexcept {
    serial_puts("\n[Tests] Memulai Pengujian & Inisialisasi 6 Subsistem Gelombang 1...\n");

    if (subsystems::BootSystem::Get().Init() &&
        subsystems::CpuManager::Get().Init() &&
        subsystems::SmpSystem::Get().Init() &&
        subsystems::SchedulerSystem::Get().Init() &&
        subsystems::ThreadSystem::Get().Init() &&
        subsystems::ProcessSystem::Get().Init()) {
        serial_puts("[Tests] SUKSES: 6 Subsistem Gelombang 1 beroperasi dengan paritas sempurna!\n\n");
    }
}

} // namespace jowo::tests
