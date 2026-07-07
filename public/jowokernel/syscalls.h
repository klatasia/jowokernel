// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT
//
// JowoKernel System Calls API

#pragma once

#include <jowokernel/types.h>

#ifdef __cplusplus
extern "C" {
#endif

// Syscall Numbers
#define SYS_EXIT       1
#define SYS_WRITE      2
#define SYS_READ       3
#define SYS_YIELD      4
#define SYS_HANDLE_CLS 5
#define SYS_CHANNEL_CREATE 6
#define SYS_CHANNEL_WRITE  7
#define SYS_CHANNEL_READ   8
#define SYS_VMO_CREATE     9

// Syscall stubs (to be linked by user-space ulib)
void sys_exit(int status);
i64 sys_write(int fd, const void* buf, u64 count);
i64 sys_read(int fd, void* buf, u64 count);
void sys_yield();
int sys_handle_close(int handle);
int sys_channel_create(int* out_handle0, int* out_handle1);
int sys_channel_write(int handle, const void* data, u32 num_bytes, const int* handles, u32 num_handles);
int sys_channel_read(int handle, void* data, u32 num_bytes, int* handles, u32 num_handles);
int sys_vmo_create(u64 size, u32 options, int* out_handle);

#ifdef __cplusplus
}
#endif
