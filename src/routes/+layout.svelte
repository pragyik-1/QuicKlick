<script lang="ts">
  import '../app.css'
  import { Button, NavBar, Select, ToastManager } from 'bluenite'
  import { page } from '$app/state'
  import { clickerState } from '$lib/clickerState.svelte'
  import { Settings, House, SlidersVertical, ListOrdered } from 'lucide-svelte'

  let { children } = $props()

  let presetOptions = $derived([
    { label: 'None', value: '' },
    ...Object.keys(clickerState.settings.presets).map((p) => ({ label: p, value: p })),
  ])
</script>

<NavBar height={45}>
  <a class="nav-brand" href="/">
    <svg
      class="brand-icon"
      class:pulse={clickerState.isRunning}
      width="22"
      height="22"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <path d="M3 3l7.07 16.97 2.51-7.39 7.39-2.51L3 3z" />
      <path d="M13 13l6 6" />
    </svg>
    QuicKlick
  </a>

  <div style="display: flex; align-items: center; gap: 0.5rem;">
    {#if Object.keys(clickerState.settings.presets).length > 0}
      <div style="width: 130px; margin-right: 0.5rem;">
        <Select options={presetOptions} bind:value={clickerState.activePreset} />
      </div>
    {/if}
    <nav class="nav-links" style="gap: 0.25rem;">
      <a
        class="nav-link"
        class:active={page.url.pathname === '/'}
        href="/"
        title="Home"
        style="padding: 0.4rem;"
      >
        <House size={18} />
      </a>
      <a
        class="nav-link"
        class:active={page.url.pathname === '/advanced'}
        href="/advanced"
        title="Advanced"
        style="padding: 0.4rem;"
      >
        <SlidersVertical size={18} />
      </a>
      <a
        class="nav-link"
        class:active={page.url.pathname === '/sequence'}
        href="/sequence"
        title="Sequence"
        style="padding: 0.4rem;"
      >
        <ListOrdered size={18} />
      </a>
      <a
        class="nav-link"
        class:active={page.url.pathname === '/settings'}
        href="/settings"
        title="Settings"
        style="padding: 0.4rem;"
      >
        <Settings size={18} />
      </a>
    </nav>
  </div>
</NavBar>

<div class="app-content">
  {@render children()}
</div>

<ToastManager />

<style>
  .app-content {
    max-width: 720px;
    margin: 1.5rem auto;
    padding: 0 1.5rem;
    box-sizing: border-box;
  }
</style>
