# KLAT OS Project Rules

> **During M1: QEMU Bring-up**

---

# 🚨 RULE 1: BUILD FREEZE

## During M1 (QEMU Bring-up), ALL development is frozen except:

### ❌ DILARANG

- Menambah fitur baru
- Mengubah GUI `klat-desktop`
- Mengubah Runtime API
- Refactor arsitektur
- Menambah subsystem baru
- Mengubah syscall interface

### ✅ BOLEH

- Memperbaiki bug yang menghalangi gate M1
- Menambah debug output untuk diagnosis
- Memperbaiki kernel panic
- Memperbaiki boot crash
- Menambah test case untuk gate yang failed

## Durasi

```
M1 dimulai → M1 selesai (100%)
```

## Exception

Jika ditemukan architectural flaw yang blockers M1, baru boleh discuss untuk perubahan.

---

# 🚨 RULE 2: REGRESSION RULE

## Setiap Gate Yang PASS Tidak Boleh Rusak

```
M1.1 ✅ PASS
    ↓
M1.2 ✅ PASS
    ↓
M1.3 ✅ PASS
    ↓
M1.4 ❌ FAIL
    ↓
    FIX M1.4
    ↓
    RETEST M1.1, M1.2, M1.3, M1.4
    ↓
    SEMUA HARUS PASS
```

## Retest Protocol

Setiap perubahan mengharuskan retest seluruh gate sebelumnya:

```
1. Fix bug di gate X
2. Retest gate 1, 2, 3, ..., X
3. Semua harus PASS
4. Baru boleh lanjut ke gate X+1
```

## Test Command

```bash
# Test semua gate M1
./tests/boot/test_boot.sh

# Jika satu gate fail, fix dan retest semua
./tests/boot/test_gate_X.sh
```

---

# 🚨 RULE 3: BUG CLASSIFICATION

## Bug Prefix

| Prefix | Arti | Contoh |
|--------|------|--------|
| BOOT | Bootloader | BOOT-001: GRUB tidak load kernel |
| KERNEL | Kernel core | KERNEL-001: Triple fault saat boot |
| MM | Memory Manager | MM-001: Page allocator return NULL |
| SCHED | Scheduler | SCHED-001: Thread tidak switch |
| PROC | Process | PROC-001: Fork gagal |
| FS | Filesystem | FS-001: VFS panic saat open |
| IPC | Inter-Process Comm | IPC-001: Channel write blocked |
| SYSCALL | Syscall | SYSCALL-001: syscall return -1 |
| RUNTIME | TypeScript Runtime | RUNTIME-001: Provider null |
| DESKTOP | Desktop UI | DESKTOP-001: Window tidak render |
| WEBKIT | Browser | WEBKIT-001: Page crash |
| BUILD | Build System | BUILD-001: Compiler error |

## Bug Format

```
[PREFIX]-[NUMBER]

Title: Deskripsi singkat

Description:
- Apa yang terjadi
- Kapan terjadi
- Error message

Impact:
- Gate mana yang affected
- Workaround ada?

Status: OPEN | IN_PROGRESS | FIXED | WONTFIX

Fixed in: [commit hash]
```

## Contoh Bug Report

```markdown
# BOOT-001

Title: Kernel tidak masuk entry point

Description:
- Kernel load tapi tidak execute _start
- QEMU hang di "Loading kernel..."

Impact:
- M1.1: FAIL
- M1.2: BLOCKED

Status: OPEN

---

# FS-003

Title: rename() gagal pada directory

Description:
- rename("/old", "/new") return -1
- errno = ENOTDIR

Impact:
- M1.8: FAIL

Status: FIXED
Fixed in: a1b2c3d
```

---

# 🚨 RULE 4: GATE COMPLETION RULE

## M1 Harus 100% Sebelum M2

```
M1: [████████████████████] 100%
    ↓
    M2
```

Bukan:

```
M1: [██████████            ] 50%
    ↓
    M2 (TERLALU DINI)
```

## Gate Yang Belum Pass = M1 Belum Selesai

- M1.1 FAIL → M1 belum selesai
- M1.2 FAIL → M1 belum selesai
- M1.3 FAIL → M1 belum selesai
- ...
- M1.9 FAIL → M1 belum selesai

## Kriteria 100%

```
M1.1 ✅
M1.2 ✅
M1.3 ✅
M1.4 ✅
M1.5 ✅
M1.6 ✅
M1.7 ✅
M1.8 ✅
M1.9 ✅
```

---

# RULE 5: MILESTONE ORDER

## Setelah M1 Selesai

Urutkan milestone baru:

```
M1: QEMU Bring-up
    ↓
M2: Kernel Validation
    ↓
M3: Runtime Native
    ↓
M4: Desktop Bring-up
    ↓
M5: Browser
    ↓
M6: Applications
```

### Alasan

Setelah shell berhasil, lakukan **kernel validation** sebelum mulai runtime integration:

- Stress test scheduler
- Memory leak detection
- VFS stress test
- IPC stress test
- Syscall validation

Ini mencegah bug sulit dilacak yang sebenarnya berasal dari kernel.

---

# IMPLEMENTASI

## Checklist Sebelum Commit (M1)

```bash
# 1. Retest gate yang affected
./tests/boot/test_gate_X.sh

# 2. Pastikan gate sebelumnya tetap PASS
./tests/boot/test_boot.sh

# 3. Update docs/bringup/M1.X-gate.md
#    dengan hasil test

# 4. Commit dengan format:
#    fix([PREFIX]): [deskripsi]
#    contoh: fix(BOOT-001): GRUB multiboot header valid

# 5. Push
git push
```

## Bug Tracker File

```bash
docs/
└── bugs/
    ├── README.md      # Bug tracker overview
    ├── BOOT.md        # Boot-related bugs
    ├── KERNEL.md      # Kernel bugs
    ├── MM.md          # Memory manager bugs
    ├── FS.md          # Filesystem bugs
    └── ...
```

---

# RINGKASAN ATURAN

| Rule | Arti |
|------|------|
| Build Freeze | Hanya fix bug, tidak tambah fitur |
| Regression Rule | Gate PASS harus tetap PASS |
| Bug Classification | Prefix-based tracking |
| Gate Completion | M1 100% sebelum M2 |
| Milestone Order | M2 = Kernel Validation |

---

*Last Updated: 2026-07-07*
