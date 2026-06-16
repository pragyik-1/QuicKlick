<script lang="ts">
  const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta"]);

  type CaptureEvent = { key: string; modifiers: string[] };

  type Props = {
    value?: string;
    onCapture?: (event: CaptureEvent) => void;
    placeholder?: string;
  };

  let {
    value = "",
    onCapture,
    placeholder = "Click to bind...",
  }: Props = $props();

  let listening = $state(false);
  let buttonRef = $state<HTMLButtonElement | null>(null);

  function normalizeKey(e: KeyboardEvent): string | null {
    if (MODIFIER_KEYS.has(e.key)) return null;

    if (e.code === "Space") return "space";

    if (e.code.startsWith("Key")) return e.code.slice(3).toLowerCase();
    if (e.code.startsWith("Digit")) return e.code.slice(5);

    const map: Record<string, string> = {
      Enter: "enter",
      Tab: "tab",
      Backspace: "backspace",
      Escape: "escape",
      ArrowLeft: "left",
      ArrowRight: "right",
      ArrowUp: "up",
      ArrowDown: "down",
      Home: "home",
      End: "end",
      PageUp: "pageup",
      PageDown: "pagedown",
      Insert: "insert",
      Delete: "delete",
    };
    if (map[e.key]) return map[e.key];

    const fMatch = e.code.match(/^F(\d+)$/);
    if (fMatch) return `f${fMatch[1]}`;

    return e.key.length === 1 ? e.key.toLowerCase() : null;
  }

  function getModifiers(e: KeyboardEvent): string[] {
    const mods: string[] = [];
    if (e.ctrlKey) mods.push("ctrl");
    if (e.shiftKey) mods.push("shift");
    if (e.altKey) mods.push("alt");
    if (e.metaKey) mods.push("meta");
    return mods;
  }

  function handleKeydown(e: KeyboardEvent) {
    e.preventDefault();
    e.stopPropagation();

    const key = normalizeKey(e);
    if (!key) return;

    listening = false;
    buttonRef?.blur();
    onCapture?.({ key, modifiers: getModifiers(e) });
  }

  function startListening() {
    listening = true;
  }

  function handleBlur() {
    listening = false;
  }

  let formattedValue = $derived(
    value ? value.charAt(0).toUpperCase() + value.slice(1) : "",
  );
  let displayText = $derived(formattedValue || placeholder);
</script>

<button
  class="hotkey-input"
  class:listening
  class:empty={!value}
  bind:this={buttonRef}
  onclick={startListening}
  onkeydown={listening ? handleKeydown : undefined}
  onblur={handleBlur}
  type="button"
>
  {#if listening}
    <span class="hotkey-listening-text">Press a key...</span>
  {:else}
    <span class="hotkey-value">{displayText}</span>
  {/if}
</button>

<style>
  .hotkey-input {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    min-height: 38px;
    padding: 0.5rem 0.75rem;
    border-radius: var(--round-md);
    border: 1px solid var(--border);
    background-color: var(--input);
    color: var(--primary-text);
    font-family: var(--font-mono);
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
    transition:
      border-color 0.2s ease,
      box-shadow 0.2s ease,
      background-color 0.2s ease;
    box-sizing: border-box;
    outline: none;
    text-align: center;
    appearance: none;
    -webkit-appearance: none;
    -moz-appearance: none;
  }

  .hotkey-input:hover {
    border-color: var(--primary);
  }

  .hotkey-input.empty .hotkey-value {
    color: var(--muted-text);
  }

  .hotkey-input.listening {
    border-color: var(--primary);
    box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.25);
    background-color: var(--input-focus);
  }

  .hotkey-listening-text {
    color: var(--primary);
    font-family: var(--font-sans);
    font-weight: 500;
    animation: subtle-pulse 1.5s infinite;
  }
</style>
