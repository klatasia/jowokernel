// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Virtual Filesystem (VFS) Implementation

#include <jowokernel/vfs.h>

namespace fs {

VNode* VirtualFileSystem::root_ = nullptr;

// A simple root directory node for scaffolding
class RootVNode : public VNode {
public:
    i64 Read(void* buffer, u64 size, u64 offset) override { return -1; } // Not readable
    i64 Write(const void* buffer, u64 size, u64 offset) override { return -1; } // Not writable
    bool IsDirectory() const override { return true; }
    
    VNode* Lookup(const char* name) override {
        // Here we would iterate through mounted filesystems or static nodes
        // like /dev, /system, etc.
        return nullptr;
    }
};

void VirtualFileSystem::Init() {
    root_ = new RootVNode();
}

VNode* VirtualFileSystem::GetRoot() {
    return root_;
}

VNode* VirtualFileSystem::Open(const char* path) {
    if (!root_) return nullptr;
    
    // Simplistic path traversal starting from root_
    // (Actual implementation requires tokenizing path by '/')
    
    return root_->Lookup(path);
}

} // namespace fs
