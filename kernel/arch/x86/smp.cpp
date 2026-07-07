// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// SMP Initialization

#include <jowokernel/smp.h>

namespace arch {
namespace x86 {

int SmpManager::num_cpus_ = 1;

void SmpManager::Init() {
    // 1. Parse ACPI MADT (Multiple APIC Description Table) to find all CPU cores
    // 2. Initialize the Local APIC for the Bootstrap Processor (BSP)
    
    // For now, we assume a single CPU until ACPI parsing is fully implemented.
    num_cpus_ = 1;
}

void SmpManager::BootApplicationProcessors() {
    // Send INIT IPI (Inter-Processor Interrupt) followed by SIPI (Startup IPI)
    // to wake up secondary cores and point them to a real-mode trampoline.
}

int SmpManager::GetCurrentCpuId() {
    // Read the Local APIC ID
    u32* apic_base = reinterpret_cast<u32*>(0xFEE00000);
    return (apic_base[0x20 / 4] >> 24) & 0xFF;
}

} // namespace x86
} // namespace arch
