# KLAT OS Integration Tests

**Purpose: Buktikan semua layer bekerja bersama.**

---

## Struktur

```
tests/
├── boot/           # Kernel boot tests
├── runtime/         # TypeScript Runtime tests
├── kernel/         # Kernel syscall tests
├── filesystem/     # VFS tests
├── process/        # Process management tests
├── window/         # Window/compositor tests
└── browser/        # WebKit tests
```

---

## M1: Boot Tests

### Boot Sequence

```
GRUB → Kernel → Memory → Scheduler → Init → Shell
```

### Test Cases

| Test | Expected | Status |
|------|----------|--------|
| test_kernel_panic | No panic | 🔴 |
| test_memory_alloc | Allocate 4KB | 🔴 |
| test_scheduler_yield | Thread switch | 🔴 |
| test_init_spawn | Init runs | 🔴 |
| test_shell_input | Keyboard input | 🔴 |
| test_syscall_basic | read/write/open | 🔴 |

### Running Boot Tests

```bash
# Build kernel
make kernel

# Create initramfs
cd userspace && ./create_initramfs.sh

# Test in QEMU
./scripts/run_qemu.sh
```

### Manual Validation Checklist

```
[ ] Kernel prints "JowoKernel" banner
[ ] No panic or triple fault
[ ] "Init" process starts
[ ] Shell prompt appears
[ ] "help" command works
[ ] "exit" command works
```

---

## M2: Runtime Tests

### Test Runner

```bash
cd klat-desktop
npm test -- --filter integration
```

### Test Cases

| Test | Provider | Expected |
|------|----------|----------|
| `process.test.ts` | NativeProcessProvider | PID returned |
| `filesystem.test.ts` | NativeFilesystemProvider | File opened |
| `memory.test.ts` | NativeMemoryProvider | Buffer allocated |
| `ipc.test.ts` | NativeIpcProvider | Channel created |

### CI Integration

```yaml
# .github/workflows/runtime-test.yml
name: Runtime Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '20'
      - run: npm install
      - run: npm test -- --filter integration
```

---

## M3: Kernel Syscall Tests

### Test Cases

| Test | Syscall | Expected |
|------|---------|----------|
| `sys_read.c` | read() | Bytes read |
| `sys_write.c` | write() | Bytes written |
| `sys_open.c` | open() | FD returned |
| `sys_close.c` | close() | 0 success |
| `sys_mmap.c` | mmap() | Address returned |
| `sys_getpid.c` | getpid() | PID > 0 |

### Running Kernel Tests

```bash
cd tests/kernel
make
./test_syscalls
```

---

## M4: Filesystem Tests

### Test Cases

| Test | Operation | Expected |
|------|-----------|----------|
| `vfs_open.c` | open("/test") | FD ≥ 0 |
| `vfs_read.c` | read(fd) | Data received |
| `vfs_write.c` | write(fd) | Bytes written |
| `vfs_stat.c` | stat("/test") | struct filled |

---

## M5: Process Tests

### Test Cases

| Test | Operation | Expected |
|------|-----------|----------|
| `process_fork.c` | fork() | Child PID |
| `process_exec.c` | execve() | New program |
| `process_wait.c` | wait4() | Exit status |

---

## M6: Window Tests

### Test Cases

| Test | Operation | Expected |
|------|-----------|----------|
| `window_create.c` | CreateWindow() | Window ID |
| `window_move.c` | SetPosition() | Window moves |
| `window_resize.c` | SetSize() | Window resizes |

---

## M7: Browser Tests

### Test Cases

| Test | Component | Expected |
|------|----------|----------|
| `webkit_load.c` | WebKit | Page renders |
| `mesa_init.c` | Mesa | Context created |
| `gpu_render.c` | GPU | Frame drawn |

---

## Test Results Format

```json
{
  "timestamp": "2026-07-07T12:00:00Z",
  "milestone": "M1",
  "tests": [
    {
      "name": "test_kernel_boot",
      "passed": true,
      "duration_ms": 1234
    },
    {
      "name": "test_init_spawn",
      "passed": false,
      "error": "Init not found in initramfs"
    }
  ],
  "summary": {
    "total": 6,
    "passed": 5,
    "failed": 1
  }
}
```

---

## Progress Tracking

| Milestone | Tests | Passed | Failed | Status |
|-----------|-------|--------|--------|--------|
| M1 Boot | 6 | 0 | 0 | 🔴 TODO |
| M2 Runtime | 4 | 0 | 0 | 🔴 TODO |
| M3 Desktop | 3 | 0 | 0 | ⚪ PLANNING |
| M4 Browser | 3 | 0 | 0 | ⚪ PLANNING |
| M5 Process | 3 | 0 | 0 | ⚪ PLANNING |
| M6 Window | 3 | 0 | 0 | ⚪ PLANNING |

---

## Continuous Integration

```yaml
# .github/workflows/ci.yml
name: KLAT OS CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  boot-test:
    name: QEMU Boot Test (M1)
    runs-on: self-hosted
    steps:
      - name: Build Kernel
        run: make kernel
      
      - name: Create Initramfs
        run: cd userspace && ./create_initramfs.sh
      
      - name: Run QEMU
        run: ./scripts/run_qemu.sh --headless
        timeout-minutes: 5
      
      - name: Check Boot Log
        run: |
          if grep -q "JowoKernel" boot.log; then
            echo "✅ Kernel boot OK"
          else
            echo "❌ Kernel boot FAIL"
            exit 1
          fi

  runtime-test:
    name: Runtime Integration (M2)
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm install
      - run: npm test -- --filter integration
```

---

## Running All Tests

```bash
#!/bin/bash
# run_all_tests.sh

echo "=========================================="
echo "   KLAT OS Integration Test Suite"
echo "=========================================="

# M1: Boot Test
echo "[M1] Boot Test..."
make kernel 2>&1 | tee tests/boot/boot.log
if grep -q "JowoKernel" tests/boot/boot.log; then
    echo "✅ M1 Boot: PASS"
else
    echo "❌ M1 Boot: FAIL"
fi

# M2: Runtime Test
echo "[M2] Runtime Test..."
cd klat-desktop && npm test -- --filter integration
if [ $? -eq 0 ]; then
    echo "✅ M2 Runtime: PASS"
else
    echo "❌ M2 Runtime: FAIL"
fi

# M3: Kernel Test
echo "[M3] Kernel Test..."
cd tests/kernel && make && ./test_syscalls
if [ $? -eq 0 ]; then
    echo "✅ M3 Kernel: PASS"
else
    echo "❌ M3 Kernel: FAIL"
fi

echo "=========================================="
echo "   Test Suite Complete"
echo "=========================================="
```

---

*Last Updated: 2026-07-07*
*Status: M1 - QEMU Bring-up is priority*
