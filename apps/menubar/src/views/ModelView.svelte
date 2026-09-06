<script lang="ts">
  import Chip from "../lib/Chip.svelte";
  import Navbar from "../lib/Navbar.svelte";
  import ProviderMark from "../lib/ProviderMark.svelte";
  import Row from "../lib/Row.svelte";
  import Zone from "../lib/Zone.svelte";
  import { describe } from "../lib/model";
  import { nav } from "../nav.svelte";
  import { inference } from "../state.svelte";

  const spec = $derived(inference.value.model);
  const model = $derived(spec ? describe(spec) : null);

  type Flag = { key: string; value: string };

  const flags = $derived.by<Flag[]>(() => {
    const llama = inference.value.llama;
    if (!llama) return [];

    const onOff = (value: boolean) => (value ? "on" : "off");
    const rows: Flag[] = [];

    const { contextLength, batchSize, gpuLayers, nCpuMoe } = llama;
    if (contextLength !== null) rows.push({ key: "Context window", value: `${contextLength}` });
    if (batchSize !== null) rows.push({ key: "Batch size", value: `${batchSize}` });
    if (gpuLayers !== null) rows.push({ key: "GPU layers", value: `${gpuLayers}` });
    if (nCpuMoe !== null) rows.push({ key: "Expert layers on CPU", value: `${nCpuMoe}` });
    if (llama.flashAttn !== null) rows.push({ key: "Flash attention", value: onOff(llama.flashAttn) });
    if (llama.offloadKqv !== null) rows.push({ key: "KQV offload", value: onOff(llama.offloadKqv) });
    if (llama.mtp !== null) rows.push({ key: "Multi-token prediction", value: onOff(llama.mtp) });
    // the daemon stores the negative, the panel shows the thing itself
    if (llama.noMmap !== null) rows.push({ key: "Memory mapping", value: onOff(!llama.noMmap) });

    return rows;
  });
</script>

<Navbar title="Model" onback={() => nav.pop()} />

<Zone label="Current">
  <Row size="large" title={model?.name ?? "—"} sub={spec ?? ""} submono dimmed={model === null}>
    {#snippet leading()}
      <ProviderMark provider={model?.provider ?? "generic"} size={26} />
    {/snippet}
    {#snippet trailing()}
      {#if model?.quant}<Chip text={model.quant} />{/if}
      {#if model?.format}<Chip text={model.format} />{/if}
    {/snippet}
  </Row>
</Zone>

<!-- an installed-models zone slots in here the day a list route exists -->

{#if flags.length > 0}
  <Zone label="Runtime · llama.cpp">
    {#each flags as flag (flag.key)}
      <Row title={flag.key}>
        {#snippet trailing()}
          <span class="value">{flag.value}</span>
        {/snippet}
      </Row>
    {/each}
  </Zone>
{/if}

<style>
  .value {
    flex: none;
    font-family: var(--font-mono);
    font-size: var(--fs-mono);
    font-variant-numeric: tabular-nums;
    color: var(--ash);
  }
</style>
