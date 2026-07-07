    .section .text
    .global _start
    .type _start, @function

_start:
    /* Clear base pointer for backtrace to stop here */
    xor %rbp, %rbp
    
    /* Ensure stack is 16-byte aligned before calling C/C++ code.
       Currently it might be aligned or not depending on kernel entry. */
    and $~15, %rsp

    /* Clear BSS */
    lea __bss_start(%rip), %rdi
    lea __bss_end(%rip), %rcx
    sub %rdi, %rcx
    xor %eax, %eax
    rep stosb

    /* Call global constructors (C++) */
    call _init_array

    /* Call main(argc, argv) */
    /* Currently Kernel doesn't pass argc/argv, so pass 0/NULL */
    xor %edi, %edi      /* argc = 0 */
    xor %esi, %esi      /* argv = NULL */
    call main

    /* Save return value of main */
    mov %eax, %edi

    /* Call global destructors (if any) */
    call _fini_array

    /* Call sys_exit(return_value) */
    call sys_exit

    /* Should never reach here, but just in case */
1:  hlt
    jmp 1b

/* Helper functions to call init/fini arrays */
    .type _init_array, @function
_init_array:
    push %rbp
    mov %rsp, %rbp
    
    lea __init_array_start(%rip), %rbx
    lea __init_array_end(%rip), %r12
1:
    cmp %r12, %rbx
    je 2f
    call *(%rbx)
    add $8, %rbx
    jmp 1b
2:
    pop %rbp
    ret

    .type _fini_array, @function
_fini_array:
    push %rbp
    mov %rsp, %rbp
    
    lea __fini_array_start(%rip), %rbx
    lea __fini_array_end(%rip), %r12
1:
    cmp %r12, %rbx
    je 2f
    call *(%rbx)
    add $8, %rbx
    jmp 1b
2:
    pop %rbp
    ret
