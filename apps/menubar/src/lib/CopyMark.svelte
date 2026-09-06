<script lang="ts">
  interface Props {
    /** the write landed, and the row says so for a beat */
    copied: boolean;
  }

  let { copied }: Props = $props();
</script>

<!-- the row itself is the click target, this only names what the click does -->
<svg
  class="copy"
  data-copied={copied}
  viewBox="0 0 12 12"
  width="12"
  height="12"
  aria-hidden="true"
  fill="none"
  stroke="currentColor"
  stroke-width="1.2"
>
  {#if copied}
    <path d="M2.5 6.3 4.8 8.7 9.5 3.5" stroke-linecap="square" />
  {:else}
    <path d="M4.2 4.2h5.3v5.3H4.2z" />
    <path d="M7.8 2.5H2.5v5.3" />
  {/if}
</svg>

<style>
  .copy {
    flex: none;
    color: var(--row-mark, var(--slate));
    transition: color var(--dur-state) ease-out;
  }

  .copy[data-copied="true"] {
    color: var(--signal);
  }
</style>
