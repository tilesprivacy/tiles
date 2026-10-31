<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    label?: string;
    /** the zone's subject is unavailable, not merely empty */
    dimmed?: boolean;
    children: Snippet;
  }

  let { label, dimmed = false, children }: Props = $props();
</script>

<section class="zone" data-dimmed={dimmed}>
  <!-- full bleed but for the panel's own ring, which it would otherwise
       double up on at both ends -->
  <div class="zone__rule"></div>
  {#if label}<h2 class="zone__label">{label}</h2>{/if}
  {@render children()}
</section>

<style>
  .zone {
    padding-bottom: 7px;
    transition: opacity var(--dur-state) ease-out;
  }

  /* the rows keep their own dimming, this is the label and the rule */
  .zone[data-dimmed="true"] {
    opacity: 0.45;
  }

  .zone__rule {
    height: var(--hairline);
    margin: 0 var(--hairline) 9px;
    background: var(--rule);
  }

  .zone__label {
    padding: 0 var(--pad-x) 5px;
    font-size: var(--fs-label);
    font-weight: 500;
    color: var(--slate);
  }
</style>
