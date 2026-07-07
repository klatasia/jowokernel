// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel Userspace Syscall Stubs
// C wrappers around the syscall instruction
//
// This file provides the implementation for all syscall wrappers declared
// in syscall_dispatcher.h

#pragma once

#include "kernel_api.h"

// ============================================================
// Syscall Implementation Stubs
// These are weak symbols that can be overridden by architecture-specific
// implementations or linked with the syscall_entry.S
// ============================================================

// Weak reference to assembly syscall entry
extern uint64_t __syscall(uint64_t nr, uint64_t a1, uint64_t a2,
                           uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6)
    __attribute__((weak));

// ============================================================
// Process Syscalls
// ============================================================

handle_t process_create(const char* name) {
    if (__syscall) {
        return (handle_t)__syscall(SYSCALL_CLONE, (uint64_t)name, 0, 0, 0, 0, 0);
    }
    return -ENOSYS;
}

int process_start(handle_t handle, uint64_t entry, uint64_t sp) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_CLONE, handle, entry, sp, 0, 0, 0);
    }
    return -ENOSYS;
}

void process_exit(int code) {
    if (__syscall) {
        __syscall(SYSCALL_EXIT, code, 0, 0, 0, 0, 0);
    }
    // If syscall doesn't return, we shouldn't be here
    while(1) { }
}

process_id_t process_get_id(void) {
    if (__syscall) {
        return (process_id_t)__syscall(SYSCALL_GETPID, 0, 0, 0, 0, 0, 0);
    }
    return 1; // Default to init
}

process_id_t process_get_parent_id(void) {
    if (__syscall) {
        return (process_id_t)__syscall(110, 0, 0, 0, 0, 0, 0); // SYS_getppid
    }
    return 0;
}

int process_kill(process_id_t pid) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_KILL, pid, 9, 0, 0, 0, 0); // SIGKILL
    }
    return -ENOSYS;
}

int process_wait(process_id_t pid, int* status, wait_options_t options, uint64_t timeout) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_WAIT4, pid, (uint64_t)status, options, timeout, 0, 0);
    }
    return -ENOSYS;
}

handle_t process_get_handle(process_id_t pid) {
    // This requires a JowoKernel-specific syscall
    if (__syscall) {
        return (handle_t)__syscall(420, pid, 0, 0, 0, 0, 0); // SYS_PROCESS_GET_HANDLE
    }
    return INVALID_HANDLE;
}

int process_get_info(process_id_t pid, process_info_t* info) {
    if (!info || pid < 0) return -EINVAL;
    // Simplified implementation
    info->pid = pid;
    info->parent_pid = process_get_parent_id();
    return 0;
}

// ============================================================
// Thread Syscalls
// ============================================================

thread_id_t thread_create(const char* name) {
    if (__syscall) {
        return (thread_id_t)__syscall(SYSCALL_CLONE,
                                       PROC_FLAG_THREAD | PROC_FLAG_SIGCHILD,
                                       0, 0, 0, 0, 0);
    }
    return -ENOSYS;
}

int thread_start(thread_id_t tid, uint64_t entry, uint64_t sp, uint64_t arg) {
    // Thread start is architecture-specific
    // In a real implementation, this would set up the thread context
    (void)tid;
    (void)entry;
    (void)sp;
    (void)arg;
    return 0;
}

void thread_exit(int code) {
    process_exit(code);
}

thread_id_t thread_get_id(void) {
    // thread ID is same as process ID in our implementation
    return (thread_id_t)process_get_id();
}

int thread_wait(thread_id_t tid, int* status, wait_options_t options, uint64_t timeout) {
    return process_wait((process_id_t)tid, status, options, timeout);
}

void thread_yield(void) {
    if (__syscall) {
        __syscall(SYSCALL_YIELD, 0, 0, 0, 0, 0, 0);
    }
}

// ============================================================
// Memory (VMO) Syscalls
// ============================================================

