#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <jowo/syscalls.h>
#include <stdarg.h>

static void print_char(char c) {
    sys_write(1, &c, 1);
}

static void print_string(const char* str) {
    sys_write(1, str, strlen(str));
}

static void print_int(int val, int base) {
    char buf[32];
    if (val == 0) {
        print_char('0');
        return;
    }
    
    int is_neg = 0;
    if (val < 0 && base == 10) {
        is_neg = 1;
        val = -val;
    }
    
    int i = 0;
    unsigned int uval = (unsigned int)val;
    while (uval > 0) {
        int rem = uval % base;
        buf[i++] = (rem < 10) ? (rem + '0') : (rem - 10 + 'a');
        uval /= base;
    }
    
    if (is_neg) {
        buf[i++] = '-';
    }
    
    while (i > 0) {
        print_char(buf[--i]);
    }
}

int printf(const char* format, ...) {
    va_list args;
    va_start(args, format);
    
    while (*format) {
        if (*format == '%') {
            format++;
            switch (*format) {
                case 's': {
                    const char* s = va_arg(args, const char*);
                    print_string(s ? s : "(null)");
                    break;
                }
                case 'd': {
                    int val = va_arg(args, int);
                    print_int(val, 10);
                    break;
                }
                case 'x': {
                    int val = va_arg(args, int);
                    print_int(val, 16);
                    break;
                }
                case 'c': {
                    char c = (char)va_arg(args, int);
                    print_char(c);
                    break;
                }
                case '%': {
                    print_char('%');
                    break;
                }
                default: {
                    print_char('%');
                    print_char(*format);
                    break;
                }
            }
        } else {
            print_char(*format);
        }
        format++;
    }
    
    va_end(args);
    return 0; // standard printf returns num of chars printed
}

int puts(const char* s) {
    print_string(s);
    print_char('\n');
    return 0;
}

int putchar(int c) {
    print_char((char)c);
    return c;
}
