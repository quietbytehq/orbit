<script lang="ts">
  import { goto } from "$app/navigation";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, asAppError, onSignInUrl } from "$lib/api";
  import type { AppError, ConnectionTest, ProviderDef, RemoteForm } from "$lib/api/types";
  import { text } from "$lib/text/index.svelte";
  import { isValidRemoteName } from "$lib/util/remote";
  import ErrorNote from "./ErrorNote.svelte";
  import FieldInput from "./FieldInput.svelte";

  let { provider, existing = null }: { provider: ProviderDef; existing?: RemoteForm | null } = $props();

  // Each route mounts its own wizard, so reading the props once for the initial state is intended.
  // svelte-ignore state_referenced_locally
  const editing = existing !== null;

  type Step = "name" | "details" | "sign-in" | "result";

  // Edit mode skips straight to the form. Create mode starts with naming the connection.
  // svelte-ignore state_referenced_locally
  let step = $state<Step>(editing ? "details" : "name");
  // svelte-ignore state_referenced_locally
  let name = $state(existing?.name ?? "");
  // svelte-ignore state_referenced_locally
  let values = $state<Record<string, string>>(
    Object.fromEntries(
      provider.fields.map((field) => [field.key, existing?.values[field.key] ?? field.default ?? ""]),
    ),
  );
  let busy = $state(false);
  let error = $state<AppError | string | null>(null);
  let signInUrl = $state<string | null>(null);
  let result = $state<ConnectionTest | null>(null);
  let savedName = $state<string | null>(null);

  // svelte-ignore state_referenced_locally
  const isBrowser = provider.auth === "browser";
  const nameValid = $derived(isValidRemoteName(name));
  const savedSecrets = $derived(existing?.savedSecrets ?? []);

  // rclone opens its own callback server, so we only need to show and open the sign-in URL it reports.
  $effect(() => {
    if (step !== "sign-in") return;
    let unlisten: (() => void) | undefined;
    onSignInUrl((url) => {
      signInUrl = url;
      openUrl(url).catch(() => {
        // The URL is still shown on screen, so the user can copy it.
      });
    }).then((stop) => (unlisten = stop));
    return () => unlisten?.();
  });

  function nextFromName() {
    error = null;
    if (!nameValid) {
      error = text.s.wizard.nameInvalid;
      return;
    }
    step = isBrowser ? "sign-in" : "details";
  }

  function currentValues(): Record<string, string> {
    return { ...values };
  }

  async function connect() {
    busy = true;
    error = null;
    try {
      await api.createRemote({ providerId: provider.id, name: name.trim(), values: currentValues() });
      savedName = name.trim();
      result = await api.testRemote(savedName);
      step = "result";
    } catch (err) {
      error = asAppError(err);
    } finally {
      busy = false;
    }
  }

  async function signIn() {
    busy = true;
    error = null;
    signInUrl = null;
    try {
      await api.startBrowserSignIn({ providerId: provider.id, name: name.trim(), values: {} });
      savedName = name.trim();
      result = await api.testRemote(savedName);
      step = "result";
    } catch (err) {
      error = asAppError(err);
      step = "sign-in";
    } finally {
      busy = false;
      signInUrl = null;
    }
  }

  async function cancelSignIn() {
    try {
      await api.cancelBrowserSignIn();
    } catch {
      // Nothing was waiting, so there is nothing to cancel.
    }
    busy = false;
    signInUrl = null;
  }

  async function save() {
    if (!existing) return;
    busy = true;
    error = null;
    result = null;
    try {
      let current = existing.name;
      const newName = name.trim();
      if (newName !== current) {
        current = await api.renameRemote(current, newName);
      }
      if (!isBrowser) {
        await api.updateRemote(current, currentValues());
      }
      savedName = current;
      name = current;
      result = await api.testRemote(current);
    } catch (err) {
      error = asAppError(err);
    } finally {
      busy = false;
    }
  }

  async function retest() {
    if (!existing) return;
    busy = true;
    error = null;
    try {
      result = await api.testRemote(existing.name);
    } catch (err) {
      error = asAppError(err);
    } finally {
      busy = false;
    }
  }
</script>

