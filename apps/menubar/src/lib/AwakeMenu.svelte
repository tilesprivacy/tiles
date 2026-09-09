<script lang="ts">
  interface Props {
    /** a session exists, running or held */
    session: "none" | "running" | "paused";
    /** `null` is the open ended one */
    onpick: (seconds: number | null) => void;
    onpause: () => void;
    onresume: () => void;
    onstop: () => void;
  }

  let { session, onpick, onpause, onresume, onstop }: Props = $props();

  const DURATIONS: { label: string; seconds: number | null }[] = [
    { label: "15 minutes", seconds: 15 * 60 },
    { label: "30 minutes", seconds: 30 * 60 },
    { label: "1 hour", seconds: 60 * 60 },
    { label: "2 hours", seconds: 2 * 60 * 60 },
    { label: "4 hours", seconds: 4 * 60 * 60 },
    { label: "Indefinitely", seconds: null },
  ];
</script>

<div class="menu" role="menu">
  {#if session !== "none"}
    <!-- what to do with the session already going, above the lengths that
         would replace it -->
    {#if session === "running"}
      <button class="menu__item menu__item--lead" role="menuitem" onclick={onpause}>
        Pause
      </button>
    {:else}
      <button class="menu__item menu__item--lead" role="menuitem" onclick={onresume}>
        Resume
      </button>
    {/if}
    <button class="menu__item menu__item--lead" role="menuitem" onclick={onstop}>
      Turn off
    </button>
    <div class="menu__rule"></div>
  {/if}

  {#each DURATIONS as duration (duration.label)}
    <button class="menu__item" role="menuitem" onclick={() => onpick(duration.seconds)}>
      {duration.label}
    </button>
  {/each}
</div>

<style>
  /* opens upward, the footer is the last thing in the panel. anchored on the
     plate's right edge so it does not hang over Quit */
  .menu {
    position: absolute;
    right: 0;
    bottom: calc(100% + 6px);
    z-index: 2;
    display: flex;
    flex-direction: column;
    /* the plate is the anchor, so the list is at least as wide as it */
    min-width: 100%;
    padding: 3px 0;
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--cut)),
      calc(100% - var(--cut)) 100%,
      0 100%
    );
    background: var(--steel);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.55);
  }

  .menu__item {
    flex: none;
    border: none;
    background: transparent;
    padding: 5px var(--pad-x);
    color: var(--ash);
    font-family: var(--font-ui);
    font-size: var(--fs-body);
    line-height: 1;
    text-align: left;
    white-space: nowrap;
    transition: color var(--dur-state) ease-out;
  }

  /* acting on the running session is the reason the menu was opened */
  .menu__item--lead {
    color: var(--bone);
  }

  .menu__item:hover {
    color: var(--signal);
  }

  .menu__rule {
    margin: 3px 0;
    height: var(--hairline);
    background: var(--rule);
  }
</style>
