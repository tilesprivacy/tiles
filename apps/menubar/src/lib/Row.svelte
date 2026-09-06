<script lang="ts">
  import type { Snippet } from "svelte";
  import { focus } from "../focus.svelte";

  interface Props {
    /** names the value the row carries, when the value alone would not */
    key?: string;
    title?: string;
    sub?: string;
    size?: "regular" | "large";
    dimmed?: boolean;
    /** the row that leads somewhere, among rows that do not */
    tone?: "default" | "signal";
    mono?: boolean;
    /** the sub is an identifier or a reading, not prose */
    submono?: boolean;
    leading?: Snippet;
    /** rides the title rather than the row's right edge */
    inline?: Snippet;
    trailing?: Snippet;
    onselect?: () => void;
  }

  let {
    key,
    title,
    sub,
    size = "regular",
    dimmed = false,
    tone = "default",
    mono = false,
    submono = false,
    leading,
    inline,
    trailing,
    onselect,
  }: Props = $props();

  let row = $state<HTMLElement | null>(null);

  const live = $derived(onselect !== undefined && !dimmed);
  const active = $derived(row !== null && focus.active === row);

  // onselect is read lazily so a fresh closure from the parent does not
  // re-register the row on every render
  $effect(() => {
    const el = row;
    if (!el || !live) return;
    return focus.register(el, () => onselect?.());
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="row"
  bind:this={row}
  data-size={size}
  data-dimmed={dimmed}
  data-tone={tone}
  data-active={active}
  role={live ? "menuitem" : undefined}
  onclick={live ? onselect : undefined}
  onmouseenter={() => live && (focus.active = row)}
  onmouseleave={() => active && (focus.active = null)}
>
  {@render leading?.()}
  {#if key}<span class="row__key">{key}</span>{/if}
  <div class="row__main">
    <span class="row__line">
      {#if title}<span class="row__title" class:row__title--mono={mono}>{title}</span>{/if}
      {@render inline?.()}
    </span>
    {#if sub}<span class="row__sub" class:row__sub--mono={submono}>{sub}</span>{/if}
  </div>
  {@render trailing?.()}
</div>

<style>
  .row {
    --row-mark: var(--slate);

    position: relative;
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: var(--h-row);
    padding: 0 var(--pad-x);
  }

  .row[data-size="large"] {
    min-height: var(--h-row-lg);
  }

  .row[data-dimmed="true"] {
    opacity: 0.5;
  }

  /* the rail, the only focus indicator in the app. no background wash */
  .row::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: var(--rail-w);
    background: transparent;
    transition: background var(--dur-state) ease-out;
  }

  .row[data-active="true"] {
    --row-mark: var(--signal);
  }

  .row[data-active="true"]::before {
    background: var(--signal);
  }

  /* paint above ::before */
  .row > :global(*) {
    position: relative;
  }

  .row__key {
    flex: none;
    font-size: var(--fs-body);
    color: var(--slate);
  }

  .row__main {
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .row__line {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }

  .row__title,
  .row__sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row__title {
    font-size: var(--fs-title);
    color: var(--bone);
    transition: color var(--dur-state) ease-out;
  }

  .row__title--mono {
    font-family: var(--font-mono);
    font-size: var(--fs-mono);
    color: var(--ash);
  }

  .row__sub {
    font-size: var(--fs-body);
    color: var(--slate);
  }

  .row__sub--mono {
    font-family: var(--font-mono);
    font-size: var(--fs-mono);
    letter-spacing: 0.01em;
    color: var(--slate);
  }

  .row[data-tone="signal"] .row__title,
  .row[data-active="true"] .row__title {
    color: var(--signal);
  }
</style>
