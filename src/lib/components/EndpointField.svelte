<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import DestinationPicker from "./DestinationPicker.svelte";
  import type { Endpoint } from "$lib/api/types";
  import { endpointLabel } from "$lib/util/endpoint";
  import { asAppError } from "$lib/api";
  import type { AppError } from "$lib/api/types";
  import { text } from "$lib/text/index.svelte";
  import ErrorNote from "./ErrorNote.svelte";

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
  let error = $state<AppError | null>(null);

  async function chooseLocalFolder() {
    error = null;
    try {
      const folder = await open({ directory: true, multiple: false });
      if (typeof folder === "string") onChange({ kind: "local", path: folder });
    } catch (err) {
      error = asAppError(err);
    }
  }
</script>

<div class="endpoint">
  <span class="label">{label}</span>
  <span class="value" class:empty={value === null}>{value ? endpointLabel(value) : text.s.picker.notChosen}</span>
  <div class="buttons">
    <button type="button" class="button" onclick={() => (picking = true)} {disabled}>{text.s.picker.cloudFolder}</button>
    <button type="button" class="button" onclick={chooseLocalFolder} {disabled}>{text.s.picker.thisComputer}</button>
  </div>
  <div class="note"><ErrorNote {error} /></div>
</div>

{#if picking}
  <DestinationPicker
    heading={text.s.picker.chooseLabel(label)}
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
  .note {
    grid-column: 2;
  }
  .note:empty {
    display: none;
  }
</style>
