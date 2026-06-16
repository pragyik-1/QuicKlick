<script lang="ts">
    import { Card } from "bluenite";
    import { clickerState } from "$lib/clickerState.svelte";
    import { toast } from "bluenite";
    import HotkeyInput from "$lib/HotkeyInput.svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import { TriangleAlert } from "lucide-svelte";

    import "./page.css";

    const SHORTCUTS = [
        {
            id: "toggle-clicker",
            name: "Toggle Clicker",
            desc: "Start or stop the autoclicker",
            defaultKey: "F6",
        },
        {
            id: "start-clicker",
            name: "Start Clicker",
            desc: "Force start the autoclicker",
            defaultKey: "Shift+F6",
        },
        {
            id: "stop-clicker",
            name: "Stop Clicker",
            desc: "Force stop the autoclicker",
            defaultKey: "Ctrl+F6",
        },
    ];

    let bindings = $state<Record<string, string>>(
        Object.fromEntries(SHORTCUTS.map((s) => [s.id, s.defaultKey])),
    );

    let isWayland = $state(false);

    onMount(async () => {
        try {
            isWayland = await invoke<boolean>("is_wayland_cmd");
        } catch (err) {
            console.error("Failed to check wayland status", err);
        }
    });

    function formatDisplay(key: string, modifiers: string[]): string {
        const parts = modifiers.map(
            (m) => m.charAt(0).toUpperCase() + m.slice(1),
        );
        parts.push(
            key.length === 1
                ? key.toUpperCase()
                : key.charAt(0).toUpperCase() + key.slice(1),
        );
        return parts.join("+");
    }

    async function onCapture(
        id: string,
        event: { key: string; modifiers: string[] },
    ) {
        try {
            await clickerState.updateShortcut(id, event.key, event.modifiers);
            bindings[id] = formatDisplay(event.key, event.modifiers);
            toast.show({
                message: "Shortcut updated",
                variant: "success",
                duration: 2000,
            });
        } catch (err) {
            toast.show({
                message: `Failed to update shortcut: ${err}`,
                variant: "danger",
            });
        }
    }
</script>

<Card title="Shortcuts">
    {#if isWayland}
        <div
            style="padding: 1rem; text-align: center; color: var(--muted-text);"
        >
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
                Changing global shortcuts for individual apps is not supported
                in Wayland. Please use your native system settings (e.g. GNOME
                Settings, KDE System Settings) to configure global shortcuts.
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
