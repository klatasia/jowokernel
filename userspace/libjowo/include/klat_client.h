// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT

#pragma once

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// =======================================================
// KLAT IPC Protokol (Komunikasi ke Compositor)
// =======================================================

typedef enum {
    KLAT_CMD_CREATE_WINDOW = 1,
    KLAT_CMD_DESTROY_WINDOW,
    KLAT_CMD_BIND_SURFACE, // Mengirim VMO handle ke compositor
    KLAT_CMD_PRESENT,
    KLAT_CMD_INPUT_EVENT,  // Diterima dari compositor
    KLAT_CMD_SET_CURSOR,
    KLAT_CMD_CLIPBOARD_WRITE,
    KLAT_CMD_CLIPBOARD_READ,
    KLAT_CMD_PING
} klat_cmd_t;

typedef enum {
    KLAT_EVENT_POINTER_MOTION = 1,
    KLAT_EVENT_POINTER_BUTTON,
    KLAT_EVENT_POINTER_WHEEL,
    KLAT_EVENT_KEYBOARD_PRESS,
    KLAT_EVENT_KEYBOARD_RELEASE,
    KLAT_EVENT_WINDOW_FOCUS_IN,
    KLAT_EVENT_WINDOW_FOCUS_OUT,
    KLAT_EVENT_WINDOW_CLOSE,
    KLAT_EVENT_CLIPBOARD_DATA
} klat_event_type_t;

typedef struct {
    uint32_t type;
    uint32_t window_id;
    int32_t x;
    int32_t y;
    uint32_t button_or_keycode;
    uint32_t state; // 1 = pressed/down, 0 = released/up
} klat_event_t;

typedef struct {
    uint32_t command;
    uint32_t window_id;
    int32_t arg1; // Width / X / Damage X
    int32_t arg2; // Height / Y / Damage Y
    int32_t arg3; // Format / Damage W
    int32_t arg4; // Stride / Damage H
} klat_message_t;

// =======================================================
// Obyek Inti Klien
// =======================================================

typedef struct _klat_connection klat_connection_t;
typedef struct _klat_window klat_window_t;
typedef struct _klat_surface klat_surface_t;

struct _klat_connection {
    int channel_fd; // Endpoint channel
    uint32_t next_window_id;
};

struct _klat_window {
    klat_connection_t* conn;
    uint32_t id;
    int width;
    int height;
    klat_surface_t* current_surface;
};

struct _klat_surface {
    klat_window_t* window;
    int vmo_handle;
    uint32_t* pixel_data;
    int width;
    int height;
    int stride;
};

// =======================================================
// Public API
// =======================================================

// Terhubung ke KLAT Compositor
klat_connection_t* klat_connect(void);
void klat_disconnect(klat_connection_t* conn);

// Manajemen Window
klat_window_t* klat_create_window(klat_connection_t* conn, int width, int height);
void klat_destroy_window(klat_window_t* window);

// Manajemen Surface (Pixel Buffer Berbasis VMO)
klat_surface_t* klat_create_surface(klat_window_t* window, int width, int height);
void klat_destroy_surface(klat_surface_t* surface);

// Sinkronisasi Frame
// Menandai surface siap dipresentasikan oleh Compositor
int klat_present(klat_window_t* window, klat_surface_t* surface, int dx, int dy, int dw, int dh);

// Manajemen Kursor & Clipboard (Global)
void klat_set_cursor(klat_connection_t* conn, int cursor_type); // 0 = Arrow, 1 = Text, 2 = Hand
void klat_clipboard_write(klat_connection_t* conn, const char* text);
void klat_clipboard_request(klat_connection_t* conn);

// Polling Event dari Compositor
// Mengembalikan 1 jika ada event yang diambil, 0 jika timeout/kosong
int klat_poll_events(klat_connection_t* conn, int timeout_ms, klat_event_t* out_event);

#ifdef __cplusplus
}
#endif
