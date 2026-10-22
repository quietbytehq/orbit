<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import RemoteWizard from "$lib/components/RemoteWizard.svelte";
  import { api, errorMessage } from "$lib/api";
  import type { ProviderDef } from "$lib/api/types";

  let provider = $state<ProviderDef | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      provider = (await api.listProviders()).find((p) => p.id === page.params.provider) ?? null;
    } catch (err) {
      error = errorMessage(err);
    }
  });
</script>

{#if provider}
  <h1>Connect {provider.title}</h1>
  <RemoteWizard {provider} />
{:else if error}
  <p class="error" role="alert">{error}</p>
{:else}
  <p class="help">Loading…</p>
{/if}

<style>
  .error {
    color: var(--status-disconnected);
  }
  .help {
    color: var(--text-secondary);
  }
</style>
