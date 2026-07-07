import type { IClipboardProvider } from '../../api/IClipboardProvider';

export class BrowserClipboardProvider implements IClipboardProvider {
    async readText(): Promise<string> {
        if (!navigator.clipboard) {
            throw new Error('Clipboard API not available in this browser');
        }
        return await navigator.clipboard.readText();
    }

    async writeText(text: string): Promise<void> {
        if (!navigator.clipboard) {
            throw new Error('Clipboard API not available in this browser');
        }
        await navigator.clipboard.writeText(text);
    }
}
