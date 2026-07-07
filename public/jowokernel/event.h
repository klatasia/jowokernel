// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// IPC Event Object

#pragma once

#include <jowokernel/types.h>

#ifdef __cplusplus
extern "C" {
#endif

// Event Handle
typedef int event_handle_t;

// Event Signals
constexpr u32 EVENT_SIGNALED = 1 << 0;

#ifdef __cplusplus
}
#endif
