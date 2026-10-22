/** Splash timeline in milliseconds. Each phase starts when the previous one ends. */
export const PHASES = {
  ring: [0, 1200],
  cloud: [1200, 1800],
  wordmark: [1800, 2400],
  tagline: [2400, 3000],
  footer: [3000, 3500],
} as const;

export type Phase = keyof typeof PHASES;

/** Total length of the intro. The main window is revealed when it ends. */
export const SPLASH_MS = PHASES.footer[1];

/** With reduced motion, the final frame is shown this long, then the splash finishes. */
export const REDUCED_HOLD_MS = 1500;

/** Rounds a phase's start and length for CSS: returns [delay, duration] in ms. */
export function phaseTiming(phase: Phase): [number, number] {
  const [start, end] = PHASES[phase];
  return [start, end - start];
}
