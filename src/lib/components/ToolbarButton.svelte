<script lang="ts">
  import type { Component } from "svelte";
  import { ICON } from "$lib/icons";

  /**
   * A toolbar button with an icon and a label. Inside a `.toolbar` container the label hides
   * when the toolbar is narrow (see app.css), and the label then shows as a tooltip.
   */
  let {
    icon,
    label,
    onclick,
    disabled = false,
    primary = false,
    destructive = false,
    type = "button",
    href,
  }: {
    icon: Component;
    label: string;
    onclick?: () => void;
    disabled?: boolean;
    primary?: boolean;
    destructive?: boolean;
    type?: "button" | "submit";
    href?: string;
  } = $props();
  const Icon = $derived(icon);
</script>

{#if href}
  <a class="button tb" class:primary class:destructive {href} title={label} aria-label={label}>
    <Icon {...ICON} />
    <span class="tb-label">{label}</span>
  </a>
{:else}
  <button class="button tb" class:primary class:destructive {type} {onclick} {disabled} title={label} aria-label={label}>
    <Icon {...ICON} />
    <span class="tb-label">{label}</span>
  </button>
{/if}

<style>
  .tb {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    text-decoration: none;
    white-space: nowrap;
  }
  .tb.destructive {
    color: var(--status-disconnected);
  }
</style>
