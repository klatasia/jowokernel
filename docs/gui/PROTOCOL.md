# KLAT GUI Window Protocol Specification

**Version**: G1.0  
**Status**: Draft - Ready for Review  
**Last Updated**: 2026-07-08  

---

## 1. Overview

Dokumen ini mendefinisikan protokol komunikasi antara aplikasi (client) dan window server (klat-wm). Protokol ini adalah ABI dan tidak akan berubah tanpa versioning.

---

## 2. Message Format

### 2.1 Header

```
┌─────────────────────────────────────────────────────────────────┐
│                        Message Header                            │
├─────────┬─────────┬─────────┬───────────────────────────────────┤
│ Size    │ Type    │ Flags   │ Sequence Number                   │
│ 4 bytes │ 2 bytes │ 2 bytes │ 4 bytes                           │
└─────────┴─────────┴─────────┴───────────────────────────────────┘
```

### 2.2 Message Types

```cpp
enum MessageType : uint16_t {
    // Client → Server
    CREATE_WINDOW      = 0x1001,
    DESTROY_WINDOW     = 0x1002,
    MOVE_WINDOW         = 0x1003,
    RESIZE_WINDOW       = 0x1004,
    SET_TITLE          = 0x1005,
    SET_STATE          = 0x1006,
    FOCUS_WINDOW       = 0x1007,
    COMMIT             = 0x1008,
    SET_WORKSPACE      = 0x1009,
    
    // Server → Client
    WINDOW_CREATED     = 0x2001,
    WINDOW_DESTROYED    = 0x2002,
    WINDOW_CONFIG       = 0x2003,
    FOCUS_GAINED       = 0x2004,
    FOCUS_LOST         = 0x2005,
    KEY_EVENT          = 0x2006,
    MOUSE_EVENT        = 0x2007,
    CLOSE_REQUEST      = 0x2008,
    PING               = 0x2009,
    PONG               = 0x200A,
};
```

---

## 3. Client → Server Messages

### 3.1 Create Window

```
Message: CREATE_WINDOW (0x1001)
Payload:
┌─────────┬─────────┬─────────┬─────────┬─────────┬─────────┐
│ X       │ Y       │ Width   │ Height  │ Type    │ Flags   │
│ 4 bytes │ 4 bytes │ 4 bytes │ 4 bytes │ 4 bytes │ 4 bytes │
└─────────┴─────────┴─────────┴─────────┴─────────┴─────────┘

Title: NULL-terminated UTF-8 string (variable)

Response: WINDOW_CREATED
┌─────────┬─────────┬─────────┐
│ WindowID│ ConfigX│ ConfigY│
│ 4 bytes │ 4 bytes│ 4 bytes│
└─────────┴─────────┴─────────┘
┌─────────┬─────────┬─────────┐
│ ConfigW│ ConfigH │ State  │
│ 4 bytes│ 4 bytes│ 4 bytes│
└─────────┴─────────┴─────────┘
```

### 3.2 Destroy Window

```
Message: DESTROY_WINDOW (0x1002)
Payload:
┌─────────┐
│ WindowID│
│ 4 bytes│
└─────────┘

Response: WINDOW_DESTROYED
┌─────────┐
│ WindowID│
│ 4 bytes│
└─────────┘
```

### 3.3 Move Window

```
Message: MOVE_WINDOW (0x1003)
Payload:
┌─────────┬─────────┬─────────┐
│ WindowID│ X       │ Y       │
│ 4 bytes│ 4 bytes│ 4 bytes│
└─────────┴─────────┴─────────┘

Response: WINDOW_CONFIG
```

### 3.4 Resize Window

```
Message: RESIZE_WINDOW (0x1004)
Payload:
┌─────────┬─────────┬─────────┬─────────┬─────────┐
│ WindowID│ Width   │ Height  │ Edges   │ Flags   │
│ 4 bytes│ 4 bytes│ 4 bytes│ 4 bytes│ 4 bytes│
└─────────┴─────────┴─────────┴─────────┴─────────┘

Edges:
- 0x01: Left
- 0x02: Right
- 0x04: Top
- 0x08: Bottom
- 0x10: Move (all edges)
- 0x20: Size (resize only)
```

### 3.5 Set Title

```
Message: SET_TITLE (0x1005)
Payload:
┌─────────┐
│ WindowID│
│ 4 bytes│
└─────────┘
Title: NULL-terminated UTF-8 string
```

### 3.6 Set State

