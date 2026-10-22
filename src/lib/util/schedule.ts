import type { Schedule } from "$lib/api/types";

/** Longest "every" interval the backend accepts: seven days, in minutes. */
export const MAX_INTERVAL_MINUTES = 7 * 24 * 60;

export type IntervalUnit = "minutes" | "hours";

/** Splits an interval into the largest whole unit for the form: 90 → 90 minutes, 120 → 2 hours. */
export function intervalParts(minutes: number): { value: number; unit: IntervalUnit } {
  if (minutes >= 60 && minutes % 60 === 0) return { value: minutes / 60, unit: "hours" };
  return { value: minutes, unit: "minutes" };
}

/** Turns form values back into minutes. Returns null for anything that is not a positive whole number. */
export function intervalMinutes(value: number, unit: IntervalUnit): number | null {
  if (!Number.isInteger(value) || value <= 0) return null;
  const minutes = unit === "hours" ? value * 60 : value;
  return minutes > MAX_INTERVAL_MINUTES ? null : minutes;
}

/** "09:05" for the time input. */
export function timeValue(hour: number, minute: number): string {
  return `${String(hour).padStart(2, "0")}:${String(minute).padStart(2, "0")}`;
}

/** Parses "09:05" from the time input. Returns null when it is not a valid time. */
export function parseTime(value: string): { hour: number; minute: number } | null {
  const match = /^(\d{1,2}):(\d{2})$/.exec(value.trim());
  if (!match) return null;
  const hour = Number(match[1]);
  const minute = Number(match[2]);
  if (hour > 23 || minute > 59) return null;
  return { hour, minute };
}

/** Short description for the job card: "Manual", "Every 30 min", "Every 2 h", "Daily at 09:00". */
export function scheduleLabel(schedule: Schedule): string {
  switch (schedule.kind) {
    case "manual":
      return "Manual";
    case "every": {
      const { value, unit } = intervalParts(schedule.minutes);
      return `Every ${value} ${unit === "hours" ? "h" : "min"}`;
    }
    case "daily":
      return `Daily at ${timeValue(schedule.hour, schedule.minute)}`;
  }
}

/**
 * When the next automatic run is, relative to `now`: "in less than a minute", "in 25 min",
 * "in 3 h", or "today at 09:00" / "tomorrow at 09:00" / a date for anything further out.
 * Takes Unix seconds.
 */
export function nextRunLabel(nextRunAt: number, now: number = Date.now() / 1000): string {
  const seconds = nextRunAt - now;
  if (seconds < 60) return "in less than a minute";
  if (seconds < 3600) return `in ${Math.round(seconds / 60)} min`;
  const next = new Date(nextRunAt * 1000);
  const time = next.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  const current = new Date(now * 1000);
  const dayDiff = Math.round((startOfDay(next) - startOfDay(current)) / 86_400_000);
  if (dayDiff === 0) return `today at ${time}`;
  if (dayDiff === 1) return `tomorrow at ${time}`;
  return `${next.toLocaleDateString(undefined, { dateStyle: "medium" })} at ${time}`;
}

function startOfDay(date: Date): number {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
}
