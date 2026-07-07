import type { INetworkProvider, NetworkState } from '../../api/INetworkProvider';
import type { IKernelTransport } from '../../transport/IKernelTransport';

export class KernelNetworkProvider implements INetworkProvider {
    constructor(private transport: IKernelTransport) {}

    async getState(): Promise<NetworkState> {
        return await this.transport.request<NetworkState>('network:getState', {});
    }

    onStateChange(callback: (state: NetworkState) => void): () => void {
        const subId = this.transport.subscribe('network:stateChange', callback);
        return () => this.transport.unsubscribe('network:stateChange', subId);
    }
}
