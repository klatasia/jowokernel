// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Global Descriptor Table (GDT) for x86_64

#include <jowokernel/types.h>

namespace arch {
namespace x86 {

struct GdtEntry {
    u16 limit_low;
    u16 base_low;
    u8  base_middle;
    u8  access;
    u8  granularity;
    u8  base_high;
} __attribute__((packed));

struct GdtDescriptor {
    u16 size;
    u64 offset;
} __attribute__((packed));

// Basic GDT with Null, Kernel Code, and Kernel Data segments
static GdtEntry gdt[3];
static GdtDescriptor gdtr;

void InitGdt() {
    // Null segment
    gdt[0] = {0, 0, 0, 0, 0, 0};

    // Kernel Code Segment (Ring 0, Executable, Read)
    gdt[1] = {0xFFFF, 0, 0, 0x9A, 0xAF, 0};

    // Kernel Data Segment (Ring 0, Read/Write)
    gdt[2] = {0xFFFF, 0, 0, 0x92, 0xAF, 0};

    gdtr.size = sizeof(gdt) - 1;
    gdtr.offset = reinterpret_cast<u64>(&gdt[0]);

    // Load GDT
    asm volatile ("lgdt %0" : : "m" (gdtr));
}

} // namespace x86
} // namespace arch
