// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// ELF 64-bit Loader

#pragma once

#include <jowokernel/types.h>
#include <jowokernel/process.h>
#include <jowokernel/vfs.h>

namespace kernel {
namespace userspace {

class ElfLoader {
public:
    // Load an ELF executable from a VNode into the given Process
    // Returns the Entry Point Address (RIP) on success, or 0 on failure.
    // If out_rsp is provided, it will allocate a stack and populate it with Auxv.
    static u64 Load(fs::VNode* file, kernel::Process* proc, u64* out_rsp = nullptr);
};

} // namespace userspace
} // namespace kernel
