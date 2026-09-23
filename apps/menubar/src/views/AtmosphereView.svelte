<script lang="ts">
  import { onDestroy } from "svelte";

  import Avatar from "../lib/Avatar.svelte";
  import CopyMark from "../lib/CopyMark.svelte";
  import Navbar from "../lib/Navbar.svelte";
  import Note from "../lib/Note.svelte";
  import Row from "../lib/Row.svelte";
  import Zone from "../lib/Zone.svelte";
  import { Copier } from "../lib/copy.svelte";
  import { nav } from "../nav.svelte";
  import { atproto, held, truncateMiddle } from "../state.svelte";

  const handle = new Copier();
  const did = new Copier();
  onDestroy(() => {
    handle.dispose();
    did.dispose();
  });

  const kept = held(
    () => atproto.value,
    (at) => (at.state === "session" ? at : null),
  );

  $effect(() => {
    if (atproto.value.state === "none") nav.pop();
  });

  const session = $derived(kept.value);

  const name = $derived(session?.displayName?.trim() || null);

  const host = $derived(session?.pds?.replace(/^https:\/\//, "") ?? null);
</script>

<Navbar title="Atmosphere Account" onback={() => nav.pop()} />

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
  <Note>
    Your Atmosphere Account is connected and ready to share conversations through your AT Protocol
    PDS.
  </Note>
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
  <Row mono title={host ?? "—"} dimmed={host === null} />
</Zone>
