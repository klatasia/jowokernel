/*
 * Copyright (c) 2026 JowoKernel Team
 * SPDX-License-Identifier: MIT
 *
 * Multiboot2 Header for GRUB compatibility
 */

.section .multiboot_header
.align 8

#define MULTIBOOT2_HEADER_MAGIC 0xE85250D6
#define MULTIBOOT_ARCHITECTURE_I386  0
#define HEADER_LENGTH (header_end - header_start)
#define CHECKSUM -(MULTIBOOT2_HEADER_MAGIC + MULTIBOOT_ARCHITECTURE_I386 + HEADER_LENGTH)

header_start:
    .long MULTIBOOT2_HEADER_MAGIC
    .long MULTIBOOT_ARCHITECTURE_I386
    .long HEADER_LENGTH
    .long CHECKSUM

    /* Tags can be added here if needed */

    /* End tag */
    .align 8
    .short 0
    .short 0
    .long  8
header_end:
