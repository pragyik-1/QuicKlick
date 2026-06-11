<script lang="ts">
  import { Card, Button, Input, Select, Switch, Row } from "bluenite";
  import { untrack } from "svelte";
  import { clickerState } from "$lib/clickerState.svelte";
  import HotkeyInput from "$lib/HotkeyInput.svelte";
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

  let device = $state<string>(clickerState.target.device);
  let button = $state<string>(clickerState.target.button ?? "Left");
  let keyCode = $state(clickerState.target.key_code ?? "");
  let useCustomPos = $state(clickerState.target.mouse_position !== null);
  let posX = $state(String(clickerState.target.mouse_position?.[0] ?? 0));
  let posY = $state(String(clickerState.target.mouse_position?.[1] ?? 0));
  let delay = $state(String(clickerState.delay));

  $effect(() => {
    device = clickerState.target.device;
    button = clickerState.target.button ?? "Left";
    keyCode = clickerState.target.key_code ?? "";
    const mp = clickerState.target.mouse_position;
    useCustomPos = mp !== null;
    if (mp) {
      posX = String(mp[0]);
      posY = String(mp[1]);
    }
    delay = String(clickerState.delay);
  });

  function applyTarget() {
    clickerState.setTarget({
      device: device as "Mouse" | "Keyboard",
      button:
        device === "Mouse" ? (button as "Left" | "Right" | "Middle") : null,
      key_code: device === "Keyboard" ? keyCode : null,
      mouse_position:
        device === "Mouse" && useCustomPos
          ? [parseInt(posX) || 0, parseInt(posY) || 0]
          : null,
    });
  }

  let prevDevice = untrack(() => device);
  let prevButton = untrack(() => button);
  let prevUseCustomPos = untrack(() => useCustomPos);

  $effect(() => {
    if (device !== prevDevice) {
      prevDevice = device;
      applyTarget();
    }
  });

  $effect(() => {
    if (button !== prevButton) {
      prevButton = button;
      applyTarget();
    }
  });

  $effect(() => {
    if (useCustomPos !== prevUseCustomPos) {
      prevUseCustomPos = useCustomPos;
      applyTarget();
    }
  });

  function onKeyCapture(e: { key: string; modifiers: string[] }) {
    keyCode = e.key;
    applyTarget();
  }

  let delayTimer: ReturnType<typeof setTimeout>;
  function onDelayInput(e: Event) {
    delay = (e.target as HTMLInputElement).value;
    clearTimeout(delayTimer);
    delayTimer = setTimeout(() => {
      const num = parseInt(delay, 10);
      if (!isNaN(num) && num > 0) clickerState.setDelay(num);
    }, 400);
  }
</script>

<Card
  title="Control"
  style="height: auto; display: flex; flex-direction: column;"
>
  <div class="toggle-section" style="margin-bottom: 1.25rem;">
    <div class="toggle-status">
      <span class="toggle-dot" class:active={clickerState.is_running}></span>
      <div>
        <div class="toggle-label">
          {clickerState.is_running ? "Running" : "Stopped"}
        </div>
        <div class="toggle-sublabel">Press the button to toggle</div>
      </div>
    </div>
    <Row gap={1}>
      <Button
        variant={clickerState.is_running ? "outline" : "fill"}
        disabled={clickerState.is_running}
        onclick={() => clickerState.toggle()}>Start</Button
      >
      <Button
        variant={clickerState.is_running ? "fill" : "outline"}
        disabled={!clickerState.is_running}
        onclick={() => clickerState.toggle()}>Stop</Button
      >
    </Row>
  </div>

  <div class="field">
    <label class="field-label" for="delay-input">Interval (ms)</label>
    <Input
      style="color: var(--primary-text);"
      id="delay-input"
      type="number"
      value={delay}
      oninput={onDelayInput}
      min="1"
      placeholder="100"
    />
  </div>

  <div style="margin-top: 0.75rem;" class="field">
    <label class="field-label" for="device-select">Device</label>
    <Select
      id="device-select"
      style="color: var(--primary-text);"
      options={DEVICE_OPTIONS}
      bind:value={device}
    />
  </div>

  {#if device === "Mouse"}
    <div class="field" style="margin-top: 0.75rem;">
      <label class="field-label" for="button-select">Mouse Button</label>
      <Select
        id="button-select"
        style="color: var(--primary-text);"
        options={BUTTON_OPTIONS}
        bind:value={button}
      />
    </div>

    <div style="margin-top: 0.75rem;">
      <Switch label="Custom Position" size="sm" bind:checked={useCustomPos} />
    </div>

    {#if useCustomPos}
      <div class="coord-fields" style="margin-top: 0.75rem;">
        <div class="field">
          <label class="field-label" for="pos-x">X</label>
          <Input
            style="color: var(--primary-text);"
            id="pos-x"
            type="number"
            value={posX}
            oninput={(e) => {
              posX = (e.target as HTMLInputElement).value;
              applyTarget();
            }}
            placeholder="0"
          />
        </div>
        <div class="field">
          <label class="field-label" for="pos-y">Y</label>
          <Input
            style="color: var(--primary-text);"
            id="pos-y"
            type="number"
            value={posY}
            oninput={(e) => {
              posY = (e.target as HTMLInputElement).value;
              applyTarget();
            }}
            placeholder="0"
          />
        </div>
      </div>
    {/if}
  {:else}
    <div class="field" style="margin-top: 0.75rem;">
      <span class="field-label">Key to Press</span>
      <HotkeyInput
        value={keyCode}
        onCapture={onKeyCapture}
        placeholder="Click to set key..."
      />
    </div>
  {/if}
</Card>
