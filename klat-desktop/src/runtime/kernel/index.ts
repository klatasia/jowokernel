/**
 * KLAT OS Runtime - Kernel API
 *
 * Unified interface for all kernel operations.
 * This is the main entry point for Runtime to interact with JowoKernel.
 *
 * Architecture:
 * ```
 * Runtime
 *    ↓
 * KernelAPI (this class)
 *    ↓
 * nativeBridge / BrowserBridge
 *    ↓
 * kernel-api (C)
 *    ↓
 * JowoKernel
 * ```
 */

import {
    type Handle,
    type ProcessId,
    type ThreadId,
    type FileDescriptor,
    type Status,
    type Rights,
    type Signal,
    type MmapProt,
    type MmapFlags,
    type OpenFlags,
    type ProcessInfo,
    type FileStats,
    INVALID_HANDLE,
    INVALID_PID,
    INVALID_FD,
    isError,
    isOk,
    getError,
    Result,
    KERNEL_SUCCESS,
    KERNEL_ENOSYS,
    KERNEL_EINVAL,
} from './types';

import {
    nativeBridge,
    getEnvironment,
    type RuntimeEnvironment,
    syscallFailed,
} from './syscall';

// ============================================================
// Process API
// ============================================================

/**
 * Process management API
 */
export class ProcessAPI {
    /**
     * Create a new process
     */
    static create(name: string): Result<Handle> {
        if (getEnvironment() === 'native') {
            const handle = nativeBridge.processCreate(name);
            if (syscallFailed(handle)) {
                return Result.err(getError(handle));
            }
            return Result.ok(handle);
        }

        // Browser/test stub - return a mock handle
        return Result.ok(100 as Handle);
    }

    /**
     * Exit current process
     */
    static exit(code: number): never {
        if (getEnvironment() === 'native') {
            nativeBridge.processExit(code);
        }
        // Browser fallback
        throw new Error(`Process exit: ${code}`);
    }

    /**
     * Get current process ID
     */
    static getId(): ProcessId {
        if (getEnvironment() === 'native') {
            return nativeBridge.processGetId() as ProcessId;
        }
        return 1 as ProcessId; // Init process
    }

    /**
     * Wait for a process to exit
     */
    static wait(pid: ProcessId, timeout: bigint = -1n): Result<Status> {
        if (getEnvironment() === 'native') {
            const result = nativeBridge.processWait(pid, timeout);
            if (syscallFailed(result)) {
                return Result.err(getError(result));
            }
            return Result.ok(result);
        }

        // Browser stub
        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Kill a process
     */
    static kill(pid: ProcessId): Result<void> {
        // Native only
        if (getEnvironment() !== 'native') {
            return Result.err(KERNEL_ENOSYS);
        }
        return Result.ok();
    }

    /**
     * Get process information
     */
    static getInfo(pid: ProcessId): Result<ProcessInfo> {
        // Native only
        if (getEnvironment() !== 'native') {
            return Result.ok({
                pid,
                parentPid: 0 as ProcessId,
                name: 'browser-process',
                flags: 0,
                startTime: 0n,
                stackBase: 0n,
                stackLimit: 0n,
                entryPoint: 0n,
            });
        }
        return Result.err(KERNEL_ENOSYS);
    }
}

// ============================================================
// Thread API
// ============================================================

/**
 * Thread management API
 */
export class ThreadAPI {
    /**
     * Create a new thread
     */
    static create(name: string): Result<ThreadId> {
        if (getEnvironment() === 'native') {
            const tid = nativeBridge.threadCreate(name);
            if (syscallFailed(tid)) {
                return Result.err(getError(tid));
            }
            return Result.ok(tid as ThreadId);
        }
        return Result.ok(1 as ThreadId);
    }

    /**
     * Exit current thread
     */
    static exit(code: number): never {
        if (getEnvironment() === 'native') {
            nativeBridge.threadExit(code);
        }
        throw new Error(`Thread exit: ${code}`);
    }

    /**
     * Get current thread ID
     */
    static getId(): ThreadId {
        if (getEnvironment() === 'native') {
            return nativeBridge.threadGetId() as ThreadId;
        }
        return 1 as ThreadId;
    }

    /**
     * Yield execution to scheduler
     */
    static yield(): void {
        // Native: would call sys_yield
        // Browser: use setTimeout
        if (typeof window !== 'undefined') {
            return;
        }
    }
}

// ============================================================
// Filesystem API
// ============================================================

/**
 * File descriptor wrapper with automatic cleanup
 */
export class FileDescriptorHandle {
    constructor(
        public readonly fd: FileDescriptor,
        private autoClose = true
    ) {}

