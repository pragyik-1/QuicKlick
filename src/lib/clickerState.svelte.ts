import { invoke } from '@tauri-apps/api/core'
import { toast } from '@hermitk/bluenite'
import { listen } from '@tauri-apps/api/event'

export interface ClickTarget {
  device: 'Mouse' | 'Keyboard'
  button: 'Left' | 'Right' | 'Middle' | null
  keyCode: string | null
  mousePosition: [number, number] | null
  clickType: 'Single' | 'Double' | 'Randomized'
  randomizeAmount?: number
}

export interface SeqTargetPayload {
  target: {
    device: string
    button: string | null
    key_code: string | null
    mouse_position: [number, number] | null
    click_type: string
    randomize_amount: number | null
  }
  wait_time: number | null
}

interface BackendAppState {
  repeat_sequence: undefined
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
  mode?: number
  sequence?: SeqTargetPayload[]
}

export interface SavedState {
  repeat_sequence: boolean
  cps: number
  click_limit: number
  is_limited: boolean
  target: {
    device: string
    button: string | null
    key_code: string | null
    mouse_position: [number, number] | null
    click_type: string
    randomize_amount: number | null
  }
  mode?: number
  sequence?: SeqTargetPayload[]
}

export interface AppSettings {
  persist_app_state: boolean
  saved_state: SavedState | null
  presets: Record<string, SavedState>
  shortcuts: Record<string, any>
}

class ClickerState {
  isRunning = $state(false)
  isLoaded = $state(false)
  settings = $state<AppSettings>({
    persist_app_state: true,
    saved_state: null,
    presets: {},
    shortcuts: {},
  })

  private _cps = $state('10')
  private _device = $state<'Mouse' | 'Keyboard'>('Mouse')
  private _button = $state<'Left' | 'Right' | 'Middle'>('Left')
  private _keyCode = $state<string>('')
  private _useCustomPos = $state(false)
  private _posX = $state('0')
  private _posY = $state('0')
  private _clickType = $state<'Single' | 'Double' | 'Randomized'>('Single')
  private _randomizeAmount = $state('10')
  private _isLimited = $state(false)
  private _numClicks = $state('0')
  private _clickLimit = $state('100')
  private _mode = $state(0)
  private _sequence = $state<SeqTargetPayload[]>([])
  private _repeatSequence = $state(true)

  private _isApplyingPreset = false
  private _activePreset = $state('')

  get activePreset() {
    return this._activePreset
  }
  set activePreset(val: string) {
    this._activePreset = val
    if (val && !this._isApplyingPreset) {
      this.loadPreset(val).catch(console.error)
    }
  }

  private clearActivePreset() {
    if (!this._isApplyingPreset) this._activePreset = ''
  }

  get cps() {
    return this._cps
  }
  set cps(val: string) {
    this._cps = val
    this.clearActivePreset()
    const n = parseFloat(val)
    if (!isNaN(n) && n > 0) this.invoke('set_cps_cmd', { cps: n })
  }

  get device() {
    return this._device
  }
  set device(val: 'Mouse' | 'Keyboard') {
    this._device = val
    this.clearActivePreset()
    this.applyTarget()
  }

  get button() {
    return this._button
  }
  set button(val: 'Left' | 'Right' | 'Middle') {
    this._button = val
    this.clearActivePreset()
    this.applyTarget()
  }

  get keyCode() {
    return this._keyCode
  }
  set keyCode(val: string) {
    this._keyCode = val
    this.clearActivePreset()
    this.applyTarget()
  }

  get useCustomPos() {
    return this._useCustomPos
  }
  set useCustomPos(val: boolean) {
    this._useCustomPos = val
    this.clearActivePreset()
    this.applyTarget()
  }

  get posX() {
    return this._posX
  }
  set posX(val: string) {
    this._posX = val
    this.clearActivePreset()
    this.applyTarget()
  }

  get posY() {
    return this._posY
  }
  set posY(val: string) {
    this._posY = val
    this.clearActivePreset()
    this.applyTarget()
  }

  get clickType() {
    return this._clickType
  }
  set clickType(val: 'Single' | 'Double' | 'Randomized') {
    this._clickType = val
    this.clearActivePreset()
    this.applyTarget()
  }

  get randomizeAmount() {
    return this._randomizeAmount
  }
  set randomizeAmount(val: string) {
    this._randomizeAmount = val
    this.clearActivePreset()
    const n = parseFloat(val)
    if (!isNaN(n) && n >= 0) this.applyTarget()
  }

  get isLimited() {
    return this._isLimited
  }
  set isLimited(val: boolean) {
    this._isLimited = val
    this.clearActivePreset()
    this.invoke('set_is_limited_cmd', { isLimited: val })
  }

  get numClicks() {
    return this._numClicks
  }
  set numClicks(val: string) {
    this._numClicks = val
    const n = parseInt(val)
    if (!isNaN(n) && n >= 0) this.invoke('set_num_clicks_cmd', { numClicks: n })
  }

  get clickLimit() {
    return this._clickLimit
  }
  set clickLimit(val: string) {
    this._clickLimit = val
    this.clearActivePreset()
    const n = parseInt(val)
    if (!isNaN(n) && n >= 0) this.invoke('set_click_limit_cmd', { limit: n })
  }

  get mode() {
    return this._mode
  }
  set mode(val: number) {
    this._mode = val
    this.clearActivePreset()
    this.invoke('set_mode_cmd', { mode: val })
  }