```
Message: SET_STATE (0x1006)
Payload:
┌─────────┬─────────┐
│ WindowID│ State   │
│ 4 bytes│ 4 bytes│
└─────────┴─────────┘

State values:
- 0: HIDDEN
- 1: MINIMIZED
- 2: NORMAL
- 3: MAXIMIZED
- 4: FULLSCREEN
```

### 3.7 Commit Frame

```
Message: COMMIT (0x1008)
Payload:
┌─────────┬─────────┬─────────┬─────────┐
│ WindowID│ BufferFD│ Width   │ Height  │
│ 4 bytes│ 4 bytes │ 4 bytes│ 4 bytes│
└─────────┴─────────┴─────────┴─────────┘
┌─────────┬─────────┐
│ Format │ Flags   │
│ 4 bytes│ 4 bytes│
└─────────┴─────────┘

Format:
- 0: RGBA8
- 1: BGRA8
- 2: RGBX8
- 3: BGRX8
```

---

## 4. Server → Client Messages

### 4.1 Key Event

```
Message: KEY_EVENT (0x2006)
Payload:
┌─────────┬─────────┬─────────┬─────────┬─────────┐
│ WindowID│ Type    │ Keycode │ Modifiers│ Repeat │
│ 4 bytes│ 4 bytes │ 4 bytes │ 4 bytes │ 2 bytes│
└─────────┴─────────┴─────────┴─────────┴─────────┘
┌─────────┐
│ Unicode│  (UTF-32)
│ 4 bytes│
└─────────┘

Type:
- 0: KEY_PRESS
- 1: KEY_RELEASE

Modifiers:
- 0x01: Shift
- 0x02: Ctrl
- 0x04: Alt
- 0x08: Super
- 0x10: CapsLock
- 0x20: NumLock
```

### 4.2 Mouse Event

```
Message: MOUSE_EVENT (0x2007)
Payload:
┌─────────┬─────────┬─────────┬─────────┬─────────┐
│ WindowID│ Type    │ X       │ Y       │ Modifiers│
│ 4 bytes│ 4 bytes │ 4 bytes │ 4 bytes │ 4 bytes │
└─────────┴─────────┴─────────┴─────────┴─────────┘
┌─────────┬─────────┬─────────┬─────────┐
│ Button  │ WheelX  │ WheelY  │ RootX   │
│ 1 byte  │ 4 bytes │ 4 bytes │ 4 bytes │
└─────────┴─────────┴─────────┴─────────┘
┌─────────┐
│ RootY  │
│ 4 bytes│
└─────────┘

Type:
- 0: MOTION
- 1: BUTTON_PRESS
- 2: BUTTON_RELEASE
- 3: ENTER
- 4: LEAVE
- 5: WHEEL

Button:
- 0: NONE
- 1: LEFT
- 2: MIDDLE
- 3: RIGHT
- 4: SCROLL_UP
- 5: SCROLL_DOWN
- 6: SCROLL_LEFT
- 7: SCROLL_RIGHT
```

---

## 5. Error Handling

### 5.1 Error Codes

```cpp
enum ErrorCode : uint32_t {
    SUCCESS              = 0,
    INVALID_WINDOW      = 1,
    INVALID_PARAMETER   = 2,
    WINDOW_LIMIT        = 3,
    BUFFER_ERROR        = 4,
    CONNECTION_LOST      = 5,
    TIMEOUT             = 6,
    PERMISSION_DENIED    = 7,
};
```

### 5.2 Error Message

```
Message: ERROR (0xF001)
Payload:
┌─────────┬─────────┬─────────┐
│ Original│ Error   │ Message │
│ Type    │ Code    │ String  │
│ 2 bytes│ 4 bytes │ var     │
└─────────┴─────────┴─────────┘
```

---

## 6. Connection Protocol

### 6.1 Connection Sequence

```
1. Client connects to klat-wm socket
2. Client sends HELLO with version
3. Server sends HELLO_ACK or ERROR
4. Client can now send/receive messages
5. On disconnect, server cleans up resources
```

### 6.2 Ping/Pong

```
Server periodically sends PING
Client must respond with PONG within 5 seconds
Failure results in disconnection
```

---

## 7. Versioning

### 7.1 Version Negotiation

```cpp
// Client sends:
struct Hello {
    uint32_t version_major;
    uint32_t version_minor;
    uint32_t version_patch;
};

// Server responds:
struct HelloAck {
    uint32_t accepted_major;
    uint32_t accepted_minor;
    uint32_t accepted_patch;
};
```

---

## 8. Change Log

| Date | Version | Changes |
|------|---------|---------|
| 2026-07-08 | G1.0 | Initial protocol specification |

---

**Status**: Draft for Review  
**Maintainer**: KLAT OS Team
