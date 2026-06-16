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
        Crosshair,
        MousePointerClick,
    } from "lucide-svelte";

    const DEVICE_OPTIONS = [
        { label: "Mouse", value: "Mouse" },
        { label: "Keyboard", value: "Keyboard" },
    ];

    const BUTTON_OPTIONS = [
        { label: "Left", value: "Left" },
        { label: "Right", value: "Right" },
        { label: "Middle", value: "Middle" },
    ];

    const CLICK_TYPE_OPTIONS = [
        { label: "Single Click", value: "Single" },
        { label: "Double Click", value: "Double" },
    ];

    function onKeyCapture(e: { key: string; modifiers: string[] }) {
        clickerState.keyCode = e.key;
    }
</script>

<div class="advanced-layout">
    <Card style="height: auto;">
        <!-- Status + Toggle -->
        <div class="compact-control" style="margin-bottom: 1rem;">
            <div class="status-chip" class:active={clickerState.isRunning}>
                <span class="status-dot-sm"></span>
                {clickerState.isRunning ? "Running" : "Stopped"}
            </div>
            <Row gap={0.5}>
                <Button
                    variant={clickerState.isRunning ? "outline" : "fill"}
                    disabled={clickerState.isRunning}
                    onclick={() => clickerState.toggle()}
                >
                    <span
                        style="display: flex; align-items: center; gap: 0.3rem;"
                    >
                        <Play size={14} /> Start
                    </span>
                </Button>
                <Button
                    variant={clickerState.isRunning ? "fill" : "outline"}
                    disabled={!clickerState.isRunning}
                    onclick={() => clickerState.toggle()}
                >
                    <span
                        style="display: flex; align-items: center; gap: 0.3rem;"
                    >
                        <Square size={14} /> Stop
                    </span>
                </Button>
            </Row>
        </div>

        <div class="divider"></div>

        <!-- Timing -->
        <div class="field-compact" style="margin-bottom: 1rem;">
            <label class="field-label-sm" for="adv-cps">
                <Gauge size={14} /> Clicks / sec
            </label>
            <Input
                style="color: var(--primary-text);"
                id="adv-cps"
                type="number"
                bind:value={clickerState.cps}
                min="0.01"
                step="0.1"
                placeholder="10"
            />
        </div>

        <!-- Target Configuration -->
        <div class="field-grid" style="margin-bottom: 1rem;">
            <div class="field-compact">
                <label class="field-label-sm" for="adv-device">
                    {#if clickerState.device === "Mouse"}
                        <Monitor size={14} />
                    {:else}
                        <Keyboard size={14} />
                    {/if}
                    Device
                </label>
                <Select
                    id="adv-device"
                    style="color: var(--primary-text);"
                    options={DEVICE_OPTIONS}
                    bind:value={clickerState.device}
                />
            </div>

            {#if clickerState.device === "Mouse"}
                <div class="field-compact">
                    <label class="field-label-sm" for="adv-button">
                        <MousePointer2 size={14} /> Button
                    </label>
                    <Select
                        id="adv-button"
                        style="color: var(--primary-text);"
                        options={BUTTON_OPTIONS}
                        bind:value={clickerState.button}
                    />
                </div>
            {:else}
                <div class="field-compact">
                    <span class="field-label-sm">
                        <Keyboard size={14} /> Key
                    </span>
                    <HotkeyInput
                        value={clickerState.keyCode}
                        onCapture={onKeyCapture}
                        placeholder="Click to set..."
                    />
                </div>
            {/if}
        </div>

        {#if clickerState.device === "Mouse"}
            <div style="margin-top: 0.75rem; margin-bottom: 1rem;">
                <Switch
                    label="Custom Position"
                    size="sm"
                    bind:checked={clickerState.useCustomPos}
                />
            </div>

            {#if clickerState.useCustomPos}
                <div class="coord-row" style="margin-bottom: 1rem;">
                    <div class="field-compact">
                        <label class="field-label-sm" for="adv-x">
                            <Crosshair size={14} /> X
                        </label>
                        <Input
                            style="color: var(--primary-text);"
                            id="adv-x"
                            type="number"
                            bind:value={clickerState.posX}
                            placeholder="0"
                        />
                    </div>
                    <div class="field-compact">
                        <label class="field-label-sm" for="adv-y">
                            <Crosshair size={14} /> Y
                        </label>
                        <Input
                            style="color: var(--primary-text);"
                            id="adv-y"
                            type="number"
                            bind:value={clickerState.posY}
                            placeholder="0"
                        />
                    </div>
                </div>
            {/if}
        {/if}

        <div class="divider"></div>

        <!-- Click Type -->
        <div class="field-compact">
            <label class="field-label-sm" for="adv-click-type">
                <MousePointerClick size={14} /> Type
            </label>
            <Select
                id="adv-click-type"
                style="color: var(--primary-text);"
                options={CLICK_TYPE_OPTIONS}
                bind:value={clickerState.clickType}
            />
        </div>
    </Card>
</div>

<style>
    .advanced-layout {
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
        margin: 1rem 0;
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
        margin-top: 0.75rem;
    }
</style>
