import type { IPowerProvider, BatteryState } from '../../api/IPowerProvider';

export class MockPowerProvider implements IPowerProvider {
    private state: BatteryState = {
        charging: true,
        level: 1,
        chargingTime: 0,
        dischargingTime: Infinity
    };
    private listeners: Array<(state: BatteryState) => void> = [];

    async getBatteryState(): Promise<BatteryState> {
        return this.state;
    }

    onBatteryStateChange(callback: (state: BatteryState) => void): () => void {
        this.listeners.push(callback);
        return () => {
            this.listeners = this.listeners.filter(cb => cb !== callback);
        };
    }

    async suspend(): Promise<void> {
        console.log('[MockPower] System suspended');
    }

    async shutdown(): Promise<void> {
        console.log('[MockPower] System shutdown');
    }

    async reboot(): Promise<void> {
        console.log('[MockPower] System rebooted');
    }

    // For testing
    simulateChange(newState: Partial<BatteryState>) {
        this.state = { ...this.state, ...newState };
        this.listeners.forEach(cb => cb(this.state));
    }
}
