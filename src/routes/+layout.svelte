<script lang="ts">
    import "../app.css";
    import { NavBar, ToastManager } from "bluenite";
    import { page } from "$app/state";
    import { clickerState } from "$lib/clickerState.svelte";
    let { children } = $props();
</script>

<NavBar>
    <a class="nav-brand" href="/">
        <svg
            class="brand-icon"
            class:pulse={clickerState.is_running}
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

    <nav class="nav-links">
        <a class="nav-link" class:active={page.url.pathname === "/"} href="/"
            >Home</a
        >
        <a
            class="nav-link"
            class:active={page.url.pathname === "/advanced"}
            href="/advanced">Advanced</a
        >
        <a
            class="nav-link"
            class:active={page.url.pathname === "/settings"}
            href="/settings">Settings</a
        >
    </nav>
</NavBar>

<div class="app-content">
    {@render children()}
</div>

<ToastManager />
