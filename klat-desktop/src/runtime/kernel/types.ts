/**
 * KLAT OS Runtime - Kernel API Bindings
 *
 * TypeScript bindings for the kernel-api C layer.
 * This provides type-safe access to JowoKernel syscalls.
 *
 * Architecture:
 * ```
 * Runtime (TypeScript)
 *    ↓
 * KernelAPIBinding (This layer)
 *    ↓
 * kernel-api (C headers)
 *    ↓
 * syscall_entry.S (Assembly)
 *    ↓
 * JowoKernel
 * ```
 */

// ============================================================
// Base Types (mirrors kernel-api/contracts/types.h)
// ============================================================

/** Handle type - represents a kernel object reference */
export type Handle = number;
export type ProcessId = number;
export type ThreadId = number;
export type FileDescriptor = number;
export type Status = number;

/** Invalid handle sentinel */
export const INVALID_HANDLE: Handle = -1 as Handle;
export const INVALID_PID: ProcessId = -1 as ProcessId;
export const INVALID_TID: ThreadId = -1 as ThreadId;
export const INVALID_FD: FileDescriptor = -1 as FileDescriptor;

/** Rights bitmasks */
export const RIGHT_DUPLICATE = 1 << 0;
export const RIGHT_TRANSFER = 1 << 1;
export const RIGHT_READ = 1 << 2;
export const RIGHT_WRITE = 1 << 3;
export const RIGHT_EXECUTE = 1 << 4;
export const RIGHT_MAP = 1 << 5;
export const RIGHT_SIGNAL = 1 << 6;
export const RIGHT_WAIT = 1 << 7;
export const RIGHT_SET_PROPERTY = 1 << 8;
export const RIGHT_GET_PROPERTY = 1 << 9;
export const RIGHT_ALL = 0xFFFFFFFF as Rights;
export type Rights = number;

/** Object signals */
export const SIGNAL_NONE = 0;
export const SIGNAL_READABLE = 1 << 0;
export const SIGNAL_WRITABLE = 1 << 1;
export const SIGNAL_PEER_CLOSED = 1 << 2;
export const SIGNAL_SIGNALED = 1 << 3;
export const SIGNAL_HANGUP = 1 << 4;
export const SIGNAL_ERROR = 1 << 5;
export const SIGNAL_POLLIN = 1 << 7;
export const SIGNAL_POLLOUT = 1 << 8;
export type Signal = number;

/** Memory protection flags */
export const MMAP_PROT_NONE = 0;
export const MMAP_PROT_READ = 1 << 0;
export const MMAP_PROT_WRITE = 1 << 1;
export const MMAP_PROT_EXEC = 1 << 2;
export const MMAP_PROT_READ_WRITE = MMAP_PROT_READ | MMAP_PROT_WRITE;
export const MMAP_PROT_READ_EXEC = MMAP_PROT_READ | MMAP_PROT_EXEC;
export const MMAP_PROT_ALL = MMAP_PROT_READ | MMAP_PROT_WRITE | MMAP_PROT_EXEC;
export type MmapProt = number;

/** Memory mapping flags */
export const MMAP_FLAG_SHARED = 1 << 0;
export const MMAP_FLAG_PRIVATE = 1 << 1;
export const MMAP_FLAG_FIXED = 1 << 2;
export const MMAP_FLAG_ANONYMOUS = 1 << 3;
export type MmapFlags = number;

/** File open flags */
export const OPEN_FLAG_READ = 1 << 0;
export const OPEN_FLAG_WRITE = 1 << 1;
export const OPEN_FLAG_APPEND = 1 << 2;
export const OPEN_FLAG_TRUNC = 1 << 3;
export const OPEN_FLAG_CREATE = 1 << 4;
export const OPEN_FLAG_EXCL = 1 << 5;
export const OPEN_FLAG_DIRECTORY = 1 << 6;
export const OPEN_FLAG_SYNC = 1 << 7;
export const OPEN_FLAG_NONBLOCK = 1 << 9;
export type OpenFlags = number;

