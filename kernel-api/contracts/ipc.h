// Copyright (c) 2026 KLAT OS Team
// SPDX-License-Identifier: MIT
//
// JowoKernel Inter-Process Communication (IPC) API

#pragma once

#include "../contracts/types.h"

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// Channel Operations
// ============================================================

// Create a channel pair
// out_handle0: Output for first handle (read end)
// out_handle1: Output for second handle (write end)
// Returns: KERNEL_SUCCESS or error
int channel_create(handle_t* out_handle0, handle_t* out_handle1);

// Write to channel
// handle: Channel handle (write end)
// buffer: Data to send
// size: Size of data
// handles: Handles to transfer (can be NULL)
// num_handles: Number of handles to transfer
// Returns: KERNEL_SUCCESS or error
int channel_write(handle_t handle, const void* buffer, uint32_t size,
                  const handle_t* handles, uint32_t num_handles);

// Read from channel
// handle: Channel handle (read end)
// buffer: Buffer for received data
// size: Size of buffer (receives actual size on return)
// handles: Buffer for received handles (can be NULL)
// num_handles: Max handles / actual received on return
// Returns: KERNEL_SUCCESS or error
int channel_read(handle_t handle, void* buffer, uint32_t size,
                  handle_t* handles, uint32_t* num_handles);

// ============================================================
// Object Waiting
// ============================================================

// Wait on a single object
// handle: Handle to wait on
// signals: Signals to wait for
// timeout: Timeout in milliseconds
// Returns: observed signals or negative error
signal_t object_wait_one(handle_t handle, signal_t signals, uint64_t timeout);

// Wait on multiple objects
// handles: Array of handles
// signals: Array of signals to wait for (per handle)
// num_items: Number of handles/signals
// timeout: Timeout in milliseconds
// out_observed: Output array of observed signals
// Returns: index of signaled object, or negative error
ssize_t object_wait_many(const handle_t* handles, const signal_t* signals,
                          uint32_t num_items, uint64_t timeout,
                          signal_t* out_observed);

// ============================================================
// Event Objects
// ============================================================

// Create an event
// Returns: event handle or negative error
handle_t event_create(void);

// Signal an event
// handle: Event handle
// Returns: KERNEL_SUCCESS or error
int event_signal(handle_t handle);

// Reset an event
// handle: Event handle
// Returns: KERNEL_SUCCESS or error
int event_reset(handle_t handle);

// ============================================================
// Timer Operations
// ============================================================

// Timer types
typedef uint32_t timer_type_t;
#define TIMER_TYPE_ONESHOT     ((timer_type_t)0)
#define TIMER_TYPE_PERIODIC    ((timer_type_t)1)
#define TIMER_TYPE_ABSOLUTE    ((timer_type_t)2)

// Create a timer
// clock_id: Clock source (CLOCK_MONOTONIC, CLOCK_REALTIME, etc.)
// Returns: timer handle or negative error
handle_t timer_create(int clock_id);

// Set timer
// handle: Timer handle
// type: Timer type
// deadline: Trigger time (in nanoseconds since epoch or relative)
// slack: Scheduling slack (in nanoseconds)
// Returns: KERNEL_SUCCESS or error
int timer_set(handle_t handle, timer_type_t type, uint64_t deadline, uint64_t slack);

// Cancel timer
// handle: Timer handle
// Returns: KERNEL_SUCCESS or error
int timer_cancel(handle_t handle);

// ============================================================
// Futex Operations (fast userspace mutex)
// ============================================================

// Futex wait
// uaddr: Address of the futex word
// val: Expected value
// timeout: Timeout (can be NULL for infinite)
// Returns: KERNEL_SUCCESS or error
int futex_wait(int* uaddr, int val, const struct timespec* timeout);

// Futex wake
// uaddr: Address of the futex word
// num: Number of waiters to wake
// Returns: number woken or negative error
int futex_wake(int* uaddr, int num);

// ============================================================
// Socket Operations (basic support)
// ============================================================

// Socket types
typedef uint32_t socket_domain_t;
typedef uint32_t socket_type_t;

#define SOCKET_DOMAIN_UNIX     ((socket_domain_t)0)   // AF_UNIX
#define SOCKET_DOMAIN_INET    ((socket_domain_t)2)   // AF_INET

#define SOCKET_TYPE_STREAM     ((socket_type_t)1)    // SOCK_STREAM
#define SOCKET_TYPE_DGRAM      ((socket_type_t)2)    // SOCK_DGRAM

// Create socket
// domain: Address domain
// type: Socket type
// protocol: Protocol (0 for auto)
// Returns: socket handle or negative error
handle_t socket_create(socket_domain_t domain, socket_type_t type, int protocol);

// Bind socket
// handle: Socket handle
// addr: Address structure
// addrlen: Address length
// Returns: KERNEL_SUCCESS or error
int socket_bind(handle_t handle, const void* addr, socklen_t addrlen);

// Listen on socket
// handle: Socket handle
// backlog: Connection queue size
// Returns: KERNEL_SUCCESS or error
int socket_listen(handle_t handle, int backlog);

// Accept connection
// handle: Listening socket handle
// addr: Client address (can be NULL)
// addrlen: Address length (can be NULL)
// Returns: client socket handle or negative error
handle_t socket_accept(handle_t handle, void* addr, socklen_t* addrlen);

// Connect to socket
// handle: Socket handle
// addr: Server address
// addrlen: Address length
// Returns: KERNEL_SUCCESS or error
int socket_connect(handle_t handle, const void* addr, socklen_t addrlen);

// Send data
// handle: Socket handle
// buffer: Data to send
// size: Size of data
// flags: Send flags
// Returns: bytes sent or negative error
ssize_t socket_send(handle_t handle, const void* buffer, size_t size, int flags);

// Receive data
// handle: Socket handle
// buffer: Buffer for received data
// size: Buffer size
// flags: Receive flags
// Returns: bytes received or negative error
ssize_t socket_recv(handle_t handle, void* buffer, size_t size, int flags);

#ifdef __cplusplus
}
#endif
