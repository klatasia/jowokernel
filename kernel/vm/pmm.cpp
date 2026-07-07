// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Physical Memory Manager Implementation

#include <jowokernel/pmm.h>

namespace vm {

// Simplified implementation using a static array for the bitmap
// In a real system, the bitmap size depends on available RAM mapped via GRUB
static constexpr u64 MAX_PAGES = 1024 * 1024; // 4GB of RAM (1M pages)
static u8 static_bitmap[MAX_PAGES / 8];

void PhysicalMemoryManager::Init(u64 memory_size) {
    total_pages_ = memory_size / PAGE_SIZE;
    if (total_pages_ > MAX_PAGES) total_pages_ = MAX_PAGES;
    free_pages_ = total_pages_;
    bitmap_ = static_bitmap;

    // Mark all as free (0)
    for (u64 i = 0; i < (total_pages_ / 8); i++) {
        bitmap_[i] = 0;
    }
}

void* PhysicalMemoryManager::AllocPage() {
    // Basic linear search for a free page (0 bit)
    for (u64 i = 0; i < total_pages_ / 8; i++) {
        if (bitmap_[i] != 0xFF) { // Has at least one free bit
            for (int bit = 0; bit < 8; bit++) {
                if (!(bitmap_[i] & (1 << bit))) {
                    // Mark as used (1)
                    bitmap_[i] |= (1 << bit);
                    free_pages_--;
                    
                    u64 page_index = (i * 8) + bit;
                    return reinterpret_cast<void*>(page_index * PAGE_SIZE);
                }
            }
        }
    }
    return nullptr; // Out of memory
}

void PhysicalMemoryManager::FreePage(void* physical_address) {
    u64 addr = reinterpret_cast<u64>(physical_address);
    if (addr % PAGE_SIZE != 0) return; // Must be page-aligned

    u64 page_index = addr / PAGE_SIZE;
    if (page_index >= total_pages_) return;

    u64 i = page_index / 8;
    int bit = page_index % 8;

    if (bitmap_[i] & (1 << bit)) {
        // Mark as free (0)
        bitmap_[i] &= ~(1 << bit);
        free_pages_++;
    }
}

} // namespace vm
