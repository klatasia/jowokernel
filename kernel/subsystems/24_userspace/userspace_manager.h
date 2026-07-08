// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Userspace Manager Architecture

#pragma once

#include <jowokernel/subsystem.h>
#include <jowokernel/types.h>

namespace jowo::subsystems {

class UserspaceManagerSubsystem final : public ISubsystem {
public:
    static UserspaceManagerSubsystem& Get() noexcept;

    UserspaceManagerSubsystem() = default;

    [[nodiscard]] const char* GetName() const noexcept override { return "UserspaceManagerSubsystem"; }
    [[nodiscard]] u32 GetVersion() const noexcept override { return 1; }
    [[nodiscard]] bool Init() noexcept override;
    void Shutdown() noexcept override;
    [[nodiscard]] SubsystemStatus GetStatus() const noexcept override { return status_; }

private:
    SubsystemStatus status_{SubsystemStatus::Uninitialized};
};

} // namespace jowo::subsystems
