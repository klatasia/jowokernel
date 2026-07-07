import type { IAudioProvider } from '../../api/IAudioProvider';
import type { IKernelTransport } from '../../transport/IKernelTransport';

export class KernelAudioProvider implements IAudioProvider {
    constructor(private transport: IKernelTransport) {}

    async setVolume(level: number): Promise<void> {
        await this.transport.send('audio:setVolume', { level });
    }

    async getVolume(): Promise<number> {
        return await this.transport.request<number>('audio:getVolume', {});
    }

    async setMuted(muted: boolean): Promise<void> {
        await this.transport.send('audio:setMuted', { muted });
    }

    async isMuted(): Promise<boolean> {
        return await this.transport.request<boolean>('audio:isMuted', {});
    }

    async playSound(asset: string): Promise<void> {
        await this.transport.send('audio:playSound', { asset });
    }
}
