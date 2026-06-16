<script lang="ts">
  import { Card, Button, Input, Select, Switch, Row } from "bluenite";
  import { clickerState } from "$lib/clickerState.svelte";
  import HotkeyInput from "$lib/HotkeyInput.svelte";
  import {
    Play,
    Square,
    Gauge,
    Monitor,
    MousePointer2,
    Keyboard,
  } from "lucide-svelte";
  import "./page.css";

  const DEVICE_OPTIONS = [
    { label: "Mouse", value: "Mouse" },
    { label: "Keyboard", value: "Keyboard" },
  ];

  const BUTTON_OPTIONS = [
    { label: "Left", value: "Left" },
    { label: "Right", value: "Right" },
    { label: "Middle", value: "Middle" },
  ];

  function onKeyCapture(e: { key: string; modifiers: string[] }) {
    clickerState.keyCode = e.key;
  }
</script>

<Card style="height: auto; display: flex; flex-direction: column;">
  <div class="toggle-section" style="margin-bottom: 1.25rem;">
    <div class="toggle-status">
      <span class="toggle-dot" class:active={clickerState.isRunning}></span>
      <div>
        <div class="toggle-label">
          {clickerState.isRunning ? "Running" : "Stopped"}
        </div>
        <div class="toggle-sublabel">Press the button to toggle</div>
      </div>
    </div>
    <Row gap={1}>
      <Button
        variant={clickerState.isRunning ? "outline" : "fill"}
        disabled={clickerState.isRunning}
        onclick={() => clickerState.toggle()}
      >
        <span style="display: flex; align-items: center; gap: 0.4rem;">
          <Play size={16} /> Start
        </span>
      </Button>
      <Button
        variant={clickerState.isRunning ? "fill" : "outline"}
        disabled={!clickerState.isRunning}
        onclick={() => clickerState.toggle()}
      >
        <span style="display: flex; align-items: center; gap: 0.4rem;">
          <Square size={16} /> Stop
        </span>
      </Button>
    </Row>
  </div>

  <div class="field">
    <label class="field-label" for="cps-input">
      <Gauge size={16} />
      Clicks per second
    </label>
    <Input
      style="color: var(--primary-text);"
      id="cps-input"
      type="number"
      bind:value={clickerState.cps}
      min="0.01"
      step="0.1"
      placeholder="10"
    />
  </div>

  <div style="margin-top: 0.75rem;" class="field">
    <label class="field-label" for="device-select">
      {#if clickerState.device === "Mouse"}
        <Monitor size={16} />
      {:else}
        <Keyboard size={16} />
      {/if}
      Device
    </label>
    <Select
      id="device-select"
      style="color: var(--primary-text);"
      options={DEVICE_OPTIONS}
      bind:value={clickerState.device}
    />
  </div>

  {#if clickerState.device === "Mouse"}
    <div class="field" style="margin-top: 0.75rem;">
      <label class="field-label" for="button-select">
        <MousePointer2 size={16} />
        Mouse Button
      </label>
      <Select
        id="button-select"
        style="color: var(--primary-text);"
        options={BUTTON_OPTIONS}
        bind:value={clickerState.button}
      />
    </div>

    <div style="margin-top: 0.75rem;">
      <Switch
        label="Custom Position"
        size="sm"
        bind:checked={clickerState.useCustomPos}
      />
    </div>

    {#if clickerState.useCustomPos}
      <div class="coord-fields" style="margin-top: 0.75rem;">
        <div class="field">
          <label class="field-label" for="pos-x">X</label>
          <Input
            style="color: var(--primary-text);"
            id="pos-x"
            type="number"
            bind:value={clickerState.posX}
            placeholder="0"
          />
        </div>
        <div class="field">
          <label class="field-label" for="pos-y">Y</label>
          <Input
            style="color: var(--primary-text);"
            id="pos-y"
            type="number"
            bind:value={clickerState.posY}
            placeholder="0"
          />
        </div>
      </div>
    {/if}
  {:else}
    <div class="field" style="margin-top: 0.75rem;">
      <span class="field-label">
        <Keyboard size={16} />
        Key to Press
      </span>
      <HotkeyInput
        value={clickerState.keyCode}
        onCapture={onKeyCapture}
        placeholder="Click to set key..."
      />
    </div>
  {/if}
</Card>
