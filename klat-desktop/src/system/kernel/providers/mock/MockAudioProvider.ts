import type { IAudioProvider } from '../../api/IAudioProvider';

export class MockAudioProvider implements IAudioProvider {
    private volume: number = 1.0;
    private muted: boolean = false;

    async setVolume(level: number): Promise<void> {
        this.volume = Math.max(0, Math.min(1, level));
    }

    async getVolume(): Promise<number> {
        return this.volume;
    }

    async setMuted(muted: boolean): Promise<void> {
        this.muted = muted;
    }

    async isMuted(): Promise<boolean> {
        return this.muted;
    }

    async playSound(asset: string): Promise<void> {
        console.log(`[MockAudio] Playing sound: ${asset} at volume ${this.muted ? 0 : this.volume}`);
    }
}