/** Wait options */
export const WAIT_OPTION_INTERRUPTIBLE = 1 << 0;
export const WAIT_OPTION_NONBLOCK = 1 << 1;
export const WAIT_INFINITE = -1 as unknown as bigint;
export const WAIT_NONBLOCK = 0n;
export type WaitOptions = number;

// ============================================================
// Error Types (mirrors kernel-api/errors/error_codes.h)
// ============================================================

export const KERNEL_SUCCESS = 0;
export const KERNEL_EPERM = 1;
export const KERNEL_ENOENT = 2;
export const KERNEL_ESRCH = 3;
export const KERNEL_EINTR = 4;
export const KERNEL_EIO = 5;
export const KERNEL_ENXIO = 6;
export const KERNEL_E2BIG = 7;
export const KERNEL_ENOEXEC = 8;
export const KERNEL_EBADF = 9;
export const KERNEL_ECHILD = 10;
export const KERNEL_EAGAIN = 11;
export const KERNEL_ENOMEM = 12;
export const KERNEL_EACCES = 13;
export const KERNEL_EFAULT = 14;
export const KERNEL_ENOTBLK = 15;
export const KERNEL_EBUSY = 16;
export const KERNEL_EEXIST = 17;
export const KERNEL_EXDEV = 18;
export const KERNEL_ENODEV = 19;
export const KERNEL_ENOTDIR = 20;
export const KERNEL_EISDIR = 21;
export const KERNEL_EINVAL = 22;
export const KERNEL_ENFILE = 23;
export const KERNEL_EMFILE = 24;
export const KERNEL_ENOTTY = 25;
export const KERNEL_ETXTBSY = 26;
export const KERNEL_EFBIG = 27;
export const KERNEL_ENOSPC = 28;
export const KERNEL_ESPIPE = 29;
export const KERNEL_EROFS = 30;
export const KERNEL_EMLINK = 31;
export const KERNEL_EPIPE = 32;
export const KERNEL_ENOSYS = 38;
export const KERNEL_EALREADY = 114;
export const KERNEL_EINPROGRESS = 115;

/** JowoKernel-specific errors */
export const KERNEL_EHANDLE = 1000;
export const KERNEL_EHANDLECLOSED = 1001;
export const KERNEL_ERIGHTS = 1002;
export const KERNEL_EOBJECTWAIT = 1003;
export const KERNEL_EIPC = 1004;
export const KERNEL_EMEMORY = 1005;
export const KERNEL_EVMO = 1006;
export const KERNEL_ETHREAD = 1007;
export const KERNEL_EPROCESS = 1008;
export const KERNEL_EVFS = 1009;
export const KERNEL_EDRIVER = 1010;

// ============================================================
// Status Helpers
// ============================================================

/** Check if status is an error (negative) */
export function isError(status: Status): boolean {
    return status < 0;
}

/** Check if status is success (non-negative) */
export function isOk(status: Status): boolean {
    return status >= 0;
}

/** Get error code from status */
export function getError(status: Status): number {
    return -status;
}

/** Create error status from error code */
export function errorStatus(errno: number): Status {
    return -errno as Status;
}

