// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel API - Main Header
// This is the primary header userspace should include

#pragma once

// Core types and error codes
#include "contracts/types.h"
#include "errors/error_codes.h"

// Handle management
#include "handles/handles.h"

// Syscall dispatcher and wrappers
#include "dispatcher/syscall_dispatcher.h"

// High-level APIs
#include "contracts/process.h"
#include "contracts/memory.h"
#include "contracts/filesystem.h"
#include "contracts/ipc.h"

// ============================================================
// Kernel API Version
// ============================================================
#define KERNEL_API_VERSION_MAJOR 1
#define KERNEL_API_VERSION_MINOR 0
#define KERNEL_API_VERSION_PATCH 0

// ============================================================
// Convenience Macros
// ============================================================

// Check kernel API version at compile time
#define KERNEL_API_CHECK_VERSION(major, minor) \
    (KERNEL_API_VERSION_MAJOR > (major) || \
     (KERNEL_API_VERSION_MAJOR == (major) && KERNEL_API_VERSION_MINOR >= (minor)))

// ============================================================
// Status Helpers
// ============================================================

// Macros for checking syscall results
#define IS_ERROR(status)    ((status) < 0)
#define IS_OK(status)      ((status) >= 0)
#define GET_ERROR(status)   (-(status))

// Convert syscall return to status
#define SYSCALL_STATUS(ret) ((ret) >= 0 ? (ret) : STATUS_ERROR((ret)))

// ============================================================
// String Error Messages
// ============================================================

#ifdef __cplusplus
const char* kernel_strerror(int err);
#else
const char* kernel_strerror(int err);
#endif

