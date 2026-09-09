<script lang="ts">
  interface Props {
    /** the assertion is held, and the cup is steaming */
    active: boolean;
  }

  let { active }: Props = $props();
</script>

<!-- sized against the label beside it, not the plate, which sets its own height -->
<svg
  class="cup"
  viewBox="0 0 12 12"
  width="15"
  height="15"
  aria-hidden="true"
  fill="none"
  stroke="currentColor"
  stroke-width="1"
>
  <!-- the cup owns the box and sits centred in it, so the steam overhangs the
       rim rather than a third of the height being held back for it -->
  <path d="M1.8 3h6.4l-.9 6h-4.6z" />
  <!-- close to a half circle, or the opening closes up at this size and the
       handle reads as a blob on the side -->
  <path d="M8.1 4.4h.9a1.4 1.4 0 0 1 0 2.8h-1" />
  {#if active}
    <!-- butt caps, square ones overhang half a stroke and the box clips them -->
    <path d="M4.3 .5v1.6" />
    <path d="M6.6 .3v1.8" />
  {/if}
</svg>

<style>
  /* block, or the inline baseline adds a descender the plate has to pad for */
  .cup {
    display: block;
    color: inherit;
  }
</style>
