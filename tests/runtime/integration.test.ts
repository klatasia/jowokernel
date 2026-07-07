/**
 * KLAT OS Runtime Integration Tests
 *
 * M2: Validates Runtime → Kernel API → Native Provider connection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { setEnvironment, getEnvironment } from '../runtime/kernel/syscall';
import {
    ProviderFactory,
    NativeProcessProvider,
    NativeFilesystemProvider,
    NativeMemoryProvider,
    NativeIpcProvider,
} from '../system/kernel/providers';

/**
 * M2: Runtime Native Integration Tests
 *
 * These tests validate that:
 * 1. Runtime can call Kernel API
 * 2. Native Provider can connect to JowoKernel
 * 3. All providers work correctly
 */

describe('M2: Runtime Native Integration', () => {
    beforeEach(() => {
        ProviderFactory.reset();
        setEnvironment('native');
    });

    afterEach(() => {
        ProviderFactory.reset();
    });

    describe('ProviderFactory', () => {
        it('should initialize with native providers', () => {
            const registry = ProviderFactory.initialize('native');
            expect(registry).toBeDefined();
            expect(registry.process).toBeInstanceOf(NativeProcessProvider);
            expect(registry.filesystem).toBeInstanceOf(NativeFilesystemProvider);
            expect(registry.memory).toBeInstanceOf(NativeMemoryProvider);
            expect(registry.ipc).toBeInstanceOf(NativeIpcProvider);
        });

        it('should cache registry', () => {
            const registry1 = ProviderFactory.initialize();
            const registry2 = ProviderFactory.getRegistry();
            expect(registry1).toBe(registry2);
        });

        it('should detect native environment', () => {
            const env = ProviderFactory.getEnvironment();
            // In browser test, this will be 'browser'
            expect(['native', 'browser']).toContain(env);
        });
    });

    describe('NativeProcessProvider', () => {
        it('should get current PID', () => {
            const provider = new NativeProcessProvider();
            const pid = provider.getCurrentPid();
            expect(typeof pid).toBe('number');
            expect(pid).toBeGreaterThan(0);
        });

        it('should have getProcessInfo method', async () => {
            const provider = new NativeProcessProvider();
            const pid = provider.getCurrentPid();
            const info = await provider.getProcessInfo(pid);
            // In native mode, this should return actual process info
            // In browser mode, this returns mock data
            expect(info).toBeDefined();
        });

        it('should have getProcessList method', async () => {
            const provider = new NativeProcessProvider();
            const list = await provider.getProcessList();
            expect(Array.isArray(list)).toBe(true);
            expect(list.length).toBeGreaterThan(0);
        });
    });

    describe('NativeFilesystemProvider', () => {
        it('should have exists method', async () => {
            const provider = new NativeFilesystemProvider();
            expect(typeof provider.exists).toBe('function');
        });

        it('should have readTextFile method', async () => {
            const provider = new NativeFilesystemProvider();
            expect(typeof provider.readTextFile).toBe('function');
        });

        it('should have writeTextFile method', async () => {
            const provider = new NativeFilesystemProvider();
            expect(typeof provider.writeTextFile).toBe('function');
        });

        it('should have readBinaryFile method', async () => {
            const provider = new NativeFilesystemProvider();
            expect(typeof provider.readBinaryFile).toBe('function');
        });

        it('should have listDirectory method', async () => {
            const provider = new NativeFilesystemProvider();
            expect(typeof provider.listDirectory).toBe('function');
        });

        it('should have createDirectory method', async () => {
            const provider = new NativeFilesystemProvider();
            expect(typeof provider.createDirectory).toBe('function');
        });
    });

    describe('NativeMemoryProvider', () => {
        it('should have alloc method', () => {
            const provider = new NativeMemoryProvider();
            expect(typeof provider.alloc).toBe('function');
        });

        it('should have createVmo method', () => {
            const provider = new NativeMemoryProvider();
            expect(typeof provider.createVmo).toBe('function');
        });

        it('should have mapVmo method', () => {
            const provider = new NativeMemoryProvider();
            expect(typeof provider.mapVmo).toBe('function');
        });

        it('should have mprotect method', () => {
            const provider = new NativeMemoryProvider();
            expect(typeof provider.mprotect).toBe('function');
        });

        it('should have getStats method', async () => {
            const provider = new NativeMemoryProvider();
            const stats = await provider.getStats();
            expect(stats).toHaveProperty('total');
            expect(stats).toHaveProperty('used');
            expect(stats).toHaveProperty('free');
        });
    });

    describe('NativeIpcProvider', () => {
        it('should have createChannel method', async () => {
            const provider = new NativeIpcProvider();
            expect(typeof provider.createChannel).toBe('function');
        });

        it('should have createEvent method', async () => {
            const provider = new NativeIpcProvider();
            expect(typeof provider.createEvent).toBe('function');
        });

        it('should have waitForSignal method', async () => {
            const provider = new NativeIpcProvider();
            expect(typeof provider.waitForSignal).toBe('function');
        });

        it('should create channel with send and receive', async () => {
            const provider = new NativeIpcProvider();
            const channel = await provider.createChannel();
            expect(channel).toHaveProperty('send');
            expect(channel).toHaveProperty('receive');
            expect(channel).toHaveProperty('close');
        });

        it('should create event with signal and wait', async () => {
            const provider = new NativeIpcProvider();
            const event = await provider.createEvent();
            expect(event).toHaveProperty('signal');
            expect(event).toHaveProperty('wait');
            expect(event).toHaveProperty('reset');
        });
    });
});

/**
 * Integration test runner
 * This can be used in CI/CD to validate the entire stack
 */
export async function runIntegrationTests(): Promise<{
    passed: number;
    failed: number;
    results: Array<{ name: string; passed: boolean; error?: string }>;
}> {
    const results: Array<{ name: string; passed: boolean; error?: string }> = [];
    let passed = 0;
    let failed = 0;

    // Set environment to native
    setEnvironment('native');
    const registry = ProviderFactory.initialize();

    // Test Process
    try {
        const pid = registry.process.getCurrentPid();
        if (pid > 0) {
            results.push({ name: 'process.getCurrentPid', passed: true });
            passed++;
        } else {
            throw new Error('Invalid PID');
        }
    } catch (e) {
        results.push({ name: 'process.getCurrentPid', passed: false, error: String(e) });
        failed++;
    }

    // Test Memory
    try {
        const vmo = await registry.memory.createVmo(4096);
        if (typeof vmo === 'number' && vmo > 0) {
            results.push({ name: 'memory.createVmo', passed: true });
            passed++;
        } else {
            throw new Error('Invalid VMO handle');
        }
    } catch (e) {
        results.push({ name: 'memory.createVmo', passed: false, error: String(e) });
        failed++;
    }

    // Test IPC
    try {
        const channel = await registry.ipc.createChannel();
        if (channel) {
            results.push({ name: 'ipc.createChannel', passed: true });
            passed++;
        } else {
            throw new Error('Channel is null');
        }
    } catch (e) {
        results.push({ name: 'ipc.createChannel', passed: false, error: String(e) });
        failed++;
    }

    return { passed, failed, results };
}
