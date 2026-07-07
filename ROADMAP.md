# ROADMAP MASTER KLAT OS v1.0

> **Target Akhir:** KLAT OS dapat boot di QEMU, menjalankan Desktop Shell, Browser berbasis WebKit, File Manager, Terminal, dan Settings menggunakan Runtime + JowoKernel tanpa bergantung pada API browser.

---

# 🚨 ATURAN UTAMA

## GUI KLAT Desktop ADALAH DESAIN FINAL - DILINDUNGI

```
klat-desktop/                          ← TIDAK BOLEH DIUBAH
│
├── Desktop Experience (STABLE)        ← DILINDUNGI
├── Launcher (STABLE)                  ← DILINDUNGI
├── Taskbar (STABLE)                   ← DILINDUNGI
├── Window Manager (STABLE)             ← DILINDUNGI
├── Components (STABLE)                 ← DILINDUNGI
├── Theme (STABLE)                      ← DILINDUNGI
├── Animation (STABLE)                  ← DILINDUNGI
└── Icons (STABLE)                     ← DILINDUNGI
```

**GUI bukan objek migrasi. GUI adalah client. Yang berubah adalah backend.**

## ❌ YANG TIDAK BOLEH

```text
- Redesign GUI
- Rewrite Desktop
- Rewrite Launcher
- Rewrite Window Manager
- Ubah struktur komponen
```

## ✅ YANG BOLEH

```text
- Build Runtime
- Build Kernel API
- Build Native Provider
- Connect ke GUI yang sudah ada
- Fix bug UI (dengan instruksi)
```

## Arsitektur Final

```
┌─────────────────────────────────────────────────────────────┐
│           KLAT DESKTOP (TETAP - DILINDUNGI)                │
│  Desktop │ Launcher │ Taskbar │ Browser │ Settings │ UI  │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    Runtime Services                            │
│                 (Berkembang - API stabil)                     │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      Kernel API Layer                         │
│                    (Phase 1 - DONE ✅)                       │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│     Native Provider  ←→  Browser Provider  ←→  Emulator      │
└─────────────────────────────┬───────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                         JowoKernel                           │
└─────────────────────────────────────────────────────────────┘
```

---

## Status Legend

| Status | Arti |
|--------|------|
| ✅ | Selesai |
| 🔴 | Prioritas Tertinggi / Sedang Dikerjakan |
| 🟡 | Dalam Perencanaan |
| ⚪ | Belum Dimulai |

---

## PHASE 0 — Architecture Freeze

**Status:** ✅ SELESAI

Arsitektur sistem telah difreeze dan terdokumentasi.

**Deliverables:**
- ✅ Runtime Architecture
- ✅ Provider Pattern
- ✅ Runtime Factory
- ✅ Window Runtime
- ✅ Process Runtime
- ✅ Event Runtime
- ✅ Filesystem Runtime
- ✅ Handle Model
- ✅ Capability Model
- ✅ Error Model

**Output:** `Specification Frozen`

---

## PHASE 1 — Kernel API Layer

**Status:** 🔴 PRIORITAS TERTINGGI

**Target:**
```
Runtime
    ↓
Kernel API
    ↓
Syscall Dispatcher
    ↓
JowoKernel
```

**Deliverables:**
```
kernel-api/
    contracts/
        process.hh
        memory.hh
        filesystem.hh
        ipc.hh
        window.hh
    dispatcher/
        syscall_dispatcher.cpp
        syscall_table.cpp
    handles/
        handle.cpp
        handle_table.cpp
    errors/
        error.cpp
        error_codes.h
    syscalls/
        sys_spawn.cpp
        sys_wait.cpp
        sys_kill.cpp
        sys_open.cpp
        sys_read.cpp
        sys_write.cpp
        sys_mmap.cpp
        sys_clone.cpp
        sys_exec.cpp
```

**Selesai jika:** `Runtime tidak lagi mengetahui syscall.`

**Task List:**
- [ ] Buat struktur direktori `kernel-api/`
- [ ] Definisikan kontrak syscall
- [ ] Implementasi Syscall Dispatcher
- [ ] Implementasi Handle Table
- [ ] Implementasi Error Model untuk kernel
- [ ] Refactor Runtime untuk pakai Kernel API
- [ ] Test kompilasi dan integrasi

---

