import { describe, expect, it } from "vitest";
import { PHASES, REDUCED_HOLD_MS, SPLASH_MS, phaseTiming } from "./timeline";

describe("splash timeline", () => {
  it("runs 3.5 seconds in contiguous phases", () => {
    const phases = Object.values(PHASES);
    expect(phases[0][0]).toBe(0);
    for (let i = 1; i < phases.length; i++) {
      expect(phases[i][0]).toBe(phases[i - 1][1]);
    }
    expect(SPLASH_MS).toBe(3500);
  });

  it("gives each phase a positive duration", () => {
    for (const phase of Object.keys(PHASES) as (keyof typeof PHASES)[]) {
      const [, duration] = phaseTiming(phase);
      expect(duration).toBeGreaterThan(0);
    }
  });

  it("holds the final frame for 1.5 seconds with reduced motion", () => {
    expect(REDUCED_HOLD_MS).toBe(1500);
  });
});
