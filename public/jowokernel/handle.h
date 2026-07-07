// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Handle Security and Rights API

#pragma once

#include <jowokernel/types.h>

namespace object {

// Capability Rights bitmask (similar to Zircon Rights)
constexpr u32 RIGHT_DUPLICATE = 1 << 0;
constexpr u32 RIGHT_TRANSFER  = 1 << 1;
constexpr u32 RIGHT_READ      = 1 << 2;
constexpr u32 RIGHT_WRITE     = 1 << 3;
constexpr u32 RIGHT_EXECUTE   = 1 << 4;

// Base class for all Kernel Objects
class KernelObject {
public:
    virtual ~KernelObject() = default;
};

// Handle represents a user-space reference to a KernelObject
class Handle {
public:
    Handle(int id, KernelObject* obj, u32 rights) 
        : id_(id), object_(obj), rights_(rights) {}
    ~Handle() {}

    int GetId() const { return id_; }
    KernelObject* GetObject() const { return object_; }
    u32 GetRights() const { return rights_; }

    // Check if handle has specific rights
    bool HasRights(u32 required_rights) const {
        return (rights_ & required_rights) == required_rights;
    }

private:
    int id_;
    KernelObject* object_;
    u32 rights_;
};

} // namespace object
