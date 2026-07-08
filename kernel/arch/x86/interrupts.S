/*
 * Copyright (c) 2026 JowoKernel Team
 * SPDX-License-Identifier: MIT
 *
 * ISR (Interrupt Service Routine) Stubs for x86_64
 */

.section .text
.code64

/* Macros for defining ISRs */
.macro ISR_NOERRCODE num
.global isr\num
isr\num:
    pushq $0            /* Push dummy error code */
    pushq $\num         /* Push interrupt vector */
    jmp isr_common
.endm

.macro ISR_ERRCODE num
.global isr\num
isr\num:
    /* Error code is already pushed by CPU */
    pushq $\num         /* Push interrupt vector */
    jmp isr_common
.endm

/* Define ISRs for Exceptions */
ISR_NOERRCODE 0   /* Divide by Zero */
ISR_NOERRCODE 1   /* Debug */
ISR_NOERRCODE 2   /* Non-maskable Interrupt */
ISR_NOERRCODE 3   /* Breakpoint */
ISR_NOERRCODE 4   /* Overflow */
ISR_NOERRCODE 5   /* Bound Range Exceeded */
ISR_NOERRCODE 6   /* Invalid Opcode */
ISR_NOERRCODE 7   /* Device Not Available */
ISR_ERRCODE   8   /* Double Fault */
ISR_NOERRCODE 9   /* Coprocessor Segment Overrun */
ISR_ERRCODE   10  /* Invalid TSS */
ISR_ERRCODE   11  /* Segment Not Present */
ISR_ERRCODE   12  /* Stack-Segment Fault */
ISR_ERRCODE   13  /* General Protection Fault */
ISR_ERRCODE   14  /* Page Fault */
ISR_NOERRCODE 15  /* Reserved */
ISR_NOERRCODE 16  /* x87 Floating-Point Exception */
ISR_ERRCODE   17  /* Alignment Check */
ISR_NOERRCODE 18  /* Machine Check */
ISR_NOERRCODE 19  /* SIMD Floating-Point Exception */
ISR_NOERRCODE 20  /* Virtualization Exception */
ISR_ERRCODE   21  /* Control Protection Exception */
ISR_NOERRCODE 22  /* Reserved */
ISR_NOERRCODE 23  /* Reserved */
ISR_NOERRCODE 24  /* Reserved */
ISR_NOERRCODE 25  /* Reserved */
ISR_NOERRCODE 26  /* Reserved */
ISR_NOERRCODE 27  /* Reserved */
ISR_NOERRCODE 28  /* Hypervisor Injection Exception */
ISR_ERRCODE   29  /* VMM Communication Exception */
ISR_ERRCODE   30  /* Security Exception */
ISR_NOERRCODE 31  /* Reserved */

/* Define ISRs for IRQs (mapped to vector 32+) */
ISR_NOERRCODE 32  /* IRQ0: Timer */
ISR_NOERRCODE 33  /* IRQ1: Keyboard */
ISR_NOERRCODE 34  /* IRQ2 */
ISR_NOERRCODE 35  /* IRQ3 */
ISR_NOERRCODE 36  /* IRQ4 */
ISR_NOERRCODE 37  /* IRQ5 */
ISR_NOERRCODE 38  /* IRQ6 */
ISR_NOERRCODE 39  /* IRQ7 */
ISR_NOERRCODE 40  /* IRQ8 */
ISR_NOERRCODE 41  /* IRQ9 */
ISR_NOERRCODE 42  /* IRQ10 */
ISR_NOERRCODE 43  /* IRQ11 */
ISR_NOERRCODE 44  /* IRQ12 */
ISR_NOERRCODE 45  /* IRQ13 */
ISR_NOERRCODE 46  /* IRQ14 */
ISR_NOERRCODE 47  /* IRQ15 */

.extern interrupt_handler
.global isr_common

/* Offset to the CS register in the interrupt frame BEFORE pushing/AFTER popping the 15 general purpose registers.
   Layout:
     RSP + 0 : Vector
     RSP + 8 : Error Code
     RSP + 16: RIP
     RSP + 24: CS
*/
.equ CS_OFFSET_IN_BASE_FRAME, 24

