/**
 * JowoLibc - Userspace C Library for KLAT OS
 *
 * Minimal C library providing standard functions and syscall wrappers.
 * This is linked with userspace programs to provide POSIX-compatible APIs.
 */

#pragma once

#include <stdint.h>
#include <stddef.h>
#include <stdarg.h>

#ifdef __cplusplus
extern "C" {
#endif

// ============================================================
// Syscall Numbers (must match JowoKernel)
// ============================================================

#define SYS_EXIT          60
#define SYS_READ          0
#define SYS_WRITE         1
#define SYS_OPEN          2
#define SYS_CLOSE         3
#define SYS_FSTAT         5
#define SYS_MMAP          9
#define SYS_MUNMAP        11
#define SYS_BRK           12
#define SYS_GETPID        39
#define SYS_CLONE         56
#define SYS_EXECVE        59
#define SYS_WAIT4         61
#define SYS_KILL          62
#define SYS_YIELD         24
#define SYS_NANOSLEEP     35
#define SYS_ARCH_PRCTL   158
#define SYS_FUTEX        202
#define SYS_CLOCK_GETTIME 228
#define SYS_OPENAT       257

// ============================================================
// Syscall Wrapper
// ============================================================

static inline long __syscall(long n, long a1, long a2, long a3, long a4, long a5, long a6) {
    long ret;
    __asm__ volatile (
        "syscall"
        : "=a"(ret)
        : "a"(n), "D"(a1), "S"(a2), "d"(a3), "r"(a4), "r"(a5), "r"(a6)
        : "rcx", "r11", "memory"
    );
    return ret;
}

// ============================================================
// Memory/IO Operations
// ============================================================

void* memset(void* s, int c, size_t n);
void* memcpy(void* dest, const void* src, size_t n);
void* memmove(void* dest, const void* src, size_t n);
int memcmp(const void* s1, const void* s2, size_t n);
size_t strlen(const char* s);
size_t strnlen(const char* s, size_t maxlen);
char* strcpy(char* dest, const char* src);
char* strncpy(char* dest, const char* src, size_t n);
int strcmp(const char* s1, const char* s2);
int strncmp(const char* s1, const char* s2, size_t n);

// ============================================================
// Output Functions
// ============================================================

void putchar(char c);
void puts(const char* s);
int printf(const char* fmt, ...);
int vprintf(const char* fmt, va_list ap);
int snprintf(char* buf, size_t size, const char* fmt, ...);

// ============================================================
// Syscall Wrappers
// ============================================================

static inline void exit(int status) {
    __syscall(SYS_EXIT, status, 0, 0, 0, 0, 0);
    __builtin_unreachable();
}

static inline ssize_t read(int fd, void* buf, size_t count) {
    return __syscall(SYS_READ, fd, (long)buf, count, 0, 0, 0);
}

static inline ssize_t write(int fd, const void* buf, size_t count) {
    return __syscall(SYS_WRITE, fd, (long)buf, count, 0, 0, 0);
}

static inline int open(const char* path, int flags, int mode) {
    return __syscall(SYS_OPEN, (long)path, flags, mode, 0, 0, 0);
}

static inline int close(int fd) {
    return __syscall(SYS_CLOSE, fd, 0, 0, 0, 0, 0);
}

static inline int getpid(void) {
    return __syscall(SYS_GETPID, 0, 0, 0, 0, 0, 0);
}

static inline void yield(void) {
    __syscall(SYS_YIELD, 0, 0, 0, 0, 0, 0);
}

static inline int nanosleep(const struct timespec* req, struct timespec* rem) {
    return __syscall(SYS_NANOSLEEP, (long)req, (long)rem, 0, 0, 0, 0);
}

// ============================================================
// Process Control
// ============================================================

struct timespec {
    long tv_sec;
    long tv_nsec;
};

struct stat {
    unsigned long st_dev;
    unsigned long st_ino;
    unsigned long st_nlink;
    unsigned int st_mode;
    unsigned int st_uid;
    unsigned int st_gid;
    int __pad0;
    unsigned long st_rdev;
    long st_size;
    long st_blksize;
    long st_blocks;
    long st_atime;
    unsigned long st_atime_nsec;
    long st_mtime;
    unsigned long st_mtime_nsec;
    long st_ctime;
    unsigned long st_ctime_nsec;
};

static inline int fstat(int fd, struct stat* buf) {
    return __syscall(SYS_FSTAT, fd, (long)buf, 0, 0, 0, 0);
}

static inline int brk(void* addr) {
    return __syscall(SYS_BRK, (long)addr, 0, 0, 0, 0, 0);
}

static inline void* sbrk(intptr_t incr) {
    static void* current_brk = (void*)0x200000000000;
    void* new_brk = (void*)((char*)current_brk + incr);
    if (brk(new_brk) == 0) {
        void* old_brk = current_brk;
        current_brk = new_brk;
        return old_brk;
    }
    return (void*)-1;
}

// ============================================================
// Memory Allocation
// ============================================================

static inline void* malloc(size_t size) {
    return sbrk(size);
}

static inline void free(void* ptr) {
    (void)ptr; // Simplified - no actual free
}

// ============================================================
// String Functions Implementation
// ============================================================

void* memset(void* s, int c, size_t n) {
    unsigned char* p = (unsigned char*)s;
    while (n--) *p++ = (unsigned char)c;
    return s;
}

void* memcpy(void* dest, const void* src, size_t n) {
    unsigned char* d = (unsigned char*)dest;
    const unsigned char* s = (const unsigned char*)src;
    while (n--) *d++ = *s++;
    return dest;
}

void* memmove(void* dest, const void* src, size_t n) {
    unsigned char* d = (unsigned char*)dest;
    const unsigned char* s = (const unsigned char*)src;
    if (d < s) {
        while (n--) *d++ = *s++;
    } else {
        d += n;
        s += n;
        while (n--) *--d = *--s;
    }
    return dest;
}

int memcmp(const void* s1, const void* s2, size_t n) {
    const unsigned char* a = (const unsigned char*)s1;
    const unsigned char* b = (const unsigned char*)s2;
    while (n--) {
        if (*a != *b) return *a - *b;
        a++; b++;
    }
    return 0;
}

size_t strlen(const char* s) {
    size_t n = 0;
    while (s[n]) n++;
    return n;
}

size_t strnlen(const char* s, size_t maxlen) {
    size_t n = 0;
    while (n < maxlen && s[n]) n++;
    return n;
}

char* strcpy(char* dest, const char* src) {
    char* d = dest;
    while ((*d++ = *src++));
    return dest;
}

char* strncpy(char* dest, const char* src, size_t n) {
    char* d = dest;
    while (n && *src) {
        *d++ = *src++;
        n--;
    }
    while (n--) *d++ = 0;
    return dest;
}

int strcmp(const char* s1, const char* s2) {
    while (*s1 && *s2 && *s1 == *s2) {
        s1++; s2++;
    }
    return (unsigned char)*s1 - (unsigned char)*s2;
}

int strncmp(const char* s1, const char* s2, size_t n) {
    while (n && *s1 && *s2 && *s1 == *s2) {
        s1++; s2++; n--;
    }
    if (n == 0) return 0;
    return (unsigned char)*s1 - (unsigned char)*s2;
}

// ============================================================
// Output Implementation
// ============================================================

void putchar(char c) {
    write(1, &c, 1);
}

void puts(const char* s) {
    while (*s) {
        putchar(*s++);
    }
    putchar('\n');
}

// Simple printf implementation
static void print_int(long val, int base, int sign) {
    char buf[32];
    int i = 0;
    char* digits = "0123456789abcdef";

    if (sign && val < 0) {
        putchar('-');
        val = -val;
    }

    if (val == 0) {
        putchar('0');
        return;
    }

    while (val > 0 && i < 32) {
        buf[i++] = digits[val % base];
        val /= base;
    }

    while (i--) putchar(buf[i]);
}

static void print_ptr(unsigned long val) {
    puts("0x");
    char buf[16];
    int i = 0;
    char* digits = "0123456789abcdef";
    while (val > 0 && i < 16) {
        buf[i++] = digits[val % 16];
        val /= 16;
    }
    while (i--) putchar(buf[i]);
}

int vprintf(const char* fmt, va_list ap) {
    int count = 0;
    char c;

    while ((c = *fmt++)) {
        if (c != '%') {
            putchar(c);
            count++;
            continue;
        }

        c = *fmt++;
        switch (c) {
            case 'c': {
                char ch = va_arg(ap, int);
                putchar(ch);
                count++;
                break;
            }
            case 's': {
                const char* s = va_arg(ap, const char*);
                while (*s) {
                    putchar(*s++);
                    count++;
                }
                break;
            }
            case 'd':
            case 'i': {
                int val = va_arg(ap, int);
                print_int(val, 10, 1);
                count++;
                break;
            }
            case 'u': {
                unsigned int val = va_arg(ap, unsigned int);
                print_int(val, 10, 0);
                count++;
                break;
            }
            case 'x':
            case 'p': {
                unsigned long val = va_arg(ap, unsigned long);
                print_int(val, 16, 0);
                count++;
                break;
            }
            case 'l': {
                long val = va_arg(ap, long);
                print_int(val, 10, 1);
                count++;
                break;
            }
            case 'z': {
                size_t val = va_arg(ap, size_t);
                print_int((long)val, 10, 0);
                count++;
                break;
            }
            case '%':
                putchar('%');
                count++;
                break;
            default:
                putchar('%');
                putchar(c);
                count += 2;
                break;
        }
    }

    return count;
}

int printf(const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int ret = vprintf(fmt, ap);
    va_end(ap);
    return ret;
}

int snprintf(char* buf, size_t size, const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);

    // Simplified: just format into buffer
    int written = 0;
    char c;

    while (written < (int)size - 1 && (c = *fmt++)) {
        if (c != '%') {
            buf[written++] = c;
            continue;
        }

        c = *fmt++;
        switch (c) {
            case 's': {
                const char* s = va_arg(ap, const char*);
                while (*s && written < (int)size - 1) {
                    buf[written++] = *s++;
                }
                break;
            }
            case 'd':
            case 'i': {
                int val = va_arg(ap, int);
                char tmp[32];
                int len = 0;
                if (val < 0) {
                    if (written < (int)size - 1) buf[written++] = '-';
                    val = -val;
                }
                if (val == 0) {
                    if (written < (int)size - 1) buf[written++] = '0';
                } else {
                    while (val > 0 && written < (int)size - 1) {
                        buf[written++] = '0' + (val % 10);
                        val /= 10;
                    }
                }
                break;
            }
            case 'u': {
                unsigned int val = va_arg(ap, unsigned int);
                while (val > 0 && written < (int)size - 1) {
                    buf[written++] = '0' + (val % 10);
                    val /= 10;
                }
                break;
            }
            case 'x':
            case 'p': {
                unsigned long val = va_arg(ap, unsigned long);
                while (val > 0 && written < (int)size - 1) {
                    buf[written++] = "0123456789abcdef"[val % 16];
                    val /= 16;
                }
                break;
            }
            case 'l': {
                long val = va_arg(ap, long);
                print_int(val, 10, 1);
                break;
            }
            case '%':
                if (written < (int)size - 1) buf[written++] = '%';
                break;
            default:
                break;
        }
    }

    buf[written] = '\0';
    va_end(ap);
    return written;
}

// ============================================================
// Startup/Cleanup
// ============================================================

extern char** environ;

static void initstdio(void) {
    // Standard file descriptors are inherited from kernel
    // stdin = fd 0, stdout = fd 1, stderr = fd 2
}

static void cleanup(void) {
    // Cleanup hooks would go here
}

// ============================================================
// Entry Point
// ============================================================

extern int main(int argc, char* argv[], char* envp[]);

void _start(void) {
    // Get arguments from kernel stack (simplified)
    int argc = 0;
    char** argv = (char**)0;
    char** envp = (char**)0;

    initstdio();

    int ret = main(argc, argv, envp);

    cleanup();

    exit(ret);
}

#ifdef __cplusplus
}
#endif
