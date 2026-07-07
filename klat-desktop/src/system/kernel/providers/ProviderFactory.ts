/**
 * KLAT OS Kernel API - Provider Factory
 *
 * Factory for creating the appropriate provider based on runtime environment.
 * This is the main entry point for accessing kernel services.
 *
 * Architecture:
 * ```
 * Runtime → ProviderFactory → Provider (Native/Browser)
 *                              ↓
 *                          KernelAPI
 *                              ↓
 *                          kernel-api
 * ```
 */

import { setEnvironment, getEnvironment, type RuntimeEnvironment } from '../../runtime/kernel/syscall';

import {
    NativeProcessProvider,
    BrowserProcessProvider,
    type IProcessProvider,
} from './ProcessProvider';

import {
    NativeFilesystemProvider,
    BrowserFilesystemProvider,
    type IFilesystemProvider,
} from './FilesystemProvider';

import {
    NativeMemoryProvider,
    BrowserMemoryProvider,
    type IMemoryProvider,
} from './MemoryProvider';

import {
    NativeIpcProvider,
    BrowserIpcProvider,
    type IIpcProvider,
} from './IpcProvider';

// Re-export provider interfaces
export type { IProcessProvider, ProcessState, SpawnOptions } from './ProcessProvider';
export type { IFilesystemProvider, FileInfo, ReadOptions, WriteOptions } from './FilesystemProvider';
export type { IMemoryProvider, MemoryRegion, AllocOptions } from './MemoryProvider';
export type { IIpcProvider, IpcChannel, EventHandle, Message } from './IpcProvider';

// ============================================================
// Provider Registry
// ============================================================

export interface ProviderRegistry {
    process: IProcessProvider;
    filesystem: IFilesystemProvider;
    memory: IMemoryProvider;
    ipc: IIpcProvider;
}

// ============================================================
// Provider Factory
// ============================================================

class ProviderFactoryImpl {
    private registry: ProviderRegistry | null = null;
    private currentEnv: RuntimeEnvironment | null = null;

    /**
     * Initialize providers for the current environment
     */
    initialize(env?: RuntimeEnvironment): ProviderRegistry {
        const environment = env ?? this.detectEnvironment();

        // Return cached registry if environment hasn't changed
        if (this.registry && this.currentEnv === environment) {
            return this.registry;
        }

        this.currentEnv = environment;
        this.registry = this.createRegistry(environment);

        return this.registry;
    }

    /**
     * Get the provider registry
     */
    getRegistry(): ProviderRegistry {
        if (!this.registry) {
            return this.initialize();
        }
        return this.registry;
    }

    /**
     * Get a specific provider
     */
    getProvider<K extends keyof ProviderRegistry>(name: K): ProviderRegistry[K] {
        return this.getRegistry()[name];
    }

    /**
     * Get current environment
     */
    getEnvironment(): RuntimeEnvironment {
        return this.currentEnv ?? this.detectEnvironment();
    }

    /**
     * Create provider registry for the given environment
     */
    private createRegistry(env: RuntimeEnvironment): ProviderRegistry {
        switch (env) {
            case 'native':
                return this.createNativeRegistry();
            case 'test':
                return this.createTestRegistry();
            case 'browser':
            default:
                return this.createBrowserRegistry();
        }
    }

    /**
     * Create native providers (uses KernelAPI → kernel-api → syscall)
     */
    private createNativeRegistry(): ProviderRegistry {
        setEnvironment('native');

        return {
            process: new NativeProcessProvider(),
            filesystem: new NativeFilesystemProvider(),
            memory: new NativeMemoryProvider(),
            ipc: new NativeIpcProvider(),
        };
    }

    /**
     * Create browser providers (uses browser APIs)
     */
    private createBrowserRegistry(): ProviderRegistry {
        setEnvironment('browser');

        return {
            process: new BrowserProcessProvider(),
            filesystem: new BrowserFilesystemProvider(),
            memory: new BrowserMemoryProvider(),
            ipc: new BrowserIpcProvider(),
        };
    }

    /**
     * Create test providers (mock implementations)
     */
    private createTestRegistry(): ProviderRegistry {
        setEnvironment('test');

        return {
            process: new BrowserProcessProvider(),
            filesystem: new BrowserFilesystemProvider(),
            memory: new BrowserMemoryProvider(),
            ipc: new BrowserIpcProvider(),
        };
    }

    /**
     * Detect the current runtime environment
     */
    private detectEnvironment(): RuntimeEnvironment {
        // Check for native indicators
        if (typeof window === 'undefined' && typeof process !== 'undefined') {
            // Node.js - likely native
            if (process.env.KLAT_NATIVE === 'true') {
                return 'native';
            }
        }

        // Check for browser indicators
        if (typeof window !== 'undefined') {
            return 'browser';
        }

        // Default to native in Node.js without browser env
        if (typeof process !== 'undefined') {
            return 'native';
        }

        // Fallback to browser (safest default)
        return 'browser';
    }

    /**
     * Reset the factory (for testing)
     */
    reset(): void {
        this.registry = null;
        this.currentEnv = null;
    }
}

// ============================================================
// Singleton Export
// ============================================================

export const ProviderFactory = new ProviderFactoryImpl();

// ============================================================
// Convenience Hooks
// ============================================================

/**
 * Get the process provider for the current environment
 */
export function getProcessProvider(): IProcessProvider {
    return ProviderFactory.getProvider('process');
}

/**
 * Get the filesystem provider for the current environment
 */
export function getFilesystemProvider(): IFilesystemProvider {
    return ProviderFactory.getProvider('filesystem');
}

/**
 * Get the memory provider for the current environment
 */
export function getMemoryProvider(): IMemoryProvider {
    return ProviderFactory.getProvider('memory');
}

/**
 * Get the IPC provider for the current environment
 */
export function getIpcProvider(): IIpcProvider {
    return ProviderFactory.getProvider('ipc');
}

// ============================================================
// Default Export
// ============================================================

export default ProviderFactory;
