/**
 * Splash timeline. The base sequence is 3.5 s of content; the two versions that actually play
 * are scaled from it: the full intro (about 5.5 s, first launch and after an update) and the
 * short one (1.5 s, every other launch). Each phase starts when the previous one ends.
 */
export const BASE_PHASES = {
  ring: [0, 1200],
  cloud: [1200, 1800],
  wordmark: [1800, 2400],
  tagline: [2400, 3000],
  footer: [3000, 3500],
} as const;

export type Phase = keyof typeof BASE_PHASES;

/** Which version plays. Mirrors `SplashPlan` without "off", which never reaches the page. */
export type SplashVersion = "full" | "short";

/** Length of the content sequence for each version, in ms. */
export const SEQUENCE_MS: Record<SplashVersion, number> = { full: 5500, short: 1500 };

/**
 * The content fades to the splash background over this long before the window closes, so the
 * cut to the main window is a soft one. A skip (click or key) runs the same fade.
 */
export const FADE_OUT_MS = 300;

/** With reduced motion, the final frame is shown this long, then the splash finishes. */
export const REDUCED_HOLD_MS = 1500;

/** Base length of the content sequence before scaling. */
export const BASE_MS = BASE_PHASES.footer[1];

export interface Timeline {
  version: SplashVersion;
  phases: Record<Phase, [start: number, end: number]>;
  /** Length of the content sequence. The fade-out starts when it ends. */
  sequenceMs: number;
  /** Moment the splash hands over to the backend: sequence plus fade-out. */
  totalMs: number;
  /** One orbit of the satellite, scaled with the sequence so it moves at a matching pace. */
  orbitMs: number;
}

/** Scales the base phases proportionally to the chosen version's length. */
export function timeline(version: SplashVersion): Timeline {
  const factor = SEQUENCE_MS[version] / BASE_MS;
  const phases = Object.fromEntries(
    (Object.keys(BASE_PHASES) as Phase[]).map((phase) => {
      const [start, end] = BASE_PHASES[phase];
      return [phase, [Math.round(start * factor), Math.round(end * factor)]];
    }),
  ) as Record<Phase, [number, number]>;
  return {
    version,
    phases,
    sequenceMs: SEQUENCE_MS[version],
    totalMs: SEQUENCE_MS[version] + FADE_OUT_MS,
    orbitMs: Math.round(3200 * factor),
  };
}

/** Rounds a phase's start and length for CSS: returns [delay, duration] in ms. */
export function phaseTiming(line: Timeline, phase: Phase): [number, number] {
  const [start, end] = line.phases[phase];
  return [start, end - start];
}