## PHASE 2 — Native Runtime Providers

**Status:** 🟡 DALAM PERENCANAAN

**Target:** Browser Provider diganti Native Provider.

**Deliverables:**
```
src/system/providers/native/
    native_process_provider.cpp
    native_filesystem_provider.cpp
    native_memory_provider.cpp
    native_window_provider.cpp
    native_browser_provider.cpp
    native_ipc_provider.cpp
    native_timer_provider.cpp
```

**Output:**
```
Runtime
    ↓
Native Provider
    ↓
Kernel API
```

**Task List:**
- [ ] Implementasi NativeProcessProvider
- [ ] Implementasi NativeFilesystemProvider
- [ ] Implementasi NativeMemoryProvider
- [ ] Implementasi NativeWindowProvider
- [ ] Implementasi NativeBrowserProvider
- [ ] Implementasi NativeIpcProvider
- [ ] Unit test untuk setiap provider

---

## PHASE 3 — Kernel Integration

**Status:** ⚪ BELUM DIMULAI

**Target:** Hubungkan Runtime dengan JowoKernel.

**Implementasi:**
```
Process      → spawn(), wait(), kill()
Memory       → mmap(), munmap(), alloc()
Filesystem   → open(), read(), write(), close()
IPC          → send(), recv(), connect()
Timer        → sleep(), alarm()
Thread       → clone(), exit(), join()
```

**Output:** `Syscall hidup dan berfungsi.`

**Task List:**
- [ ] Integrasi Process subsystem
- [ ] Integrasi Memory subsystem
- [ ] Integrasi Filesystem (VFS)
- [ ] Integrasi IPC
- [ ] Integrasi Timer
- [ ] Integrasi Thread
- [ ] Test integrasi di QEMU

---

## PHASE 4 — Boot Pipeline

**Status:** ⚪ BELUM DIMULAI

**Target:** Desktop muncul.

**Boot Sequence:**
```
Bootloader
    ↓
JowoKernel
    ↓
Memory Manager
    ↓
Scheduler
    ↓
Runtime Bootstrap
    ↓
Core Services
    ↓
Desktop Shell
```

**Milestone:** `Desktop pertama muncul di QEMU.`

**Task List:**
- [ ] Buat GRUB/UEFI bootloader config
- [ ] Setup Initial RAM Disk (initrd)
- [ ] Buat boot kernel image
- [ ] Verifikasi boot di QEMU
- [ ] Test minimal desktop shell
- [ ] Debug dan fix boot issues

---

## PHASE 5 — Window System

**Status:** ⚪ BELUM DIMULAI

**Target:** Window Manager native.

**Implementasi:**
```
Window Runtime
    ↓
Compositor
    ↓
Framebuffer
    ↓
GPU Driver
```

**Fitur:**
- [ ] Move window
- [ ] Resize window
- [ ] Focus management
- [ ] Minimize
- [ ] Maximize
- [ ] Close

**Task List:**
- [ ] Setup framebuffer
- [ ] Implementasi basic compositor
- [ ] Connect ke DRM/KMS
- [ ] Implementasi window operations
- [ ] Test di QEMU dengan GPU

---

## PHASE 6 — Filesystem

**Status:** ⚪ BELUM DIMULAI

**Target:** File Manager native.

**Implementasi:**
```
Runtime
    ↓
VFS
    ↓
Disk Driver
```

**Fitur:**
- [ ] Copy file
- [ ] Move file
- [ ] Delete file
- [ ] Rename file
- [ ] Watch directory
- [ ] Mount filesystem

**Task List:**
- [ ] Implementasi VFS layer
- [ ] Driver ATA/AHCI
- [ ] Driver VirtIO Block
- [ ] Ext2/ FAT filesystem support
- [ ] Integrasi dengan Desktop File Manager

---

## PHASE 7 — Browser

**Status:** ⚪ BELUM DIMULAI

**Target:** Browser memakai WebKit.

**Implementasi:**
```
Browser App
    ↓
Browser Runtime
    ↓
WebKit
    ↓
Mesa
```

**Hilangkan:** `iframe` selamanya.

**Task List:**
- [ ] Build WebKit untuk KLAT OS
- [ ] Buat WebKit provider
- [ ] Implementasi tab management
- [ ] History dan bookmark
- [ ] DevTools integration

---

