// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Device Manager Implementation

#include <jowokernel/device.h>

namespace dev {

Device::Device(const char* name) {
    int i = 0;
    while (name[i] != '\0' && i < 31) {
        name_[i] = name[i];
        i++;
    }
    name_[i] = '\0';
}

Device* DeviceManager::devices_[64] = {nullptr};
int DeviceManager::num_devices_ = 0;

void DeviceManager::Init() {
    num_devices_ = 0;
}

void DeviceManager::RegisterDevice(Device* device) {
    if (num_devices_ < 64) {
        devices_[num_devices_++] = device;
    }
}

Device* DeviceManager::GetDevice(const char* name) {
    for (int i = 0; i < num_devices_; i++) {
        // Very basic string comparison
        const char* d_name = devices_[i]->GetName();
        int j = 0;
        bool match = true;
        while (name[j] != '\0' && d_name[j] != '\0') {
            if (name[j] != d_name[j]) {
                match = false;
                break;
            }
            j++;
        }
        if (match && name[j] == '\0' && d_name[j] == '\0') {
            return devices_[i];
        }
    }
    return nullptr;
}

} // namespace dev
