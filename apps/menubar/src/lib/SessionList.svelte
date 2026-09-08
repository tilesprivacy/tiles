<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  import type { Session } from "../state.svelte";
  import Row from "./Row.svelte";
  import { relativeTime } from "./time";

  interface Props {
    sessions: Session[];
  }

  let { sessions }: Props = $props();

  function open(id: string) {
    void invoke("open_session", { id }).catch(() => {});
  }
</script>

{#each sessions as session (session.id)}
  <!-- the name is the conversation's first prompt, so it carries the row -->
  <Row title={session.name} onselect={() => open(session.id)}>
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
