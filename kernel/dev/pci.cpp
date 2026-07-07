// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// PCI Bus Implementation (x86_64 Port I/O)

#include <jowokernel/pci.h>

namespace dev {

// Basic x86 Port I/O instructions
static inline void outl(u16 port, u32 value) {
    asm volatile ("outl %0, %1" : : "a"(value), "Nd"(port));
}

static inline u32 inl(u16 port) {
    u32 value;
    asm volatile ("inl %1, %0" : "=a"(value) : "Nd"(port));
    return value;
}

u32 PciBus::ReadConfig(u8 bus, u8 slot, u8 func, u8 offset) {
    u32 address = (u32)((bus << 16) | (slot << 11) | (func << 8) | (offset & 0xFC) | ((u32)0x80000000));
    // Write to CONFIG_ADDRESS
    outl(0xCF8, address);
    // Read from CONFIG_DATA
    return inl(0xCFC);
}

void PciBus::CheckFunction(u8 bus, u8 slot, u8 func) {
    u32 reg0 = ReadConfig(bus, slot, func, 0); // Contains Vendor and Device ID
    u16 vendor = (u16)(reg0 & 0xFFFF);
    
    if (vendor == 0xFFFF) return; // Device doesn't exist

    u16 device = (u16)(reg0 >> 16);
    
    u32 reg8 = ReadConfig(bus, slot, func, 8); // Contains Class and Subclass
    u8 class_code = (u8)(reg8 >> 24);
    u8 subclass   = (u8)(reg8 >> 16);

    // In a real kernel, we would add this to a device tree/list
    // printf("PCI Device Found: %04X:%04X (Class %02X)\n", vendor, device, class_code);
}

void PciBus::CheckDevice(u8 bus, u8 slot) {
    u32 reg0 = ReadConfig(bus, slot, 0, 0);
    if ((reg0 & 0xFFFF) == 0xFFFF) return; // Slot empty

    // Check function 0
    CheckFunction(bus, slot, 0);

    // Determine if multi-function device
    u32 reg12 = ReadConfig(bus, slot, 0, 12);
    u8 header_type = (u8)(reg12 >> 16);
    
    if (header_type & 0x80) {
        for (u8 func = 1; func < 8; func++) {
            u32 reg = ReadConfig(bus, slot, func, 0);
            if ((reg & 0xFFFF) != 0xFFFF) {
                CheckFunction(bus, slot, func);
            }
        }
    }
}

void PciBus::ScanBus() {
    for (u16 bus = 0; bus < 256; bus++) {
        for (u8 slot = 0; slot < 32; slot++) {
            CheckDevice((u8)bus, slot);
        }
    }
}

void PciBus::Init() {
    ScanBus();
}

} // namespace dev
