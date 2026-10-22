import type { OverallStatus, RcdState } from "$lib/api/types";

/** Mirrors `status::overall` in the Rust backend. Keep the two in step. */
export function overallStatus(rcd: RcdState, activeTransfers = 0, activeJobs = 0): OverallStatus {
  if (rcd.state !== "healthy") return "disconnected";
  return activeTransfers + activeJobs > 0 ? "syncing" : "synced";
}

export const STATUS_LABEL: Record<OverallStatus, string> = {
  synced: "Up to date",
  syncing: "Working",
  disconnected: "Disconnected",
};

export const STATUS_COLOR_VAR: Record<OverallStatus, string> = {
  synced: "var(--status-synced)",
  syncing: "var(--status-syncing)",
  disconnected: "var(--status-disconnected)",
};
