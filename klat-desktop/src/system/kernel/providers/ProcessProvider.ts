/**
 * KLAT OS Kernel API - Process Provider
 *
 * Interface and implementations for process management.
 */

import {
    ProcessAPI,
    KernelAPI,
    type ProcessId,
    type Handle,
    type ProcessInfo,
    Result,
    INVALID_PID,
    getError,
    isOk,
} from '../../runtime/kernel';

// ============================================================
// Types
// ============================================================

export interface ProcessState {
    pid: number;
    name: string;
    status: 'running' | 'sleeping' | 'stopped' | 'zombie' | 'dead';
    memoryUsage?: number;
    cpuUsage?: number;
    parentPid?: number;
    createdAt?: number;
}

export interface SpawnOptions {
    name: string;
    argv?: string[];
    env?: Record<string, string>;
    cwd?: string;
    stdio?: 'inherit' | 'pipe' | 'ignore';
}

// ============================================================
// Interface
// ============================================================

export interface IProcessProvider {
    /**
     * Get current process ID
     */
    getCurrentPid(): number;

    /**
     * Get process information
     */
    getProcessInfo(pid: number): Promise<ProcessInfo | null>;

    /**
     * Get all running processes
     */
    getProcessList(): Promise<ProcessState[]>;

    /**
     * Spawn a new process
     */
    spawn(options: SpawnOptions): Promise<number>;

    /**
     * Wait for process to exit
     */
    wait(pid: number, timeout?: number): Promise<number>;

    /**
     * Kill a process
     */
    kill(pid: number): Promise<boolean>;

    /**
     * Get current working directory
     */
    getCwd(): Promise<string>;

    /**
     * Set current working directory
     */
    setCwd(path: string): Promise<void>;
}

// ============================================================
// Native Implementation (uses KernelAPI)
// ============================================================

export class NativeProcessProvider implements IProcessProvider {
    getCurrentPid(): number {
        return ProcessAPI.getId();
    }

    async getProcessInfo(pid: number): Promise<ProcessInfo | null> {
        const result = ProcessAPI.getInfo(pid as ProcessId);
        if (result.ok && result.value) {
            return result.value;
        }
        return null;
    }

    async getProcessList(): Promise<ProcessState[]> {
        // In native mode, this would enumerate all processes
        // For now, return just the current process
        const pid = this.getCurrentPid();
        const info = await this.getProcessInfo(pid);
        if (info) {
            return [{
                pid: info.pid,
                name: info.name,
                status: 'running',
                parentPid: info.parentPid,
                createdAt: Number(info.startTime),
            }];
        }
        return [];
    }

    async spawn(options: SpawnOptions): Promise<number> {
        const result = ProcessAPI.create(options.name);
        if (result.ok) {
            return Number(result.value);
        }
        throw new Error(`Failed to spawn process: ${options.name}`);
    }

    async wait(pid: number, timeout?: number): Promise<number> {
        const timeoutNs = timeout ? BigInt(timeout) * 1000000n : -1n;
        const result = ProcessAPI.wait(pid as ProcessId, timeoutNs);
        if (result.ok) {
            return result.value as number;
        }
        return -1;
    }

    async kill(pid: number): Promise<boolean> {
        const result = ProcessAPI.kill(pid as ProcessId);
        return result.ok;
    }

    async getCwd(): Promise<string> {
        // Would use syscall to get current directory
        return '/';
    }

    async setCwd(_path: string): Promise<void> {
        // Would use syscall to set current directory
    }
}

// ============================================================
// Browser Implementation (mock)
// ============================================================

export class BrowserProcessProvider implements IProcessProvider {
    getCurrentPid(): number {
        return 1;
    }

    async getProcessInfo(_pid: number): Promise<ProcessInfo | null> {
        return {
            pid: _pid,
            parentPid: 0,
            name: 'browser-process',
            flags: 0,
            startTime: 0n,
            stackBase: 0n,
            stackLimit: 0n,
            entryPoint: 0n,
        };
    }

    async getProcessList(): Promise<ProcessState[]> {
        return [{
            pid: 1,
            name: 'main',
            status: 'running',
            createdAt: Date.now(),
        }];
    }

    async spawn(options: SpawnOptions): Promise<number> {
        console.log('[BrowserProcessProvider] spawn:', options);
        return Math.floor(Math.random() * 10000) + 100;
    }

    async wait(_pid: number, timeout?: number): Promise<number> {
        if (timeout && timeout > 0) {
            await new Promise(resolve => setTimeout(resolve, timeout));
        }
        return 0;
    }

    async kill(_pid: number): Promise<boolean> {
        return false;
    }

    async getCwd(): Promise<string> {
        return '/';
    }

    async setCwd(_path: string): Promise<void> {
        // No-op in browser
    }
}

// ============================================================
// Default Export
// ============================================================

export type { ProcessInfo } from '../../runtime/kernel';
