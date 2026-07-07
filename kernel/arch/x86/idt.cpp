// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Interrupt Descriptor Table (IDT) for x86_64

#include <jowokernel/types.h>

namespace arch {
namespace x86 {

struct IdtEntry {
    u16 offset_low;
    u16 selector;
    u8  ist;
    u8  type_attr;
    u16 offset_mid;
    u32 offset_high;
    u32 zero;
} __attribute__((packed));

struct IdtDescriptor {
    u16 size;
    u64 offset;
} __attribute__((packed));

#define IDT_ENTRIES 256
static IdtEntry idt[IDT_ENTRIES];
static IdtDescriptor idtr;

void SetIdtGate(int n, u64 handler) {
    idt[n].offset_low = handler & 0xFFFF;
    idt[n].selector = 0x08; // Kernel Code Segment offset in GDT
    idt[n].ist = 0;
    idt[n].type_attr = 0x8E; // Interrupt Gate
    idt[n].offset_mid = (handler >> 16) & 0xFFFF;
    idt[n].offset_high = (handler >> 32) & 0xFFFFFFFF;
    idt[n].zero = 0;
}

void InitIdt() {
    idtr.offset = reinterpret_cast<u64>(&idt[0]);
    idtr.size = sizeof(IdtEntry) * IDT_ENTRIES - 1;

    // Load IDT
    asm volatile ("lidt %0" : : "m" (idtr));
    
    // Enable interrupts
    asm volatile ("sti");
}

} // namespace x86
} // namespace arch
