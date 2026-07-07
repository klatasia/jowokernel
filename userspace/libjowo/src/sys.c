#include <jowo/syscalls.h>

uint64_t syscall0(uint64_t n) {
    uint64_t ret;
    asm volatile("syscall" : "=a"(ret) : "a"(n) : "rcx", "r11", "memory");
    return ret;
}

uint64_t syscall1(uint64_t n, uint64_t arg1) {
    uint64_t ret;
    asm volatile("syscall" : "=a"(ret) : "a"(n), "D"(arg1) : "rcx", "r11", "memory");
    return ret;
}

uint64_t syscall2(uint64_t n, uint64_t arg1, uint64_t arg2) {
    uint64_t ret;
    asm volatile("syscall" : "=a"(ret) : "a"(n), "D"(arg1), "S"(arg2) : "rcx", "r11", "memory");
    return ret;
}

uint64_t syscall3(uint64_t n, uint64_t arg1, uint64_t arg2, uint64_t arg3) {
    uint64_t ret;
    asm volatile("syscall" : "=a"(ret) : "a"(n), "D"(arg1), "S"(arg2), "d"(arg3) : "rcx", "r11", "memory");
    return ret;
}

uint64_t syscall5(uint64_t n, uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5) {
    uint64_t ret;
    register uint64_t r10 asm("r10") = arg4;
    register uint64_t r8 asm("r8") = arg5;
    asm volatile("syscall" : "=a"(ret) : "a"(n), "D"(arg1), "S"(arg2), "d"(arg3), "r"(r10), "r"(r8) : "rcx", "r11", "memory");
    return ret;
}

void sys_exit(int code) {
    syscall1(SYS_EXIT, (uint64_t)code);
    while(1);
}

void sys_write(int fd, const char* str, uint64_t len) {
    syscall3(SYS_WRITE, (uint64_t)fd, (uint64_t)str, len);
}

int sys_getpid(void) {
    return (int)syscall0(SYS_GETPID);
}

int sys_kill(int pid, int sig) {
    return (int)syscall2(SYS_KILL, (uint64_t)pid, (uint64_t)sig);
}

void sys_yield(void) {
    syscall0(SYS_YIELD);
}

uint64_t sys_get_ticks(void) {
    return syscall0(SYS_GET_TICKS);
}

uint16_t sys_ioport_read16(uint16_t port) {
    return (uint16_t)syscall1(SYS_IOPORT_READ16, (uint64_t)port);
}

void sys_ioport_write16(uint16_t port, uint16_t val) {
    syscall2(SYS_IOPORT_WRITE16, (uint64_t)port, (uint64_t)val);
}

int sys_vmo_create_physical(uint64_t phys_addr, uint64_t size, uint32_t options, int* out_handle, uint64_t virt_addr) {
    return (int)syscall5(SYS_VMO_CREATE_PHYSICAL, phys_addr, size, (uint64_t)options, (uint64_t)out_handle, virt_addr);
}

uint64_t syscall4(uint64_t n, uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4) {
    uint64_t ret;
    register uint64_t r10 asm("r10") = arg4;
    asm volatile("syscall" : "=a"(ret) : "a"(n), "D"(arg1), "S"(arg2), "d"(arg3), "r"(r10) : "rcx", "r11", "memory");
    return ret;
}

int sys_vmo_create(uint64_t size, uint32_t options, int* out_handle, uint64_t virt_addr) {
    return (int)syscall4(SYS_VMO_CREATE, size, (uint64_t)options, (uint64_t)out_handle, virt_addr);
}

int sys_interrupt_bind(int vector, int* out_handle) {
    return (int)syscall2(SYS_INTERRUPT_BIND, (uint64_t)vector, (uint64_t)out_handle);
}

int sys_object_wait(int handle) {
    return (int)syscall1(SYS_OBJECT_WAIT, (uint64_t)handle);
}

void sys_nanosleep(uint64_t nanoseconds) {
    syscall1(SYS_NANOSLEEP, nanoseconds);
}

int sys_fork(void) {
    return (int)syscall0(SYS_FORK);
}

int sys_wait4(int pid, int* status) {
    return (int)syscall2(SYS_WAIT4, (uint64_t)pid, (uint64_t)status);
}

int sys_object_wait_many(int* handles, uint32_t* wait_signals, uint32_t num_items, uint64_t timeout, uint32_t* out_observed) {
    return (int)syscall5(SYS_OBJECT_WAIT_MANY, (uint64_t)handles, (uint64_t)wait_signals, (uint64_t)num_items, timeout, (uint64_t)out_observed);
}

int sys_get_time(struct jowo_rtc_time* time_out) {
    return (int)syscall1(SYS_GET_TIME, (uint64_t)time_out);
}
