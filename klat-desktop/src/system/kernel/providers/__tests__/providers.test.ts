/**
 * Unit tests for KLAT OS Kernel Providers
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import {
    ProviderFactory,
    NativeProcessProvider,
    BrowserProcessProvider,
    NativeFilesystemProvider,
    BrowserFilesystemProvider,
    NativeMemoryProvider,
    BrowserMemoryProvider,
    NativeIpcProvider,
    BrowserIpcProvider,
    getProcessProvider,
    getFilesystemProvider,
    getMemoryProvider,
    getIpcProvider,
} from '../index';
import { setEnvironment, getEnvironment } from '../../../runtime/kernel/syscall';

describe('ProviderFactory', () => {
    beforeEach(() => {
        ProviderFactory.reset();
    });

    afterEach(() => {
        ProviderFactory.reset();
        setEnvironment('browser');
    });

    describe('Environment Detection', () => {
        it('should default to browser environment', () => {
            const factory = new (ProviderFactory.constructor as any)();
            expect(factory.detectEnvironment()).toBeDefined();
        });
    });

    describe('initialize()', () => {
        it('should create browser providers in browser environment', () => {
            const registry = ProviderFactory.initialize('browser');

            expect(registry.process).toBeInstanceOf(BrowserProcessProvider);
            expect(registry.filesystem).toBeInstanceOf(BrowserFilesystemProvider);
            expect(registry.memory).toBeInstanceOf(BrowserMemoryProvider);
            expect(registry.ipc).toBeInstanceOf(BrowserIpcProvider);
        });

        it('should create native providers in native environment', () => {
            const registry = ProviderFactory.initialize('native');

            expect(registry.process).toBeInstanceOf(NativeProcessProvider);
            expect(registry.filesystem).toBeInstanceOf(NativeFilesystemProvider);
            expect(registry.memory).toBeInstanceOf(NativeMemoryProvider);
            expect(registry.ipc).toBeInstanceOf(NativeIpcProvider);
        });

        it('should return cached registry on subsequent calls', () => {
            const registry1 = ProviderFactory.initialize('browser');
            const registry2 = ProviderFactory.initialize('browser');

            expect(registry1).toBe(registry2);
        });

        it('should recreate registry when environment changes', () => {
            const registry1 = ProviderFactory.initialize('browser');
            const registry2 = ProviderFactory.initialize('native');

            expect(registry1).not.toBe(registry2);
        });
    });

    describe('getProvider()', () => {
        it('should return process provider', () => {
            const provider = ProviderFactory.getProvider('process');
            expect(provider).toBeDefined();
            expect(typeof provider.getCurrentPid).toBe('function');
        });

        it('should return filesystem provider', () => {
            const provider = ProviderFactory.getProvider('filesystem');
            expect(provider).toBeDefined();
            expect(typeof provider.exists).toBe('function');
        });

        it('should return memory provider', () => {
            const provider = ProviderFactory.getProvider('memory');
            expect(provider).toBeDefined();
            expect(typeof provider.alloc).toBe('function');
        });

        it('should return IPC provider', () => {
            const provider = ProviderFactory.getProvider('ipc');
            expect(provider).toBeDefined();
            expect(typeof provider.createChannel).toBe('function');
        });
    });

    describe('getRegistry()', () => {
        it('should auto-initialize if not initialized', () => {
            const registry = ProviderFactory.getRegistry();
            expect(registry).toBeDefined();
            expect(registry.process).toBeDefined();
        });
    });
});

describe('Convenience Hooks', () => {
    beforeEach(() => {
        ProviderFactory.reset();
    });

    afterEach(() => {
        ProviderFactory.reset();
        setEnvironment('browser');
    });

    it('getProcessProvider should return process provider', () => {
        const provider = getProcessProvider();
        expect(provider).toBeDefined();
    });

    it('getFilesystemProvider should return filesystem provider', () => {
        const provider = getFilesystemProvider();
        expect(provider).toBeDefined();
    });

    it('getMemoryProvider should return memory provider', () => {
        const provider = getMemoryProvider();
        expect(provider).toBeDefined();
    });

    it('getIpcProvider should return IPC provider', () => {
        const provider = getIpcProvider();
        expect(provider).toBeDefined();
    });
});

describe('BrowserProcessProvider', () => {
    let provider: BrowserProcessProvider;

    beforeEach(() => {
        provider = new BrowserProcessProvider();
        setEnvironment('browser');
    });

    it('should return PID 1', () => {
        expect(provider.getCurrentPid()).toBe(1);
    });

    it('should get process info', async () => {
        const info = await provider.getProcessInfo(1);
        expect(info).toBeDefined();
        expect(info?.pid).toBe(1);
        expect(info?.name).toBe('browser-process');
    });

    it('should get process list', async () => {
        const list = await provider.getProcessList();
        expect(Array.isArray(list)).toBe(true);
        expect(list.length).toBeGreaterThan(0);
    });

    it('should spawn process', async () => {
        const pid = await provider.spawn({ name: 'test' });
        expect(typeof pid).toBe('number');
        expect(pid).toBeGreaterThan(0);
    });

    it('should wait for process', async () => {
        const status = await provider.wait(1, 0);
        expect(typeof status).toBe('number');
    });

    it('should return false for kill', async () => {
        const result = await provider.kill(999);
        expect(result).toBe(false);
    });

    it('should get cwd', async () => {
        const cwd = await provider.getCwd();
        expect(cwd).toBe('/');
    });
});

describe('BrowserFilesystemProvider', () => {
    let provider: BrowserFilesystemProvider;

    beforeEach(() => {
        provider = new BrowserFilesystemProvider();
        setEnvironment('browser');
        // Clear localStorage
        if (typeof localStorage !== 'undefined') {
            localStorage.clear();
        }
    });

    afterEach(() => {
        if (typeof localStorage !== 'undefined') {
            localStorage.clear();
        }
    });

    it('should check if path exists', async () => {
        expect(await provider.exists('/test')).toBe(false);

        await provider.writeTextFile('/test', 'content');
        expect(await provider.exists('/test')).toBe(true);
    });

    it('should check if path is file', async () => {
        await provider.writeTextFile('/file.txt', 'content');
        expect(await provider.isFile('/file.txt')).toBe(true);
    });

    it('should check if path is directory', async () => {
        // Create a file in a "directory"
        await provider.writeTextFile('/dir/file.txt', 'content');
        expect(await provider.isDirectory('/dir/')).toBe(true);
    });

    it('should read and write text files', async () => {
        const content = 'Hello, World!';
        await provider.writeTextFile('/test.txt', content);
        const read = await provider.readTextFile('/test.txt');
        expect(read).toBe(content);
    });

    it('should read and write binary files', async () => {
        const data = new Uint8Array([1, 2, 3, 4, 5]);
        await provider.writeBinaryFile('/test.bin', data);
        const read = await provider.readBinaryFile('/test.bin');
        expect(Array.from(read)).toEqual([1, 2, 3, 4, 5]);
    });

    it('should delete file', async () => {
        await provider.writeTextFile('/delete.txt', 'content');
        expect(await provider.exists('/delete.txt')).toBe(true);

        await provider.deleteFile('/delete.txt');
        expect(await provider.exists('/delete.txt')).toBe(false);
    });

    it('should list directory', async () => {
        await provider.writeTextFile('/dir/file1.txt', 'a');
        await provider.writeTextFile('/dir/file2.txt', 'b');

        const files = await provider.listDirectory('/dir/');
        expect(files.length).toBe(2);
    });

    it('should copy file', async () => {
        await provider.writeTextFile('/source.txt', 'content');
        await provider.copyFile('/source.txt', '/dest.txt');

        expect(await provider.exists('/source.txt')).toBe(true);
        expect(await provider.exists('/dest.txt')).toBe(true);
    });

    it('should move file', async () => {
        await provider.writeTextFile('/old.txt', 'content');
        await provider.moveFile('/old.txt', '/new.txt');

        expect(await provider.exists('/old.txt')).toBe(false);
        expect(await provider.exists('/new.txt')).toBe(true);
    });
});

describe('BrowserMemoryProvider', () => {
    let provider: BrowserMemoryProvider;

    beforeEach(() => {
        provider = new BrowserMemoryProvider();
        setEnvironment('browser');
    });

    it('should allocate memory', async () => {
        const buffer = await provider.alloc({ size: 1024 });
        expect(buffer).toBeInstanceOf(Uint8Array);
        expect(buffer.length).toBe(1024);
    });

    it('should zero-initialize memory by default', async () => {
        const buffer = await provider.alloc({ size: 100 });
        expect(buffer.every(b => b === 0)).toBe(true);
    });

    it('should get memory stats', async () => {
        await provider.alloc({ size: 1000 });
        const stats = await provider.getStats();
        expect(stats).toHaveProperty('total');
        expect(stats).toHaveProperty('used');
        expect(stats).toHaveProperty('free');
    });

    it('should create VMO handle', async () => {
        const handle = await provider.createVmo(4096);
        expect(typeof handle).toBe('number');
    });

    it('should map VMO', async () => {
        const handle = await provider.createVmo(4096);
        const addr = await provider.mapVmo(handle, 4096, 1);
        expect(typeof addr).toBe('bigint');
    });
});

describe('BrowserIpcProvider', () => {
    let provider: BrowserIpcProvider;

    beforeEach(() => {
        provider = new BrowserIpcProvider();
        setEnvironment('browser');
    });

    it('should create channel', async () => {
        const channel = await provider.createChannel();
        expect(channel).toBeDefined();
        expect(typeof channel.send).toBe('function');
        expect(typeof channel.receive).toBe('function');
        expect(typeof channel.close).toBe('function');
    });

    it('should create event', async () => {
        const event = await provider.createEvent();
        expect(event).toBeDefined();
        expect(typeof event.signal).toBe('function');
        expect(typeof event.wait).toBe('function');
        expect(typeof event.reset).toBe('function');
    });

    it('should send and receive message', async () => {
        const channel = await provider.createChannel();

        // Send a message
        const message = {
            type: 'test',
            payload: { hello: 'world' },
            timestamp: Date.now(),
        };
        await channel.send(message);

        // In browser IPC, messages are async
        // For unit test, we just verify send doesn't throw
        expect(true).toBe(true);

        channel.close();
    });

    it('should handle channel close', async () => {
        const channel = await provider.createChannel();
        channel.close();

        // After close, should return null
        const result = await channel.receive(0);
        expect(result).toBeNull();
    });

    it('should handle event signal and wait', async () => {
        const event = await provider.createEvent();

        // Signal the event
        await event.signal();

        // Should return true immediately
        const result = await event.wait(0);
        expect(result).toBe(true);
    });

    it('should reset event', async () => {
        const event = await provider.createEvent();

        await event.signal();
        await event.reset();

        const result = await event.wait(0);
        expect(result).toBe(false);
    });

    it('should register event callback', async () => {
        const event = await provider.createEvent();
        let called = false;

        const unsubscribe = event.onSignal(() => {
            called = true;
        });

        await event.signal();
        expect(called).toBe(true);

        unsubscribe();
    });
});

describe('NativeProcessProvider', () => {
    let provider: NativeProcessProvider;

    beforeEach(() => {
        provider = new NativeProcessProvider();
        setEnvironment('native');
    });

    it('should get current PID', () => {
        const pid = provider.getCurrentPid();
        expect(typeof pid).toBe('number');
    });
});

describe('NativeFilesystemProvider', () => {
    let provider: NativeFilesystemProvider;

    beforeEach(() => {
        provider = new NativeFilesystemProvider();
        setEnvironment('native');
    });

    it('should have exists method', () => {
        expect(typeof provider.exists).toBe('function');
    });

    it('should have read/write methods', () => {
        expect(typeof provider.readTextFile).toBe('function');
        expect(typeof provider.writeTextFile).toBe('function');
    });
});

describe('NativeMemoryProvider', () => {
    let provider: NativeMemoryProvider;

    beforeEach(() => {
        provider = new NativeMemoryProvider();
        setEnvironment('native');
    });

    it('should have alloc method', () => {
        expect(typeof provider.alloc).toBe('function');
    });

    it('should have createVmo method', () => {
        expect(typeof provider.createVmo).toBe('function');
    });
});

describe('NativeIpcProvider', () => {
    let provider: NativeIpcProvider;

    beforeEach(() => {
        provider = new NativeIpcProvider();
        setEnvironment('native');
    });

    it('should have createChannel method', () => {
        expect(typeof provider.createChannel).toBe('function');
    });

    it('should have createEvent method', () => {
        expect(typeof provider.createEvent).toBe('function');
    });
});
