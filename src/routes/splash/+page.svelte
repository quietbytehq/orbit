<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "$lib/api";

  // Geometry matches assets/tray-icons-v2/gen.py so the splash echoes the app icon.
  const CX = 128;
  const CY = 132;
  const RX = 118;
  const RY = 34;
  const ROT = -20;
  const ORBIT_MS = 3200;
  const START_ANGLE = 200;

  const FULL_MS = 1500;
  const REDUCED_MS = 400;

  let satellite = $state({ x: 0, y: 0 });
  let finished = false;

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

  async function finish() {
    if (finished) return;
    finished = true;
    try {
      await api.splashFinished();
    } catch {
      // In a plain browser there is no backend to hand over to. Nothing else to do.
    }
  }

  onMount(() => {
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const timer = window.setTimeout(finish, reduced ? REDUCED_MS : FULL_MS);

    let frame = 0;
    if (!reduced) {
      const started = performance.now();
      const tick = (now: number) => {
        const angle = START_ANGLE + ((now - started) / ORBIT_MS) * 360;
        satellite = pointAt(angle);
        frame = requestAnimationFrame(tick);
      };
      frame = requestAnimationFrame(tick);
    } else {
      satellite = pointAt(START_ANGLE);
    }

    return () => {
      window.clearTimeout(timer);
      cancelAnimationFrame(frame);
    };
  });

  function onKey(event: KeyboardEvent) {
    if (event.key === "Enter" || event.key === " " || event.key === "Escape") {
      void finish();
    }
  }
</script>

<svelte:head>
  <style>
    :global(html),
    :global(body) {
      background: transparent !important;
    }
  </style>
</svelte:head>

<div class="splash" role="button" tabindex="0" aria-label="Skip intro" onclick={finish} onkeydown={onKey}>
  <svg viewBox="0 0 256 256" class="mark" aria-hidden="true">
    <ellipse
      class="ring"
      cx={CX}
      cy={CY}
      rx={RX}
      ry={RY}
      transform="rotate({ROT} {CX} {CY})"
      pathLength="1"
    />
    <g class="cloud">
      <circle cx="70" cy="146" r="30" />
      <circle cx="116" cy="112" r="44" />
      <circle cx="168" cy="124" r="36" />
      <circle cx="194" cy="148" r="28" />
      <rect x="70" y="130" width="124" height="46" />
    </g>
    <circle class="satellite" cx={satellite.x} cy={satellite.y} r="11" />
  </svg>
  <p class="wordmark">Orbit</p>
</div>

<style>
  :global(html),
  :global(body) {
    background: transparent;
  }
  .splash {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    background: var(--splash-bg);
    border-radius: 28px;
    color: var(--splash-ink);
    cursor: pointer;
    user-select: none;
    backdrop-filter: blur(30px) saturate(180%);
    -webkit-backdrop-filter: blur(30px) saturate(180%);
  }
  .mark {
    width: 220px;
    height: 220px;
    overflow: visible;
  }
  .ring {
    fill: none;
    stroke: currentColor;
    stroke-width: 7;
    stroke-linecap: round;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    animation: draw-ring 0.6s ease-out 0.05s forwards;
  }
  .satellite {
    fill: currentColor;
    opacity: 0;
    animation: fade-in 0.3s ease-out 0.45s forwards;
  }
  .cloud {
    fill: currentColor;
    opacity: 0;
    animation: fade-in 0.5s ease-out 0.9s forwards;
  }
  .wordmark {
    margin: 0;
    font-family: var(--font-ui);
    font-size: 18px;
    font-weight: 600;
    letter-spacing: 0.2px;
    opacity: 0;
    animation: fade-in 0.4s ease-out 1.1s forwards;
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
</style>
