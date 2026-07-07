// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel C++ API Wrappers
// Type-safe C++ wrappers around the C Kernel API

#pragma once

#include "kernel_api.h"

#include <cstdint>
#include <cstring>
#include <type_traits>

namespace kernel {

// ============================================================
// Status Type
// ============================================================

class Status {
public:
    Status() : value_(0) {}
    explicit Status(int err) : value_(err < 0 ? err : 0) {}

    bool is_ok() const { return value_ >= 0; }
    bool is_error() const { return value_ < 0; }
    int error() const { return -value_; }
    int raw_value() const { return value_; }

    const char* strerror() const {
        return ::kernel_strerror(error());
    }

    static Status Ok() { return Status(); }
    static Status Error(int err) { return Status(-err); }

private:
    int value_;
};

// ============================================================
// Handle RAII Wrapper
// ============================================================

class Handle {
public:
    Handle() : handle_(INVALID_HANDLE) {}
    explicit Handle(handle_t h) : handle_(h) {}
    ~Handle() { close(); }

    Handle(Handle&& other) noexcept : handle_(other.handle_) {
        other.handle_ = INVALID_HANDLE;
    }

    Handle& operator=(Handle&& other) noexcept {
        if (this != &other) {
            close();
            handle_ = other.handle_;
            other.handle_ = INVALID_HANDLE;
        }
        return *this;
    }

    Handle(const Handle&) = delete;
    Handle& operator=(const Handle&) = delete;

    bool is_valid() const { return handle_ != INVALID_HANDLE; }
    handle_t get() const { return handle_; }
    handle_t release() {
        handle_t h = handle_;
        handle_ = INVALID_HANDLE;
        return h;
    }

    void close() {
        if (is_valid()) {
            handle_close(handle_);
            handle_ = INVALID_HANDLE;
        }
    }

    Status duplicate(rights_t rights, Handle* out) const {
        handle_t h = handle_duplicate(handle_, rights);
        if (h < 0) {
            return Status::Error(-h);
        }
        *out = Handle(h);
        return Status::Ok();
    }

private:
    handle_t handle_;
};

// ============================================================
// Process API
// ============================================================

class Process {
public:
    static Handle Create(const char* name) {
        return Handle(process_create(name));
    }

    static Status Start(Handle& proc, uint64_t entry, uint64_t sp) {
        int result = process_start(proc.get(), entry, sp);
        if (result < 0) {
            return Status::Error(result);
        }
        return Status::Ok();
    }

    static process_id_t GetId() {
        return process_get_id();
    }

    static process_id_t GetParentId() {
        return process_get_parent_id();
    }

    static Status Kill(process_id_t pid) {
        int result = process_kill(pid);
        return Status::Error(result);
    }

    static void Exit(int code) {
        process_exit(code);
    }

    static Status Wait(process_id_t pid, int* status, uint64_t timeout = WAIT_INFINITE) {
        int result = process_wait(pid, status, 0, timeout);
        return Status::Error(result);
    }
};

// ============================================================
// Thread API
// ============================================================

class Thread {
public:
    static thread_id_t Create(const char* name) {
        return thread_create(name);
    }

    static Status Start(thread_id_t tid, uint64_t entry, uint64_t sp, uint64_t arg = 0) {
        int result = thread_start(tid, entry, sp, arg);
        return Status::Error(result);
    }

    static thread_id_t GetId() {
        return thread_get_id();
    }

    static void Yield() {
        thread_yield();
    }

    static void Exit(int code) {
        thread_exit(code);
    }

    static Status Wait(thread_id_t tid, int* status, uint64_t timeout = WAIT_INFINITE) {
        int result = thread_wait(tid, status, 0, timeout);
        return Status::Error(result);
    }
};

// ============================================================
// Memory (VMO) API
// ============================================================

class Vmo {
public:
    Vmo() : handle_(INVALID_HANDLE) {}
    explicit Vmo(handle_t h) : handle_(h) {}

    static Vmo Create(size_t size, vmo_options_t options = VMO_OPTION_NONE) {
        return Vmo(vmo_create(size, options));
    }

    static Vmo CreatePhysical(uint64_t physical_addr, size_t size) {
        return Vmo(vmo_create_physical(physical_addr, size));
    }

    bool is_valid() const { return handle_.is_valid(); }

    Status Read(uint64_t offset, void* buffer, size_t size) {
        ssize_t result = vmo_read(handle_.get(), offset, buffer, size);
        if (result < 0) {
            return Status::Error(result);
        }
        return Status::Ok();
    }

    Status Write(uint64_t offset, const void* buffer, size_t size) {
        ssize_t result = vmo_write(handle_.get(), offset, buffer, size);
        if (result < 0) {
            return Status::Error(result);
        }
        return Status::Ok();
    }

    size_t GetSize() const {
        ssize_t result = vmo_get_size(handle_.get());
        return static_cast<size_t>(result);
    }

    Status Resize(size_t new_size) {
        int result = vmo_resize(handle_.get(), new_size);
        return Status::Error(result);
    }

