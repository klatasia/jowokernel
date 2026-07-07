# KLAT OS Bug Tracker

> **Single source of truth untuk semua bug proyek**

---

# Bug Status Flow

```
OPEN
    ↓
CONFIRMED
    ↓
IN_PROGRESS
    ↓
FIXED
    ↓
REGRESSION TEST
    ↓
CLOSED
```

## Status Definitions

| Status | Arti |
|--------|------|
| OPEN | Bug baru ditemukan |
| CONFIRMED | Bug dikonfirmasi valid |
| IN_PROGRESS | Sedang diperbaiki |
| FIXED | Perbaikan selesai (butuh test) |
| REGRESSION TEST | Ditest ulang |
| CLOSED | Sudah selesai dan stabil |

---

# Bug Format

```
[CODE]-[NUMBER]

Title: [Judul singkat]

Status: [STATUS]

Date Opened:
Date Confirmed:
Date Fixed:
Date Closed:

Affected Gate:
Severity: CRITICAL / HIGH / MEDIUM / LOW

Description:
[Detail bug]

Evidence:
[Serial log / screenshot / crash dump]

Fix:
[Commit hash atau "not fixed"]

Notes:
[Additional context]
```

---

# Active Bugs

## OPEN

| Code | Title | Severity | Gate |
|------|-------|----------|------|
| (none) | | | |

---

## CONFIRMED

| Code | Title | Severity | Gate |
|------|-------|----------|------|
| (none) | | | |

---

## IN_PROGRESS

| Code | Title | Severity | Gate |
|------|-------|----------|------|
| (none) | | | |

---

## FIXED (needs regression test)

| Code | Title | Gate | Fixed In |
|------|-------|------|----------|
| (none) | | | |

---

## REGRESSION TEST

| Code | Title | Gate | Status |
|------|-------|------|--------|
| (none) | | | |

---

# Closed Bugs

| Code | Title | Severity | Gate | Closed |
|------|-------|----------|------|--------|
| (none) | | | | |

---

# Bug Count

```
OPEN:          0
CONFIRMED:      0
IN_PROGRESS:    0
FIXED:         0
REGRESSION:     0
CLOSED:         0
─────────────────
TOTAL:          0
```

---

# Bug Classification

| Prefix | Scope | Example |
|--------|-------|---------|
| BOOT | Bootloader | BOOT-001 |
| KERNEL | Kernel core | KERNEL-001 |
| MM | Memory Manager | MM-001 |
| SCHED | Scheduler | SCHED-001 |
| PROC | Process | PROC-001 |
| FS | Filesystem | FS-001 |
| IPC | Inter-Process Comm | IPC-001 |
| SYSCALL | Syscall | SYSCALL-001 |
| RUNTIME | TypeScript Runtime | RUNTIME-001 |
| DESKTOP | Desktop UI | DESKTOP-001 |
| WEBKIT | Browser | WEBKIT-001 |
| BUILD | Build System | BUILD-001 |

---

# Severity Levels

| Level | Arti | Response |
|-------|------|----------|
| CRITICAL | System crash / data loss | Immediate fix |
| HIGH | Major feature broken | Fast fix |
| MEDIUM | Feature degraded | Normal fix |
| LOW | Cosmetic / minor | When convenient |

---

# Rules

1. Setiap bug harus punya prefix
2. Bug tidak boleh CLOSED tanpa regression test
3. CRITICAL bug blockers M1 gates
4. Bug tracker adalah single source of truth
5. Commit message harus reference bug code

## Commit Format

```
fix(BOOT-001): GRUB multiboot header valid
fix(MM-003): page allocator return NULL
feat(SCHED-002): thread priority support
test(FS-005): add stress test
```

---

# Severity Bug Count

```
CRITICAL:  0
HIGH:       0
MEDIUM:     0
LOW:        0
───────────────
TOTAL:      0
```

---

# Gate Filter

| Gate | Bugs |
|------|------|
| M1.1 Bootloader | 0 |
| M1.2 Kernel Entry | 0 |
| M1.3 Memory Manager | 0 |
| M1.4 Scheduler | 0 |
| M1.5 Init Process | 0 |
| M1.6 Shell | 0 |
| M1.7 Spawn Process | 0 |
| M1.8 Filesystem RW | 0 |
| M1.9 Shutdown | 0 |
| M2 Kernel Validation | 0 |
| M3 Runtime Native | 0 |
| M4 Desktop | 0 |
| M5 Browser | 0 |

---

*Last Updated: 2026-07-07*
