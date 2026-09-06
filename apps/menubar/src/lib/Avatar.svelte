<script lang="ts">
  interface Props {
    nickname: string;
    size?: number;
  }

  let { nickname, size = 22 }: Props = $props();

  const initials = $derived(
    nickname
      .split(/[.\-\s_]+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0].toUpperCase())
      .join("") || "?",
  );
</script>

<!-- square and cut, so it sits in the same family as the provider marks -->
<div class="avatar" style="--size: {size}px; --fs: {Math.round(size * 0.4)}px">
  {initials}
</div>

<style>
  .avatar {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: var(--size);
    height: var(--size);
    clip-path: polygon(
      0 0,
      100% 0,
      100% calc(100% - var(--cut)),
      calc(100% - var(--cut)) 100%,
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
</style>
