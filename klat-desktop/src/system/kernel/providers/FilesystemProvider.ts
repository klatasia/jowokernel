/**
 * KLAT OS Kernel API - Filesystem Provider
 *
 * Interface and implementations for filesystem operations.
 */

import {
    FileAPI,
    FileDescriptorHandle,
    type FileStats,
    type FileDescriptor,
    OPEN_FLAG_READ,
    OPEN_FLAG_WRITE,
    OPEN_FLAG_CREATE,
    OPEN_FLAG_TRUNC,
    OPEN_FLAG_APPEND,
    MMAP_PROT_READ,
    MMAP_PROT_WRITE,
    MMAP_FLAG_PRIVATE,
    Result,
    isOk,
    KERNEL_ENOENT,
    KERNEL_EISDIR,
    KERNEL_ENOTDIR,
    FS_MODE_TYPE_REG,
    FS_MODE_TYPE_DIR,
} from '../../runtime/kernel';

// ============================================================
// Types
// ============================================================

export interface FileInfo {
    name: string;
    path: string;
    size: number;
    isDirectory: boolean;
    isFile: boolean;
    createdAt?: number;
    modifiedAt?: number;
    permissions?: string;
}

export interface ReadOptions {
    offset?: number;
    length?: number;
}

export interface WriteOptions {
    offset?: number;
    append?: boolean;
}

// ============================================================
// Interface
// ============================================================

export interface IFilesystemProvider {
    /**
     * Check if path exists
     */
    exists(path: string): Promise<boolean>;

    /**
     * Check if path is a file
     */
    isFile(path: string): Promise<boolean>;

    /**
     * Check if path is a directory
     */
    isDirectory(path: string): Promise<boolean>;

    /**
     * Read file as text
     */
    readTextFile(path: string, options?: ReadOptions): Promise<string>;

    /**
     * Read file as binary
     */
    readBinaryFile(path: string, options?: ReadOptions): Promise<Uint8Array>;

    /**
     * Write text to file
     */
    writeTextFile(path: string, content: string, options?: WriteOptions): Promise<void>;

    /**
     * Write binary to file
     */
    writeBinaryFile(path: string, content: Uint8Array, options?: WriteOptions): Promise<void>;

    /**
     * Create directory
     */
    createDirectory(path: string): Promise<void>;

    /**
     * Delete file
     */
    deleteFile(path: string): Promise<void>;

    /**
     * Delete directory
     */
    deleteDirectory(path: string, recursive?: boolean): Promise<void>;

    /**
     * List directory contents
     */
    listDirectory(path: string): Promise<FileInfo[]>;

    /**
     * Get file info
     */
    getFileInfo(path: string): Promise<FileInfo | null>;

    /**
     * Copy file
     */
    copyFile(source: string, destination: string): Promise<void>;

    /**
     * Move file
     */
    moveFile(source: string, destination: string): Promise<void>;

    /**
     * Rename file or directory
     */
    rename(oldPath: string, newPath: string): Promise<void>;

    /**
     * Get total disk space
     */
    getTotalSpace(): Promise<number>;

    /**
     * Get available disk space
     */
    getFreeSpace(): Promise<number>;
}

// ============================================================
// Native Implementation (uses KernelAPI)
// ============================================================

export class NativeFilesystemProvider implements IFilesystemProvider {
    async exists(path: string): Promise<boolean> {
        try {
            const result = await this.getFileInfo(path);
            return result !== null;
        } catch {
            return false;
        }
    }

    async isFile(path: string): Promise<boolean> {
        const info = await this.getFileInfo(path);
        return info?.isFile ?? false;
    }

    async isDirectory(path: string): Promise<boolean> {
        const info = await this.getFileInfo(path);
        return info?.isDirectory ?? false;
    }

    async readTextFile(path: string, _options?: ReadOptions): Promise<string> {
        const file = FileAPI.open(path, OPEN_FLAG_READ);
        if (!file.ok) {
            throw new Error(`Failed to open file: ${path}`);
        }

        const buffer = new ArrayBuffer(65536);
        const result = file.value.read(buffer);

        file.value.close();

        if (!result.ok) {
            throw new Error(`Failed to read file: ${path}`);
        }

        const decoder = new TextDecoder();
        return decoder.decode(buffer.slice(0, result.value));
    }

