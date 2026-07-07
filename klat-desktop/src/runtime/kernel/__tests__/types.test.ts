/**
 * Unit tests for KLAT OS Runtime Kernel API Types
 */

import { describe, it, expect } from 'vitest';
import {
    isError,
    isOk,
    getError,
    errorStatus,
    strerror,
    Result,
    KERNEL_SUCCESS,
    KERNEL_ENOENT,
    KERNEL_ENOMEM,
    KERNEL_ENOSYS,
    KERNEL_EHANDLE,
    INVALID_HANDLE,
    INVALID_PID,
    INVALID_FD,
    MMAP_PROT_READ,
    MMAP_PROT_WRITE,
    MMAP_PROT_READ_WRITE,
    OPEN_FLAG_READ,
    OPEN_FLAG_WRITE,
    SIGNAL_READABLE,
    SIGNAL_WRITABLE,
    isFile,
    isDirectory,
    FileStats,
} from '../types';

describe('Kernel Types', () => {
    describe('Status Helpers', () => {
        it('should identify errors correctly', () => {
            expect(isError(-1)).toBe(true);
            expect(isError(-1000)).toBe(true);
            expect(isError(0)).toBe(false);
            expect(isError(1)).toBe(false);
            expect(isError(100)).toBe(false);
        });

        it('should identify success correctly', () => {
            expect(isOk(0)).toBe(true);
            expect(isOk(1)).toBe(true);
            expect(isOk(100)).toBe(true);
            expect(isOk(-1)).toBe(false);
            expect(isOk(-KERNEL_ENOSYS)).toBe(false);
        });

        it('should get error code from status', () => {
            expect(getError(-1)).toBe(1);
            expect(getError(-KERNEL_ENOENT)).toBe(KERNEL_ENOENT);
            expect(getError(-KERNEL_ENOMEM)).toBe(KERNEL_ENOMEM);
            expect(getError(0)).toBe(0);
        });

        it('should create error status', () => {
            expect(errorStatus(KERNEL_ENOENT)).toBe(-KERNEL_ENOENT);
            expect(errorStatus(KERNEL_ENOSYS)).toBe(-KERNEL_ENOSYS);
            expect(errorStatus(0)).toBe(0);
        });
    });

    describe('Error Messages', () => {
        it('should return correct error messages', () => {
            expect(strerror(KERNEL_SUCCESS)).toBe('Success');
            expect(strerror(KERNEL_ENOENT)).toBe('No such file or directory');
            expect(strerror(KERNEL_ENOMEM)).toBe('Out of memory');
            expect(strerror(KERNEL_ENOSYS)).toBe('Function not implemented');
        });

        it('should return unknown error for invalid codes', () => {
            expect(strerror(9999)).toContain('Unknown error');
        });
    });

    describe('Invalid Handles', () => {
        it('should have correct invalid sentinel values', () => {
            expect(INVALID_HANDLE).toBe(-1);
            expect(INVALID_PID).toBe(-1);
            expect(INVALID_FD).toBe(-1);
        });

        it('should recognize invalid handles', () => {
            expect(isError(INVALID_HANDLE)).toBe(true);
            expect(isError(INVALID_PID)).toBe(true);
        });
    });

    describe('Memory Protection Flags', () => {
        it('should have correct flag values', () => {
            expect(MMAP_PROT_READ).toBe(1);
            expect(MMAP_PROT_WRITE).toBe(2);
            expect(MMAP_PROT_READ_WRITE).toBe(3);
        });

        it('should combine flags correctly', () => {
            expect(MMAP_PROT_READ_WRITE & MMAP_PROT_READ).toBe(MMAP_PROT_READ);
            expect(MMAP_PROT_READ_WRITE & MMAP_PROT_WRITE).toBe(MMAP_PROT_WRITE);
        });
    });

    describe('Open Flags', () => {
        it('should have correct flag values', () => {
            expect(OPEN_FLAG_READ).toBe(1);
            expect(OPEN_FLAG_WRITE).toBe(2);
        });
    });

    describe('Signals', () => {
        it('should have correct signal values', () => {
            expect(SIGNAL_READABLE).toBe(1);
            expect(SIGNAL_WRITABLE).toBe(2);
        });
    });

    describe('File Stats', () => {
        it('should detect file type', () => {
            const fileStats: FileStats = {
                dev: 1n,
                ino: 1n,
                mode: 0x8000 | 0o644, // Regular file
                nlink: 1,
                uid: 0,
                gid: 0,
                rdev: 0n,
                size: 1024n,
                blksize: 4096n,
                blocks: 2n,
                atime: 0n,
                mtime: 0n,
                ctime: 0n,
            };

            expect(isFile(fileStats)).toBe(true);
            expect(isDirectory(fileStats)).toBe(false);
        });

        it('should detect directory type', () => {
            const dirStats: FileStats = {
                dev: 1n,
                ino: 2n,
                mode: 0x4000 | 0o755, // Directory
                nlink: 4,
                uid: 0,
                gid: 0,
                rdev: 0n,
                size: 4096n,
                blksize: 4096n,
                blocks: 8n,
                atime: 0n,
                mtime: 0n,
                ctime: 0n,
            };

            expect(isDirectory(dirStats)).toBe(true);
            expect(isFile(dirStats)).toBe(false);
        });
    });
});

describe('Result Type', () => {
    it('should create ok result with value', () => {
        const result = Result.ok(42);
        expect(result.ok).toBe(true);
        expect(result.value).toBe(42);
        expect(result.error).toBeUndefined();
    });

    it('should create ok result without value', () => {
        const result: Result<void> = Result.ok();
        expect(result.ok).toBe(true);
        expect(result.value).toBeUndefined();
    });

    it('should create error result', () => {
        const result = Result.err(KERNEL_ENOSYS);
        expect(result.ok).toBe(false);
        expect(result.error).toBe(KERNEL_ENOSYS);
        expect(result.value).toBeUndefined();
    });

    it('should create result from status - success', () => {
        const result = Result.fromStatus(0, 100);
        expect(result.ok).toBe(true);
        expect(result.value).toBe(100);
    });

    it('should create result from status - error', () => {
        const result = Result.fromStatus(-KERNEL_ENOENT);
        expect(result.ok).toBe(false);
        expect(result.error).toBe(KERNEL_ENOENT);
    });

    it('should convert to string - ok', () => {
        const result = Result.ok({ id: 1 });
        expect(result.toString()).toContain('Ok');
    });

    it('should convert to string - error', () => {
        const result = Result.err(KERNEL_EHANDLE);
        expect(result.toString()).toContain('Err');
        expect(result.toString()).toContain('Invalid handle');
    });
});
