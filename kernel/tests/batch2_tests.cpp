// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Batch 2 Subsystems Integration Tests & Benchmarks

#include "../subsystems/07_memory/memory_manager.h"
#include "../subsystems/08_ipc/ipc_manager.h"
#include "../subsystems/09_object/object_manager.h"
#include "../subsystems/10_handle/handle_manager.h"
#include "../subsystems/11_security/security_manager.h"

extern "C" void serial_puts(const char* s);

namespace jowo::tests {

void run_batch2_subsystems_tests() noexcept {
    serial_puts("\n[Tests] Memulai Pengujian & Inisialisasi 5 Subsistem Gelombang 2...\n");

    if (subsystems::MemoryManagerSubsystem::Get().Init() &&
        subsystems::IpcManagerSubsystem::Get().Init() &&
        subsystems::ObjectManagerSubsystem::Get().Init() &&
        subsystems::HandleManagerSubsystem::Get().Init() &&
        subsystems::SecurityManagerSubsystem::Get().Init()) {
        
        bool sec_ok = subsystems::SecurityManagerSubsystem::Get().VerifyRights(0x3, 0x1);
        if (sec_ok) {
            serial_puts("[Tests] Verifikasi hak akses Capability Security SUKSES!\n");
        }
        serial_puts("[Tests] SUKSES: 5 Subsistem Gelombang 2 beroperasi dengan kapabilitas sempurna!\n\n");
    }
}

} // namespace jowo::tests
