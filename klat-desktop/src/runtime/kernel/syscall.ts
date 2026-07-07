/**
 * KLAT OS Runtime - Kernel Syscall Bindings
 *
 * TypeScript bindings for native syscalls.
 * These functions call into the native kernel-api layer.
 *
 * IMPORTANT: These are stub implementations for browser/development.
 * In production (native), these will be replaced with actual syscall bindings.
 */

// ============================================================
// Native Import Placeholder
// ============================================================

/**
 * @internal
 * In native mode, this would import from native code.
 * For browser/development, we use stub implementations.
 */
declare const __kernel: {
    process_create: (name: string) => number;
    process_exit: (code: number) => never;
    process_get_id: () => number;
    process_wait: (pid: number, timeout: bigint) => number;

    thread_create: (name: string) => number;
    thread_exit: (code: number) => never;
    thread_get_id: () => number;

    fs_open: (path: string, flags: number, mode: number) => number;
    fs_close: (fd: number) => number;
    fs_read: (fd: number, buf: ArrayBuffer, size: number) => number;
    fs_write: (fd: number, buf: ArrayBuffer, size: number) => number;
    fs_lseek: (fd: number, offset: bigint, whence: number) => bigint;

    vmo_create: (size: bigint, options: number) => number;
    vmo_map: (vmo: number, size: bigint, prot: number, flags: number) => bigint;

    handle_close: (handle: number) => number;
    channel_create: () => [number, number];
    channel_write: (handle: number, data: ArrayBuffer) => number;
    channel_read: (handle: number, size: number) => ArrayBuffer;
};

// ============================================================
// Environment Detection
// ============================================================

export type RuntimeEnvironment = 'browser' | 'native' | 'test';

let _environment: RuntimeEnvironment = 'browser';

/**
 * Get current runtime environment
 */
export function getEnvironment(): RuntimeEnvironment {
    return _environment;
}

/**
 * Set runtime environment (for testing)
 */
export function setEnvironment(env: RuntimeEnvironment): void {
    _environment = env;
}

// ============================================================
// Native Syscall Bridge
// ============================================================

/**
 * @internal
 * Bridge to native syscalls.
 * In browser mode, this returns a stub that throws.
 */
class NativeBridge {
    private nativeAvailable = false;

    constructor() {
        // Check if native bindings are available
        if (typeof __kernel !== 'undefined') {
            this.nativeAvailable = true;
        }
    }

    get isNative(): boolean {
        return this.nativeAvailable;
    }

    // Process syscalls
    processCreate(name: string): number {
        if (this.nativeAvailable) {
            return __kernel.process_create(name);
        }
        // Stub for non-native
        throw new Error('Native syscall not available');
    }

    processExit(code: number): never {
        if (this.nativeAvailable) {
            return __kernel.process_exit(code);
        }
        throw new Error('Native syscall not available');
    }

    processGetId(): number {
        if (this.nativeAvailable) {
            return __kernel.process_get_id();
        }
        return 1; // PID 1 for init
    }

    processWait(pid: number, timeout: bigint): number {
        if (this.nativeAvailable) {
            return __kernel.process_wait(pid, timeout);
        }
        return -38; // ENOSYS
    }

    // Thread syscalls
    threadCreate(name: string): number {
        if (this.nativeAvailable) {
            return __kernel.thread_create(name);
        }
        return -38;
    }

    threadExit(code: number): never {
        if (this.nativeAvailable) {
            return __kernel.thread_exit(code);
        }
        throw new Error('Native syscall not available');
    }

    threadGetId(): number {
        if (this.nativeAvailable) {
            return __kernel.thread_get_id();
        }
        return 1;
    }

    // Filesystem syscalls
    fsOpen(path: string, flags: number, mode: number): number {
        if (this.nativeAvailable) {
            return __kernel.fs_open(path, flags, mode);
        }
        return -38;
    }

    fsClose(fd: number): number {
        if (this.nativeAvailable) {
            return __kernel.fs_close(fd);
        }
        return -38;
    }

    fsRead(fd: number, buf: ArrayBuffer, size: number): number {
        if (this.nativeAvailable) {
            return __kernel.fs_read(fd, buf, size);
        }
        return -38;
    }

    fsWrite(fd: number, buf: ArrayBuffer, size: number): number {
        if (this.nativeAvailable) {
            return __kernel.fs_write(fd, buf, size);
        }
        return -38;
    }

    fsLseek(fd: number, offset: bigint, whence: number): bigint {
        if (this.nativeAvailable) {
            return __kernel.fs_lseek(fd, offset, whence);
        }
        return -38n;
    }

    // VMO syscalls
    vmoCreate(size: bigint, options: number): number {
        if (this.nativeAvailable) {
            return __kernel.vmo_create(size, options);
        }
        return -38;
    }

    vmoMap(vmo: number, size: bigint, prot: number, flags: number): bigint {
        if (this.nativeAvailable) {
            return __kernel.vmo_map(vmo, size, prot, flags);
        }
        return BigInt(-38);
    }

    // Handle syscalls
    handleClose(handle: number): number {
        if (this.nativeAvailable) {
            return __kernel.handle_close(handle);
        }
        return -38;
    }

    // IPC syscalls
    channelCreate(): [number, number] {
        if (this.nativeAvailable) {
            return __kernel.channel_create();
        }
        return [-38, -38];
    }

    channelWrite(handle: number, data: ArrayBuffer): number {
        if (this.nativeAvailable) {
            return __kernel.channel_write(handle, data);
        }
        return -38;
    }

    channelRead(handle: number, size: number): ArrayBuffer {
        if (this.nativeAvailable) {
            return __kernel.channel_read(handle, size);
        }
        throw new Error('Native syscall not available');
    }
}

export const nativeBridge = new NativeBridge();

// ============================================================
// Syscall Status Check
// ============================================================

/**
 * Check if syscall returned an error
 */
export function syscallFailed(status: number): boolean {
    return status < 0;
}

/**
 * Convert syscall return to error code
 */
export function syscallError(status: number): number {
    return -status;
}

/**
 * Throw if syscall failed
 */
export function syscallThrow(status: number): void {
    if (syscallFailed(status)) {
        throw new Error(`Syscall failed: ${status}`);
    }
}
