<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    children: Snippet;
  }

  let { children }: Props = $props();
</script>

<p class="note">{@render children()}</p>

<style>
  .note {
    padding: 2px var(--pad-x);
    color: var(--slate);
    font-size: var(--fs-body);
    line-height: 1.35;
  }
</style>
