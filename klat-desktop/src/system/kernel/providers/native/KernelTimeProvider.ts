import type { ITimeProvider } from '../../api/ITimeProvider';
import type { IKernelTransport } from '../../transport/IKernelTransport';

export class KernelTimeProvider implements ITimeProvider {
    constructor(private transport: IKernelTransport) {}

    async now(): Promise<number> {
        return await this.transport.request<number>('time:now', {});
    }

    async getTimezoneOffset(): Promise<number> {
        return await this.transport.request<number>('time:getTimezoneOffset', {});
    }
}
