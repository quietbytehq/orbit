<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Dialog from "$lib/components/Dialog.svelte";
  import { api, errorMessage } from "$lib/api";
  import type { MountInfo, MountSupport, RemoteSummary } from "$lib/api/types";
  import { rcd } from "$lib/stores/rcd.svelte";

  const WINFSP_PAGE = "https://winfsp.dev/rel/";

  let support = $state<MountSupport | null>(null);
  let mounts = $state<MountInfo[]>([]);
  let remotes = $state<RemoteSummary[]>([]);
  let error = $state<string | null>(null);
  let busy = $state(false);
  let mounting = $state<RemoteSummary | null>(null);
  let location = $state("");

  async function load() {
    error = null;
    try {
      [support, remotes] = await Promise.all([api.mountSupport(), api.listRemotes()]);
      mounts = rcd.state.state === "healthy" ? await api.listMounts() : [];
    } catch (err) {
      error = errorMessage(err);
    }
  }

  onMount(() => {
    void load();
  });

  $effect(() => {
    if (rcd.state.state === "healthy") void load();
  });

  async function startMount(remote: RemoteSummary) {
    mounting = remote;
    error = null;
    location = await api.suggestMountLocation(remote.name).catch(() => "");
  }

  async function confirmMount() {
    if (!mounting) return;
    busy = true;
    error = null;
    try {
      await api.mountRemote(mounting.name, location.trim());
      mounting = null;
      await load();
    } catch (err) {
      error = errorMessage(err);
    } finally {
      busy = false;
    }
  }

  async function unmount(mount: MountInfo) {
    error = null;
    try {
      await api.unmountRemote(mount.mountPoint);
      await load();
    } catch (err) {
      error = errorMessage(err);
    }
  }

  const mountedNames = $derived(new Set(mounts.map((mount) => mount.remote)));
</script>

<h1>Mounts</h1>
<p class="help">A mounted remote appears as a folder{support?.platform === "windows" ? " with a drive letter" : ""} on this computer. Mounts end when Orbit quits.</p>

{#if support && !support.available}
  <section class="card notice-card">
    <h2>Mounting needs one more install</h2>
    <p>{support.requirement}</p>
    <div class="actions">
      {#if support.platform === "windows"}
        <button class="button" onclick={() => openUrl(WINFSP_PAGE)}>Get WinFsp</button>
      {/if}
      <button class="button" onclick={load}>Check again</button>
    </div>
  </section>
{/if}

{#if rcd.state.state !== "healthy"}
  <p class="notice">Orbit cannot reach rclone right now.</p>
{/if}
{#if error}<p class="error" role="alert">{error}</p>{/if}

<section class="section">
  <h2>Mounted now</h2>
  {#if mounts.length === 0}
    <p class="help">Nothing is mounted.</p>
  {:else}
    <ul class="list">
      {#each mounts as mount (mount.mountPoint)}
        <li class="card row">
          <div>
            <span class="name">{mount.remote}</span>
            <span class="place">{mount.mountPoint}</span>
          </div>
          <button class="button" onclick={() => unmount(mount)}>Unmount</button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section class="section">
  <h2>Mount a remote</h2>
  {#if remotes.length === 0}
    <p class="help">Add a remote first, then mount it here.</p>
  {:else}
    <ul class="list">
      {#each remotes as remote (remote.name)}
        <li class="card row">
          <div>
            <span class="name">{remote.name}</span>
            <span class="place">{remote.providerTitle}</span>
          </div>
          {#if mountedNames.has(remote.name)}
            <span class="place">Mounted</span>
          {:else}
            <button class="button" onclick={() => startMount(remote)} disabled={!support?.available || rcd.state.state !== "healthy"}>
              Mount…
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if mounting}
  <Dialog title={`Mount ${mounting.name}`} onClose={() => (mounting = null)}>
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void confirmMount();
      }}
    >
      <label class="label" for="mount-location">
        {support?.platform === "windows" ? "Drive letter" : "Folder"}
      </label>
      <input id="mount-location" class="input" bind:value={location} disabled={busy} />
      <p class="help">
        {support?.platform === "windows"
          ? "Pick a free drive letter. The remote will appear in File Explorer under it."
          : "The folder is created if it does not exist. It must be empty."}
      </p>
      <div class="dialog-actions">
        <button type="button" class="button" onclick={() => (mounting = null)} disabled={busy}>Cancel</button>
        <button type="submit" class="button primary" disabled={busy || !location.trim()}>
          {busy ? "Mounting…" : "Mount"}
        </button>
      </div>
    </form>
  </Dialog>
{/if}

<style>
  h1 {
    margin: 0 0 6px;
  }
  h2 {
    font-size: 14px;
    font-weight: 600;
    margin: 0 0 10px;
  }
  .help,
  .place {
    color: var(--text-secondary);
    font-size: 12px;
    margin: 0;
  }
  .section {
    margin-top: 22px;
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 680px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .row > div {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .name {
    font-weight: 600;
  }
  .place {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .notice-card {
    max-width: 680px;
    margin: 12px 0 0;
  }
  .notice-card h2 {
    margin-bottom: 6px;
  }
  .notice-card p {
    margin: 0 0 12px;
    color: var(--text-secondary);
  }
  .actions,
  .dialog-actions {
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }
  .notice,
  .error {
    color: var(--status-disconnected);
  }
  .label {
    font-weight: 500;
    display: block;
    margin-bottom: 4px;
  }
  .input {
    width: 100%;
    background: var(--control-bg);
    border: none;
    border-radius: var(--radius-control);
    padding: 7px 10px;
    margin-bottom: 8px;
    user-select: text;
  }
</style>
