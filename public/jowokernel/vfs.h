// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Virtual Filesystem (VFS) API

#pragma once

#include <jowokernel/types.h>

namespace fs {

// Represents a node in the VFS tree (File, Directory, or Device)
class VNode {
public:
    virtual ~VNode() = default;

    virtual i64 Read(void* buffer, u64 size, u64 offset) = 0;
    virtual i64 Write(const void* buffer, u64 size, u64 offset) = 0;
    
    virtual bool IsDirectory() const = 0;
    
    virtual VNode* Lookup(const char* name) { return nullptr; }
};

// VFS Mount Point Manager
class VirtualFileSystem {
public:
    static void Init();
    static VNode* GetRoot();
    static VNode* Open(const char* path);

private:
    static VNode* root_;
};

} // namespace fs
