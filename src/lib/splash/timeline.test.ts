import { describe, expect, it } from "vitest";
import { BASE_PHASES, FADE_OUT_MS, REDUCED_HOLD_MS, SEQUENCE_MS, phaseTiming, timeline } from "./timeline";

describe("splash timeline", () => {
  it("keeps the base phases contiguous from zero", () => {
    const phases = Object.values(BASE_PHASES);
    expect(phases[0][0]).toBe(0);
    for (let i = 1; i < phases.length; i++) {
      expect(phases[i][0]).toBe(phases[i - 1][1]);
    }
  });

  it("plays about 5.5 seconds in full and 1.5 seconds when short", () => {
    expect(SEQUENCE_MS.full).toBe(5500);
    expect(SEQUENCE_MS.short).toBe(1500);
    expect(timeline("full").sequenceMs).toBe(5500);
    expect(timeline("short").sequenceMs).toBe(1500);
  });

  it("stretches every phase proportionally and keeps them contiguous", () => {
    for (const version of ["full", "short"] as const) {
      const line = timeline(version);
      const phases = Object.values(line.phases);
      expect(phases[0][0]).toBe(0);
      expect(phases[phases.length - 1][1]).toBe(line.sequenceMs);
      for (let i = 1; i < phases.length; i++) {
        expect(phases[i][0]).toBe(phases[i - 1][1]);
      }
      // The ring takes the same share of the sequence in both versions.
      const [, ringLength] = phaseTiming(line, "ring");
      expect(ringLength / line.sequenceMs).toBeCloseTo(1200 / 3500, 2);
      for (const phase of Object.keys(line.phases) as (keyof typeof line.phases)[]) {
        expect(phaseTiming(line, phase)[1]).toBeGreaterThan(0);
      }
    }
  });

  it("fades out over the last 0.3 seconds, after the content sequence", () => {
    expect(FADE_OUT_MS).toBe(300);
    const line = timeline("full");
    expect(line.totalMs).toBe(line.sequenceMs + FADE_OUT_MS);
  });

  it("holds the final frame for 1.5 seconds with reduced motion", () => {
    expect(REDUCED_HOLD_MS).toBe(1500);
  });
});
