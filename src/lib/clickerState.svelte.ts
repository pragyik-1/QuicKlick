import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ClickTarget {
  device: 'Mouse' | 'Keyboard';
  button: 'Left' | 'Right' | 'Middle' | null;
  key_code: string | null;
  mouse_position: [number, number] | null;
}

export interface AppState {
  is_running: boolean;
  delay: number;
  target: ClickTarget;
}

class ClickerState {
  is_running = $state(false);
  delay = $state(100);
  target = $state<ClickTarget>({
    device: 'Mouse',
    button: 'Left',
    key_code: null,
    mouse_position: null
  });

  is_loaded = $state(false);

  constructor() {
    if (typeof window !== 'undefined') {
      this.init();
    }
  }

  async init() {
    try {
      const state = await invoke<AppState>('get_app_state_cmd');
      this.updateFromPayload(state);
      this.is_loaded = true;

      await listen<AppState>('state_change', (event) => {
        this.updateFromPayload(event.payload);
      });

      await listen<unknown>('error', (event) => {
        console.error('Clicker error:', event.payload);
      });
    } catch (err) {
      console.error('Failed to initialize clicker state:', err);
    }
  }

  updateFromPayload(state: AppState) {
    if (!state) return;

    if (typeof state.is_running === 'boolean') {
      this.is_running = state.is_running;
    }
    if (typeof state.delay === 'number') {
      this.delay = state.delay;
    }

    if (state.target) {
      this.target.device = state.target.device;
      this.target.button = state.target.button;
      this.target.key_code = state.target.key_code;
      this.target.mouse_position = state.target.mouse_position;
    }
  }

  async toggle() {
    try {
      const running = await invoke<boolean>('toggle_clicker_cmd');
      this.is_running = running;
      return running;
    } catch (err) {
      console.error('Failed to toggle clicker:', err);
      return this.is_running;
    }
  }

  async setDelay(newDelay: number) {
    try {
      await invoke('set_delay_cmd', { delay: newDelay });
      this.delay = newDelay;
    } catch (err) {
      console.error('Failed to set delay:', err);
    }
  }

  async setTarget(newTarget: Partial<ClickTarget>) {
    try {
      const payload = {
        device: newTarget.device ?? this.target.device,
        button: newTarget.button !== undefined ? newTarget.button : this.target.button,
        key_code: newTarget.key_code !== undefined ? newTarget.key_code : this.target.key_code,
        mouse_position: newTarget.mouse_position !== undefined ? newTarget.mouse_position : this.target.mouse_position
      };
      await invoke('set_target_cmd', { target: payload });
      this.target.device = payload.device;
      this.target.button = payload.button;
      this.target.key_code = payload.key_code;
      this.target.mouse_position = payload.mouse_position;
    } catch (err) {
      console.error('Failed to set target:', err);
    }
  }

  async updateShortcut(id: string, newKey: string, modifiers: string[]) {
    try {
      await invoke('update_shortcut_cmd', { id, newKey, modifiers });
    } catch (err) {
      console.error('Failed to update shortcut:', err);
      throw err;
    }
  }
}

export const clickerState = new ClickerState();
