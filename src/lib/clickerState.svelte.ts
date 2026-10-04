import { invoke } from '@tauri-apps/api/core'
import { toast } from '@hermitk/bluenite'
import { listen } from '@tauri-apps/api/event'
import { CPS_MAX, CPS_MIN } from '../constants'

export type ClickTypeValue = 'Single' | 'Double' | 'Randomized' | 'Hold'

export interface ClickTarget {
  device: 'Mouse' | 'Keyboard'
  button: 'Left' | 'Right' | 'Middle' | null
  keyCode: string | null
  mousePosition: [number, number] | null
  clickType: ClickTypeValue
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

export interface ShortcutEvent {
  key: string
  modifiers: string[]
}

export interface AppSettings {
  persist_app_state: boolean
  saved_state: SavedState | null
  presets: Record<string, SavedState>
  shortcuts: Record<string, ShortcutEvent>
  use_evdev_shortcuts: boolean
  /** Preset name assigned to each preset shortcut slot, keyed by slot id. Optional
   * because a settings.json written before preset slots existed has no such key. */
  preset_slots?: Record<string, string>
}

/** Settings as the app holds them: `preset_slots` is always present, because a
 * settings.json from before preset slots existed omits the key entirely. */
type ResolvedAppSettings = AppSettings & { preset_slots: Record<string, string> }

function withPresetSlots(raw: AppSettings): ResolvedAppSettings {
  return { ...raw, preset_slots: raw.preset_slots ?? {} }
}

class ClickerState {
  isRunning = $state(false)
  isLoaded = $state(false)
  settings = $state<ResolvedAppSettings>({
    persist_app_state: true,
    saved_state: null,
    presets: {},
    shortcuts: {},
    use_evdev_shortcuts: true,
    preset_slots: {},
  })

  private _cps = $state('10')
  private _device = $state<'Mouse' | 'Keyboard'>('Mouse')
  private _button = $state<'Left' | 'Right' | 'Middle'>('Left')
  private _keyCode = $state<string>('')
  private _useCustomPos = $state(false)
  private _posX = $state('0')
  private _posY = $state('0')
  private _clickType = $state<ClickTypeValue>('Single')
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

  private updateState(fn: () => void) {
    fn()
    this.clearActivePreset()
  }

  private updateTarget(fn: () => void) {
    this.updateState(fn)
    this.applyTarget().catch((err) => toast.show({ message: `Failed to apply target: ${err}`, variant: 'danger' }))
  }

  get cps() {
    return this._cps
  }
  set cps(val: string) {
    this._cps = val
    this.clearActivePreset()
    const n = parseFloat(val)
    if (isNaN(n)) return
    // Zero is a real setting only with hold so 0 is accpted
    if (n < CPS_MIN || n > CPS_MAX) {
      toast.show({
        message: `Clicks per second must be between ${CPS_MIN} and ${CPS_MAX}`,
        variant: 'danger',
      })
      return
    }
    this.invoke('set_cps_cmd', { cps: n })
  }

  get device() {
    return this._device
  }
  set device(val: 'Mouse' | 'Keyboard') {
    this.updateTarget(() => this._device = val)
  }

  get button() {
    return this._button
  }
  set button(val: 'Left' | 'Right' | 'Middle') {
    this.updateTarget(() => this._button = val)
  }

  get keyCode() {
    return this._keyCode
  }
  set keyCode(val: string) {
    this.updateTarget(() => this._keyCode = val)
  }

  get useCustomPos() {
    return this._useCustomPos
  }
  set useCustomPos(val: boolean) {
    this.updateTarget(() => this._useCustomPos = val)
  }

  get posX() {
    return this._posX
  }
  set posX(val: string) {
    this.updateTarget(() => this._posX = val)
  }

  get posY() {
    return this._posY
  }
  set posY(val: string) {
    this.updateState(() => this._posY = val)
  }

