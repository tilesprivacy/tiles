<script lang="ts">
  import type { Session } from "../state.svelte";
  import Row from "./Row.svelte";
  import { relativeTime } from "./time";

  interface Props {
    sessions: Session[];
  }

  let { sessions }: Props = $props();

  /** no route opens a session yet, the row is live so the rail and the arrow
      keys already reach it */
  function open() {}
</script>

{#each sessions as session (session.id)}
  <!-- the name is the conversation's first prompt, so it carries the row -->
  <Row title={session.name} onselect={open}>
    {#snippet trailing()}
      <span class="when">{relativeTime(session.createdAt)}</span>
    {/snippet}
  </Row>
{/each}

<style>
  .when {
    flex: none;
    font-family: var(--font-mono);
    font-size: var(--fs-mono);
    font-variant-numeric: tabular-nums;
    color: var(--slate);
  }
</style>
