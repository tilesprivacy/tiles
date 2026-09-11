<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy, onMount } from "svelte";

  import { act } from "../lib/act";

  import Avatar from "../lib/Avatar.svelte";
  import CopyMark from "../lib/CopyMark.svelte";
  import Navbar from "../lib/Navbar.svelte";
  import OpenMark from "../lib/OpenMark.svelte";
  import Row from "../lib/Row.svelte";
  import Zone from "../lib/Zone.svelte";
  import { Copier } from "../lib/copy.svelte";
  import { nav } from "../nav.svelte";
  import { account, truncateMiddle } from "../state.svelte";

  const copier = new Copier();
  onDestroy(() => copier.dispose());

  // the daemon misses a tick and reports unknown, and an account already on
  // screen should not blink out with it. the identity itself does not change
  let held = $state(account.value.state === "local" ? account.value : null);

  $effect(() => {
    const at = account.value;
    if (at.state === "local") held = at;
    // no account is not a blip, and this view is of one that is gone
    else if (at.state === "none") nav.pop();
  });

  // const, so the row below can narrow it inside its own handler
  const local = $derived(held);

  // read once on open rather than polled, the daemon rewrites it only on a
  // `tiles data path` and this view is not on screen for long
  let dataDir = $state<string | null>(null);
  onMount(() => {
    void invoke<string>("data_dir")
      .then((dir) => (dataDir = dir))
      .catch(() => {});
  });

  const home = $derived(dataDir?.replace(/^\/Users\/[^/]+/, "~") ?? "—");
</script>

<Navbar title="Tiles Account" onback={() => nav.pop()} />

<!-- no label, an avatar beside a name does not need one told -->
<Zone>
  <Row size="large" title={local?.nickname ?? "—"} dimmed={local === null}>
    {#snippet leading()}
      <Avatar nickname={local?.nickname ?? "?"} size={26} />
    {/snippet}
  </Row>
  <!-- the sub said what the name was, this says what the account is -->
  <p class="note">
    Your Tiles Account is generated and secured on this device. It is ready for peer-to-peer sync,
    remote inference, and other local-first features, using DIDs and UCANs for zero-trust
    authentication and authorization.
  </p>
</Zone>

<Zone label="Decentralized ID">
  <Row
    mono
    title={local ? truncateMiddle(local.did, 30, 10) : "—"}
    dimmed={local === null}
    onselect={local ? () => copier.copy(local.did) : undefined}
  >
    {#snippet inline()}
      <CopyMark copied={copier.copied} />
    {/snippet}
  </Row>
</Zone>

<Zone label="Data folder">
  <Row
    mono
    title={home}
    dimmed={dataDir === null}
    onselect={dataDir ? () => act("reveal_path", { path: dataDir }) : undefined}
  >
    {#snippet inline()}
      {#if dataDir}<OpenMark />{/if}
    {/snippet}
  </Row>
</Zone>

<style>
  /* the grey the row's sub used to carry, on its own line under the whole row */
  .note {
    padding: 2px var(--pad-x) 2px;
    color: var(--slate);
    font-size: var(--fs-body);
    line-height: 1.35;
  }
</style>
