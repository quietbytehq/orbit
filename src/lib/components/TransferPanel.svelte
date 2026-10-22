<script lang="ts">
  import { transfers } from "$lib/stores/transfers.svelte";
  import type { TransferItem } from "$lib/api/types";
  import { formatBytes, formatDuration, formatSpeed } from "$lib/util/format";

  let expanded = $state(true);
  const DIRECTION_LABEL = { upload: "Upload", download: "Download", copy: "Copy", move: "Move" } as const;

  function progress(item: TransferItem): number | null {
    if (item.totalBytes <= 0) return null;
    return Math.min(100, (item.bytes / item.totalBytes) * 100);
  }

  function describe(item: TransferItem): string {
    switch (item.state.state) {
      case "queued":
        return "Waiting";
      case "running": {
        const amount = item.totalBytes > 0 ? `${formatBytes(item.bytes)} of ${formatBytes(item.totalBytes)}` : "Starting";
        const speed = item.speed > 0 ? ` · ${formatSpeed(item.speed)}` : "";
        const eta = item.etaSeconds !== null ? ` · ${formatDuration(item.etaSeconds)} left` : "";
        return `${amount}${speed}${eta}`;
      }
      case "done":
        return "Done";
      case "failed":
        return item.state.message;
      case "cancelled":
        return "Cancelled";
    }
  }

  const isActive = (item: TransferItem) => item.state.state === "queued" || item.state.state === "running";
</script>

{#if transfers.items.length > 0}
  <section class="panel" aria-label="Transfers">
    <header>
      <button class="toggle" onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
        Transfers{transfers.active.length > 0 ? ` · ${transfers.active.length} in progress` : ""}
      </button>
      <span class="spacer"></span>
      {#if transfers.finished.length > 0}
        <button class="link" onclick={() => transfers.clearFinished()}>Clear finished</button>
      {/if}
    </header>

    {#if expanded}
      <ul>
        {#each transfers.items as item (item.id)}
          {@const percent = progress(item)}
          <li data-state={item.state.state}>
            <div class="info">
              <span class="name" title={item.name}>{DIRECTION_LABEL[item.direction]} · {item.name}</span>
              <span class="route">{item.from} → {item.to}</span>
            </div>
            <div class="bar" aria-hidden="true">
              <div class:indeterminate={item.state.state === "running" && percent === null} style:width={percent !== null ? `${percent}%` : undefined}></div>
            </div>
            <span class="status" class:failed={item.state.state === "failed"}>{describe(item)}</span>
            {#if isActive(item)}
              <button class="link" onclick={() => transfers.cancel(item.id)}>Cancel</button>
            {:else}
              <span class="spacer-sm"></span>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </section>
{/if}

<style>
  .panel {
    border-top: 1px solid var(--separator);
    background: var(--sidebar-bg);
    backdrop-filter: blur(30px) saturate(180%);
    -webkit-backdrop-filter: blur(30px) saturate(180%);
    max-height: 40vh;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    padding: 6px 14px;
    gap: 10px;
  }
  .toggle {
    appearance: none;
    border: none;
    background: none;
    font-weight: 600;
    padding: 2px 0;
    cursor: pointer;
  }
  .spacer {
    flex: 1;
  }
  .spacer-sm {
    width: 56px;
  }
  .link {
    appearance: none;
    border: none;
    background: none;
    color: var(--accent);
    cursor: pointer;
    padding: 2px 4px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0 14px 10px;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  li {
    display: grid;
    grid-template-columns: minmax(160px, 1.4fr) minmax(90px, 1fr) minmax(140px, 1.2fr) auto;
    align-items: center;
    gap: 12px;
  }
  .info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name,
  .route {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .route {
    color: var(--text-secondary);
    font-size: 11px;
  }
  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--control-bg);
    overflow: hidden;
    position: relative;
  }
  .bar > div {
    height: 100%;
    background: var(--status-syncing);
    border-radius: 3px;
    transition: width 0.3s ease;
  }
  .bar > div.indeterminate {
    width: 40%;
    position: absolute;
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      left: -40%;
    }
    to {
      left: 100%;
    }
  }
  .status {
    color: var(--text-secondary);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .status.failed {
    color: var(--status-disconnected);
  }
  li[data-state="done"] .bar > div {
    background: var(--status-synced);
  }
  li[data-state="failed"] .bar > div {
    background: var(--status-disconnected);
  }
</style>
