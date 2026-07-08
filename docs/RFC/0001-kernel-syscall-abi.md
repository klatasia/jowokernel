# RFC-0001: Kernel Syscall ABI

```
RFC: 0001
Title: Kernel Syscall ABI Freeze
Status: Accepted
Author: JowoKernel Team
Date: 2026-07-08
Supersedes: N/A
```

---

## Summary

This RFC establishes the frozen Application Binary Interface (ABI) for JowoKernel syscall interface, ensuring compatibility for userspace programs and runtime libraries.

---

## Motivation

As KLAT OS progresses from kernel development to userspace and GUI development, a stable syscall interface is critical. Once applications are built against the syscall interface, changes to syscall numbers or semantics would break all existing binaries.

This RFC formally freezes the current syscall interface to enable:

1. Development of userspace runtime libraries
2. libc implementation
3. SDK development
4. Application development

---

## Detailed Design

### Syscall Calling Convention

Following Linux x86_64 syscall ABI:

```
┌─────────────────────────────────────────────────────────────┐
│                  SYSCALL REGISTER USAGE                     │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Syscall Number:  rax                                        │
│  Argument 1:     rdi                                        │
│  Argument 2:     rsi                                        │
│  Argument 3:     rdx                                        │
│  Argument 4:     r10  (NOT rcx!)                           │
│  Argument 5:     r8                                         │
│  Argument 6:     r9                                         │
│                                                              │
│  Return Value:   rax (negative = error)                     │
│                                                              │
│  Clobbered:      rcx, r11 (syscall instruction clobbers)   │
│  Preserved:      rbx, rbp, r12-r15                        │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Error Handling

Syscalls return negative error codes in rax:

```c
#define SYSCALL_ERR(errno) (-(errno))

// Example:
// return SYSCALL_ERR(EFAULT)  // returns -14 in rax
```

### Error Codes

| Name | Value | Description |
|------|-------|-------------|
| EPERM | 1 | Operation not permitted |
| ENOENT | 2 | No such file or directory |
| EINTR | 4 | Interrupted system call |
| EIO | 5 | I/O error |
| EAGAIN | 11 | Try again |
| ENOMEM | 12 | Out of memory |
| EFAULT | 14 | Bad address |
| EINVAL | 22 | Invalid argument |
| ENOSYS | 38 | Invalid syscall |

---

## Implemented Syscalls

### SYS_WRITE (1)

```c
ssize_t sys_write(int fd, const void* buf, size_t count);
```

**Arguments:**
- `fd`: File descriptor (1=stdout, 2=stderr)
- `buf`: Userspace buffer pointer
- `count`: Number of bytes to write

**Returns:**
- Number of bytes written on success
- `-EFAULT` if buffer is invalid
- `-EBADF` if fd is invalid

**Validation:**
- Buffer must be in userspace (< 0xFFFF800000000000)
- Max chunk size: 64KB

### SYS_YIELD (24)

```c
int sys_yield(void);
```

**Arguments:** None

**Returns:** 0 (success)

**Behavior:** Yields CPU to scheduler, may switch to another thread.

### SYS_EXIT (60)

```c
void sys_exit(int status);
```

**Arguments:**
- `status`: Exit status code

**Returns:** Does not return

**Behavior:**
- Marks process as ZOMBIE
- Wakes parent if waiting (waitpid)
- Terminates all threads in process
- Closes all file descriptors

### SYS_GETPID (172)

```c
pid_t sys_getpid(void);
```

**Arguments:** None

**Returns:** Current process ID

---

## Future Syscalls (Reserved)

The following syscall numbers are reserved but not yet implemented:

| Number | Name | Status |
|--------|------|--------|
| 0 | sys_read | Reserved |
| 2 | sys_open | Planned |
| 3 | sys_close | Planned |
| 9 | sys_mmap | Planned |
| 12 | sys_brk | Planned |
| 57 | sys_fork | Planned |
| 59 | sys_execve | Planned |
| 61 | sys_wait4 | Planned |

---

## Validation Requirements

All syscalls must validate:

1. **Pointer Validation (copy_from_user)**
   - All userspace pointers must be validated
   - Reject pointers >= 0xFFFF800000000000 (kernel space)
   - Return -EFAULT on invalid pointer

2. **Buffer Bounds**
   - Validate buffer doesn't wrap around
   - Validate buffer doesn't extend into kernel space

3. **Argument Ranges**
   - Validate file descriptors are in range
   - Validate sizes are reasonable

---

## Implementation Notes

### copy_from_user()

```cpp
static bool copy_from_user(void* dest, const void* src, u64 len) {
    u64 src_addr = (u64)src;
    u64 end_addr = src_addr + len;

    // Must be in userspace
    if (src_addr >= 0xFFFF800000000000ULL ||
        end_addr > 0xFFFF800000000000ULL ||
        end_addr < src_addr) {
        return false;  // EFAULT
    }

    // Copy data
    memcpy(dest, src, len);
    return true;
}
```

### Ring Transition (swapgs)

All interrupts and syscalls must properly handle Ring 0 ↔ Ring 3 transitions using swapgs:

- Entry from Ring 3: swapgs to load kernel GS base
- Return to Ring 3: swapgs to restore user GS base

See `kernel/arch/x86/interrupts.S` for implementation.

---

## Alternatives Considered

### 1. Different Syscall Numbers

Some operating systems use different syscall numbers. We chose Linux-compatible numbers to:

- Ease porting of existing software
- Allow reuse of existing documentation
- Simplify libc implementation

### 2. Different Register Convention

Using rcx for argument 4 (like Windows) was considered, but rejected in favor of Linux convention for compatibility.

### 3. No Pointer Validation

Allowing direct kernel access to userspace pointers (like early Unix) was rejected for security reasons.

---

## Implementation Plan

| Phase | Task | Status |
|-------|------|--------|
| 1 | Implement basic syscalls (write, yield, exit, getpid) | ✅ Done |
| 2 | Add copy_from_user validation | ✅ Done |
| 3 | Implement swapgs for Ring transitions | ✅ Done |
| 4 | Add EFAULT handling | ✅ Done |
| 5 | Add regression tests | ✅ Done |
| 6 | Freeze ABI | ✅ Done (this RFC) |

---

## Backwards Compatibility

Once frozen, the syscall ABI will maintain backwards compatibility. Future syscalls will use new numbers; existing syscalls will never change their semantics or numbers.

---

## References

- [Linux syscall ABI](https://gitlab.com/x86-psABIs/linux-ABI)
- [System V AMD64 ABI](https://gitlab.com/x86-psABIs/x86-64-ABI)
- [Intel SDM Vol. 2A](https://www.intel.com/sdm)

---

## Change Log

| Date | Author | Changes |
|------|--------|---------|
| 2026-07-08 | JowoKernel Team | Initial RFC |
| 2026-07-08 | JowoKernel Team | Accepted |
