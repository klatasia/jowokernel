import type { ITimeProvider } from '../../api/ITimeProvider';

export class MockTimeProvider implements ITimeProvider {
    private baseTime: number = Date.now();
    
    async now(): Promise<number> {
        return this.baseTime;
    }

    async getTimezoneOffset(): Promise<number> {
        return 0; // UTC
    }
    
    // For testing
    setTime(time: number) {
        this.baseTime = time;
    }
}
