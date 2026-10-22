<script lang="ts">
  import { onMount } from "svelte";
  import StatusBadge from "$lib/components/StatusBadge.svelte";
  import { api, errorMessage } from "$lib/api";
  import type { RemoteSummary } from "$lib/api/types";
  import { jobs } from "$lib/stores/jobs.svelte";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { transfers } from "$lib/stores/transfers.svelte";
  import { remoteBadge, remotesInUse } from "$lib/util/remote";

  let remotes = $state<RemoteSummary[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let testing = $state<Record<string, boolean>>({});
  const inUse = $derived(remotesInUse(transfers.items, jobs.items));

  async function load() {
    loading = true;
    error = null;
    try {
      remotes = await api.listRemotes();
    } catch (err) {
      error = errorMessage(err);
    } finally {
      loading = false;
    }
  }

  async function test(name: string) {
    testing[name] = true;
    try {
      const outcome = await api.testRemote(name);
      remotes = remotes.map((remote) =>
        remote.name === name
          ? { ...remote, health: outcome.ok ? { state: "ok" } : { state: "failed", message: outcome.message } }
          : remote,
      );
    } catch (err) {
      error = errorMessage(err);
    } finally {
      testing[name] = false;
    }
  }

  onMount(() => {
    void load();
    void transfers.start();
    void jobs.start();
  });

  // Reload when rclone comes back, since the list cannot be read while it is down.
  $effect(() => {
    if (rcd.state.state === "healthy" && !loading && remotes.length === 0 && error) {
      void load();
    }
  });
</script>

<div class="header">
  <h1>Remotes</h1>
  <a class="button primary" href="/remotes/new">Add remote</a>
</div>

{#if rcd.state.state !== "healthy"}
  <p class="notice">Orbit cannot reach rclone right now, so connections cannot be checked.</p>
{/if}

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

{#if loading}
  <p class="help">Loading connections…</p>
{:else if remotes.length === 0}
  <section class="card empty">
    <h2>No connections yet</h2>
    <p>Add Google Drive, OneDrive, Dropbox, an S3 bucket or an SFTP server to get started.</p>
    <a class="button primary" href="/remotes/new">Add your first remote</a>
  </section>
{:else}
  <ul class="list">
    {#each remotes as remote (remote.name)}
      {@const badge = remoteBadge(remote, inUse)}
      <li class="card row">
        <div class="meta">
          <span class="name">{remote.name}</span>
          <span class="type">{remote.providerTitle}</span>
          {#if remote.health.state === "failed"}
            <span class="help">{remote.health.message}</span>
          {/if}
        </div>
        <div class="side">
          {#if badge}<StatusBadge status={badge} />{/if}
          <button class="button" onclick={() => test(remote.name)} disabled={testing[remote.name]}>
            {testing[remote.name] ? "Checking…" : "Test"}
          </button>
          <a class="button" href={`/browse/${encodeURIComponent(remote.name)}`}>Open</a>
          <a class="button" href={`/remotes/${encodeURIComponent(remote.name)}`}>Edit</a>
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }
  .header h1 {
    margin: 0;
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    font-size: 14px;
  }
  .type,
  .help {
    color: var(--text-secondary);
    font-size: 12px;
  }
  .side {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
  }
  .empty h2 {
    font-size: 15px;
    margin: 0 0 6px;
  }
  .empty p {
    color: var(--text-secondary);
    margin: 0 0 14px;
  }
  .notice {
    color: var(--status-disconnected);
  }
  .error {
    color: var(--status-disconnected);
  }
  :global(a.button) {
    text-decoration: none;
    display: inline-block;
  }
</style>
