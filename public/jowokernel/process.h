// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Process Management API

#pragma once

#include <jowokernel/types.h>

namespace kernel {

// Process states
enum class ProcessState {
    INITIAL,
    RUNNING,
    DEAD
};

class Process {
public:
    Process(int id, const char* name);
    ~Process();

    int GetId() const { return id_; }
    ProcessState GetState() const { return state_; }
    void SetState(ProcessState state) { state_ = state; }

    // Returns the root page table (CR3) for this process
    u64 GetPageTable() const { return page_table_physical_base_; }

private:
    int id_;
    char name_[32];
    ProcessState state_;
    
    // Physical address of the PML4 root table for this specific process
    u64 page_table_physical_base_;
};

} // namespace kernel