  get sequence() {
    return this._sequence
  }
  set sequence(val: SeqTargetPayload[]) {
    this._sequence = val
    this.clearActivePreset()
    this.invoke('set_sequence_cmd', { sequence: val })
  }

  get repeatSequence() {
    return this._repeatSequence
  }
  set repeatSequence(val: boolean) {
    this._repeatSequence = val
    this.clearActivePreset()
    this.invoke('set_repeat_sequence_cmd', { repeat: val })
  }

  constructor() {
    if (typeof window !== 'undefined') this.init()
  }

  private async init() {
    try {
      this.settings = await invoke<AppSettings>('get_settings_cmd')

      const state = await invoke<BackendAppState>('get_app_state_cmd')
      this.updateFromBackend(state)
      this.isLoaded = true

      await listen<BackendAppState>('state_change', (e) => this.updateFromBackend(e.payload))
      await listen<unknown>('error', (e) => {
        if (typeof e.payload === 'string') {
          toast.show({ message: e.payload, variant: 'danger' })
        } else {
          console.error('Clicker error:', e.payload)
        }
      })
    } catch (err) {
      console.error('Failed to initialize clicker state:', err)
    }
  }

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
    if (!this.isRunning && this._mode === 1 && this._sequence.length === 0) {
      toast.show({ message: 'Add steps to the sequence before starting', variant: 'warn' })
      return false
    }
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
      await this.syncSettings()
    } catch (err) {
      console.error('Failed to update shortcut:', err)
      throw err
    } finally {
      this.sync()
    }
  }

  async syncSettings() {
    this.settings = await invoke<AppSettings>('get_settings_cmd')
  }

  async setPersistAppState(val: boolean) {
    this.settings.persist_app_state = val
    await invoke('set_persist_app_state_cmd', { persist: val })
  }

  async savePreset(name: string) {
    const currentState: SavedState = {
      cps: parseFloat(this._cps) || 10,
      click_limit: parseInt(this._clickLimit) || 100,
      is_limited: this._isLimited,
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
      mode: this._mode,
      sequence: [...this._sequence],
      repeat_sequence: this._repeatSequence,
    }
    await invoke('save_preset_cmd', { name, state: currentState })
    await this.syncSettings()
  }

  async loadPreset(name: string) {
    const preset = this.settings.presets[name]
    if (preset) {
      this._isApplyingPreset = true
      try {
        this.cps = preset.cps.toString()
        this.clickLimit = preset.click_limit.toString()
        this.isLimited = preset.is_limited
        this.device = preset.target.device as 'Mouse' | 'Keyboard'
        this.button = (preset.target.button as 'Left' | 'Right' | 'Middle') ?? 'Left'
        this.keyCode = preset.target.key_code ?? ''
        this.useCustomPos = preset.target.mouse_position !== null
        if (preset.target.mouse_position) {
          this.posX = preset.target.mouse_position[0].toString()
          this.posY = preset.target.mouse_position[1].toString()
        }
        this.clickType =
          (preset.target.click_type as 'Single' | 'Double' | 'Randomized') ?? 'Single'
        this.randomizeAmount = (preset.target.randomize_amount ?? 0).toString()
        if (preset.mode !== undefined) {
          this.mode = preset.mode
        }
        if (preset.sequence !== undefined) {
          this.sequence = [...preset.sequence]
        }
        if (preset.repeat_sequence !== undefined) {
          this.repeatSequence = preset.repeat_sequence
        }
        this._activePreset = name
        toast.show({ message: 'Preset loaded', variant: 'success' })
      } catch (err) {
        toast.show({ message: `Failed to load preset: ${err}`, variant: 'danger' })
      } finally {
        this._isApplyingPreset = false
      }
    }
  }

  async deletePreset(name: string) {
    await invoke('delete_preset_cmd', { name })
    await this.syncSettings()
  }

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

  private updateFromBackend(state: BackendAppState) {
    if (!state) return

    this.isRunning = state.is_running ?? this.isRunning
    this._isLimited = state.is_limited ?? this._isLimited
    this._numClicks = state.num_clicks?.toString() ?? this._numClicks
    if (state.mode !== undefined) this._mode = state.mode
    if (state.sequence !== undefined) this._sequence = state.sequence
    if (state.repeat_sequence !== undefined) this._repeatSequence = state.repeat_sequence

    if (typeof state.cps === 'number') {
      const current = parseFloat(this._cps)
      // Only reflect the backend value when the user isn't mid-edit (the field
      // is empty/invalid while typing), otherwise clearing the input would be
      // immediately undone by the sync after every command.
      if (!isNaN(current) && Math.abs(current - state.cps) > 0.001) {
        this._cps = state.cps.toString()
      }
    }
    if (typeof state.click_limit === 'number') {
      const current = parseInt(this._clickLimit)
      if (!isNaN(current) && current !== state.click_limit) {
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
        const curX = parseInt(this._posX)
        if (!isNaN(curX) && curX !== t.mouse_position[0]) this._posX = String(t.mouse_position[0])
        const curY = parseInt(this._posY)
        if (!isNaN(curY) && curY !== t.mouse_position[1]) this._posY = String(t.mouse_position[1])
      }
      this._clickType = (t.click_type as 'Single' | 'Double' | 'Randomized') ?? 'Single'
      const curRandomize = parseInt(this._randomizeAmount)
      const backendRandomize = t.randomize_amount ?? 0
      if (!isNaN(curRandomize) && curRandomize !== backendRandomize) {
        this._randomizeAmount = backendRandomize.toString()
      }
    }
  }
}

export const clickerState = new ClickerState()
