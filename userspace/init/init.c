/**
 * KLAT OS Init Process
 *
 * The first userspace process spawned by the kernel.
 * Responsible for setting up the system and launching the shell.
 */

#include "../libjowo/jowolibc.h"

// Simple string buffer for commands
#define CMD_BUF_SIZE 256
static char cmd_buffer[CMD_BUF_SIZE];
static int cmd_pos = 0;

void print_banner(void) {
    printf("\n");
    printf("====================================\n");
    printf("   KLAT OS v1.0.0\n");
    printf("   JowoKernel - Desktop Microkernel\n");
    printf("====================================\n");
    printf("\n");
}

void print_prompt(void) {
    printf("root@klatos:/# ");
}

int read_command(char* buf, int max_len) {
    int pos = 0;
    char c;

    while (pos < max_len - 1) {
        // Read one character
        ssize_t n = read(0, &c, 1);
        if (n <= 0) {
            break;
        }

        // Handle backspace
        if (c == '\b' || c == 127) {
            if (pos > 0) {
                pos--;
                printf("\b \b");
            }
            continue;
        }

        // Handle Enter
        if (c == '\r' || c == '\n') {
            printf("\n");
            break;
        }

        // Handle Ctrl+C
        if (c == 3) {
            printf("^C\n");
            pos = 0;
            break;
        }

        // Echo and store
        putchar(c);
        buf[pos++] = c;
    }

    buf[pos] = '\0';
    return pos;
}

int builtin_echo(char** args, int argc) {
    for (int i = 1; i < argc; i++) {
        printf("%s", args[i]);
        if (i < argc - 1) printf(" ");
    }
    printf("\n");
    return 0;
}

int builtin_help(char** args, int argc) {
    printf("KLAT OS Shell Commands:\n");
    printf("  help     - Show this help message\n");
    printf("  echo     - Print text to stdout\n");
    printf("  clear    - Clear the screen\n");
    printf("  exit     - Exit the shell\n");
    printf("  pwd      - Print working directory\n");
    printf("  ls       - List directory contents\n");
    printf("  cat      - Display file contents\n");
    printf("  ps       - Show running processes\n");
    printf("  meminfo  - Show memory information\n");
    printf("  hostname - Show system hostname\n");
    printf("  uptime   - Show system uptime\n");
    printf("  version  - Show KLAT OS version\n");
    return 0;
}

int builtin_clear(char** args, int argc) {
    (void)args;
    (void)argc;
    printf("\033[2J\033[H");  // ANSI escape sequence
    return 0;
}

int builtin_exit(char** args, int argc) {
    (void)args;
    (void)argc;
    printf("Goodbye from KLAT OS!\n");
    exit(0);
    return 0;
}

int builtin_pwd(char** args, int argc) {
    (void)args;
    (void)argc;
    printf("/\n");
    return 0;
}

int builtin_ls(char** args, int argc) {
    (void)args;
    // Open directory or root
    const char* path = "/";
    if (argc > 1) {
        path = args[1];
    }

    int fd = open(path, 0, 0);
    if (fd < 0) {
        printf("ls: cannot access '%s': No such file or directory\n", path);
        return 1;
    }

    // For now, just print some entries
    printf("bin   dev   etc   home   lib   proc   root   sys   tmp   usr\n");

    close(fd);
    return 0;
}

int builtin_cat(char** args, int argc) {
    if (argc < 2) {
        printf("cat: missing file operand\n");
        return 1;
    }

    int fd = open(args[1], 0, 0);
    if (fd < 0) {
        printf("cat: %s: No such file or directory\n", args[1]);
        return 1;
    }

    char buf[256];
    ssize_t n;
    while ((n = read(fd, buf, sizeof(buf) - 1)) > 0) {
        buf[n] = '\0';
        printf("%s", buf);
    }

    close(fd);
    return 0;
}

int builtin_ps(char** args, int argc) {
    (void)args;
    (void)argc;
    printf("  PID   TTY         TIME CMD\n");
    printf("    1   ttyS0     00:00:00 init\n");
    return 0;
}

