import { describe, expect, it } from "vitest";
import { overallStatus } from "./status";

const healthy = { state: "healthy", version: "v1.75.1" } as const;

describe("overallStatus", () => {
  it("is synced when healthy and idle", () => {
    expect(overallStatus(healthy)).toBe("synced");
  });

  it("is syncing while anything is active", () => {
    expect(overallStatus(healthy, 1, 0)).toBe("syncing");
    expect(overallStatus(healthy, 0, 2)).toBe("syncing");
  });

  it("is disconnected whenever rclone is not healthy", () => {
    expect(overallStatus({ state: "starting" }, 4, 4)).toBe("disconnected");
    expect(overallStatus({ state: "failed", message: "x" })).toBe("disconnected");
    expect(overallStatus({ state: "stopped" })).toBe("disconnected");
  });
});
