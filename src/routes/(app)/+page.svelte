<script lang="ts">
  import StatusBadge from "$lib/components/StatusBadge.svelte";
  import { rcd } from "$lib/stores/rcd.svelte";
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

<h1>Overview</h1>

<section class="card engine" aria-labelledby="engine-title">
  <div class="row">
    <div>
      <h2 id="engine-title">rclone engine</h2>
      <p class="detail">
        {#if rcd.state.state === "healthy"}
          rclone {rcd.state.version} is running on this computer.
        {:else if rcd.state.state === "starting"}
          Starting rclone…
        {:else if rcd.state.state === "restarting"}
          Reconnecting, attempt {rcd.state.attempt} of 3…
        {:else if rcd.state.state === "failed"}
          {rcd.state.message}
        {:else}
          rclone is stopped.
        {/if}
      </p>
    </div>
    <div class="side">
      <StatusBadge {status} />
      <button class="button with-icon" onclick={restart} disabled={restarting || rcd.state.state === "starting"}>
        <RefreshCw {...ICON} />Restart rclone
      </button>
    </div>
  </div>
  {#if rcd.error}
    <p class="error" role="alert">{rcd.error}</p>
  {/if}
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
    gap: 14px;
    flex: none;
  }
  .error {
    color: var(--status-disconnected);
    margin: 12px 0 0;
  }
</style>
