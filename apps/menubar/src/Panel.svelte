<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  import Stack from "./lib/Stack.svelte";
  import { focus } from "./focus.svelte";
  import { nav, type ViewId } from "./nav.svelte";
  import { connect } from "./state.svelte";
  import AccountView from "./views/AccountView.svelte";
  import ModelView from "./views/ModelView.svelte";
  import RootView from "./views/RootView.svelte";
  import SessionsView from "./views/SessionsView.svelte";

  let root: HTMLDivElement;

  onMount(() => {
    const disconnect = connect();

    // the stack animates its height, so this fires every frame of a push. one
    // call per frame, and the host ignores a height it already has
    let frame = 0;
    let sent = -1;
    const report = () => {
      frame = 0;
      const height = Math.ceil(root.getBoundingClientRect().height);
      if (height === 0 || height === sent) return;
      sent = height;
      void invoke("resize_panel", { height }).catch(() => {});
    };

    const observer = new ResizeObserver(() => {
      if (frame === 0) frame = requestAnimationFrame(report);
    });
    observer.observe(root);

    return () => {
      disconnect();
      observer.disconnect();
      if (frame !== 0) cancelAnimationFrame(frame);
    };
  });

  // a push leaves both views registered until the animation ends
  $effect(() => {
    void nav.top;
    focus.reset();
  });

  // keydown not keyup, the panel should be gone before the key comes back up
  function onkeydown(event: KeyboardEvent) {
    switch (event.key) {
      case "Escape":
        event.preventDefault();
        if (nav.depth > 0) {
          nav.pop();
        } else {
          void invoke("hide_panel").catch(() => {});
        }
        return;
      case "ArrowDown":
        event.preventDefault();
        focus.move(1);
        return;
      case "ArrowUp":
        event.preventDefault();
        focus.move(-1);
        return;
      case "Enter":
        if (
          event.defaultPrevented ||
          (event.target instanceof Element &&
            event.target.closest("button, a, input, select, textarea, [contenteditable='true']"))
        ) {
          return;
        }
        event.preventDefault();
        focus.activate();
        return;
    }
  }
</script>

<svelte:window {onkeydown} />

<div class="panel" bind:this={root}>
  <Stack>
    {#snippet view(id: ViewId)}
      {#if id === "account"}
        <AccountView />
      {:else if id === "sessions"}
        <SessionsView />
      {:else if id === "model"}
        <ModelView />
      {:else}
        <RootView />
      {/if}
    {/snippet}
  </Stack>
</div>

<style>
  .panel {
    width: 100%;
    border-radius: var(--radius-panel);
    /* the yellow masthead is a rect, the corners have to clip it */
    overflow: hidden;
    /* black on a dark desktop needs an edge of its own */
    box-shadow: inset 0 0 0 var(--hairline) var(--rule);
    background: var(--void);
  }
</style>
