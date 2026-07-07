#include <stdlib.h>
#include <jowo/syscalls.h>

// Dummy dso_handle for C++ static destructors
extern "C" void* __dso_handle = nullptr;

extern "C" int __cxa_atexit(void (*)(void*), void*, void*) {
    return 0; // We don't fully support atexit yet
}

extern "C" void __cxa_pure_virtual() {
    sys_write(1, "Pure virtual function called in userspace!\n", 43);
    sys_exit(-1);
}

// Global operator new and delete
void* operator new(size_t size) {
    return malloc(size);
}

void* operator new[](size_t size) {
    return malloc(size);
}

void operator delete(void* p) noexcept {
    free(p);
}

void operator delete(void* p, size_t) noexcept {
    free(p);
}

void operator delete[](void* p) noexcept {
    free(p);
}

void operator delete[](void* p, size_t) noexcept {
    free(p);
}
