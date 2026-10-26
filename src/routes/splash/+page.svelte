<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";
  import { FADE_OUT_MS, REDUCED_HOLD_MS, phaseTiming, timeline, type Timeline } from "$lib/splash/timeline";
  import { applyTheme } from "$lib/util/theme";

  // Geometry matches assets/tray-icons-v2/gen.py so the splash echoes the app icon.
  const CX = 128;
  const CY = 132;
  const RX = 118;
  const RY = 34;
  const ROT = -20;
  const START_ANGLE = 200;

  let satellite = $state({ x: 0, y: 0 });
  let still = $state(false);
  /** True once the content is fading to the background. */
  let leaving = $state(false);
  let finished = false;
  /** Which version plays. Set once the backend has answered; nothing animates before that. */
  let line = $state<Timeline | null>(null);
  let version = $state("");

  const ringTiming = $derived(line ? phaseTiming(line, "ring") : [0, 0]);
  const cloudTiming = $derived(line ? phaseTiming(line, "cloud") : [0, 0]);
  const wordTiming = $derived(line ? phaseTiming(line, "wordmark") : [0, 0]);
  const taglineTiming = $derived(line ? phaseTiming(line, "tagline") : [0, 0]);
  const footerTiming = $derived(line ? phaseTiming(line, "footer") : [0, 0]);

  function pointAt(angleDeg: number) {
    const t = (angleDeg * Math.PI) / 180;
    const r = (ROT * Math.PI) / 180;
    const x = RX * Math.cos(t);
    const y = RY * Math.sin(t);
    return {
      x: CX + x * Math.cos(r) - y * Math.sin(r),
      y: CY + x * Math.sin(r) + y * Math.cos(r),
    };
  }

  /**
   * Fades the content to the background, then hands over to the backend, which closes this
   * window. With reduced motion there is no fade: the hand-over happens at once.
   */
  async function finish() {
    if (finished) return;
    finished = true;
    leaving = true;
    if (!still) {
      await new Promise((resolve) => window.setTimeout(resolve, FADE_OUT_MS));
    }
    try {
      await api.splashFinished();
    } catch {
      // In a plain browser there is no backend to hand over to. Nothing else to do.
    }
  }

  onMount(() => {
    void api.getSettings().then((settings) => applyTheme(settings.theme)).catch(() => {});
    void api.appInfo().then((info) => (version = info.version)).catch(() => {});

    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    still = reduced;
    let timer = 0;
    let frame = 0;

    // The backend decides between the full and the short intro. In a plain browser there is no
    // backend, so the full one plays.
    void api
      .getSplashPlan()
      .then((plan) => (plan === "short" ? "short" : "full"))
      .catch(() => "full" as const)
      .then((version) => {
        if (finished) return;
        line = timeline(version);
        timer = window.setTimeout(finish, reduced ? REDUCED_HOLD_MS : line.sequenceMs);
        if (reduced) {
          satellite = pointAt(START_ANGLE);
        } else {
          const started = performance.now();
          const orbitMs = line.orbitMs;
          const tick = (now: number) => {
            const angle = START_ANGLE + ((now - started) / orbitMs) * 360;
            satellite = pointAt(angle);
            frame = requestAnimationFrame(tick);
          };
          frame = requestAnimationFrame(tick);
        }
      });

    return () => {
      window.clearTimeout(timer);
      cancelAnimationFrame(frame);
    };
  });
</script>

<svelte:head>
  <style>
    :global(html),
    :global(body) {
      background: transparent !important;
    }
  </style>
</svelte:head>

<svelte:window onkeydown={() => void finish()} />

<div
  class="splash"
  class:still
  class:leaving
  role="button"
  tabindex="0"
  aria-label="Skip intro"
  onclick={finish}
  onkeydown={(event) => {
    event.preventDefault();
    void finish();
  }}
