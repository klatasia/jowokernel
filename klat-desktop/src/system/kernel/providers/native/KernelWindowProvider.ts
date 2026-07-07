import type { IWindowProvider, WindowState } from '../../api/IWindowProvider';
import type { IKernelTransport } from '../../transport/IKernelTransport';

export class KernelWindowProvider implements IWindowProvider {
    constructor(private transport: IKernelTransport) {}

    async createWindow(options: Partial<WindowState>): Promise<string> {
        return await this.transport.request<string>('window:create', options);
    }

    async destroyWindow(id: string): Promise<void> {
        await this.transport.send('window:destroy', { id });
    }

    async setWindowTitle(id: string, title: string): Promise<void> {
        await this.transport.send('window:setTitle', { id, title });
    }

    async setWindowBounds(id: string, x: number, y: number, width: number, height: number): Promise<void> {
        await this.transport.send('window:setBounds', { id, x, y, width, height });
    }

    async focusWindow(id: string): Promise<void> {
        await this.transport.send('window:focus', { id });
    }

    async maximizeWindow(id: string): Promise<void> {
        await this.transport.send('window:maximize', { id });
    }

    async minimizeWindow(id: string): Promise<void> {
        await this.transport.send('window:minimize', { id });
    }

    async restoreWindow(id: string): Promise<void> {
        await this.transport.send('window:restore', { id });
    }

    onWindowStateChange(id: string, callback: (state: WindowState) => void): () => void {
        const subId = this.transport.subscribe(`window:stateChange:${id}`, callback);
        return () => this.transport.unsubscribe(`window:stateChange:${id}`, subId);
    }
}
