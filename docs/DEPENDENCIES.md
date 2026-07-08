# KLAT OS Dependency Graph

**Version**: 1.0  
**Status**: Active  
**Last Updated**: 2026-07-08  

---

## 1. Overview

Dokumen ini mendefinisikan dependency graph antar domain dan milestone. Ini memastikan arsitektur tetap bersih dan pekerjaan paralel tidak saling menghambat.

---

## 2. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      KLAT OS STACK                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    APPLICATIONS                             │  │
│  │  Browser  │  Terminal  │  File Manager  │  Settings     │  │
│  └──────────────────────────┬────────────────────────────────┘  │
│                               │                                    │
│  ┌────────────────────────────┼────────────────────────────────┐  │
│  │                      SDK                                 │  │
│  │   Widgets  │  Window API  │  Canvas API  │  Input API    │  │
│  └────────────────────────────┼────────────────────────────────┘  │
│                               │                                    │
│  ┌────────────────────────────┼────────────────────────────────┐  │
│  │                   DESKTOP SHELL                           │  │
│  │        klat-desktop  │  klat-wm  │  Compositor              │  │
│  └────────────────────────────┼────────────────────────────────┘  │
│                               │                                    │
│  ┌────────────────────────────┼────────────────────────────────┐  │
│  │                      RUNTIME                              │  │
│  │     libc  │  libpthread  │  libdl  │  POSIX Layer         │  │
│  └────────────────────────────┼────────────────────────────────┘  │
│                               │                                    │
│  ┌────────────────────────────┼────────────────────────────────┐  │
│  │                      KERNEL                               │  │
│  │   Scheduler  │  VFS  │  Memory  │  Drivers  │  Syscalls   │  │
│  └────────────────────────────┴────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Domain Dependency Graph

### 3.1 Dependency Rules

```
┌─────────────────────────────────────────────────────────────────┐
│                     DEPENDENCY RULES                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌──────────┐                                                   │
│   │   SDK    │ ←─ Depends on: Runtime                            │
│   └────┬─────┘                                                   │
│        │                                                         │
│   ┌────┴─────┐                                                   │
│   │   GUI    │ ←─ Depends on: Runtime, SDK                       │
│   └────┬─────┘                                                   │
│        │                                                         │
│   ┌────┴─────┐                                                   │
│   │ Runtime  │ ←─ Depends on: Kernel                             │
│   └────┬─────┘                                                   │
│        │                                                         │
│   ┌────┴─────┐                                                   │
│   │  Kernel  │ ←─ Foundation, no dependencies                     │
│   └──────────┘                                                   │
│                                                                  │
│   IMPORTANT:                                                     │
│   - GUI NEVER depends directly on Kernel                          │
│   - Applications NEVER depend directly on Kernel                  │
│   - Always go through the layers                                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Dependency Matrix

| Component | Kernel | Runtime | GUI | SDK | Apps |
|-----------|--------|---------|-----|-----|------|
| **Kernel** | - | - | - | - | - |
| **Runtime** | ✅ | - | - | - | - |
| **GUI** | ❌ | ✅ | - | ✅ | - |
| **SDK** | ❌ | ✅ | - | - | - |
| **Apps** | ❌ | ✅ | ✅ | ✅ | - |

✅ = Can depend on  
❌ = Must NOT depend on

---

## 4. Kernel Milestone Dependencies

```
M3.3 ──────────────────────────────────────────────────
  │
  ▼
M3.4 ──────────────────────────────────────────────────
  │  Requirements:
  │  - Stable syscall interface
  │  - ELF loader
  │
  ├──► R1 (Runtime: crt0)
  │
  └──► M3.5 ──────────────────────────────────────────
        │
        ├──► M3.6 ──────────────────────────────────
        │     │
        │     └──► R2 (Runtime: libc)
        │
        └──► M3.7 ──────────────────────────────────
              │
              └──► G1 (GUI: Graphics)
```

---

## 5. Runtime Milestone Dependencies

```
R1 (crt0) ──────────────────────────────────────────────
  │  Requirements:
  │  - M3.4 syscall interface
  │
  ├──► R2 (libc) ────────────────────────────────────────
  │     │
  │     └──► R3 (Dynamic Linker)
  │
  └──► S1 (SDK: Widgets)
        │
        └──► S2 (SDK: Application API)
              │
              └──► G2 (klat-wm)
