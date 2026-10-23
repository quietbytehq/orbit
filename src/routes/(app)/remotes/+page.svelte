<script lang="ts">
  import { onMount } from "svelte";
  import ErrorNote from "$lib/components/ErrorNote.svelte";
  import StatusBadge from "$lib/components/StatusBadge.svelte";
  import { api, asAppError } from "$lib/api";
  import type { AppError, RemoteSummary } from "$lib/api/types";
  import { jobs } from "$lib/stores/jobs.svelte";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { transfers } from "$lib/stores/transfers.svelte";
  import { text } from "$lib/text/index.svelte";
  import { remoteBadge, remotesInUse } from "$lib/util/remote";
  import { ICON, ICON_LG, Plus, providerIcon } from "$lib/icons";

  let remotes = $state<RemoteSummary[]>([]);
  let loading = $state(true);
  let error = $state<AppError | null>(null);
  let testing = $state<Record<string, boolean>>({});
  const inUse = $derived(remotesInUse(transfers.items, jobs.items));

  async function load() {
    loading = true;
    error = null;
    try {
      remotes = await api.listRemotes();
    } catch (err) {
      error = asAppError(err);
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
      error = asAppError(err);
    } finally {
      testing[name] = false;
    }
  }

  onMount(() => {
    void load();
    void transfers.start();
    void jobs.start();
  });

  // Reload when the engine comes back, since the list cannot be read while it is down.
  $effect(() => {
    if (rcd.state.state === "healthy" && !loading && remotes.length === 0 && error) {
      void load();
    }
  });
</script>

<div class="header">
  <h1>{text.s.storage.title}</h1>
  <a class="button primary with-icon" href="/remotes/new"><Plus {...ICON} />{text.s.storage.add}</a>
</div>
<p class="intro">{text.s.storage.intro}</p>

{#if rcd.state.state !== "healthy"}
  <p class="notice">{text.s.storage.cannotCheck}</p>
{/if}

<ErrorNote {error} />

{#if loading}
  <p class="help">{text.s.storage.loading}</p>
{:else if remotes.length === 0}
  <section class="card empty">
    <h2>{text.s.storage.emptyTitle}</h2>
    <p>{text.s.storage.emptyBody}</p>
    <a class="button primary" href="/remotes/new">{text.s.storage.addFirst}</a>
  </section>
{:else}
  <ul class="list">
    {#each remotes as remote (remote.name)}
      {@const badge = remoteBadge(remote, inUse)}
      {@const Icon = providerIcon(remote.providerId)}
      <li class="card row">
        <span class="glyph"><Icon {...ICON_LG} /></span>
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
            {testing[remote.name] ? text.s.storage.checking : text.s.storage.check}
          </button>
          <a class="button" href={`/browse/${encodeURIComponent(remote.name)}`}>{text.s.storage.open}</a>
          <a class="button" href={`/remotes/${encodeURIComponent(remote.name)}`}>{text.s.storage.edit}</a>
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
    margin-bottom: 8px;
  }
  .header h1 {
    margin: 0;
  }
  .intro {
    color: var(--text-secondary);
    margin: 0 0 18px;
    max-width: 640px;
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
  .glyph {
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: var(--control-bg);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
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
</style>
