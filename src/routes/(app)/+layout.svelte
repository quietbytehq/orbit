<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { transfers } from "$lib/stores/transfers.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { text } from "$lib/text/index.svelte";
  import TransferPanel from "$lib/components/TransferPanel.svelte";
  import { ICON, NAV_ICONS } from "$lib/icons";

  let { children } = $props();

  onMount(() => {
    void text.load();
    void rcd.start();
    void transfers.start();
    void settings.load();
  });

  function isCurrent(href: string): boolean {
    const path = page.url.pathname;
    return href === "/" ? path === "/" : path === href || path.startsWith(`${href}/`);
  }

  const sections = $derived([
    { href: "/", label: text.s.nav.overview },
    { href: "/remotes", label: text.s.nav.storage },
    { href: "/sync", label: text.s.nav.sync },
    { href: "/mounts", label: text.s.nav.drives },
    { href: "/settings", label: text.s.nav.settings },
  ]);
</script>

<div class="shell">
  <aside class="sidebar" aria-label="Sections">
    <div class="brand">
      <img src="/favicon.png" alt="" width="22" height="22" />
      <span>{text.s.app.name}</span>
    </div>
    <nav>
      {#each sections as section (section.href)}
        {@const Icon = NAV_ICONS[section.href]}
        <a href={section.href} class="item" aria-current={isCurrent(section.href) ? "page" : undefined}>
          <Icon {...ICON} />
          <span>{section.label}</span>
        </a>
      {/each}
    </nav>
  </aside>
  <div class="column">
    <main class="content">
      {@render children()}
    </main>
    <TransferPanel />
  </div>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 200px 1fr;
    height: 100vh;
    background: var(--window-bg);
  }
  .sidebar {
    background: var(--sidebar-bg);
    backdrop-filter: blur(30px) saturate(180%);
    -webkit-backdrop-filter: blur(30px) saturate(180%);
    border-right: 1px solid var(--separator);
    padding: 40px 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 10px;
    font-weight: 600;
    font-size: 14px;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 6px 10px;
    border-radius: var(--radius-control);
    text-decoration: none;
    color: var(--text-primary);
  }
  .item:hover {
    background: var(--control-bg);
  }
  .item[aria-current="page"] {
    background: var(--sidebar-selected);
    font-weight: 500;
  }
  .item :global(svg) {
    color: var(--text-secondary);
    flex: none;
  }
  .item[aria-current="page"] :global(svg) {
    color: var(--accent);
  }
  .column {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
  }
  .content {
    overflow: auto;
    padding: 36px 32px 32px;
    min-width: 0;
    flex: 1;
  }
  :global(.content h1) {
    font-size: 22px;
    font-weight: 700;
    margin: 0 0 20px;
  }
  :global(.card) {
    background: var(--panel-bg);
    backdrop-filter: blur(30px) saturate(180%);
    -webkit-backdrop-filter: blur(30px) saturate(180%);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-card);
    padding: 16px 18px;
  }
  :global(.button) {
    appearance: none;
    border: none;
    border-radius: var(--radius-control);
    background: var(--control-bg);
    padding: 6px 14px;
    cursor: pointer;
    font-weight: 500;
  }
  :global(.button:hover) {
    background: var(--control-bg-hover);
  }
  :global(.button.primary) {
    background: var(--accent);
    color: #fff;
  }
  :global(.button.primary:hover) {
    filter: brightness(1.08);
  }
  :global(.button:disabled) {
    opacity: 0.5;
    cursor: default;
  }
  :global(.button.destructive) {
    color: var(--status-disconnected);
  }
  :global(.button.destructive-fill) {
    background: var(--status-disconnected);
    color: #fff;
  }
  :global(a.button) {
    text-decoration: none;
    display: inline-block;
  }
</style>
