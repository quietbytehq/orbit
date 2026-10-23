<script lang="ts">
  import { onMount } from "svelte";
  import ErrorNote from "$lib/components/ErrorNote.svelte";
  import JobForm from "$lib/components/JobForm.svelte";
  import LogDialog from "$lib/components/LogDialog.svelte";
  import { api, asAppError } from "$lib/api";
  import type { JobView, RunOutcome } from "$lib/api/types";
  import { jobs } from "$lib/stores/jobs.svelte";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { text } from "$lib/text/index.svelte";
  import { endpointLabel } from "$lib/util/endpoint";
  import { formatBytes, formatDuration, formatSpeed, timeAgo } from "$lib/util/format";
  import { nextRunLabel, scheduleLabel } from "$lib/util/schedule";
  import { Copy, FolderSync, ICON, Pencil, Play, Plus, Square, Trash } from "$lib/icons";

  let creating = $state(false);
  let editing = $state<JobView | null>(null);
  let confirmingDelete = $state<JobView | null>(null);
  let expanded = $state<Record<string, boolean>>({});
  let logTitle = $state("");
  let logText = $state<string | null>(null);
  /** Unix seconds, refreshed every half minute so "next run in…" stays right. */
  let now = $state(Date.now() / 1000);

  onMount(() => {
    void jobs.start();
    const clock = window.setInterval(() => (now = Date.now() / 1000), 30_000);
    return () => window.clearInterval(clock);
  });

  function outcomeLabel(outcome: RunOutcome): string {
    return text.s.sync.outcome[outcome.state];
  }

  function outcomeTitle(outcome: RunOutcome): string {
    return outcome.state === "failed" ? outcome.message : outcomeLabel(outcome);
  }

  async function openLog(job: JobView, runId: string, startedAt: number) {
    logTitle = text.s.sync.logTitle(job.name, new Date(startedAt * 1000).toLocaleString());
    logText = null;
    try {
      logText = await api.readJobLog(job.id, runId);
    } catch (err) {
      logText = asAppError(err).message;
    }
  }

  function percent(view: JobView): number | null {
    const progress = view.progress;
    if (!progress || progress.totalBytes <= 0) return null;
    return Math.min(100, (progress.bytes / progress.totalBytes) * 100);
  }

  async function remove(job: JobView) {
    confirmingDelete = null;
    await jobs.act(() => api.deleteJob(job.id));
  }
</script>

<div class="header">
  <h1>{text.s.sync.title}</h1>
  <button class="button primary with-icon" onclick={() => (creating = true)}><Plus {...ICON} />{text.s.sync.newJob}</button>
</div>

<p class="help">{text.s.sync.intro}</p>

