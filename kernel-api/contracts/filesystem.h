// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel Filesystem API

#pragma once

#include "../contracts/types.h"

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// File Operations
// ============================================================

// Open a file
// path: File path (null-terminated string)
// flags: Open flags (OPEN_FLAG_*)
// mode: File mode (for creation)
// Returns: File descriptor or negative error
file_descriptor_t fs_open(const char* path, open_flags_t flags, file_mode_t mode);

// Close a file descriptor
// fd: File descriptor from fs_open
// Returns: KERNEL_SUCCESS or error
int fs_close(file_descriptor_t fd);

// Read from file
// fd: File descriptor
// buffer: Destination buffer
// size: Maximum bytes to read
// Returns: bytes read, 0 on EOF, or negative error
ssize_t fs_read(file_descriptor_t fd, void* buffer, size_t size);

// Write to file
// fd: File descriptor
// buffer: Source buffer
// size: Number of bytes to write
// Returns: bytes written or negative error
ssize_t fs_write(file_descriptor_t fd, const void* buffer, size_t size);

// Seek in file
// fd: File descriptor
// offset: Offset (interpretation depends on whence)
// whence: SEEK_SET, SEEK_CUR, or SEEK_END
// Returns: new file position or negative error
off_t fs_lseek(file_descriptor_t fd, off_t offset, int whence);

#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

// Sync file to disk
// fd: File descriptor
// Returns: KERNEL_SUCCESS or error
int fs_sync(file_descriptor_t fd);

// Truncate file
// path: File path
// length: New length
// Returns: KERNEL_SUCCESS or error
int fs_truncate(const char* path, uint64_t length);

// ============================================================
// Directory Operations
// ============================================================

// Create directory
// path: Directory path
// mode: Permissions
// Returns: KERNEL_SUCCESS or error
int fs_mkdir(const char* path, file_mode_t mode);

// Remove directory
// path: Directory path (must be empty)
// Returns: KERNEL_SUCCESS or error
int fs_rmdir(const char* path);

// Remove file
// path: File path
// Returns: KERNEL_SUCCESS or error
int fs_unlink(const char* path);

// Rename/move file or directory
// old_path: Source path
// new_path: Destination path
// Returns: KERNEL_SUCCESS or error
int fs_rename(const char* old_path, const char* new_path);

// ============================================================
// File/Directory Info
// ============================================================

// File/directory stat structure
typedef struct {
    uint64_t dev;         // Device ID
    uint64_t ino;         // Inode number
    uint32_t mode;        // File type and mode
    uint32_t nlink;       // Number of hard links
    uint32_t uid;         // User ID
    uint32_t gid;         // Group ID
    uint64_t rdev;        // Device ID (if special file)
    uint64_t size;        // Total size in bytes
    uint64_t blksize;     // Block size for I/O
    uint64_t blocks;      // Number of 512B blocks allocated
    uint64_t atime;       // Access time (seconds)
    uint64_t mtime;       // Modification time (seconds)
    uint64_t ctime;       // Status change time (seconds)
} fs_stat_t;

// File type helpers
#define FS_MODE_TYPE_MASK    0xF000
#define FS_MODE_TYPE_REG     0x8000   // Regular file
#define FS_MODE_TYPE_DIR     0x4000   // Directory
#define FS_MODE_TYPE_CHR     0x2000   // Character device
#define FS_MODE_TYPE_BLK     0x6000   // Block device
#define FS_MODE_TYPE_FIFO    0x1000   // FIFO/pipe
#define FS_MODE_TYPE_LNK     0xA000   // Symbolic link
#define FS_MODE_TYPE_SOCK    0xC000   // Socket

// Get file/directory info
// path: File/directory path
// stat: Stat structure to fill
// Returns: KERNEL_SUCCESS or error
int fs_stat(const char* path, fs_stat_t* stat);

// Stat by file descriptor
// fd: File descriptor
// stat: Stat structure to fill
// Returns: KERNEL_SUCCESS or error
int fs_fstat(file_descriptor_t fd, fs_stat_t* stat);

// Check file type
// path: File path
// Returns: 1 if exists and is regular file, 0 otherwise
int fs_is_file(const char* path);

// Check directory type
// path: Directory path
// Returns: 1 if exists and is directory, 0 otherwise
int fs_is_dir(const char* path);

// ============================================================
// Path Operations
// ============================================================

// Create hard link
// old_path: Existing path
// new_path: New path
// Returns: KERNEL_SUCCESS or error
int fs_link(const char* old_path, const char* new_path);

// Create symbolic link
// old_path: Target path
// new_path: Link path
// Returns: KERNEL_SUCCESS or error
int fs_symlink(const char* old_path, const char* new_path);

// Read symbolic link
// path: Symlink path
// buffer: Buffer for target path
// size: Buffer size
// Returns: bytes in target path or negative error
ssize_t fs_readlink(const char* path, char* buffer, size_t size);

#ifdef __cplusplus
}
#endif