handle_t vmo_create(size_t size, vmo_options_t options) {
    if (__syscall) {
        return (handle_t)__syscall(SYSCALL_VMO_CREATE, size, options, 0, 0, 0, 0);
    }
    return INVALID_HANDLE;
}

handle_t vmo_create_physical(uint64_t physical_addr, size_t size) {
    if (__syscall) {
        return (handle_t)__syscall(SYSCALL_VMO_CREATE_PHYSICAL, physical_addr, size, 0, 0, 0, 0);
    }
    return INVALID_HANDLE;
}

ssize_t vmo_read(handle_t handle, uint64_t offset, void* buffer, size_t size) {
    // Read from VMO - typically done via mmap
    (void)handle;
    (void)offset;
    (void)buffer;
    (void)size;
    return -ENOSYS;
}

ssize_t vmo_write(handle_t handle, uint64_t offset, const void* buffer, size_t size) {
    // Write to VMO - typically done via mmap
    (void)handle;
    (void)offset;
    (void)buffer;
    (void)size;
    return -ENOSYS;
}

ssize_t vmo_get_size(handle_t handle) {
    // Get VMO size - requires JowoKernel-specific syscall
    (void)handle;
    return -ENOSYS;
}

int vmo_resize(handle_t handle, size_t size) {
    // Resize VMO - requires JowoKernel-specific syscall
    (void)handle;
    (void)size;
    return -ENOSYS;
}

uint64_t vmo_map(handle_t vmo_handle, uint64_t addr, size_t size,
                 mmap_prot_t prot, mmap_flags_t flags, uint64_t vmo_offset) {
    if (__syscall) {
        return __syscall(SYSCALL_MMAP, addr, size, prot, flags, vmo_handle, vmo_offset);
    }
    return (uint64_t)-ENOSYS;
}

int vmo_unmap(uint64_t addr, size_t size) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_MUNMAP, addr, size, 0, 0, 0, 0);
    }
    return -ENOSYS;
}

int vmo_protect(uint64_t addr, size_t size, mmap_prot_t prot) {
    if (__syscall) {
        return (int)__syscall(10, addr, size, prot, 0, 0, 0); // SYS_mprotect
    }
    return -ENOSYS;
}

uint64_t phys_alloc(uint64_t num_pages) {
    // Physical allocation is kernel-only
    (void)num_pages;
    return 0;
}

void phys_free(uint64_t phys_addr, uint64_t num_pages) {
    (void)phys_addr;
    (void)num_pages;
}

// ============================================================
// Filesystem Syscalls
// ============================================================

file_descriptor_t fs_open(const char* path, open_flags_t flags, file_mode_t mode) {
    if (__syscall) {
        return (file_descriptor_t)__syscall(SYSCALL_OPEN,
                                              (uint64_t)path, flags, mode, 0, 0, 0);
    }
    return INVALID_FD;
}

int fs_close(file_descriptor_t fd) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_CLOSE, fd, 0, 0, 0, 0, 0);
    }
    return -ENOSYS;
}

ssize_t fs_read(file_descriptor_t fd, void* buffer, size_t size) {
    if (__syscall) {
        return (ssize_t)__syscall(SYSCALL_READ, fd, (uint64_t)buffer, size, 0, 0, 0);
    }
    return -ENOSYS;
}

ssize_t fs_write(file_descriptor_t fd, const void* buffer, size_t size) {
    if (__syscall) {
        return (ssize_t)__syscall(SYSCALL_WRITE, fd, (uint64_t)buffer, size, 0, 0, 0);
    }
    return -ENOSYS;
}

off_t fs_lseek(file_descriptor_t fd, off_t offset, int whence) {
    if (__syscall) {
        return (off_t)__syscall(SYSCALL_LSEEK, fd, offset, whence, 0, 0, 0);
    }
    return -ENOSYS;
}

int fs_sync(file_descriptor_t fd) {
    // fsync is not directly supported
    (void)fd;
    return 0;
}

