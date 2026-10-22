import { describe, expect, it } from "vitest";
import { formatBytes, formatDuration, formatSpeed, timeAgo } from "./format";

describe("formatBytes", () => {
  it("uses bytes below one kilobyte", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(999)).toBe("999 B");
  });

  it("uses decimal units", () => {
    expect(formatBytes(1000)).toBe("1.0 KB");
    expect(formatBytes(1_500_000)).toBe("1.5 MB");
    expect(formatBytes(2_000_000_000)).toBe("2.0 GB");
  });

  it("drops the decimal for large values", () => {
    expect(formatBytes(120_000_000)).toBe("120 MB");
  });

  it("caps at petabytes", () => {
    expect(formatBytes(5e18)).toBe("5000 PB");
  });

  it("guards against invalid input", () => {
    expect(formatBytes(-1)).toBe("—");
    expect(formatBytes(Number.NaN)).toBe("—");
  });
});

describe("formatSpeed", () => {
  it("appends /s", () => {
    expect(formatSpeed(2_500_000)).toBe("2.5 MB/s");
  });
});

describe("formatDuration", () => {
  it("shows seconds, minutes and hours", () => {
    expect(formatDuration(5)).toBe("5 s");
    expect(formatDuration(0.2)).toBe("1 s");
    expect(formatDuration(180)).toBe("3 min");
    expect(formatDuration(3600)).toBe("1 h");
    expect(formatDuration(3900)).toBe("1 h 5 min");
  });

  it("hides unknown values", () => {
    expect(formatDuration(null)).toBe("");
    expect(formatDuration(-3)).toBe("");
  });
});

describe("timeAgo", () => {
  const now = 1_000_000;
  it("describes recent times in plain words", () => {
    expect(timeAgo(now - 10, now)).toBe("just now");
    expect(timeAgo(now - 300, now)).toBe("5 min ago");
    expect(timeAgo(now - 7200, now)).toBe("2 h ago");
  });
});
