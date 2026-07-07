/**
 * KLAT OS Kernel Providers
 *
 * Providers for process, filesystem, memory, and IPC operations.
 * Automatically selects the appropriate implementation based on runtime.
 */

// Provider Factory
export {
    ProviderFactory,
    getProcessProvider,
    getFilesystemProvider,
    getMemoryProvider,
    getIpcProvider,
    type ProviderRegistry,
} from './ProviderFactory';

// Process Provider
export {
    NativeProcessProvider,
    BrowserProcessProvider,
    type IProcessProvider,
    type ProcessState,
    type SpawnOptions,
} from './ProcessProvider';

// Filesystem Provider
export {
    NativeFilesystemProvider,
    BrowserFilesystemProvider,
    type IFilesystemProvider,
    type FileInfo,
    type ReadOptions,
    type WriteOptions,
} from './FilesystemProvider';

// Memory Provider
export {
    NativeMemoryProvider,
    BrowserMemoryProvider,
    type IMemoryProvider,
    type MemoryRegion,
    type AllocOptions,
} from './MemoryProvider';

// IPC Provider
export {
    NativeIpcProvider,
    BrowserIpcProvider,
    type IIpcProvider,
    type IpcChannel,
    type EventHandle,
    type Message,
} from './IpcProvider';
