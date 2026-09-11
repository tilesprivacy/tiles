<script lang="ts">
  import { act } from "./act";

  import AwakeMenu from "./AwakeMenu.svelte";
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

  // the state event fires on change only, so the count is this side's to keep
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

    // h:mm:ss past the hour, or 4 hours and 4 minutes both read as 4:00
    return hours > 0
      ? `${hours}:${pad(minutes)}:${pad(total % 60)}`
      : `${pad(minutes)}:${pad(total % 60)}`;
  }

  const session = $derived.by<"none" | "running" | "paused">(() =>
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

  // a session with an end counts down to it, one without counts up from where
  // it started, and a held one reads whatever it was banked at
  const reading = $derived.by(() => {
    const { paused, since, until, frozen } = awake.value;
    if (paused) return frozen === null ? "" : clock(Math.round(frozen / 1000));
    // ceil, so a fresh 15 minute session reads 15:00 rather than 14:59
    if (until !== null) return clock(Math.ceil(Math.max(0, until - now) / 1000));
    // floor, so a stopwatch starts at 00:00 rather than 00:01
    if (since !== null) return clock(Math.floor(Math.max(0, now - since) / 1000));
    return "";
  });

  function pick(seconds: number | null) {
    menu = false;
    if (!powerAllowed) return;
    act("awake_start", { seconds });
  }

  function run(command: string) {
    menu = false;
    act(command);
  }

  // capture, or the panel's own handler pops the view out from under the menu
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
      <!-- the click that dismisses should not also land on whatever is under
           it, so it is caught here rather than on the window -->
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
      <!-- steam is the run, so a held session has none -->
      <CupMark active={session === "running"} />
      <!-- a session says where its clock is, and only a plate with none has to
           explain what it is for -->
      {#if session !== "none"}
        <span class="footer__count">{reading}</span>
      {:else}
        <span>Keep awake</span>
      {/if}

      <!-- every click opens the menu, in every state, so the plate says so -->
      <svg class="footer__trail" viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
        <path
          d="M2 6.5 5 3.5 8 6.5"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="square"
        />
      </svg>
    </button>
  </div>

  <button class="footer__quit" onclick={() => act("quit_app")}>
    Quit
  </button>
</footer>

<style>
  .footer {
    /* both plates sit on this, and the icon and the word centre inside it */
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

  /* full strength, a solid this small goes muddy the moment it is faded and
     the plate's own hover is already carrying the state */
  .footer__trail {
    flex: none;
    display: block;
  }

  /* a reading and not a word, so it is set like the version is. tabular, or
     the plate breathes under it every second */
  .footer__count {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  /* the menu hangs off this rather than the footer, so it lines up on the cup */
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

  /* a set height rather than padding, so a plate holding an icon and a plate
     holding a word still come out the same size */
  .footer__cup,
  .footer__quit {
    flex: none;
    display: flex;
    align-items: center;
    height: var(--h-plate);
    padding: 0 9px;
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--cut)),
      calc(100% - var(--cut)) 100%,
      0 100%
    );
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
    gap: 6px;
    /* over the scrim, or the plate goes dead the moment the menu opens */
    position: relative;
    z-index: 2;
  }

  .footer__cup[data-state="none"]:hover:not(:disabled) {
    color: var(--bone);
  }

  /* running, the same flood the sign in chip takes when its drawer is open */
  .footer__cup[data-state="running"] {
    background: var(--signal);
    color: var(--void);
  }

  /* yellow has nowhere brighter to go, so the hover eases the plate back. the
     mark stays the void, bone on yellow is the contrast bug */
  .footer__cup[data-state="running"]:hover:not(:disabled) {
    background: rgba(247, 255, 97, 0.8);
  }

  /* held, so the accent sits on the reading rather than under it. the clock
     still accounts for a session that exists, it is just not running */
  .footer__cup[data-state="paused"] {
    color: var(--signal);
  }

  .footer__quit:hover {
    background: var(--signal);
    color: var(--void);
  }
</style>