{#if rcd.state.state !== "healthy"}
  <p class="notice">{text.s.sync.cannotRun}</p>
{/if}
<ErrorNote error={jobs.error} />

{#if jobs.items.length === 0}
  <section class="card empty">
    <h2>{text.s.sync.emptyTitle}</h2>
    <p>{text.s.sync.emptyBody}</p>
    <button class="button primary" onclick={() => (creating = true)}>{text.s.sync.createFirst}</button>
  </section>
{:else}
  <ul class="list">
    {#each jobs.items as view (view.id)}
      {@const last = view.runs[0]}
      <li class="card job">
        <div class="top">
          <div class="title">
            <span class="name">{view.name}</span>
            <span class="pill with-icon">{#if view.kind === "sync"}<FolderSync size={12} strokeWidth={2} aria-hidden="true" />{text.s.sync.kindSync}{:else}<Copy size={12} strokeWidth={2} aria-hidden="true" />{text.s.sync.kindCopy}{/if}</span>
            {#if view.schedule.kind !== "manual"}<span class="pill">{scheduleLabel(view.schedule)}</span>{/if}
          </div>
          <label class="toggle">
            <input
              type="checkbox"
              checked={view.enabled}
              onchange={(event) => jobs.act(() => api.setJobEnabled(view.id, event.currentTarget.checked))}
            />
            <span>{view.enabled ? text.s.sync.on : text.s.sync.off}</span>
          </label>
        </div>

        <p class="route">{endpointLabel(view.source)} → {endpointLabel(view.destination)}</p>

        {#if view.progress}
          {@const pct = percent(view)}
          <div class="progress">
            <div class="bar" aria-hidden="true">
              <div class:indeterminate={pct === null} style:width={pct !== null ? `${pct}%` : undefined}></div>
            </div>
            <p class="status">
              {text.s.sync.files(view.progress.files)}{view.progress.totalBytes > 0 ? ` · ${text.s.sync.of(formatBytes(view.progress.bytes), formatBytes(view.progress.totalBytes))}` : ""}{view.progress.speed > 0 ? ` · ${formatSpeed(view.progress.speed)}` : ""}{view.progress.etaSeconds !== null ? ` · ${text.s.sync.left(formatDuration(view.progress.etaSeconds))}` : ""}
              {#if view.progress.errors > 0}<span class="problem"> · {text.s.sync.errorsSoFar(view.progress.errors)}</span>{/if}
            </p>
            {#if view.progress.currentFile}<p class="file" title={view.progress.currentFile}>{view.progress.currentFile}</p>{/if}
          </div>
        {:else if last}
          <p class="status" class:problem={last.outcome.state === "failed"}>
            {text.s.sync.lastRun(timeAgo(last.finishedAt ?? last.startedAt), outcomeTitle(last.outcome))}{last.files > 0 ? ` · ${text.s.sync.files(last.files)}` : ""}{view.nextRunAt !== null ? ` · ${text.s.sync.nextRun(nextRunLabel(view.nextRunAt, now))}` : ""}
          </p>
        {:else}
          <p class="status">{text.s.sync.neverRun}{view.nextRunAt !== null ? ` · ${text.s.sync.nextRun(nextRunLabel(view.nextRunAt, now))}` : ""}</p>
        {/if}

        <div class="actions">
          {#if view.progress}
            <button class="button with-icon" onclick={() => jobs.act(() => api.cancelJob(view.id))}><Square {...ICON} />{text.s.sync.stop}</button>
          {:else}
            <button class="button primary with-icon" onclick={() => jobs.act(() => api.runJob(view.id))} disabled={rcd.state.state !== "healthy"}>
              <Play {...ICON} />{text.s.sync.runNow}
            </button>
          {/if}
          <button class="button with-icon" onclick={() => (editing = view)} disabled={view.progress !== null}><Pencil {...ICON} />{text.s.sync.edit}</button>
          <button class="button" onclick={() => (expanded[view.id] = !expanded[view.id])} aria-expanded={!!expanded[view.id]}>
            {expanded[view.id] ? text.s.sync.hideHistory : text.s.sync.history(view.runs.length)}
          </button>
          <span class="spacer"></span>
          {#if confirmingDelete?.id === view.id}
            <button class="button" onclick={() => (confirmingDelete = null)}>{text.s.common.keep}</button>
            <button class="button destructive-fill" onclick={() => remove(view)}>{text.s.sync.deleteButton}</button>
          {:else}
            <button class="button destructive with-icon" onclick={() => (confirmingDelete = view)} disabled={view.progress !== null}><Trash {...ICON} />{text.s.sync.deleteEllipsis}</button>
          {/if}
        </div>

        {#if expanded[view.id]}
          {#if view.runs.length === 0}
            <p class="status">{text.s.sync.noRuns}</p>
          {:else}
            <table class="runs">
              <tbody>
                {#each view.runs as run (run.id)}
                  <tr>
                    <td>{new Date(run.startedAt * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" })}</td>
                    <td class="outcome" data-state={run.outcome.state} title={run.outcome.state === "failed" ? (run.outcome.details ?? run.outcome.message) : outcomeTitle(run.outcome)}>{outcomeLabel(run.outcome)}</td>
                    <td class="muted">{run.scheduled ? text.s.sync.scheduled : text.s.sync.byHand}</td>
                    <td>{text.s.sync.files(run.files)}</td>
                    <td>{run.errors > 0 ? text.s.sync.errors(run.errors) : ""}</td>
                    <td class="end"><button class="link" onclick={() => openLog(view, run.id, run.startedAt)}>{text.s.sync.viewLog}</button></td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        {/if}
      </li>
    {/each}
  </ul>
{/if}

{#if creating}
  <JobForm
    onSaved={() => (creating = false)}
    onCancel={() => (creating = false)}
  />
{/if}

{#if editing}
  <JobForm existing={editing} onSaved={() => (editing = null)} onCancel={() => (editing = null)} />
{/if}

{#if logText !== null || logTitle}
  <LogDialog title={logTitle} text={logText} onClose={() => { logTitle = ""; logText = null; }} />
{/if}

<style>
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }
  .header h1 {
    margin: 0;
  }
  .help {
    color: var(--text-secondary);
    margin: 0 0 18px;
    max-width: 640px;
  }
  .list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .job {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .name {
    font-weight: 600;
    font-size: 14px;
  }
  .pill {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 999px;
    background: var(--control-bg);
    color: var(--text-secondary);
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .route,
  .status,
  .file {
    margin: 0;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .route {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .status.problem,
  .problem {
    color: var(--status-disconnected);
  }
  .file {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  }
  .progress {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--control-bg);
    overflow: hidden;
    position: relative;
  }
  .bar > div {
    height: 100%;
    background: var(--status-syncing);
    border-radius: 3px;
    transition: width 0.3s ease;
  }
  .bar > div.indeterminate {
    width: 40%;
    position: absolute;
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      left: -40%;
    }
    to {
      left: 100%;
    }
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .spacer {
    flex: 1;
  }
  .runs {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  .runs td {
    padding: 6px 4px;
    border-top: 1px solid var(--separator);
    white-space: nowrap;
  }
  .runs .outcome[data-state="succeeded"] {
    color: var(--status-synced);
  }
  .runs .outcome[data-state="failed"] {
    color: var(--status-disconnected);
  }
  .runs .outcome[data-state="running"] {
    color: var(--status-syncing);
  }
  .runs .end {
    text-align: right;
  }
  .runs .muted {
    color: var(--text-secondary);
  }
  .link {
    appearance: none;
    border: none;
    background: none;
    color: var(--accent);
    cursor: pointer;
    padding: 0;
  }
  .empty h2 {
    font-size: 15px;
    margin: 0 0 6px;
  }
  .empty p {
    color: var(--text-secondary);
    margin: 0 0 14px;
  }
  .notice {
    color: var(--status-disconnected);
  }
</style>
