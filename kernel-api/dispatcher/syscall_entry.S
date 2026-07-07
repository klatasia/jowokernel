# x86_64 Syscall Entry
# JowoKernel Userspace Syscall Stubs
#
# This file provides the low-level syscall entry point for userspace programs.
# The syscall instruction on x86_64 passes arguments as follows:
#   syscall instruction:
#     syscall number -> rax
#     arg1 -> rdi
#     arg2 -> rsi
#     arg3 -> rdx
#     arg4 -> r10 (note: NOT rcx, because syscall clobbers rcx)
#     arg5 -> r8
#     arg6 -> r9
#   Return value -> rax

.text

# Syscall wrapper - this is called from C code
# extern uint64_t __syscall(uint64_t nr, uint64_t a1, uint64_t a2,
#                            uint64_t a3, uint64_t a4, uint64_t a5, uint64_t a6);
.globl __syscall
.type __syscall, @function
__syscall:
    movq    %rdi, %rax          # syscall number
    movq    %rsi, %rdi          # arg1
    movq    %rdx, %rsi          # arg2
    movq    %rcx, %rdx          # arg3 (rcx was arg4 on input)
    movq    %r8, %r10           # arg4 (must be r10, not rcx)
    movq    %r9, %r8            # arg5
    pushq   %r9                 # arg6 (push to stack for call)
    syscall                       # invoke syscall instruction
    popq    %rcx                # cleanup stack
    ret

# Alternative entry point using syscall directly
# This can be used for performance-critical code
.globl __syscall_direct
.type __syscall_direct, @function
__syscall_direct:
    # Input registers for syscall instruction:
    # rax = syscall number
    # rdi, rsi, rdx, r10, r8, r9 = arguments
    # Output: rax = return value
    syscall
    ret

# ============================================================
# Common syscall wrappers for POSIX compatibility
# ============================================================

# void sys_exit(int status) __attribute__((noreturn));
.globl sys_exit
.type sys_exit, @function
sys_exit:
    movl    %edi, %ebx          # status in ebx
    movl    $60, %eax           # SYS_exit
    syscall
    hlt                             # should never reach here

# ssize_t sys_read(int fd, void* buf, size_t count);
.globl sys_read
.type sys_read, @function
sys_read:
    movl    %edi, %eax          # fd
    movq    %rsi, %rdi          # buf
    movq    %rdx, %rsi          # count
    movl    $0, %eax            # SYS_read = 0
    syscall
    ret

# ssize_t sys_write(int fd, const void* buf, size_t count);
.globl sys_write
.type sys_write, @function
sys_write:
    movl    %edi, %eax          # fd
    movq    %rsi, %rdi          # buf
    movq    %rdx, %rsi          # count
    movl    $1, %eax            # SYS_write = 1
    syscall
    ret

# int sys_open(const char* path, int flags, int mode);
.globl sys_open
.type sys_open, @function
sys_open:
    movq    %rdi, %rax          # path
    movl    %esi, %edx          # flags
    movl    %edx, %edx
    movl    %edx, %ecx          # mode
    movl    $2, %eax            # SYS_open = 2
    syscall
    ret

# int sys_close(int fd);
.globl sys_close
.type sys_close, @function
sys_close:
    movl    %edi, %eax          # fd
    movl    $3, %eax            # SYS_close = 3
    syscall
    ret

# void* sys_mmap(void* addr, size_t length, int prot, int flags, int fd, off_t offset);
.globl sys_mmap
.type sys_mmap, @function
sys_mmap:
    # x86_64 syscall: mmap(addr, length, prot, flags, fd, offset)
    movq    %rdi, %r10          # addr
    movq    %rsi, %rsi          # length
    movl    %edx, %edx          # prot
    movl    %ecx, %r9d          # flags (ecx was pushed)
    movl    %r9d, %r9d
    movl    8(%rsp), %r8d       # fd from stack
    movq    16(%rsp), %rcx      # offset from stack
    movq    %rcx, %rcx
    movl    $9, %eax            # SYS_mmap = 9
    syscall
    ret

# int sys_munmap(void* addr, size_t length);
.globl sys_munmap
.type sys_munmap, @function
sys_munmap:
    movq    %rdi, %rsi          # addr
    movq    %rsi, %rsi
    movq    %rdx, %rdx          # length
    movl    $11, %eax           # SYS_munmap = 11
    syscall
    ret

# int sys_mprotect(void* addr, size_t length, int prot);
.globl sys_mprotect
.type sys_mprotect, @function
sys_mprotect:
    movq    %rdi, %rsi          # addr
    movq    %rsi, %rsi
    movq    %rdx, %rdx          # length
    movl    %ecx, %r9d          # prot (ecx was pushed)
    movl    $10, %eax           # SYS_mprotect = 10
    syscall
    ret

# pid_t sys_getpid(void);
.globl sys_getpid
.type sys_getpid, @function
sys_getpid:
    movl    $39, %eax           # SYS_getpid = 39
    syscall
    ret

# void sys_yield(void);
.globl sys_yield
.type sys_yield, @function
sys_yield:
    movl    $24, %eax           # SYS_sched_yield = 24
    syscall
    ret

# int sys_nanosleep(const struct timespec* req, struct timespec* rem);
.globl sys_nanosleep
.type sys_nanosleep, @function
sys_nanosleep:
    movq    %rdi, %rax          # req
    movq    %rsi, %rdi          # rem
    movl    $35, %eax           # SYS_nanosleep = 35
    syscall
    ret

# int sys_clock_gettime(clockid_t clk_id, struct timespec* tp);
.globl sys_clock_gettime
.type sys_clock_gettime, @function
sys_clock_gettime:
    movl    %edi, %eax          # clk_id
    movq    %rsi, %rdi          # tp
    movl    $228, %eax          # SYS_clock_gettime = 228
    syscall
    ret

