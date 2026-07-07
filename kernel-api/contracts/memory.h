// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel Memory Management API

#pragma once

#include "../contracts/types.h"

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// VMO (Virtual Memory Object) Operations
// ============================================================

// VMO options
typedef uint32_t vmo_options_t;

#define VMO_OPTION_NONE       ((vmo_options_t)0)
#define VMO_OPTION_CONTIGUOUS ((vmo_options_t)(1 << 0))   // Allocate contiguous physical memory
#define VMO_OPTION_NON_RESIZABLE ((vmo_options_t)(1 << 1)) // Non-resizable
#define VMO_OPTION_NO_ATOMIC ((vmo_options_t)(1 << 2))    // Don't commit on creation

// Create a VMO
// size: Size in bytes (aligned to page boundary)
// options: VMO creation options
// Returns: VMO handle or negative error
handle_t vmo_create(size_t size, vmo_options_t options);

// Create a physical VMO (for device memory mapping)
// physical_addr: Physical address to map
// size: Size in bytes
// Returns: VMO handle or negative error
handle_t vmo_create_physical(uint64_t physical_addr, size_t size);

// Read from VMO
// handle: VMO handle
// offset: Offset within VMO
// buffer: Destination buffer
// size: Number of bytes to read
// Returns: bytes read or negative error
ssize_t vmo_read(handle_t handle, uint64_t offset, void* buffer, size_t size);

// Write to VMO
// handle: VMO handle
// offset: Offset within VMO
// buffer: Source buffer
// size: Number of bytes to write
// Returns: bytes written or negative error
ssize_t vmo_write(handle_t handle, uint64_t offset, const void* buffer, size_t size);

// Get VMO size
// Returns: size or negative error
ssize_t vmo_get_size(handle_t handle);

// Resize VMO
// Returns: KERNEL_SUCCESS or error
int vmo_resize(handle_t handle, size_t size);

// ============================================================
// Memory Mapping (mmap)
// ============================================================

// Memory mapping flags (extended)
typedef uint32_t mmap_ext_flags_t;

#define MMAP_EXT_FLAG_DEVICE    ((mmap_ext_flags_t)(1 << 16))  // Map device memory
#define MMAP_EXT_FLAG_GPU       ((mmap_ext_flags_t)(1 << 17))  // Map GPU memory
#define MMAP_EXT_FLAG_IO        ((mmap_ext_flags_t)(1 << 18))  // Map I/O port range

// Map VMO into process address space
// addr: Desired address (0 = let kernel choose)
// size: Size to map
// prot: Protection flags (MMAP_PROT_*)
// flags: Mapping flags (MMAP_FLAG_*)
// vmo_handle: Handle to VMO
// vmo_offset: Offset within VMO
// Returns: Mapped address or -1 on error
uint64_t vmo_map(handle_t vmo_handle, uint64_t addr, size_t size,
                 mmap_prot_t prot, mmap_flags_t flags, uint64_t vmo_offset);

// Unmap memory
// addr: Address from previous mmap
// size: Size to unmap
// Returns: KERNEL_SUCCESS or error
int vmo_unmap(uint64_t addr, size_t size);

// Change memory protection
// addr: Address from previous mmap
// size: Size to change
// prot: New protection flags
// Returns: KERNEL_SUCCESS or error
int vmo_protect(uint64_t addr, size_t size, mmap_prot_t prot);

// ============================================================
// Direct Physical Memory Access (kernel only)
// ============================================================

// Allocate physical pages (kernel context)
// num_pages: Number of pages to allocate
// Returns: Physical address or 0 on failure
uint64_t phys_alloc(uint64_t num_pages);

// Free physical pages (kernel context)
// phys_addr: Physical address from phys_alloc
// num_pages: Number of pages to free
void phys_free(uint64_t phys_addr, uint64_t num_pages);

#ifdef __cplusplus
}
#endif