```

---

## 6. GUI Milestone Dependencies

```
G1 (Graphics Foundation) ────────────────────────────────
  │  Requirements:
  │  - M3.4 syscall interface
  │  - Stable kernel ABI
  │
  ├──► G2 (klat-wm) ────────────────────────────────────
  │     │  Requirements:
  │     │  - G1 graphics
  │     │  - R2 libc (basic)
  │     │
  │     └──► G3 (klat-desktop)
  │           │
  │           └──► S3 (SDK: Package Manager)
  │
  └──► G4 (Built-in Apps)
        │
        └──► G5 (Browser)
```

---

## 7. Cross-Domain Dependencies

### 7.1 Critical Path to klat-desktop

```
M3.3 ───► M3.4 ───► R1 ───► R2 ───► G1 ───► G2 ───► G3
  │        │        │       │       │       │       │
  │        │        │       │       │       │       │
  └────────┴────────┴───────┴───────┴───────┴───────► KLAT Desktop
                       (12 milestones)
```

### 7.2 Parallel Development Tracks

```
┌─────────────────────────────────────────────────────────────┐
│  TRACK 1: Kernel                                             │
│  M3.3 → M3.4 → M3.5 → M3.6 → M3.7 → M4                    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  TRACK 2: Runtime                                            │
│         R1 → R2 → R3 → R4                                   │
│              ↑                                              │
│              └─ Depends on M3.5                             │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  TRACK 3: GUI                                               │
│                    G1 → G2 → G3 → G4 → G5                 │
│                    ↑                                        │
│                    └─ Depends on M3.4 + R2                  │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  TRACK 4: SDK                                               │
│                S1 → S2 → S3 → S4                           │
│                ↑                                            │
│                └─ Depends on R2                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 8. Implementation Order (Sprint Planning)

### Sprint 1 (After v0.3.3)

| Task | Track | Dependency |
|------|-------|------------|
| M3.4.1 crt0 | Kernel | M3.3 |
| M3.4.2 argc/argv | Kernel | M3.4.1 |
| R1 specification | Runtime | - |
| G1.0 review | GUI | - |

### Sprint 2

| Task | Track | Dependency |
|------|-------|------------|
| M3.4.3 libc minimal | Kernel | M3.4.2 |
| M3.4.4 brk() | Kernel | M3.4.3 |
| R1 implementation | Runtime | M3.4.2 |
| G1.1 DRM init | GUI | M3.4 |

### Sprint 3

| Task | Track | Dependency |
|------|-------|------------|
| M3.4.5 execve() | Kernel | M3.4.4 |
| R2 libc | Runtime | R1 |
| G1.2 Surface | GUI | G1.1 |

### Sprint 4

| Task | Track | Dependency |
|------|-------|------------|
| M3.4.6 waitpid() | Kernel | M3.4.5 |
| G2 klat-wm | GUI | R2, G1.2 |

---

## 9. Rules Summary

```
╔═══════════════════════════════════════════════════════════════╗
║                    ARCHITECTURE RULES                         ║
╠═══════════════════════════════════════════════════════════════╣
║                                                               ║
║  1. Kernel is the foundation - no dependencies               ║
║                                                               ║
║  2. Runtime depends on Kernel                                 ║
║                                                               ║
║  3. GUI depends on Runtime AND SDK                            ║
║                                                               ║
║  4. SDK depends on Runtime                                    ║
║                                                               ║
║  5. Applications depend on GUI, SDK, Runtime, Kernel          ║
║                                                               ║
║  6. NEVER skip layers - no direct dependencies                ║
║                                                               ║
║  7. Each milestone must complete before dependent starts       ║
║                                                               ║
║  8. Parallel tracks can develop simultaneously                ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
```

---

## 10. Change Log

| Date | Version | Changes |
|------|---------|---------|
| 2026-07-08 | 1.0 | Initial dependency graph |

---

**Maintainer**: KLAT OS Team
