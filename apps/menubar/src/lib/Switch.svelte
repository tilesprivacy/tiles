<script lang="ts">
  interface Props {
    on: boolean;
    disabled?: boolean;
    /** a request is out and the daemon has not answered yet */
    pending?: boolean;
    /** the ground behind it is yellow, so the lit cell has to be the dark half */
    invert?: boolean;
    /** small sits inline with a row's title, regular carries a masthead */
    size?: "regular" | "small";
    /** the halo reads as a button, and is wrong on anything this side of a row */
    glow?: boolean;
    label: string;
    onchange: () => void;
  }

  let {
    on,
    disabled = false,
    pending = false,
    invert = false,
    size = "regular",
    glow = true,
    label,
    onchange,
  }: Props = $props();

  /** the runner rides the middle of the 1px frame, so every point is offset a half */
  const BOX = {
    regular: { w: 53, h: 22, cut: 4, seam: 26.5 },
    small: { w: 37, h: 16, cut: 3, seam: 18.5 },
  } as const;

  const box = $derived(BOX[size]);
  const edge = $derived(
    `M0.5 0.5 H${box.w - 0.5} V${box.h - box.cut} L${box.w - box.cut} ${box.h - 0.5} H0.5 Z`,
  );
</script>

<!-- the frame and the seam are one surface the cells are inset into, rather
     than borders on the cells, so a lit cell can never paint over its own edge -->
<button
  class="switch"
  role="switch"
  aria-checked={on}
  aria-busy={pending}
  aria-label={label}
  data-on={on}
  data-pending={pending}
  data-invert={invert}
  data-size={size}
  data-glow={glow}
  {disabled}
  onclick={onchange}
