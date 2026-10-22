import type { RemoteSummary } from "$lib/api/types";
import type { OverallStatus } from "$lib/api/types";

/** Name rule from the backend: 1 to 40 letters, digits, dashes or underscores. */
const NAME_PATTERN = /^[A-Za-z0-9_-]{1,40}$/;

export function isValidRemoteName(name: string): boolean {
  return NAME_PATTERN.test(name.trim());
}

/** The badge a remote shows, or null when Orbit has not checked it yet. */
export function remoteBadge(remote: RemoteSummary): OverallStatus | null {
  switch (remote.health.state) {
    case "ok":
      return "synced";
    case "failed":
      return "disconnected";
    default:
      return null;
  }
}
