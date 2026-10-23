<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import ErrorNote from "$lib/components/ErrorNote.svelte";
  import { asAppError } from "$lib/api";
  import type { AppError, RcloneLogLevel, Settings, SplashMode, Theme } from "$lib/api/types";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { text } from "$lib/text/index.svelte";
  import { ICON, RefreshCw } from "$lib/icons";

  let pathError = $state<AppError | null>(null);

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
      pathError = asAppError(err);
    }
  }

  const THEMES: Theme[] = ["system", "light", "dark"];
  const SPLASH_MODES: SplashMode[] = ["full", "short", "off"];
  const LOG_LEVELS: RcloneLogLevel[] = ["ERROR", "NOTICE", "INFO", "DEBUG"];

  const engineStatus = $derived(
    rcd.state.state === "healthy"
      ? text.s.settings.engineRunning(rcd.state.version)
      : rcd.state.state === "starting"
        ? text.s.overview.starting
        : rcd.state.state === "restarting"
          ? text.s.overview.restarting(rcd.state.attempt)
          : rcd.state.state === "failed"
            ? rcd.state.message
            : text.s.overview.stopped,
  );
</script>

<h1>{text.s.settings.title}</h1>

<ErrorNote error={settings.error} />

{#if settings.value}
  {@const current = settings.value}

  <section class="card">
    <h2>{text.s.settings.appearance}</h2>
    <fieldset>
      <legend class="sr">{text.s.settings.colourScheme}</legend>
      {#each THEMES as option (option)}
        <label class="option">
          <input type="radio" name="theme" value={option} checked={current.theme === option} onchange={() => update({ theme: option })} />
          <span>{text.s.settings.themes[option]}</span>
        </label>
      {/each}
    </fieldset>
  </section>

  <section class="card">
    <h2>{text.s.settings.transfers}</h2>
    <label class="row">
      <span>
        <span class="title">{text.s.settings.concurrency}</span>
        <span class="help">{text.s.settings.concurrencyHelp}</span>
      </span>
      <select value={current.transferConcurrency} onchange={(event) => update({ transferConcurrency: Number(event.currentTarget.value) })}>
        {#each [1, 2, 3, 4, 5, 6, 7, 8] as count (count)}
          <option value={count}>{count}</option>
        {/each}
      </select>
    </label>
  </section>

  <section class="card">
    <h2>{text.s.settings.startup}</h2>
    <label class="row">
      <span>
        <span class="title">{text.s.settings.startOnLogin}</span>
        <span class="help">{text.s.settings.startOnLoginHelp}</span>
      </span>
      <input type="checkbox" checked={current.startOnLogin} onchange={(event) => update({ startOnLogin: event.currentTarget.checked })} />
    </label>
    <fieldset>
      <legend class="title">{text.s.settings.intro}</legend>
      <p class="help">{text.s.settings.introHelp}</p>
      {#each SPLASH_MODES as option (option)}
        <label class="option">
          <input type="radio" name="splash" value={option} checked={current.splash === option} onchange={() => update({ splash: option })} />
          <span>{text.s.settings.splash[option]} <span class="help inline">{text.s.settings.splash[`${option}Help`]}</span></span>
        </label>
      {/each}
    </fieldset>
  </section>

  <section class="card">
    <h2>{text.s.settings.advanced}</h2>
    <p class="help">{text.s.settings.engineIntro}</p>

    <div class="field">
      <span class="title">{text.s.settings.enginePath}</span>
      <span class="path" class:empty={!current.rclonePath}>{current.rclonePath ?? text.s.settings.builtIn}</span>
      <div class="actions">
        <button class="button" onclick={browseRclone}>{text.s.settings.choose}</button>
        {#if current.rclonePath}
          <button class="button" onclick={() => update({ rclonePath: null })}>{text.s.settings.useBuiltIn}</button>
        {/if}
      </div>
      <ErrorNote error={pathError} />
    </div>

    <label class="row">
      <span>
        <span class="title">{text.s.settings.logDetail}</span>
        <span class="help">{text.s.settings.logHelp}</span>
      </span>
      <select value={current.rcloneLogLevel} onchange={(event) => update({ rcloneLogLevel: event.currentTarget.value as RcloneLogLevel })}>
        {#each LOG_LEVELS as level (level)}
          <option value={level}>{text.s.settings.logLevels[level]}</option>
        {/each}
      </select>
    </label>

    <div class="actions">
      <span class="help">{text.s.settings.engineStatus(engineStatus)}</span>
      <button class="button with-icon" onclick={() => rcd.restart()}><RefreshCw {...ICON} />{text.s.settings.restartEngine}</button>
    </div>
  </section>
{:else}
  <p class="help">{text.s.settings.loading}</p>
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
  .help.inline {
    display: inline;
    margin-left: 6px;
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
  .sr {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
