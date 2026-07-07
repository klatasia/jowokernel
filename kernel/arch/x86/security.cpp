// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Kernel Address Space Layout Randomization (KASLR) & Security Features

#include <jowokernel/types.h>

namespace arch {
namespace x86 {
namespace security {

// Hardware Random Number Generator via x86 instruction 'rdrand'
bool GetHardwareRandom(u64* out_rand) {
    u8 success;
    u64 rand_val;
    asm volatile (
        "rdrand %0\n"
        "setc %1\n"
        : "=r"(rand_val), "=qm"(success)
    );
    if (success) {
        *out_rand = rand_val;
        return true;
    }
    return false;
}

void InitKaslr() {
    u64 random_seed = 0;
    
    // Attempt to gather entropy from hardware
    if (!GetHardwareRandom(&random_seed)) {
        // Fallback to rdtsc (Time Stamp Counter) if rdrand fails
        u32 lo, hi;
        asm volatile ("rdtsc" : "=a"(lo), "=d"(hi));
        random_seed = ((u64)hi << 32) | lo;
    }

    // In a fully working KASLR implementation, the bootloader or early boot
    // routine would use this random seed to slide/offset the virtual address 
    // of the kernel code (text) and data sections in the page tables.
    
    // kernel_base_address += (random_seed % KASLR_MAX_OFFSET) & PAGE_MASK;
}

// Enable Supervisor Mode Execution Prevention (SMEP) and Access Prevention (SMAP)
void EnableSmepSmap() {
    u64 cr4;
    asm volatile ("mov %%cr4, %0" : "=r"(cr4));
    
    // Bit 20 = SMEP (Prevents kernel from executing user-space code)
    // Bit 21 = SMAP (Prevents kernel from reading/writing user-space data unintentionally)
    cr4 |= (1ULL << 20) | (1ULL << 21);
    
    asm volatile ("mov %0, %%cr4" : : "r"(cr4));
}

} // namespace security
} // namespace x86
} // namespace arch