int fs_truncate(const char* path, uint64_t length) {
    // Requires ftruncate or truncate syscall
    (void)path;
    (void)length;
    return -ENOSYS;
}

int fs_mkdir(const char* path, file_mode_t mode) {
    // Requires mkdirat syscall
    (void)path;
    (void)mode;
    return -ENOSYS;
}

int fs_rmdir(const char* path) {
    // Requires unlinkat with AT_REMOVEDIR
    (void)path;
    return -ENOSYS;
}

int fs_unlink(const char* path) {
    // Requires unlinkat syscall
    (void)path;
    return -ENOSYS;
}

int fs_rename(const char* old_path, const char* new_path) {
    // Requires renameat syscall
    (void)old_path;
    (void)new_path;
    return -ENOSYS;
}

int fs_stat(const char* path, fs_stat_t* stat) {
    // Requires statx syscall
    (void)path;
    (void)stat;
    return -ENOSYS;
}

int fs_fstat(file_descriptor_t fd, fs_stat_t* stat) {
    if (__syscall && stat) {
        return (int)__syscall(SYSCALL_FSTAT, fd, (uint64_t)stat, 0, 0, 0, 0);
    }
    return -ENOSYS;
}

int fs_is_file(const char* path) {
    fs_stat_t st;
    if (fs_stat(path, &st) == 0) {
        return (st.mode & FS_MODE_TYPE_MASK) == FS_MODE_TYPE_REG;
    }
    return 0;
}

int fs_is_dir(const char* path) {
    fs_stat_t st;
    if (fs_stat(path, &st) == 0) {
        return (st.mode & FS_MODE_TYPE_MASK) == FS_MODE_TYPE_DIR;
    }
    return 0;
}

int fs_link(const char* old_path, const char* new_path) {
    (void)old_path;
    (void)new_path;
    return -ENOSYS;
}

int fs_symlink(const char* old_path, const char* new_path) {
    (void)old_path;
    (void)new_path;
    return -ENOSYS;
}

ssize_t fs_readlink(const char* path, char* buffer, size_t size) {
    (void)path;
    (void)buffer;
    (void)size;
    return -ENOSYS;
}

// ============================================================
// IPC Syscalls
// ============================================================

int channel_create(handle_t* out_handle0, handle_t* out_handle1) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_CHANNEL_CREATE,
                               (uint64_t)out_handle0, (uint64_t)out_handle1, 0, 0, 0, 0);
    }
    if (out_handle0) *out_handle0 = INVALID_HANDLE;
    if (out_handle1) *out_handle1 = INVALID_HANDLE;
    return -ENOSYS;
}

int channel_write(handle_t handle, const void* buffer, uint32_t size,
                  const handle_t* handles, uint32_t num_handles) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_CHANNEL_WRITE,
                               handle, (uint64_t)buffer, size,
                               (uint64_t)handles, num_handles, 0);
    }
    return -ENOSYS;
}

int channel_read(handle_t handle, void* buffer, uint32_t size,
                 handle_t* handles, uint32_t* num_handles) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_CHANNEL_READ,
                               handle, (uint64_t)buffer, size,
                               (uint64_t)handles, (uint64_t)num_handles, 0);
    }
    return -ENOSYS;
}

signal_t object_wait_one(handle_t handle, signal_t signals, uint64_t timeout) {
    if (__syscall) {
        return (signal_t)__syscall(SYSCALL_OBJECT_WAIT,
                                     handle, signals, timeout, 0, 0, 0);
    }
    return SIGNAL_NONE;
}

ssize_t object_wait_many(const handle_t* handles, const signal_t* signals,
                         uint32_t num_items, uint64_t timeout,
                         signal_t* out_observed) {
    if (__syscall) {
        return (ssize_t)__syscall(SYSCALL_OBJECT_WAIT_MANY,
                                    (uint64_t)handles, (uint64_t)signals,
                                    num_items, timeout,
                                    (uint64_t)out_observed, 0);
    }
    return -ENOSYS;
}