    async readBinaryFile(path: string, _options?: ReadOptions): Promise<Uint8Array> {
        const file = FileAPI.open(path, OPEN_FLAG_READ);
        if (!file.ok) {
            throw new Error(`Failed to open file: ${path}`);
        }

        const buffer = new ArrayBuffer(65536);
        const result = file.value.read(buffer);

        file.value.close();

        if (!result.ok) {
            throw new Error(`Failed to read file: ${path}`);
        }

        return new Uint8Array(buffer.slice(0, result.value));
    }

    async writeTextFile(path: string, content: string, options?: WriteOptions): Promise<void> {
        const flags = OPEN_FLAG_WRITE | OPEN_FLAG_CREATE;
        const finalFlags = options?.append ? flags | OPEN_FLAG_APPEND : flags | OPEN_FLAG_TRUNC;

        const file = FileAPI.open(path, finalFlags);
        if (!file.ok) {
            throw new Error(`Failed to open file: ${path}`);
        }

        const encoder = new TextEncoder();
        const buffer = encoder.encode(content);
        const ab = buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength);

        const result = file.value.write(ab);
        file.value.close();

        if (!result.ok) {
            throw new Error(`Failed to write file: ${path}`);
        }
    }

    async writeBinaryFile(path: string, content: Uint8Array, options?: WriteOptions): Promise<void> {
        const flags = OPEN_FLAG_WRITE | OPEN_FLAG_CREATE;
        const finalFlags = options?.append ? flags | OPEN_FLAG_APPEND : flags | OPEN_FLAG_TRUNC;

        const file = FileAPI.open(path, finalFlags);
        if (!file.ok) {
            throw new Error(`Failed to open file: ${path}`);
        }

        const result = file.value.write(content.buffer);
        file.value.close();

        if (!result.ok) {
            throw new Error(`Failed to write file: ${path}`);
        }
    }

    async createDirectory(path: string): Promise<void> {
        // Would use mkdir syscall
        console.log('[NativeFilesystemProvider] createDirectory:', path);
    }

    async deleteFile(path: string): Promise<void> {
        // Would use unlink syscall
        console.log('[NativeFilesystemProvider] deleteFile:', path);
    }

    async deleteDirectory(path: string, _recursive?: boolean): Promise<void> {
        // Would use rmdir syscall
        console.log('[NativeFilesystemProvider] deleteDirectory:', path);
    }

    async listDirectory(path: string): Promise<FileInfo[]> {
        // Would use readdir syscall
        console.log('[NativeFilesystemProvider] listDirectory:', path);
        return [];
    }

    async getFileInfo(path: string): Promise<FileInfo | null> {
        // Would use stat syscall
        console.log('[NativeFilesystemProvider] getFileInfo:', path);
        return null;
    }

    async copyFile(_source: string, _destination: string): Promise<void> {
        // Would use read/write syscalls
    }

    async moveFile(_source: string, _destination: string): Promise<void> {
        // Would use rename syscall
    }

    async rename(_oldPath: string, _newPath: string): Promise<void> {
        // Would use rename syscall
    }

    async getTotalSpace(): Promise<number> {
        return 0;
    }

    async getFreeSpace(): Promise<number> {
        return 0;
    }
}

// ============================================================
// Browser Implementation (uses localStorage / IndexedDB)
// ============================================================

export class BrowserFilesystemProvider implements IFilesystemProvider {
    async exists(path: string): Promise<boolean> {
        if (typeof localStorage !== 'undefined') {
            return localStorage.getItem(path) !== null;
        }
        return false;
    }

    async isFile(path: string): Promise<boolean> {
        if (typeof localStorage !== 'undefined') {
            const value = localStorage.getItem(path);
            if (value !== null) {
                try {
                    JSON.parse(value);
                    return true;
                } catch {
                    return false;
                }
            }
        }
        return false;
    }

    async isDirectory(path: string): Promise<boolean> {
        if (typeof localStorage !== 'undefined') {
            const prefix = path.endsWith('/') ? path : path + '/';
            for (let i = 0; i < localStorage.length; i++) {
                const key = localStorage.key(i);
                if (key && key.startsWith(prefix)) {
                    return true;
                }
            }
        }
        return false;
    }

    async readTextFile(path: string): Promise<string> {
        if (typeof localStorage !== 'undefined') {
            const data = localStorage.getItem(path);
            if (data === null) {
                throw new Error(`File not found: ${path}`);
            }
            return data;
        }
        throw new Error('Storage not available');
    }

