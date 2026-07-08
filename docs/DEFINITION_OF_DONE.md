# KLAT OS Definition of Done

**Version**: 1.0  
**Status**: Active  
**Last Updated**: 2026-07-08  

---

## 1. Overview

Dokumen ini mendefinisikan standar kelulusan untuk setiap milestone. Setiap domain memiliki criteria yang jelas sehingga progress dapat diukur secara konsisten.

---

## 2. Kernel Domain

### 2.1 General Kernel DoD

| Criteria | Description |
|----------|-------------|
| **Boot** | Kernel boots to shell without fault |
| **Regression Test** | All tests in `tests/regression/run_all.sh` pass |
| **Documentation** | Milestone document updated with results |
| **ABI Stability** | No breaking changes to frozen ABI |

### 2.2 Per-Milestone

#### M3.4 - Userspace Runtime

| Criteria | Description |
|----------|-------------|
| **crt0** | C runtime startup code functional |
| **argc/argv** | Arguments passed correctly to main() |
| **envp** | Environment variables accessible |
| **brk()** | Heap allocation via brk syscall |
| **Regression** | Userspace tests pass |
| **Documentation** | ABI updated, crt0 documented |

#### M3.5 - Filesystem Runtime

| Criteria | Description |
|----------|-------------|
| **VFS** | Virtual filesystem switch operational |
| **open/close** | File descriptor operations work |
| **read/write** | File I/O operations work |
| **Regression** | Filesystem tests pass |

---

## 3. Runtime Domain

### 3.1 General Runtime DoD

| Criteria | Description |
|----------|-------------|
| **Compilation** | All code compiles without errors |
| **Static Test** | All functions have unit tests |
| **ABI Documentation** | Public API documented |
| **Regression** | Runtime tests pass |

### 3.2 Per-Milestone

#### R1 - crt0 Startup

| Criteria | Description |
|----------|-------------|
| **Startup** | crt0.S initializes correctly |
| **argc/argv** | Arguments passed to main() |
| **Stack Align** | Stack aligned to 16 bytes |
| **Exit** | atexit() handlers called |
| **Test** | "Hello World" compiles and runs |

#### R2 - libc Minimal

| Criteria | Description |
|----------|-------------|
| **printf** | Formatted output works |
| **malloc/free** | Heap allocation functional |
| **string** | String functions work |
| **Regression** | libc tests pass |

#### R3 - Dynamic Linker

| Criteria | Description |
|----------|-------------|
| **ld.so** | Dynamic linker loads shared libraries |
| **dlopen** | Runtime library loading works |
| **Symbols** | Symbol resolution correct |
| **Regression** | Dynamic linking tests pass |

---

## 4. GUI Domain

### 4.1 General GUI DoD

| Criteria | Description |
|----------|-------------|
| **Compilation** | All components compile |
| **Demo** | Basic demo runs in QEMU |
| **Input** | Mouse/keyboard input functional |
| **Screenshot** | Visual proof of functionality |
| **Documentation** | API documented |

### 4.2 Per-Milestone

#### G1 - Graphics Foundation

| Criteria | Description |
|----------|-------------|
| **DRM Init** | DRM/KMS initialized |
| **GPU Memory** | GBM buffer allocation works |
| **Surface** | Single surface renders |
| **Mouse** | Cursor moves on screen |
| **Screenshot** | Visual proof |

#### G2 - klat-wm

| Criteria | Description |
|----------|-------------|
| **Window** | Single window renders |
| **Close** | Close button works |
| **Input** | Keyboard focus works |
| **Screenshot** | Window visible |

#### G3 - klat-desktop

| Criteria | Description |
|----------|-------------|
| **Panel** | Taskbar visible |
| **Launcher** | App launcher works |
| **Multi-Window** | Multiple windows managed |
| **Screenshot** | Desktop visible |

---

## 5. SDK Domain

### 5.1 General SDK DoD

| Criteria | Description |
|----------|-------------|
| **Headers** | Public headers complete |
| **Compilation** | Examples compile |
| **Documentation** | API reference complete |
| **Regression** | SDK tests pass |

### 5.2 Per-Milestone

#### S1 - Widget Library

| Criteria | Description |
|----------|-------------|
| **Button** | Button widget works |
| **Input** | Text input works |
| **Layout** | Box layout works |
| **Test App** | Example app runs |

#### S2 - Application API

| Criteria | Description |
|----------|-------------|
| **Window API** | Window creation works |
| **Canvas API** | Drawing works |
| **Event API** | Events handled |
| **Test App** | Example app runs |

---

## 6. Definition Levels

### 6.1 Types of Completeness

| Level | Description | Requirements |
|-------|-------------|--------------|
| **Draft** | Initial implementation | Code compiles |
| **Alpha** | Basic functionality | Meets DoD criteria |
| **Beta** | Feature complete | All tests pass |
| **Stable** | Production ready | No known issues |

### 6.2 Milestone States

```
┌──────────┐
│  Draft   │ ← Implementation started
└────┬─────┘
     │ Code compiles
     ▼
┌──────────┐
│  Alpha   │ ← Basic functionality works
└────┬─────┘
     │ Meets Definition of Done
     ▼
┌──────────┐
│  Beta    │ ← All features implemented
└────┬─────┘
     │ No regressions
     ▼
┌──────────┐
│ Stable   │ ← Ready for release
└──────────┘
```

---

## 7. Regression Requirements

### 7.1 Pre-Commit

Before any commit:
- [ ] Code compiles
- [ ] New tests pass
- [ ] Existing tests pass

### 7.2 Pre-Merge

Before merging to main:
- [ ] `./tests/regression/run_all.sh` passes
- [ ] No new compiler warnings
- [ ] Documentation updated

### 7.3 Pre-Release

Before tagging release:
- [ ] All DoD criteria met
- [ ] Regression suite 100% pass
- [ ] Documentation complete
- [ ] Changelog updated

---

## 8. Change Log

| Date | Version | Changes |
|------|---------|---------|
| 2026-07-08 | 1.0 | Initial Definition of Done |

---

**Maintainer**: KLAT OS Team
