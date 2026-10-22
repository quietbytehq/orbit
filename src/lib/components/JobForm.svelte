<script lang="ts">
  import Dialog from "./Dialog.svelte";
  import EndpointField from "./EndpointField.svelte";
  import { api, errorMessage } from "$lib/api";
  import type { Endpoint, JobInput, JobKind, JobView } from "$lib/api/types";

  let {
    existing = null,
    onSaved,
    onCancel,
  }: {
    existing?: JobView | null;
    onSaved: (view: JobView) => void;
    onCancel: () => void;
  } = $props();

  // The form is opened fresh for each job, so the initial values are read once on purpose.
  // svelte-ignore state_referenced_locally
  let name = $state(existing?.name ?? "");
  // svelte-ignore state_referenced_locally
  let kind = $state<JobKind>(existing?.kind ?? "copy");
  // svelte-ignore state_referenced_locally
  let source = $state<Endpoint | null>(existing?.source ?? null);
  // svelte-ignore state_referenced_locally
  let destination = $state<Endpoint | null>(existing?.destination ?? null);
  let busy = $state(false);
  let error = $state<string | null>(null);

  const KIND_TEXT: Record<JobKind, { title: string; help: string }> = {
    copy: {
      title: "Copy",
      help: "Adds new and changed files to the destination. Nothing is deleted.",
    },
    sync: {
      title: "Sync",
      help: "Makes the destination match the source. Files in the destination that are not in the source are deleted.",
    },
  };

  async function save() {
    if (!source || !destination) {
      error = "Choose both the source and the destination.";
      return;
    }
    busy = true;
    error = null;
    const input: JobInput = { name, kind, source, destination };
    try {
      const view = existing ? await api.updateJob(existing.id, input) : await api.createJob(input);
      onSaved(view);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      busy = false;
    }
  }
</script>

<Dialog title={existing ? "Edit job" : "New job"} onClose={onCancel}>
  <form
    class="form"
    onsubmit={(event) => {
      event.preventDefault();
      void save();
    }}
  >
    <div class="field">
      <label for="job-name">Name</label>
      <input id="job-name" bind:value={name} placeholder="for example, Photos backup" disabled={busy} />
    </div>

    <fieldset class="kinds" disabled={busy}>
      <legend>What should happen</legend>
      {#each ["copy", "sync"] as const as option (option)}
        <label class="kind" class:selected={kind === option}>
          <input type="radio" name="kind" value={option} bind:group={kind} />
          <span class="kind-title">{KIND_TEXT[option].title}</span>
          <span class="kind-help">{KIND_TEXT[option].help}</span>
        </label>
      {/each}
    </fieldset>

    {#if kind === "sync"}
      <p class="warning" role="note">
        Sync deletes files in the destination that are missing from the source. Check the destination before you run it.
      </p>
    {/if}

    <EndpointField label="Source" value={source} onChange={(endpoint) => (source = endpoint)} disabled={busy} />
    <EndpointField label="Destination" value={destination} onChange={(endpoint) => (destination = endpoint)} disabled={busy} />

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="actions">
      <button type="button" class="button" onclick={onCancel} disabled={busy}>Cancel</button>
      <button type="submit" class="button primary" disabled={busy || !name.trim()}>
        {busy ? "Saving…" : existing ? "Save changes" : "Create job"}
      </button>
    </div>
  </form>
</Dialog>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  label {
    font-weight: 500;
  }
  input:not([type]) {
    background: var(--control-bg);
    border: none;
    border-radius: var(--radius-control);
    padding: 6px 9px;
    user-select: text;
  }
  .kinds {
    border: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  legend {
    font-weight: 500;
    margin-bottom: 6px;
  }
  .kind {
    display: grid;
    grid-template-columns: auto auto 1fr;
    gap: 2px 8px;
    padding: 8px 10px;
    border-radius: var(--radius-control);
    background: var(--control-bg);
    cursor: pointer;
    font-weight: normal;
  }
  .kind.selected {
    background: var(--sidebar-selected);
  }
  .kind input {
    grid-row: span 2;
    margin: 3px 0 0;
  }
  .kind-title {
    font-weight: 600;
  }
  .kind-help {
    grid-column: 2 / 4;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .warning {
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--radius-control);
    background: rgba(255, 149, 0, 0.14);
    color: var(--text-primary);
    font-size: 12px;
  }
  .error {
    color: var(--status-disconnected);
    margin: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 4px;
  }
</style>
