<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import DestinationPicker from "./DestinationPicker.svelte";
  import type { Endpoint } from "$lib/api/types";
  import { endpointLabel } from "$lib/util/endpoint";
  import { errorMessage } from "$lib/api";

  let {
    label,
    value,
    onChange,
    disabled = false,
  }: {
    label: string;
    value: Endpoint | null;
    onChange: (endpoint: Endpoint) => void;
    disabled?: boolean;
  } = $props();

  let picking = $state(false);
  let error = $state<string | null>(null);

  async function chooseLocalFolder() {
    error = null;
    try {
      const folder = await open({ directory: true, multiple: false });
      if (typeof folder === "string") onChange({ kind: "local", path: folder });
    } catch (err) {
      error = errorMessage(err);
    }
  }
</script>

<div class="endpoint">
  <span class="label">{label}</span>
  <span class="value" class:empty={value === null}>{value ? endpointLabel(value) : "Not chosen yet"}</span>
  <div class="buttons">
    <button type="button" class="button" onclick={() => (picking = true)} {disabled}>Remote folder…</button>
    <button type="button" class="button" onclick={chooseLocalFolder} {disabled}>This computer…</button>
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</div>

{#if picking}
  <DestinationPicker
    heading={`Choose ${label.toLowerCase()}`}
    initialRemote={value?.kind === "remote" ? value.remote : ""}
    onChoose={(remote, path) => {
      onChange({ kind: "remote", remote, path });
      picking = false;
    }}
    onCancel={() => (picking = false)}
  />
{/if}

<style>
  .endpoint {
    display: grid;
    grid-template-columns: 90px 1fr;
    gap: 6px 12px;
    align-items: center;
    padding: 10px 0;
    border-bottom: 1px solid var(--separator);
  }
  .label {
    color: var(--text-secondary);
    font-weight: 500;
  }
  .value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .value.empty {
    color: var(--text-secondary);
    font-style: italic;
  }
  .buttons {
    grid-column: 2;
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .error {
    grid-column: 2;
    color: var(--status-disconnected);
    margin: 0;
  }
</style>
