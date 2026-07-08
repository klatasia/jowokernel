# KLAT GUI Architecture Specification

**Version**: G1.0  
**Status**: Draft - Ready for Review  
**Last Updated**: 2026-07-08  
**Domain**: GUI  

---

## 1. Overview

Dokumen ini mendefinisikan arsitektur sistem GUI untuk KLAT OS, termasuk window server, compositor, rendering pipeline, dan sistem input.

### 1.1 Design Goals

| Goal | Description |
|------|-------------|
| **Native Performance** | GPU-accelerated rendering |
| **Security** | Isolated application surfaces |
| **Consistency** | Unified theme system |
| **Accessibility** | Built-in accessibility support |
| **Extensibility** | Plugin-based architecture |

### 1.2 Non-Goals

- X11 compatibility (native only)
- Wayland compatibility (initially)
- Remote display (future consideration)

---

## 2. System Architecture

### 2.1 Layer Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     APPLICATION LAYER                             │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ Browser │  │ Terminal │  │ Files   │  │ Settings │           │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘           │
└───────┼─────────────┼─────────────┼─────────────┼─────────────────┘
        │             │             │             │
        └─────────────┴─────────────┴─────────────┘
                          │ Surface API
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                        SDK LAYER                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    klat-sdk                               │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐   │   │
│  │  │ Surface │  │ Window  │  │ Widget  │  │  Input  │   │   │
│  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘   │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                    klat-desktop LAYER                            │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   Desktop Shell                           │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐              │   │
│  │  │ Launcher│  │  Panel  │  │Desktop  │              │   │
│  │  └─────────┘  └─────────┘  └─────────┘              │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                      klat-wm LAYER                               │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                 Window Manager                            │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐              │   │
│  │  │ Window  │  │ Workspace│  │  Input  │              │   │
│  │  │ Manager │  │ Manager  │  │ Router  │              │   │
│  │  └─────────┘  └─────────┘  └─────────┘              │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Compositor LAYER                              │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                 Compositor                               │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐              │   │
│  │  │ Buffer  │  │ Render  │  │ Present │              │   │
│  │  │ Manager │  │ Engine  │  │ Queue   │              │   │
│  │  └─────────┘  └─────────┘  └─────────┘              │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Graphics LAYER                                 │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐       │
│  │   DRM   │  │ Vulkan  │  │   EGL   │  │   GEM   │       │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘       │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                       KERNEL LAYER                               │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐                      │
│  │  GPU    │  │  Input  │  │  VFS    │                      │
│  │ Driver  │  │ Devices │  │          │                      │
│  └─────────┘  └─────────┘  └─────────┘                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Component Specification

### 3.1 klat-wm (Window Manager)

**Responsibility**: Window lifecycle, positioning, Z-order, focus management

| Feature | Description |
|---------|-------------|
| Window Creation | Allocate window ID, register with compositor |
| Window Destruction | Cleanup resources, notify application |
| Window Positioning | Absolute and relative positioning |
| Window Sizing | Minimum, maximum, resize handles |
| Z-Order | Stacking order management |
| Focus Management | Keyboard focus, focus follows mouse |
| Multi-Monitor | Multi-monitor support |

### 3.2 Compositor

**Responsibility**: Surface composition, frame timing, buffer management

| Feature | Description |
|---------|-------------|
| Buffer Allocation | GBM bo allocation for surfaces |
| Rendering | GPU-accelerated composition |
| Frame Timing | VSync, frame pacing |
| Damage Tracking | Incremental updates |
| Output Management | Monitor detection, mode setting |

### 3.3 Desktop Shell

**Responsibility**: Desktop environment UI, panel, launcher, notifications

| Feature | Description |
|---------|-------------|
| Desktop Background | Wallpaper, widgets |
| Panel | Taskbar, system tray, clock |
| Launcher | Application menu |
| Notifications | Notification center |
| Workspaces | Virtual desktops |

---

## 4. Surface API (Frozen Interface)

### 4.1 Surface Creation

```cpp
// Application requests a new surface
Surface* surface = klat_surface_create(
    width,      // Surface width
    height,     // Surface height
    format,     // Pixel format (RGBA8, BGRA8)
    flags       // Flags (NONE, POPUP, FULLSCREEN)
);
```

### 4.2 Surface Operations

```cpp
// Commit a new frame to the surface
int klat_surface_commit(Surface* surface, const FrameData* frame);

// Request buffer for rendering
Buffer* klat_surface_get_buffer(Surface* surface, int width, int height);

// Set surface properties
int klat_surface_set_title(Surface* surface, const char* title);
int klat_surface_set_size(Surface* surface, int width, int height);
int klat_surface_set_position(Surface* surface, int x, int y);

// Close surface
int klat_surface_close(Surface* surface);
```

### 4.3 Event Handling

```cpp
// Event types
enum EventType {
    KEY_PRESS,
    KEY_RELEASE,
    MOUSE_MOVE,
    MOUSE_BUTTON,
    MOUSE_WHEEL,
    FOCUS_GAIN,
    FOCUS_LOSE,
    RESIZE,
    CLOSE,
    EXPOSE
};

// Event loop
while (klat_surface_poll_event(surface, &event)) {
    // Handle event
}
```

---

## 5. Window Protocol

### 5.1 Messages (IPC)

