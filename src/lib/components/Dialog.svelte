<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    onClose,
    children,
    actions,
  }: { title: string; onClose: () => void; children: Snippet; actions?: Snippet } = $props();

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="overlay" role="presentation" onclick={(event) => event.target === event.currentTarget && onClose()}>
  <div class="sheet" role="dialog" aria-modal="true" aria-labelledby="dialog-title">
    <h2 id="dialog-title">{title}</h2>
    <div class="body">{@render children()}</div>
    {#if actions}
      <div class="actions">{@render actions()}</div>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.25);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
    padding: 16px;
  }
  .sheet {
    background: var(--window-bg);
    border-radius: var(--radius-window);
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
    padding: 20px 22px 18px;
    width: min(460px, 100%);
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h2 {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
  }
  .body {
    overflow: auto;
    min-height: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  :global(.sheet p) {
    margin: 0 0 10px;
    color: var(--text-secondary);
  }
</style>
