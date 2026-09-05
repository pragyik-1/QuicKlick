<script lang="ts">
  import { Card, Button, Input, Select, Switch, Row } from '@hermitk/bluenite'
  import { clickerState, type SeqTargetPayload } from '$lib/clickerState.svelte'
  import HotkeyInput from '$lib/HotkeyInput.svelte'
  import {
    Play,
    Square,
    Monitor,
    MousePointer2,
    Keyboard,
    Crosshair,
    MousePointerClick,
    TimerReset,
    ListPlus,
    Trash2,
  } from 'lucide-svelte'
  import { DEVICE_OPTIONS, BUTTON_OPTIONS, CLICK_TYPE_OPTIONS } from '../../constants'

  let formDevice = $state<'Mouse' | 'Keyboard'>('Mouse')
  let formButton = $state<'Left' | 'Right' | 'Middle'>('Left')
  let formKeyCode = $state<string>('')
  let formUseCustomPos = $state(false)
  let formPosX = $state('0')
  let formPosY = $state('0')
  let formClickType = $state<'Single' | 'Double' | 'Randomized'>('Single')
  let formRandomizeAmount = $state('10')
  let formWaitTime = $state('1000')

  function onKeyCapture(e: { key: string; modifiers: string[] }) {
    formKeyCode = e.key
  }

  function addTarget() {
    const newTarget: SeqTargetPayload = {
      target: {
        device: formDevice,
        button: formDevice === 'Mouse' ? formButton : null,
        key_code: formDevice === 'Keyboard' ? formKeyCode : null,
        mouse_position:
          formDevice === 'Mouse' && formUseCustomPos
            ? [parseInt(formPosX) || 0, parseInt(formPosY) || 0]
            : null,
        click_type: formClickType,
        randomize_amount:
          formClickType === 'Randomized' ? parseInt(formRandomizeAmount) || 0 : null,
      },
      wait_time: parseInt(formWaitTime) || 1000,
    }

    clickerState.sequence = [...clickerState.sequence, newTarget]
  }

  function removeTarget(index: number) {
    const newSequence = [...clickerState.sequence]
    newSequence.splice(index, 1)
    clickerState.sequence = newSequence
  }

  let isSequenceMode = $derived(clickerState.mode === 1)

  function toggleSequenceMode(enabled: boolean) {
    clickerState.mode = enabled ? 1 : 0
  }
</script>