>
  <div class="content" class:ready={line !== null} style:transition-duration="{FADE_OUT_MS}ms">
    <svg viewBox="0 0 256 256" class="mark" aria-hidden="true">
      <ellipse
        class="ring"
        cx={CX}
        cy={CY}
        rx={RX}
        ry={RY}
        transform="rotate({ROT} {CX} {CY})"
        pathLength="1"
        style:animation-delay="{ringTiming[0]}ms"
        style:animation-duration="{ringTiming[1]}ms"
      />
      <g class="cloud" style:animation-delay="{cloudTiming[0]}ms" style:animation-duration="{cloudTiming[1]}ms">
        <circle cx="70" cy="146" r="30" />
        <circle cx="116" cy="112" r="44" />
        <circle cx="168" cy="124" r="36" />
        <circle cx="194" cy="148" r="28" />
        <rect x="70" y="130" width="124" height="46" />
      </g>
      <circle class="satellite" cx={satellite.x} cy={satellite.y} r="11" />
    </svg>

    <p class="wordmark" style:animation-delay="{wordTiming[0]}ms" style:animation-duration="{wordTiming[1]}ms">Orbit</p>
    <p class="tagline" style:animation-delay="{taglineTiming[0]}ms" style:animation-duration="{taglineTiming[1]}ms">
      All your clouds, in one orbit.
    </p>
    <p class="footer" style:animation-delay="{footerTiming[0]}ms" style:animation-duration="{footerTiming[1]}ms">
      Powered by rclone{version ? ` · v${version}` : ""}
    </p>
  </div>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
  }
  /* Smooth ease-out curves only. Nothing overshoots, so nothing bounces. */
  :root {
    --ease-out: cubic-bezier(0.22, 1, 0.36, 1);
  }
  .splash {
    position: relative;
    height: 100vh;
    background: var(--splash-bg);
    border-radius: 28px;
    color: var(--splash-ink);
    cursor: pointer;
    user-select: none;
    overflow: hidden;
  }
  /*
   * Everything but the background sits in .content, so the whole sequence can fade out as one.
   * The children keep their own keyframe animations; those would override a per-element opacity,
   * which is why the wrapper is what fades.
   */
  .content {
    position: relative;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    transition: opacity 300ms ease-out; /* the duration is set inline from FADE_OUT_MS */
  }
  .splash.leaving .content {
    opacity: 0;
  }
  /* Nothing animates until the backend has said which version plays. */
  .content:not(.ready) * {
    animation-play-state: paused;
  }
  .mark {
    width: 180px;
    height: 180px;
    overflow: visible;
    margin-bottom: 4px;
  }
  .ring {
    fill: none;
    stroke: currentColor;
    stroke-width: 7;
    stroke-linecap: round;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: draw-ring 1200ms var(--ease-out) forwards;
  }
  .satellite {
    fill: currentColor;
    opacity: 0;
    animation: fade-in 400ms var(--ease-out) 200ms forwards;
  }
  .cloud {
    fill: currentColor;
    opacity: 0;
    transform-box: fill-box;
    transform-origin: center;
    animation: cloud-in 600ms var(--ease-out) forwards;
  }
  .wordmark {
    margin: 10px 0 0;
    font-family: "SF Pro Display", Inter, var(--font-ui);
    font-size: 34px;
    font-weight: 300;
    letter-spacing: 0.04em;
    opacity: 0;
    animation: rise 600ms var(--ease-out) forwards;
  }
  .tagline {
    margin: 0;
    font-family: var(--font-ui);
    font-size: 13px;
    font-weight: 400;
    letter-spacing: 0.01em;
    opacity: 0;
    animation: rise 600ms var(--ease-out) forwards;
  }
  .footer {
    position: absolute;
    bottom: 16px;
    left: 0;
    right: 0;
    margin: 0;
    text-align: center;
    font-family: var(--font-ui);
    font-size: 11px;
    color: var(--splash-muted);
    opacity: 0;
    animation: fade-in 500ms var(--ease-out) forwards;
  }

  /* Reduced motion: show the final frame at once. The timer in script still runs. */
  .splash.still *,
  .splash.still {
    animation: none !important;
    transition: none !important;
  }
  .splash.still .ring {
    stroke-dashoffset: 0;
  }
  .splash.still .satellite,
  .splash.still .cloud,
  .splash.still .wordmark,
  .splash.still .tagline,
  .splash.still .footer {
    opacity: 1;
    transform: none;
  }

  @keyframes draw-ring {
    to {
      stroke-dashoffset: 0;
    }
  }
  @keyframes fade-in {
    to {
      opacity: 1;
    }
  }
  @keyframes cloud-in {
    from {
      opacity: 0;
      transform: scale(0.92);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
