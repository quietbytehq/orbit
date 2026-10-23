<script lang="ts">
  import ErrorNote from "$lib/components/ErrorNote.svelte";
  import StatusBadge from "$lib/components/StatusBadge.svelte";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { text } from "$lib/text/index.svelte";
  import { overallStatus } from "$lib/util/status";
  import { ICON, RefreshCw } from "$lib/icons";

  let status = $derived(overallStatus(rcd.state));
  let restarting = $state(false);

  async function restart() {
    restarting = true;
    await rcd.restart();
    restarting = false;
  }
</script>

<h1>{text.s.overview.title}</h1>

<section class="card engine" aria-labelledby="engine-title">
  <div class="row">
    <div>
      <h2 id="engine-title">{text.s.overview.engineTitle}</h2>
      <p class="detail">
        {#if rcd.state.state === "healthy"}
          {text.s.overview.ready(rcd.state.version)}
        {:else if rcd.state.state === "starting"}
          {text.s.overview.starting}
        {:else if rcd.state.state === "restarting"}
          {text.s.overview.restarting(rcd.state.attempt)}
        {:else if rcd.state.state === "failed"}
          {rcd.state.message}
        {:else}
          {text.s.overview.stopped}
        {/if}
      </p>
    </div>
    <div class="side">
      <StatusBadge {status} />
      <button class="button with-icon" onclick={restart} disabled={restarting || rcd.state.state === "starting"}>
        <RefreshCw {...ICON} />{text.s.overview.restart}
      </button>
    </div>
  </div>
  <ErrorNote error={rcd.error} />
</section>

<style>
  h2 {
    font-size: 15px;
    font-weight: 600;
    margin: 0 0 4px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  .detail {
    margin: 0;
    color: var(--text-secondary);
  }
  .side {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: none;
  }
  .engine {
    max-width: 680px;
  }
</style>
