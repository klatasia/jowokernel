// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Physical Memory Manager (PMM)

#pragma once

#include <jowokernel/types.h>

namespace vm {

// Page size in x86_64 is typically 4KB
constexpr u64 PAGE_SIZE = 4096;

class PhysicalMemoryManager {
public:
    static PhysicalMemoryManager& Get() {
        static PhysicalMemoryManager instance;
        return instance;
    }

    void Init(u64 memory_size);

    // Allocates a single physical page (4KB)
    void* AllocPage();

    // Frees a previously allocated physical page
    void FreePage(void* physical_address);

private:
    PhysicalMemoryManager() = default;
    
    // Bitmap to track free/used pages. Each bit represents 1 page.
    u8* bitmap_;
    u64 total_pages_;
    u64 free_pages_;
    
    // Simple lock to protect the PMM
    // kernel::Spinlock lock_; (Would be added here)
};

} // namespace vm
