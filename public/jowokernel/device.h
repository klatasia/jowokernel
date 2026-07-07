// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Device Manager API

#pragma once

#include <jowokernel/types.h>
#include <jowokernel/vfs.h>

namespace dev {

// A hardware device abstraction
class Device : public fs::VNode {
public:
    Device(const char* name);
    virtual ~Device() = default;

    const char* GetName() const { return name_; }

    // Inherited from VNode
    bool IsDirectory() const override { return false; }

private:
    char name_[32];
};

// Manages all registered devices in the system
class DeviceManager {
public:
    static void Init();
    static void RegisterDevice(Device* device);
    static Device* GetDevice(const char* name);

private:
    // Simple static array for scaffolding
    static Device* devices_[64];
    static int num_devices_;
};

} // namespace dev