    isValid(): boolean {
        return this.fd >= 0;
    }

    close(): void {
        if (this.autoClose && this.isValid()) {
            FileAPI.close(this.fd);
        }
    }

    read(buffer: ArrayBuffer): Result<number> {
        if (getEnvironment() === 'native') {
            const bytes = nativeBridge.fsRead(this.fd, buffer, buffer.byteLength);
            if (syscallFailed(bytes)) {
                return Result.err(getError(bytes));
            }
            return Result.ok(bytes);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    write(buffer: ArrayBuffer): Result<number> {
        if (getEnvironment() === 'native') {
            const bytes = nativeBridge.fsWrite(this.fd, buffer, buffer.byteLength);
            if (syscallFailed(bytes)) {
                return Result.err(getError(bytes));
            }
            return Result.ok(bytes);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    lseek(offset: bigint, whence: number): Result<bigint> {
        if (getEnvironment() === 'native') {
            const result = nativeBridge.fsLseek(this.fd, offset, whence);
            if (syscallFailed(Number(result))) {
                return Result.err(getError(Number(result)));
            }
            return Result.ok(result);
        }
        return Result.err(KERNEL_ENOSYS);
    }
}

/**
 * Filesystem API
 */
export class FileAPI {
    /**
     * Open a file
     */
    static open(
        path: string,
        flags: OpenFlags = 0,
        mode: number = 0o644
    ): Result<FileDescriptorHandle> {
        if (getEnvironment() === 'native') {
            const fd = nativeBridge.fsOpen(path, flags, mode);
            if (syscallFailed(fd)) {
                return Result.err(getError(fd));
            }
            return Result.ok(new FileDescriptorHandle(fd as FileDescriptor));
        }

        // Browser stub - try localStorage
        if (typeof localStorage !== 'undefined' && path.startsWith('/local/')) {
            const key = path.slice(7);
            return Result.ok(new FileDescriptorHandle(100 as FileDescriptor, false));
        }

        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Close a file descriptor
     */
    static close(fd: FileDescriptor): Result<void> {
        if (getEnvironment() === 'native') {
            const result = nativeBridge.fsClose(fd);
            if (syscallFailed(result)) {
                return Result.err(getError(result));
            }
            return Result.ok();
        }
        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Read from file descriptor
     */
    static read(fd: FileDescriptor, buffer: ArrayBuffer): Result<number> {
        if (getEnvironment() === 'native') {
            const bytes = nativeBridge.fsRead(fd, buffer, buffer.byteLength);
            if (syscallFailed(bytes)) {
                return Result.err(getError(bytes));
            }
            return Result.ok(bytes);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Write to file descriptor
     */
    static write(fd: FileDescriptor, buffer: ArrayBuffer): Result<number> {
        if (getEnvironment() === 'native') {
            const bytes = nativeBridge.fsWrite(fd, buffer, buffer.byteLength);
            if (syscallFailed(bytes)) {
                return Result.err(getError(bytes));
            }
            return Result.ok(bytes);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Seek in file
     */
    static lseek(fd: FileDescriptor, offset: bigint, whence: number): Result<bigint> {
        if (getEnvironment() === 'native') {
            const result = nativeBridge.fsLseek(fd, offset, whence);
            if (syscallFailed(Number(result))) {
                return Result.err(getError(Number(result)));
            }
            return Result.ok(result);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Get file stats
     */
    static stat(path: string): Result<FileStats> {
        // Native only
        if (getEnvironment() !== 'native') {
            return Result.err(KERNEL_ENOSYS);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Create directory
     */
    static mkdir(path: string, mode: number = 0o755): Result<void> {
        // Native only
        if (getEnvironment() !== 'native') {
            return Result.err(KERNEL_ENOSYS);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Remove file
     */
    static unlink(path: string): Result<void> {
        // Native only
        if (getEnvironment() !== 'native') {
            return Result.err(KERNEL_ENOSYS);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Check if path is a file
     */
    static isFile(path: string): boolean {
        if (typeof localStorage !== 'undefined' && path.startsWith('/local/')) {
            return localStorage.getItem(path.slice(7)) !== null;
        }
        return false;
    }

    /**
     * Check if path is a directory
     */
    static isDirectory(path: string): boolean {
        // Browser stub
        return path.endsWith('/');
    }
}

// ============================================================
// Memory API
// ============================================================

/**
 * Memory (VMO) API
 */
export class MemoryAPI {
    /**
     * Create a VMO (Virtual Memory Object)
     */
    static createVmo(size: bigint, options: number = 0): Result<Handle> {
        if (getEnvironment() === 'native') {
            const handle = nativeBridge.vmoCreate(size, options);
            if (syscallFailed(handle)) {
                return Result.err(getError(handle));
            }
            return Result.ok(handle);
        }
        return Result.ok(200 as Handle); // Mock handle
    }

    /**
     * Map a VMO into address space
     */
    static map(
        vmo: Handle,
        size: bigint,
        prot: MmapProt,
        flags: MmapFlags
    ): Result<bigint> {
        if (getEnvironment() === 'native') {
            const addr = nativeBridge.vmoMap(vmo, size, prot, flags);
            if (syscallFailed(Number(addr))) {
                return Result.err(getError(Number(addr)));
            }
            return Result.ok(addr);
        }
        return Result.ok(0x700000000000n + BigInt(vmo * 0x1000)); // Mock address
    }

    /**
     * Allocate anonymous memory
     */
    static alloc(size: number): Result<ArrayBuffer> {
        // Browser fallback
        if (typeof window === 'undefined') {
            // Node.js
            const { Buffer } = require('buffer');
            return Result.ok(Buffer.alloc(size).buffer as ArrayBuffer);
        }
        return Result.ok(new ArrayBuffer(size));
    }
}

// ============================================================
// IPC API
// ============================================================

/**
 * Channel IPC wrapper
 */
export class ChannelHandle {
    constructor(
        public readonly readHandle: Handle,
        public readonly writeHandle: Handle
    ) {}

    static create(): Result<ChannelHandle> {
        if (getEnvironment() === 'native') {
            const [readHandle, writeHandle] = nativeBridge.channelCreate();
            if (syscallFailed(readHandle)) {
                return Result.err(getError(readHandle));
            }
            return Result.ok(new ChannelHandle(readHandle, writeHandle));
        }
        return Result.ok(new ChannelHandle(300 as Handle, 301 as Handle));
    }

    write(data: ArrayBuffer): Result<void> {
        if (getEnvironment() === 'native') {
            const result = nativeBridge.channelWrite(this.writeHandle, data);
            if (syscallFailed(result)) {
                return Result.err(getError(result));
            }
            return Result.ok();
        }
        return Result.ok();
    }

    read(size: number): Result<ArrayBuffer> {
        if (getEnvironment() === 'native') {
            const data = nativeBridge.channelRead(this.readHandle, size);
            return Result.ok(data);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    close(): void {
        HandleAPI.close(this.readHandle);
        HandleAPI.close(this.writeHandle);
    }
}

/**
 * IPC API
 */
export class IPCAPI {
    /**
     * Create a channel pair for IPC
     */
    static createChannel(): Result<ChannelHandle> {
        return ChannelHandle.create();
    }

    /**
     * Wait on an object
     */
    static waitOne(handle: Handle, signals: Signal, timeout: bigint): Result<Signal> {
        // Native only
        if (getEnvironment() !== 'native') {
            return Result.ok(SIGNAL_READABLE);
        }
        return Result.err(KERNEL_ENOSYS);
    }
}

// ============================================================
// Handle API
// ============================================================

/**
 * Handle API
 */
export class HandleAPI {
    /**
     * Close a handle
     */
    static close(handle: Handle): Result<void> {
        if (getEnvironment() === 'native') {
            const result = nativeBridge.handleClose(handle);
            if (syscallFailed(result)) {
                return Result.err(getError(result));
            }
            return Result.ok();
        }
        return Result.ok(); // Browser stub
    }

    /**
     * Duplicate a handle with new rights
     */
    static duplicate(handle: Handle, rights: Rights): Result<Handle> {
        if (getEnvironment() !== 'native') {
            return Result.ok(handle);
        }
        return Result.err(KERNEL_ENOSYS);
    }

    /**
     * Check if handle is valid
     */
    static isValid(handle: Handle): boolean {
        return handle >= 0;
    }
}

// ============================================================
// Kernel API Facade
// ============================================================

/**
 * KernelAPI - Main facade for all kernel operations
 *
 * Usage:
 * ```typescript
 * import { KernelAPI, ProcessAPI, FileAPI } from './kernel';
 *
 * // Process operations
 * const proc = ProcessAPI.create('my-app');
 * const pid = ProcessAPI.getId();
 *
 * // File operations
 * const file = FileAPI.open('/data/config.json');
 * const buffer = new ArrayBuffer(1024);
 * file.read(buffer);
 * file.close();
 *
 * // IPC
 * const channel = IPCAPI.createChannel();
 * channel.write(new ArrayBuffer(256));
 * ```
 */
export const KernelAPI = {
    process: ProcessAPI,
    thread: ThreadAPI,
    file: FileAPI,
    memory: MemoryAPI,
    ipc: IPCAPI,
    handle: HandleAPI,
};

export default KernelAPI;