int builtin_meminfo(char** args, int argc) {
    (void)args;
    (void)argc;
    printf("Memory Information:\n");
    printf("  Total:      4 GB\n");
    printf("  Free:       3.5 GB\n");
    printf("  Available:  3.2 GB\n");
    printf("  Buffers:    128 MB\n");
    return 0;
}

int builtin_hostname(char** args, int argc) {
    (void)args;
    (void)argc;
    printf("klatos\n");
    return 0;
}

int builtin_uptime(char** args, int argc) {
    (void)args;
    (void)argc;
    printf("  System running for 0 minutes\n");
    return 0;
}

int builtin_version(char** args, int argc) {
    (void)args;
    (void)argc;
    printf("KLAT OS v1.0.0\n");
    printf("JowoKernel v1.0.0\n");
    printf("Build: 2026-07-07\n");
    return 0;
}

// Parse command line into arguments
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
    if (argc == 0) {
        return 0;
    }

    // Built-in commands
    if (strcmp(args[0], "help") == 0) {
        return builtin_help(args, argc);
    }
    if (strcmp(args[0], "echo") == 0) {
        return builtin_echo(args, argc);
    }
    if (strcmp(args[0], "clear") == 0 || strcmp(args[0], "cls") == 0) {
        return builtin_clear(args, argc);
    }
    if (strcmp(args[0], "exit") == 0 || strcmp(args[0], "logout") == 0) {
        return builtin_exit(args, argc);
    }
    if (strcmp(args[0], "pwd") == 0) {
        return builtin_pwd(args, argc);
    }
    if (strcmp(args[0], "ls") == 0 || strcmp(args[0], "dir") == 0) {
        return builtin_ls(args, argc);
    }
    if (strcmp(args[0], "cat") == 0) {
        return builtin_cat(args, argc);
    }
    if (strcmp(args[0], "ps") == 0) {
        return builtin_ps(args, argc);
    }
    if (strcmp(args[0], "meminfo") == 0 || strcmp(args[0], "free") == 0) {
        return builtin_meminfo(args, argc);
    }
    if (strcmp(args[0], "hostname") == 0) {
        return builtin_hostname(args, argc);
    }
    if (strcmp(args[0], "uptime") == 0) {
        return builtin_uptime(args, argc);
    }
    if (strcmp(args[0], "version") == 0 || strcmp(args[0], "ver") == 0) {
        return builtin_version(args, argc);
    }

    // Unknown command
    printf("%s: command not found\n", args[0]);
    return 127;
}

// Main shell loop
void shell_loop(void) {
    char* args[32];

    print_banner();

    while (1) {
        print_prompt();

        if (read_command(cmd_buffer, CMD_BUF_SIZE) <= 0) {
            break;
        }

        int argc = parse_command(cmd_buffer, args, 32);
        if (argc > 0) {
            execute_command(args, argc);
        }
    }
}

// Init process main
int main(int argc, char* argv[], char* envp[]) {
    (void)argc;
    (void)argv;
    (void)envp;

    // Basic initialization
    printf("[Init] KLAT OS Init Process Starting...\n");
    printf("[Init] PID: %d\n", getpid());
    printf("[Init] Setting up system...\n");

    // Run filesystem checks
    printf("[Init] Checking filesystems...\n");
    int fd = open("/", 0, 0);
    if (fd >= 0) {
        printf("[Init] Root filesystem mounted.\n");
        close(fd);
    } else {
        printf("[Init] Warning: Could not access root filesystem.\n");
    }

    // Mount virtual filesystems
    printf("[Init] Mounting virtual filesystems...\n");
    // In a real system: mount("/proc", "/proc", "proc", 0, NULL);

    // Set hostname
    printf("[Init] Setting hostname to 'klatos'...\n");

    // Start shell
    printf("[Init] Starting shell...\n");
    printf("\n");

    shell_loop();

    printf("\n[Init] Shutting down...\n");
    return 0;
}
