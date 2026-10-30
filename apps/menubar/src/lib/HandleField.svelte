<script lang="ts">
  interface Props {
    open: boolean;
    pending: boolean;
    onsubmit: (handle: string) => void;
    oncancel: () => void;
  }

  let { open, pending, onsubmit, oncancel }: Props = $props();

  let value = $state("");
  let input = $state<HTMLInputElement | null>(null);

  const handle = $derived(value.trim().replace(/^@+/, ""));
  const ready = $derived(handle.length > 0);

  $effect(() => {
    if (open) {
      input?.focus();
    } else {
      input?.blur();
      if (!pending) value = "";
    }
  });

  function submit() {
    if (!ready || pending) return;
    onsubmit(handle);
  }

  function key(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.stopPropagation();
      event.preventDefault();
      oncancel();
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      submit();
    }
  }
</script>

<div class="field" data-ready={ready} data-pending={pending}>
  <span class="field__cell">
    <span class="field__at">@</span>
    <input
      bind:this={input}
      bind:value
      class="field__input"
      type="text"
      placeholder="name.bsky.social"
      spellcheck="false"
      autocapitalize="off"
      autocomplete="off"
      autocorrect="off"
      disabled={pending}
      aria-label="Atmosphere account handle"
      onkeydown={key}
    />
  </span>
  <button class="field__go" disabled={!ready || pending} aria-label="Sign in" onclick={submit}>
    <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
      <path
        d="M10 2.5v4h-7M5 4.5l-2 2 2 2"
        fill="none"
        stroke="currentColor"
        stroke-width="1.2"
        stroke-linecap="square"
      />
    </svg>
  </button>
</div>

<style>
  .field {
    --frame: var(--frame-rest);

    position: relative;
    display: flex;
    gap: 1px;
    height: 26px;
    padding: 1px;
    background: var(--frame);
    clip-path: var(--clip-cut);
    transition: background var(--dur-state) ease-out;
  }

  .field:focus-within,
  .field[data-pending="true"] {
    --frame: var(--frame-live);
  }

  .field__cell {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 5px;
    min-width: 0;
    padding: 0 8px;
    background: var(--steel);
  }

  .field__at {
    flex: none;
    color: var(--signal);
    font-family: var(--font-mono);
    font-size: var(--fs-mono);
  }

  .field__input {
    flex: 1;
    min-width: 0;
    border: none;
    background: none;
    color: var(--bone);
    caret-color: var(--signal);
    font-family: var(--font-mono);
    font-size: var(--fs-mono);
  }

  .field__input::placeholder {
    color: var(--slate);
    opacity: 0.6;
  }

  .field__go {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    border: none;
    padding: 0;
    background: var(--steel);
    color: var(--cell-fg);
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--cut) + 1px),
      calc(100% - var(--cut) + 1px) 100%,
      0 100%
    );
    font-family: var(--font-mono);
    font-size: var(--fs-mono);
    line-height: 1;
    transition:
      background var(--dur-state) ease-out,
      color var(--dur-state) ease-out;
  }

  .field[data-ready="true"] .field__go:not(:disabled) {
    background: var(--signal);
    color: var(--void);
  }

  .field__go:not(:disabled):hover {
    color: var(--bone);
  }

  .field[data-ready="true"] .field__go:not(:disabled):hover {
    background: var(--bone);
    color: var(--void);
  }

  .field[data-pending="true"]::after {
    content: "";
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: var(--hairline);
    background: linear-gradient(90deg, transparent, var(--signal), transparent);
    animation: sweep 1.4s linear infinite;
  }

  @keyframes sweep {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(100%);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .field[data-pending="true"]::after {
      background: var(--signal);
      opacity: 0.5;
      animation: none;
    }
  }
</style>
