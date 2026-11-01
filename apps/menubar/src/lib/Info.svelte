<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import type { Snippet } from "svelte";

  interface Props {
    text: string;
    /** richer tip, `text` still labels the button */
    children?: Snippet;
  }

  let { text, children }: Props = $props();

  let hovered = $state(false);
  let dismissed = $state(false);
  const open = $derived(hovered && !dismissed);

  // state, not :hover. the panel hides with no pointerleave and webkit keeps a stale :hover
  $effect(() => {
    const off = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (!focused) hovered = false;
    });
    return () => void off.then((unlisten) => unlisten());
  });

  // capture, so escape closes a showing tip rather than reaching the panel's back
  function onkeydowncapture(event: KeyboardEvent) {
    if (event.key !== "Escape" || !open) return;
    event.preventDefault();
    event.stopPropagation();
    dismissed = true;
  }
</script>

<svelte:window {onkeydowncapture} />

<button
  type="button"
  class="info"
  aria-label={text}
  data-open={open}
  tabindex="-1"
  onpointerenter={() => {
    hovered = true;
    dismissed = false;
  }}
  onpointerleave={() => (hovered = false)}
>
  <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
    <circle cx="6" cy="6" r="5.2" fill="none" stroke="currentColor" stroke-width="1.2" />
    <circle cx="6" cy="3.7" r="0.75" fill="currentColor" />
    <path d="M6 5.4v3.4" stroke="currentColor" stroke-width="1.2" />
  </svg>
  <span class="info__tip" aria-hidden="true">
    <!-- clip-path takes the drop shadow with it, so the halo is a sibling -->
    <span class="info__halo"></span>
    <span class="info__body">
      {#if children}{@render children()}{:else}{text}{/if}
    </span>
  </span>
</button>

<style>
  /* static, so the tip spans the navbar rather than starting at the icon */
  .info {
    flex: none;
    display: inline-flex;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--slate);
    outline: none;
    transition: color var(--dur-state) ease-out;
  }

  .info:hover {
    color: var(--bone);
  }

  .info__tip {
    position: absolute;
    left: var(--pad-x);
    right: var(--pad-x);
    top: 100%;
    z-index: 3;
    color: var(--ash);
    font-size: var(--fs-label);
    line-height: 1.35;
    white-space: normal;
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--dur-state) ease-out;
  }

  .info__halo {
    position: absolute;
    inset: 2px;
    box-shadow: var(--lift);
  }

  .info__body {
    position: relative;
    display: block;
    padding: 7px 10px;
    clip-path: var(--clip-cut);
    background: var(--steel);
  }

  .info[data-open="true"] .info__tip {
    opacity: 1;
  }
</style>
