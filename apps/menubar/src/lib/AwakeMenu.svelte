<script lang="ts">
  interface Props {
    session: "none" | "running" | "paused";
    /** new or resumed sessions need mains or enough battery */
    available: boolean;
    note: string;
    onpick: (seconds: number | null) => void;
    onpause: () => void;
    onresume: () => void;
    onstop: () => void;
  }

  let { session, available, note, onpick, onpause, onresume, onstop }: Props = $props();

  const DURATIONS: { label: string; seconds: number | null }[] = [
    { label: "15 minutes", seconds: 15 * 60 },
    { label: "30 minutes", seconds: 30 * 60 },
    { label: "1 hour", seconds: 60 * 60 },
    { label: "2 hours", seconds: 2 * 60 * 60 },
    { label: "4 hours", seconds: 4 * 60 * 60 },
    { label: "Indefinitely", seconds: null },
  ];
</script>

<div class="menu">
  <!-- clip-path takes the drop shadow with it, so the halo is a sibling -->
  <span class="menu__halo"></span>

  <div class="menu__list" role="menu">
    <p class="menu__note" data-alert={!available}>{note}</p>
    <div class="menu__rule"></div>

    {#if session !== "none"}
      {#if session === "running"}
        <button class="menu__item menu__item--lead" role="menuitem" onclick={onpause}>
          Pause
        </button>
      {:else}
        <button
          class="menu__item menu__item--lead"
          role="menuitem"
          disabled={!available}
          onclick={onresume}
        >
          Resume
        </button>
      {/if}
      <button class="menu__item menu__item--lead" role="menuitem" onclick={onstop}>
        Turn off
      </button>
      <div class="menu__rule"></div>
    {/if}

    {#each DURATIONS as duration (duration.label)}
      <button
        class="menu__item"
        role="menuitem"
        disabled={!available}
        onclick={() => onpick(duration.seconds)}
      >
        {duration.label}
      </button>
    {/each}
  </div>
</div>

<style>
  .menu {
    position: absolute;
    right: 0;
    bottom: calc(100% + 6px);
    z-index: 2;
    min-width: 100%;
  }

  .menu__halo {
    position: absolute;
    inset: 2px;
    box-shadow: var(--lift);
  }

  .menu__list {
    position: relative;
    display: flex;
    flex-direction: column;
    padding: 3px 0;
    clip-path: var(--clip-cut);
    background: var(--steel);
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

  .menu__item--lead {
    color: var(--bone);
  }

  .menu__item:hover {
    color: var(--signal);
  }

  .menu__item:disabled {
    color: var(--slate);
    opacity: 0.45;
  }

  .menu__note {
    width: 190px;
    margin: 0;
    padding: 7px var(--pad-x);
    color: var(--slate);
    font-family: var(--font-ui);
    font-size: var(--fs-meta);
    line-height: 1.35;
  }

  .menu__note[data-alert="true"] {
    color: var(--alert);
  }

  .menu__rule {
    margin: 3px 0;
    height: var(--hairline);
    background: var(--rule);
  }
</style>
