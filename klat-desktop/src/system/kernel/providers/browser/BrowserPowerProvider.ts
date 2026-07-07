import type { IPowerProvider, BatteryState } from '../../api/IPowerProvider';

export class BrowserPowerProvider implements IPowerProvider {
    private batteryManager: any = null;

    private async getBatteryManager(): Promise<any> {
        if (!this.batteryManager && (navigator as any).getBattery) {
            this.batteryManager = await (navigator as any).getBattery();
        }
        return this.batteryManager;
    }

    async getBatteryState(): Promise<BatteryState> {
        const manager = await this.getBatteryManager();
        if (manager) {
            return {
                charging: manager.charging,
                level: manager.level,
                chargingTime: manager.chargingTime,
                dischargingTime: manager.dischargingTime
            };
        }
        return {
            charging: true,
            level: 1,
            chargingTime: 0,
            dischargingTime: Infinity
        }; // Mock full battery if API not available
    }

    onBatteryStateChange(callback: (state: BatteryState) => void): () => void {
        const handler = async () => {
            const state = await this.getBatteryState();
            callback(state);
        };
        
        this.getBatteryManager().then(manager => {
            if (manager) {
                manager.addEventListener('chargingchange', handler);
                manager.addEventListener('levelchange', handler);
                manager.addEventListener('chargingtimechange', handler);
                manager.addEventListener('dischargingtimechange', handler);
            }
        });

        return () => {
            if (this.batteryManager) {
                this.batteryManager.removeEventListener('chargingchange', handler);
                this.batteryManager.removeEventListener('levelchange', handler);
                this.batteryManager.removeEventListener('chargingtimechange', handler);
                this.batteryManager.removeEventListener('dischargingtimechange', handler);
            }
        };
    }

    async suspend(): Promise<void> {
        console.warn('Suspend not supported in browser environment');
    }

    async shutdown(): Promise<void> {
        console.warn('Shutdown not supported in browser environment');
    }

    async reboot(): Promise<void> {
        console.warn('Reboot not supported in browser environment');
        window.location.reload();
    }
}
