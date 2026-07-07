import type { IAudioProvider } from '../../api/IAudioProvider';

export class BrowserAudioProvider implements IAudioProvider {
    private volume: number = 1.0;
    private muted: boolean = false;
    private audioContext: AudioContext | null = null;

    private getAudioContext(): AudioContext {
        if (!this.audioContext) {
            this.audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
        }
        return this.audioContext;
    }

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
        if (this.muted) return;
        try {
            const audio = new Audio(asset);
            audio.volume = this.volume;
            await audio.play();
        } catch (error) {
            console.error('Error playing sound', error);
        }
    }
}
