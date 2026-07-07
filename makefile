# JowoKernel Root Makefile
# Copyright (c) 2026 JowoKernel Team
# SPDX-License-Identifier: MIT

ARCH ?= x86_64

# Toolchain
CC := clang
CXX := clang++
AS := clang
LD := ld.lld

BUILD_DIR := build-$(ARCH)

# Flags
CFLAGS := -O2 -Wall -Wextra -target x86_64-pc-elf -ffreestanding -mno-red-zone
CXXFLAGS := $(CFLAGS) -fno-exceptions -fno-rtti
ASFLAGS := -target x86_64-pc-elf
LDFLAGS := -nostdlib -static --no-dynamic-linker -T kernel/arch/x86/kernel.ld

# Kernel Sources
KERNEL_SRC_S := kernel/arch/x86/start.S kernel/arch/x86/context.S kernel/arch/x86/multiboot.S
KERNEL_SRC_CPP := kernel/top/lk_main.cpp kernel/vm/vm_manager.cpp kernel/object/handle.cpp \
                  kernel/arch/x86/gdt.cpp kernel/arch/x86/idt.cpp kernel/syscalls/syscalls.cpp \
                  kernel/kernel/scheduler.cpp kernel/vm/vmo.cpp kernel/object/channel.cpp \
                  kernel/kernel/spinlock.cpp kernel/vm/pmm.cpp kernel/arch/x86/paging.cpp \
                  kernel/arch/x86/apic.cpp kernel/kernel/process.cpp \
                  kernel/kernel/wait_queue.cpp kernel/arch/x86/smp.cpp \
                  kernel/dev/pci.cpp kernel/arch/x86/security.cpp kernel/object/event.cpp \
                  kernel/fs/vfs.cpp kernel/dev/device_manager.cpp
               $(patsubst %.cpp,$(BUILD_DIR)/%.o,$(KERNEL_SRC_CPP))

TARGET := $(BUILD_DIR)/jowokernel.bin

.PHONY: all clean kernel

all: kernel

kernel: $(TARGET)

$(TARGET): $(KERNEL_OBJS)
	@echo "Linking kernel..."
	@mkdir -p $(dir $@)
	$(LD) $(LDFLAGS) -o $@ $^
	@echo "Build successful: $@"

$(BUILD_DIR)/%.o: %.S
	@echo "AS $<"
	@mkdir -p $(dir $@)
	$(AS) $(ASFLAGS) -c $< -o $@

$(BUILD_DIR)/%.o: %.cpp
	@echo "CXX $<"
	@mkdir -p $(dir $@)
	$(CXX) $(CXXFLAGS) -Ipublic -c $< -o $@

clean:
	rm -rf $(BUILD_DIR)
