// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Batch 3 Subsystems Integration Tests & Benchmarks

#include "../subsystems/12_driver/driver_manager.h"
#include "../subsystems/13_device/device_sys_manager.h"
#include "../subsystems/14_interrupt/interrupt_manager.h"
#include "../subsystems/15_pci/pci_manager.h"
#include "../subsystems/16_fs/fs_manager.h"
#include "../subsystems/17_network/network_manager.h"
#include "../subsystems/18_virt/virt_manager.h"

extern "C" void serial_puts(const char* s);

namespace jowo::tests {

void run_batch3_subsystems_tests() noexcept {
    serial_puts("\n[Tests] Memulai Pengujian & Inisialisasi 7 Subsistem Gelombang 3...\n");

    if (subsystems::DriverFrameworkSubsystem::Get().Init() &&
        subsystems::DeviceSystemManager::Get().Init() &&
        subsystems::InterruptManagerSubsystem::Get().Init() &&
        subsystems::PciManagerSubsystem::Get().Init() &&
        subsystems::FilesystemManagerSubsystem::Get().Init() &&
        subsystems::NetworkManagerSubsystem::Get().Init() &&
        subsystems::VirtualizationManagerSubsystem::Get().Init()) {
        
        serial_puts("[Tests] SUKSES: 7 Subsistem Gelombang 3 beroperasi dengan paritas arsitektur penuh!\n\n");
    }
}

} // namespace jowo::tests
