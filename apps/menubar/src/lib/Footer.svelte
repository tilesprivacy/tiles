<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface Props {
    /** the daemon's version, or why there is no version to show */
    note: string;
    /** a version reads quietly, a failure does not */
    alert?: boolean;
  }

  let { note, alert = false }: Props = $props();
</script>

<footer class="footer">
  <span class="footer__note" data-alert={alert}>{note}</span>
  <button class="footer__quit" onclick={() => void invoke("quit_app").catch(() => {})}>
    Quit
  </button>
</footer>

<style>
  .footer {
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
    font-size: var(--fs-mono);
    font-variant-numeric: tabular-nums;
    color: var(--slate);
  }

  .footer__note[data-alert="true"] {
    color: var(--alert);
  }

  .footer__quit {
    flex: none;
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--cut)),
      calc(100% - var(--cut)) 100%,
      0 100%
    );
    padding: 4px 9px;
    border: none;
    background: var(--steel);
    color: var(--ash);
    font-family: var(--font-ui);
    font-size: var(--fs-label);
    line-height: 1;
    transition:
      background var(--dur-state) ease-out,
      color var(--dur-state) ease-out;
  }

  .footer__quit:hover {
    background: var(--signal);
    color: var(--void);
  }
</style>
