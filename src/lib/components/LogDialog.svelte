<script lang="ts">
  import Dialog from "./Dialog.svelte";
  import { text } from "$lib/text/index.svelte";

  let { title, text: body, onClose }: { title: string; text: string | null; onClose: () => void } = $props();
</script>

<Dialog {title} {onClose}>
  {#if body === null}
    <p>{text.s.sync.loadingLog}</p>
  {:else}
    <pre class="log">{body}</pre>
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
