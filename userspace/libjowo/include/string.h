#ifndef JOWO_STRING_H
#define JOWO_STRING_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

size_t strlen(const char* str);
void* memcpy(void* dest, const void* src, size_t n);
void* memset(void* s, int c, size_t n);
char* strcpy(char* dest, const char* src);
int strcmp(const char* s1, const char* s2);

#ifdef __cplusplus
}
#endif

#endif // JOWO_STRING_H
