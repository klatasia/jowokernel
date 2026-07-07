# libjowo Makefile

CC = clang
CXX = clang++
AR = ar

CFLAGS = -target x86_64-pc-elf -O2 -Wall -Wextra -ffreestanding -fno-stack-protector -fno-pie -mno-red-zone -mcmodel=large -fno-builtin -Iinclude
CXXFLAGS = $(CFLAGS) -fno-exceptions -fno-rtti

SRCS_C = src/sys.c src/string.c src/stdio.c src/signal.c
SRCS_CXX = src/cxx_runtime.cpp src/stdlib/malloc.cpp src/graphics/renderer_manager.cpp
SRCS_ASM = arch/x86/crt0.S

OBJS = $(SRCS_C:.c=.o) $(SRCS_CXX:.cpp=.o) $(SRCS_ASM:.S=.o)

TARGET = libjowo.a
CRT0 = arch/x86/crt0.o

all: $(TARGET) $(CRT0)

$(TARGET): $(filter-out arch/x86/crt0.o, $(OBJS))
	$(AR) rcs $@ $^

arch/x86/crt0.o: arch/x86/crt0.S
	$(CC) -target x86_64-pc-elf -c $< -o $@

%.o: %.c
	$(CC) $(CFLAGS) -c $< -o $@

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) $(TARGET)
