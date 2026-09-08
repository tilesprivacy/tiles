<script lang="ts">
  // vendored, never fetched, see src/assets/providers
  const files = import.meta.glob("../assets/providers/*.svg", {
    eager: true,
    query: "?raw",
    import: "default",
  }) as Record<string, string>;

  const marks: Record<string, string> = {};
  for (const [path, svg] of Object.entries(files)) {
    marks[path.split("/").at(-1)!.slice(0, -4)] = svg;
  }

  interface Props {
    provider: string;
    size?: number;
  }

  let { provider, size = 22 }: Props = $props();

  const glyph = $derived(marks[provider] ?? marks.generic);
</script>

<span class="provider" style="--size: {size}px" aria-hidden="true">{@html glyph}</span>

<style>
  .provider {
    flex: none;
    display: flex;
    align-items: center;
    justify-content: center;
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
    /* the provider is a fact about the model, not a reading of focus */
    color: var(--signal);
  }

  .provider :global(svg) {
    display: block;
    width: 62%;
    height: 62%;
  }
</style>
