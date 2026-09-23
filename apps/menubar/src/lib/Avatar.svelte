<script lang="ts">
  interface Props {
    nickname: string;
    size?: number;
    src?: string | null;
  }

  let { nickname, size = 22, src = null }: Props = $props();

  const initials = $derived(
    nickname
      .split(/[.\-\s_]+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0].toUpperCase())
      .join("") || "?",
  );

  // keyed to src: an effect clearing these runs after the flush
  let loaded = $state<string | null>(null);
  let broken = $state<string | null>(null);

  const showing = $derived(!!src && broken !== src);
  const ready = $derived(!!src && loaded === src);
</script>

<div
  class="avatar"
  data-framed={showing}
  style="--size: {size}px; --fs: {Math.round(size * 0.4)}px"
>
  <span class="avatar__cell">
    {initials}
    {#if showing}
      <img
        class="avatar__img"
        {src}
        alt=""
        data-ready={ready}
        onload={() => (loaded = src)}
        onerror={() => (broken = src)}
      />
    {/if}
  </span>
</div>

<style>
  .avatar {
    --frame-w: 0px;

    flex: none;
    width: var(--size);
    height: var(--size);
    padding: var(--frame-w);
    clip-path: var(--clip-cut);
    transition: background var(--dur-state) ease-out;
  }

  .avatar[data-framed="true"] {
    --frame-w: 2px;

    background: var(--mark-frame, rgba(255, 255, 255, 0.22));
  }

  .avatar__cell {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--cut) + var(--frame-w)),
      calc(100% - var(--cut) + var(--frame-w)) 100%,
      0 100%
    );
    background: var(--steel);
    color: var(--row-mark, var(--ash));
    font-family: var(--font-mono);
    font-size: var(--fs);
    font-weight: 500;
    letter-spacing: 0.02em;
    transition: color var(--dur-state) ease-out;
  }

  .avatar__img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity var(--dur-state) ease-out;
  }

  .avatar__img[data-ready="true"] {
    opacity: 1;
  }
</style>
