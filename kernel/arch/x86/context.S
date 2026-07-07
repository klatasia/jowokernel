/*
 * Copyright (c) 2026 JowoKernel Team
 * SPDX-License-Identifier: MIT
 *
 * Context Switching for x86_64
 */

.section .text
.global arch_context_switch

/*
 * void arch_context_switch(Thread* old_thread, Thread* new_thread)
 * RDI = old_thread
 * RSI = new_thread
 */
arch_context_switch:
    /* Save current registers onto the stack */
    push %rbx
    push %rbp
    push %r12
    push %r13
    push %r14
    push %r15

    /* Save current stack pointer into old_thread->rsp (assuming offset 0) */
    mov %rsp, (%rdi)

    /* Load new stack pointer from new_thread->rsp (assuming offset 0) */
    mov (%rsi), %rsp

    /* Restore registers from the new stack */
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %rbp
    pop %rbx

    ret
