// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// PCI Bus Enumerator

#pragma once

#include <jowokernel/types.h>

namespace dev {

struct PciDevice {
    u16 vendor_id;
    u16 device_id;
    u8  bus;
    u8  slot;
    u8  func;
    u8  class_code;
    u8  subclass;
};

class PciBus {
public:
    static void Init();
    static void ScanBus();

private:
    static u32 ReadConfig(u8 bus, u8 slot, u8 func, u8 offset);
    static void CheckDevice(u8 bus, u8 slot);
    static void CheckFunction(u8 bus, u8 slot, u8 func);
};

} // namespace dev
