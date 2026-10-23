import { api, asAppError, onJobs } from "$lib/api";
import type { AppError, JobView } from "$lib/api/types";

/** Saved jobs and their live progress. The backend pushes every change. */
class JobStore {
  items = $state<JobView[]>([]);
  error = $state<AppError | null>(null);
  #started = false;

  async start() {
    if (this.#started) return;
    this.#started = true;
    await onJobs((items) => {
      this.items = items;
    });
    await this.refresh();
  }

  async refresh() {
    try {
      this.items = await api.listJobs();
    } catch (err) {
      this.error = asAppError(err);
    }
  }

  /** Runs an action and reports its error in `error` instead of throwing. */
  async act(action: () => Promise<unknown>): Promise<boolean> {
    this.error = null;
    try {
      await action();
      return true;
    } catch (err) {
      this.error = asAppError(err);
      return false;
    }
  }

  get active(): number {
    return this.items.filter((item) => item.progress !== null).length;
  }
}

export const jobs = new JobStore();
