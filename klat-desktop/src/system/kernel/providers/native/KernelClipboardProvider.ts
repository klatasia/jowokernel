import type { IClipboardProvider } from '../../api/IClipboardProvider';
import type { IKernelTransport } from '../../transport/IKernelTransport';

export class KernelClipboardProvider implements IClipboardProvider {
    constructor(private transport: IKernelTransport) {}

    async readText(): Promise<string> {
        return await this.transport.request<string>('clipboard:read', {});
    }

    async writeText(text: string): Promise<void> {
        await this.transport.send('clipboard:write', { text });
    }
}
