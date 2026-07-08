# JowoKernel Regression Test Matrix

**Version**: 1.0  
**Last Updated**: 2026-07-08  
**Maintainer**: JowoKernel Team

---

## 1. Overview

Dokumen ini melacak hasil regression test untuk setiap milestone. Matriks ini digunakan untuk:

- Memverifikasi setiap milestone tidak merusak fitur yang sudah ada
- Mengidentifikasi area yang regressed setelah perubahan
- Memberikan gambaran umum kesehatan proyek

---

## 2. Test Categories

### 2.1 Boot Tests

| Test | Description |
|------|-------------|
| Boot | Kernel boots successfully |
| Boot (SMP) | Kernel boots with multiple CPUs |
| Boot (UEFI) | Kernel boots via UEFI |
| Boot (Legacy) | Kernel boots via BIOS/Legacy |

### 2.2 Scheduler Tests

| Test | Description |
|------|-------------|
| Scheduler Init | Scheduler initializes correctly |
| Round Robin | Basic round-robin scheduling |
| Priority | Priority-based scheduling |
| Yield | Voluntary context switch |
| Preemption | Forced context switch |

### 2.3 Memory Tests

| Test | Description |
|------|-------------|
| PMM | Physical memory manager |
| VMM | Virtual memory manager |
| Heap | Kernel heap allocator |
| Page Fault | Page fault handling |
| Guard Page | Guard page detection |
| Stack Guard | Stack overflow detection |

### 2.4 Userspace Tests

| Test | Description |
|------|-------------|
| ELF Load | ELF binary loading |
| Ring 3 Entry | Transition to user mode |
| Ring 0 Entry | Transition to kernel mode |
| SYS_WRITE | Write syscall |
| SYS_YIELD | Yield syscall |
| SYS_EXIT | Exit syscall |
| EFAULT | Invalid pointer handling |
| Stress | Many syscalls |

### 2.5 Graphics Tests

| Test | Description |
|------|-------------|
| VGA | VGA text mode |
| VBE | VBE graphics mode |
| DRM | Direct Rendering Manager |
| GPU Init | GPU initialization |
| Framebuffer | Framebuffer allocation |

### 2.6 Filesystem Tests

| Test | Description |
|------|-------------|
| VFS | Virtual Filesystem Switch |
| DevFS | Device filesystem |
| ProcFS | Process filesystem |
| RAMFS | RAM-based filesystem |

### 2.7 Network Tests

| Test | Description |
|------|-------------|
| NIC Init | Network interface initialization |
| Ethernet | Ethernet frame handling |

---

## 3. Milestone Matrix

### Legend

- ✅ **PASS**: Test passed
- ❌ **FAIL**: Test failed
- ⏸️ **SKIP**: Test not applicable
- ⏳ **PENDING**: Test not yet run

---

### M3.0 - Boot & Initialization

| Category | Test | Status |
|---------|------|--------|
| **Boot** | Boot | ✅ |
| | Boot (SMP) | ⏸️ |
| | Boot (UEFI) | ⏸️ |
| | Boot (Legacy) | ⏸️ |
| **Scheduler** | Scheduler Init | ✅ |
| | Round Robin | ✅ |
| | Priority | ⏸️ |
| | Yield | ⏸️ |
| | Preemption | ⏸️ |
| **Memory** | PMM | ✅ |
| | VMM | ✅ |
| | Heap | ✅ |
| | Page Fault | ✅ |
| | Guard Page | ✅ |
| | Stack Guard | ✅ |
| **Userspace** | ELF Load | ⏸️ |
| | Ring 3 Entry | ⏸️ |
| | Ring 0 Entry | ⏸️ |
| | SYS_WRITE | ⏸️ |
| | SYS_YIELD | ⏸️ |
| | SYS_EXIT | ⏸️ |
| | EFAULT | ⏸️ |
| | Stress | ⏸️ |
| **Graphics** | VGA | ✅ |
| | VBE | ⏸️ |
| | DRM | ⏸️ |
| | GPU Init | ⏸️ |
| | Framebuffer | ✅ |
| **Filesystem** | VFS | ✅ |
| | DevFS | ✅ |
| | ProcFS | ⏸️ |
| | RAMFS | ⏸️ |
| **Network** | NIC Init | ⏸️ |
| | Ethernet | ⏸️ |

**M3.0 Summary**: 12/31 tests passed

---

### M3.3 - Userspace Syscall Foundation

