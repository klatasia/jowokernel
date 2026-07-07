// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// x86_64 Paging Implementation (VMM Level)

#include <jowokernel/types.h>
#include <jowokernel/pmm.h>

namespace arch {
namespace x86 {

// x86_64 Page Table Entry Flags
constexpr u64 PTE_PRESENT  = 1ULL << 0;
constexpr u64 PTE_WRITABLE = 1ULL << 1;
constexpr u64 PTE_USER     = 1ULL << 2;
constexpr u64 PTE_NX       = 1ULL << 63; // No-Execute

// 4-Level Paging structures
struct PageTable {
    u64 entries[512];
};

class PageDirectory {
public:
    PageDirectory() {
        // Allocate a physical page for the PML4
        pml4_ = reinterpret_cast<PageTable*>(vm::PhysicalMemoryManager::Get().AllocPage());
        for(int i=0; i<512; i++) pml4_->entries[i] = 0;
    }

    void MapMemory(u64 virtual_address, u64 physical_address, u64 flags) {
        // Highly simplified demonstration of mapping.
        // A real implementation requires traversing PML4 -> PDPT -> PD -> PT,
        // allocating new tables along the way if they don't exist.
        
        u64 pml4_idx = (virtual_address >> 39) & 0x1FF;
        
        // Let's pretend we mapped it correctly for scaffolding purposes
        // pml4_->entries[pml4_idx] = physical_address | flags | PTE_PRESENT;
    }

    void Load() {
        u64 cr3_val = reinterpret_cast<u64>(pml4_);
        asm volatile("mov %0, %%cr3" : : "r"(cr3_val) : "memory");
    }

private:
    PageTable* pml4_;
};

} // namespace x86
} // namespace arch
