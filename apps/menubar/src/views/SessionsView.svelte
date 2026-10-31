<script lang="ts">
  import Navbar from "../lib/Navbar.svelte";
  import Row from "../lib/Row.svelte";
  import SessionList from "../lib/SessionList.svelte";
  import Zone from "../lib/Zone.svelte";
  import { nav } from "../nav.svelte";
  import { sessions } from "../state.svelte";

  const all = $derived(sessions.value.state === "ready" ? sessions.value.sessions : []);
</script>

<Navbar title="Chats" onback={() => nav.pop()} />

<Zone label="Recent">
  {#if all.length > 0}
    <!-- the panel clamps to the screen and clips whatever is past it, so the
         list has to run out of room before the window does -->
    <div class="list">
      <SessionList sessions={all} />
    </div>
  {:else}
    <Row title="—" dimmed />
  {/if}
</Zone>

<style>
  .list {
    max-height: 280px;
    overflow-y: auto;
  }
</style>