# int sys_clone(int flags, void* stack, int* parent_tid, int* child_tid, void* tls);
.globl sys_clone
.type sys_clone, @function
sys_clone:
    movl    %edi, %eax          # flags
    movq    %rsi, %rdi          # stack
    movq    %rdx, %rsi          # parent_tid
    movq    %rcx, %r10          # child_tid (must be r10)
    movq    8(%rsp), %r8        # tls from stack
    movq    16(%rsp), %r9       # unused from stack
    movl    $56, %eax           # SYS_clone = 56
    syscall
    ret

# int sys_execve(const char* path, char* const argv[], char* const envp[]);
.globl sys_execve
.type sys_execve, @function
sys_execve:
    movq    %rdi, %rax          # path
    movq    %rsi, %rdi          # argv
    movq    %rdx, %rsi          # envp
    movl    $59, %eax           # SYS_execve = 59
    syscall
    ret

# int sys_wait4(int pid, int* status, int options, struct rusage* rusage);
.globl sys_wait4
.type sys_wait4, @function
sys_wait4:
    movl    %edi, %eax          # pid
    movq    %rsi, %rdi          # status
    movl    %edx, %esi          # options
    movq    %rcx, %rdx          # rusage
    movl    $61, %eax           # SYS_wait4 = 61
    syscall
    ret

# int sys_kill(int pid, int sig);
.globl sys_kill
.type sys_kill, @function
sys_kill:
    movl    %edi, %eax          # pid
    movl    %esi, %edi          # sig
    movl    $62, %eax           # SYS_kill = 62
    syscall
    ret

# int sys_arch_prctl(int code, void* addr);
.globl sys_arch_prctl
.type sys_arch_prctl, @function
sys_arch_prctl:
    movl    %edi, %eax          # code
    movq    %rsi, %rdi          # addr
    movl    $158, %eax          # SYS_arch_prctl = 158
    syscall
    ret

# int sys_futex(int* uaddr, int op, int val, const struct timespec* timeout, int* uaddr2, int val3);
.globl sys_futex
.type sys_futex, @function
sys_futex:
    movq    %rdi, %rax          # uaddr
    movl    %esi, %edi          # op
    movl    %edx, %esi          # val
    movq    %rcx, %rdx          # timeout
    movq    %r8, %r10          # uaddr2 (must be r10)
    movl    %r9d, %r9d          # val3
    movl    $202, %eax          # SYS_futex = 202
    syscall
    ret

# int sys_pipe2(int pipefd[2], int flags);
.globl sys_pipe2
.type sys_pipe2, @function
sys_pipe2:
    movq    %rdi, %rax          # pipefd
    movl    %esi, %esi          # flags
    movl    $293, %eax          # SYS_pipe2 = 293
    syscall
    ret

# off_t sys_lseek(int fd, off_t offset, int whence);
.globl sys_lseek
.type sys_lseek, @function
sys_lseek:
    movl    %edi, %eax          # fd
    movq    %rsi, %rsi          # offset
    movl    %edx, %edx          # whence
    movl    $8, %eax            # SYS_lseek = 8
    syscall
    ret

# ============================================================
# JowoKernel-specific syscall wrappers
# ============================================================

# int sys_handle_close(int handle);
.globl sys_handle_close
.type sys_handle_close, @function
sys_handle_close:
    movl    %edi, %eax          # handle
    movl    $406, %eax          # SYS_HANDLE_CLOSE = 406
    syscall
    ret

# int sys_channel_create(int* out_handle0, int* out_handle1);
.globl sys_channel_create
.type sys_channel_create, @function
sys_channel_create:
    movq    %rdi, %rax          # out_handle0
    movq    %rax, %rdi
    movq    %rax, %rsi          # out_handle1 (same for now, need separate)
    movl    $400, %eax          # SYS_CHANNEL_CREATE = 400
    syscall
    ret

# int sys_vmo_create(uint64_t size, uint32_t options, int* out_handle);
.globl sys_vmo_create
.type sys_vmo_create, @function
sys_vmo_create:
    movq    %rdi, %rax          # size
    movl    %esi, %edi          # options
    movq    %rdx, %rsi          # out_handle
    movl    $407, %eax          # SYS_VMO_CREATE = 407
    syscall
    ret

# int sys_object_wait(int handle, uint32_t signals, uint64_t timeout);
.globl sys_object_wait
.type sys_object_wait, @function
sys_object_wait:
    movl    %edi, %eax          # handle
    movl    %esi, %edi          # signals
    movq    %rdx, %rsi          # timeout
    movl    $408, %eax          # SYS_OBJECT_WAIT = 408
    syscall
    ret

# uint64_t sys_get_ticks(void);
.globl sys_get_ticks
.type sys_get_ticks, @function
sys_get_ticks:
    movl    $410, %eax          # SYS_GET_TICKS = 410
    syscall
    ret

# ============================================================
# Misc utilities
# ============================================================

# void _start(void) - entry point for executables
# This is provided by the C runtime, but we can override here if needed
.globl _start
.type _start, @function
_start:
    # Standard x86_64 ELF entry
    xorl    %ebp, %ebp
    movq    %rsp, %rbp
    # argc is at top of stack, argv follows
    andq    $-16, %rsp          # align stack to 16 bytes
    # Call main with (argc, argv, envp)
    call    main
    # main returned - exit with its return value
    movl    %eax, %edi
    movl    $60, %eax           # SYS_exit
    syscall
    hlt

.size __syscall, .-__syscall
.size sys_exit, .-sys_exit
.size sys_read, .-sys_read
.size sys_write, .-sys_write
.size sys_open, .-sys_open
.size sys_close, .-sys_close
.size _start, .-_start
