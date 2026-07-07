#pragma once
#include <cairo/cairo.h>
#include <stdint.h>
#include <stdlib.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <unistd.h>

// KLAT Surface Backend Abstraction
// Ini adalah jembatan antara Cairo Image Surface dan Framebuffer / Compositor KLAT OS.
// Di Tahap 5 ini, kita menargetkan buffer virtual di memory, namun dapat langsung di-blit
// ke Framebuffer fisik atau VMO di JowoKernel.

typedef struct _klat_surface {
    cairo_surface_t* cairo_surface;
    uint32_t* pixel_data;
    int width;
    int height;
    int stride;
    
    // File descriptor untuk integrasi dengan DRM/Framebuffer JowoKernel
    int fb_fd;
} klat_surface_t;

static inline klat_surface_t* klat_surface_create(int width, int height) {
    klat_surface_t* surface = (klat_surface_t*)malloc(sizeof(klat_surface_t));
    if (!surface) return NULL;

    surface->width = width;
    surface->height = height;
    surface->stride = cairo_format_stride_for_width(CAIRO_FORMAT_ARGB32, width);
    
    // Alokasi RAM userspace untuk buffer
    surface->pixel_data = (uint32_t*)calloc(height, surface->stride);
    if (!surface->pixel_data) {
        free(surface);
        return NULL;
    }

    surface->fb_fd = -1;
    surface->cairo_surface = cairo_image_surface_create_for_data(
        (unsigned char*)surface->pixel_data, 
        CAIRO_FORMAT_ARGB32, 
        width, height, surface->stride
    );

    return surface;
}

static inline klat_surface_t* klat_surface_create_for_framebuffer(const char* fb_dev, int width, int height) {
    klat_surface_t* surface = (klat_surface_t*)malloc(sizeof(klat_surface_t));
    if (!surface) return NULL;

    surface->width = width;
    surface->height = height;
    surface->stride = cairo_format_stride_for_width(CAIRO_FORMAT_ARGB32, width);
    
    // Coba buka Framebuffer JowoKernel
    surface->fb_fd = open(fb_dev, O_RDWR);
    if (surface->fb_fd >= 0) {
        // Mmap VMO dari Kernel (Ukuran = height * stride)
        surface->pixel_data = (uint32_t*)mmap(NULL, height * surface->stride, PROT_READ | PROT_WRITE, MAP_SHARED, surface->fb_fd, 0);
        if (surface->pixel_data == MAP_FAILED) {
            close(surface->fb_fd);
            surface->fb_fd = -1;
            surface->pixel_data = (uint32_t*)calloc(height, surface->stride); // Fallback
        }
    } else {
        surface->pixel_data = (uint32_t*)calloc(height, surface->stride);
    }

    surface->cairo_surface = cairo_image_surface_create_for_data(
        (unsigned char*)surface->pixel_data, 
        CAIRO_FORMAT_ARGB32, 
        width, height, surface->stride
    );

    return surface;
}

static inline cairo_t* klat_surface_create_context(klat_surface_t* surface) {
    if (!surface || !surface->cairo_surface) return NULL;
    return cairo_create(surface->cairo_surface);
}

static inline void klat_surface_destroy(klat_surface_t* surface) {
    if (!surface) return;
    
    if (surface->cairo_surface) {
        cairo_surface_destroy(surface->cairo_surface);
    }

    if (surface->fb_fd >= 0) {
        munmap(surface->pixel_data, surface->height * surface->stride);
        close(surface->fb_fd);
    } else {
        free(surface->pixel_data);
    }

    free(surface);
}
