<script lang="ts">
  import { onMount } from "svelte";
  import Dialog from "$lib/components/Dialog.svelte";
  import JobForm from "$lib/components/JobForm.svelte";
  import LogDialog from "$lib/components/LogDialog.svelte";
  import { api, errorMessage } from "$lib/api";
  import type { JobView, RunOutcome } from "$lib/api/types";
  import { jobs } from "$lib/stores/jobs.svelte";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { endpointLabel } from "$lib/util/endpoint";
  import { nextRunLabel, scheduleLabel } from "$lib/util/schedule";
  import { formatBytes, formatDuration, formatSpeed, timeAgo } from "$lib/util/format";

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
    switch (outcome.state) {
      case "running":
        return "Running";
      case "succeeded":
        return "Finished";
      case "failed":
        return "Problem";
      case "cancelled":
        return "Stopped";
    }
  }

  function outcomeTitle(outcome: RunOutcome): string {
    return outcome.state === "failed" ? outcome.message : outcomeLabel(outcome);
  }

  async function openLog(job: JobView, runId: string, startedAt: number) {
    logTitle = `${job.name}, ${new Date(startedAt * 1000).toLocaleString()}`;
    logText = null;
    try {
      logText = await api.readJobLog(job.id, runId);
    } catch (err) {
      logText = errorMessage(err);
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
  <h1>Sync</h1>
  <button class="button primary" onclick={() => (creating = true)}>New job</button>
</div>

<p class="help">
  Jobs copy or sync folders between a remote and this computer, or between two remotes. Each run is recorded here with
  its log.
</p>

{#if rcd.state.state !== "healthy"}
  <p class="notice">Orbit cannot reach rclone right now, so jobs cannot run.</p>
{/if}
{#if jobs.error}<p class="error" role="alert">{jobs.error}</p>{/if}

{#if jobs.items.length === 0}
  <section class="card empty">
    <h2>No jobs yet</h2>
    <p>Create a job to copy a folder to a remote, keep a backup in sync, or pull a remote folder down to this computer.</p>
    <button class="button primary" onclick={() => (creating = true)}>Create your first job</button>
  </section>
{:else}
  <ul class="list">
    {#each jobs.items as view (view.id)}
      {@const last = view.runs[0]}
      <li class="card job">
        <div class="top">
          <div class="title">
            <span class="name">{view.name}</span>
            <span class="pill">{view.kind === "sync" ? "Sync" : "Copy"}</span>
            {#if view.schedule.kind !== "manual"}<span class="pill">{scheduleLabel(view.schedule)}</span>{/if}
          </div>
          <label class="toggle">
            <input
              type="checkbox"
              checked={view.enabled}
              onchange={(event) => jobs.act(() => api.setJobEnabled(view.id, event.currentTarget.checked))}
            />
            <span>{view.enabled ? "On" : "Off"}</span>
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
              {view.progress.files} files{view.progress.totalBytes > 0 ? ` · ${formatBytes(view.progress.bytes)} of ${formatBytes(view.progress.totalBytes)}` : ""}{view.progress.speed > 0 ? ` · ${formatSpeed(view.progress.speed)}` : ""}{view.progress.etaSeconds !== null ? ` · ${formatDuration(view.progress.etaSeconds)} left` : ""}
              {#if view.progress.errors > 0}<span class="problem"> · {view.progress.errors} errors so far</span>{/if}
            </p>
            {#if view.progress.currentFile}<p class="file" title={view.progress.currentFile}>{view.progress.currentFile}</p>{/if}
          </div>
        {:else if last}
          <p class="status" class:problem={last.outcome.state === "failed"}>
            Last run {timeAgo(last.finishedAt ?? last.startedAt)}: {outcomeTitle(last.outcome)}{last.files > 0 ? ` · ${last.files} files` : ""}{view.nextRunAt !== null ? ` · Next run ${nextRunLabel(view.nextRunAt, now)}` : ""}
          </p>
        {:else}
          <p class="status">Never run{view.nextRunAt !== null ? ` · Next run ${nextRunLabel(view.nextRunAt, now)}` : ""}</p>
        {/if}

        <div class="actions">
          {#if view.progress}
            <button class="button" onclick={() => jobs.act(() => api.cancelJob(view.id))}>Stop</button>
          {:else}
            <button class="button primary" onclick={() => jobs.act(() => api.runJob(view.id))} disabled={rcd.state.state !== "healthy"}>
              Run now
            </button>
          {/if}
          <button class="button" onclick={() => (editing = view)} disabled={view.progress !== null}>Edit</button>
          <button class="button" onclick={() => (expanded[view.id] = !expanded[view.id])} aria-expanded={!!expanded[view.id]}>
            {expanded[view.id] ? "Hide history" : `History (${view.runs.length})`}
          </button>
          <span class="spacer"></span>
          {#if confirmingDelete?.id === view.id}
            <button class="button" onclick={() => (confirmingDelete = null)}>Keep</button>
            <button class="button destructive-fill" onclick={() => remove(view)}>Delete job</button>
          {:else}
            <button class="button destructive" onclick={() => (confirmingDelete = view)} disabled={view.progress !== null}>Delete…</button>
          {/if}
        </div>

        {#if expanded[view.id]}
          {#if view.runs.length === 0}
            <p class="status">No runs yet.</p>
          {:else}
            <table class="runs">
              <tbody>
                {#each view.runs as run (run.id)}
                  <tr>
                    <td>{new Date(run.startedAt * 1000).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" })}</td>
                    <td class="outcome" data-state={run.outcome.state} title={outcomeTitle(run.outcome)}>{outcomeLabel(run.outcome)}</td>
                    <td class="muted">{run.scheduled ? "Scheduled" : "By hand"}</td>
                    <td>{run.files} files</td>
                    <td>{run.errors > 0 ? `${run.errors} errors` : ""}</td>
                    <td class="end"><button class="link" onclick={() => openLog(view, run.id, run.startedAt)}>View log</button></td>
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
  .notice,
  .error {
    color: var(--status-disconnected);
  }
  :global(.button.destructive) {
    color: var(--status-disconnected);
  }
  :global(.button.destructive-fill) {
    background: var(--status-disconnected);
    color: #fff;
    border: none;
    border-radius: var(--radius-control);
    padding: 6px 14px;
    cursor: pointer;
    font-weight: 500;
  }
</style>
