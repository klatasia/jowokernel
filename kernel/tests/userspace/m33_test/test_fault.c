// test_fault.c
// M3.3 EFAULT Test - Invalid pointer should return -EFAULT

typedef unsigned long long u64;

static inline u64 syscall3(u64 num, u64 arg1, u64 arg2, u64 arg3) {
    u64 ret;
    asm volatile (
        "syscall"
        : "=a"(ret)
        : "a"(num), "D"(arg1), "S"(arg2), "d"(arg3)
        : "rcx", "r11", "memory"
    );
    return ret;
}

void _start() {
    const char* msg_ok = "[M3.3] Valid write works!\n";
    const char* msg_bad = "[M3.3] EFAULT should have been returned!\n";

    // Test 1: Valid write (should succeed)
    syscall3(1, 1, (u64)msg_ok, 25);

    // Test 2: Invalid kernel pointer (should return -EFAULT = -14)
    u64 ret = syscall3(1, 1, (u64)0xFFFFFFFFFFFFFFFF, 10);

    if ((s64)ret < 0) {
        const char* msg_einval = "[M3.3] EFAULT returned correctly!\n";
        syscall3(1, 1, (u64)msg_einval, 30);
    } else {
        syscall3(1, 1, (u64)msg_bad, 34);
    }

    // Exit cleanly
    asm volatile ("syscall" : : "a"(60), "D"(0) : "rcx", "r11", "memory");

    while(1) {}
}
