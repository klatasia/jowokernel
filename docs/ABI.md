# JowoKernel ABI Specification

**Version**: 0.3.3  
**Status**: Frozen (as of M3.3 PASS)  
**Last Updated**: 2026-07-08

---

## 1. Overview

Dokumen ini mendefinisikan Application Binary Interface (ABI) untuk JowoKernel. Semua perubahan pada ABI harus didokumentasikan dan melalui proses review sebelum diimplementasikan.

---

## 2. Syscall Numbers

Syscall numbers adalah **FROZEN** dan tidak akan berubah.

### Implemented Syscalls

| Name | Number | Arguments | Return | Description |
|------|--------|-----------|--------|-------------|
| `SYS_WRITE` | 1 | `(int fd, const void* buf, size_t count)` | `ssize_t` | Write to file descriptor |
| `SYS_YIELD` | 24 | `(void)` | `int` | Yield CPU to scheduler |
| `SYS_EXIT` | 60 | `(int status)` | `void` | Terminate process |
| `SYS_GETPID` | 172 | `(void)` | `pid_t` | Get current process ID |

### Reserved Syscalls (Future)

| Name | Number | Status |
|------|--------|--------|
| `SYS_READ` | 0 | Reserved |
| `SYS_OPEN` | 2 | Planned |
| `SYS_CLOSE` | 3 | Planned |
| `SYS_STAT` | 4 | Planned |
| `SYS_FSTAT` | 5 | Planned |
| `SYS_MMAP` | 9 | Planned |
| `SYS_MPROTECT` | 10 | Planned |
| `SYS_MUNMAP` | 11 | Planned |
| `SYS_BRK` | 12 | Planned |
| `SYS_RTSIGACTION` | 13 | Planned |
| `SYS_RTSIGPROCMASK` | 14 | Planned |
| `SYS_RTSIGRETURN` | 15 | Planned |
| `SYS_IOCTL` | 16 | Planned |
| `SYS_ACCESS` | 21 | Planned |
| `SYS_DUP` | 32 | Planned |
| `SYS_DUP2` | 33 | Planned |
| `SYS_PAUSE` | 34 | Planned |
| `SYS_ALARM` | 37 | Planned |
| `SYS_SETITIMER` | 38 | Planned |
| `SYS_GETPID` | 39 | Planned (alias) |
| `SYS_SOCKET` | 41 | Planned |
| `SYS_CONNECT` | 42 | Planned |
| `SYS_ACCEPT` | 43 | Planned |
| `SYS_SENDTO` | 44 | Planned |
| `SYS_RECVFROM` | 45 | Planned |
| `SYS_SENDMSG` | 46 | Planned |
| `SYS_RECVMSG` | 47 | Planned |
| `SYS_SHUTDOWN` | 48 | Planned |
| `SYS_BIND` | 49 | Planned |
| `SYS_LISTEN` | 50 | Planned |
| `SYS_GETSOCKNAME` | 51 | Planned |
| `SYS_GETPEERNAME` | 52 | Planned |
| `SYS_SOCKETPAIR` | 53 | Planned |
| `SYS_SETSOCKOPT` | 54 | Planned |
| `SYS_GETSOCKOPT` | 55 | Planned |
| `SYS_FORK` | 57 | Planned |
| `SYS_VFORK` | 58 | Planned |
| `SYS_EXECVE` | 59 | Planned |
| `SYS_WAIT4` | 61 | Planned |
| `SYS_KILL` | 62 | Planned |
| `SYS_UNAME` | 63 | Planned |
| `SYS_SEMGET` | 64 | Planned |
| `SYS_SEMOP` | 65 | Planned |
| `SYS_SEMCTL` | 66 | Planned |
| `SYS_SHMDT` | 67 | Planned |
| `SYS_MSGGET` | 68 | Planned |
| `SYS_MSGSND` | 69 | Planned |
| `SYS_MSGRCV` | 70 | Planned |
| `SYS_MSGCTL` | 71 | Planned |
| `SYS_FCNTL` | 72 | Planned |
| `SYS_FLOCK` | 73 | Planned |
| `SYS_FSYNC` | 74 | Planned |
| `SYS_FDATASYNC` | 75 | Planned |
| `SYS_TRUNCATE` | 76 | Planned |
| `SYS_FTRUNCATE` | 77 | Planned |
| `SYS_GETDENTS` | 78 | Planned |
| `SYS_GETCWD` | 79 | Planned |
| `SYS_CHDIR` | 80 | Planned |
| `SYS_FCHDIR` | 81 | Planned |
| `SYS_RENAME` | 82 | Planned |
| `SYS_MKDIR` | 83 | Planned |
| `SYS_RMDIR` | 84 | Planned |
| `SYS_CREAT` | 85 | Planned |
| `SYS_LINK` | 86 | Planned |
| `SYS_UNLINK` | 87 | Planned |
| `SYS_SYMLINK` | 88 | Planned |
| `SYS_READLINK` | 89 | Planned |
| `SYS_CHMOD` | 90 | Planned |
| `SYS_FCHMOD` | 91 | Planned |
| `SYS_CHOWN` | 92 | Planned |
| `SYS_FCHOWN` | 93 | Planned |
| `SYS_LCHOWN` | 94 | Planned |
| `SYS_UMASK` | 95 | Planned |
| `SYS_GETTIMEOFDAY` | 96 | Planned |
| `SYS_GETRLIMIT` | 97 | Planned |
| `SYS_GETRUSAGE` | 98 | Planned |
| `SYS_SYSINFO` | 99 | Planned |
| `SYS_TIMES` | 100 | Planned |
| `SYS_GETUID` | 102 | Planned |
| `SYS_SYSLOG` | 103 | Planned |
| `SYS_GETGID` | 104 | Planned |
| `SYS_SETUID` | 105 | Planned |
| `SYS_SETGID` | 106 | Planned |
| `SYS_GETEUID` | 107 | Planned |
| `SYS_GETEGID` | 108 | Planned |
| `SYS_SETPGID` | 109 | Planned |
| `SYS_GETPGID` | 110 | Planned |
| `SYS_GETPPID` | 111 | Planned |
| `SYS_SETPGRP` | 112 | Planned |
| `SYS_SETREGID` | 113 | Planned |
| `SYS_SETREUID` | 114 | Planned |
| `SYS_GETGROUPS` | 115 | Planned |
| `SYS_SETGROUPS` | 116 | Planned |
| `SYS_SETRESUID` | 117 | Planned |
| `SYS_GETRESUID` | 118 | Planned |
| `SYS_SETRESGID` | 119 | Planned |
| `SYS_GETRESGID` | 120 | Planned |
| `SYS_GETPGID` | 121 | Planned |
| `SYS_SETFSUID` | 122 | Planned |
| `SYS_SETFSGID` | 123 | Planned |
| `SYS_LSEEK` | 8 | Planned |
| `SYS_MLOCK` | 203 | Planned |
| `SYS_MUNLOCK` | 204 | Planned |
| `SYS_MLOCKALL` | 205 | Planned |
| `SYS_MUNLOCKALL` | 206 | Planned |

