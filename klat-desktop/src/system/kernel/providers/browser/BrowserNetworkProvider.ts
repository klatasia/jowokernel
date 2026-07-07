import type { INetworkProvider, NetworkState } from '../../api/INetworkProvider';

export class BrowserNetworkProvider implements INetworkProvider {
    async getState(): Promise<NetworkState> {
        const connection = (navigator as any).connection;
        return {
            online: navigator.onLine,
            type: connection ? connection.type : 'unknown'
        };
    }

    onStateChange(callback: (state: NetworkState) => void): () => void {
        const handler = async () => {
            const state = await this.getState();
            callback(state);
        };
        
        window.addEventListener('online', handler);
        window.addEventListener('offline', handler);
        const connection = (navigator as any).connection;
        if (connection) {
            connection.addEventListener('change', handler);
        }

        return () => {
            window.removeEventListener('online', handler);
            window.removeEventListener('offline', handler);
            if (connection) {
                connection.removeEventListener('change', handler);
            }
        };
    }
}
