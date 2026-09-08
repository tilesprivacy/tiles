<script module lang="ts">
  /** running is the only mode that floods, so it is the only one read at a glance */
  export type Mode = "down" | "connecting" | "starting" | "idle" | "running";
</script>

<script lang="ts">
  import Chip from "./Chip.svelte";
  import Mark from "./Mark.svelte";
  import Switch from "./Switch.svelte";

  interface Props {
    mode: Mode;
    on: boolean;
    /** asked for, not yet arrived */
    pending: boolean;
    disabled: boolean;
    ontoggle: () => void;
  }

  let { mode, on, pending, disabled, ontoggle }: Props = $props();

  const running = $derived(mode === "running");
</script>

<header class="masthead" data-mode={mode}>
  <span class="masthead__brand">
    <span class="masthead__glyph"><Mark size={22} /></span>
    <span class="masthead__word">Tiles</span>
  </span>
  <span class="masthead__badge"><Chip text="alpha" size="small" /></span>
  <Switch
    {on}
    {pending}
    {disabled}
    size="small"
    invert={running}
    label="Inference"
    onchange={ontoggle}
  />
</header>

<style>
  .masthead {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: var(--h-masthead);
    padding: 0 var(--pad-x);
    background: var(--void);
    color: var(--bone);
    transition:
      background var(--dur-state) ease-out,
      color var(--dur-state) ease-out;
  }

  /* the whole block is the status light, which is why there is no dot anywhere */
  .masthead[data-mode="running"] {
    background: var(--signal);
    color: var(--void);
  }

  .masthead[data-mode="down"] {
    color: var(--slate);
  }

  .masthead[data-mode="down"]::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: var(--rail-w);
    background: var(--alert);
  }

  /* the mark stays about twice the wordmark's cap height and the two stay
     tight, so they read as one lockup */
  .masthead__brand {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  /* the ground already carries the state, so the mark just takes the
     masthead's colour: black on the flood, white off it */
  .masthead__glyph {
    display: flex;
    flex: none;
  }

  .masthead__word {
    font-size: var(--fs-title);
    font-weight: 600;
    letter-spacing: var(--tracking-display);
  }

  .masthead__badge {
    display: flex;
    flex: none;
    margin-right: auto;
  }

  /* the chip's grey would sit heavy on the yellow, this is the switch's
     inverted track instead */
  .masthead[data-mode="running"] .masthead__badge :global(.chip) {
    background: rgba(0, 0, 0, 0.12);
    color: rgba(0, 0, 0, 0.6);
  }
</style>