---

## 3. Register Convention

### 3.1 Syscall Arguments (Linux x86-64 ABI)

```
rax = syscall number
rdi = arg1
rsi = arg2
rdx = arg3
r10 = arg4  (not rcx!)
r8  = arg5
r9  = arg6
```

### 3.2 Return Value

```
rax = return value (signed for errors)
```

Negative return values indicate errors (like Linux).

### 3.3 Preserved vs Clobbered

**Preserved (callee-saved)**:
- rbx, rcx, rdx, rsi, rdi, rbp, r8-r15
- rsp (stack pointer)
- RIP (instruction pointer)

**Clobbered (caller-saved)**:
- rax (syscall return)
- rcx (syscall instruction clobbers this)
- r11 (RFLAGS saved by syscall)

---

## 4. Process ABI

### 4.1 Initial Register State

When a new process starts (after `execve`), registers are set as:

```
rax = 0          (no syscall return value)
rdi = argc       (argument count)
rsi = argv       (pointer to argument vector)
rdx = envp       (pointer to environment vector)
rsp = initial stack pointer (aligned to 16 bytes)
rip = entry point (from ELF header)
```

### 4.2 Stack Layout

```
High Address
    ┌────────────────────────┐
    │     NULL (envp end)    │  ← envp[envc] = NULL
    ├────────────────────────┤
    │     ENV[envc-1]        │
    ├────────────────────────┤
    │     ...                │
    ├────────────────────────┤
    │     ENV[0]             │
    ├────────────────────────┤
    │     NULL (argv end)    │  ← argv[argc] = NULL
    ├────────────────────────┤
    │     ARGV[argc-1]       │
    ├────────────────────────┤
    │     ...                │
    ├────────────────────────┤
    │     ARGV[0]            │  ← program name
    ├────────────────────────┤
    │     argc              │  ← argument count
    ├────────────────────────┤
    │   (return address)     │
    ├────────────────────────┤
    │   (optional red zone)   │  ← 128 bytes below rsp
    └────────────────────────┘
    
Low Address (stack grows down)
```

### 4.3 Process States

| State | Value | Description |
|-------|-------|-------------|
| `INITIAL` | 0 | Process being created |
| `RUNNING` | 1 | Process is running |
| `ZOMBIE` | 2 | Process terminated, waiting for parent |
| `DEAD` | 3 | Process fully cleaned up |

---

## 5. ELF ABI

### 5.1 ELF Class

- **Supported**: ELF64 (ELFCLASS64)
- **Endianness**: Little Endian (ELFDATA2LSB)
- **Machine**: EM_X86_64

### 5.2 Required Segments

| Type | Permissions | Required |
|------|-------------|----------|
| `PT_LOAD` | RX (code) | Yes |
| `PT_LOAD` | RW (data) | Yes |
| `PT_GNU_STACK` | RW | Yes (for executable stack) |

