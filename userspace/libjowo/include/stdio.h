#ifndef JOWO_STDIO_H
#define JOWO_STDIO_H

#include <stddef.h>
#include <stdarg.h>

#ifdef __cplusplus
extern "C" {
#endif

int printf(const char* format, ...);
int sprintf(char* str, const char* format, ...);
int snprintf(char* str, size_t size, const char* format, ...);
int puts(const char* s);
int putchar(int c);

#ifdef __cplusplus
}
#endif

#endif // JOWO_STDIO_H