## PHASE 8 — Desktop Applications

**Status:** ⚪ BELUM DIMULAI

**Target:** Semua aplikasi memakai Runtime.

**Aplikasi:**
- [ ] Browser (WebKit)
- [ ] Settings
- [ ] Terminal
- [ ] Files (File Manager)
- [ ] Gallery
- [ ] Music
- [ ] Video
- [ ] Text Editor

---

## PHASE 9 — Package Manager

**Status:** ⚪ BELUM DIMULAI

**Implementasi:**
```
KLAT Store
    ↓
Package Runtime
    ↓
Package Manager
    ↓
Repository
```

**Task List:**
- [ ] Package format definition
- [ ] Package manifest parser
- [ ] Repository client
- [ ] Install/Update/Uninstall
- [ ] Dependency resolution
- [ ] Digital signature verification

---

## PHASE 10 — Hardware Support

**Status:** ⚪ BELUM DIMULAI

**Hardware:**
- [ ] USB
- [ ] Bluetooth
- [ ] Camera
- [ ] Audio
- [ ] Printer
- [ ] Power Management
- [ ] Battery
- [ ] Network (Ethernet/WiFi)

---

## PHASE 11 — Optimization

**Status:** ⚪ BELUM DIMULAI

**Optimisasi:**
- [ ] Mesa GPU acceleration
- [ ] LLVM backend
- [ ] JIT compilation
- [ ] Caching layer
- [ ] Thread pool
- [ ] Zero copy I/O

---

## PHASE 12 — Release

**Output:**
- [ ] ISO image
- [ ] Installer
- [ ] Recovery partition
- [ ] Auto-updater
- [ ] Documentation

---

## Arsitektur Final KLAT OS

```
┌─────────────────────────────────────────────────────────────┐
│                    Applications                              │
│  Browser | Settings | Terminal | Files | Gallery | etc.     │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Desktop Experience                         │
│            Desktop Shell | Window Manager                   │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Runtime Services                          │
│   Process | Memory | Filesystem | IPC | Window | Event     │
│                    Runtime Factory                           │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                    Kernel API Layer                          │
│    Process API | Memory API | FS API | IPC API | Window API │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                   Syscall Dispatcher                         │
│              Syscall Table | Handler Registry               │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                       JowoKernel                             │
│  Boot | Memory | Scheduler | Process | IPC | VFS | Drivers  │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                         Drivers                              │
│    GPU | USB | Storage | Network | Input | Audio            │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│                        Hardware                              │
│          CPU | RAM | Disk | GPU | Network Card              │
└─────────────────────────────────────────────────────────────┘
```

---

## Cara Kerja Tim (WAJIB)

Setiap task harus mengikuti prosedur berikut:

```
1. ANALISIS
   ↓
2. LAPORAN
   ↓
3. PERSETUJUAN
   ↓
4. IMPLEMENTASI
   ↓
5. BUILD
   ↓
6. TEST QEMU
   ↓
7. BUG FIX
   ↓
8. COMMIT
   ↓
9. LANJUT TASK BERIKUTNYA
```

**Tidak boleh langsung membuat kode tanpa analisis.**

---

## Prioritas Saat Ini

| Priority | Phase | Description |
|----------|-------|-------------|
| 1 | PHASE 1 | Kernel API Layer |
| 2 | PHASE 2 | Native Runtime Providers |
| 3 | PHASE 3 | Runtime ↔ JowoKernel Integration |
| 4 | PHASE 4 | Boot Pipeline |
| 5 | PHASE 5 | Window System |
| 6 | PHASE 6 | Filesystem |
| 7 | PHASE 7 | WebKit Integration |
| 8 | PHASE 8 | Desktop Applications |
| 9 | PHASE 9 | Package Manager |
| 10 | PHASE 10-12 | Hardware, Optimization, Release |

---

## Catatan Penting

- Setiap phase **WAJIB** memenuhi kriteria berikut sebelum lanjut:
  1. Build berhasil tanpa error
  2. Test unit lulus
  3. Tidak ada regresi pada phase sebelumnya
  4. Berhasil boot/test di QEMU
  5. Architecture review lolos

- Phase tidak boleh dilompati. Urutan adalah linear.

---

*Last Updated: 2026-07-07*
*Version: 1.0*
*Author: KLAT OS Team*
