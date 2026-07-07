import type { IStorageProvider } from '../../api/IStorageProvider';

export class BrowserStorageProvider implements IStorageProvider {
    async readFile(path: string): Promise<Uint8Array> {
        const data = localStorage.getItem(path);
        if (!data) throw new Error(`File not found: ${path}`);
        const parsed = JSON.parse(data);
        return new Uint8Array(parsed);
    }

    async readTextFile(path: string): Promise<string> {
        const data = localStorage.getItem(path);
        if (data === null) throw new Error(`File not found: ${path}`);
        return data;
    }

    async writeFile(path: string, data: Uint8Array): Promise<void> {
        localStorage.setItem(path, JSON.stringify(Array.from(data)));
    }

    async writeTextFile(path: string, text: string): Promise<void> {
        localStorage.setItem(path, text);
    }

    async deleteFile(path: string): Promise<void> {
        localStorage.removeItem(path);
    }

    async exists(path: string): Promise<boolean> {
        return localStorage.getItem(path) !== null;
    }

    async readDir(path: string): Promise<string[]> {
        const prefix = path.endsWith('/') ? path : path + '/';
        const files: string[] = [];
        for (let i = 0; i < localStorage.length; i++) {
            const key = localStorage.key(i);
            if (key && key.startsWith(prefix)) {
                files.push(key.substring(prefix.length).split('/')[0]);
            }
        }
        return [...new Set(files)]; // Unique files
    }

    async createDir(path: string): Promise<void> {
        // In local storage, directories are just prefixes, so nothing is explicitly created.
    }
}
