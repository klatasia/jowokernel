# JowoKernel Architecture Overview

**Version**: 1.0  
**Last Updated**: 2026-07-08

---

## 1. System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     USERPACE LAYER                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │  Apps   │  │  Shell   │  │  libC   │  │ Widgets │           │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘           │
│       │            │            │            │                 │
│       └────────────┴────────────┴────────────┘                 │
│                          │                                       │
│                    System Calls                                   │
├─────────────────────────┼───────────────────────────────────────┤
│                    KERNEL LAYER                                  │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │Process  │  │Scheduler│  │  VFS    │  │ Memory  │           │
│  │Manager  │  │         │  │         │  │Manager  │           │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘           │
│                          │                                       │
├─────────────────────────┼───────────────────────────────────────┤
│                    HARDWARE LAYER                                │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │   CPU   │  │   RAM   │  │   GPU   │  │   I/O   │           │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘           │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Kernel Architecture

### 2.1 Core Components

```
kernel/
├── arch/x86/           # Architecture-specific code
│   ├── interrupts.S    # ISR, swapgs
│   ├── gdt.cpp        # Global Descriptor Table
│   ├── idt.cpp        # Interrupt Descriptor Table
│   ├── paging.cpp      # Page tables
│   └── smp.cpp        # Symmetric Multi-Processing
│
├── kernel/             # Core subsystems
│   ├── scheduler.cpp   # CPU scheduler
│   ├── process.cpp     # Process management
│   ├── spinlock.cpp   # Synchronization
│   └── wait_queue.cpp # Wait queues
│
├── vm/                 # Virtual memory
│   ├── pmm.cpp         # Physical memory manager
│   ├── vmm.cpp         # Virtual memory manager
│   ├── kheap.cpp       # Kernel heap
│   └── vmo.cpp         # Virtual Memory Object
│
├── fs/                 # Filesystem
│   └── vfs.cpp         # Virtual Filesystem Switch
│
├── syscalls/           # System call interface
│   └── syscalls.cpp    # Syscall handlers
│
└── subsystems/         # Modular subsystems (M1-M7)
    ├── 01_boot/        # Boot manager
    ├── 04_sched/       # Scheduler manager
    ├── 24_userspace/   # Userspace manager
    └── ...
```

### 2.2 Process Model

```
Process
├── PID (unique identifier)
├── Page Table (address space)
├── Handle Table (file descriptors)
├── Thread List
└── State (INITIAL, RUNNING, ZOMBIE, DEAD)

Thread
├── TID (unique identifier)
├── Stack (kernel + user stack)
├── State (INITIAL, RUNNING, READY, BLOCKED, TERMINATED)
├── Registers (CPU context)
└── FPU State
```

---

## 3. Runtime Architecture

### 3.1 Startup Sequence

```
Bootloader → Kernel → Init Process → Shell → Applications
                              ↓
                        Userspace Runtime
                              ↓
                    crt0 → main()
```

### 3.2 Runtime Components

```
runtime/
├── crt0/              # C Runtime Startup
│   ├── crt0.S         # Assembly startup
│   ├── start.S         # Entry point
│   └── init.c          # C initialization
│
├── libc/               # C Standard Library
│   ├── string/        # String functions
│   ├── stdio/         # I/O functions
│   ├── stdlib/        # Utility functions
│   ├── unistd/        # POSIX functions
│   └── assert/        # Assertions
│
├── libdl/              # Dynamic Linker
│   ├── ld.so          # Runtime linker
│   └── dlfcn.c        # Dynamic loading
│
└── libpthread/         # POSIX Threads
    └── pthread.c       # Thread implementation
```

---

## 4. GUI Architecture

### 4.1 Window System

```
┌─────────────────────────────────────────────────────────────┐
│                      klat-desktop                            │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                    klat-wm                             │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐            │  │
│  │  │ Window   │  │ Window   │  │ Window   │            │  │
│  │  │ Manager  │  │ List     │  │ Stack    │            │  │
│  │  └─────────┘  └─────────┘  └─────────┘            │  │
│  └───────────────────────────────────────────────────────┘  │
│                           ↓                                  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                    Compositor                          │  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐            │  │
│  │  │ Buffer  │  │ Render   │  │ Present │            │  │
│  │  │ Manager │  │ Engine   │  │ Queue   │            │  │
│  │  └─────────┘  └─────────┘  └─────────┘            │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                      Graphics Driver                         │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐      │
│  │   DRM   │  │ Vulkan  │  │   EGL   │  │  GEM    │      │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘      │
└─────────────────────────────────────────────────────────────┘
```

---

## 5. Memory Layout

### 5.1 Virtual Address Space (x86_64)

```
0x0000000000000000  ┌─────────────────────────────────────┐
                    │                                     │
                    │         User Space (128TB)          │
                    │                                     │
                    │  0x0000000000400000  Code (.text)   │
                    │  0x0000000000600000  Data (.data)  │
                    │  0x00000000007F0000  Heap (brk)    │
                    │                                     │
                    │  0x00007FFFFFFFFFFF  Stack          │
                    │                                     │
0x00007FFF80000000  ├─────────────────────────────────────┤
                    │                                     │
                    │         Kernel Space (128TB)        │
                    │                                     │
                    │  0xFFFF800000000000  Kernel Image   │
                    │                                     │
                    │  0xFFFFFFFF80000000  Kernel Heap     │
                    │                                     │
                    │  0xFFFFFFFFFFFFFFFF  I/O Space      │
                    └─────────────────────────────────────┘
```

---

## 6. System Call Interface

### 6.1 Syscall Flow

```
User Space                    Kernel Space
   │                              │
   │ syscall  SYS_WRITE           │
   │ ────────────────────────────►│
   │                              │
   │                         1. swapgs (load kernel GS)
   │                         2. Save user RSP → GS:0
   │                         3. Load kernel RSP ← GS:8
   │                         4. Push registers
   │                         5. call handle_syscall()
   │                              │
   │                         6. copy_from_user()
   │                         7. Dispatch to handler
   │                         8. Return to userspace
   │                              │
   │     return = bytes_written   │
   │ ◄─────────────────────────────│
   │                              │
   │                         1. pop registers
   │                         2. swapgs (restore user GS)
   │                         3. sysretq
```

---

## 7. Filesystem Hierarchy

```
/                   # Root
├── bin/            # Executables
├── lib/            # Libraries
├── etc/            # Configuration
├── var/            # Variable data
├── tmp/            # Temporary files
├── home/           # User home directories
│   └── user/
└── dev/            # Device files
    ├── tty/
    ├── null/
    └── zero/
```

---

## 8. Boot Sequence

```
1. BIOS/UEFI POST
        ↓
2. Bootloader (GRUB)
        ↓
3. Multiboot Header → Kernel Entry
        ↓
4. Early Setup (GDT, IDT, Stack)
        ↓
5. Physical Memory Detection
        ↓
6. Paging Enable (MMU)
        ↓
7. Kernel Main (kmain)
        ↓
8. Subsystem Initialization
   ├── Boot Manager
   ├── CPU Manager
   ├── Memory Manager
   ├── Scheduler
   ├── Interrupt Manager
   └── Filesystem Manager
        ↓
9. Init Process (PID 1)
        ↓
10. Shell / Userspace
```

---

## 9. Related Documents

- [ABI Specification](../ABI.md)
- [Syscall Documentation](../abi/syscall.md)
- [ELF Format](../abi/elf.md)
- [Process ABI](../abi/process.md)
- [Kernel Milestones](../milestones/README.md)

---

**Maintainer**: JowoKernel Team
