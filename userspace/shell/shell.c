/**
 * KLAT OS Simple Shell
 *
 * A minimal shell for KLAT OS userspace.
 * Provides basic command-line interface.
 */

#include "../libjowo/jowolibc.h"

#define CMD_BUF_SIZE 256
static char cmd_buffer[CMD_BUF_SIZE];

void print_prompt(void) {
    printf("klatos> ");
}

int read_command(char* buf, int max_len) {
    int pos = 0;
    char c;

    while (pos < max_len - 1) {
        ssize_t n = read(0, &c, 1);
        if (n <= 0) {
            break;
        }

        if (c == '\b' || c == 127) {
            if (pos > 0) {
                pos--;
                printf("\b \b");
            }
            continue;
        }

        if (c == '\r' || c == '\n') {
            printf("\n");
            break;
        }

        if (c == 3) {  // Ctrl+C
            printf("^C\n");
            return 0;
        }

        putchar(c);
        buf[pos++] = c;
    }

    buf[pos] = '\0';
    return pos;
}

int builtin_help(void) {
    printf("KLAT OS Shell - Built-in commands:\n");
    printf("  help    - Show this help\n");
    printf("  echo    - Print text\n");
    printf("  clear   - Clear screen\n");
    printf("  exit    - Exit shell\n");
    printf("  pwd     - Print working directory\n");
    printf("  ls      - List directory\n");
    printf("  cat     - Display file\n");
    printf("  ps      - Show processes\n");
    printf("  whoami  - Show current user\n");
    printf("  date    - Show current date\n");
    printf("  uname   - Show system info\n");
    printf("  clear   - Clear screen\n");
    return 0;
}

int builtin_echo(char** args, int argc) {
    for (int i = 1; i < argc; i++) {
        printf("%s", args[i]);
        if (i < argc - 1) printf(" ");
    }
    printf("\n");
    return 0;
}

int builtin_clear(void) {
    printf("\033[2J\033[H");
    return 0;
}

int builtin_pwd(void) {
    printf("/\n");
    return 0;
}

int builtin_ls(void) {
    printf("bin   dev   etc   home   lib   proc   root   sys   tmp   usr\n");
    return 0;
}

int builtin_ps(void) {
    printf("  PID TTY         TIME CMD\n");
    printf("    1 ttyS0     00:00:00 init\n");
    printf(" %4d ttyS0     00:00:00 shell\n", getpid());
    return 0;
}

int builtin_whoami(void) {
    printf("root\n");
    return 0;
}

int builtin_date(void) {
    printf("Thu Jul  7 00:00:00 UTC 2026\n");
    return 0;
}

int builtin_uname(void) {
    printf("KLAT OS 1.0.0 JowoKernel x86_64\n");
    return 0;
}

int parse_command(char* cmd, char** args, int max_args) {
    int argc = 0;
    int in_word = 0;

    for (int i = 0; cmd[i] && argc < max_args - 1; i++) {
        if (cmd[i] == ' ' || cmd[i] == '\t') {
            if (in_word) {
                cmd[i] = '\0';
                in_word = 0;
            }
        } else {
            if (!in_word) {
                args[argc++] = &cmd[i];
                in_word = 1;
            }
        }
    }

    args[argc] = NULL;
    return argc;
}

int execute_command(char** args, int argc) {
    if (argc == 0) return 0;

    if (strcmp(args[0], "help") == 0) return builtin_help();
    if (strcmp(args[0], "echo") == 0) return builtin_echo(args, argc);
    if (strcmp(args[0], "clear") == 0 || strcmp(args[0], "cls") == 0) return builtin_clear();
    if (strcmp(args[0], "exit") == 0 || strcmp(args[0], "quit") == 0) return 1;
    if (strcmp(args[0], "pwd") == 0) return builtin_pwd();
    if (strcmp(args[0], "ls") == 0 || strcmp(args[0], "dir") == 0) return builtin_ls();
    if (strcmp(args[0], "ps") == 0) return builtin_ps();
    if (strcmp(args[0], "whoami") == 0) return builtin_whoami();
    if (strcmp(args[0], "date") == 0) return builtin_date();
    if (strcmp(args[0], "uname") == 0) return builtin_uname();

    printf("%s: command not found\n", args[0]);
    return 127;
}

int main(int argc, char* argv[], char* envp[]) {
    (void)argc;
    (void)argv;
    (void)envp;

    printf("\nKLAT OS Shell v1.0.0\n");
    printf("Type 'help' for available commands.\n\n");

    char* args[32];

    while (1) {
        print_prompt();

        if (read_command(cmd_buffer, CMD_BUF_SIZE) <= 0) {
            break;
        }

        int argc = parse_command(cmd_buffer, args, 32);
        if (argc > 0) {
            int ret = execute_command(args, argc);
            if (ret == 1) {
                break;
            }
        }
    }

    printf("\nShell exited.\n");
    return 0;
}
