<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import ErrorNote from "$lib/components/ErrorNote.svelte";
  import RemoteWizard from "$lib/components/RemoteWizard.svelte";
  import { api, asAppError } from "$lib/api";
  import type { AppError, ProviderDef, RemoteForm } from "$lib/api/types";
  import { text } from "$lib/text/index.svelte";

  let form = $state<RemoteForm | null>(null);
  let provider = $state<ProviderDef | null>(null);
  let error = $state<AppError | null>(null);
  let confirmingDelete = $state(false);
  let deleting = $state(false);

  const name = $derived(decodeURIComponent(page.params.name ?? ""));

  onMount(async () => {
    try {
      const [loaded, providers] = await Promise.all([api.getRemoteForm(name), api.listProviders()]);
      form = loaded;
      provider = providers.find((p) => p.id === loaded.providerId) ?? null;
    } catch (err) {
      error = asAppError(err);
    }
  });

  async function remove() {
    deleting = true;
    error = null;
    try {
      await api.deleteRemote(name);
      await goto("/remotes");
    } catch (err) {
      error = asAppError(err);
      deleting = false;
    }
  }
</script>

<h1>{name}</h1>

<ErrorNote {error} />

{#if form && provider}
  <RemoteWizard {provider} existing={form} />

  <section class="danger">
    {#if confirmingDelete}
      <p>
        <strong>{text.s.storage.removeTitle(name)}</strong>
        {text.s.storage.removeBody}
      </p>
      <div class="actions">
        <button class="button" onclick={() => (confirmingDelete = false)} disabled={deleting}>{text.s.common.cancel}</button>
        <button class="button destructive" onclick={remove} disabled={deleting}>
          {deleting ? text.s.storage.removing : text.s.storage.remove}
        </button>
      </div>
    {:else}
      <button class="button destructive" onclick={() => (confirmingDelete = true)}>{text.s.storage.removeEllipsis}</button>
    {/if}
  </section>
{:else if !error}
  <p class="help">{text.s.common.loading}</p>
{/if}

<style>
  .help {
    color: var(--text-secondary);
  }
  .danger {
    margin-top: 28px;
    max-width: 560px;
    border-top: 1px solid var(--separator);
    padding-top: 16px;
  }
  .danger p {
    margin: 0 0 12px;
    color: var(--text-secondary);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
  }
</style>
