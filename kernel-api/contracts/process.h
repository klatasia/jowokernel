// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel Process Management API

#pragma once

#include "../contracts/types.h"

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// Process Information
// ============================================================

typedef struct {
    process_id_t pid;
    process_id_t parent_pid;
    char name[64];
    uint32_t flags;
    uint64_t start_time;
    uint64_t stack_base;
    uint64_t stack_limit;
    uint64_t entry_point;
} process_info_t;

// ============================================================
// Process Creation
// ============================================================

// Create a new process
// name: Process name (up to 63 chars)
// Returns: new process handle or negative error
handle_t process_create(const char* name);

// Start a process (makes itRunnable)
// handle: Process handle from process_create
// entry: Entry point address
// sp: Initial stack pointer
// Returns: KERNEL_SUCCESS or error code
int process_start(handle_t handle, uint64_t entry, uint64_t sp);

// Exit current process
// code: Exit code
void process_exit(int code);

// ============================================================
// Process Query
// ============================================================

// Get current process ID
process_id_t process_get_id(void);

// Get parent process ID
process_id_t process_get_parent_id(void);

// Get process info
// Returns: KERNEL_SUCCESS if info filled
int process_get_info(process_id_t pid, process_info_t* info);

// Get process handle by ID
// Returns: handle or negative error
handle_t process_get_handle(process_id_t pid);

// ============================================================
// Process Control
// ============================================================

// Kill a process
// Returns: KERNEL_SUCCESS or error code
int process_kill(process_id_t pid);

// Wait for process to exit
// pid: Process to wait for
// status: Exit status output (can be NULL)
// options: Wait options
// timeout: Timeout in milliseconds (WAIT_INFINITE for no timeout)
// Returns: KERNEL_SUCCESS, KERNEL_EOBJECTWAIT on timeout, or error
int process_wait(process_id_t pid, int* status, wait_options_t options, uint64_t timeout);

// ============================================================
// Thread Management (within process)
// ============================================================

// Create a thread in current process
// name: Thread name (up to 63 chars)
// Returns: thread ID or negative error
thread_id_t thread_create(const char* name);

// Start a thread
// tid: Thread ID from thread_create
// entry: Entry point address
// sp: Initial stack pointer
// arg: Argument to pass to thread
// Returns: KERNEL_SUCCESS or error code
int thread_start(thread_id_t tid, uint64_t entry, uint64_t sp, uint64_t arg);

// Exit current thread
// code: Exit code
void thread_exit(int code);

// Get current thread ID
thread_id_t thread_get_id(void);

// Wait for thread to exit
// tid: Thread to wait for
// status: Exit status output (can be NULL)
// timeout: Timeout in milliseconds
// Returns: KERNEL_SUCCESS, KERNEL_EOBJECTWAIT on timeout, or error
int thread_wait(thread_id_t tid, int* status, wait_options_t options, uint64_t timeout);

// Yield execution to scheduler
void thread_yield(void);

#ifdef __cplusplus
}
#endif
