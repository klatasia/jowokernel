// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT

#include <signal.h>
#include <jowo/syscalls.h>

// Assembly trampoline yang mengembalikan eksekusi via SYS_SIGRETURN
// Ini harus dipanggil saat signal handler selesai (stack pointer menunjuk ke instruksi pertama dari fungsi ini).
extern void __restore_rt(void);
extern void __jowo_sigreturn_trampoline(void);

// Implementasi assembly trampoline di GNU C menggunakan inline assembly global
asm (
    ".text\n"
    ".global __jowo_sigreturn_trampoline\n"
    "__jowo_sigreturn_trampoline:\n"
    "    mov $15, %rax\n" // SYS_RT_SIGRETURN = 15
    "    syscall\n"
);

int kill(int pid, int sig) {
    return (int)syscall2(SYS_KILL, (uint64_t)pid, (uint64_t)sig);
}

int sigaction(int signum, const struct sigaction* act, struct sigaction* oldact) {
    struct sigaction sa;
    if (act) {
        sa = *act;
        // Jika tidak ada restorer, isikan dengan default trampoline kita
        if (!(sa.sa_flags & SA_RESTORER) || !sa.sa_restorer) {
            sa.sa_flags |= SA_RESTORER;
            sa.sa_restorer = __jowo_sigreturn_trampoline;
        }
        
        return (int)syscall3(SYS_RT_SIGACTION, (uint64_t)signum, (uint64_t)&sa, (uint64_t)oldact);
    }
    
    return (int)syscall3(SYS_RT_SIGACTION, (uint64_t)signum, 0, (uint64_t)oldact);
}
