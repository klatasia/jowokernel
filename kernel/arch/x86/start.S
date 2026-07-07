/*
 * Copyright (c) 2026 JowoKernel Team
 * SPDX-License-Identifier: MIT
 *
 * x86_64 Architecture Entry
 */

.section .text.boot
.global _start

_start:
    /* Early boot sequence for x86_64 */
    /* 1. Setup a basic stack */
    mov $0x100000, %rsp

    /* 2. Clear interrupts */
    cli

    /* 3. Call the C/C++ kernel entry point */
    call lk_main

    /* Fallback halt loop */
.Lhalt:
    hlt
    jmp .Lhalt