/** Get error message for an error code */
export function strerror(errno: number): string {
    const messages: Record<number, string> = {
        [KERNEL_SUCCESS]: 'Success',
        [KERNEL_EPERM]: 'Operation not permitted',
        [KERNEL_ENOENT]: 'No such file or directory',
        [KERNEL_ESRCH]: 'No such process',
        [KERNEL_EINTR]: 'Interrupted system call',
        [KERNEL_EIO]: 'I/O error',
        [KERNEL_ENXIO]: 'No such device or address',
        [KERNEL_ENOMEM]: 'Out of memory',
        [KERNEL_EACCES]: 'Permission denied',
        [KERNEL_EFAULT]: 'Bad address',
        [KERNEL_EEXIST]: 'File exists',
        [KERNEL_ENODEV]: 'No such device',
        [KERNEL_ENOTDIR]: 'Not a directory',
        [KERNEL_EISDIR]: 'Is a directory',
        [KERNEL_EINVAL]: 'Invalid argument',
        [KERNEL_EMFILE]: 'Too many open files',
        [KERNEL_ENOSPC]: 'No space left on device',
        [KERNEL_ENOSYS]: 'Function not implemented',
        [KERNEL_EALREADY]: 'Operation already in progress',
        [KERNEL_EINPROGRESS]: 'Operation now in progress',
        [KERNEL_EHANDLE]: 'Invalid handle',
        [KERNEL_EHANDLECLOSED]: 'Handle already closed',
        [KERNEL_ERIGHTS]: 'Insufficient rights',
        [KERNEL_EOBJECTWAIT]: 'Object wait timeout',
        [KERNEL_EIPC]: 'IPC error',
        [KERNEL_EMEMORY]: 'Memory allocation failed',
        [KERNEL_EVMO]: 'VMO operation failed',
        [KERNEL_EPROCESS]: 'Process error',
        [KERNEL_EVFS]: 'VFS operation failed',
    };
    return messages[errno] ?? `Unknown error (${errno})`;
}

// ============================================================
// Kernel API Result Type
// ============================================================

/**
 * Result type for kernel operations
 * Wraps Status with additional type safety
 */
export class Result<T = void> {
    private constructor(
        public readonly ok: boolean,
        public readonly value?: T,
        public readonly error?: number
    ) {}

    static ok<T>(value?: T): Result<T> {
        return new Result(true, value);
    }

    static err<T>(error: number): Result<T> {
        return new Result(false, undefined, error);
    }

    static fromStatus(status: Status, value?: T): Result<T> {
        if (status < 0) {
            return Result.err(-status);
        }
        return Result.ok(value ?? (status as unknown as T));
    }

    toString(): string {
        if (this.ok) {
            return `Ok(${JSON.stringify(this.value)})`;
        }
        return `Err(${strerror(this.error!)})`;
    }
}

// ============================================================
// Process Info
// ============================================================

export interface ProcessInfo {
    pid: ProcessId;
    parentPid: ProcessId;
    name: string;
    flags: number;
    startTime: bigint;
    stackBase: bigint;
    stackLimit: bigint;
    entryPoint: bigint;
}

// ============================================================
// File Stats
// ============================================================

export interface FileStats {
    dev: bigint;
    ino: bigint;
    mode: number;
    nlink: number;
    uid: number;
    gid: number;
    rdev: bigint;
    size: bigint;
    blksize: bigint;
    blocks: bigint;
    atime: bigint;
    mtime: bigint;
    ctime: bigint;
}

/** File type constants */
export const FS_MODE_TYPE_REG = 0x8000;
export const FS_MODE_TYPE_DIR = 0x4000;
export const FS_MODE_TYPE_CHR = 0x2000;
export const FS_MODE_TYPE_BLK = 0x6000;
export const FS_MODE_TYPE_FIFO = 0x1000;
export const FS_MODE_TYPE_LNK = 0xA000;
export const FS_MODE_TYPE_SOCK = 0xC000;

export function isFile(stats: FileStats): boolean {
    return (stats.mode & 0xF000) === FS_MODE_TYPE_REG;
}

export function isDirectory(stats: FileStats): boolean {
    return (stats.mode & 0xF000) === FS_MODE_TYPE_DIR;
}

// ============================================================
// Version Info
// ============================================================

export const KERNEL_API_VERSION_MAJOR = 1;
export const KERNEL_API_VERSION_MINOR = 0;
export const KERNEL_API_VERSION_PATCH = 0;

export function checkVersion(major: number, minor: number): boolean {
    return KERNEL_API_VERSION_MAJOR > major ||
        (KERNEL_API_VERSION_MAJOR === major && KERNEL_API_VERSION_MINOR >= minor);
}