### 5.3 Virtual Address Layout

```
0x400000  ┌─────────────────┐
          │   .text         │  (executable, readable)
          ├─────────────────┤
          │   .rodata       │  (readable only)
          ├─────────────────┤
          │   .data         │  (readable, writable)
          ├─────────────────┤
          │   .bss          │  (readable, writable, zeroed)
0x600000  └─────────────────┘
          
0x7FFF_FFFF  (user space limit)
          
0xFFFF8000_00000000  (kernel space start)
```

---

## 6. Memory Model

### 6.1 User Space Range

```
Canonical user address space:
  Low:  0x0000000000000000
  High: 0x00007FFFFFFFFFFF  (47-bit canonical)
```

### 6.2 Kernel Space Range

```
Canonical kernel address space:
  Low:  0xFFFF800000000000
  High: 0xFFFFFFFFFFFFFFFF  (64-bit canonical)
```

### 6.3 copy_from_user Validation

```cpp
// Valid user address: < 0xFFFF800000000000
static bool is_user_address(const void* addr) {
    return ((u64)addr) < 0xFFFF800000000000ULL;
}
```

---

## 7. Error Codes

### 7.1 Standard Errors

| Name | Value | Description |
|------|-------|-------------|
| `EPERM` | 1 | Operation not permitted |
| `ENOENT` | 2 | No such file or directory |
| `EINTR` | 4 | Interrupted system call |
| `EIO` | 5 | I/O error |
| `EAGAIN` | 11 | Try again |
| `ENOMEM` | 12 | Out of memory |
| `EFAULT` | 14 | Bad address |
| `EBUSY` | 16 | Device or resource busy |
| `EEXIST` | 17 | File exists |
| `EXDEV` | 18 | Cross-device link |
| `ENODEV` | 19 | No such device |
| `ENOTDIR` | 20 | Not a directory |
| `EISDIR` | 21 | Is a directory |
| `EINVAL` | 22 | Invalid argument |
| `ENFILE` | 23 | File table overflow |
| `EMFILE` | 24 | Too many open files |
| `ENOTTY` | 25 | Not a typewriter |
| `ETXTBSY` | 26 | Text file busy |
| `EFBIG` | 27 | File too large |
| `ENOSPC` | 28 | No space left on device |
| `ESPIPE` | 29 | Illegal seek |
| `EROFS` | 30 | Read-only file system |
| `EMLINK` | 31 | Too many links |
| `EPIPE` | 32 | Broken pipe |
| `EDOM` | 33 | Math argument out of domain |
| `ERANGE` | 34 | Math result not representable |
| `ENOSYS` | 38 | Invalid syscall |
| `ENAMETOOLONG` | 36 | File name too long |

---

## 8. Virtual File Descriptors

### 8.1 Standard Descriptors

| FD | Name | Default Stream |
|----|------|----------------|
| 0 | `STDIN_FILENO` | Standard input |
| 1 | `STDOUT_FILENO` | Standard output |
| 2 | `STDERR_FILENO` | Standard error |

### 8.2 FD Table

- Maximum FD per process: 256
- FD 0-2 are reserved for stdio
- New FDs are allocated lowest available

---

## 9. Thread ABI

### 9.1 Thread Local Storage (TLS)

TLS is reserved for future implementation.

### 9.2 Thread States

| State | Value | Description |
|-------|-------|-------------|
| `INITIAL` | 0 | Thread being created |
| `RUNNING` | 1 | Thread is running |
| `READY` | 2 | Thread ready to run |
| `BLOCKED` | 3 | Thread waiting |
| `TERMINATED` | 4 | Thread finished |

---

## 10. Future Extensions

### 10.1 Planned Features

| Feature | Priority | Status |
|---------|----------|--------|
| VDSO | High | Planned |
| Signal Handling | High | Planned |
| Thread-local Storage | Medium | Planned |
| Futex | Medium | Planned |
| User-space Futex | Medium | Planned |

### 10.2 VDSO Layout (Future)

```
0xFFFFFFFFFF600000  ┌─────────────────┐
                    │   vvar          │  (read-only)
                    ├─────────────────┤
                    │   vdso          │  (executable)
                    ├─────────────────┤
                    │   vclock        │  (time)
                    └─────────────────┘
```

---

## 11. Change Log

| Version | Date | Changes |
|---------|------|---------|
| 0.3.3 | 2026-07-08 | Initial frozen ABI, M3.3 PASS |
| - | - | - |

---

## 12. References

- [Linux x86-64 syscall ABI](https://gitlab.com/x86-psABIs/linux-ABI/-/wikis/home)
- [ELF64 Specification](https://refspecs.linuxfoundation.org/elf/gabi4+/contents.html)
- [System V AMD64 ABI](https://gitlab.com/x86-psABIs/x86-64-ABI)
- [Intel Software Developer Manual Vol. 2](https://www.intel.com/sdm)

---

**Maintainer**: JowoKernel Team  
**Approval Required For**: Any ABI change
