# KLAT OS Exit Criteria

> **Syarat lulus untuk setiap milestone**

---

# M1: QEMU Bring-up

## EXIT CRITERIA

```
PASS JIKA:

✓ M1.1 Bootloader    → GRUB load kernel
✓ M1.2 Kernel Entry  → Banner "JowoKernel" tercetak
✓ M1.3 Memory Mgr    → PMM initialized
✓ M1.4 Scheduler     → Thread switching berfungsi
✓ M1.5 Init Process  → init.elf berjalan
✓ M1.6 Shell         → Shell prompt aktif
✓ M1.7 Spawn Process → fork() berhasil
✓ M1.8 Filesystem RW → open/read/write berhasil
✓ M1.9 Shutdown      → Clean shutdown

✓ Tidak ada BUG Critical terbuka
✓ Semua gate M1.1-M1.9: PASS
```

## Exit Checklist

```
[ ] M1.1: GRUB → Kernel: PASS
[ ] M1.2: Banner tercetak: PASS
[ ] M1.3: PMM works: PASS
[ ] M1.4: Thread switch: PASS
[ ] M1.5: Init runs: PASS
[ ] M1.6: Shell prompt: PASS
[ ] M1.7: fork() works: PASS
[ ] M1.8: FS RW works: PASS
[ ] M1.9: Shutdown: PASS

[ ] Critical bugs: 0
[ ] Regression: semua gate PASS
```

---

# M2: Kernel Validation

## EXIT CRITERIA

```
PASS JIKA:

✓ M2.1 Scheduler stress  → 1000 thread switch tanpa crash
✓ M2.2 Memory leak      → 24 jam tanpa memory leak
✓ M2.3 VFS stress        → 1000 open/close tanpa corruption
✓ M2.4 Syscall          → Semua syscall return benar
✓ M2.5 IPC stress       → 1000 message tanpa deadlock

✓ Tidak ada BUG Critical terbuka
✓ Stress test >24 jam tanpa kernel panic
```

## Exit Checklist

```
[ ] M2.1: Scheduler stress: 1000 switch PASS
[ ] M2.2: Memory leak: 24 jam PASS
[ ] M2.3: VFS stress: 1000 ops PASS
[ ] M2.4: Syscall: Semua PASS
[ ] M2.5: IPC stress: 1000 msg PASS

[ ] Critical bugs: 0
[ ] Regression: M1 gates still PASS
```

---

# M3: Runtime Native

## EXIT CRITERIA

```
PASS JIKA:

✓ M3.1 Runtime-Kernel    → TypeScript Runtime → JowoKernel
✓ M3.2 Process Provider  → NativeProcessProvider.test() PASS
✓ M3.3 Filesystem       → NativeFilesystemProvider.test() PASS
✓ M3.4 Memory Provider  → NativeMemoryProvider.test() PASS
✓ M3.5 IPC Provider     → NativeIpcProvider.test() PASS

✓ Tidak ada Browser API palsu
✓ Tidak ada localStorage di Runtime
✓ Integration test 100%
✓ Tidak ada BUG Critical terbuka
```

## Exit Checklist

```
[ ] M3.1: Runtime → Kernel: PASS
[ ] M3.2: Process: PASS
[ ] M3.3: Filesystem: PASS
[ ] M3.4: Memory: PASS
[ ] M3.5: IPC: PASS

[ ] No fake Browser API: CONFIRMED
[ ] Integration tests: 100%
[ ] Critical bugs: 0
[ ] Regression: M1, M2 gates still PASS
```

---

# M4: Desktop Bring-up

## EXIT CRITERIA

```
PASS JIKA:

✓ M4.1 Desktop build    → klat-desktop build berhasil
✓ M4.2 Desktop appears → Desktop Shell muncul
✓ M4.3 Window drag     → Window bisa dipindah
✓ M4.4 Taskbar         → Taskbar berfungsi
✓ M4.5 Same as browser → Tampilan IDENTIK dengan browser

✓ Tidak ada visual regression
✓ Tidak ada bug Desktop baru
✓ GUI TIDAK DIUBAH
✓ Tidak ada BUG Critical terbuka
```

## Exit Checklist

```
[ ] M4.1: Desktop build: PASS
[ ] M4.2: Desktop appears: PASS
[ ] M4.3: Window drag: PASS
[ ] M4.4: Taskbar: PASS
[ ] M4.5: Visual match browser: PASS

[ ] GUI unchanged: CONFIRMED
[ ] Visual regression: NONE
[ ] Critical bugs: 0
[ ] Regression: M1-M3 gates still PASS
```

---

# M5: Browser

## EXIT CRITERIA

```
PASS JIKA:

✓ M5.1 WebKit build      → WebKit berhasil di-build
✓ M5.2 WebKit Provider   → Browser Runtime → WebKit
✓ M5.3 Mesa init         → Mesa context diciptakan
✓ M5.4 GPU acceleration  → Hardware acceleration works
✓ M5.5 iframe removed     → Tidak ada iframe

✓ HTML page load: PASS
✓ CSS rendering: PASS
✓ JavaScript execute: PASS
✓ HTTPS request: PASS
✓ Local storage: PASS
✓ iframe count: 0
✓ Tidak ada BUG Critical terbuka
```

## Exit Checklist

```
[ ] M5.1: WebKit build: PASS
[ ] M5.2: WebKit Provider: PASS
[ ] M5.3: Mesa: PASS
[ ] M5.4: GPU accel: PASS
[ ] M5.5: iframe: 0

[ ] HTML: PASS
[ ] CSS: PASS
[ ] JavaScript: PASS
[ ] HTTPS: PASS
[ ] iframe count: 0
[ ] Critical bugs: 0
[ ] Regression: M1-M4 gates still PASS
```

---

# Exit Criteria Summary

| Milestone | Gates | Critical Bugs | Regression |
|-----------|-------|---------------|------------|
| M1 QEMU | 9 | 0 | - |
| M2 Kernel Validation | 5 | 0 | M1 |
| M3 Runtime Native | 5 | 0 | M1, M2 |
| M4 Desktop | 5 | 0 | M1-M3 |
| M5 Browser | 5 | 0 | M1-M4 |

---

# RULES

1. Milestone tidak boleh PASS jika ada Critical bug terbuka
2. Regression test WAJIB untuk semua gate sebelumnya
3. Exit criteria harus diverifikasi manual
4. Bukti (screenshot/log) harus ada di docs/bringup/

---

*Last Updated: 2026-07-07*
