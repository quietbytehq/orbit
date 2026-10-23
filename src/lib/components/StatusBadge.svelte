<script lang="ts">
  import type { OverallStatus } from "$lib/api/types";
  import { text } from "$lib/text/index.svelte";
  import { STATUS_COLOR_VAR } from "$lib/util/status";

  let { status, showLabel = true }: { status: OverallStatus; showLabel?: boolean } = $props();
  const label = $derived(text.s.status[status]);
</script>

<span class="badge" data-status={status} title={label}>
  <span class="dot" style:background={STATUS_COLOR_VAR[status]} aria-hidden="true"></span>
  {#if showLabel}<span>{label}</span>{:else}<span class="sr">{label}</span>{/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    flex: none;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
