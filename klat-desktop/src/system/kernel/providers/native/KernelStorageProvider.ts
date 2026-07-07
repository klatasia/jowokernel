import type { IStorageProvider } from '../../api/IStorageProvider';
import type { IKernelTransport } from '../../transport/IKernelTransport';

export class KernelStorageProvider implements IStorageProvider {
    constructor(private transport: IKernelTransport) {}

    async readFile(path: string): Promise<Uint8Array> {
        const data = await this.transport.request<number[]>('storage:readFile', { path });
        return new Uint8Array(data);
    }

    async readTextFile(path: string): Promise<string> {
        return await this.transport.request<string>('storage:readTextFile', { path });
    }

    async writeFile(path: string, data: Uint8Array): Promise<void> {
        await this.transport.send('storage:writeFile', { path, data: Array.from(data) });
    }

    async writeTextFile(path: string, text: string): Promise<void> {
        await this.transport.send('storage:writeTextFile', { path, text });
    }

    async deleteFile(path: string): Promise<void> {
        await this.transport.send('storage:deleteFile', { path });
    }

    async exists(path: string): Promise<boolean> {
        return await this.transport.request<boolean>('storage:exists', { path });
    }

    async readDir(path: string): Promise<string[]> {
        return await this.transport.request<string[]>('storage:readDir', { path });
    }

    async createDir(path: string): Promise<void> {
        await this.transport.send('storage:createDir', { path });
    }
}
