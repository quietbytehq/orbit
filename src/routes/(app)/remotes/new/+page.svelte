<script lang="ts">
  import { onMount } from "svelte";
  import ErrorNote from "$lib/components/ErrorNote.svelte";
  import { api, asAppError } from "$lib/api";
  import type { AppError, ProviderDef } from "$lib/api/types";
  import { text } from "$lib/text/index.svelte";
  import { ICON_LG, providerIcon } from "$lib/icons";

  let providers = $state<ProviderDef[]>([]);
  let error = $state<AppError | null>(null);

  onMount(async () => {
    try {
      providers = await api.listProviders();
    } catch (err) {
      error = asAppError(err);
    }
  });
</script>

<h1>{text.s.wizard.title}</h1>
<p class="help">{text.s.wizard.intro}</p>

<ErrorNote {error} />

<div class="grid">
  {#each providers as provider (provider.id)}
    {@const Icon = providerIcon(provider.id)}
    <a class="card provider" href={`/remotes/new/${provider.id}`}>
      <span class="glyph"><Icon {...ICON_LG} /></span>
      <span class="title">{provider.title}</span>
      <span class="tagline">{provider.tagline}</span>
      <span class="auth">{provider.auth === "browser" ? text.s.wizard.authBrowser : text.s.wizard.authCredentials}</span>
    </a>
  {/each}
</div>

<style>
  .help {
    color: var(--text-secondary);
    margin: -12px 0 20px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 12px;
  }
  .provider {
    display: flex;
    flex-direction: column;
    gap: 6px;
    text-decoration: none;
    color: inherit;
    transition: transform 0.12s ease;
  }
  .provider:hover {
    transform: translateY(-1px);
  }
  .glyph {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: var(--control-bg);
    color: var(--accent);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    margin-bottom: 4px;
  }
  .title {
    font-weight: 600;
    font-size: 14px;
  }
  .tagline {
    color: var(--text-secondary);
    font-size: 12px;
  }
  .auth {
    margin-top: auto;
    font-size: 11px;
    color: var(--accent);
    font-weight: 500;
  }
</style>
