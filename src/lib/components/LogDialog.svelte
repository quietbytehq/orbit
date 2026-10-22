<script lang="ts">
  import Dialog from "./Dialog.svelte";

  let { title, text, onClose }: { title: string; text: string | null; onClose: () => void } = $props();
</script>

<Dialog {title} {onClose}>
  {#if text === null}
    <p>Loading the log…</p>
  {:else}
    <pre class="log">{text}</pre>
  {/if}
</Dialog>

<style>
  .log {
    margin: 0;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    background: var(--control-bg);
    border-radius: var(--radius-control);
    padding: 10px 12px;
    max-height: 56vh;
    overflow: auto;
    user-select: text;
  }
</style>
