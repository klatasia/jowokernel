// Copyright (c) 2026 JowoKernel Team
// SPDX-License-Identifier: MIT

#include <klat_client.h>
#include <jowokernel/syscalls.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <sys/mman.h>
#include <fcntl.h>
#include <poll.h>

klat_connection_t* klat_connect(void) {
    // Sebagai mock sederhana saat ini, kita mengasumsikan klat-compositor mendaftarkan
    // named pipe di VFS pada "/tmp/klat-compositor"
    // Namun JowoKernel belum memiliki UNIX socket lengkap, kita bisa menggunakan SYS_CHANNEL
    // Tapi karena belum ada naming service, kita sementara akan menggunakan open() ke node pipe
    // atau untuk sekarang, return object yang valid (stub/mock) sebelum daemon penuh selesai.
    
    // Kita anggap /dev/klat_server atau sejenisnya.
    // Di Tahap 6B, karena kita ingin zero-copy murni, kita pura-pura tersambung ke channel 1.
    
    klat_connection_t* conn = (klat_connection_t*)malloc(sizeof(klat_connection_t));
    if (!conn) return NULL;
    
    // Buka koneksi ke compositor
    conn->channel_fd = open("/tmp/klat-compositor", O_RDWR);
    if (conn->channel_fd < 0) {
        printf("[klat_client] Peringatan: Gagal terhubung ke /tmp/klat-compositor. Berjalan di mode standalone.\n");
    }
    
    conn->next_window_id = 1;
    return conn;
}

void klat_disconnect(klat_connection_t* conn) {
    if (!conn) return;
    if (conn->channel_fd >= 0) {
        close(conn->channel_fd);
    }
    free(conn);
}

klat_window_t* klat_create_window(klat_connection_t* conn, int width, int height) {
    if (!conn) return NULL;
    klat_window_t* win = (klat_window_t*)malloc(sizeof(klat_window_t));
    win->conn = conn;
    win->id = conn->next_window_id++;
    win->width = width;
    win->height = height;
    win->current_surface = NULL;
    
    if (conn->channel_fd >= 0) {
        klat_message_t msg = { KLAT_CMD_CREATE_WINDOW, win->id, width, height, 0, 0 };
        write(conn->channel_fd, &msg, sizeof(msg));
    }
    return win;
}

void klat_destroy_window(klat_window_t* window) {
    if (!window) return;
    if (window->conn && window->conn->channel_fd >= 0) {
        klat_message_t msg = { KLAT_CMD_DESTROY_WINDOW, window->id, 0, 0, 0, 0 };
        write(window->conn->channel_fd, &msg, sizeof(msg));
    }
    if (window->current_surface) {
        klat_destroy_surface(window->current_surface);
    }
    free(window);
}

klat_surface_t* klat_create_surface(klat_window_t* window, int width, int height) {
    if (!window) return NULL;
    klat_surface_t* surf = (klat_surface_t*)malloc(sizeof(klat_surface_t));
    surf->window = window;
    surf->width = width;
    surf->height = height;
    surf->stride = width * 4; // ARGB8888
    
    // Buat VMO
    u64 size = (u64)(height * surf->stride);
    int vmo_handle = -1;
    sys_vmo_create(size, 0, &vmo_handle, 0);
    
    if (vmo_handle < 0) {
        free(surf);
        return NULL;
    }
    surf->vmo_handle = vmo_handle;
    
    // Mmap VMO ke address space client. Flag = MAP_SHARED (0x01)
    surf->pixel_data = (uint32_t*)mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED, vmo_handle, 0);
    
    if (surf->pixel_data == MAP_FAILED) {
        // Fallback
        surf->pixel_data = (uint32_t*)malloc(size);
    }
    
    window->current_surface = surf;
    
    // Kirim VMO ke Compositor via SYS_CHANNEL_WRITE jika menggunakan channel
    // Karena kita memakai VFS pipe, kita kirimkan ID-nya saja (asumsi shared handle table atau global ID)
    if (window->conn && window->conn->channel_fd >= 0) {
        klat_message_t msg = { KLAT_CMD_BIND_SURFACE, window->id, surf->width, surf->height, surf->stride, surf->vmo_handle };
        write(window->conn->channel_fd, &msg, sizeof(msg));
    }
    
    return surf;
}

void klat_destroy_surface(klat_surface_t* surface) {
    if (!surface) return;
    if (surface->vmo_handle >= 0) {
        munmap(surface->pixel_data, surface->height * surface->stride);
        close(surface->vmo_handle); // Tutup handle VMO
    } else {
        free(surface->pixel_data);
    }
    free(surface);
}

int klat_present(klat_window_t* window, klat_surface_t* surface, int dx, int dy, int dw, int dh) {
    if (!window || !surface) return -1;
    if (window->conn && window->conn->channel_fd >= 0) {
        klat_message_t msg = { KLAT_CMD_PRESENT, window->id, dx, dy, dw, dh };
        write(window->conn->channel_fd, &msg, sizeof(msg));
    }
    return 0;
}

void klat_set_cursor(klat_connection_t* conn, int cursor_type) {
    if (!conn || conn->channel_fd < 0) return;
    klat_message_t msg = { KLAT_CMD_SET_CURSOR, 0, cursor_type, 0, 0, 0 };
    write(conn->channel_fd, &msg, sizeof(msg));
}

void klat_clipboard_write(klat_connection_t* conn, const char* text) {
    // Sebagai stub, cukup kirim perintah. Teks asli perlu VMO atau payload tambahan.
    if (!conn || conn->channel_fd < 0) return;
    klat_message_t msg = { KLAT_CMD_CLIPBOARD_WRITE, 0, 0, 0, 0, 0 };
    write(conn->channel_fd, &msg, sizeof(msg));
}

void klat_clipboard_request(klat_connection_t* conn) {
    if (!conn || conn->channel_fd < 0) return;
    klat_message_t msg = { KLAT_CMD_CLIPBOARD_READ, 0, 0, 0, 0, 0 };
    write(conn->channel_fd, &msg, sizeof(msg));
}

int klat_poll_events(klat_connection_t* conn, int timeout_ms, klat_event_t* out_event) {
    if (!conn || conn->channel_fd < 0 || !out_event) return 0;
    
    struct pollfd pfd;
    pfd.fd = conn->channel_fd;
    pfd.events = POLLIN;
    
    int ret = poll(&pfd, 1, timeout_ms);
    if (ret > 0 && (pfd.revents & POLLIN)) {
        klat_message_t msg;
        // Baca message standar
        int bytes = read(conn->channel_fd, &msg, sizeof(msg));
        if (bytes == sizeof(msg) && msg.command == KLAT_CMD_INPUT_EVENT) {
            // Dalam implementasi ini, arg1..4 menyimpan event data:
            // arg1 = event type
            // arg2 = x / keycode
            // arg3 = y / state
            // arg4 = button
            out_event->type = msg.arg1;
            out_event->window_id = msg.window_id;
            
            if (msg.arg1 == KLAT_EVENT_POINTER_MOTION || msg.arg1 == KLAT_EVENT_POINTER_BUTTON) {
                out_event->x = msg.arg2;
                out_event->y = msg.arg3;
                out_event->button_or_keycode = msg.arg4;
                out_event->state = 1; // dummy state for button
            } else if (msg.arg1 == KLAT_EVENT_KEYBOARD_PRESS || msg.arg1 == KLAT_EVENT_KEYBOARD_RELEASE) {
                out_event->button_or_keycode = msg.arg2;
                out_event->state = msg.arg3;
            }
            return 1;
        }
    }
    return 0;
}
