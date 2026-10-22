<script lang="ts">
  import { page } from "$app/state";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { open } from "@tauri-apps/plugin-dialog";
  import Dialog from "$lib/components/Dialog.svelte";
  import DestinationPicker from "$lib/components/DestinationPicker.svelte";
  import { api, errorMessage } from "$lib/api";
  import type { Endpoint, FileEntry, TransferRequest } from "$lib/api/types";
  import { transfers } from "$lib/stores/transfers.svelte";
  import { formatBytes } from "$lib/util/format";
  import { isInside, join, normalizePath, split } from "$lib/util/path";

  const remote = $derived(decodeURIComponent(page.params.name ?? ""));
  const path = $derived(normalizePath(page.url.searchParams.get("path") ?? ""));

  let entries = $state<FileEntry[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let selected = $state<string[]>([]);
  let anchor = $state<number | null>(null);
  let dropping = $state(false);
  let busy = $state(false);
  let dialog = $state<"newFolder" | "rename" | "delete" | "copy" | "move" | null>(null);
  let inputName = $state("");

  const crumbs = $derived(breadcrumb(path));
  const selectedEntries = $derived(entries.filter((entry) => selected.includes(entry.path)));

  function breadcrumb(current: string) {
    const parts = current.split("/").filter(Boolean);
    return parts.map((part, index) => ({ label: part, path: parts.slice(0, index + 1).join("/") }));
  }

  function go(next: string) {
    void goto(`/browse/${encodeURIComponent(remote)}?path=${encodeURIComponent(normalizePath(next))}`);
  }

  async function load() {
    loading = true;
    error = null;
    try {
      entries = await api.listFiles(remote, path);
      selected = [];
      anchor = null;
    } catch (err) {
      error = errorMessage(err);
      entries = [];
    } finally {
      loading = false;
    }
  }

  // Reload whenever the connection or folder changes.
  $effect(() => {
    void remote;
    void path;
    void load();
  });

  onMount(() => {
    let stop: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          dropping = true;
        } else if (payload.type === "leave") {
          dropping = false;
        } else if (payload.type === "drop") {
          dropping = false;
          void uploadPaths(payload.paths);
        }
      })
      .then((unlisten) => (stop = unlisten));
    return () => stop?.();
  });

  type Modifiers = Pick<MouseEvent, "shiftKey" | "ctrlKey" | "metaKey">;

  function select(modifiers: Modifiers, index: number) {
    const entry = entries[index];
    if (modifiers.shiftKey && anchor !== null) {
      const [from, to] = [Math.min(anchor, index), Math.max(anchor, index)];
      selected = entries.slice(from, to + 1).map((item) => item.path);
    } else if (modifiers.metaKey || modifiers.ctrlKey) {
      selected = selected.includes(entry.path)
        ? selected.filter((item) => item !== entry.path)
        : [...selected, entry.path];
      anchor = index;
    } else {
      selected = [entry.path];
      anchor = index;
    }
  }

  function openEntry(entry: FileEntry) {
    if (entry.isDir) go(entry.path);
  }

  function onRowKey(event: KeyboardEvent, index: number) {
    if (event.key === "Enter" && selectedEntries.length === 1) {
      openEntry(selectedEntries[0]);
    } else if (event.key === " ") {
      event.preventDefault();
      select(event, index);
    }
  }

  function up() {
    go(split(path)[0]);
  }

  function requestsFor(items: FileEntry[], source: Endpoint, destination: Endpoint, mode: "copy" | "move"): TransferRequest[] {
    return items.map((item) => ({ source, destination, mode, name: item.name, isDir: item.isDir }));
  }

  async function uploadPaths(paths: string[]) {
    if (paths.length === 0) return;
    notice = null;
    try {
      const locals = await api.inspectLocalPaths(paths);
      const requests: TransferRequest[] = locals.map((local) => ({
        source: { kind: "local", path: local.parent },
        destination: { kind: "remote", remote, path },
        mode: "copy",
        name: local.name,
        isDir: local.isDir,
      }));
      if (await transfers.enqueue(requests)) {
        notice = `Uploading ${requests.length === 1 ? requests[0].name : `${requests.length} items`} to ${remote}.`;
      }
    } catch (err) {
      error = errorMessage(err);
    }
  }

  async function pickUpload() {
    const picked = await open({ multiple: true, directory: false });
    if (!picked) return;
    await uploadPaths(Array.isArray(picked) ? picked : [picked]);
  }

  async function download() {
    const folder = await open({ directory: true, multiple: false });
    if (!folder || Array.isArray(folder)) return;
    const requests = requestsFor(
      selectedEntries,
      { kind: "remote", remote, path },
      { kind: "local", path: folder },
      "copy",
    );
    if (await transfers.enqueue(requests)) {
      notice = `Downloading ${requests.length === 1 ? requests[0].name : `${requests.length} items`} to ${folder}.`;
    }
  }

  async function runAndReload(action: () => Promise<unknown>) {
    busy = true;
    error = null;
    try {
      await action();
      dialog = null;
      await load();
    } catch (err) {
      error = errorMessage(err);
    } finally {
      busy = false;
    }
  }

  function startNewFolder() {
    inputName = "";
    dialog = "newFolder";
  }

  function startRename() {
    inputName = selectedEntries[0]?.name ?? "";
    dialog = "rename";
  }

  function newFolder() {
    const name = inputName.trim();
    if (!name) return;
    void runAndReload(() => api.createFolder(remote, join(path, name)));
  }

  function rename() {
    const entry = selectedEntries[0];
    const name = inputName.trim();
    if (!entry || !name) return;
    void runAndReload(() => api.renameItem(remote, entry.path, entry.isDir, name));
  }

  function remove() {
    const items = selectedEntries.map((entry) => ({ path: entry.path, isDir: entry.isDir }));
    void runAndReload(() => api.deleteItems(remote, items));
  }

  /** Copy or move to the chosen folder. Refuses moves that would do nothing or move a folder into itself. */
  async function chooseDestination(mode: "copy" | "move", destRemote: string, destPath: string) {
    const sameFolder = destRemote === remote && normalizePath(destPath) === path;
    if (mode === "move" && sameFolder) {
      error = "Choose a different folder to move these items to.";
      return;
    }
    if (mode === "move" && destRemote === remote && selectedEntries.some((e) => e.isDir && isInside(destPath, e.path))) {
      error = "A folder cannot be moved into itself.";
      return;
    }
    dialog = null;
    error = null;
    const requests = requestsFor(
      selectedEntries,
      { kind: "remote", remote, path },
      { kind: "remote", remote: destRemote, path: destPath },
      mode,
    );
    if (await transfers.enqueue(requests)) {
      notice = `${mode === "copy" ? "Copying" : "Moving"} ${requests.length === 1 ? requests[0].name : `${requests.length} items`} to ${destRemote}.`;
    }
  }

  function formatDate(value: string | null): string {
    if (!value) return "";
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? "" : date.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
  }

  function onKeydown(event: KeyboardEvent) {
    if (dialog || busy) return;
    if (event.key === "Delete" && selectedEntries.length > 0) {
      dialog = "delete";
    } else if (event.key === "Backspace" && path !== "") {
      up();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<header class="toolbar">
  <div class="title">
    <nav class="crumbs" aria-label="Location">
      <a href="/remotes">Remotes</a>
      <span aria-hidden="true">/</span>
      {#if path === ""}
        <span class="current">{remote}</span>
      {:else}
        <button class="link" onclick={() => go("")}>{remote}</button>
        {#each crumbs as crumb, index (crumb.path)}
          <span aria-hidden="true">/</span>
          {#if index === crumbs.length - 1}
            <span class="current">{crumb.label}</span>
          {:else}
            <button class="link" onclick={() => go(crumb.path)}>{crumb.label}</button>
          {/if}
        {/each}
      {/if}
    </nav>
  </div>
  <div class="actions">
    <button class="button icon" onclick={up} disabled={path === ""} aria-label="Up one folder" title="Up one folder">↑</button>
    <button class="button" onclick={() => load()} disabled={loading}>Refresh</button>
    <button class="button" onclick={startNewFolder}>New folder</button>
    <button class="button" onclick={pickUpload}>Upload…</button>
    <button class="button" onclick={download} disabled={selectedEntries.length === 0}>Download…</button>
    <button class="button" onclick={() => (dialog = "copy")} disabled={selectedEntries.length === 0}>Copy to…</button>
    <button class="button" onclick={() => (dialog = "move")} disabled={selectedEntries.length === 0}>Move to…</button>
    <button class="button" onclick={startRename} disabled={selectedEntries.length !== 1}>Rename…</button>
    <button class="button destructive" onclick={() => (dialog = "delete")} disabled={selectedEntries.length === 0}>Delete…</button>
  </div>
</header>

{#if error}<p class="error" role="alert">{error}</p>{/if}
{#if notice}<p class="notice" role="status">{notice}</p>{/if}

<div class="listing" class:dropping aria-busy={loading}>
  {#if dropping}
    <div class="drop-hint">Drop to upload to {path || remote}</div>
  {/if}
  <table>
    <thead>
      <tr>
        <th scope="col">Name</th>
        <th scope="col" class="modified">Modified</th>
        <th scope="col" class="size">Size</th>
      </tr>
    </thead>
    <tbody>
      {#each entries as entry, index (entry.path)}
        <tr
          class:selected={selected.includes(entry.path)}
          tabindex="0"
          onclick={(event) => select(event, index)}
          ondblclick={() => openEntry(entry)}
          onkeydown={(event) => onRowKey(event, index)}
          aria-selected={selected.includes(entry.path)}
        >
          <td class="name"><span class="icon" aria-hidden="true">{entry.isDir ? "📁" : "📄"}</span>{entry.name}</td>
          <td class="modified">{formatDate(entry.modTime)}</td>
          <td class="size">{entry.isDir ? "—" : formatBytes(entry.size)}</td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if !loading && entries.length === 0 && !error}
    <p class="empty">This folder is empty. Drop files here or use Upload.</p>
  {/if}
</div>

{#if dialog === "newFolder"}
  <Dialog title="New folder" onClose={() => (dialog = null)}>
    <form onsubmit={(event) => { event.preventDefault(); newFolder(); }}>
      <input class="input" bind:value={inputName} placeholder="Folder name" aria-label="Folder name" />
      <div class="dialog-actions">
        <button type="button" class="button" onclick={() => (dialog = null)}>Cancel</button>
        <button type="submit" class="button primary" disabled={busy || !inputName.trim()}>Create</button>
      </div>
    </form>
  </Dialog>
{:else if dialog === "rename" && selectedEntries[0]}
  <Dialog title="Rename" onClose={() => (dialog = null)}>
    <form onsubmit={(event) => { event.preventDefault(); rename(); }}>
      <input class="input" bind:value={inputName} aria-label="New name" />
      <div class="dialog-actions">
        <button type="button" class="button" onclick={() => (dialog = null)}>Cancel</button>
        <button type="submit" class="button primary" disabled={busy || !inputName.trim()}>Rename</button>
      </div>
    </form>
  </Dialog>
{:else if dialog === "delete"}
  <Dialog title={`Delete ${selectedEntries.length === 1 ? selectedEntries[0].name : `${selectedEntries.length} items`}?`} onClose={() => (dialog = null)}>
    <p>This is permanent. Folders are deleted with everything inside them, and nothing goes to a trash.</p>
    <div class="dialog-actions">
      <button class="button" onclick={() => (dialog = null)}>Cancel</button>
      <button class="button destructive-fill" onclick={remove} disabled={busy}>Delete permanently</button>
    </div>
  </Dialog>
{:else if dialog === "copy" || dialog === "move"}
  <DestinationPicker
    heading={dialog === "copy" ? "Copy to" : "Move to"}
    initialRemote={remote}
    onChoose={(destRemote, destPath) => chooseDestination(dialog === "copy" ? "copy" : "move", destRemote, destPath)}
    onCancel={() => (dialog = null)}
  />
{/if}

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .title {
    min-width: 0;
  }
  .crumbs {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 15px;
    flex-wrap: wrap;
  }
  .crumbs a {
    color: var(--accent);
    text-decoration: none;
  }
  .current {
    font-weight: 600;
  }
  .link {
    appearance: none;
    border: none;
    background: none;
    color: var(--accent);
    padding: 0;
    cursor: pointer;
    font: inherit;
  }
  .actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .button.icon {
    min-width: 32px;
  }
  .error {
    color: var(--status-disconnected);
  }
  .notice {
    color: var(--text-secondary);
  }
  .listing {
    position: relative;
    background: var(--panel-bg);
    border-radius: var(--radius-card);
    box-shadow: var(--shadow-card);
    overflow: hidden;
    min-height: 240px;
  }
  .listing.dropping {
    outline: 2px dashed var(--accent);
    outline-offset: -6px;
  }
  .drop-hint {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(10, 132, 255, 0.08);
    color: var(--accent);
    font-weight: 600;
    z-index: 1;
    pointer-events: none;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th {
    text-align: left;
    font-weight: 500;
    color: var(--text-secondary);
    font-size: 12px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--separator);
  }
  td {
    padding: 7px 12px;
    white-space: nowrap;
  }
  tbody tr {
    cursor: default;
  }
  tbody tr:hover {
    background: var(--control-bg);
  }
  tbody tr.selected {
    background: var(--sidebar-selected);
  }
  tbody tr:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .name {
    width: 100%;
    max-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name .icon {
    margin-right: 8px;
  }
  .modified {
    color: var(--text-secondary);
    width: 1%;
  }
  .size {
    text-align: right;
    color: var(--text-secondary);
    width: 1%;
  }
  .empty {
    text-align: center;
    color: var(--text-secondary);
    padding: 40px 12px;
  }
  .input {
    width: 100%;
    background: var(--control-bg);
    border: none;
    border-radius: var(--radius-control);
    padding: 7px 10px;
    margin-bottom: 14px;
    user-select: text;
  }
  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
  :global(.button.destructive) {
    color: var(--status-disconnected);
  }
  :global(.button.destructive-fill) {
    background: var(--status-disconnected);
    color: #fff;
    border: none;
    border-radius: var(--radius-control);
    padding: 6px 14px;
    cursor: pointer;
    font-weight: 500;
  }
</style>
