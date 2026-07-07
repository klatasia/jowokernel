# KLAT OS MILESTONES

> **Fokus: Buktikan bahwa semua layer bekerja bersama, bukan tambah layer baru.**

---

# 🚨 ATURAN UTAMA

## ATURAN: M1 → M2

```
M1 harus 100% sebelum M2 dimulai.

M1 100%
    ↓
    baru M2
```

**Tidak ada kompromi.**

## ATURAN: Regression Rule

```
Gate yang PASS tidak boleh rusak.

M1.1 ✅ → Fix M1.2 → Retest M1.1, M1.2 → M1.1 harus tetap ✅
```

## ATURAN: Build Freeze

```
Selama M1:
- ❌ Jangan tambah fitur
- ❌ Jangan ubah GUI
- ❌ Jangan ubah Runtime API
- ✅ Hanya fix bug yang blockers M1
```

---

# MILESTONE M1 — QEMU Bring-up 🔴 PRIORITAS

## Gate Structure

```
M1.1 Bootloader    [ ]
    ↓
M1.2 Kernel Entry [ ]
    ↓
M1.3 Memory Mgr   [ ]
    ↓
M1.4 Scheduler    [ ]
    ↓
M1.5 Init Process [ ]
    ↓
M1.6 Shell        [ ]
    ↓
M1.7 Spawn Process[ ]
    ↓
M1.8 Filesystem RW[ ]
    ↓
M1.9 Shutdown     [ ]
```

## M1 Gates

### M1.1 Bootloader
**Target:** GRUB → Kernel Entry
- [ ] GRUB memuat kernel.bin
- [ ] Multiboot header valid
- [ ] Kernel entry point ditemukan
- [ ] Boot info diteruskan ke kernel

### M1.2 Kernel Entry
**Target:** Kernel Entry → Main initialization
- [ ] `_start` dieksekusi
- [ ] GDT diinisialisasi
- [ ] IDT diinisialisasi
- [ ] Serial output berfungsi
- [ ] Banner "JowoKernel" tercetak

### M1.3 Memory Manager
**Target:** Memory Manager → PMM initialized
- [ ] Physical Memory Manager initialized
- [ ] Memory map diparse
- [ ] Page allocator berfungsi
- [ ] test_vmm() PASS

### M1.4 Scheduler
**Target:** Scheduler → Multitasking enabled
- [ ] Scheduler initialized
- [ ] Idle thread berjalan
- [ ] Thread switching berfungsi
- [ ] APIC timer berjalan

### M1.5 Init Process
**Target:** Init Process → /bin/init running
- [ ] init.elf dimuat dari initramfs
- [ ] ELF loader parse header
- [ ] User page table diciptakan
- [ ] Stack userspace dialokasikan
- [ ] enter_user_mode() dipanggil
- [ ] Kembali ke kernel jika exit

### M1.6 Shell
**Target:** Shell → Interactive shell ready
- [ ] Shell prompt muncul
- [ ] Keyboard interrupt handler berfungsi
- [ ] `help` bekerja
- [ ] `echo` bekerja
- [ ] `clear` bekerja
- [ ] `version` bekerja
- [ ] `exit` kembali ke init

### M1.7 Spawn Process
**Target:** Spawn Process → child process created
- [ ] fork() syscall berfungsi
- [ ] Child process memiliki page table sendiri
- [ ] File descriptors di-duplicate
- [ ] wait4() mengembalikan exit status

### M1.8 Filesystem RW
**Target:** Filesystem → open/read/write/mkdir/rename/delete
- [ ] VFS initialized
- [ ] Root filesystem mounted
- [ ] open() berfungsi
- [ ] read() berfungsi
- [ ] write() berfungsi
- [ ] mkdir() berfungsi
- [ ] rename() berfungsi
- [ ] unlink() berfungsi

### M1.9 Shutdown
**Target:** Shutdown → Clean system shutdown
- [ ] Signal handling berfungsi
- [ ] Exit syscall membersihkan resources
- [ ] Memory di-freed
- [ ] Serial output "Shutting down..."
- [ ] QEMU exit gracefully

## Quick Test Command

```bash
# Build kernel
make kernel

# Create initramfs
cd userspace && ./create_initramfs.sh

# Test boot
./scripts/run_qemu.sh

# Check output
grep -E "JowoKernel|Init|Shell" boot.log
```

## Status Overview

| Gate | Status | Date | Notes |
|------|--------|------|-------|
| M1.1 Bootloader | ⚪ | - | |
| M1.2 Kernel Entry | ⚪ | - | |
| M1.3 Memory Manager | ⚪ | - | |
| M1.4 Scheduler | ⚪ | - | |
| M1.5 Init Process | ⚪ | - | |
| M1.6 Shell | ⚪ | - | |
| M1.7 Spawn Process | ⚪ | - | |
| M1.8 Filesystem RW | ⚪ | - | |
| M1.9 Shutdown | ⚪ | - | |

**M1 Progress:** 0%

## Detailed Logs

Lihat `docs/bringup/` untuk log per gate.

---

# MILESTONE M2 — Kernel Validation 🔴

## ATURAN: M2 belum boleh dimulai sebelum M1 100%

