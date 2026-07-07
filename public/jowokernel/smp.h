// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// Symmetric Multiprocessing (SMP) Initialization

#pragma once

#include <jowokernel/types.h>

namespace arch {
namespace x86 {

class SmpManager {
public:
    static void Init();
    static void BootApplicationProcessors(); // Boot secondary cores
    static int GetCurrentCpuId();
    
private:
    static int num_cpus_;
};

} // namespace x86
} // namespace arch