<div class="sequence-layout">
  <Card style="height: auto;">
    <!-- Status + Mode Toggle -->
    <div class="compact-control" style="margin-bottom: 1rem;">
      <div class="status-chip" class:active={clickerState.isRunning}>
        <span class="status-dot-sm"></span>
        {clickerState.isRunning ? 'Running' : 'Stopped'}
      </div>
      <Row gap={0.5}>
        <Button
          variant={clickerState.isRunning ? 'outline' : 'fill'}
          disabled={clickerState.isRunning}
          onclick={() => clickerState.toggle()}
        >
          <span style="display: flex; align-items: center; gap: 0.3rem;">
            <Play size={14} /> Start
          </span>
        </Button>
        <Button
          variant={clickerState.isRunning ? 'fill' : 'outline'}
          disabled={!clickerState.isRunning}
          onclick={() => clickerState.toggle()}
        >
          <span style="display: flex; align-items: center; gap: 0.3rem;">
            <Square size={14} /> Stop
          </span>
        </Button>
      </Row>
    </div>

    <div style="margin-bottom: 1rem;">
      <Switch
        label="Enable Sequence Mode"
        size="md"
        checked={isSequenceMode}
        onchange={(e) => toggleSequenceMode(e.currentTarget.checked)}
      />
      <div style="font-size: 0.8rem; color: var(--muted-text); margin-top: 0.3rem;">
        When enabled, the clicker will execute the sequence below in order instead of clicking
        repeatedly.
      </div>
    </div>

    {#if isSequenceMode}
      <div style="margin-bottom: 1rem;">
        <Switch label="Repeat Sequence" size="sm" bind:checked={clickerState.repeatSequence} />
        <div style="font-size: 0.8rem; color: var(--muted-text); margin-top: 0.3rem;">
          If enabled, the sequence will loop continuously. Otherwise, it fires only once.
        </div>
      </div>
    {/if}

    <div class="divider"></div>

    <h3 style="margin-bottom: 0.75rem; font-size: 1.1rem;">Sequence Steps</h3>

    {#if clickerState.sequence.length === 0}
      <div
        style="padding: 1.5rem; text-align: center; color: var(--muted-text); background: rgba(255,255,255,0.03); border-radius: var(--round-md);"
      >
        No steps added to sequence yet.
      </div>
    {:else}
      <div class="sequence-list">
        {#each clickerState.sequence as step, index}
          <div class="sequence-item">
            <div class="step-index">{index + 1}</div>
            <div class="step-content">
              <div class="step-title">
                {step.target.device} Click ({step.target.device === 'Mouse'
                  ? step.target.button
                  : step.target.key_code})
              </div>
              <div class="step-details">
                Type: {step.target.click_type} • Wait: {step.wait_time}ms
                {#if step.target.mouse_position}
                  • Pos: {step.target.mouse_position[0]}, {step.target.mouse_position[1]}
                {/if}
              </div>
            </div>
            <Button variant="ghost" onclick={() => removeTarget(index)}>
              <Trash2 size={16} color="var(--danger)" />
            </Button>
          </div>
        {/each}
      </div>
    {/if}

    <div class="divider"></div>

    <h3 style="margin-bottom: 0.75rem; font-size: 1.1rem;">Add New Step</h3>

    <div class="field-grid" style="margin-bottom: 1rem;">
      <div class="field-compact">
        <label class="field-label-sm" for="seq-device">
          {#if formDevice === 'Mouse'}
            <Monitor size={14} />
          {:else}
            <Keyboard size={14} />
          {/if}
          Device
        </label>
        <Select
          id="seq-device"
          style="color: var(--primary-text);"
          options={DEVICE_OPTIONS}
          bind:value={formDevice}
        />
      </div>

      {#if formDevice === 'Mouse'}
        <div class="field-compact">
          <label class="field-label-sm" for="seq-button">
            <MousePointer2 size={14} /> Button
          </label>
          <Select
            id="seq-button"
            style="color: var(--primary-text);"
            options={BUTTON_OPTIONS}
            bind:value={formButton}
          />
        </div>
      {:else}
        <div class="field-compact">
          <span class="field-label-sm">
            <Keyboard size={14} /> Key
          </span>
          <HotkeyInput value={formKeyCode} onCapture={onKeyCapture} placeholder="Click to set..." />
        </div>
      {/if}
    </div>

    {#if formDevice === 'Mouse'}
      <div style="margin-top: 0.75rem; margin-bottom: 1rem;">
        <Switch label="Custom Position" size="sm" bind:checked={formUseCustomPos} />
      </div>

      {#if formUseCustomPos}
        <div class="coord-row" style="margin-bottom: 1rem;">
          <div class="field-compact">
            <label class="field-label-sm" for="seq-x">
              <Crosshair size={14} /> X
            </label>
            <Input
              style="color: var(--primary-text);"
              id="seq-x"
              type="number"
              bind:value={formPosX}
              placeholder="0"
            />
          </div>
          <div class="field-compact">
            <label class="field-label-sm" for="seq-y">
              <Crosshair size={14} /> Y
            </label>
            <Input
              style="color: var(--primary-text);"
              id="seq-y"
              type="number"
              bind:value={formPosY}
              placeholder="0"
            />
          </div>
        </div>
      {/if}
    {/if}

    <div class="field-grid" style="margin-bottom: 1rem;">
      <div class="field-compact">
        <label class="field-label-sm" for="seq-click-type">
          <MousePointerClick size={14} /> Type
        </label>
        <Select
          id="seq-click-type"
          style="color: var(--primary-text);"
          options={CLICK_TYPE_OPTIONS}
          bind:value={formClickType}
        />
      </div>

      <div class="field-compact">
        <label class="field-label-sm" for="seq-wait">
          <TimerReset size={14} /> Wait Time (ms)
        </label>
        <Input
          style="color: var(--primary-text);"
          id="seq-wait"
          type="number"
          bind:value={formWaitTime}
          min="0"
          placeholder="1000"
        />
      </div>
    </div>

    {#if formClickType === 'Randomized'}
      <div class="field-compact" style="margin-bottom: 1rem;">
        <label class="field-label-sm" for="seq-randomize">
          <MousePointerClick size={14} /> Variance (ms)
        </label>
        <Input
          style="color: var(--primary-text);"
          id="seq-randomize"
          type="number"
          bind:value={formRandomizeAmount}
          min="0"
          placeholder="0"
        />
      </div>
    {/if}

    <Button variant="outline" onclick={addTarget} style="width: 100%;">
      <span style="display: flex; align-items: center; gap: 0.4rem;">
        <ListPlus size={16} /> Add to Sequence
      </span>
    </Button>
  </Card>
</div>

<style>
  .sequence-layout {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .compact-control {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }

  .divider {
    height: 1px;
    background: var(--border, rgba(255, 255, 255, 0.1));
    margin: 1.25rem 0;
  }

  .status-chip {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--danger);
    background: rgba(255, 94, 94, 0.08);
    padding: 0.3rem 0.7rem;
    border-radius: var(--round-lg);
    border: 1px solid rgba(255, 94, 94, 0.15);
    transition: all 0.3s ease;
  }

  .status-chip.active {
    color: var(--success);
    background: rgba(16, 185, 129, 0.08);
    border-color: rgba(16, 185, 129, 0.15);
  }

  .status-dot-sm {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: currentColor;
    flex-shrink: 0;
  }

  .status-chip.active .status-dot-sm {
    animation: blink 1.2s infinite alternate;
  }

  .field-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .field-compact {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .field-label-sm {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--secondary-text);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }

  .coord-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .sequence-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .sequence-item {
    display: flex;
    align-items: center;
    gap: 1rem;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
    border-radius: var(--round-md);
    padding: 0.75rem;
  }

  .step-index {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background: var(--primary);
    color: var(--primary-text);
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    font-size: 0.85rem;
    flex-shrink: 0;
  }

  .step-content {
    flex: 1;
    min-width: 0;
  }

  .step-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--primary-text);
    margin-bottom: 0.2rem;
  }

  .step-details {
    font-size: 0.8rem;
    color: var(--muted-text);
  }
</style>
