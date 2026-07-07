/**
 * KLAT OS Kernel API - IPC Provider
 *
 * Interface and implementations for Inter-Process Communication.
 */

import {
    IPCAPI,
    ChannelHandle,
    HandleAPI,
    type Handle,
    SIGNAL_READABLE,
    SIGNAL_WRITABLE,
    SIGNAL_PEER_CLOSED,
    WAIT_INFINITE,
    Result,
} from '../../runtime/kernel';

// ============================================================
// Types
// ============================================================

export interface Message {
    type: string;
    payload: any;
    sender?: number;
    timestamp: number;
}

export interface IpcChannel {
    send(message: Message): Promise<void>;
    receive(timeout?: number): Promise<Message | null>;
    close(): void;
}

export interface EventHandle {
    signal(): Promise<void>;
    wait(timeout?: number): Promise<boolean>;
    reset(): Promise<void>;
    onSignal(callback: () => void): () => void;
}

// ============================================================
// Interface
// ============================================================

export interface IIpcProvider {
    /**
     * Create an IPC channel pair
     */
    createChannel(): Promise<IpcChannel>;

    /**
     * Create an event
     */
    createEvent(): Promise<EventHandle>;

    /**
     * Wait for handle to be signaled
     */
    waitForSignal(handle: number, signals: number, timeout?: number): Promise<number>;

    /**
     * Send a message through a channel
     */
    sendMessage(channel: IpcChannel, message: Message): Promise<void>;

    /**
     * Receive a message from a channel
     */
    receiveMessage(channel: IpcChannel, timeout?: number): Promise<Message | null>;
}

// ============================================================
// Channel Implementation
// ============================================================

class NativeIpcChannel implements IpcChannel {
    private channel: ChannelHandle;
    private closed = false;

    constructor(channel: ChannelHandle) {
        this.channel = channel;
    }

    async send(message: Message): Promise<void> {
        if (this.closed) {
            throw new Error('Channel is closed');
        }

        const data = new TextEncoder().encode(JSON.stringify(message));
        const result = this.channel.write(data.buffer);

        if (!result.ok) {
            throw new Error('Failed to send message');
        }
    }

    async receive(timeout?: number): Promise<Message | null> {
        if (this.closed) {
            return null;
        }

        const timeoutNs = timeout ? BigInt(timeout) * 1000000n : WAIT_INFINITE;

        // Wait for readable signal
        const signal = await IPCAPI.waitOne(this.channel.readHandle, SIGNAL_READABLE, timeoutNs);
        if (!signal.ok || (signal.value & SIGNAL_READABLE) === 0) {
            return null;
        }

        // Read the message
        const buffer = new ArrayBuffer(65536);
        const result = this.channel.read(buffer.byteLength);

        if (!result.ok) {
            return null;
        }

        try {
            const text = new TextDecoder().decode(buffer.slice(0, result.value as number));
            return JSON.parse(text) as Message;
        } catch {
            return null;
        }
    }

    close(): void {
        if (!this.closed) {
            this.closed = true;
            this.channel.close();
        }
    }
}

// ============================================================
// Event Implementation
// ============================================================

class NativeEventHandle implements EventHandle {
    private handle: Handle;
    private listeners: Array<() => void> = [];

    constructor(handle: Handle) {
        this.handle = handle;
    }

    async signal(): Promise<void> {
        // Would use event_signal syscall
        console.log('[NativeEventHandle] signal:', this.handle);
    }

    async wait(timeout?: number): Promise<boolean> {
        const timeoutNs = timeout ? BigInt(timeout) * 1000000n : WAIT_INFINITE;
        const result = IPCAPI.waitOne(this.handle, SIGNAL_SIGNALED, timeoutNs);
        return result.ok && (result.value & SIGNAL_SIGNALED) !== 0;
    }

    async reset(): Promise<void> {
        // Would use event_reset syscall
        console.log('[NativeEventHandle] reset:', this.handle);
    }

    onSignal(callback: () => void): () => void {
        this.listeners.push(callback);
        return () => {
            const index = this.listeners.indexOf(callback);
            if (index !== -1) {
                this.listeners.splice(index, 1);
            }
        };
    }
}

// ============================================================
// Native Implementation (uses KernelAPI)
// ============================================================

export class NativeIpcProvider implements IIpcProvider {
    async createChannel(): Promise<IpcChannel> {
        const result = IPCAPI.createChannel();
        if (result.ok && result.value) {
            return new NativeIpcChannel(result.value);
        }
        throw new Error('Failed to create IPC channel');
    }

    async createEvent(): Promise<EventHandle> {
        // Would use event_create syscall
        // For now, return a mock
        return {
            signal: async () => {},
            wait: async () => true,
            reset: async () => {},
            onSignal: () => () => {},
        };
    }

