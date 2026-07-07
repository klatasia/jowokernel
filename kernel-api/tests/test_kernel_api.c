/**
 * @file test_kernel_api.c
 * @brief Unit tests for JowoKernel API
 *
 * Compile with:
 *   clang -I.. -DTEST -o test_kernel_api test_kernel_api.c
 *
 * Run:
 *   ./test_kernel_api
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

#include "../kernel_api.h"

// ============================================================
// Test Utilities
// ============================================================

static int tests_run = 0;
static int tests_passed = 0;

#define TEST(name) void test_##name(void)
#define RUN_TEST(name) do { \
    printf("  Running %s... ", #name); \
    tests_run++; \
    test_##name(); \
    printf("PASSED\n"); \
    tests_passed++; \
} while(0)

#define ASSERT(cond) do { \
    if (!(cond)) { \
        printf("FAILED at line %d\n", __LINE__); \
        exit(1); \
    } \
} while(0)

#define ASSERT_EQ(a, b) do { \
    if ((a) != (b)) { \
        printf("FAILED: %ld != %ld at line %d\n", (long)(a), (long)(b), __LINE__); \
        exit(1); \
    } \
} while(0)

#define ASSERT_STR_EQ(a, b) do { \
    if (strcmp((a), (b)) != 0) { \
        printf("FAILED: \"%s\" != \"%s\" at line %d\n", (a), (b), __LINE__); \
        exit(1); \
    } \
} while(0)

// ============================================================
// Error Code Tests
// ============================================================

TEST(error_codes_basic) {
    ASSERT(KERNEL_SUCCESS == 0);
    ASSERT(KERNEL_EPERM == 1);
    ASSERT(KERNEL_ENOENT == 2);
    ASSERT(KERNEL_EINVAL == 22);
    ASSERT(KERNEL_ENOSYS == 38);
}

TEST(error_codes_jowo) {
    ASSERT(KERNEL_EHANDLE == 1000);
    ASSERT(KERNEL_EHANDLECLOSED == 1001);
    ASSERT(KERNEL_ERIGHTS == 1002);
    ASSERT(KERNEL_EOBJECTWAIT == 1003);
    ASSERT(KERNEL_EIPC == 1004);
    ASSERT(KERNEL_EMEMORY == 1005);
    ASSERT(KERNEL_EVMO == 1006);
}

TEST(strerror_basic) {
    ASSERT_STR_EQ(kernel_strerror(0), "Success");
    ASSERT_STR_EQ(kernel_strerror(KERNEL_ENOENT), "No such file or directory");
    ASSERT_STR_EQ(kernel_strerror(KERNEL_ENOMEM), "Out of memory");
    ASSERT_STR_EQ(kernel_strerror(KERNEL_ENOSYS), "Function not implemented");
    ASSERT_STR_EQ(kernel_strerror(KERNEL_EINVAL), "Invalid argument");
}

TEST(strerror_jowo) {
    ASSERT_STR_EQ(kernel_strerror(KERNEL_EHANDLE), "Invalid handle");
    ASSERT_STR_EQ(kernel_strerror(KERNEL_ERIGHTS), "Insufficient rights");
    ASSERT_STR_EQ(kernel_strerror(KERNEL_EOBJECTWAIT), "Object wait timeout");
}

TEST(strerror_unknown) {
    const char* msg = kernel_strerror(99999);
    ASSERT(msg != NULL);
    ASSERT(strstr(msg, "Unknown error") != NULL);
}

// ============================================================
// Type Tests
// ============================================================

TEST(types_handles) {
    ASSERT(INVALID_HANDLE == -1);
    ASSERT(INVALID_PID == -1);
    ASSERT(INVALID_FD == -1);
    ASSERT(INVALID_TID == -1);
}

TEST(types_rights) {
    ASSERT(RIGHT_DUPLICATE == (1 << 0));
    ASSERT(RIGHT_TRANSFER == (1 << 1));
    ASSERT(RIGHT_READ == (1 << 2));
    ASSERT(RIGHT_WRITE == (1 << 3));
    ASSERT(RIGHT_EXECUTE == (1 << 4));
    ASSERT(RIGHT_MAP == (1 << 5));
    ASSERT(RIGHT_SIGNAL == (1 << 6));
    ASSERT(RIGHT_WAIT == (1 << 7));
}

TEST(types_signals) {
    ASSERT(SIGNAL_READABLE == (1 << 0));
    ASSERT(SIGNAL_WRITABLE == (1 << 1));
    ASSERT(SIGNAL_PEER_CLOSED == (1 << 2));
    ASSERT(SIGNAL_SIGNALED == (1 << 3));
}

TEST(types_mmap_prot) {
    ASSERT(MMAP_PROT_READ == 1);
    ASSERT(MMAP_PROT_WRITE == 2);
    ASSERT(MMAP_PROT_EXEC == 4);
    ASSERT((MMAP_PROT_READ | MMAP_PROT_WRITE) == MMAP_PROT_READ_WRITE);
}

TEST(types_mmap_flags) {
    ASSERT(MMAP_FLAG_SHARED == (1 << 0));
    ASSERT(MMAP_FLAG_PRIVATE == (1 << 1));
    ASSERT(MMAP_FLAG_FIXED == (1 << 2));
    ASSERT(MMAP_FLAG_ANONYMOUS == (1 << 3));
}

TEST(types_open_flags) {
    ASSERT(OPEN_FLAG_READ == (1 << 0));
    ASSERT(OPEN_FLAG_WRITE == (1 << 1));
    ASSERT(OPEN_FLAG_APPEND == (1 << 2));
    ASSERT(OPEN_FLAG_TRUNC == (1 << 3));
    ASSERT(OPEN_FLAG_CREATE == (1 << 4));
}

TEST(types_syscalls) {
    ASSERT(SYSCALL_READ == 0);
    ASSERT(SYSCALL_WRITE == 1);
    ASSERT(SYSCALL_OPEN == 2);
    ASSERT(SYSCALL_CLOSE == 3);
    ASSERT(SYSCALL_MMAP == 9);
    ASSERT(SYSCALL_EXIT == 60);
    ASSERT(SYSCALL_GETPID == 39);
    ASSERT(SYSCALL_CLONE == 56);
}

TEST(types_jowo_syscalls) {
    ASSERT(SYSCALL_CHANNEL_CREATE == 400);
    ASSERT(SYSCALL_CHANNEL_WRITE == 401);
    ASSERT(SYSCALL_CHANNEL_READ == 402);
    ASSERT(SYSCALL_VMO_CREATE == 407);
    ASSERT(SYSCALL_HANDLE_CLOSE == 406);
}

// ============================================================
// Status Tests
// ============================================================

TEST(status_helpers) {
    status_t ok = 0;
    status_t err_neg = -1;
    status_t err_big = -KERNEL_ENOSYS;

    ASSERT(IS_ERROR(err_neg) == 1);
    ASSERT(IS_ERROR(err_big) == 1);
    ASSERT(IS_ERROR(ok) == 0);

    ASSERT(GET_ERROR(-1) == 1);
    ASSERT(GET_ERROR(-KERNEL_ENOENT) == KERNEL_ENOENT);
}

// ============================================================
// Version Tests
// ============================================================

TEST(version) {
    ASSERT(KERNEL_API_VERSION_MAJOR == 1);
    ASSERT(KERNEL_API_VERSION_MINOR == 0);
    ASSERT(KERNEL_API_VERSION_PATCH == 0);

    ASSERT(KERNEL_API_CHECK_VERSION(1, 0) == 1);
    ASSERT(KERNEL_API_CHECK_VERSION(0, 9) == 1);
    ASSERT(KERNEL_API_CHECK_VERSION(2, 0) == 0);
}

// ============================================================
// File Stats Tests
// ============================================================

TEST(fs_mode_types) {
    ASSERT(FS_MODE_TYPE_REG == 0x8000);
    ASSERT(FS_MODE_TYPE_DIR == 0x4000);
    ASSERT(FS_MODE_TYPE_CHR == 0x2000);
    ASSERT(FS_MODE_TYPE_BLK == 0x6000);
    ASSERT(FS_MODE_TYPE_FIFO == 0x1000);
    ASSERT(FS_MODE_TYPE_LNK == 0xA000);
    ASSERT(FS_MODE_TYPE_SOCK == 0xC000);
}

// ============================================================
// Main
// ============================================================

int main(int argc, char* argv[]) {
    (void)argc;
    (void)argv;

    printf("\n=== JowoKernel API Unit Tests ===\n\n");

    printf("Error Codes:\n");
    RUN_TEST(error_codes_basic);
    RUN_TEST(error_codes_jowo);
    RUN_TEST(strerror_basic);
    RUN_TEST(strerror_jowo);
    RUN_TEST(strerror_unknown);

    printf("\nTypes:\n");
    RUN_TEST(types_handles);
    RUN_TEST(types_rights);
    RUN_TEST(types_signals);
    RUN_TEST(types_mmap_prot);
    RUN_TEST(types_mmap_flags);
    RUN_TEST(types_open_flags);
    RUN_TEST(types_syscalls);
    RUN_TEST(types_jowo_syscalls);

    printf("\nStatus:\n");
    RUN_TEST(status_helpers);

    printf("\nVersion:\n");
    RUN_TEST(version);

    printf("\nFile Stats:\n");
    RUN_TEST(fs_mode_types);

    printf("\n=== Results ===\n");
    printf("Passed: %d/%d\n", tests_passed, tests_run);

    if (tests_passed == tests_run) {
        printf("ALL TESTS PASSED!\n");
        return 0;
    } else {
        printf("SOME TESTS FAILED!\n");
        return 1;
    }
}
