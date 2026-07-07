import type { IStorageProvider } from '../../api/IStorageProvider';

export class MockStorageProvider implements IStorageProvider {
    private memFs: Map<string, Uint8Array> = new Map();

    async readFile(path: string): Promise<Uint8Array> {
        const data = this.memFs.get(path);
        if (!data) throw new Error(`File not found: ${path}`);
        return new Uint8Array(data); // return copy
    }

    async readTextFile(path: string): Promise<string> {
        const data = await this.readFile(path);
        return new TextDecoder().decode(data);
    }

    async writeFile(path: string, data: Uint8Array): Promise<void> {
        this.memFs.set(path, new Uint8Array(data));
    }

    async writeTextFile(path: string, text: string): Promise<void> {
        const data = new TextEncoder().encode(text);
        await this.writeFile(path, data);
    }

    async deleteFile(path: string): Promise<void> {
        this.memFs.delete(path);
    }

    async exists(path: string): Promise<boolean> {
        return this.memFs.has(path);
    }

    async readDir(path: string): Promise<string[]> {
        const prefix = path.endsWith('/') ? path : path + '/';
        const files: string[] = [];
        for (const key of this.memFs.keys()) {
            if (key.startsWith(prefix)) {
                files.push(key.substring(prefix.length).split('/')[0]);
            }
        }
        return [...new Set(files)];
    }

    async createDir(path: string): Promise<void> {
        // mock dir
    }
}
