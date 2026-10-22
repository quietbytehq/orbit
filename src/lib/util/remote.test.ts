import { describe, expect, it } from "vitest";
import { isValidRemoteName, remoteBadge, remotesInUse } from "./remote";
import type { JobView, RemoteSummary, TransferItem } from "$lib/api/types";

const base: RemoteSummary = {
  name: "Work",
  rcloneType: "drive",
  providerId: "drive",
  providerTitle: "Google Drive",
  health: { state: "unknown" },
};

const noneInUse = new Set<string>();

function transfer(overrides: Partial<TransferItem>): TransferItem {
  return {
    id: 1,
    name: "a.txt",
    direction: "upload",
    isDir: false,
    from: "/home/ana",
    to: "Work:/",
    remotes: ["Work"],
    state: { state: "running" },
    bytes: 0,
    totalBytes: 0,
    speed: 0,
    etaSeconds: null,
    ...overrides,
  };
}

describe("isValidRemoteName", () => {
  it("accepts letters, digits, dashes and underscores", () => {
    expect(isValidRemoteName("Work-Drive_2")).toBe(true);
    expect(isValidRemoteName("  spaced  ")).toBe(true);
  });

  it("rejects empty, long and punctuated names", () => {
    expect(isValidRemoteName("")).toBe(false);
    expect(isValidRemoteName("a".repeat(41))).toBe(false);
    expect(isValidRemoteName("has space in it")).toBe(false);
    expect(isValidRemoteName("colon:name")).toBe(false);
  });
});

describe("remoteBadge", () => {
  it("shows nothing until the remote has been checked", () => {
    expect(remoteBadge(base, noneInUse)).toBeNull();
  });

  it("maps test results to badges", () => {
    expect(remoteBadge({ ...base, health: { state: "ok" } }, noneInUse)).toBe("synced");
    expect(remoteBadge({ ...base, health: { state: "failed", message: "no" } }, noneInUse)).toBe("disconnected");
  });

  it("shows syncing while the remote is in use, even if its last check failed", () => {
    expect(remoteBadge({ ...base, health: { state: "failed", message: "no" } }, new Set(["Work"]))).toBe("syncing");
  });
});

describe("remotesInUse", () => {
  const job: JobView = {
    id: "j",
    name: "Backup",
    kind: "sync",
    source: { kind: "local", path: "/home/ana" },
    destination: { kind: "remote", remote: "Archive", path: "" },
    enabled: true,
    schedule: { kind: "manual" },
    runs: [],
    progress: { runId: "r", bytes: 0, totalBytes: 0, files: 0, errors: 0, speed: 0, etaSeconds: null, currentFile: null },
    nextRunAt: null,
  };

  it("collects remotes from active transfers and running jobs", () => {
    const names = remotesInUse([transfer({})], [job]);
    expect([...names].sort()).toEqual(["Archive", "Work"]);
  });

  it("ignores finished transfers and idle jobs", () => {
    expect(remotesInUse([transfer({ state: { state: "done" } })], [{ ...job, progress: null }]).size).toBe(0);
  });
});
