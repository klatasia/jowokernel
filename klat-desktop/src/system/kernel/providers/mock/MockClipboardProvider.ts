import type { IClipboardProvider } from '../../api/IClipboardProvider';

export class MockClipboardProvider implements IClipboardProvider {
    private clipboardData: string = '';

    async readText(): Promise<string> {
        return this.clipboardData;
    }

    async writeText(text: string): Promise<void> {
        this.clipboardData = text;
    }
}