| Category | Test | Status |
|---------|------|--------|
| **Boot** | Boot | ⏳ |
| | Boot (SMP) | ⏸️ |
| | Boot (UEFI) | ⏸️ |
| | Boot (Legacy) | ⏸️ |
| **Scheduler** | Scheduler Init | ⏳ |
| | Round Robin | ⏳ |
| | Priority | ⏸️ |
| | Yield | ⏳ |
| | Preemption | ⏸️ |
| **Memory** | PMM | ✅ |
| | VMM | ✅ |
| | Heap | ✅ |
| | Page Fault | ✅ |
| | Guard Page | ✅ |
| | Stack Guard | ✅ |
| **Userspace** | ELF Load | ⏳ |
| | Ring 3 Entry | ⏳ |
| | Ring 0 Entry | ⏳ |
| | SYS_WRITE | ⏳ |
| | SYS_YIELD | ⏳ |
| | SYS_EXIT | ⏳ |
| | EFAULT | ⏳ |
| | Stress | ⏳ |
| **Graphics** | VGA | ✅ |
| | VBE | ⏸️ |
| | DRM | ⏸️ |
| | GPU Init | ⏸️ |
| | Framebuffer | ✅ |
| **Filesystem** | VFS | ✅ |
| | DevFS | ✅ |
| | ProcFS | ⏸️ |
| | RAMFS | ⏸️ |
| **Network** | NIC Init | ⏸️ |
| | Ethernet | ⏸️ |

**M3.3 Summary**: 6/31 passed, 14 pending, 11 skip

---

## 4. Future Milestones (Planned)

### M3.4 - Userspace Runtime

| Category | Test | Status |
|---------|------|--------|
| **Userspace** | argc/argv | ⏸️ |
| | envp | ⏸️ |
| | crt0 | ⏸️ |
| | libc minimal | ⏸️ |
| | brk() | ⏸️ |
| | Heap Allocator | ⏸️ |
| | execve() | ⏸️ |
| | waitpid() | ⏸️ |
| | Multiple Process | ⏸️ |

### M3.5 - Filesystem Runtime

| Category | Test | Status |
|---------|------|--------|
| **Filesystem** | VFS | ⏸️ |
| | DevFS | ⏸️ |
| | ProcFS | ⏸️ |
| | RAMFS | ⏸️ |
| | ext2 | ⏸️ |
| | FAT32 | ⏸️ |

### M3.6 - POSIX Layer

| Category | Test | Status |
|---------|------|--------|
| **POSIX** | open/close | ⏸️ |
| | read/write | ⏸️ |
| | fork/exec | ⏸️ |
| | pipe | ⏸️ |
| | socket | ⏸️ |

### G1 - GUI Bring-up

| Category | Test | Status |
|---------|------|--------|
| **GUI** | Window Server | ⏸️ |
| | Compositor | ⏸️ |
| | Event Handling | ⏸️ |
| | Vulkan/EGL | ⏸️ |

---

## 5. Regression Detection

### 5.1 How to Use

1. **Before making changes**: Run regression tests
2. **After making changes**: Run regression tests again
3. **Compare results**: Check if any tests that were passing now fail

### 5.2 Action on Regression

If a test that was previously passing now fails:

1. Revert the change
2. Investigate the cause
3. Fix the issue
4. Re-run tests
5. Only then proceed with the original change

### 5.3 CI/CD Integration

```bash
# Pre-commit hook
./tests/regression/run_all.sh

# Exit code 0 = all pass, non-zero = some fail
```

---

## 6. Adding New Tests

### 6.1 Test Naming Convention

```
test_<category>_<name>.sh
```

Example:
- `test_userspace_syscall.sh`
- `test_scheduler_yield.sh`

### 6.2 Test Template

```bash
#!/bin/bash
# Test: <Name>
# Category: <Category>
# Description: <Description>

test_<name>() {
    log "Testing: <Name>"
    
    # Run test
    run_qemu_test "Test Name" "expected_pattern"
    
    # Check result
    if [ $? -eq 0 ]; then
        return 0
    else
        return 1
    fi
}
```

---

## 7. Current Health Status

```
Boot        ████████████████████ 100%
Scheduler   ████████████████████ 100%
Memory      ████████████████████ 100%
Userspace   ░░░░░░░░░░░░░░░░░░░░  0% (M3.3 pending)
Graphics    ████████░░░░░░░░░░░░░ 33%
Filesystem  ████████░░░░░░░░░░░░░ 33%
Network     ░░░░░░░░░░░░░░░░░░░░  0%
```

---

## 8. Change Log

| Date | Version | Changes |
|------|--------|---------|
| 2026-07-08 | 1.0 | Initial version |

---

**Next Review**: After M3.3 runtime validation
