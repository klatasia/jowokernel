// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Process Management Implementation

#include <jowokernel/process.h>
#include <jowokernel/pmm.h>

namespace kernel {

Process::Process(int id, const char* name) : id_(id), state_(ProcessState::INITIAL) {
    // Copy process name
    int i = 0;
    while (name[i] != '\0' && i < 31) {
        name_[i] = name[i];
        i++;
    }
    name_[i] = '\0';

    // Allocate a new root page table (PML4) for this process to ensure isolation.
    // Each process gets its own virtual address space mapping.
    page_table_physical_base_ = reinterpret_cast<u64>(vm::PhysicalMemoryManager::Get().AllocPage());
}

Process::~Process() {
    state_ = ProcessState::DEAD;
    
    // Free the root page table
    if (page_table_physical_base_) {
        vm::PhysicalMemoryManager::Get().FreePage(reinterpret_cast<void*>(page_table_physical_base_));
    }
}

} // namespace kernel
