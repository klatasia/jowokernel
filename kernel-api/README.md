# JowoKernel API (kernel-api)

**Layer abstraksi antara Runtime dan syscall JowoKernel.**

## Tujuan

```
┌─────────────────────────────────────────────────────────┐
│                     Desktop Runtime                       │
│            (TypeScript - klat-desktop/src)              │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                   Kernel API Layer                       │
│                   (kernel-api/)                         │
│                                                          │
│   contracts/    → Interface definitions                  │
│   dispatcher/   → Syscall routing                       │
│   handles/      → Handle management                      │
│   errors/       → Error codes & messages                │
└──────────────────────────┬──────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────┐
│                    JowoKernel                            │
│                 (kernel/)                               │
│                                                          │
│   syscalls/     → Syscall handlers                      │
│   vm/           → Memory management                     │
│   fs/           → Virtual filesystem                    │
└─────────────────────────────────────────────────────────┘
```

## Struktur Direktori

```
kernel-api/
├── kernel_api.h         # Main header (include this)
├── kernel_cpp.h         # C++ wrapper classes (optional)
│
├── contracts/           # Interface definitions
│   ├── types.h         # Base types (handle_t, rights_t, signal_t, dll)
│   ├── process.h        # Process & thread API
│   ├── memory.h         # VMO & memory mapping API
│   ├── filesystem.h     # File & directory API
│   └── ipc.h           # Channel, event, timer, socket API
│
├── dispatcher/          # Syscall layer
│   ├── syscall_dispatcher.h  # Syscall numbers & wrappers
│   ├── syscall_entry.S      # x86_64 syscall assembly stubs
│   └── syscall_impl.c       # C syscall implementations
│
├── handles/             # Handle management
│   └── handles.h        # Handle operations
│
├── errors/              # Error handling
│   ├── error_codes.h    # Error code definitions (Linux + JowoKernel)
│   └── error_impl.cpp   # Error string lookup
│
└── tests/              # Unit tests
    ├── Makefile
    └── test_kernel_api.c  # C unit tests
```

**TypeScript Integration (klat-desktop):**
```
klat-desktop/src/runtime/kernel/
├── index.ts            # KernelAPI facade
├── types.ts           # TypeScript type bindings
├── syscall.ts         # Native syscall bridge
└── __tests__/
    ├── types.test.ts   # Type tests
    └── api.test.ts     # API tests
```

## Penggunaan

### C API

```c
#include <kernel-api/kernel_api.h>

// Membaca file
int fd = fs_open("/boot/init", OPEN_FLAG_READ, 0);
if (fd >= 0) {
    char buf[256];
    ssize_t n = fs_read(fd, buf, sizeof(buf));
    fs_close(fd);
}

// Membuat process
handle_t proc = process_create("myapp");
if (proc >= 0) {
    process_start(proc, entry_point, stack_pointer);
    process_exit(0);
}

// Memory mapping
uint64_t addr = vmo_map(vmo, 0, size, MMAP_PROT_READ | MMAP_PROT_WRITE,
                         MMAP_FLAG_PRIVATE, 0);
```

### C++ API (Recommended)

```cpp
#include <kernel-api/kernel_cpp.h>

using namespace kernel;

// Membaca file
auto fd = File::Open("/boot/init");
if (fd.is_valid()) {
    char buf[256];
    ssize_t n = Read(fd.get(), buf, sizeof(buf));
}

// Membuat process dengan RAII
auto proc = Process::Create("myapp");
if (proc.is_valid()) {
    Process::Start(proc, entry, stack);
}

// VMO dengan RAII
auto vmo = Vmo::Create(4096);
vmo.Write(0, data, size);
uint64_t addr = Mmap(nullptr, 4096, MMAP_PROT_READ, 
                      MMAP_FLAG_PRIVATE, vmo.get_handle());
```

### Channel IPC

```cpp
// Buat channel pair
Channel ch = Channel::Create();
if (ch.is_valid()) {
    // Kirim data
    ch.Write(buffer, size);
    
    // Terima data
    ch.Read(receive_buf, size);
}
```

## Error Handling

```cpp
// C-style
int result = fs_open("/file", OPEN_FLAG_READ, 0);
if (result < 0) {
    printf("Error: %s\n", kernel_strerror(-result));
}

// C++ style
auto status = File::Mkdir("/newdir");
if (status.is_error()) {
    printf("Error: %s\n", status.strerror());
}
```

## Compile

Tambahkan ke compile flags:
```
-Ipath/to/kernel-api -fno-exceptions -ffreestanding
```

Link dengan userspace libc:
```
-Lpath/to/userspace/libc -ljowo
```

## Status

| Component | Status |
|-----------|--------|
| Types | ✅ Done |
| Error Codes (100+) | ✅ Done |
| Process API | ✅ Done |
| Memory API | ✅ Done |
| Filesystem API | ✅ Done |
| IPC API | ✅ Done |
| Handle API | ✅ Done |
| C++ Wrappers (RAII) | ✅ Done |
| Syscall Entry (x86_64) | ✅ Done |
| Userspace Stubs | ✅ Done |
| C Unit Tests | ✅ Done |
| TypeScript Bindings | ✅ Done |
| TypeScript Tests | ✅ Done |

## TODO

- [ ] Native integration testing (QEMU)
- [ ] Performance benchmarks
- [ ] Documentation untuk setiap API

---

## Testing

### C Unit Tests
```bash
cd kernel-api/tests
make test
```

### TypeScript Tests
```bash
cd klat-desktop
npm test -- --filter kernel
```

---

*Last Updated: 2026-07-07*
*KLAT OS Team*
