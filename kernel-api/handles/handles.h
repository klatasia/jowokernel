// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel Handle Management API

#pragma once

#include "../contracts/types.h"

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// Handle Info Structure
// ============================================================

typedef struct {
    handle_t handle;
    int32_t type;           // Object type
    rights_t rights;        // Rights on this handle
    uint64_t koid;          // Kernel object ID
} handle_info_t;

// Object types (matches JowoKernel internal)
typedef enum {
    OBJ_TYPE_NONE = 0,
    OBJ_TYPE_PROCESS,
    OBJ_TYPE_THREAD,
    OBJ_TYPE_VMEM,
    OBJ_TYPE_CHANNEL,
    OBJ_TYPE_EVENT,
    OBJ_TYPE_FIFO,
    OBJ_TYPE_GUEST,
    OBJ_TYPE_HANDLE_TABLE,
    OBJ_TYPE_HEAP,
    OBJ_TYPE_INTERRUPT,
    OBJ_TYPE_IO_MEMORY,
    OBJ_TYPE_IOPORT,
    OBJ_TYPE_PCI_DEVICE,
    OBJ_TYPE_PROCESSOR,
    OBJ_TYPE_PROFILE,
    OBJ_TYPE_RESOURCE,
    OBJ_TYPE_SOCKET,
    OBJ_TYPE_STREAM,
    OBJ_TYPE_THREAD_CONTROL,
    OBJ_TYPE_TIMER,
    OBJ_TYPE_VCPU,
    OBJ_TYPE_VMAR,
    OBJ_TYPE_VMO,
    OBJ_TYPE_GPU_CONTEXT,
    OBJ_TYPE_FB,
    OBJ_TYPE_DISPLAY,
    OBJ_TYPE_AUDIO,
    OBJ_TYPE_NETWORK,
    OBJ_TYPE_FS,
    OBJ_TYPE_DIR,
    OBJ_TYPE_FILE,
    OBJ_TYPE_DEVICE,
} object_type_t;

// ============================================================
// Handle Operations
// ============================================================

// Close a handle
// Returns: KERNEL_SUCCESS or error code
int handle_close(handle_t handle);

// Close multiple handles
// Returns: number of handles closed, or error code if negative
ssize_t handle_close_many(const handle_t* handles, size_t count);

// Duplicate a handle
// Returns: new handle or negative error
handle_t handle_duplicate(handle_t handle, rights_t rights);

// Replace a handle with another
// The old_handle is closed, new_handle takes its place
// Returns: KERNEL_SUCCESS or error code
int handle_replace(handle_t old_handle, handle_t new_handle);

// Get handle info
// Returns: KERNEL_SUCCESS if info filled
int handle_get_info(handle_t handle, handle_info_t* info);

// ============================================================
// Handle Validity Check
// ============================================================

// Check if a handle is valid
// Returns: 1 if valid, 0 if invalid
int handle_is_valid(handle_t handle);

// Check if current process has rights on handle
// Returns: 1 if has rights, 0 if not
int handle_has_rights(handle_t handle, rights_t rights);

#ifdef __cplusplus
}
#endif
