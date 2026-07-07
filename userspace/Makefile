# JowoLibc - Userspace C Library for KLAT OS
# Copyright (c) 2026 KLAT OS Team
# SPDX-License-Identifier: MIT

.PHONY: all clean install

# Cross-compiler for x86_64
CC ?= clang
CFLAGS ?= -Wall -Wextra -O2 -ffreestanding -fno-stack-protector -nostdlib
LDFLAGS ?= -nostdlib -e _start

# Output directories
BUILD_DIR = ../build-x86_64
INIT_DIR = $(BUILD_DIR)/init
SHELL_DIR = $(BUILD_DIR)/shell
LIB_DIR = $(BUILD_DIR)/lib

# Source directories
USERSpace = ..
LIBJOWO_DIR = $(USERSpace)/userspace/libjowo
INIT_SRC = $(USERSpace)/userspace/init/init.c
SHELL_SRC = $(USERSpace)/userspace/shell/shell.c

# Include path
INCLUDES = -I$(LIBJOWO_DIR)

# Output files
INIT_OUT = $(INIT_DIR)/init.elf
SHELL_OUT = $(SHELL_DIR)/shell.elf

all: directories $(INIT_OUT) $(SHELL_OUT)

directories:
	@mkdir -p $(INIT_DIR) $(SHELL_DIR) $(LIB_DIR)

$(INIT_OUT): $(INIT_SRC) $(LIBJOWO_DIR)/jowolibc.h
	@echo "Building init..."
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $(INIT_SRC)

$(SHELL_OUT): $(SHELL_SRC) $(LIBJOWO_DIR)/jowolibc.h
	@echo "Building shell..."
	$(CC) $(CFLAGS) $(INCLUDES) -o $@ $(SHELL_SRC)

clean:
	rm -rf $(BUILD_DIR)

install: all
	@echo "Userspace binaries built. Copy to initramfs."
	@echo "  init: $(INIT_OUT)"
	@echo "  shell: $(SHELL_OUT)"