isr_common:
    /* Check if interrupt came from User Mode (Ring 3) by checking the RPL of the pushed CS register.
       If it came from Ring 3, we must swapgs to load the Kernel GS Base. */
    testq $3, CS_OFFSET_IN_BASE_FRAME(%rsp)
    jz .L_kernel_interrupt
    swapgs
.L_kernel_interrupt:

    /* Save general purpose registers */
    pushq %r15
    pushq %r14
    pushq %r13
    pushq %r12
    pushq %r11
    pushq %r10
    pushq %r9
    pushq %r8
    pushq %rbp
    pushq %rdi
    pushq %rsi
    pushq %rdx
    pushq %rcx
    pushq %rbx
    pushq %rax

    /* Pass frame pointer (RSP) as first argument to interrupt_handler (RDI) */
    movq %rsp, %rdi
    
    /* Align stack to 16 bytes for System V ABI (to prevent SSE #GP) */
    test $15, %rsp
    jz 1f
    push $0
    call interrupt_handler
    add $8, %rsp
    jmp 2f
1:
    call interrupt_handler
2:

    /* Restore general purpose registers */
    popq %rax
    popq %rbx
    popq %rcx
    popq %rdx
    popq %rsi
    popq %rdi
    popq %rbp
    popq %r8
    popq %r9
    popq %r10
    popq %r11
    popq %r12
    popq %r13
    popq %r14
    popq %r15

    /* Check if we are returning to User Mode (Ring 3).
       After popping the 15 registers, RSP is back to the base frame. */
    testq $3, CS_OFFSET_IN_BASE_FRAME(%rsp)
    jz .L_return_to_kernel
    swapgs
.L_return_to_kernel:

    /* Remove vector and error code from stack */
    addq $16, %rsp
    
    /* Return from interrupt */
    iretq

.global syscall_entry
.extern handle_syscall

.section .text
syscall_entry:
    /* SWAPGS switches the GS base to the Kernel GS Base (MSR 0xC0000102) */
    swapgs
    
    /* Save user RSP to CpuSyscallState.user_rsp (offset 0) */
    movq %rsp, %gs:0
    
    /* Switch to kernel stack from CpuSyscallState.kernel_rsp (offset 8) */
    movq %gs:8, %rsp
    
    /* Build interrupt frame for saving context */
    pushq $0x1B                   /* User SS */
    pushq %gs:0                   /* User RSP */
    pushq %r11                    /* User RFLAGS */
    pushq $0x23                   /* User CS */
    pushq %rcx                    /* User RIP */
    
    /* Push general purpose registers */
    pushq %r15
    pushq %r14
    pushq %r13
    pushq %r12
    pushq %r11
    pushq %r10
    pushq %r9
    pushq %r8
    pushq %rbp
    pushq %rdi
    pushq %rsi
    pushq %rdx
    pushq %rcx
    pushq %rbx
    pushq %rax
    
    /* Shift arguments for C/C++ handle_syscall:
     * InterruptFrame* frame -> RDI (Arg 1)
     */
    movq %rsp, %rdi
    
    call handle_syscall
    
    /* Save return value (RAX) into the stack frame's saved RAX slot */
    movq %rax, (%rsp)
    
    /* Restore general purpose registers */
    popq %rax
    popq %rbx
    popq %rcx
    popq %rdx
    popq %rsi
    popq %rdi
    popq %rbp
    popq %r8
    popq %r9
    popq %r10
    popq %r11
    popq %r12
    popq %r13
    popq %r14
    popq %r15
    
    /* Load user RIP, RFLAGS, RSP to restore state */
    popq %rcx       /* User RIP */
    addq $8, %rsp   /* Skip CS */
    popq %r11       /* User RFLAGS */
    popq %rsp       /* User RSP (switches to user stack) */
    
    /* Restore User GS Base */
    swapgs
    
    sysretq

.global user_test_start
.global user_test_end
user_test_start:
    /* SYS_WRITE (2) */
    movq $2, %rax
    movq $1, %rdi
    leaq msg_user(%rip), %rsi
    movq $60, %rdx
    syscall
    
    /* SYS_YIELD (4) */
    movq $4, %rax
    syscall

.Luser_loop:
    jmp .Luser_loop

msg_user:
    .ascii "[User Mode] Sukses memanggil syscall SYS_WRITE dari Ring 3!\n"
user_test_end:

