import { describe, expect, it } from "vitest";
import { isValidRemoteName, remoteBadge } from "./remote";
import type { RemoteSummary } from "$lib/api/types";

const base: RemoteSummary = {
  name: "Work",
  rcloneType: "drive",
  providerId: "drive",
  providerTitle: "Google Drive",
  health: { state: "unknown" },
};

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
    expect(remoteBadge(base)).toBeNull();
  });

  it("maps test results to badges", () => {
    expect(remoteBadge({ ...base, health: { state: "ok" } })).toBe("synced");
    expect(remoteBadge({ ...base, health: { state: "failed", message: "no" } })).toBe("disconnected");
  });
});
