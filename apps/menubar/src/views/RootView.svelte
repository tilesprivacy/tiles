<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";

  import Avatar from "../lib/Avatar.svelte";
  import Chevron from "../lib/Chevron.svelte";
  import Chip from "../lib/Chip.svelte";
  import CopyMark from "../lib/CopyMark.svelte";
  import Footer from "../lib/Footer.svelte";
  import Masthead, { type Mode } from "../lib/Masthead.svelte";
  import ProviderMark from "../lib/ProviderMark.svelte";
  import Row from "../lib/Row.svelte";
  import Switch from "../lib/Switch.svelte";
  import SessionList from "../lib/SessionList.svelte";
  import Zone from "../lib/Zone.svelte";
  import { Copier } from "../lib/copy.svelte";
  import { contextLabel, describe } from "../lib/model";
  import { nav } from "../nav.svelte";
  import { account, health, inference, remote, sessions, truncateMiddle } from "../state.svelte";

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

<Zone label="Tiles Account">
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
