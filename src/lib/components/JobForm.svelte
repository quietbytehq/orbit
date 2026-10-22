<script lang="ts">
  import Dialog from "./Dialog.svelte";
  import EndpointField from "./EndpointField.svelte";
  import { api, errorMessage } from "$lib/api";
  import type { Endpoint, JobInput, JobKind, JobView, Schedule } from "$lib/api/types";
  import { intervalMinutes, intervalParts, parseTime, timeValue, type IntervalUnit } from "$lib/util/schedule";

  let {
    existing = null,
    onSaved,
    onCancel,
  }: {
    existing?: JobView | null;
    onSaved: (view: JobView) => void;
    onCancel: () => void;
  } = $props();

  // The form is opened fresh for each job, so the initial values are read once on purpose.
  // svelte-ignore state_referenced_locally
  let name = $state(existing?.name ?? "");
  // svelte-ignore state_referenced_locally
  let kind = $state<JobKind>(existing?.kind ?? "copy");
  // svelte-ignore state_referenced_locally
  let source = $state<Endpoint | null>(existing?.source ?? null);
  // svelte-ignore state_referenced_locally
  let destination = $state<Endpoint | null>(existing?.destination ?? null);
  // svelte-ignore state_referenced_locally
  const initialSchedule: Schedule = existing?.schedule ?? { kind: "manual" };
  const initialInterval = intervalParts(initialSchedule.kind === "every" ? initialSchedule.minutes : 60);
  let scheduleKind = $state<Schedule["kind"]>(initialSchedule.kind);
  let intervalValue = $state(initialInterval.value);
  let intervalUnit = $state<IntervalUnit>(initialInterval.unit);
  let dailyTime = $state(
    initialSchedule.kind === "daily" ? timeValue(initialSchedule.hour, initialSchedule.minute) : "09:00",
  );
  let busy = $state(false);
  let error = $state<string | null>(null);

  const SCHEDULE_TEXT: Record<Schedule["kind"], { title: string; help: string }> = {
    manual: { title: "Only when I run it", help: "Nothing happens until you press Run now." },
    every: { title: "Every…", help: "Repeats while Orbit is open, counted from the last run." },
    daily: { title: "Daily at…", help: "Once a day at this time, if Orbit is open then." },
  };

  /** Reads the schedule from the form. Returns a message instead when the values do not make sense. */
  function readSchedule(): Schedule | string {
    switch (scheduleKind) {
      case "manual":
        return { kind: "manual" };
      case "every": {
        const minutes = intervalMinutes(intervalValue, intervalUnit);
        return minutes === null ? "Choose a whole number of minutes or hours, up to seven days." : { kind: "every", minutes };
      }
      case "daily": {
        const time = parseTime(dailyTime);
        return time === null ? "Choose a time of day." : { kind: "daily", ...time };
      }
    }
  }

  const KIND_TEXT: Record<JobKind, { title: string; help: string }> = {
    copy: {
      title: "Copy",
      help: "Adds new and changed files to the destination. Nothing is deleted.",
    },
    sync: {
      title: "Sync",
      help: "Makes the destination match the source. Files in the destination that are not in the source are deleted.",
    },
  };

  async function save() {
    if (!source || !destination) {
      error = "Choose both the source and the destination.";
      return;
    }
    const schedule = readSchedule();
    if (typeof schedule === "string") {
      error = schedule;
      return;
    }
    busy = true;
    error = null;
    const input: JobInput = { name, kind, source, destination, schedule };
    try {
      const view = existing ? await api.updateJob(existing.id, input) : await api.createJob(input);
      onSaved(view);
    } catch (err) {
      error = errorMessage(err);
    } finally {
      busy = false;
    }
  }
</script>

