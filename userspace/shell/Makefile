# Shell Makefile

CXX = clang++
LD = ld.lld

CXXFLAGS = -target x86_64-pc-elf -O2 -Wall -Wextra -ffreestanding -fno-stack-protector -fno-pie -mno-red-zone -mcmodel=large -fno-builtin -fno-exceptions -fno-rtti -I../libjowo/include
LDFLAGS = -nostdlib -static --no-dynamic-linker -T linker.ld -z max-page-size=0x1000 -N

SRCS = main.cpp
OBJS = $(SRCS:.cpp=.o)

LIBJOWO_DIR = ../libjowo
LIBJOWO = $(LIBJOWO_DIR)/libjowo.a
CRT0 = $(LIBJOWO_DIR)/arch/x86/crt0.o

TARGET = shell.elf

all: $(TARGET)

$(TARGET): $(OBJS) $(CRT0) $(LIBJOWO)
	$(LD) $(LDFLAGS) $(CRT0) $(OBJS) $(LIBJOWO) -o $@
	xxd -i $@ > shell_bin.h

%.o: %.cpp
	$(CXX) $(CXXFLAGS) -c $< -o $@

clean:
	rm -f $(OBJS) $(TARGET) shell_bin.h