```
Client → Server Messages:
├── create_window      Create new window
├── destroy_window     Destroy window
├── move_window        Move window position
├── resize_window      Resize window
├── set_title         Set window title
├── set_state         Set window state (maximized, minimized)
├── focus_window       Request focus
└── commit            Commit frame buffer

Server → Client Messages:
├── window_created     Window creation ACK
├── window_config      Configuration update
├── focus_gained       Focus gained
├── focus_lost         Focus lost
├── key_event          Keyboard event
├── mouse_event        Mouse event
└── close_request      Close requested
```

### 5.2 Window States

| State | Description |
|-------|-------------|
| `HIDDEN` | Window exists but not visible |
| `MINIMIZED` | Window minimized to taskbar |
| `NORMAL` | Normal windowed state |
| `MAXIMIZED` | Maximized to fill screen |
| `FULLSCREEN` | Fullscreen exclusive mode |

---

## 6. Rendering Pipeline

### 6.1 Pipeline Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      APPLICATION                              │
│  ┌─────────────┐                                            │
│  │   Canvas    │  ← Skia Graphics Library                   │
│  └──────┬──────┘                                            │
└──────────┼────────────────────────────────────────────────────┘
           │ draw()
           ▼
┌─────────────────────────────────────────────────────────────┐
│                      SURFACE                                 │
│  ┌─────────────┐                                            │
│  │   Buffer    │  ← Shared memory / DRM bo                    │
│  └──────┬──────┘                                            │
└──────────┼────────────────────────────────────────────────────┘
           │ commit()
           ▼
┌─────────────────────────────────────────────────────────────┐
│                    COMPOSITOR                               │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   Render     │→│  Composite   │→│  Present     │        │
│  │   Engine     │  │  (GPU)      │  │  (Queue)    │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────┐
│                    HARDWARE                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │    DRM      │→│   Vulkan     │→│   Display    │        │
│  │   (Kernel)  │  │   Driver    │  │   Output    │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 Rendering Options

| Backend | Description | Status |
|---------|-------------|--------|
| Skia | 2D Graphics Library | Planned |
| Vulkan | GPU-accelerated rendering | Planned |
| Software | Fallback rendering | Future |

---

## 7. Input System

### 7.1 Input Devices

| Device | Events |
|--------|--------|
| Keyboard | Key press, key release, modifiers |
| Mouse | Move, button, wheel, hover |
| Touch | Touch, multi-touch, gesture |
| Tablet | Stylus, pressure |

### 7.2 Input Routing

```
┌─────────────────────────────────────────────────────────────┐
│                     INPUT DEVICE                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │  Keyboard   │  │   Mouse     │  │   Touch     │      │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘      │
└─────────┼─────────────────┼─────────────────┼──────────────┘
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      INPUT MANAGER                          │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐             │  │
│  │  │ Hotplug │  │  Input  │  │  Focus  │             │  │
│  │  │ Manager │→│  Router │→│ Manager │             │  │
│  │  └─────────┘  └─────────┘  └─────────┘             │  │
│  └─────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   FOCUSED WINDOW                            │
│  Events delivered to the focused application surface        │
└─────────────────────────────────────────────────────────────┘
```

---

## 8. Theme System

### 8.1 Design Tokens

```cpp
struct Theme {
    // Spacing
    int spacing_xs;      // 4px
    int spacing_sm;       // 8px
    int spacing_md;       // 16px
    int spacing_lg;       // 24px
    int spacing_xl;       // 32px
    
    // Sizing
    int corner_radius;    // 8px
    int border_width;      // 1px
    
    // Animation
    int anim_duration_fast;    // 150ms
    int anim_duration_normal;  // 250ms
    int anim_duration_slow;    // 400ms
    
    // Typography
    const char* font_family;
    int font_size_sm;
    int font_size_md;
    int font_size_lg;
    
    // Colors
    Color accent;
    Color background;
    Color surface;
    Color text_primary;
    Color text_secondary;
    Color border;
    Color error;
    Color warning;
    Color success;
};
```

### 8.2 Widget Style

| Widget | Default | Hover | Active | Disabled |
|--------|---------|-------|--------|---------|
| Button | surface | surface + border | accent | 50% opacity |
| Input | surface | border = accent | border = accent | 50% opacity |
| Slider | accent | accent + glow | accent | 50% opacity |

---

## 9. Security Model

### 9.1 Surface Isolation

Each application surface is isolated:
- Separate GPU command buffer
- Separate memory allocation
- No direct memory access to other surfaces

### 9.2 IPC Security

- Validated message format
- Size-limited messages
- Rate limiting on events
- Capability-based permissions

---

## 10. Dependencies

### 10.1 External Dependencies

| Library | Purpose | Version |
|---------|---------|---------|
| Skia | 2D Graphics | Latest stable |
| Vulkan | GPU Rendering | 1.3+ |
| GBM | Buffer Management | Latest |
| libinput | Input Devices | Latest |

### 10.2 Internal Dependencies

```
kernel/
    └── GPU Driver (M3.4+)

runtime/
    └── libc (R2+)
    └── libpthread (R3+)

gui/
    ├── klat-wm
    ├── compositor
    └── klat-desktop
```

---

## 11. Related Documents

- [Protocol Specification](./PROTOCOL.md)
- [Rendering Pipeline](./RENDERING.md)
- [Input System](./INPUT.md)
- [Theme Specification](./THEMES.md)
- [Window API](./WINDOWS.md)

---

## 12. Change Log

| Date | Version | Changes |
|------|--------|---------|
| 2026-07-08 | G1.0 | Initial draft |

---

**Status**: Draft for Review  
**Next Step**: Review and finalize specifications before implementation  
**Maintainer**: KLAT OS Team
