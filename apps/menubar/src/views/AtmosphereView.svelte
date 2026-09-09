<script lang="ts">
  import { onDestroy } from "svelte";

  import Avatar from "../lib/Avatar.svelte";
  import CopyMark from "../lib/CopyMark.svelte";
  import Navbar from "../lib/Navbar.svelte";
  import Row from "../lib/Row.svelte";
  import Zone from "../lib/Zone.svelte";
  import { Copier } from "../lib/copy.svelte";
  import { nav } from "../nav.svelte";
  import { atproto, truncateMiddle } from "../state.svelte";

  // one per row, so copying the did does not light the handle's mark
  const handle = new Copier();
  const did = new Copier();
  onDestroy(() => {
    handle.dispose();
    did.dispose();
  });

  // the daemon misses a tick and reports unknown, and an account already on
  // screen should not blink out with it. the identity itself does not change
  let held = $state(atproto.value.state === "session" ? atproto.value : null);

  $effect(() => {
    const at = atproto.value;
    if (at.state === "session") held = at;
    // signed out is not a blip, and this view is of an account that is gone
    else if (at.state === "none") nav.pop();
  });

  // const, so the rows below can narrow it inside their own handlers
  const session = $derived(held);

  const name = $derived(session?.displayName?.trim() || null);

  // the scheme is the same for every one of them, the host is the part that says
  // where the account actually lives
  const host = $derived(session?.pds?.replace(/^https:\/\//, "") ?? null);
</script>

<Navbar title="Atmosphere Account" onback={() => nav.pop()} />

<!-- no label, an avatar beside a name does not need one told -->
<Zone>
  <Row
    size="large"
    title={name ?? (session ? `@${session.handle}` : "—")}
    dimmed={session === null}
  >
    {#snippet leading()}
      <Avatar nickname={name ?? session?.handle ?? "?"} src={session?.avatar} size={26} />
    {/snippet}
  </Row>
  <!-- the sub said what the name was, this says where the account is -->
  <p class="note">This account lives on the server below, and Tiles reads it there directly</p>
</Zone>

<Zone label="Handle">
  <Row
    mono
    title={session ? `@${session.handle}` : "—"}
    dimmed={session === null}
    onselect={session ? () => handle.copy(session.handle) : undefined}
  >
    {#snippet inline()}
      <CopyMark copied={handle.copied} />
    {/snippet}
  </Row>
</Zone>

<Zone label="Decentralized ID">
  <Row
    mono
    title={session ? truncateMiddle(session.did, 30, 10) : "—"}
    dimmed={session === null}
    onselect={session ? () => did.copy(session.did) : undefined}
  >
    {#snippet inline()}
      <CopyMark copied={did.copied} />
    {/snippet}
  </Row>
</Zone>

<Zone label="Personal data server">
  <!-- read with the profile, so it lands a beat after the identity does -->
  <Row mono title={host ?? "—"} dimmed={host === null} />
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
