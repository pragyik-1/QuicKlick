import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface ClickTarget {
  device: 'Mouse' | 'Keyboard'
  button: 'Left' | 'Right' | 'Middle' | null
  keyCode: string | null
  mousePosition: [number, number] | null
  clickType: 'Single' | 'Double' | 'Randomized'
  randomizeAmount?: number
}

interface BackendAppState {
  is_running: boolean
  cps: number
  is_limited: boolean
  num_clicks: number
  click_limit: number
  target: {
    device: string
    button: string | null
    key_code: string | null
    mouse_position: [number, number] | null
    click_type: string
    randomize_amount: number | null
  }
}

class ClickerState {
  isRunning = $state(false)
  isLoaded = $state(false)

  // ---------------------------------------------------------------------------
  // Reactive fields — setters sync to the backend automatically
  // ---------------------------------------------------------------------------

  private _cps = $state('10')
  private _device = $state<'Mouse' | 'Keyboard'>('Mouse')
  private _button = $state<'Left' | 'Right' | 'Middle'>('Left')
  private _keyCode = $state<string>('')
  private _useCustomPos = $state(false)
  private _posX = $state('0')
  private _posY = $state('0')
  private _clickType = $state<'Single' | 'Double' | 'Randomized'>('Single')
  private _randomizeAmount = $state('0')
  private _isLimited = $state(false)
  private _numClicks = $state('0')
  private _clickLimit = $state('0')

  // --- CPS ---
  get cps() { return this._cps }
  set cps(val: string) {
    this._cps = val
    const n = parseFloat(val)
    if (!isNaN(n) && n > 0) this.invoke('set_cps_cmd', { cps: n })
  }

  // --- Target fields (all trigger applyTarget) ---
  get device() { return this._device }
  set device(val: 'Mouse' | 'Keyboard') { this._device = val; this.applyTarget() }

  get button() { return this._button }
  set button(val: 'Left' | 'Right' | 'Middle') { this._button = val; this.applyTarget() }

  get keyCode() { return this._keyCode }
  set keyCode(val: string) { this._keyCode = val; this.applyTarget() }

  get useCustomPos() { return this._useCustomPos }
  set useCustomPos(val: boolean) { this._useCustomPos = val; this.applyTarget() }

  get posX() { return this._posX }
  set posX(val: string) { this._posX = val; this.applyTarget() }

  get posY() { return this._posY }
  set posY(val: string) { this._posY = val; this.applyTarget() }

  get clickType() { return this._clickType }
  set clickType(val: 'Single' | 'Double' | 'Randomized') { this._clickType = val; this.applyTarget() }

  get randomizeAmount() { return this._randomizeAmount }
  set randomizeAmount(val: string) {
    this._randomizeAmount = val
    const n = parseFloat(val)
    if (!isNaN(n) && n >= 0) this.applyTarget()
  }

  // --- Click limit fields ---
  get isLimited() { return this._isLimited }
  set isLimited(val: boolean) { this._isLimited = val; this.invoke('set_is_limited_cmd', { isLimited: val }) }

  get numClicks() { return this._numClicks }
  set numClicks(val: string) {
    this._numClicks = val
    const n = parseInt(val)
    if (!isNaN(n) && n >= 0) this.invoke('set_num_clicks_cmd', { numClicks: n })
  }

  get clickLimit() { return this._clickLimit }
  set clickLimit(val: string) {
    this._clickLimit = val
    const n = parseInt(val)
    if (!isNaN(n) && n >= 0) this.invoke('set_click_limit_cmd', { limit: n })
  }

  // ---------------------------------------------------------------------------
  // Initialization & event listeners
  // ---------------------------------------------------------------------------

  constructor() {
    if (typeof window !== 'undefined') this.init()
  }

  private async init() {
    try {
      const state = await invoke<BackendAppState>('get_app_state_cmd')
      this.updateFromBackend(state)
      this.isLoaded = true

      await listen<BackendAppState>('state_change', (e) => this.updateFromBackend(e.payload))
      await listen<unknown>('error', (e) => console.error('Clicker error:', e.payload))
    } catch (err) {
      console.error('Failed to initialize clicker state:', err)
    }
  }

  // ---------------------------------------------------------------------------
  // Backend communication helpers
  // ---------------------------------------------------------------------------

  /** Invoke a backend command and resync state afterwards. */
  private async invoke(cmd: string, args?: Record<string, unknown>) {
    try {
      await invoke(cmd, args)
    } catch (err) {
      console.error(`Failed: ${cmd}`, err)
    } finally {
      this.sync()
    }
  }

  async sync() {
    const state = await invoke<BackendAppState>('get_app_state_cmd')
    this.updateFromBackend(state)
  }

  async toggle() {
    try {
      const running = await invoke<boolean>('toggle_clicker_cmd')
      this.isRunning = running
      return running
    } catch (err) {
      console.error('Failed to toggle clicker:', err)
      return this.isRunning
    } finally {
      this.sync()
    }
  }

  async updateShortcut(id: string, newKey: string, modifiers: string[]) {
    try {
      await invoke('update_shortcut_cmd', { id, newKey, modifiers })
    } catch (err) {
      console.error('Failed to update shortcut:', err)
      throw err
    } finally {
      this.sync()
    }
  }

  // ---------------------------------------------------------------------------
  // State synchronization
  // ---------------------------------------------------------------------------

  private async applyTarget() {
    await this.invoke('set_target_cmd', {
      target: {
        device: this._device,
        button: this._device === 'Mouse' ? this._button : null,
        key_code: this._device === 'Keyboard' ? this._keyCode : null,
        mouse_position:
          this._device === 'Mouse' && this._useCustomPos
            ? [parseInt(this._posX) || 0, parseInt(this._posY) || 0]
            : null,
        click_type: this._clickType,
        randomize_amount:
          this._clickType === 'Randomized' ? parseInt(this._randomizeAmount) || 0 : null,
      },
    })
  }

  /** Update local reactive state from a backend payload without triggering backend calls. */
  private updateFromBackend(state: BackendAppState) {
    if (!state) return

    this.isRunning = state.is_running ?? this.isRunning
    this._isLimited = state.is_limited ?? this._isLimited
    this._numClicks = state.num_clicks?.toString() ?? this._numClicks

    // Only overwrite user-editable text fields if the backend value differs
    if (typeof state.cps === 'number') {
      const current = parseFloat(this._cps)
      if (isNaN(current) || Math.abs(current - state.cps) > 0.001) {
        this._cps = state.cps.toString()
      }
    }
    if (typeof state.click_limit === 'number') {
      const current = parseInt(this._clickLimit)
      if (isNaN(current) || current !== state.click_limit) {
        this._clickLimit = state.click_limit.toString()
      }
    }

    if (state.target) {
      const t = state.target
      this._device = t.device as 'Mouse' | 'Keyboard'
      this._button = (t.button as 'Left' | 'Right' | 'Middle') ?? 'Left'
      this._keyCode = t.key_code ?? ''
      this._useCustomPos = t.mouse_position !== null
      if (t.mouse_position) {
        if (parseInt(this._posX) !== t.mouse_position[0]) this._posX = String(t.mouse_position[0])
        if (parseInt(this._posY) !== t.mouse_position[1]) this._posY = String(t.mouse_position[1])
      }
      this._clickType = (t.click_type as 'Single' | 'Double' | 'Randomized') ?? 'Single'
      this._randomizeAmount = (t.randomize_amount ?? 0).toString()
    }
  }
}

export const clickerState = new ClickerState()