    async waitForSignal(handle: number, signals: number, timeout?: number): Promise<number> {
        const timeoutNs = timeout ? BigInt(timeout) * 1000000n : WAIT_INFINITE;
        const result = IPCAPI.waitOne(handle as Handle, signals, timeoutNs);
        if (result.ok) {
            return result.value;
        }
        return 0;
    }

    async sendMessage(channel: IpcChannel, message: Message): Promise<void> {
        await channel.send(message);
    }

    async receiveMessage(channel: IpcChannel, timeout?: number): Promise<Message | null> {
        return channel.receive(timeout);
    }
}

// ============================================================
// Browser Implementation (uses BroadcastChannel / MessageChannel)
// ============================================================

class BrowserIpcChannel implements IpcChannel {
    private bc: BroadcastChannel | MessagePort;
    private messageQueue: Message[] = [];
    private listeners: Array<(msg: Message) => void> = [];
    private closed = false;
    private useBroadcast: boolean;

    constructor(name: string, port?: MessagePort) {
        this.useBroadcast = port === undefined;

        if (this.useBroadcast) {
            this.bc = new BroadcastChannel(name);
            this.bc.onmessage = (event) => {
                const message = event.data as Message;
                this.messageQueue.push(message);
                this.listeners.forEach(cb => cb(message));
            };
        } else {
            this.bc = port!;
            this.bc.onmessage = (event) => {
                const message = event.data as Message;
                this.messageQueue.push(message);
                this.listeners.forEach(cb => cb(message));
            };
        }
    }

    async send(message: Message): Promise<void> {
        if (this.closed) {
            throw new Error('Channel is closed');
        }
        this.bc.postMessage(message);
    }

    async receive(timeout?: number): Promise<Message | null> {
        if (this.closed) {
            return null;
        }

        // Check for queued messages
        if (this.messageQueue.length > 0) {
            return this.messageQueue.shift()!;
        }

        // Wait for message
        if (timeout !== undefined && timeout > 0) {
            return new Promise((resolve) => {
                const listener = (msg: Message) => {
                    resolve(msg);
                    this.removeListener(listener);
                };
                this.addListener(listener);

                setTimeout(() => {
                    this.removeListener(listener);
                    resolve(null);
                }, timeout);
            });
        }

        // No timeout - wait indefinitely
        return new Promise((resolve) => {
            const listener = (msg: Message) => {
                resolve(msg);
                this.removeListener(listener);
            };
            this.addListener(listener);
        });
    }

    close(): void {
        if (!this.closed) {
            this.closed = true;
            if (this.useBroadcast && this.bc instanceof BroadcastChannel) {
                this.bc.close();
            }
        }
    }

    private addListener(cb: (msg: Message) => void): void {
        this.listeners.push(cb);
    }

    private removeListener(cb: (msg: Message) => void): void {
        const index = this.listeners.indexOf(cb);
        if (index !== -1) {
            this.listeners.splice(index, 1);
        }
    }
}

class BrowserEventHandle implements EventHandle {
    private listeners: Array<() => void> = [];
    private signaled = false;

    async signal(): Promise<void> {
        this.signaled = true;
        this.listeners.forEach(cb => cb());
    }

    async wait(timeout?: number): Promise<boolean> {
        if (this.signaled) {
            return true;
        }

        if (timeout !== undefined && timeout > 0) {
            return new Promise((resolve) => {
                setTimeout(() => resolve(false), timeout);
            });
        }

        // No timeout - wait indefinitely (but JS doesn't support this well)
        return false;
    }

    async reset(): Promise<void> {
        this.signaled = false;
    }

    onSignal(callback: () => void): () => void {
        this.listeners.push(callback);
        return () => {
            const index = this.listeners.indexOf(callback);
            if (index !== -1) {
                this.listeners.splice(index, 1);
            }
        };
    }
}

export class BrowserIpcProvider implements IIpcProvider {
    private channels: Map<string, IpcChannel> = new Map();

    async createChannel(): Promise<IpcChannel> {
        const id = Math.random().toString(36).substring(2, 15);
        const channel = new BrowserIpcChannel(`klat-ipc-${id}`);
        this.channels.set(id, channel);
        return channel;
    }

    async createEvent(): Promise<EventHandle> {
        return new BrowserEventHandle();
    }

    async waitForSignal(_handle: number, _signals: number, _timeout?: number): Promise<number> {
        // Browser doesn't have native waitForSignal
        return 0;
    }

    async sendMessage(channel: IpcChannel, message: Message): Promise<void> {
        await channel.send(message);
    }

    async receiveMessage(channel: IpcChannel, timeout?: number): Promise<Message | null> {
        return channel.receive(timeout);
    }
}

// ============================================================
// Default Export
// ============================================================