    async readBinaryFile(path: string): Promise<Uint8Array> {
        if (typeof localStorage !== 'undefined') {
            const data = localStorage.getItem(path);
            if (data === null) {
                throw new Error(`File not found: ${path}`);
            }
            const parsed = JSON.parse(data);
            return new Uint8Array(parsed);
        }
        throw new Error('Storage not available');
    }

    async writeTextFile(path: string, content: string): Promise<void> {
        if (typeof localStorage !== 'undefined') {
            localStorage.setItem(path, content);
            return;
        }
        throw new Error('Storage not available');
    }

    async writeBinaryFile(path: string, content: Uint8Array): Promise<void> {
        if (typeof localStorage !== 'undefined') {
            localStorage.setItem(path, JSON.stringify(Array.from(content)));
            return;
        }
        throw new Error('Storage not available');
    }

    async createDirectory(_path: string): Promise<void> {
        // Directories are implicit in localStorage
    }

    async deleteFile(path: string): Promise<void> {
        if (typeof localStorage !== 'undefined') {
            localStorage.removeItem(path);
        }
    }

    async deleteDirectory(path: string, recursive?: boolean): Promise<void> {
        if (typeof localStorage !== 'undefined') {
            const prefix = path.endsWith('/') ? path : path + '/';

            if (recursive) {
                const keysToDelete: string[] = [];
                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    if (key && (key === path || key.startsWith(prefix))) {
                        keysToDelete.push(key);
                    }
                }
                keysToDelete.forEach(key => localStorage.removeItem(key));
            } else {
                localStorage.removeItem(path);
            }
        }
    }

    async listDirectory(path: string): Promise<FileInfo[]> {
        if (typeof localStorage !== 'undefined') {
            const prefix = path.endsWith('/') ? path : path + '/';
            const seen = new Set<string>();
            const files: FileInfo[] = [];

            for (let i = 0; i < localStorage.length; i++) {
                const key = localStorage.key(i);
                if (key && key.startsWith(prefix)) {
                    const relativePath = key.substring(prefix.length);
                    const firstSegment = relativePath.split('/')[0];

                    if (firstSegment && !seen.has(firstSegment)) {
                        seen.add(firstSegment);
                        const fullPath = prefix + firstSegment;
                        files.push({
                            name: firstSegment,
                            path: fullPath,
                            size: localStorage.getItem(fullPath)?.length ?? 0,
                            isDirectory: fullPath !== key,
                            isFile: fullPath === key,
                        });
                    }
                }
            }

            return files;
        }
        return [];
    }

    async getFileInfo(path: string): Promise<FileInfo | null> {
        if (typeof localStorage !== 'undefined') {
            if (localStorage.getItem(path) !== null) {
                return {
                    name: path.split('/').pop() || path,
                    path,
                    size: localStorage.getItem(path)?.length ?? 0,
                    isDirectory: false,
                    isFile: true,
                };
            }

            // Check if it's a directory
            const prefix = path.endsWith('/') ? path : path + '/';
            for (let i = 0; i < localStorage.length; i++) {
                const key = localStorage.key(i);
                if (key && key.startsWith(prefix)) {
                    return {
                        name: path.split('/').pop() || path,
                        path,
                        size: 0,
                        isDirectory: true,
                        isFile: false,
                    };
                }
            }
        }
        return null;
    }

    async copyFile(source: string, destination: string): Promise<void> {
        if (typeof localStorage !== 'undefined') {
            const data = localStorage.getItem(source);
            if (data !== null) {
                localStorage.setItem(destination, data);
            }
        }
    }

    async moveFile(source: string, destination: string): Promise<void> {
        if (typeof localStorage !== 'undefined') {
            const data = localStorage.getItem(source);
            if (data !== null) {
                localStorage.setItem(destination, data);
                localStorage.removeItem(source);
            }
        }
    }

    async rename(oldPath: string, newPath: string): Promise<void> {
        await this.moveFile(oldPath, newPath);
    }

    async getTotalSpace(): Promise<number> {
        if (typeof navigator !== 'undefined' && navigator.storage) {
            const estimate = await navigator.storage.estimate();
            return estimate.quota || 0;
        }
        return 0;
    }

    async getFreeSpace(): Promise<number> {
        if (typeof navigator !== 'undefined' && navigator.storage) {
            const estimate = await navigator.storage.estimate();
            return (estimate.quota || 0) - (estimate.usage || 0);
        }
        return 0;
    }
}

// ============================================================
// Default Export
// ============================================================

export type { FileStats } from '../../runtime/kernel';
