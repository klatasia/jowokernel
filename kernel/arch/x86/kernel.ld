/*
 * Copyright (c) 2026 JowoKernel Team
 * SPDX-License-Identifier: MIT
 *
 * Linker script for JowoKernel x86_64
 */

ENTRY(_start)

SECTIONS {
    /* Load the kernel at 1MB (standard for PC bootloaders like GRUB) */
    . = 1M;

    /* Multiboot Header MUST be early in the file */
    .boot : {
        KEEP(*(.multiboot_header))
        *(.text.boot)
    }

    .text : ALIGN(4K) {
        *(.text)
        *(.text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata)
        *(.rodata.*)
    }

    .data : ALIGN(4K) {
        *(.data)
        *(.data.*)
    }

    .bss : ALIGN(4K) {
        *(COMMON)
        *(.bss)
        *(.bss.*)
    }

    /* End marker */
    _end = .;
}
