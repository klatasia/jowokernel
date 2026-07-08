// test_yield.c
// M3.3 Basic Syscall test

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

static inline u64 syscall1(u64 num, u64 arg1) {
    u64 ret;
    asm volatile (
        "syscall"
        : "=a"(ret)
        : "a"(num), "D"(arg1)
        : "rcx", "r11", "memory"
    );
    return ret;
}

static inline u64 syscall0(u64 num) {
    u64 ret;
    asm volatile (
        "syscall"
        : "=a"(ret)
        : "a"(num)
        : "rcx", "r11", "memory"
    );
    return ret;
}

void _start() {
    const char* msgA = "[Test M3.3] Yield Test A\n";
    const char* msgB = "[Test M3.3] Yield Test A lagi sesudah yield!\n";
    
    // SYS_WRITE = 1
    syscall3(1, 1, (u64)msgA, 25);
    
    // SYS_YIELD = 24
    syscall0(24);
    
    // SYS_WRITE = 1
    syscall3(1, 1, (u64)msgB, 43);
    
    // SYS_EXIT = 60
    syscall1(60, 0);
    
    while(1) {}
}
