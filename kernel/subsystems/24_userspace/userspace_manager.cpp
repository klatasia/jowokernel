// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Userspace Manager Implementation

#include "userspace_manager.h"

extern "C" void serial_puts(const char* s);

namespace jowo::subsystems {

static UserspaceManagerSubsystem g_userspace_instance;

UserspaceManagerSubsystem& UserspaceManagerSubsystem::Get() noexcept {
    return g_userspace_instance;
}

bool UserspaceManagerSubsystem::Init() noexcept {
    status_ = SubsystemStatus::Initializing;
    serial_puts("[UserspaceManagerSubsystem] Mengekspor halaman vDSO (Virtual Dynamic Shared Object) menuju Ring 3...\n");
    status_ = SubsystemStatus::Active;
    return true;
}

void UserspaceManagerSubsystem::Shutdown() noexcept {
    status_ = SubsystemStatus::Suspended;
}

} // namespace jowo::subsystems
