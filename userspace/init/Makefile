CC = clang
LD = ld.lld
CFLAGS = -target x86_64-pc-elf -O0 -Wall -Wextra -ffreestanding -fno-stack-protector -fno-pie -mno-red-zone -mcmodel=large -fno-builtin
LDFLAGS = -nostdlib -static --no-dynamic-linker -T linker.ld -z max-page-size=0x1000 -N

all: init_bin.h

init.o: main.c
	$(CC) $(CFLAGS) -c main.c -o init.o

init.elf: init.o
	$(LD) $(LDFLAGS) init.o -o init.elf

init_bin.h: init.elf
	xxd -i init.elf > init_bin.h

clean:
	rm -f init.o init.elf init_bin.h
