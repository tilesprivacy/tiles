<script lang="ts">
  import type { Snippet } from "svelte";
  import { nav, type ViewId } from "../nav.svelte";

  let { view }: { view: Snippet<[ViewId]> } = $props();

  // the view sliding out, held only for the length of the transition
  let leaving = $state<{ id: ViewId; dir: "push" | "pop" } | null>(null);
  // views mount at their start transform and stay there until this flips, see
  // the two-frame wait below
  let running = $state(false);
  let height = $state<number | null>(null);

  // plain, not state: the effect below writes them, and depending on its own
  // writes would make it run itself a second time every push
  let previous = nav.top;
  let previousDepth = nav.depth;
  let current: HTMLDivElement;

  $effect(() => {
    const next = nav.top;
    const depth = nav.depth;
    if (next === previous) return;

    leaving = { id: previous, dir: depth > previousDepth ? "push" : "pop" };
    running = false;
    previous = next;
    previousDepth = depth;
  });

  // a transition added in the same paint as the element can have its start
  // value skipped, which renders both views untransformed for a frame. two
  // frames is the browser actually painting the start
  $effect(() => {
    if (leaving === null || running) return;

    let second = 0;
    const first = requestAnimationFrame(() => {
      second = requestAnimationFrame(() => (running = true));
    });
    return () => {
      cancelAnimationFrame(first);
      cancelAnimationFrame(second);
    };
  });

  $effect(() => {
    if (leaving === null) return;

    const ms = Number.parseFloat(
      getComputedStyle(document.documentElement).getPropertyValue("--dur-push"),
    );
    const timer = setTimeout(() => (leaving = null), ms);
    return () => clearTimeout(timer);
  });

  // measured live, so a view that changes size resizes the panel too
  $effect(() => {
    void nav.top;
    const observer = new ResizeObserver(() => {
      height = current.getBoundingClientRect().height;
    });
    observer.observe(current);
    return () => observer.disconnect();
  });
</script>

<div
  class="stack"
  data-measured={height !== null}
  style={height === null ? undefined : `height: ${height}px`}
>
  {#if leaving}
    <div
      class="view"
      data-anim={leaving.dir === "push" ? "out-to-left" : "out-to-right"}
      data-running={running}
      aria-hidden="true"
    >
      {@render view(leaving.id)}
    </div>
  {/if}

  <div
    bind:this={current}
    class="view"
    data-anim={leaving ? (leaving.dir === "push" ? "in-from-right" : "in-from-left") : undefined}
    data-running={running}
  >
    {@render view(nav.top)}
  </div>
</div>

<style>
  .stack {
    position: relative;
    overflow: hidden;
    transition: height var(--dur-push) var(--ease-push);
  }

  /* the first measurement is the panel's real height, not a change to animate */
  .stack[data-measured="false"] {
    transition: none;
  }

  /* opaque, the view on top has to hide the one travelling under it rather
     than letting both render through each other */
  .view {
    width: 100%;
    background: var(--void);
  }

  .view[data-anim] {
    position: absolute;
    top: 0;
    left: 0;
  }

  /* whichever view is heading for the right edge is the one on top, so a push
     slides over the old view and a pop uncovers it */
  .view[data-anim="in-from-right"],
  .view[data-anim="out-to-right"] {
    z-index: 1;
  }

  .view[data-anim][data-running="true"] {
    transition: transform var(--dur-push) var(--ease-push);
  }

  /* the outgoing view trails rather than matching the incoming one, the same
     parallax AppKit uses */
  .view[data-anim="in-from-right"] {
    transform: translateX(100%);
  }
  .view[data-anim="out-to-left"] {
    transform: translateX(0);
  }
  .view[data-anim="in-from-left"] {
    transform: translateX(-38%);
  }
  .view[data-anim="out-to-right"] {
    transform: translateX(0);
  }

  .view[data-anim="in-from-right"][data-running="true"],
  .view[data-anim="in-from-left"][data-running="true"] {
    transform: translateX(0);
  }
  .view[data-anim="out-to-left"][data-running="true"] {
    transform: translateX(-38%);
  }
  .view[data-anim="out-to-right"][data-running="true"] {
    transform: translateX(100%);
  }
</style>
