<script lang="ts">
  import { onMount } from "svelte";
  import Dialog from "./Dialog.svelte";
  import { api, errorMessage } from "$lib/api";
  import type { FileEntry, RemoteSummary } from "$lib/api/types";
  import { split } from "$lib/util/path";

  let {
    heading,
    initialRemote,
    onChoose,
    onCancel,
  }: {
    heading: string;
    initialRemote: string;
    onChoose: (remote: string, path: string) => void;
    onCancel: () => void;
  } = $props();

  let remotes = $state<RemoteSummary[]>([]);
  // svelte-ignore state_referenced_locally
  let remote = $state(initialRemote);
  let path = $state("");
  let folders = $state<FileEntry[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(false);

  const crumbs = $derived(crumbsFor(path));

  function crumbsFor(current: string): { label: string; path: string }[] {
    const parts = current.split("/").filter(Boolean);
    return [
      { label: "Top level", path: "" },
      ...parts.map((part, index) => ({ label: part, path: parts.slice(0, index + 1).join("/") })),
    ];
  }

  async function load() {
    loading = true;
    error = null;
    try {
      const entries = await api.listFiles(remote, path);
      folders = entries.filter((entry) => entry.isDir);
    } catch (err) {
      error = errorMessage(err);
      folders = [];
    } finally {
      loading = false;
    }
  }

  function choose(nextRemote: string) {
    remote = nextRemote;
    path = "";
    void load();
  }

  onMount(async () => {
    try {
      remotes = await api.listRemotes();
    } catch (err) {
      error = errorMessage(err);
    }
    void load();
  });

  function up() {
    path = split(path)[0];
    void load();
  }
</script>

<Dialog title={heading} onClose={onCancel}>
  <div class="picker">
    <label class="row">
      <span>Connection</span>
      <select value={remote} onchange={(event) => choose(event.currentTarget.value)}>
        {#each remotes as item (item.name)}
          <option value={item.name}>{item.name}</option>
        {/each}
      </select>
    </label>

    <nav class="crumbs" aria-label="Folder">
      {#each crumbs as crumb, index (crumb.path)}
        <button class="link" onclick={() => { path = crumb.path; void load(); }} disabled={index === crumbs.length - 1}>{crumb.label}</button>
        {#if index < crumbs.length - 1}<span aria-hidden="true">/</span>{/if}
      {/each}
    </nav>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <ul class="folders" aria-busy={loading}>
      {#if path !== ""}
        <li><button class="folder" onclick={up}>↑ Up one level</button></li>
      {/if}
      {#each folders as folder (folder.path)}
        <li>
          <button class="folder" onclick={() => { path = folder.path; void load(); }}>📁 {folder.name}</button>
        </li>
      {/each}
      {#if !loading && folders.length === 0 && !error}
        <li class="empty">No folders here.</li>
      {/if}
    </ul>
  </div>

  {#snippet actions()}
    <button class="button" onclick={onCancel}>Cancel</button>
    <button class="button primary" onclick={() => onChoose(remote, path)} disabled={!remote}>Choose this folder</button>
  {/snippet}
</Dialog>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 220px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }
  select {
    background: var(--control-bg);
    border: none;
    border-radius: var(--radius-control);
    padding: 5px 8px;
  }
  .crumbs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    font-size: 12px;
    color: var(--text-secondary);
    align-items: center;
  }
  .link {
    appearance: none;
    border: none;
    background: none;
    color: var(--accent);
    padding: 0;
    cursor: pointer;
  }
  .link:disabled {
    color: var(--text-primary);
    cursor: default;
  }
  .folders {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--separator);
    border-radius: var(--radius-control);
    max-height: 260px;
    overflow: auto;
  }
  .folder {
    appearance: none;
    width: 100%;
    text-align: left;
    border: none;
    background: none;
    padding: 7px 10px;
    cursor: pointer;
  }
  .folder:hover {
    background: var(--control-bg);
  }
  .empty {
    padding: 10px;
    color: var(--text-secondary);
  }
  .error {
    color: var(--status-disconnected);
  }
</style>
