import type { ITimeProvider } from '../../api/ITimeProvider';

export class BrowserTimeProvider implements ITimeProvider {
    async now(): Promise<number> {
        return Date.now();
    }

    async getTimezoneOffset(): Promise<number> {
        return new Date().getTimezoneOffset();
    }
}
