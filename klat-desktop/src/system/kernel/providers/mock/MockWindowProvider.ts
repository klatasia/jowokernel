import type { IWindowProvider, WindowState } from '../../api/IWindowProvider';

export class MockWindowProvider implements IWindowProvider {
    private windows: Map<string, WindowState> = new Map();
    private listeners: Map<string, Array<(state: WindowState) => void>> = new Map();

    private notifyListeners(id: string) {
        const state = this.windows.get(id);
        if (state && this.listeners.has(id)) {
            const cbs = this.listeners.get(id)!;
            cbs.forEach(cb => cb(state));
        }
    }

    async createWindow(options: Partial<WindowState>): Promise<string> {
        const id = 'mock_win_' + Math.random().toString(36).substring(2, 9);
        const state: WindowState = {
            id,
            title: options.title || 'Window',
            x: options.x || 100,
            y: options.y || 100,
            width: options.width || 800,
            height: options.height || 600,
            focused: options.focused ?? true,
            maximized: options.maximized ?? false,
            minimized: options.minimized ?? false
        };
        this.windows.set(id, state);
        console.log(`[MockWindow] Created window ${id}`, state);
        return id;
    }

    async destroyWindow(id: string): Promise<void> {
        this.windows.delete(id);
        this.listeners.delete(id);
        console.log(`[MockWindow] Destroyed window ${id}`);
    }

    async setWindowTitle(id: string, title: string): Promise<void> {
        const win = this.windows.get(id);
        if (win) {
            win.title = title;
            this.notifyListeners(id);
        }
    }

    async setWindowBounds(id: string, x: number, y: number, width: number, height: number): Promise<void> {
        const win = this.windows.get(id);
        if (win) {
            win.x = x;
            win.y = y;
            win.width = width;
            win.height = height;
            this.notifyListeners(id);
        }
    }

    async focusWindow(id: string): Promise<void> {
        const win = this.windows.get(id);
        if (win) {
            win.focused = true;
            this.notifyListeners(id);
        }
    }

    async maximizeWindow(id: string): Promise<void> {
        const win = this.windows.get(id);
        if (win) {
            win.maximized = true;
            this.notifyListeners(id);
        }
    }

    async minimizeWindow(id: string): Promise<void> {
        const win = this.windows.get(id);
        if (win) {
            win.minimized = true;
            this.notifyListeners(id);
        }
    }

    async restoreWindow(id: string): Promise<void> {
        const win = this.windows.get(id);
        if (win) {
            win.maximized = false;
            win.minimized = false;
            this.notifyListeners(id);
        }
    }

    onWindowStateChange(id: string, callback: (state: WindowState) => void): () => void {
        if (!this.listeners.has(id)) {
            this.listeners.set(id, []);
        }
        this.listeners.get(id)!.push(callback);
        
        return () => {
            const arr = this.listeners.get(id);
            if (arr) {
                this.listeners.set(id, arr.filter(cb => cb !== callback));
            }
        };
    }
}