{#if editing}
  <form class="card" onsubmit={(event) => { event.preventDefault(); void save(); }}>
    <div class="field">
      <label for="remote-name">{text.s.wizard.nameLabel}</label>
      <input id="remote-name" bind:value={name} disabled={busy} spellcheck="false" />
      <p class="help">{text.s.storage.renameHelp}</p>
    </div>

    {#if isBrowser}
      <p class="help">{text.s.wizard.signInOnlyInBrowser(provider.title)}</p>
    {:else}
      {#each provider.fields as field (field.key)}
        <FieldInput {field} bind:value={values[field.key]} savedSecret={savedSecrets.includes(field.key)} disabled={busy} />
      {/each}
    {/if}

    <ErrorNote {error} />
    {#if result}
      {#if result.ok}
        <p class="success" role="status">{result.message}</p>
      {:else}
        <ErrorNote error={{ kind: "rclone", message: result.message, details: result.details }} role="status" />
      {/if}
    {/if}

    <div class="actions">
      <button type="button" class="button" onclick={retest} disabled={busy}>{text.s.wizard.testConnection}</button>
      <button type="submit" class="button primary" disabled={busy || !nameValid}>
        {busy ? text.s.common.saving : text.s.common.save}
      </button>
    </div>
  </form>
{:else}
  <div class="card">
    <ol class="steps" aria-label="Progress">
      <li class:current={step === "name"} class:done={step !== "name"}>{text.s.wizard.steps.name}</li>
      <li class:current={step === "details" || step === "sign-in"} class:done={step === "result"}>
        {isBrowser ? text.s.wizard.steps.signIn : text.s.wizard.steps.details}
      </li>
      <li class:current={step === "result"}>{text.s.wizard.steps.check}</li>
    </ol>

    {#if step === "name"}
      <form onsubmit={(event) => { event.preventDefault(); nextFromName(); }}>
        <div class="field">
          <label for="remote-name">{text.s.wizard.nameQuestion}</label>
          <input id="remote-name" bind:value={name} placeholder={text.s.wizard.namePlaceholder} spellcheck="false" />
          <p class="help">{text.s.wizard.nameHelp}</p>
        </div>
        <ErrorNote {error} />
        <div class="actions">
          <a class="button" href="/remotes/new">{text.s.common.back}</a>
          <button type="submit" class="button primary" disabled={!nameValid}>{text.s.common.continue}</button>
        </div>
      </form>
    {:else if step === "details"}
      <form onsubmit={(event) => { event.preventDefault(); void connect(); }}>
        {#each provider.fields as field (field.key)}
          <FieldInput {field} bind:value={values[field.key]} disabled={busy} />
        {/each}
        <ErrorNote {error} />
        <div class="actions">
          <button type="button" class="button" onclick={() => (step = "name")} disabled={busy}>{text.s.common.back}</button>
          <button type="submit" class="button primary" disabled={busy}>
            {busy ? text.s.wizard.connecting : text.s.wizard.connect}
          </button>
        </div>
      </form>
    {:else if step === "sign-in"}
      <div class="sign-in">
        <h2>{text.s.wizard.signInTitle(provider.title)}</h2>
        {#if busy}
          <p class="help">{text.s.wizard.signInWaiting}</p>
          {#if signInUrl}
            <p class="url">
              {text.s.wizard.signInLinkIntro} <a href={signInUrl} target="_blank" rel="noopener">{text.s.wizard.signInLink}</a>.
            </p>
          {/if}
          <div class="actions">
            <button type="button" class="button" onclick={cancelSignIn}>{text.s.common.cancel}</button>
          </div>
        {:else}
          <p class="help">{text.s.wizard.signInHelp(provider.tagline)}</p>
          <ErrorNote {error} />
          <div class="actions">
            <button type="button" class="button" onclick={() => (step = "name")}>{text.s.common.back}</button>
            <button type="button" class="button primary" onclick={signIn}>{text.s.wizard.signInButton(provider.title)}</button>
          </div>
        {/if}
      </div>
    {:else if step === "result" && result}
      <div class="outcome">
        {#if result.ok}
          <h2>{text.s.wizard.connected(savedName ?? name)}</h2>
          <p class="success">{result.message}</p>
          <div class="actions">
            <a class="button primary" href="/remotes">{text.s.common.done}</a>
          </div>
        {:else}
          <h2>{text.s.wizard.savedButFailed(savedName ?? name)}</h2>
          <ErrorNote error={{ kind: "rclone", message: result.message, details: result.details }} role="status" />
          <p class="help">{text.s.wizard.savedButFailedHelp}</p>
          <div class="actions">
            <a class="button" href="/remotes">{text.s.wizard.keepForNow}</a>
            <button type="button" class="button primary" onclick={() => goto(`/remotes/${encodeURIComponent(savedName ?? "")}`)}>
              {text.s.wizard.editDetails}
            </button>
          </div>
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .card {
    max-width: 560px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 14px;
  }
  label {
    font-weight: 500;
  }
  input {
    background: var(--control-bg);
    border: 1px solid transparent;
    border-radius: var(--radius-control);
    padding: 6px 9px;
    user-select: text;
  }
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 0;
  }
  .help {
    margin: 0;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .success {
    color: var(--status-synced);
    margin: 0 0 12px;
  }
  .url {
    word-break: break-all;
    font-size: 12px;
  }
  .url a {
    color: var(--accent);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 6px;
  }
  .steps {
    display: flex;
    gap: 8px;
    list-style: none;
    padding: 0;
    margin: 0 0 18px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .steps li {
    padding: 3px 10px;
    border-radius: 999px;
    background: var(--control-bg);
  }
  .steps li.current {
    color: var(--accent);
    background: var(--sidebar-selected);
    font-weight: 600;
  }
  .steps li.done {
    color: var(--text-primary);
  }
  h2 {
    font-size: 16px;
    font-weight: 600;
    margin: 0 0 8px;
  }
  .sign-in p,
  .outcome p {
    margin: 0 0 12px;
  }
</style>