## Alasan

Setelah shell berhasil, lakukan **kernel validation** sebelum mulai runtime integration:
- Stress test scheduler
- Memory leak detection
- VFS stress test
- IPC stress test
- Syscall validation

Ini mencegah bug sulit dilacak yang sebenarnya berasal dari kernel.

## Target

```
JowoKernel
    ↓
Stress Test
    ↓
Scheduler Test
    ↓
Memory Test
    ↓
VFS Test
    ↓
Syscall Test
    ↓
IPC Test
```

## Gates

- [ ] M2.1 Scheduler stress test
- [ ] M2.2 Memory leak detection
- [ ] M2.3 VFS stress test
- [ ] M2.4 Syscall validation
- [ ] M2.5 IPC stress test

## Status Overview

| Gate | Status |
|------|--------|
| M2.1 Scheduler stress | ⚪ |
| M2.2 Memory leak | ⚪ |
| M2.3 VFS stress | ⚪ |
| M2.4 Syscall validation | ⚪ |
| M2.5 IPC stress | ⚪ |

**M2 Progress:** 0%

---

# MILESTONE M3 — Runtime Native 🔴

## ATURAN: M3 belum boleh dimulai sebelum M2 100%

## Target

```
TypeScript Runtime
    ↓
Kernel API
    ↓
Native Provider
    ↓
JowoKernel
```

## Test App

```typescript
Runtime Test App
    ↓
create process
open file
read file
write file
```

## Gates

- [ ] M3.1 Runtime → Kernel API connection
- [ ] M3.2 NativeProcessProvider.test()
- [ ] M3.3 NativeFilesystemProvider.test()
- [ ] M3.4 NativeMemoryProvider.test()
- [ ] M3.5 NativeIpcProvider.test()

## Status Overview

| Gate | Status |
|------|--------|
| M3.1 Runtime-Kernel connection | ⚪ |
| M3.2 Process Provider | ⚪ |
| M3.3 Filesystem Provider | ⚪ |
| M3.4 Memory Provider | ⚪ |
| M3.5 IPC Provider | ⚪ |

**M3 Progress:** 0%

---

# MILESTONE M4 — Desktop Bring-up 🟡

## ATURAN: M4 belum boleh dimulai sebelum M3 100%

## Target

```
klat-desktop (TIDAK BERUBAH)
    ↓
Window Runtime
    ↓
Kernel API
    ↓
Compositor
```

## Gates

- [ ] M4.1 klat-desktop build berhasil
- [ ] M4.2 Desktop Shell muncul di QEMU
- [ ] M4.3 Window bisa di-drag
- [ ] M4.4 Taskbar berfungsi
- [ ] M4.5 Tampilan sama dengan browser

## Status Overview

| Gate | Status |
|------|--------|
| M4.1 Desktop build | ⚪ |
| M4.2 Desktop appears | ⚪ |
| M4.3 Window drag | ⚪ |
| M4.4 Taskbar | ⚪ |
| M4.5 Same as browser | ⚪ |

**M4 Progress:** 0%

---

# MILESTONE M5 — Browser 🟡

## Target

```
Browser Runtime
    ↓
WebKit
    ↓
Mesa
    ↓
GPU
```

## Gates

- [ ] M4.1 WebKit build untuk KLAT OS
- [ ] M4.2 WebKit Provider
- [ ] M4.3 Mesa integration
- [ ] M4.4 GPU acceleration
- [ ] M4.5 iframe dihapus

## Status Overview

| Gate | Status |
|------|--------|
| M4.1 WebKit build | ⚪ |
| M4.2 WebKit Provider | ⚪ |
| M4.3 Mesa | ⚪ |
| M4.4 GPU | ⚪ |
| M4.5 iframe removed | ⚪ |

**M4 Progress:** 0%

---

# PROGRESS SUMMARY

```
M1: QEMU Bring-up     [          ] 0%
M2: Kernel Validation [          ] 0%
M3: Runtime Native    [          ] 0%
M4: Desktop Bring-up  [          ] 0%
M5: Browser          [          ] 0%
```

| Milestone | Gates | Passed | Failed |
|-----------|-------|--------|--------|
| M1 | 9 | 0 | 0 |
| M2 | 5 | 0 | 0 |
| M3 | 5 | 0 | 0 |
| M4 | 5 | 0 | 0 |
| **Total** | **24** | **0** | **0** |

---

# IMPLEMENTATION STATUS

```
IMPLEMENTASI: ✅ SELESAI
├── Kernel: ✅
├── Runtime: ✅
├── Providers: ✅
└── Userspace: ✅

VALIDASI: 🔴 PRIORITAS
├── M1 QEMU Bring-up: 🔴 START
├── M2 Runtime: 🔴 WAIT M1
└── M3 Desktop: 🟡 WAIT M2
```

---

# NEXT ACTION

1. **Run M1.1 boot test**
```bash
make kernel
./scripts/run_qemu.sh
```

2. **Check output**
```bash
grep "JowoKernel" boot.log
```

3. **Update docs/bringup/M1.1-bootloader.md**

---

*Updated: 2026-07-07*
*Focus: M1.1 → M1.9 (100% sebelum M2)*
