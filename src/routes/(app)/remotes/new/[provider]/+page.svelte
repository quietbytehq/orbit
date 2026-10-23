<script lang="ts">
  import { page } from "$app/state";
  import { onMount } from "svelte";
  import ErrorNote from "$lib/components/ErrorNote.svelte";
  import RemoteWizard from "$lib/components/RemoteWizard.svelte";
  import { api, asAppError } from "$lib/api";
  import type { AppError, ProviderDef } from "$lib/api/types";
  import { text } from "$lib/text/index.svelte";

  let provider = $state<ProviderDef | null>(null);
  let error = $state<AppError | null>(null);

  onMount(async () => {
    try {
      provider = (await api.listProviders()).find((p) => p.id === page.params.provider) ?? null;
    } catch (err) {
      error = asAppError(err);
    }
  });
</script>

{#if provider}
  <h1>{provider.title}</h1>
  <RemoteWizard {provider} />
{:else if error}
  <ErrorNote {error} />
{:else}
  <p class="help">{text.s.common.loading}</p>
{/if}

<style>
  .help {
    color: var(--text-secondary);
  }
</style>
