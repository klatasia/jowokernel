import type { INetworkProvider, NetworkState } from '../../api/INetworkProvider';

export class MockNetworkProvider implements INetworkProvider {
    private state: NetworkState = { online: true, type: 'wifi', ssid: 'MockNetwork' };
    private listeners: Array<(state: NetworkState) => void> = [];

    async getState(): Promise<NetworkState> {
        return this.state;
    }

    onStateChange(callback: (state: NetworkState) => void): () => void {
        this.listeners.push(callback);
        return () => {
            this.listeners = this.listeners.filter(cb => cb !== callback);
        };
    }

    // For testing
    simulateChange(newState: Partial<NetworkState>) {
        this.state = { ...this.state, ...newState };
        this.listeners.forEach(cb => cb(this.state));
    }
}
