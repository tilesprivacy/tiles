<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";

  import Avatar from "../lib/Avatar.svelte";
  import Chevron from "../lib/Chevron.svelte";
  import Chip from "../lib/Chip.svelte";
  import CopyMark from "../lib/CopyMark.svelte";
  import Footer from "../lib/Footer.svelte";
  import HandleField from "../lib/HandleField.svelte";
  import Masthead, { type Mode } from "../lib/Masthead.svelte";
  import ProviderMark from "../lib/ProviderMark.svelte";
  import Row from "../lib/Row.svelte";
  import Switch from "../lib/Switch.svelte";
  import SessionList from "../lib/SessionList.svelte";
  import Zone from "../lib/Zone.svelte";
  import { Copier } from "../lib/copy.svelte";
  import { contextLabel, describe } from "../lib/model";
  import { nav } from "../nav.svelte";
  import {
    account,
    atproto,
    health,
    inference,
    remote,
    sessions,
    truncateMiddle,
  } from "../state.svelte";

  /** how many fit under the masthead before the panel gets tall */
  const PREVIEW = 3;

  const copier = new Copier();
  onDestroy(() => copier.dispose());

  let sharePending = $state(false);
  /** what the share was asked for, worn until the daemon reports the same */
  let wanted = $state<boolean | null>(null);
  /** the call itself, which is the only window a second click is swallowed in */
  let inflight = $state(false);
  /** what the daemon was last asked for, held until the power state gets there */
  let request = $state<"on" | "off" | null>(null);
  // plain, not state: the effect below writes it, and depending on its own
  // write would make it run itself a second time
  let sawStarting = false;

  const power = $derived(inference.value.power);
  // starting is not on, or the switch flips before the light has run
  const on = $derived(power === "on");
  const busy = $derived(inflight || request !== null || power === "starting");

  const mode = $derived.by<Mode>(() => {
    if (health.value.state === "down") return "down";
    if (health.value.state === "starting") return "connecting";
    if (power === "starting") return "starting";
    return power === "on" ? "running" : "idle";
  });

  // the masthead is the state, so the footer only carries what it cannot say
  const note = $derived.by(() => {
    switch (health.value.state) {
      case "down":
        return health.value.reason;
      case "starting":
        return "Connecting";
      case "up":
        return health.value.version;
    }
  });

  const model = $derived(inference.value.model ? describe(inference.value.model) : null);
  const modelSub = $derived.by(() => {
    const parts: string[] = [];
    const context = inference.value.llama?.contextLength;
    if (context) parts.push(`${contextLabel(context)} context`);
    if (model?.format) parts.push(model.format);

    return parts.join(" · ");
  });

  const identity = $derived.by(() => {
    switch (account.value.state) {
      case "local":
        return {
          name: account.value.nickname,
          title: account.value.nickname,
          sub: truncateMiddle(account.value.did, 16, 6),
        };
      case "none":
        return { name: "?", title: "No account yet", sub: "Run tiles account create" };
      case "unknown":
        return { name: "?", title: "—", sub: "" };
    }
  });

  const atmosphere = $derived.by(() => {
    switch (atproto.value.state) {
      case "session": {
        // the name takes the title when there is one, and the handle carries
        // the did off the row when it does
        const name = atproto.value.displayName?.trim() || null;
        return {
          name: name ?? atproto.value.handle,
          title: name ?? `@${atproto.value.handle}`,
          sub: name ? `@${atproto.value.handle}` : truncateMiddle(atproto.value.did, 16, 6),
          avatar: atproto.value.avatar ?? null,
        };
      }
      case "pending":
        return {
          name: atproto.value.handle,
          title: `@${atproto.value.handle}`,
          sub: "Waiting for your browser",
          avatar: null,
        };
      default:
        return { name: "?", title: "Not connected", sub: "", avatar: null };
    }
  });

  const signedOut = $derived(atproto.value.state === "none");
  const signingIn = $derived(atproto.value.state === "pending");
  const signedIn = $derived(atproto.value.state === "session");

  // signed out the row opens the drawer, signed in it pushes, and mid-login it
  // is neither
  const enterAtmosphere = $derived(
    signedOut ? askForHandle : signedIn ? () => nav.push("atmosphere") : undefined,
  );

  let drawer = $state(false);
  let signInError = $state<string | null>(null);

  // the browser takes key off the panel, so the drawer would come back open
  // over an account that is already signed in
  $effect(() => {
    if (atproto.value.state === "session") drawer = false;
  });

  function askForHandle() {
    drawer = !drawer;
    if (!drawer) signInError = null;
  }

  // the daemon holds this open for the whole browser round trip, so the await
  // outlives the panel being on screen
  async function signIn(handle: string) {
    signInError = null;
    try {
      await invoke("atproto_login", { handle });
      drawer = false;
    } catch (err) {
      signInError = String(err);
    }
  }

  const recent = $derived(sessions.value.state === "ready" ? sessions.value.sessions : []);
  // pushing would show exactly what is already on screen
  const hasMore = $derived(recent.length > PREVIEW);

  // the proxy forwards straight to the local server, so a ticket handed out
  // with inference down answers nothing
  const canShare = $derived(power === "on");
  const sharing = $derived(remote.value.state === "sharing" ? remote.value : null);

  // plain, not state: the effect below writes it, and depending on its own
  // write would make it run itself a second time
  let unshared = false;

  // the request stands until the power state gets where it was going, or settles
  // somewhere else because the load failed
  $effect(() => {
    if (request === null) return;
    if (health.value.state !== "up") {
      request = null;
      return;
    }
    if (power === "starting") {
      sawStarting = true;
      return;
    }
    if (power === request || (sawStarting && (power === "on" || power === "off"))) {
      request = null;
    }
  });

  async function toggle() {
    if (inflight || health.value.state !== "up") return;
    // a click while the daemon is still working reverses the request rather
    // than being swallowed, so the switch is never stuck waiting
    const next = (request ?? (on ? "on" : "off")) === "on" ? "off" : "on";
    request = next;
    sawStarting = false;
    inflight = true;
    try {
      await invoke("inference_set", { on: next === "on" });
    } catch (err) {
      console.error("[inference]", err);
      request = null;
    } finally {
      inflight = false;
    }
  }

  // inference going down takes the share with it, rather than leaving a ticket
  // pointing at an engine that is not there. only on a confirmed off, never on
  // the unknown the panel opens with
  $effect(() => {
    if (power === "on") {
      unshared = false;
      return;
    }
    if (power !== "off" || sharing === null || sharePending || unshared) return;
    unshared = true;
    void share();
  });

  $effect(() => {
    if (wanted === null || remote.value.state === "unknown") return;
    if ((sharing !== null) === wanted) wanted = null;
  });

  async function share() {
    if (sharePending || (!canShare && sharing === null)) return;
    // the switch flips on the click and the ticket line comes with it, rather
    // than the row sitting still while the daemon mints one
    const next = sharing === null;
    wanted = next;
    sharePending = true;
    try {
      await invoke("remote_set", { on: next });
    } catch (err) {
      console.error("[remote]", err);
      wanted = null;
    } finally {
      sharePending = false;
    }
  }
