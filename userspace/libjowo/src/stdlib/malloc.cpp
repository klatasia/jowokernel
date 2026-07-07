#include <stdlib.h>
#include <jowo/syscalls.h>
#include <stddef.h>

// A very simple bump allocator backed by VMO for the userspace

static char* heap_base = NULL;
static size_t heap_size = 0;
static size_t heap_used = 0;

void* malloc(size_t size) {
    if (size == 0) return NULL;
    
    // Align size to 16 bytes
    size = (size + 15) & ~15;

    if (!heap_base || heap_used + size > heap_size) {
        // Allocate a new VMO for the heap. Let's say 2MB.
        size_t alloc_size = 2 * 1024 * 1024;
        if (size > alloc_size) alloc_size = (size + 4095) & ~4095;

        int handle = 0;
        // Map it somewhere in user space. Let's choose 0x40000000.
        // Wait, VMO create without physical doesn't take physical address.
        // It takes size, options, out_handle, virt_addr.
        // But what if 0x40000000 is already used? 
        // We'll use a dynamic virtual address starting from 0x60000000 and increment it.
        static uint64_t next_vaddr = 0x60000000;
        
        int res = sys_vmo_create(alloc_size, 3 /* READ | WRITE */, &handle, next_vaddr);
        if (res != 0) {
            return NULL; // OOM or error
        }

        heap_base = (char*)next_vaddr;
        heap_size = alloc_size;
        heap_used = 0;
        next_vaddr += alloc_size;
    }

    void* ptr = heap_base + heap_used;
    heap_used += size;
    return ptr;
}

void free(void* ptr) {
    // Basic bump allocator doesn't free individual blocks
    (void)ptr;
}

int atoi(const char* str) {
    int res = 0;
    int sign = 1;
    if (*str == '-') {
        sign = -1;
        str++;
    }
    while (*str >= '0' && *str <= '9') {
        res = res * 10 + (*str - '0');
        str++;
    }
    return res * sign;
}

void itoa(int n, char s[]) {
    int i = 0;
    int sign = n;
    if (sign < 0) n = -n;
    do {
        s[i++] = n % 10 + '0';
    } while ((n /= 10) > 0);
    if (sign < 0) s[i++] = '-';
    s[i] = '\0';
    
    // reverse
    int j = 0;
    int k = i - 1;
    while (j < k) {
        char temp = s[j];
        s[j] = s[k];
        s[k] = temp;
        j++;
        k--;
    }
}
