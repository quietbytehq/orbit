<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { errorMessage } from "$lib/api";
  import type { RcloneLogLevel, Settings, Theme } from "$lib/api/types";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { settings } from "$lib/stores/settings.svelte";

  let pathError = $state<string | null>(null);

  onMount(() => {
    void settings.load();
  });

  function update(change: Partial<Settings>) {
    if (!settings.value) return;
    void settings.save({ ...settings.value, ...change });
  }

  async function browseRclone() {
    pathError = null;
    try {
      const picked = await open({ multiple: false, directory: false });
      if (typeof picked === "string") update({ rclonePath: picked });
    } catch (err) {
      pathError = errorMessage(err);
    }
  }

  const THEMES: { value: Theme; label: string }[] = [
    { value: "system", label: "Match the system" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ];

  const LOG_LEVELS: { value: RcloneLogLevel; label: string }[] = [
    { value: "ERROR", label: "Errors only" },
    { value: "NOTICE", label: "Notices and errors" },
    { value: "INFO", label: "Normal (recommended)" },
    { value: "DEBUG", label: "Debug (large logs)" },
  ];
</script>

<h1>Settings</h1>

{#if settings.error}<p class="error" role="alert">{settings.error}</p>{/if}

{#if settings.value}
  {@const current = settings.value}

  <section class="card">
    <h2>Appearance</h2>
    <fieldset>
      <legend class="sr">Colour scheme</legend>
      {#each THEMES as option (option.value)}
        <label class="option">
          <input type="radio" name="theme" value={option.value} checked={current.theme === option.value} onchange={() => update({ theme: option.value })} />
          <span>{option.label}</span>
        </label>
      {/each}
    </fieldset>
  </section>

  <section class="card">
    <h2>Transfers</h2>
    <label class="row">
      <span>
        <span class="title">Files at the same time</span>
        <span class="help">Each transfer uses bandwidth. Three is a good default for most connections.</span>
      </span>
      <select value={current.transferConcurrency} onchange={(event) => update({ transferConcurrency: Number(event.currentTarget.value) })}>
        {#each [1, 2, 3, 4, 5, 6, 7, 8] as count (count)}
          <option value={count}>{count}</option>
        {/each}
      </select>
    </label>
  </section>

  <section class="card">
    <h2>rclone</h2>
    <p class="help">
      Orbit runs the rclone version it ships with. Change this only if you need a specific build.
    </p>

    <div class="field">
      <span class="title">Custom rclone program</span>
      <span class="path" class:empty={!current.rclonePath}>{current.rclonePath ?? "Using the built-in rclone"}</span>
      <div class="actions">
        <button class="button" onclick={browseRclone}>Choose…</button>
        {#if current.rclonePath}
          <button class="button" onclick={() => update({ rclonePath: null })}>Use built-in</button>
        {/if}
      </div>
      {#if pathError}<p class="error" role="alert">{pathError}</p>{/if}
    </div>

    <label class="row">
      <span>
        <span class="title">Log detail</span>
        <span class="help">Written to the rclone log in your Orbit data folder.</span>
      </span>
      <select value={current.rcloneLogLevel} onchange={(event) => update({ rcloneLogLevel: event.currentTarget.value as RcloneLogLevel })}>
        {#each LOG_LEVELS as level (level.value)}
          <option value={level.value}>{level.label}</option>
        {/each}
      </select>
    </label>

    <div class="actions">
      <span class="help">Status: {rcd.state.state === "healthy" ? `running (${rcd.state.version})` : rcd.state.state}</span>
      <button class="button" onclick={() => rcd.restart()}>Restart rclone</button>
    </div>
  </section>

  <section class="card">
    <h2>Startup</h2>
    <label class="row">
      <span>
        <span class="title">Show the intro animation</span>
        <span class="help">Applies the next time Orbit starts. Click or press a key during the intro to skip it.</span>
      </span>
      <input type="checkbox" checked={current.splashEnabled} onchange={(event) => update({ splashEnabled: event.currentTarget.checked })} />
    </label>
  </section>
{:else}
  <p class="help">Loading settings…</p>
{/if}

<style>
  h1 {
    margin: 0 0 16px;
  }
  .card {
    max-width: 620px;
    margin-bottom: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h2 {
    font-size: 14px;
    font-weight: 600;
    margin: 0;
  }
  fieldset {
    border: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .option {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .title {
    display: block;
    font-weight: 500;
  }
  .help {
    display: block;
    color: var(--text-secondary);
    font-size: 12px;
    margin: 0;
  }
  .path {
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .path.empty {
    color: var(--text-secondary);
  }
  .actions {
    display: flex;
    gap: 10px;
    align-items: center;
    justify-content: flex-end;
    flex-wrap: wrap;
  }
  select {
    background: var(--control-bg);
    border: none;
    border-radius: var(--radius-control);
    padding: 5px 8px;
    flex: none;
  }
  .error {
    color: var(--status-disconnected);
    margin: 0;
  }
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
