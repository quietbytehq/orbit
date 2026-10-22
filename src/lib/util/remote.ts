import type { JobView, RemoteSummary, TransferItem } from "$lib/api/types";
import type { OverallStatus } from "$lib/api/types";

/** Name rule from the backend: 1 to 40 letters, digits, dashes or underscores. */
const NAME_PATTERN = /^[A-Za-z0-9_-]{1,40}$/;

export function isValidRemoteName(name: string): boolean {
  return NAME_PATTERN.test(name.trim());
}

/** Names of remotes that a running transfer or sync job is reading from or writing to. */
export function remotesInUse(transfers: TransferItem[], jobs: JobView[]): Set<string> {
  const names = new Set<string>();
  for (const transfer of transfers) {
    if (transfer.state.state === "queued" || transfer.state.state === "running") {
      transfer.remotes.forEach((name) => names.add(name));
    }
  }
  for (const job of jobs) {
    if (job.progress === null) continue;
    for (const endpoint of [job.source, job.destination]) {
      if (endpoint.kind === "remote") names.add(endpoint.remote);
    }
  }
  return names;
}

/**
 * The badge a remote shows, or null when Orbit has not checked it yet.
 * A remote in use by a transfer or job shows as syncing, whatever its last check said.
 */
export function remoteBadge(remote: RemoteSummary, inUse: Set<string>): OverallStatus | null {
  if (inUse.has(remote.name)) return "syncing";
  switch (remote.health.state) {
    case "ok":
      return "synced";
    case "failed":
      return "disconnected";
    default:
      return null;
  }
}