handle_t event_create(void) {
    // Event is typically created via a JowoKernel-specific syscall
    // For now, return an invalid handle
    return INVALID_HANDLE;
}

int event_signal(handle_t handle) {
    (void)handle;
    return -ENOSYS;
}

int event_reset(handle_t handle) {
    (void)handle;
    return -ENOSYS;
}

handle_t timer_create(int clock_id) {
    (void)clock_id;
    return INVALID_HANDLE;
}

int timer_set(handle_t handle, timer_type_t type, uint64_t deadline, uint64_t slack) {
    (void)handle;
    (void)type;
    (void)deadline;
    (void)slack;
    return -ENOSYS;
}

int timer_cancel(handle_t handle) {
    (void)handle;
    return -ENOSYS;
}

int futex_wait(int* uaddr, int val, const struct timespec* timeout) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_FUTEX, (uint64_t)uaddr, 0, val,
                               (uint64_t)timeout, 0, 0);
    }
    return -ENOSYS;
}

int futex_wake(int* uaddr, int num) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_FUTEX, (uint64_t)uaddr, 1, num,
                               0, 0, 0);  // FUTEX_WAKE = 1
    }
    return -ENOSYS;
}

handle_t socket_create(socket_domain_t domain, socket_type_t type, int protocol) {
    (void)domain;
    (void)type;
    (void)protocol;
    return INVALID_HANDLE;
}

int socket_bind(handle_t handle, const void* addr, socklen_t addrlen) {
    (void)handle;
    (void)addr;
    (void)addrlen;
    return -ENOSYS;
}

int socket_listen(handle_t handle, int backlog) {
    (void)handle;
    (void)backlog;
    return -ENOSYS;
}

handle_t socket_accept(handle_t handle, void* addr, socklen_t* addrlen) {
    (void)handle;
    (void)addr;
    (void)addrlen;
    return INVALID_HANDLE;
}

int socket_connect(handle_t handle, const void* addr, socklen_t addrlen) {
    (void)handle;
    (void)addr;
    (void)addrlen;
    return -ENOSYS;
}

ssize_t socket_send(handle_t handle, const void* buffer, size_t size, int flags) {
    (void)handle;
    (void)buffer;
    (void)size;
    (void)flags;
    return -ENOSYS;
}

ssize_t socket_recv(handle_t handle, void* buffer, size_t size, int flags) {
    (void)handle;
    (void)buffer;
    (void)size;
    (void)flags;
    return -ENOSYS;
}

// ============================================================
// Handle Syscalls
// ============================================================

int handle_close(handle_t handle) {
    if (__syscall) {
        return (int)__syscall(SYSCALL_HANDLE_CLOSE, handle, 0, 0, 0, 0, 0);
    }
    return -ENOSYS;
}

ssize_t handle_close_many(const handle_t* handles, size_t count) {
    if (!handles || count == 0) return 0;
    ssize_t closed = 0;
    for (size_t i = 0; i < count; i++) {
        if (handle_close(handles[i]) == 0) {
            closed++;
        }
    }
    return closed;
}

handle_t handle_duplicate(handle_t handle, rights_t rights) {
    // handle_duplicate requires JowoKernel-specific syscall
    (void)handle;
    (void)rights;
    return INVALID_HANDLE;
}

int handle_replace(handle_t old_handle, handle_t new_handle) {
    (void)old_handle;
    (void)new_handle;
    return -ENOSYS;
}

int handle_get_info(handle_t handle, handle_info_t* info) {
    if (!info) return -EINVAL;
    info->handle = handle;
    info->type = OBJ_TYPE_NONE;
    info->rights = 0;
    info->koid = 0;
    return 0;
}

int handle_is_valid(handle_t handle) {
    return handle >= 0;
}

int handle_has_rights(handle_t handle, rights_t rights) {
    (void)handle;
    (void)rights;
    return 0;
}
