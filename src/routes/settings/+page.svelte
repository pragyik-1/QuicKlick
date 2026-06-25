<script lang="ts">
  import { Card, Input, Button, Switch, Col, Row } from 'bluenite'
  import { clickerState } from '$lib/clickerState.svelte'
  import { toast } from 'bluenite'
  import HotkeyInput from '$lib/HotkeyInput.svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import { TriangleAlert, Save, FolderOpen, Trash2 } from 'lucide-svelte'

  import './page.css'

  const SHORTCUTS = [
    {
      id: 'toggle-clicker',
      name: 'Toggle Clicker',
      desc: 'Start or stop the autoclicker',
      defaultKey: 'F6',
    },
    {
      id: 'start-clicker',
      name: 'Start Clicker',
      desc: 'Force start the autoclicker',
      defaultKey: 'Shift+F6',
    },
    {
      id: 'stop-clicker',
      name: 'Stop Clicker',
      desc: 'Force stop the autoclicker',
      defaultKey: 'Ctrl+F6',
    },
  ]

  let bindings = $state<Record<string, string>>(
    Object.fromEntries(SHORTCUTS.map((s) => [s.id, s.defaultKey])),
  )

  let isWayland = $state(false)
  let newPresetName = $state('')

  onMount(async () => {
    try {
      isWayland = await invoke<boolean>('is_wayland_cmd')
      await clickerState.syncSettings()
      for (const [id, event] of Object.entries(clickerState.settings.shortcuts)) {
        bindings[id] = formatDisplay(event.key, event.modifiers)
      }
    } catch (err) {
      console.error('Failed to check wayland status', err)
    }
  })

  async function handleSavePreset() {
    if (!newPresetName.trim()) return
    try {
      await clickerState.savePreset(newPresetName)
      toast.show({ message: 'Preset saved', variant: 'success' })
      newPresetName = ''
    } catch (e) {
      toast.show({ message: `Failed to save preset: ${e}`, variant: 'danger' })
    }
  }

  async function handleLoadPreset(name: string) {
    try {
      await clickerState.loadPreset(name)
      toast.show({ message: 'Preset loaded', variant: 'success' })
    } catch (e) {
      toast.show({ message: `Failed to load preset: ${e}`, variant: 'danger' })
    }
  }

  async function handleDeletePreset(name: string) {
    try {
      await clickerState.deletePreset(name)
      toast.show({ message: 'Preset deleted', variant: 'success' })
    } catch (e) {
      toast.show({ message: `Failed to delete preset: ${e}`, variant: 'danger' })
    }
  }

  function formatDisplay(key: string, modifiers: string[]): string {
    const parts = modifiers.map((m) => m.charAt(0).toUpperCase() + m.slice(1))
    parts.push(key.length === 1 ? key.toUpperCase() : key.charAt(0).toUpperCase() + key.slice(1))
    return parts.join('+')
  }

  async function onCapture(id: string, event: { key: string; modifiers: string[] }) {
    try {
      await clickerState.updateShortcut(id, event.key, event.modifiers)
      bindings[id] = formatDisplay(event.key, event.modifiers)
      toast.show({
        message: 'Shortcut updated',
        variant: 'success',
        duration: 2000,
      })
    } catch (err) {
      toast.show({
        message: `Failed to update shortcut: ${err}`,
        variant: 'danger',
      })
    }
  }
</script>

<Card style="margin-bottom: 1.5rem;" title="Shortcuts">
  {#if isWayland}
    <div style="padding: 1rem; text-align: center; color: var(--muted-text);">
      <TriangleAlert
        size={48}
        style="margin: 0 auto 1rem auto; display: block; color: var(--warn);"
      />
      <p
        style="margin-bottom: 0.5rem; color: var(--primary-text); font-weight: 600; font-size: 1.1rem;"
      >
        Wayland Detected
      </p>
      <p style="margin: 0; line-height: 1.5;">
        Changing global shortcuts for individual apps is not supported in Wayland. Please use your
        native system settings (e.g. GNOME Settings, KDE System Settings) to configure global
        shortcuts.
      </p>
    </div>
  {:else}
    <div class="shortcut-list">
      {#each SHORTCUTS as shortcut}
        <div class="shortcut-row">
          <div class="shortcut-info">
            <span class="shortcut-name">{shortcut.name}</span>
            <span class="shortcut-desc">{shortcut.desc}</span>
          </div>
          <div class="shortcut-input">
            <HotkeyInput
              value={bindings[shortcut.id]}
              onCapture={(e) => onCapture(shortcut.id, e)}
            />
          </div>
        </div>
      {/each}
    </div>
  {/if}
</Card>

<Card style="margin-bottom: 1.5rem;" title="App State Persistence">
  <div style="margin-bottom: 1rem; color: var(--muted-text); font-size: 0.85rem; line-height: 1.4;">
    When enabled, your clicker configuration (CPS, limits, target) is saved when changed and
    restored on startup.
  </div>
  <Switch
    label="Persist App State"
    checked={clickerState.settings.persist_app_state}
    onchange={(e) => clickerState.setPersistAppState(e.currentTarget.checked)}
  />
</Card>

<Card style="margin-bottom: 1.5rem;" title="Presets">
  <div style="margin-bottom: 1rem;">
    <div style="display: flex; gap: 0.5rem; align-items: stretch;">
      <Input
        bind:value={newPresetName}
        placeholder="Preset Name"
        style="flex: 1; color: var(--primary-text);"
      />
      <Button variant="fill" onclick={handleSavePreset}>
        <Save size={16} style="margin-right: 0.4rem; display: inline-block;" /> Save
      </Button>
    </div>
  </div>
  <div>
    {#each Object.keys(clickerState.settings.presets) as presetName}
      <Row
        style="display: flex; justify-content: space-between; align-items: center; padding: 0.75rem; background: var(--input); border-radius: var(--round-md); margin-bottom: 0.5rem;"
      >
        <span style="color: var(--primary-text); font-weight: 500;">{presetName}</span>
        <div style="display: flex; gap: 0.5rem;">
          <Button variant="outline" onclick={() => handleLoadPreset(presetName)}>
            <FolderOpen size={16} />
          </Button>
          <Button variant="outline" onclick={() => handleDeletePreset(presetName)}>
            <Trash2 size={16} color="var(--danger)" />
          </Button>
        </div>
      </Row>
    {:else}
      <div style="text-align: center; color: var(--muted-text); padding: 1rem; font-size: 0.9rem;">
        No presets saved yet. Create a preset to quickly load configurations.
      </div>
    {/each}
  </div>
</Card>
