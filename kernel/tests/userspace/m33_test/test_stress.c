// test_stress.c
// M3.3 Stress Test - Many consecutive syscalls

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
    const char* msg_start = "[M3.3] Starting stress test (1000 syscalls)...\n";
    const char* msg_done = "[M3.3] Stress test complete!\n";
    const char* msg_iter = "[M3.3] Iter: ";

    syscall3(1, 1, (u64)msg_start, 43);

    // Stress test: 1000 consecutive write syscalls
    for (int i = 0; i < 1000; i++) {
        // Just do yield to stress the syscall path
        asm volatile ("syscall" : : "a"(24) : "rcx", "r11", "memory");
    }

    syscall3(1, 1, (u64)msg_done, 30);

    // Exit
    asm volatile ("syscall" : : "a"(60), "D"(0) : "rcx", "r11", "memory");

    while(1) {}
}
