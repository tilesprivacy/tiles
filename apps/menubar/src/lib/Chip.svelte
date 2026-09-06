<script lang="ts">
  interface Props {
    text: string;
    size?: "regular" | "small";
  }

  let { text, size = "regular" }: Props = $props();
</script>

<span class="chip" data-size={size}>{text}</span>

<style>
  /* the one cut echoing the switch's, chips only and never rows */
  .chip {
    flex: none;
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--cut)),
      calc(100% - var(--cut)) 100%,
      0 100%
    );
    padding: 3px 5px;
    background: var(--steel);
    color: var(--ash);
    font-family: var(--font-mono);
    font-size: var(--fs-chip);
    font-weight: 600;
    line-height: 1;
    letter-spacing: var(--tracking-chip);
    font-variant-numeric: tabular-nums;
  }

  /* rides a 13px word, so the box lands on its cap height and the cut comes
     down with it */
  .chip[data-size="small"] {
    --cut: 3px;
    padding: 2px 4px;
    font-size: 8px;
  }
</style>
