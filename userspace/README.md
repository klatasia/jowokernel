# KLAT OS Userspace

**Userspace programs and libraries for KLAT OS**

## Structure

```
userspace/
├── libjowo/           # Minimal C library
│   └── jowolibc.h      # Header + implementation
│
├── init/              # Init process
│   └── init.c          # First userspace process
│
├── shell/             # Shell program
│   └── shell.c         # Command-line shell
│
├── Makefile           # Build system
└── README.md          # This file
```

## Building

```bash
cd userspace
make
```

This produces:
- `../build-x86_64/init/init.elf` - Init process
- `../build-x86_64/shell/shell.elf` - Shell binary

## libjowo (JowoLibc)

A minimal C library providing:

### Syscall Wrappers
- `read()`, `write()`, `open()`, `close()`
- `exit()`, `getpid()`, `yield()`
- `fstat()`, `brk()`, `sbrk()`, `nanosleep()`

### Memory Functions
- `memset()`, `memcpy()`, `memmove()`, `memcmp()`

### String Functions
- `strlen()`, `strcpy()`, `strcmp()`, etc.

### I/O Functions
- `printf()`, `puts()`, `putchar()`
- `vprintf()`, `snprintf()`

## Init Process

The init process (`/bin/init`) is the first userspace process spawned by the kernel.

Responsibilities:
1. Initialize the system
2. Mount filesystems
3. Set hostname
4. Start the shell

## Shell

The shell provides a command-line interface with built-in commands:
- `help` - Show available commands
- `echo` - Print text
- `clear` - Clear screen
- `exit` - Exit shell
- `pwd` - Print working directory
- `ls` - List directory
- `cat` - Display file
- `ps` - Show processes
- And more...

## Boot Flow

```
Bootloader (GRUB)
    ↓
JowoKernel (kernel/)
    ↓
Init Process (/bin/init)
    ↓
Shell (/bin/shell)
    ↓
User Commands
```

## Kernel Integration

The userspace programs are compiled as ELF executables and loaded by the kernel's ELF loader:

```cpp
// kernel/subsystems/24_userspace/elf_loader.h
kernel::userspace::ElfLoader::Load(init_file, proc, &user_rsp);
```

---

*Last Updated: 2026-07-07*
*KLAT OS Team*
