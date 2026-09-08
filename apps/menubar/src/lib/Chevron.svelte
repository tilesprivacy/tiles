<script lang="ts">
  interface Props {
    back?: boolean;
  }

  let { back = false }: Props = $props();
</script>

<svg
  class="chevron"
  data-back={back}
  viewBox="0 0 12 12"
  width="12"
  height="12"
  aria-hidden="true"
>
  <path
    d="M4.5 2 8.5 6l-4 4"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="square"
  />
</svg>

<style>
  .chevron {
    flex: none;
    color: var(--row-mark, var(--slate));
    transition: color var(--dur-state) ease-out;
  }

  .chevron[data-back="true"] {
    transform: rotate(180deg);
    color: var(--ash);
  }
</style>
