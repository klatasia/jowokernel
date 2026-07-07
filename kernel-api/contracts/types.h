// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel API Base Types
// Platform-independent types for Kernel API

#pragma once

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// Handle Types
// ============================================================

// Handle ID type (matches JowoKernel internal)
typedef int32_t handle_t;
typedef int32_t process_id_t;
typedef int32_t thread_id_t;
typedef int32_t file_descriptor_t;

// Invalid handle sentinel
#define INVALID_HANDLE ((handle_t)-1)
#define INVALID_PID    ((process_id_t)-1)
#define INVALID_TID    ((thread_id_t)-1)
#define INVALID_FD    ((file_descriptor_t)-1)

// ============================================================
// Size Types
// ============================================================
typedef uint64_t size_t;
typedef int64_t  ssize_t;

// ============================================================
// Rights Bitmasks (Zircon-style)
// ============================================================
typedef uint32_t rights_t;

// Standard rights
#define RIGHT_DUPLICATE    ((rights_t)(1 << 0))
#define RIGHT_TRANSFER     ((rights_t)(1 << 1))
#define RIGHT_READ         ((rights_t)(1 << 2))
#define RIGHT_WRITE        ((rights_t)(1 << 3))
#define RIGHT_EXECUTE      ((rights_t)(1 << 4))
#define RIGHT_MAP          ((rights_t)(1 << 5))
#define RIGHT_SIGNAL       ((rights_t)(1 << 6))
#define RIGHT_WAIT         ((rights_t)(1 << 7))
#define RIGHT_SET_PROPERTY ((rights_t)(1 << 8))
#define RIGHT_GET_PROPERTY ((rights_t)(1 << 9))

// Convenience right sets
#define RIGHT_SAME_RIGHTS  ((rights_t)(1 << 31))
#define RIGHT_ALL          ((rights_t)0xFFFFFFFF)

// ============================================================
// Object Signals
// ============================================================
typedef uint32_t signal_t;

// Standard signals
#define SIGNAL_NONE        ((signal_t)0)
#define SIGNAL_READABLE    ((signal_t)(1 << 0))
#define SIGNAL_WRITABLE    ((signal_t)(1 << 1))
#define SIGNAL_PEER_CLOSED ((signal_t)(1 << 2))
#define SIGNAL_SIGNALED    ((signal_t)(1 << 3))
#define SIGNAL_HANGUP      ((signal_t)(1 << 4))
#define SIGNAL_ERROR       ((signal_t)(1 << 5))
#define SIGNAL_OOB         ((signal_t)(1 << 6))
#define SIGNAL_POLLIN      ((signal_t)(1 << 7))
#define SIGNAL_POLLOUT     ((signal_t)(1 << 8))

// ============================================================
// Process Creation Flags
// ============================================================
typedef uint32_t proc_flags_t;

#define PROC_FLAG_NONE        ((proc_flags_t)0)
#define PROC_FLAG_FORK        ((proc_flags_t)(1 << 0))   // Fork-style clone
#define PROC_FLAG_VFORK       ((proc_flags_t)(1 << 1))   // vfork-style
#define PROC_FLAG_SIGCHILD    ((proc_flags_t)(1 << 2))   // Signal parent on exit
#define PROC_FLAG_THREAD      ((proc_flags_t)(1 << 3))   // Create thread, not process
#define PROC_FLAG_VM_CLONE    ((proc_flags_t)(1 << 4))   // Clone VM
#define PROC_FLAG_FS_CLONE    ((proc_flags_t)(1 << 5))   // Clone filesystem
#define PROC_FLAG_FILES_CLONE ((proc_flags_t)(1 << 6))   // Clone file descriptors
#define PROC_FLAG_SIGHAND     ((proc_flags_t)(1 << 7))   // Clone signal handlers

// ============================================================
// Memory Protection Flags
// ============================================================
typedef uint32_t mmap_prot_t;
typedef uint32_t mmap_flags_t;

#define MMAP_PROT_NONE        ((mmap_prot_t)0)
#define MMAP_PROT_READ        ((mmap_prot_t)(1 << 0))
#define MMAP_PROT_WRITE       ((mmap_prot_t)(1 << 1))
#define MMAP_PROT_EXEC        ((mmap_prot_t)(1 << 2))
#define MMAP_PROT_READ_WRITE  (MMAP_PROT_READ | MMAP_PROT_WRITE)
#define MMAP_PROT_READ_EXEC   (MMAP_PROT_READ | MMAP_PROT_EXEC)
#define MMAP_PROT_ALL         (MMAP_PROT_READ | MMAP_PROT_WRITE | MMAP_PROT_EXEC)

#define MMAP_FLAG_NONE        ((mmap_flags_t)0)
#define MMAP_FLAG_SHARED      ((mmap_flags_t)(1 << 0))
#define MMAP_FLAG_PRIVATE     ((mmap_flags_t)(1 << 1))
#define MMAP_FLAG_FIXED       ((mmap_flags_t)(1 << 2))
#define MMAP_FLAG_ANONYMOUS   ((mmap_flags_t)(1 << 3))
#define MMAP_FLAG_NORESERVE   ((mmap_flags_t)(1 << 4))

// ============================================================
// File Open Flags
// ============================================================
typedef uint32_t open_flags_t;

#define OPEN_FLAG_NONE        ((open_flags_t)0)
#define OPEN_FLAG_READ        ((open_flags_t)(1 << 0))
#define OPEN_FLAG_WRITE       ((open_flags_t)(1 << 1))
#define OPEN_FLAG_APPEND      ((open_flags_t)(1 << 2))
#define OPEN_FLAG_TRUNC       ((open_flags_t)(1 << 3))
#define OPEN_FLAG_CREATE      ((open_flags_t)(1 << 4))
#define OPEN_FLAG_EXCL        ((open_flags_t)(1 << 5))
#define OPEN_FLAG_DIRECTORY   ((open_flags_t)(1 << 6))
#define OPEN_FLAG_SYNC        ((open_flags_t)(1 << 7))
#define OPEN_FLAG_NOCTTY      ((open_flags_t)(1 << 8))
#define OPEN_FLAG_NONBLOCK    ((open_flags_t)(1 << 9))

// File mode (permissions)
typedef uint32_t file_mode_t;
#define FILE_MODE_DEFAULT     ((file_mode_t)0644)
#define FILE_MODE_DIR         ((file_mode_t)0755)

// ============================================================
// Wait Options
// ============================================================
typedef uint32_t wait_options_t;

#define WAIT_OPTION_NONE      ((wait_options_t)0)
#define WAIT_OPTION_INTERRUPTIBLE ((wait_options_t)(1 << 0))
#define WAIT_OPTION_NONBLOCK  ((wait_options_t)(1 << 1))

// Special timeout values
#define WAIT_INFINITE         ((uint64_t)-1)
#define WAIT_NONBLOCK         ((uint64_t)0)

// ============================================================
// Result Type (for C compatibility)
// ============================================================

// Status type - negative values are errors (negated errno)
// Zero is success
// Positive values are returns (e.g., file descriptors, pointers)
typedef int64_t status_t;

#define STATUS_OK             ((status_t)0)
#define STATUS_ERROR(err)     ((status_t)(-(err)))

// Check if status is an error
#define STATUS_IS_ERROR(s)    ((s) < 0)
#define STATUS_IS_OK(s)       ((s) >= 0)

// Get error code from status
#define STATUS_GET_ERROR(s)   ((int)(-(s)))

#ifdef __cplusplus
}
#endif
