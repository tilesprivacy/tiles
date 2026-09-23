<script lang="ts">
  import { act } from "./act";

  import AwakeMenu from "./AwakeMenu.svelte";
  import Chevron from "./Chevron.svelte";
  import CupMark from "./CupMark.svelte";
  import { awake } from "../state.svelte";

  interface Props {
    /** the daemon's version, or why there is no version to show */
    note: string;
    /** a version reads quietly, a failure does not */
    alert?: boolean;
  }

  let { note, alert = false }: Props = $props();

  let menu = $state(false);

  let now = $state(Date.now());

  $effect(() => {
    if (!awake.value.active || awake.value.paused) return;
    now = Date.now();
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  function clock(total: number): string {
    const pad = (value: number) => String(value).padStart(2, "0");
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);

    return hours > 0
      ? `${hours}:${pad(minutes)}:${pad(total % 60)}`
      : `${pad(minutes)}:${pad(total % 60)}`;
  }

  const session: "none" | "running" | "paused" = $derived(
    awake.value.paused ? "paused" : awake.value.active ? "running" : "none",
  );
  const powerAllowed = $derived(
    awake.value.power.pluggedIn ||
      (awake.value.power.batteryPercent !== null && awake.value.power.batteryPercent > 10),
  );
  const powerCopy = $derived.by(() => {
    const percent = awake.value.power.batteryPercent;
    if (!powerAllowed && percent !== null) {
      return `${percent}% battery · Connect a charger to enable`;
    }
    return "Available when plugged in or battery is above 10%";
  });

  const reading = $derived.by(() => {
    const { paused, since, until, frozen } = awake.value;
    if (paused) return frozen === null ? "" : clock(Math.round(frozen / 1000));
    if (until !== null) return clock(Math.ceil(Math.max(0, until - now) / 1000));
    if (since !== null) return clock(Math.floor(Math.max(0, now - since) / 1000));
    return "";
  });

  function pick(seconds: number | null) {
    menu = false;
    if (!powerAllowed) return;
    act("awake_start", { seconds });
  }

  function run(command: "awake_pause" | "awake_resume" | "awake_stop") {
    menu = false;
    act(command);
  }

  // capture, or the panel's handler pops the view out from under the menu
  function onkeydowncapture(event: KeyboardEvent) {
    if (!menu || event.key !== "Escape") return;
    event.preventDefault();
    event.stopPropagation();
    menu = false;
  }
</script>

<svelte:window {onkeydowncapture} />

<footer class="footer">
  <span class="footer__note" data-alert={alert}>{note}</span>

  <div class="footer__cupwrap">
    {#if menu}
      <button
        class="footer__scrim"
        tabindex="-1"
        aria-label="Close"
        onclick={() => (menu = false)}
      ></button>
      <AwakeMenu
        {session}
        available={powerAllowed}
        note={powerCopy}
        onpick={pick}
        onpause={() => run("awake_pause")}
        onresume={() => run("awake_resume")}
        onstop={() => run("awake_stop")}
      />
    {/if}

    <button
      class="footer__cup"
      data-state={session}
      aria-label="Keep this Mac awake"
      aria-haspopup="menu"
      aria-expanded={menu}
      title={powerAllowed ? "Keep this Mac awake" : powerCopy}
      onclick={() => (menu = !menu)}
    >
      <CupMark active={session === "running"} />
      {#if session !== "none"}
        <span class="footer__count">{reading}</span>
      {:else}
        <span>Keep awake</span>
      {/if}

      <Chevron dir="up" />
    </button>
  </div>

  <button class="footer__quit" onclick={() => act("quit_app")}>
    Quit
  </button>
</footer>

<style>
  .footer {
    --h-plate: 22px;

    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    height: var(--h-nav);
    padding: 0 var(--pad-x);
  }

  /* stops either side of the panel's ring rather than crossing it, so the
     joint is one hairline and not two stacked */
  .footer::before {
    content: "";
    position: absolute;
    top: 0;
    left: var(--hairline);
    right: var(--hairline);
    height: var(--hairline);
    background: var(--rule);
  }

  .footer__note {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: var(--fs-body);
    font-variant-numeric: tabular-nums;
    color: var(--slate);
  }

  .footer__note[data-alert="true"] {
    color: var(--alert);
  }

  .footer__count {
    min-width: 7ch;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    text-align: left;
  }

  .footer__cupwrap {
    position: relative;
    flex: none;
    display: flex;
  }

  .footer__scrim {
    position: fixed;
    inset: 0;
    z-index: 1;
    border: none;
    background: transparent;
  }

  .footer__cup,
  .footer__quit {
    flex: none;
    display: flex;
    align-items: center;
    height: var(--h-plate);
    padding: 0 9px;
    clip-path: var(--clip-cut);
    border: none;
    background: var(--steel);
    color: var(--ash);
    font-family: var(--font-ui);
    font-size: var(--fs-body);
    line-height: 1;
    transition:
      background var(--dur-state) ease-out,
      color var(--dur-state) ease-out;
  }

  .footer__cup {
    --row-mark: currentColor;

    gap: 6px;
    position: relative;
    z-index: 2;
  }

  .footer__cup[data-state="none"]:hover:not(:disabled) {
    color: var(--bone);
  }

  .footer__cup[data-state="running"] {
    background: var(--signal);
    color: var(--void);
  }

  .footer__cup[data-state="running"]:hover:not(:disabled) {
    background: rgba(247, 255, 97, 0.8);
  }

  .footer__cup[data-state="paused"] {
    color: var(--signal);
  }

  .footer__quit:hover {
    background: var(--signal);
    color: var(--void);
  }
</style>