<Dialog title={existing ? "Edit job" : "New job"} onClose={onCancel}>
  <form
    class="form"
    onsubmit={(event) => {
      event.preventDefault();
      void save();
    }}
  >
    <div class="field">
      <label for="job-name">Name</label>
      <input id="job-name" bind:value={name} placeholder="for example, Photos backup" disabled={busy} />
    </div>

    <fieldset class="kinds" disabled={busy}>
      <legend>What should happen</legend>
      {#each ["copy", "sync"] as const as option (option)}
        <label class="kind" class:selected={kind === option}>
          <input type="radio" name="kind" value={option} bind:group={kind} />
          <span class="kind-title">{KIND_TEXT[option].title}</span>
          <span class="kind-help">{KIND_TEXT[option].help}</span>
        </label>
      {/each}
    </fieldset>

    {#if kind === "sync"}
      <p class="warning" role="note">
        Sync deletes files in the destination that are missing from the source. Check the destination before you run it.
      </p>
    {/if}

    <EndpointField label="Source" value={source} onChange={(endpoint) => (source = endpoint)} disabled={busy} />
    <EndpointField label="Destination" value={destination} onChange={(endpoint) => (destination = endpoint)} disabled={busy} />

    <fieldset class="kinds" disabled={busy}>
      <legend>When to run</legend>
      {#each ["manual", "every", "daily"] as const as option (option)}
        <label class="kind" class:selected={scheduleKind === option}>
          <input type="radio" name="schedule" value={option} bind:group={scheduleKind} />
          <span class="kind-title">{SCHEDULE_TEXT[option].title}</span>
          <span class="kind-help">{SCHEDULE_TEXT[option].help}</span>
          {#if option === "every" && scheduleKind === "every"}
            <span class="kind-controls">
              <input
                type="number"
                class="short"
                min="1"
                step="1"
                inputmode="numeric"
                aria-label="Run every"
                bind:value={intervalValue}
              />
              <select aria-label="Unit" bind:value={intervalUnit}>
                <option value="minutes">minutes</option>
                <option value="hours">hours</option>
              </select>
            </span>
          {:else if option === "daily" && scheduleKind === "daily"}
            <span class="kind-controls">
              <input type="time" aria-label="Time of day" bind:value={dailyTime} />
            </span>
          {/if}
        </label>
      {/each}
    </fieldset>
    <p class="note">Schedules only run while Orbit is open. A run missed while it was closed is not made up.</p>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="actions">
      <button type="button" class="button" onclick={onCancel} disabled={busy}>Cancel</button>
      <button type="submit" class="button primary" disabled={busy || !name.trim()}>
        {busy ? "Saving…" : existing ? "Save changes" : "Create job"}
      </button>
    </div>
  </form>
</Dialog>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  label {
    font-weight: 500;
  }
  input:not([type]) {
    background: var(--control-bg);
    border: none;
    border-radius: var(--radius-control);
    padding: 6px 9px;
    user-select: text;
  }
  .kinds {
    border: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  legend {
    font-weight: 500;
    margin-bottom: 6px;
  }
  .kind {
    display: grid;
    grid-template-columns: auto auto 1fr;
    gap: 2px 8px;
    padding: 8px 10px;
    border-radius: var(--radius-control);
    background: var(--control-bg);
    cursor: pointer;
    font-weight: normal;
  }
  .kind.selected {
    background: var(--sidebar-selected);
  }
  .kind input {
    grid-row: span 2;
    margin: 3px 0 0;
  }
  .kind-title {
    font-weight: 600;
  }
  .kind-help {
    grid-column: 2 / 4;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .kind-controls {
    grid-column: 2 / 4;
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }
  .kind-controls input,
  .kind-controls select {
    background: var(--control-bg);
    border: none;
    border-radius: var(--radius-control);
    padding: 4px 8px;
    user-select: text;
  }
  .kind-controls .short {
    width: 72px;
  }
  .note {
    margin: -4px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
  }
  .warning {
    margin: 0;
    padding: 8px 10px;
    border-radius: var(--radius-control);
    background: rgba(255, 149, 0, 0.14);
    color: var(--text-primary);
    font-size: 12px;
  }
  .error {
    color: var(--status-disconnected);
    margin: 0;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    margin-top: 4px;
  }
</style>
