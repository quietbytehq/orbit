<script lang="ts">
  import type { AppError } from "$lib/api/types";
  import { text } from "$lib/text/index.svelte";

  /**
   * Shows a problem in plain words, with the technical text (if any) behind a toggle.
   * Accepts a plain string too, for messages the UI composes itself.
   */
  let {
    error,
    role = "alert",
  }: { error: AppError | string | null | undefined; role?: "alert" | "status" } = $props();

  let open = $state(false);
  const message = $derived(typeof error === "string" ? error : (error?.message ?? ""));
  const details = $derived(typeof error === "string" ? undefined : error?.details);
</script>

{#if error}
  <div class="note" {role}>
    <p class="message">{message}</p>
    {#if details}
      <button class="toggle" type="button" onclick={() => (open = !open)} aria-expanded={open}>
        {open ? text.s.common.hideDetails : text.s.common.showDetails}
      </button>
      {#if open}
        <pre class="details">{details}</pre>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .note {
    margin: 0 0 12px;
  }
  .message {
    margin: 0;
    color: var(--status-disconnected);
  }
  .toggle {
    appearance: none;
    border: none;
    background: none;
    padding: 2px 0;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: pointer;
    text-decoration: underline;
  }
  .details {
    margin: 4px 0 0;
    padding: 8px 10px;
    border-radius: var(--radius-control);
    background: var(--control-bg);
    color: var(--text-secondary);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 11px;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 160px;
    overflow: auto;
    user-select: text;
  }
</style>
