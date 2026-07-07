// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT

#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// POSIX Signal Numbers
#define SIGHUP    1
#define SIGINT    2
#define SIGQUIT   3
#define SIGILL    4
#define SIGTRAP   5
#define SIGABRT   6
#define SIGBUS    7
#define SIGFPE    8
#define SIGKILL   9
#define SIGUSR1   10
#define SIGSEGV   11
#define SIGUSR2   12
#define SIGPIPE   13
#define SIGALRM   14
#define SIGTERM   15
#define SIGCHLD   17
#define SIGCONT   18
#define SIGSTOP   19
#define SIGTSTP   20
#define SIGTTIN   21
#define SIGTTOU   22
#define SIGURG    23
#define SIGXCPU   24
#define SIGXFSZ   25
#define SIGVTALRM 26
#define SIGPROF   27
#define SIGWINCH  28
#define SIGIO     29
#define SIGPWR    30
#define SIGSYS    31

#define SIGRTMIN  32
#define SIGRTMAX  64
#define NSIG      64

typedef void (*__sighandler_t)(int);
#define SIG_DFL ((__sighandler_t)0)
#define SIG_IGN ((__sighandler_t)1)
#define SIG_ERR ((__sighandler_t)-1)

typedef uint64_t sigset_t;

struct sigaction {
    union {
        __sighandler_t sa_handler;
        void (*sa_sigaction)(int, void*, void*);
    } __sigaction_handler;
    sigset_t sa_mask;
    int sa_flags;
    void (*sa_restorer)(void);
};

#define sa_handler   __sigaction_handler.sa_handler
#define sa_sigaction __sigaction_handler.sa_sigaction

#define SA_NOCLDSTOP  0x00000001
#define SA_NOCLDWAIT  0x00000002
#define SA_SIGINFO    0x00000004
#define SA_ONSTACK    0x08000000
#define SA_RESTART    0x10000000
#define SA_NODEFER    0x40000000
#define SA_RESETHAND  0x80000000
#define SA_RESTORER   0x04000000

// API functions
int kill(int pid, int sig);
int sigaction(int signum, const struct sigaction* act, struct sigaction* oldact);

#ifdef __cplusplus
}
#endif
