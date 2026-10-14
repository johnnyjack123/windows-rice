<script lang="ts">
  import type { ProcessInfo } from '$backend/contract';
  import { errorCode } from '$backend/transport';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import AppIcon from '$ui/AppIcon.svelte';
  import Banner from '$ui/Banner.svelte';
  import Icon from '$ui/Icon.svelte';
  import Segmented from '$ui/Segmented.svelte';
  import Spark from '$ui/Spark.svelte';
  import { useIsland } from '../../context.svelte';

  const { backend, controller, panel } = useIsland();
  const sys = backend.monitor;
  const s = $derived(i18n.m.system);
  const latest = $derived(sys.latest);
  let sort = $state<'cpu' | 'memory'>('cpu');
  let leaving = $state<number[]>([]);
  let denied = $state<string | null>(null);

  // Samples only flow while this section is visible.
  $effect(() => {
    if (controller.expanded && panel.section === 'system') {
      sys.subscribe();
      return () => sys.unsubscribe();
    }
  });

  const processes = $derived(
    [...sys.processes].sort((a, b) => (sort === 'cpu' ? b.cpu - a.cpu : b.memoryBytes - a.memoryBytes)),
  );
  const ramFraction = $derived(latest ? latest.memory.usedBytes / latest.memory.totalBytes : 0);

  async function kill(p: ProcessInfo): Promise<void> {
    denied = null;
    leaving = [...leaving, p.pid];
    try {
      await sys.kill(p.pid);
    } catch (error) {
      if (errorCode(error) === 'access_denied') {
        denied = p.name;
      }
    } finally {
      leaving = leaving.filter((pid) => pid !== p.pid);
    }
  }
</script>

<div class="meters">
  <div class="meter">
    <div class="meter__head">
      <span class="meter__label">{s.cpu}</span>
      <span class="meter__value num">{Math.round(latest?.cpu.usage ?? 0)}<small>%</small></span>
    </div>
    <Spark values={sys.cpu} max={100} />
    <div class="meter__sub">{latest ? `${latest.cpu.name}${latest.cpu.ghz ? ` · ${fmt.num(latest.cpu.ghz)} GHz` : ''}` : ''}</div>
  </div>
  <div class="meter">
    <div class="meter__head">
      <span class="meter__label">{s.ram}</span>
      <span class="meter__value num">{fmt.num((latest?.memory.usedBytes ?? 0) / 1024 ** 3)}<small>GB</small></span>
    </div>
    <div class="meter__bar"><div class="meter__fill" style:--p={ramFraction}></div></div>
    <div class="meter__sub">{latest ? s.ofTotal(fmt.bytes(latest.memory.totalBytes), Math.round(ramFraction * 100)) : ''}</div>
  </div>
  <div class="meter">
    <div class="meter__head">
      <span class="meter__label">{s.gpu}</span>
      <span class="meter__value num">{latest?.gpu ? Math.round(latest.gpu.usage) : '–'}<small>%</small></span>
    </div>
    <Spark values={sys.gpu} max={100} />
    <div class="meter__sub">
      {latest?.gpu ? `${latest.gpu.name} · ${fmt.num(latest.gpu.vramUsedBytes / 1024 ** 3)} / ${fmt.num(latest.gpu.vramTotalBytes / 1024 ** 3, 0)} GB` : s.unavailable}
    </div>
  </div>
  <div class="meter">
    <div class="meter__head">
      <span class="meter__label">{s.network}</span>
      <span class="meter__value num">{fmt.num((latest?.network.downBytesPerSec ?? 0) / 1024 ** 2)}<small>MB/s</small></span>
    </div>
    <Spark values={sys.down} max={Math.max(3 * 1024 ** 2, ...sys.down)} />
    <div class="meter__sub">↓ {fmt.rate(latest?.network.downBytesPerSec ?? 0)} · ↑ {fmt.rate(latest?.network.upBytesPerSec ?? 0)}</div>
  </div>
</div>

{#if denied}
  <Banner tone="error" icon="error" title={s.kill(denied)} body={s.killDenied} />
{/if}

<div class="section-label">
  {s.processes}
  <span style:margin-left="auto">
    <Segmented
      label={s.sortBy}
      value={sort}
      options={[{ value: 'cpu', label: s.cpu }, { value: 'memory', label: s.ram }]}
      onchange={(v) => (sort = v)}
    />
  </span>
</div>
<div class="pane__scroll scroll">
  <ul>
    {#each processes as p (p.pid)}
      <li class="row proc has-state" class:is-leaving={leaving.includes(p.pid)}>
        <AppIcon name={p.name} src={p.iconUrl} />
        <span class="row__title">{p.name}</span>
        <span class="proc__vals num"><span>{fmt.num(p.cpu)} %</span><span>{fmt.bytes(p.memoryBytes)}</span></span>
        <button class="icon-btn has-state proc__kill" aria-label={s.kill(p.name)} title={s.kill(p.name)} onclick={() => void kill(p)}>
          <Icon name="close" />
        </button>
      </li>
    {/each}
  </ul>
</div>
