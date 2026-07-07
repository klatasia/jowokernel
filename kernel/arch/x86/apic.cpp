// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Advanced Programmable Interrupt Controller (APIC)

#include <jowokernel/types.h>

namespace arch {
namespace x86 {

// APIC Register offsets
constexpr u32 APIC_SVR   = 0xF0; // Spurious Interrupt Vector Register
constexpr u32 APIC_TIMER = 0x320; // LVT Timer Register
constexpr u32 APIC_TICR  = 0x380; // Timer Initial Count Register

class LocalApic {
public:
    static void Init() {
        // Base address is usually 0xFEE00000
        u32* apic_base = reinterpret_cast<u32*>(0xFEE00000);

        // 1. Enable APIC (set bit 8 in Spurious Vector Register)
        // Also map spurious interrupts to vector 0xFF
        apic_base[APIC_SVR / 4] = 0x1FF;

        // 2. Configure Local Timer
        // Set to periodic mode (bit 17) and map to interrupt vector 32 (0x20)
        apic_base[APIC_TIMER / 4] = 0x20000 | 0x20;

        // 3. Set Initial Count for the timer
        apic_base[APIC_TICR / 4] = 10000000;
        
        // Timer interrupts will now fire and hit IDT entry 0x20.
        // The IDT handler for 0x20 should call Scheduler::Get().Schedule();
    }
};

} // namespace x86
} // namespace arch
