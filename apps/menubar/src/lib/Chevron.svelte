<script lang="ts">
  interface Props {
    dir?: "forward" | "back" | "up";
  }

  let { dir = "forward" }: Props = $props();
</script>

<svg class="chevron" data-dir={dir} viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
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

  .chevron[data-dir="back"] {
    transform: rotate(180deg);
    color: var(--ash);
  }

  .chevron[data-dir="up"] {
    transform: rotate(-90deg);
  }
</style>