>
    <!-- a plain rect a shade inside the chip, so its shadow reaches into the
         chamfer's cut instead of stopping at a border box the cut is inside of -->
    <span class="switch__halo"></span>
  <span class="switch__chip">
    <span class="switch__cells">
      <span class="switch__cell">0</span>
      <span class="switch__cell">1</span>
    </span>
    <!-- the light runs the frame, splits at the top of the seam, and the two
         halves land on the bottom of the seam together -->
    {#if pending}
      <svg class="switch__run" viewBox="0 0 {box.w} {box.h}" aria-hidden="true">
        <path class="switch__run-edge" d={edge} />
        <line class="switch__run-seam" x1={box.seam} y1="0.5" x2={box.seam} y2={box.h - 0.5} />
      </svg>
    {/if}
  </span>
</button>

<style>
  .switch {
    /* two cells, one seam, two frame edges. integer cells keep the seam
       landing on a whole pixel at either size */
    --cell: 25px;
    --h: 22px;
    --chamfer: 4px;
    --fs: 10px;

    --frame: rgba(255, 255, 255, 0.22);
    --cell-bg: var(--steel);
    --cell-fg: #5c5c64;
    --lit-bg: var(--signal);
    --lit-fg: var(--void);
    /* never a filter: a filter that is not `none` builds a render surface in
       every state, and webkit clips it at the fold and snaps it when it grows.
       the halo below casts this instead */
    --glow: 0 0 0 rgba(247, 255, 97, 0), 0 0 0 rgba(247, 255, 97, 0);

    position: relative;
    flex: none;
    width: calc(var(--cell) * 2 + 3px);
    height: var(--h);
    padding: 0;
    border: none;
    background: none;
  }

  .switch__halo {
    position: absolute;
    inset: 2px;
    box-shadow: var(--glow);
    transition: box-shadow var(--dur-state) ease-out;
  }

  /* the row's title is 13px, so this sits just over it */
  .switch[data-size="small"] {
    --cell: 17px;
    --h: 16px;
    --chamfer: 3px;
    --fs: 8px;
  }

  .switch[data-glow="false"] .switch__halo {
    display: none;
  }

  .switch__chip {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: var(--frame);
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--chamfer)),
      calc(100% - var(--chamfer)) 100%,
      0 100%
    );
    transition: background var(--dur-state) ease-out;
  }

  /* one px in on every side and one px between, so the frame and the seam are
     the same line. the inner chamfer is one shorter, which holds the cut
     parallel to the outer one */
  .switch__cells {
    position: absolute;
    inset: 1px;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1px;
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--chamfer) + 1px),
      calc(100% - var(--chamfer) + 1px) 100%,
      0 100%
    );
  }

  .switch__cell {
    display: grid;
    place-items: center;
    background: var(--cell-bg);
    color: var(--cell-fg);
    font-family: var(--font-mono);
    font-size: var(--fs);
    /* the two glyphs have to sit on the same advance or the seam looks off */
    font-variant-numeric: tabular-nums;
    line-height: 1;
    transition:
      background var(--dur-state) ease-out,
      color var(--dur-state) ease-out;
  }

  .switch[data-on="false"]:not([data-pending="true"]) .switch__cell:first-child,
  .switch[data-on="true"]:not([data-pending="true"]) .switch__cell:last-child {
    background: var(--lit-bg);
    color: var(--lit-fg);
  }

  /* every rule here carries :not([data-pending]) so the pending state wins on
     its own rather than on specificity, and every inverted rule carries one
     attribute more than the rule it has to beat */
  .switch[data-on="true"]:not([data-pending="true"]) {
    --frame: rgba(247, 255, 97, 0.5);
    --glow: 0 0 8px rgba(247, 255, 97, 0.55), 0 0 18px rgba(247, 255, 97, 0.28);
  }

  /* it has to read as a button before it is pressed, not only after */
  .switch:not([data-pending="true"]):hover:not(:disabled) {
    --frame: rgba(255, 255, 255, 0.4);
    --glow: 0 0 6px rgba(247, 255, 97, 0.35), 0 0 0 rgba(247, 255, 97, 0);
  }

  .switch[data-on="true"]:not([data-pending="true"]):hover:not(:disabled) {
    --frame: rgba(247, 255, 97, 0.8);
    --glow: 0 0 12px rgba(247, 255, 97, 0.75), 0 0 24px rgba(247, 255, 97, 0.4);
  }

  /* on the lit masthead the chip is the dark object and the signal is whichever
     half is live, which is the same reading as on black. a midtone cell only
     muddied it, and a halo on yellow reads as a smudge, so there is none at rest */
  .switch[data-invert="true"] {
    --frame: var(--void);
    --cell-bg: var(--void);
    --cell-fg: rgba(247, 255, 97, 0.38);
    --lit-bg: var(--signal);
    --lit-fg: var(--void);
    --glow: 0 0 0 rgba(0, 0, 0, 0), 0 0 0 rgba(0, 0, 0, 0);
  }

  .switch[data-invert="true"][data-on="true"]:not([data-pending="true"]) {
    --frame: var(--void);
    --glow: 0 0 0 rgba(0, 0, 0, 0), 0 0 0 rgba(0, 0, 0, 0);
  }

  /* yellow on yellow is nothing, so the hover lift is the plate coming off the
     ground rather than a brighter edge */
  .switch[data-invert="true"]:not([data-pending="true"]):hover:not(:disabled),
  .switch[data-invert="true"][data-on="true"]:not([data-pending="true"]):hover:not(:disabled) {
    --frame: var(--void);
    --cell-fg: rgba(247, 255, 97, 0.62);
    --glow: 0 0 6px rgba(0, 0, 0, 0.45), 0 0 0 rgba(0, 0, 0, 0);
  }

  .switch:disabled {
    opacity: 0.35;
  }

  /* neither cell claims the new state until the daemon confirms it, which is
     also what makes the light legible */
  .switch[data-pending="true"] {
    --glow: 0 0 0 rgba(247, 255, 97, 0), 0 0 0 rgba(247, 255, 97, 0);
  }

  .switch__run {
    position: absolute;
    inset: 0;
    fill: none;
    stroke: var(--signal);
    stroke-width: 1;
  }

  /* the plate is black and the ground is yellow, so neither of them is what
     the light can be */
  .switch[data-invert="true"] .switch__run {
    stroke: var(--bone);
  }

  /* one dash per lap, so the gap is the rest of the path. the seam holds off
     the path until the lap reaches its top, then runs slow enough that both
     leading edges land on the bottom of the seam in the same frame */
  .switch[data-size="regular"] .switch__run-edge {
    stroke-dasharray: 14 129.95;
    animation: edge-r 1.4s linear infinite;
  }

  .switch[data-size="regular"] .switch__run-seam {
    stroke-dasharray: 7 21;
    animation: seam-r 1.4s linear infinite;
  }

  .switch[data-size="small"] .switch__run-edge {
    stroke-dasharray: 10 90.54;
    animation: edge-s 1.4s linear infinite;
  }

  .switch[data-size="small"] .switch__run-seam {
    stroke-dasharray: 5 15;
    animation: seam-s 1.4s linear infinite;
  }

  @keyframes edge-r {
    from {
      stroke-dashoffset: 14px;
    }
    to {
      stroke-dashoffset: -129.95px;
    }
  }

  /* 18.06% is the seam's top along the lap, 67.35% its bottom */
  @keyframes seam-r {
    0%,
    18.06% {
      stroke-dashoffset: 7px;
      opacity: 1;
    }
    67.35% {
      stroke-dashoffset: -14px;
      opacity: 1;
    }
    70%,
    100% {
      stroke-dashoffset: -14px;
      opacity: 0;
    }
  }

  @keyframes edge-s {
    from {
      stroke-dashoffset: 10px;
    }
    to {
      stroke-dashoffset: -90.54px;
    }
  }

  @keyframes seam-s {
    0%,
    17.9% {
      stroke-dashoffset: 5px;
      opacity: 1;
    }
    67.18% {
      stroke-dashoffset: -10px;
      opacity: 1;
    }
    70%,
    100% {
      stroke-dashoffset: -10px;
      opacity: 0;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .switch__halo,
    .switch__chip,
    .switch__cell {
      transition: none;
    }

    /* a lit frame says the same thing without the travel */
    .switch__run {
      display: none;
    }

    .switch[data-pending="true"] .switch__chip {
      background: rgba(247, 255, 97, 0.45);
    }
  }
</style>
