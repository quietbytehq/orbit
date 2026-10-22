<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import RemoteWizard from "$lib/components/RemoteWizard.svelte";
  import { api, errorMessage } from "$lib/api";
  import type { ProviderDef, RemoteForm } from "$lib/api/types";

  let form = $state<RemoteForm | null>(null);
  let provider = $state<ProviderDef | null>(null);
  let error = $state<string | null>(null);
  let confirmingDelete = $state(false);
  let deleting = $state(false);

  const name = $derived(decodeURIComponent(page.params.name ?? ""));

  onMount(async () => {
    try {
      const [loaded, providers] = await Promise.all([api.getRemoteForm(name), api.listProviders()]);
      form = loaded;
      provider = providers.find((p) => p.id === loaded.providerId) ?? null;
    } catch (err) {
      error = errorMessage(err);
    }
  });

  async function remove() {
    deleting = true;
    error = null;
    try {
      await api.deleteRemote(name);
      await goto("/remotes");
    } catch (err) {
      error = errorMessage(err);
      deleting = false;
    }
  }
</script>

<h1>{name}</h1>

{#if error}<p class="error" role="alert">{error}</p>{/if}

{#if form && provider}
  <RemoteWizard {provider} existing={form} />

  <section class="danger">
    {#if confirmingDelete}
      <p>
        Remove <strong>{name}</strong> from Orbit? Its settings and sign-in are deleted from rclone.
        Files on the service are not touched.
      </p>
      <div class="actions">
        <button class="button" onclick={() => (confirmingDelete = false)} disabled={deleting}>Cancel</button>
        <button class="button destructive" onclick={remove} disabled={deleting}>
          {deleting ? "Removing…" : "Remove connection"}
        </button>
      </div>
    {:else}
      <button class="button destructive" onclick={() => (confirmingDelete = true)}>Remove connection…</button>
    {/if}
  </section>
{:else if !error}
  <p class="help">Loading…</p>
{/if}

<style>
  .error {
    color: var(--status-disconnected);
  }
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
  :global(.button.destructive) {
    color: var(--status-disconnected);
  }
</style>
