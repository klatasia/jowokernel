import type { IPowerProvider, BatteryState } from '../../api/IPowerProvider';
import type { IKernelTransport } from '../../transport/IKernelTransport';

export class KernelPowerProvider implements IPowerProvider {
    constructor(private transport: IKernelTransport) {}

    async getBatteryState(): Promise<BatteryState> {
        return await this.transport.request<BatteryState>('power:getBatteryState', {});
    }

    onBatteryStateChange(callback: (state: BatteryState) => void): () => void {
        const subId = this.transport.subscribe('power:batteryStateChange', callback);
        return () => this.transport.unsubscribe('power:batteryStateChange', subId);
    }

    async suspend(): Promise<void> {
        await this.transport.send('power:suspend', {});
    }

    async shutdown(): Promise<void> {
        await this.transport.send('power:shutdown', {});
    }

    async reboot(): Promise<void> {
        await this.transport.send('power:reboot', {});
    }
}
