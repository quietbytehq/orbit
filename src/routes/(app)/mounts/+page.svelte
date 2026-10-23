<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import Dialog from "$lib/components/Dialog.svelte";
  import ErrorNote from "$lib/components/ErrorNote.svelte";
  import { api, asAppError } from "$lib/api";
  import type { AppError, MountInfo, MountSupport, RemoteSummary } from "$lib/api/types";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { text } from "$lib/text/index.svelte";
  import { Eject, HardDrive, ICON, ICON_LG, RefreshCw, providerIcon } from "$lib/icons";

  const WINFSP_PAGE = "https://winfsp.dev/rel/";

  let support = $state<MountSupport | null>(null);
  let mounts = $state<MountInfo[]>([]);
  let remotes = $state<RemoteSummary[]>([]);
  let error = $state<AppError | null>(null);
  let busy = $state(false);
  let mounting = $state<RemoteSummary | null>(null);
  let location = $state("");

  async function load() {
    error = null;
    try {
      [support, remotes] = await Promise.all([api.mountSupport(), api.listRemotes()]);
      mounts = rcd.state.state === "healthy" ? await api.listMounts() : [];
    } catch (err) {
      error = asAppError(err);
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
      error = asAppError(err);
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
      error = asAppError(err);
    }
  }

  const mountedNames = $derived(new Set(mounts.map((mount) => mount.remote)));
</script>

<h1>{text.s.drives.title}</h1>
<p class="help">{text.s.drives.intro}</p>

{#if support && !support.available}
  <section class="card notice-card">
    <h2>{text.s.drives.needsInstallTitle}</h2>
    <p>{support.requirement}</p>
    <div class="actions">
      {#if support.platform === "windows"}
        <button class="button" onclick={() => openUrl(WINFSP_PAGE)}>{text.s.drives.getWinFsp}</button>
      {/if}
      <button class="button with-icon" onclick={load}><RefreshCw {...ICON} />{text.s.drives.checkAgain}</button>
    </div>
  </section>
{/if}

{#if rcd.state.state !== "healthy"}
  <p class="notice">{text.s.common.engineDown}</p>
{/if}
<ErrorNote {error} />

<section class="section">
  <h2>{text.s.drives.connectedNow}</h2>
  {#if mounts.length === 0}
    <p class="help">{text.s.drives.nothingConnected}</p>
  {:else}
    <ul class="list">
      {#each mounts as mount (mount.mountPoint)}
        <li class="card row">
          <span class="glyph"><HardDrive {...ICON_LG} /></span>
          <div>
            <span class="name">{mount.remote}</span>
            <span class="place">{mount.mountPoint}</span>
          </div>
          <button class="button with-icon" onclick={() => unmount(mount)}><Eject {...ICON} />{text.s.drives.remove}</button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<section class="section">
  <h2>{text.s.drives.available}</h2>
  {#if remotes.length === 0}
    <p class="help">{text.s.drives.addStorageFirst}</p>
  {:else}
    <ul class="list">
      {#each remotes as remote (remote.name)}
        {@const Icon = providerIcon(remote.providerId)}
        <li class="card row">
          <span class="glyph"><Icon {...ICON_LG} /></span>
          <div>
            <span class="name">{remote.name}</span>
            <span class="place">{remote.providerTitle}</span>
          </div>
          {#if mountedNames.has(remote.name)}
            <span class="place">{text.s.drives.connected}</span>
          {:else}
            <button class="button with-icon" onclick={() => startMount(remote)} disabled={!support?.available || rcd.state.state !== "healthy"}>
              <HardDrive {...ICON} />{text.s.drives.show}
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

{#if mounting}
  <Dialog title={text.s.drives.dialogTitle(mounting.name)} onClose={() => (mounting = null)}>
    <form
      onsubmit={(event) => {
        event.preventDefault();
        void confirmMount();
      }}
    >
      <label class="label" for="mount-location">{text.s.drives.locationLabel}</label>
      <input id="mount-location" class="input" bind:value={location} disabled={busy} />
      <p class="help">{text.s.drives.locationHelp}</p>
      <div class="dialog-actions">
        <button type="button" class="button" onclick={() => (mounting = null)} disabled={busy}>{text.s.common.cancel}</button>
        <button type="submit" class="button primary" disabled={busy || !location.trim()}>
          {busy ? text.s.drives.connecting : text.s.drives.connect}
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
    flex: 1;
  }
  .glyph {
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: 10px;
    background: var(--control-bg);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
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
  .notice {
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
