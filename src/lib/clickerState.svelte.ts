import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export interface ClickTarget {
  device: 'Mouse' | 'Keyboard';
  button: 'Left' | 'Right' | 'Middle' | null;
  keyCode: string | null;
  mousePosition: [number, number] | null;
  clickType: 'Single' | 'Double' | 'Randomized';
  randomizeAmount?: number;
}

interface BackendAppState {
  is_running: boolean;
  cps: number;
  target: {
    device: string;
    button: string | null;
    key_code: string | null;
    mouse_position: [number, number] | null;
    click_type: string;
    randomize_amount: number | null;
  };
}

class ClickerState {
  isRunning = $state(false);
  isLoaded = $state(false);

  private _cps = $state("10");
  private _device = $state<"Mouse" | "Keyboard">("Mouse");
  private _button = $state<"Left" | "Right" | "Middle">("Left");
  private _keyCode = $state<string>("");
  private _useCustomPos = $state<boolean>(false);
  private _posX = $state<string>("0");
  private _posY = $state<string>("0");
  private _clickType = $state<"Single" | "Double" | "Randomized">("Single");
  private _randomizeAmount = $state<string>("0");

  get cps() { return this._cps; }
  set cps(val: string) {
    this._cps = val;
    const cpsNum = parseFloat(val);
    if (!isNaN(cpsNum) && cpsNum > 0) {
      this.setCps(cpsNum);
    }
  }

  get device() { return this._device; }
  set device(val: "Mouse" | "Keyboard") { this._device = val; this.applyTarget(); }

  get button() { return this._button; }
  set button(val: "Left" | "Right" | "Middle") { this._button = val; this.applyTarget(); }

  get keyCode() { return this._keyCode; }
  set keyCode(val: string) { this._keyCode = val; this.applyTarget(); }

  get useCustomPos() { return this._useCustomPos; }
  set useCustomPos(val: boolean) { this._useCustomPos = val; this.applyTarget(); }

  get posX() { return this._posX; }
  set posX(val: string) { this._posX = val; this.applyTarget(); }

  get posY() { return this._posY; }
  set posY(val: string) { this._posY = val; this.applyTarget(); }

  get clickType() { return this._clickType; }
  set clickType(val: "Single" | "Double" | "Randomized") { this._clickType = val; this.applyTarget(); }

  get randomizeAmount() { return this._randomizeAmount; }
  set randomizeAmount(val: string) {
    this._randomizeAmount = val; const randNum = parseFloat(val);
    if (!isNaN(randNum) && randNum >= 0) {
      this.applyTarget();
    }
  }

  constructor() {
    if (typeof window !== 'undefined') {
      this.init();
    }
  }

  async init() {
    try {
      const state = await invoke<BackendAppState>('get_app_state_cmd');
      this.updateFromPayload(state);
      this.isLoaded = true;

      await listen<BackendAppState>('state_change', (event) => {
        this.updateFromPayload(event.payload);
      });

      await listen<unknown>('error', (event) => {
        console.error('Clicker error:', event.payload);
      });
    } catch (err) {
      console.error('Failed to initialize clicker state:', err);
    }
  }

  async sync() {
    const state = await invoke<BackendAppState>('get_app_state_cmd');
    this.updateFromPayload(state);
  }

  updateFromPayload(state: BackendAppState) {
    if (!state) return;

    if (typeof state.is_running === 'boolean') {
      this.isRunning = state.is_running;
    }
    if (typeof state.cps === 'number') {
      const currentCpsNum = parseFloat(this._cps);
      if (isNaN(currentCpsNum) || Math.abs(currentCpsNum - state.cps) > 0.001) {
        this._cps = state.cps.toString();
      }
    }

    if (state.target) {
      this._device = state.target.device as "Mouse" | "Keyboard";
      this._button = (state.target.button as "Left" | "Right" | "Middle") ?? "Left";
      this._keyCode = state.target.key_code ?? "";
      this._useCustomPos = state.target.mouse_position !== null;
      if (state.target.mouse_position) {
        if (parseInt(this._posX) !== state.target.mouse_position[0]) {
          this._posX = String(state.target.mouse_position[0]);
        }
        if (parseInt(this._posY) !== state.target.mouse_position[1]) {
          this._posY = String(state.target.mouse_position[1]);
        }
      }
      this._clickType = (state.target.click_type as "Single" | "Double" | "Randomized") ?? "Single";
      this._randomizeAmount = (state.target.randomize_amount ?? 0).toString();
    }
  }

  async applyTarget() {
    try {
      const payload = {
        device: this._device,
        button: this._device === 'Mouse' ? this._button : null,
        key_code: this._device === 'Keyboard' ? this._keyCode : null,
        mouse_position: this._device === 'Mouse' && this._useCustomPos
          ? [parseInt(this._posX) || 0, parseInt(this._posY) || 0]
          : null,
        click_type: this._clickType,
        randomize_amount: this._clickType === 'Randomized' ? (parseInt(this._randomizeAmount) || 0) : null
      };
      await invoke('set_target_cmd', { target: payload });
    } catch (err) {
      console.error('Failed to apply target:', err);
    } finally {
      this.sync();
    }
  }

  async toggle() {
    try {
      const running = await invoke<boolean>('toggle_clicker_cmd');
      this.isRunning = running;
      return running;
    } catch (err) {
      console.error('Failed to toggle clicker:', err);
      return this.isRunning;
    } finally {
      this.sync();
    }
  }

  async setCps(newCps: number) {
    try {
      await invoke('set_cps_cmd', { cps: newCps });
    } catch (err) {
      console.error('Failed to set cps:', err);
    } finally {
      this.sync();
    }
  }

  async updateShortcut(id: string, newKey: string, modifiers: string[]) {
    try {
      await invoke('update_shortcut_cmd', { id, newKey, modifiers });
    } catch (err) {
      console.error('Failed to update shortcut:', err);
      throw err;
    } finally {
      this.sync();
    }
  }
}

export const clickerState = new ClickerState();
