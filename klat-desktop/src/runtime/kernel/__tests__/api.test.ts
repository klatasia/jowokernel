/**
 * Unit tests for KLAT OS Runtime Kernel API
 */

import { describe, it, expect, beforeEach } from 'vitest';
import {
    KernelAPI,
    ProcessAPI,
    ThreadAPI,
    FileAPI,
    MemoryAPI,
    IPCAPI,
    HandleAPI,
    FileDescriptorHandle,
    ChannelHandle,
} from '../index';
import { setEnvironment, getEnvironment, type RuntimeEnvironment } from '../syscall';

describe('KernelAPI', () => {
    describe('Environment', () => {
        it('should start in browser environment', () => {
            // Default is browser
            expect(getEnvironment()).toBe('browser');
        });

        it('should switch to native environment', () => {
            setEnvironment('native');
            expect(getEnvironment()).toBe('native');
        });

        it('should switch to test environment', () => {
            setEnvironment('test');
            expect(getEnvironment()).toBe('test');
        });

        afterEach(() => {
            // Reset to browser for other tests
            setEnvironment('browser');
        });
    });

    describe('ProcessAPI', () => {
        it('should create process in browser environment', () => {
            setEnvironment('browser');
            const result = ProcessAPI.create('test-process');
            expect(result.ok).toBe(true);
            expect(result.value).toBeDefined();
        });

        it('should get process ID', () => {
            setEnvironment('browser');
            const pid = ProcessAPI.getId();
            expect(pid).toBe(1);
        });

        it('should create thread', () => {
            setEnvironment('browser');
            const result = ThreadAPI.create('test-thread');
            expect(result.ok).toBe(true);
        });

        it('should get thread ID', () => {
            setEnvironment('browser');
            const tid = ThreadAPI.getId();
            expect(tid).toBe(1);
        });
    });

    describe('FileAPI', () => {
        it('should return error for open in browser (non-local path)', () => {
            setEnvironment('browser');
            const result = FileAPI.open('/etc/config');
            expect(result.ok).toBe(false);
        });

        it('should check if path is file', () => {
            setEnvironment('browser');
            // Default is false for unknown paths
            expect(FileAPI.isFile('/unknown/file.txt')).toBe(false);
        });

        it('should check if path is directory', () => {
            setEnvironment('browser');
            // Path ending with / is treated as directory
            expect(FileAPI.isDirectory('/some/path/')).toBe(true);
            expect(FileAPI.isDirectory('/some/path')).toBe(false);
        });

        it('should close invalid fd', () => {
            setEnvironment('browser');
            const result = FileAPI.close(-1 as any);
            // Should return ok even for invalid fd in browser mode
            expect(result.ok).toBe(true);
        });
    });

    describe('MemoryAPI', () => {
        it('should allocate memory', () => {
            setEnvironment('browser');
            const result = MemoryAPI.alloc(1024);
            expect(result.ok).toBe(true);
            expect(result.value).toBeDefined();
            expect((result.value as ArrayBuffer).byteLength).toBe(1024);
        });

        it('should create VMO', () => {
            setEnvironment('browser');
            const result = MemoryAPI.createVmo(4096n);
            expect(result.ok).toBe(true);
        });

        it('should map VMO', () => {
            setEnvironment('browser');
            const vmoResult = MemoryAPI.createVmo(4096n);
            if (vmoResult.ok) {
                const mapResult = MemoryAPI.map(vmoResult.value, 4096n, 1, 0);
                expect(mapResult.ok).toBe(true);
            }
        });
    });

    describe('IPCAPI', () => {
        it('should create channel', () => {
            setEnvironment('browser');
            const result = IPCAPI.createChannel();
            expect(result.ok).toBe(true);
            expect(result.value).toBeInstanceOf(ChannelHandle);
        });

        it('should wait on handle', () => {
            setEnvironment('browser');
            const result = IPCAPI.waitOne(100, 1, -1n);
            expect(result.ok).toBe(true);
        });
    });

    describe('HandleAPI', () => {
        it('should close handle', () => {
            setEnvironment('browser');
            const result = HandleAPI.close(100);
            expect(result.ok).toBe(true);
        });

        it('should validate handle', () => {
            setEnvironment('browser');
            expect(HandleAPI.isValid(100)).toBe(true);
            expect(HandleAPI.isValid(-1)).toBe(false);
        });
    });
});

describe('FileDescriptorHandle', () => {
    it('should create with valid fd', () => {
        const handle = new FileDescriptorHandle(10);
        expect(handle.isValid()).toBe(true);
        expect(handle.fd).toBe(10);
    });

    it('should create with auto-close enabled', () => {
        const handle = new FileDescriptorHandle(10, true);
        expect(handle.autoClose).toBe(true);
    });

    it('should create with auto-close disabled', () => {
        const handle = new FileDescriptorHandle(10, false);
        expect(handle.autoClose).toBe(false);
    });
});

describe('ChannelHandle', () => {
    it('should create channel with read and write handles', () => {
        const channel = new ChannelHandle(100, 101);
        expect(channel.readHandle).toBe(100);
        expect(channel.writeHandle).toBe(101);
    });
});

describe('KernelAPI Facade', () => {
    it('should export all sub-APIs', () => {
        expect(KernelAPI.process).toBe(ProcessAPI);
        expect(KernelAPI.thread).toBe(ThreadAPI);
        expect(KernelAPI.file).toBe(FileAPI);
        expect(KernelAPI.memory).toBe(MemoryAPI);
        expect(KernelAPI.ipc).toBe(IPCAPI);
        expect(KernelAPI.handle).toBe(HandleAPI);
    });

    it('should be usable via facade', () => {
        setEnvironment('browser');
        const pid = KernelAPI.process.getId();
        expect(pid).toBe(1);
    });
});