  get clickType() {
    return this._clickType
  }
  set clickType(val: ClickTypeValue) {
    this.updateTarget(() => this._clickType = val)
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
    this.updateState(() => {
      this._mode = val
      this.invoke('set_mode_cmd', { mode: val })
    })
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
      this.settings = withPresetSlots(await invoke<AppSettings>('get_settings_cmd'))

      const state = await invoke<BackendAppState>('get_app_state_cmd')
      this.updateFromBackend(state)
      this.isLoaded = true

      await listen<BackendAppState>('state_change', (e) => this.updateFromBackend(e.payload))
      await listen<string>('preset_triggered', (e) => this.loadPreset(e.payload))
      await listen<unknown>('error', (e) => {
        if (typeof e.payload === 'string') {
          toast.show({ message: e.payload, variant: 'danger' })
        } else {
          console.error('Clicker error:', e.payload)
        }
      })
    } catch (err) {
      toast.show({ message: `Failed to initialize clicker state: ${err}`, variant: 'danger' })
    }
  }

  private async invoke<T>(cmd: string, args?: Record<string, unknown>, showToast: boolean = true): Promise<T> {
    try {
      const res = await invoke<T>(cmd, args)
      return res
    } catch (err) {
      if (showToast) {
        toast.show({ message: `Failed: ${cmd}: ${err}`, variant: 'danger' })
      }
      throw err
    } finally {
      this.sync()
    }
  }

  async isWayland(): Promise<boolean> {
    return await this.invoke<boolean>('is_wayland_cmd').catch((err) => {
      toast.show({ message: `Failed to check Wayland: ${err}`, variant: 'danger' })
      return false
    })
  }

  async isLinux() {
    return await this.invoke<boolean>('is_linux_cmd').catch((err) => {
      toast.show({ message: `Failed to check Linux: ${err}`, variant: 'danger' })
      return false
    })
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
      const running = await this.invoke<boolean>('toggle_clicker_cmd')
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
      await this.invoke('update_shortcut_cmd', { id, newKey, modifiers }, false)
      await this.syncSettings()
    } catch (err) {
      console.error('Failed to update shortcut:', err)
      throw err
    } finally {
      this.sync()
    }
  }

  /** Leaves an action with no binding. The backend drops the entry entirely, so
   * `settings.shortcuts[id]` is undefined afterwards and reads as "None". */
  async clearShortcut(id: string) {
    try {
      await this.invoke('clear_shortcut_cmd', { id }, false)
      await this.syncSettings()
    } catch (err) {
      toast.show({ message: `Failed to clear shortcut: ${err}`, variant: 'danger' })
      throw err
    }
  }

  /** Points a preset shortcut slot at a saved preset, or clears the slot when
   * `name` is empty. */
  async setPresetSlot(id: string, name: string) {
    try {
      await this.invoke('set_preset_slot_cmd', { slot: id, name }, false)
      await this.syncSettings()
    } catch (err) {
      toast.show({ message: `Failed to assign preset: ${err}`, variant: 'danger' })
      throw err
    }
  }

  async syncSettings() {
    this.settings = withPresetSlots(await this.invoke<AppSettings>('get_settings_cmd'))
  }

  async setPersistAppState(val: boolean) {
    this.settings.persist_app_state = val
    await this.invoke('set_persist_app_state_cmd', { persist: val })
  }

  async setUseEvdevShortcuts(val: boolean) {
    this.settings.use_evdev_shortcuts = val
    await this.invoke('set_use_evdev_shortcuts_cmd', { enabled: val })
  }

  async savePreset(name: string) {
    // A CPS of zero is a real setting (hold until stopped), so only an empty or
    // unparsable field falls back to the default.
    const cps = parseFloat(this._cps)
    const currentState: SavedState = {
      cps: isNaN(cps) ? 10 : cps,
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
    await this.invoke('save_preset_cmd', { name, state: currentState })
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
        this.clickType = (preset.target.click_type as ClickTypeValue) ?? 'Single'
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
    try {
      await this.invoke('delete_preset_cmd', { name }, false)
      await this.syncSettings()
    } catch (err) {
      toast.show({ message: `Failed to delete preset: ${err}`, variant: 'danger' })
    }
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
      this._clickType = (t.click_type as ClickTypeValue) ?? 'Single'
      const curRandomize = parseInt(this._randomizeAmount)
      const backendRandomize = t.randomize_amount ?? 0
      if (!isNaN(curRandomize) && curRandomize !== backendRandomize) {
        this._randomizeAmount = backendRandomize.toString()
      }
    }
  }
}

export const clickerState = new ClickerState()
