/**
 * KLAT OS Kernel API - Memory Provider
 *
 * Interface and implementations for memory management.
 */

import {
    MemoryAPI,
    type Handle,
    MMAP_PROT_READ,
    MMAP_PROT_WRITE,
    MMAP_PROT_READ_WRITE,
    MMAP_FLAG_PRIVATE,
    MMAP_FLAG_ANONYMOUS,
    Result,
} from '../../runtime/kernel';

// ============================================================
// Types
// ============================================================

export interface MemoryRegion {
    address: bigint;
    size: number;
    protection: 'r' | 'w' | 'x' | 'rw' | 'rwx';
    shared: boolean;
    mapped: boolean;
}

export interface AllocOptions {
    size: number;
    executable?: boolean;
    shared?: boolean;
    zero?: boolean;
}

// ============================================================
// Interface
// ============================================================

export interface IMemoryProvider {
    /**
     * Allocate memory
     */
    alloc(options: AllocOptions): Promise<Uint8Array>;

    /**
     * Free memory
     */
    free(ptr: Uint8Array): Promise<void>;

    /**
     * Get memory statistics
     */
    getStats(): Promise<{
        total: number;
        used: number;
        free: number;
    }>;

    /**
     * Create a VMO (Virtual Memory Object)
     */
    createVmo(size: number): Promise<number>;

    /**
     * Map VMO into address space
     */
    mapVmo(handle: number, size: number, prot: number): Promise<bigint>;

    /**
     * Unmap memory region
     */
    unmap(address: bigint, size: number): Promise<void>;

    /**
     * Change memory protection
     */
    mprotect(address: bigint, size: number, prot: number): Promise<void>;

    /**
     * Get list of memory regions
     */
    getRegions(): Promise<MemoryRegion[]>;
}

// ============================================================
// Native Implementation (uses KernelAPI)
// ============================================================

export class NativeMemoryProvider implements IMemoryProvider {
    async alloc(options: AllocOptions): Promise<Uint8Array> {
        const result = MemoryAPI.alloc(options.size);
        if (result.ok && result.value) {
            return result.value;
        }
        throw new Error('Failed to allocate memory');
    }

    async free(_ptr: Uint8Array): Promise<void> {
        // In native mode, memory is managed by the kernel
        // Explicit free is typically not needed for mmap'd memory
    }

    async getStats(): Promise<{
        total: number;
        used: number;
        free: number;
    }> {
        // Would query kernel for memory statistics
        return {
            total: 0,
            used: 0,
            free: 0,
        };
    }

    async createVmo(size: number): Promise<number> {
        const result = MemoryAPI.createVmo(BigInt(size));
        if (result.ok && result.value !== undefined) {
            return Number(result.value);
        }
        throw new Error('Failed to create VMO');
    }

    async mapVmo(handle: number, size: number, prot: number): Promise<bigint> {
        const result = MemoryAPI.map(
            handle as Handle,
            BigInt(size),
            prot,
            MMAP_FLAG_PRIVATE
        );
        if (result.ok && result.value !== undefined) {
            return result.value;
        }
        throw new Error('Failed to map VMO');
    }

    async unmap(address: bigint, size: number): Promise<void> {
        // Would use munmap syscall
        console.log('[NativeMemoryProvider] unmap:', address, size);
    }

    async mprotect(address: bigint, size: number, prot: number): Promise<void> {
        // Would use mprotect syscall
        console.log('[NativeMemoryProvider] mprotect:', address, size, prot);
    }

    async getRegions(): Promise<MemoryRegion[]> {
        // Would query kernel for memory regions
        return [];
    }
}

// ============================================================
// Browser Implementation (uses ArrayBuffer / SharedArrayBuffer)
// ============================================================

export class BrowserMemoryProvider implements IMemoryProvider {
    private allocations: Map<Uint8Array, { size: number }> = new Map();

    async alloc(options: AllocOptions): Promise<Uint8Array> {
        const buffer = new ArrayBuffer(options.size);
        const view = new Uint8Array(buffer);

        // Zero the buffer if requested
        if (options.zero !== false) {
            view.fill(0);
        }

        this.allocations.set(view, { size: options.size });

        return view;
    }

    async free(ptr: Uint8Array): Promise<void> {
        this.allocations.delete(ptr);
        // In JS, ArrayBuffer is garbage collected
    }

    async getStats(): Promise<{
        total: number;
        used: number;
        free: number;
    }> {
        let used = 0;
        for (const [, meta] of this.allocations) {
            used += meta.size;
        }

        // Get browser memory info if available
        let total = 0;
        if (typeof performance !== 'undefined' && 'memory' in performance) {
            const memInfo = (performance as any).memory;
            total = memInfo.jsHeapSizeLimit || 0;
        }

        return {
            total,
            used,
            free: total - used,
        };
    }

    async createVmo(size: number): Promise<number> {
        // In browser, we simulate VMO with a handle
        const handle = this.allocations.size + 100;
        this.allocations.set(new Uint8Array(size), { size });
        return handle;
    }

    async mapVmo(handle: number, size: number, prot: number): Promise<bigint> {
        // Simulate mapping - return a fake address
        console.log('[BrowserMemoryProvider] mapVmo:', handle, size, prot);
        return BigInt(handle * 0x10000);
    }

    async unmap(address: bigint, size: number): Promise<void> {
        // No-op in browser
        console.log('[BrowserMemoryProvider] unmap:', address, size);
    }

    async mprotect(address: bigint, size: number, prot: number): Promise<void> {
        // No-op in browser (no security model for ArrayBuffer)
        console.log('[BrowserMemoryProvider] mprotect:', address, size, prot);
    }

    async getRegions(): Promise<MemoryRegion[]> {
        const regions: MemoryRegion[] = [];
        let baseAddr = 0x1000000000n;

        for (const [ptr, meta] of this.allocations) {
            regions.push({
                address: baseAddr,
                size: meta.size,
                protection: 'rw',
                shared: false,
                mapped: true,
            });
            baseAddr += BigInt(meta.size);
            // Prevent GC of ptr
            void ptr[0];
        }

        return regions;
    }
}

// ============================================================
// Default Export
// ============================================================