    handle_t get_handle() const { return handle_.get(); }

private:
    Handle handle_;
};

// ============================================================
// File Descriptor RAII Wrapper
// ============================================================

class FileDescriptor {
public:
    FileDescriptor() : fd_(INVALID_FD) {}
    explicit FileDescriptor(int fd) : fd_(fd) {}
    ~FileDescriptor() { close(); }

    FileDescriptor(FileDescriptor&& other) noexcept : fd_(other.fd_) {
        other.fd_ = INVALID_FD;
    }

    FileDescriptor& operator=(FileDescriptor&& other) noexcept {
        if (this != &other) {
            close();
            fd_ = other.fd_;
            other.fd_ = INVALID_FD;
        }
        return *this;
    }

    FileDescriptor(const FileDescriptor&) = delete;
    FileDescriptor& operator=(const FileDescriptor&) = delete;

    bool is_valid() const { return fd_ >= 0; }
    int get() const { return fd_; }
    int release() {
        int f = fd_;
        fd_ = INVALID_FD;
        return f;
    }

    void close() {
        if (is_valid()) {
            fs_close(fd_);
            fd_ = INVALID_FD;
        }
    }

private:
    int fd_;
};

// ============================================================
// Filesystem API
// ============================================================

class File {
public:
    static FileDescriptor Open(const char* path, open_flags_t flags = OPEN_FLAG_READ,
                               file_mode_t mode = FILE_MODE_DEFAULT) {
        return FileDescriptor(fs_open(path, flags, mode));
    }

    static Status Mkdir(const char* path, file_mode_t mode = FILE_MODE_DIR) {
        int result = fs_mkdir(path, mode);
        return Status::Error(result);
    }

    static Status Rmdir(const char* path) {
        int result = fs_rmdir(path);
        return Status::Error(result);
    }

    static Status Unlink(const char* path) {
        int result = fs_unlink(path);
        return Status::Error(result);
    }

    static Status Rename(const char* old_path, const char* new_path) {
        int result = fs_rename(old_path, new_path);
        return Status::Error(result);
    }

    static Status Stat(const char* path, fs_stat_t* stat) {
        int result = fs_stat(path, stat);
        return Status::Error(result);
    }

    static bool IsFile(const char* path) {
        return fs_is_file(path) != 0;
    }

    static bool IsDir(const char* path) {
        return fs_is_dir(path) != 0;
    }
};

// ============================================================
// File Read/Write Helpers
// ============================================================

inline ssize_t Read(int fd, void* buffer, size_t size) {
    return fs_read(fd, buffer, size);
}

inline ssize_t Write(int fd, const void* buffer, size_t size) {
    return fs_write(fd, buffer, size);
}

inline off_t Lseek(int fd, off_t offset, int whence) {
    return fs_lseek(fd, offset, whence);
}

// ============================================================
// Channel API
// ============================================================

class Channel {
public:
    Channel() : read_handle_(INVALID_HANDLE), write_handle_(INVALID_HANDLE) {}

    static Channel Create() {
        Channel ch;
        handle_t h0, h1;
        int result = channel_create(&h0, &h1);
        if (result == 0) {
            ch.read_handle_ = Handle(h0);
            ch.write_handle_ = Handle(h1);
        }
        return ch;
    }

    bool is_valid() const {
        return read_handle_.is_valid() && write_handle_.is_valid();
    }

    Handle& read_handle() { return read_handle_; }
    Handle& write_handle() { return write_handle_; }

    Status Write(const void* buffer, uint32_t size,
                 const handle_t* handles = nullptr, uint32_t num_handles = 0) {
        int result = channel_write(write_handle_.get(), buffer, size, handles, num_handles);
        return Status::Error(result);
    }

    Status Read(void* buffer, uint32_t size,
                handle_t* handles = nullptr, uint32_t* num_handles = nullptr) {
        int result = channel_read(read_handle_.get(), buffer, size, handles, num_handles);
        return Status::Error(result);
    }

private:
    Handle read_handle_;
    Handle write_handle_;
};

// ============================================================
// Memory Mapping Helpers
// ============================================================

inline uint64_t Mmap(void* addr, size_t size, mmap_prot_t prot, mmap_flags_t flags,
                     handle_t vmo_handle, uint64_t vmo_offset = 0) {
    return vmo_map(vmo_handle, reinterpret_cast<uint64_t>(addr), size, prot, flags, vmo_offset);
}

inline int Munmap(void* addr, size_t size) {
    return vmo_unmap(reinterpret_cast<uint64_t>(addr), size);
}

inline int Mprotect(void* addr, size_t size, mmap_prot_t prot) {
    return vmo_protect(reinterpret_cast<uint64_t>(addr), size, prot);
}

// ============================================================
// Object Wait Helpers
// ============================================================

inline signal_t WaitOne(handle_t handle, signal_t signals, uint64_t timeout = WAIT_INFINITE) {
    return object_wait_one(handle, signals, timeout);
}

}  // namespace kernel
