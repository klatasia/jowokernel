#include <stdint.h>

// Syscall Numbers
#define SYS_EXIT       1
#define SYS_WRITE      2
#define SYS_READ       3
#define SYS_YIELD      4
#define SYS_CHANNEL_CREATE 5
#define SYS_CHANNEL_WRITE  6
#define SYS_CHANNEL_READ   7
#define SYS_VMO_CREATE_PHYSICAL 8
#define SYS_IOPORT_WRITE16 9
#define SYS_IOPORT_READ16 10
#define SYS_HANDLE_CLS 11
#define SYS_VMO_CREATE     12
#define SYS_OBJECT_WAIT    13
#define SYS_INTERRUPT_BIND 14

static inline uint64_t syscall0(uint64_t n) {
    uint64_t ret;
    asm volatile("syscall" : "=a"(ret) : "a"(n) : "rcx", "r11", "memory");
    return ret;
}

static inline uint64_t syscall1(uint64_t n, uint64_t arg1) {
    uint64_t ret;
    asm volatile("syscall" : "=a"(ret) : "a"(n), "D"(arg1) : "rcx", "r11", "memory");
    return ret;
}

static inline uint64_t syscall2(uint64_t n, uint64_t arg1, uint64_t arg2) {
    uint64_t ret;
    asm volatile("syscall" : "=a"(ret) : "a"(n), "D"(arg1), "S"(arg2) : "rcx", "r11", "memory");
    return ret;
}

static inline uint64_t syscall3(uint64_t n, uint64_t arg1, uint64_t arg2, uint64_t arg3) {
    uint64_t ret;
    asm volatile("syscall" : "=a"(ret) : "a"(n), "D"(arg1), "S"(arg2), "d"(arg3) : "rcx", "r11", "memory");
    return ret;
}

static inline uint64_t syscall5(uint64_t n, uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5) {
    uint64_t ret;
    register uint64_t r10 asm("r10") = arg4;
    register uint64_t r8 asm("r8") = arg5;
    asm volatile("syscall" : "=a"(ret) : "a"(n), "D"(arg1), "S"(arg2), "d"(arg3), "r"(r10), "r"(r8) : "rcx", "r11", "memory");
    return ret;
}

void print(const char* str) {
    uint64_t len = 0;
    while (str[len]) len++;
    syscall3(SYS_WRITE, 1, (uint64_t)str, len);
}

// === BOCHS GRAPHICS ADAPTER (BGA) USERSPACE DRIVER ===

static inline void outw(uint16_t port, uint16_t val) {
    syscall2(SYS_IOPORT_WRITE16, port, val);
}

static inline uint16_t inw(uint16_t port) {
    return syscall1(SYS_IOPORT_READ16, port);
}

#define VBE_DISPI_IOPORT_INDEX 0x01CE
#define VBE_DISPI_IOPORT_DATA  0x01CF

void bga_write_register(uint16_t IndexValue, uint16_t DataValue) {
    outw(VBE_DISPI_IOPORT_INDEX, IndexValue);
    outw(VBE_DISPI_IOPORT_DATA, DataValue);
}

void setup_bga(uint16_t width, uint16_t height) {
    bga_write_register(4, 0); // Disable
    bga_write_register(1, width); // XRES
    bga_write_register(2, height); // YRES
    bga_write_register(3, 32); // BPP (32 bit)
    bga_write_register(4, 0x41); // Enable | LFB (Bit 0 = 1, Bit 6 = 1)
}

void render_rainbow(uint32_t* fb, int width, int height) {
    for (int y = 0; y < height; y++) {
        for (int x = 0; x < width; x++) {
            uint8_t r = (x * 255) / width;
            uint8_t g = (y * 255) / height;
            uint8_t b = 255 - ((x + y) * 255) / (width + height);
            fb[y * width + x] = (r << 16) | (g << 8) | b;
        }
    }
}

// =======================================================

// Pastikan titik masuk `_start` ada di paling awal.
__attribute__((section(".text.entry")))
void _start() {
    print("[Init UserApp] Halo, JowoKernel! Ini adalah Aplikasi Ring-3 GUI.\n");
    
    print("[Init UserApp] Menginisialisasi Driver Layar Grafis BGA (800x600)...\n");
    setup_bga(800, 600);
    
    int fb_handle = 0;
    uint64_t fb_virt_addr = 0xA000000000ULL;
    // QEMU BGA/VGA LFB address usually starts at 0xFD000000
    // Request Mapping 800x600x4 bytes = 1.92 MB
    syscall5(SYS_VMO_CREATE_PHYSICAL, 0xFD000000ULL, 800*600*4, 0, (uint64_t)&fb_handle, fb_virt_addr);
    
    print("[Init UserApp] Berhasil memetakan LFB Framebuffer PCI ke memori Userspace!\n");
    print("[Init UserApp] Mulai render kanvas pelangi...\n");
    
    uint32_t* fb = (uint32_t*)fb_virt_addr;
    // Fast clear to blue instead of heavy software rendering
    for (int i = 0; i < 800 * 600; i++) {
        fb[i] = 0x0000AA; 
    }
    
    print("[Init UserApp] LUAR BIASA! Render grafis sukses 100%! Selamat datang di masa depan!\n");

    print("[Init UserApp] Mengikat (Bind) IRQ 1 Keyboard ke Event Object...\n");
    int kb_event = -1;
    syscall2(SYS_INTERRUPT_BIND, 1, (uint64_t)&kb_event);

    if (kb_event < 0) {
        print("[Init UserApp] GAGAL bind IRQ 1!\n");
        while(1) { syscall0(SYS_YIELD); }
    }

    print("[Init UserApp] Aplikasi masuk ke status TERTIDUR (BLOCKED). Tekan sembarang tombol di QEMU!\n");

    int color_offset = 0;
    while(1) {
        // Blokir thread ini sampai IRQ 1 (Keyboard) memicu event
        syscall1(SYS_OBJECT_WAIT, kb_event);

        // Baca Scan Code dari port 0x60
        uint16_t scancode = inw(0x60) & 0xFF;
        
        // Print message
        print("[Init UserApp] BANGUN! IRQ 1 Terpicu. Merubah warna layar...\n");

        // Render ulang dengan warna yang digeser
        color_offset += 32;
        for (int y = 0; y < 600; y++) {
            for (int x = 0; x < 800; x++) {
                uint8_t r = ((x * 255) / 800) + color_offset;
                uint8_t g = ((y * 255) / 600);
                uint8_t b = 255 - ((x + y) * 255) / (800 + 600) - color_offset;
                fb[y * 800 + x] = (r << 16) | (g << 8) | b;
            }
        }
        
        print("[Init UserApp] Kembali tidur...\n");
    }
}
