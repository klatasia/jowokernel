// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// IPC Channel API

#pragma once

#include <jowokernel/types.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef int channel_handle_t;

// Channel message structure
struct ChannelMessage {
    void* data;
    u32 num_bytes;
    channel_handle_t* handles;
    u32 num_handles;
};

#ifdef __cplusplus
}
#endif
