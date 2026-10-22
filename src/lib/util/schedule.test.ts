import { describe, expect, it } from "vitest";
import {
  MAX_INTERVAL_MINUTES,
  intervalMinutes,
  intervalParts,
  nextRunLabel,
  parseTime,
  scheduleLabel,
  timeValue,
} from "./schedule";

describe("intervals", () => {
  it("shows whole hours as hours and everything else as minutes", () => {
    expect(intervalParts(30)).toEqual({ value: 30, unit: "minutes" });
    expect(intervalParts(60)).toEqual({ value: 1, unit: "hours" });
    expect(intervalParts(90)).toEqual({ value: 90, unit: "minutes" });
    expect(intervalParts(180)).toEqual({ value: 3, unit: "hours" });
  });

  it("converts form values to minutes and rejects bad ones", () => {
    expect(intervalMinutes(15, "minutes")).toBe(15);
    expect(intervalMinutes(2, "hours")).toBe(120);
    expect(intervalMinutes(0, "minutes")).toBeNull();
    expect(intervalMinutes(1.5, "hours")).toBeNull();
    expect(intervalMinutes(-5, "minutes")).toBeNull();
    expect(intervalMinutes(MAX_INTERVAL_MINUTES, "minutes")).toBe(MAX_INTERVAL_MINUTES);
    expect(intervalMinutes(MAX_INTERVAL_MINUTES + 1, "minutes")).toBeNull();
    expect(intervalMinutes(7 * 24, "hours")).toBe(MAX_INTERVAL_MINUTES);
    expect(intervalMinutes(7 * 24 + 1, "hours")).toBeNull();
  });
});

describe("times of day", () => {
  it("round-trips through the time input", () => {
    expect(timeValue(9, 5)).toBe("09:05");
    expect(timeValue(23, 59)).toBe("23:59");
    expect(parseTime("09:05")).toEqual({ hour: 9, minute: 5 });
    expect(parseTime("9:05")).toEqual({ hour: 9, minute: 5 });
    expect(parseTime("24:00")).toBeNull();
    expect(parseTime("12:60")).toBeNull();
    expect(parseTime("")).toBeNull();
    expect(parseTime("noon")).toBeNull();
  });
});

describe("labels", () => {
  it("describes each schedule kind", () => {
    expect(scheduleLabel({ kind: "manual" })).toBe("Manual");
    expect(scheduleLabel({ kind: "every", minutes: 30 })).toBe("Every 30 min");
    expect(scheduleLabel({ kind: "every", minutes: 120 })).toBe("Every 2 h");
    expect(scheduleLabel({ kind: "daily", hour: 7, minute: 0 })).toBe("Daily at 07:00");
  });

  it("says when the next run is", () => {
    const now = new Date(2026, 2, 10, 8, 0, 0).getTime() / 1000;
    expect(nextRunLabel(now + 30, now)).toBe("in less than a minute");
    expect(nextRunLabel(now + 25 * 60, now)).toBe("in 25 min");
    expect(nextRunLabel(now + 3 * 3600, now)).toMatch(/^today at /);
    expect(nextRunLabel(now + 26 * 3600, now)).toMatch(/^tomorrow at /);
    expect(nextRunLabel(now + 5 * 86_400, now)).toMatch(/ at /);
    expect(nextRunLabel(now + 5 * 86_400, now)).not.toMatch(/^(today|tomorrow)/);
  });
});
