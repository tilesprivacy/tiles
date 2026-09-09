<script lang="ts">
  interface Props {
    nickname: string;
    size?: number;
    /** a data uri read off the pds, initials when it is absent or will not decode */
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

  let failed = $state(false);
  let ready = $state(false);

  // a new picture gets its own chance to load, and its own chance to fail
  $effect(() => {
    src;
    failed = false;
    ready = false;
  });

  const showing = $derived(!!src && !failed);
</script>

<!-- the switch's construction, a frame with the mark seated a pixel inside, so
     a picture reads as machined into the row rather than laid over it -->
<div
  class="avatar"
  data-framed={showing}
  style="--size: {size}px; --fs: {Math.round(size * 0.4)}px"
>
  <span class="avatar__cell">
    <!-- initials sit under the picture, so the cell is never empty mid-load -->
    {initials}
    {#if showing}
      <img
        class="avatar__img"
        {src}
        alt=""
        data-ready={ready}
        onload={() => (ready = true)}
        onerror={() => (failed = true)}
      />
    {/if}
  </span>
</div>

<style>
  .avatar {
    /* no frame by default, initials are already type on steel */
    --frame-w: 0px;

    flex: none;
    width: var(--size);
    height: var(--size);
    padding: var(--frame-w);
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--cut)),
      calc(100% - var(--cut)) 100%,
      0 100%
    );
    transition: background var(--dur-state) ease-out;
  }

  /* a photograph needs seating, and the row hands down what colour to seat it in */
  .avatar[data-framed="true"] {
    --frame-w: 2px;

    background: var(--mark-frame, rgba(255, 255, 255, 0.22));
  }

  /* the cut is shortened by the inset so it stays parallel to the frame's */
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