</script>

<Masthead {mode} {on} pending={busy} disabled={health.value.state !== "up"} ontoggle={toggle} />

<Zone label="Accounts">
  <h3 class="account">Tiles Account</h3>
  <Row
    size="large"
    title={identity.title}
    sub={identity.sub}
    submono={account.value.state === "local"}
    dimmed={account.value.state !== "local"}
    onselect={account.value.state === "local" ? () => nav.push("account") : undefined}
  >
    {#snippet leading()}
      <Avatar nickname={identity.name} />
    {/snippet}
    {#snippet trailing()}
      {#if account.value.state === "local"}<Chevron />{/if}
    {/snippet}
  </Row>

  <h3 class="account">Atmosphere Account</h3>
  <Row
    size="large"
    title={atmosphere.title}
    sub={atmosphere.sub}
    submono={atproto.value.state === "session"}
    dimmed={atproto.value.state === "unknown"}
    onselect={enterAtmosphere}
  >
    {#snippet leading()}
      <Avatar nickname={atmosphere.name} src={atmosphere.avatar} />
    {/snippet}
    {#snippet trailing()}
      {#if signedOut}
        <span class="signin" data-open={drawer}>Sign in</span>
      {:else if signedIn}
        <Chevron />
      {/if}
    {/snippet}
  </Row>

  <!-- the panel's height follows its content, so opening this grows the window
       with it, one resize per frame the way a push does -->
  <div class="drawer" data-open={drawer || signingIn}>
    <div class="drawer__clip" inert={!drawer && !signingIn}>
      <div class="drawer__body">
        <HandleField
          open={drawer && !signingIn}
          pending={signingIn}
          onsubmit={signIn}
          oncancel={() => (drawer = false)}
        />
        {#if signInError}<p class="drawer__error">{signInError}</p>{/if}
      </div>
    </div>
  </div>
</Zone>

<Zone label="Model" dimmed={!canShare}>
  {#if model}
    <Row
      size="large"
      title={model.name}
      sub={modelSub}
      submono
      dimmed={!canShare}
      onselect={canShare ? () => nav.push("model") : undefined}
    >
      {#snippet leading()}
        <ProviderMark provider={model.provider} />
      {/snippet}
      {#snippet trailing()}
        {#if model.quant}<Chip text={model.quant} />{/if}
        <Chevron />
      {/snippet}
    </Row>
  {:else}
    <Row size="large" title="No model configured" sub="Run tiles model use" dimmed>
      {#snippet leading()}
        <ProviderMark provider="generic" />
      {/snippet}
    </Row>
  {/if}
</Zone>

<Zone label="Chats">
  {#if recent.length > 0}
    <SessionList sessions={recent.slice(0, PREVIEW)} />
    {#if hasMore}
      <Row title="All chats" tone="signal" onselect={() => nav.push("sessions")}>
        {#snippet trailing()}
          <Chip text={String(recent.length)} />
          <Chevron />
        {/snippet}
      </Row>
    {/if}
  {:else}
    <Row title={sessions.value.state === "ready" ? "No chats yet" : "—"} dimmed />
  {/if}
</Zone>

<Zone label="Share your compute" dimmed={!canShare}>
  <Row title="Remote inference" dimmed={!canShare || remote.value.state === "unknown"}>
    {#snippet trailing()}
      <Switch
        on={canShare && (wanted ?? sharing !== null)}
        disabled={!canShare}
        size="small"
        glow={false}
        label="Remote inference"
        onchange={share}
      />
    {/snippet}
  </Row>
  {#if canShare && sharing}
    <Row
      key="Ticket"
      mono
      title={truncateMiddle(sharing.ticket, 20, 8)}
      onselect={() => copier.copy(sharing.ticket)}
    >
      {#snippet inline()}
        <CopyMark copied={copier.copied} />
      {/snippet}
    </Row>
  {:else if canShare && wanted === true}
    <Row key="Ticket">
      {#snippet inline()}
        <span class="ticket-skeleton" aria-label="Minting a ticket"></span>
      {/snippet}
    </Row>
  {/if}
</Zone>

<Footer {note} alert={health.value.state === "down"} />

<style>
  /* the zone's rule runs full bleed and carries a zone label, so a rule that
     is inset and interrupted by its own name is the level under it */
  .account {
    display: flex;
    align-items: center;
    gap: 7px;
    margin-bottom: 5px;
    padding: 0 var(--pad-x);
    color: var(--slate);
    font-size: var(--fs-chip);
    font-weight: 500;
    letter-spacing: var(--tracking-chip);
    opacity: 0.8;
  }

  .account::after {
    content: "";
    flex: 1;
    height: var(--hairline);
    background: var(--rule);
  }

  .account:not(:first-of-type) {
    margin-top: 9px;
  }

  /* the zone label carries 5 of the 9 this wants, and a label under a label
     needs the same step everything else separates on */
  .account:first-of-type {
    margin-top: 4px;
  }

  /* the row is the button, so this is a span. two buttons in one row is one
     too many, and the rail already says which row is live */
  .signin {
    flex: none;
    clip-path: polygon(0 0, 100% 0, 100% calc(100% - 3px), calc(100% - 3px) 100%, 0 100%);
    padding: 3px 7px;
    background: var(--steel);
    color: var(--row-mark, var(--ash));
    font-size: var(--fs-label);
    font-weight: 500;
    transition:
      background var(--dur-state) ease-out,
      color var(--dur-state) ease-out;
  }

  .signin[data-open="true"] {
    background: var(--signal);
    color: var(--void);
  }

  .drawer {
    display: grid;
    grid-template-rows: 0fr;
    transition: grid-template-rows var(--dur-push) var(--ease-push);
  }

  .drawer[data-open="true"] {
    grid-template-rows: 1fr;
  }

  /* the padding rides inside the clip, so a shut drawer is exactly zero */
  .drawer__clip {
    min-height: 0;
    overflow: hidden;
  }

  .drawer__body {
    padding: 3px var(--pad-x) 5px;
  }

  .drawer__error {
    padding-top: 5px;
    color: var(--alert);
    font-size: var(--fs-label);
  }

  /* the ticket's own line, at the width the truncated one lands on */
  .ticket-skeleton {
    width: 186px;
    height: 11px;
    background: var(--slate);
    animation: ticket-pulse 1.4s ease-in-out infinite;
  }

  @keyframes ticket-pulse {
    0%,
    100% {
      opacity: 0.35;
    }
    50% {
      opacity: 0.75;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .ticket-skeleton {
      opacity: 0.5;
      animation: none;
    }
  }
</style>
