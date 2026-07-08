.global _start
_start:
    /* We are in Ring 3 now! */
    /* Infinite loop to test timer preemption */
    /* First, let's cause a delay */
    mov , %rcx
1:
    dec %rcx
    jnz 1b

    /* Now, let's intentionally access kernel memory to cause #PF */
    mov , %rax
    mov (%rax), %rbx

    /* If we get here, something is wrong, loop forever */
2:  jmp 2b
