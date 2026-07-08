// test_yield2.c
// M3.3 Yield Stress Test - Alternating threads

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
    const char* msg_a = "[M3.3-T1] Thread A running\n";
    const char* msg_b = "[M3.3-T1] Thread B should be running now\n";
    const char* msg_done = "[M3.3-T1] Yield stress complete!\n";

    // Initial message
    syscall3(1, 1, (u64)msg_a, 24);

    // 100 cycles of yield
    for (int i = 0; i < 100; i++) {
        asm volatile ("syscall" : : "a"(24) : "rcx", "r11", "memory");
    }

    // Final message
    syscall3(1, 1, (u64)msg_b, 33);
    syscall3(1, 1, (u64)msg_done, 29);

    // Exit
    asm volatile ("syscall" : : "a"(60), "D"(0) : "rcx", "r11", "memory");

    while(1) {}
}
