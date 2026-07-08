# JowoKernel Project Roadmap

**Version**: 1.0  
**Last Updated**: 2026-07-08  
**Status**: Active Development

---

## 1. Project Overview

JowoKernel adalah proyek sistem operasi desktop native yang dikembangkan secara modular dengan empat domain utama. Setiap domain dapat berkembang secara paralel menggunakan milestone kecil yang selalu dapat di-boot.

### Project Domains

```
┌─────────────────────────────────────────────────────────────────┐
│                        JowoKernel                                │
├─────────────┬─────────────┬─────────────┬───────────────────────┤
│   Kernel    │   Runtime   │     GUI     │         SDK           │
├─────────────┼─────────────┼─────────────┼───────────────────────┤
│ M1          │ R1          │ G1          │ S1                     │
│ M2          │ R2          │ G2          │ S2                     │
│ M3          │ R3          │ G3          │ S3                     │
│ M4          │ R4          │ G4          │ S4                     │
└─────────────┴─────────────┴─────────────┴───────────────────────┘
```

---

## 2. Domain Structure

### 2.1 Kernel (`kernel/`)

Fokus: Scheduler, Memory Management, VFS, Drivers, IPC, Network

```
kernel/
├── arch/x86/           # x86_64 architecture
├── kernel/             # Core kernel subsystems
├── vm/                 # Virtual memory
├── fs/                 # Filesystem
├── syscalls/           # System calls
├── subsystems/         # Modular subsystems
└── tests/             # Kernel tests
```

### 2.2 Runtime (`runtime/`)

Fokus: C Runtime, libc, Dynamic Linker, POSIX Compatibility

```
runtime/
├── crt0/              # C runtime startup
├── libc/               # C standard library
├── libdl/              # Dynamic linker
├── libpthread/         # POSIX threads
└── tests/              # Runtime tests
```

### 2.3 GUI (`gui/`)

Fokus: Graphics, Window Manager, Desktop Environment

```
gui/
├── klat-wm/            # Window Manager
├── klat-desktop/        # Desktop Environment
├── compositor/          # Wayland-style compositor
├── renderer/           # Graphics renderer
└── apps/               # Built-in applications
```

### 2.4 SDK (`sdk/`)

Fokus: Widgets, Application API, Package Manager, Developer Tools

```
sdk/
├── include/            # Public headers
├── lib/                # Libraries
├── tools/              # Developer tools
├── docs/               # SDK documentation
└── examples/           # Example code
```

---

## 3. Kernel Milestones

### Current: M3.3 ✅ (Ready for Validation)

| Milestone | Description | Status |
|-----------|-------------|--------|
| M1 | Boot & Initialization | ✅ PASS |
| M2 | Memory Architecture | ✅ PASS |
| M3 | Scheduler & Threads | ✅ PASS |
| M3.3 | Userspace Syscall | ⏳ Validation |

### Planned: M3.4 - M4

| Milestone | Description | Target |
|-----------|-------------|--------|
| M3.4 | Userspace Runtime | TBD |
| M3.5 | Filesystem Runtime | TBD |
| M3.6 | POSIX Layer | TBD |
| M3.7 | Network Stack | TBD |
| M4 | Full Kernel Feature Set | TBD |

---

## 4. Runtime Milestones

### Planned

| Milestone | Description | Dependencies |
|-----------|-------------|--------------|
| R1 | crt0 Startup Code | M3.4 |
| R2 | libc Minimal (printf, malloc) | R1 |
| R3 | Dynamic Linker (ld.so) | R2 |
| R4 | POSIX Compatibility | R3, M3.5 |

### R1 Detail

- [ ] crt0.S - C runtime startup
- [ ] argc/argv support
- [ ] envp support
- [ ] Stack alignment to 16 bytes
- [ ] Initial TLS setup

### R2 Detail

- [ ] printf/sprintf
- [ ] malloc/free
- [ ] open/close/read/write
- [ ] fork/execve
- [ ] atexit handlers

---

## 5. GUI Milestones

### Planned

| Milestone | Description | Dependencies |
|-----------|-------------|--------------|
| G1 | Native Graphics (Vulkan/EGL) | M3.4 |
| G2 | KLAT Window Manager | G1, R2 |
| G3 | KLAT Desktop Environment | G2 |
| G4 | Built-in Applications | G3 |

### G1 Detail

- [ ] DRM/KMS integration
- [ ] Vulkan driver
- [ ] EGL context
- [ ] GPU memory management
- [ ] Frame scheduler

### G2 Detail

- [ ] X11/Wayland-compatible protocol
- [ ] Window management
- [ ] Multi-window support
- [ ] Event handling
- [ ] Workspace/virtual desktop

---

## 6. SDK Milestones

### Planned

| Milestone | Description | Dependencies |
|-----------|-------------|--------------|
| S1 | Widget Library | G2 |
| S2 | Application API | S1 |
| S3 | Package Manager | S2 |
| S4 | Developer Tools | S3 |

---

## 7. Development Roadmap

### Phase 1: Foundation (Current)

```
2026 Q3
├── M3.3 Validation      ← HERE
├── M3.4 Development
├── R1 Planning
└── G1 Research
```

### Phase 2: Runtime Foundation

```
2026 Q4
├── M3.4 PASS
├── R1 Development
├── G1 Development
└── klat-desktop v0.1.0
```

### Phase 3: Desktop Ready

```
2027 Q1
├── M3.5 PASS
├── R2 Development
├── G2 Development
└── klat-desktop v0.5.0
```

### Phase 4: Alpha Release

```
2027 Q2
├── KLAT OS Alpha
├── R3 Development
├── G3 Development
└── Basic Applications
```

---

## 8. Parallel Development Tracks

```
TRACK 1: Kernel
─────────────────────────────────────────
M3.3 → M3.4 → M3.5 → M3.6 → M4

TRACK 2: Runtime
─────────────────────────────────────────
        R1 → R2 → R3 → R4

TRACK 3: GUI
─────────────────────────────────────────
        G1 → G2 → G3 → G4

TRACK 4: SDK
─────────────────────────────────────────
              S1 → S2 → S3 → S4
```

Each track can develop independently once dependencies are met.

---

## 9. Dependencies Map

```
Kernel ← Runtime ← GUI ← SDK
   ↑       ↑       ↑
   └───────┴───────┘
         (ABI)
```

- Kernel provides stable ABI
- Runtime builds on kernel ABI
- GUI builds on runtime + kernel
- SDK builds on all domains

---

## 10. Repository Structure (Future)

```
jowokernel/           ← Main kernel repository
├── kernel/           ← Kernel source
├── runtime/          ← C runtime & libc
├── gui/              ← GUI components
├── sdk/              ← SDK
└── docs/             ← Documentation

klat-desktop/         ← Desktop environment repository
klat-sdk/            ← SDK repository
klat-apps/           ← Application repository
```

---

## 11. Current Status

```
Kernel    ████████████░░░░░░░░░░░░░  60% (M3.3 pending)
Runtime   ░░░░░░░░░░░░░░░░░░░░░░░░  0%  (Planning)
GUI       ░░░░░░░░░░░░░░░░░░░░░░░░  0%  (Planning)
SDK       ░░░░░░░░░░░░░░░░░░░░░░░░  0%  (Planning)
```

---

## 12. Change Log

| Date | Version | Changes |
|------|--------|---------|
| 2026-07-08 | 1.0 | Initial roadmap with four-domain structure |

---

**Maintainer**: JowoKernel Team
**Next Review**: After M3.3 PASS
