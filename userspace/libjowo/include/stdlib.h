#ifndef JOWO_STDLIB_H
#define JOWO_STDLIB_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

void* malloc(size_t size);
void free(void* ptr);

// Other basic stdlib functions
int atoi(const char* str);
void itoa(int n, char s[]);

#ifdef __cplusplus
}
#endif

#endif // JOWO_STDLIB_H
