#ifndef JOWO_SYSCALLS_H
#define JOWO_SYSCALLS_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Standard POSIX (Linux x86_64 ABI)
#define SYS_READ       0
#define SYS_WRITE      1
#define SYS_OPEN       2
#define SYS_CLOSE      3
#define SYS_MMAP       9
#define SYS_MUNMAP     11
#define SYS_BRK        12
#define SYS_RT_SIGACTION 13
#define SYS_RT_SIGRETURN 15
#define SYS_YIELD      24
#define SYS_NANOSLEEP  35
#define SYS_GETPID     39
#define SYS_FORK       57
#define SYS_EXECVE     59
#define SYS_EXIT       60
#define SYS_WAIT4      61
#define SYS_KILL       62
#define SYS_ARCH_PRCTL 158

// JowoKernel Specific (Mulai dari 400+)
#define SYS_CHANNEL_CREATE 400
#define SYS_CHANNEL_WRITE  401
#define SYS_CHANNEL_READ   402
#define SYS_VMO_CREATE_PHYSICAL 403
#define SYS_IOPORT_WRITE16 404
#define SYS_IOPORT_READ16 405
#define SYS_HANDLE_CLS 406
#define SYS_VMO_CREATE     407
#define SYS_OBJECT_WAIT    408
#define SYS_INTERRUPT_BIND 409
#define SYS_GET_TICKS      410
#define SYS_OBJECT_WAIT_MANY 411
#define SYS_GET_TIME       412

// Raw Syscall Wrappers (Inline Assembly)
uint64_t syscall0(uint64_t n);
uint64_t syscall1(uint64_t n, uint64_t arg1);
uint64_t syscall2(uint64_t n, uint64_t arg1, uint64_t arg2);
uint64_t syscall3(uint64_t n, uint64_t arg1, uint64_t arg2, uint64_t arg3);
uint64_t syscall5(uint64_t n, uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5);

// API Functions
void sys_exit(int code);
void sys_write(int fd, const char* str, uint64_t len);
int getpid(void);
void sys_yield(void);
uint16_t sys_ioport_read16(uint16_t port);
void sys_ioport_write16(uint16_t port, uint16_t val);
int sys_vmo_create_physical(uint64_t phys_addr, uint64_t size, uint32_t options, int* out_handle, uint64_t virt_addr);
int sys_vmo_create(uint64_t size, uint32_t options, int* out_handle, uint64_t virt_addr);
int sys_interrupt_bind(int vector, int* out_handle);
int sys_object_wait(int handle);
uint64_t sys_get_ticks(void);
void sys_nanosleep(uint64_t nanoseconds);
int sys_fork(void);
int sys_wait4(int pid, int* status);
int sys_object_wait_many(int* handles, uint32_t* wait_signals, uint32_t num_items, uint64_t timeout, uint32_t* out_observed);
int sys_getpid(void);
int sys_kill(int pid, int sig);

struct jowo_rtc_time {
    uint16_t year;
    uint8_t month;
    uint8_t day;
    uint8_t hour;
    uint8_t minute;
    uint8_t second;
};
int sys_get_time(struct jowo_rtc_time* time_out);

// Object Signals
#define SIGNAL_READABLE 1
#define SIGNAL_WRITABLE 2
#define SIGNAL_PEER_CLOSED 4
#define SIGNAL_SIGNALED 8

#ifdef __cplusplus
}
#endif

#endif // JOWO_SYSCALLS_H
