<script lang="ts">
  import glyph from "../assets/marks/tiles.svg?raw";

  interface Props {
    size?: number;
  }

  let { size = 20 }: Props = $props();
</script>

<span class="mark" style="--size: {size}px" aria-hidden="true">{@html glyph}</span>

<style>
  .mark {
    flex: none;
    display: block;
    width: var(--size);
    height: var(--size);
    color: inherit;
  }

  .mark :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
