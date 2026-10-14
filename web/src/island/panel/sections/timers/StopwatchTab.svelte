<script lang="ts">
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../../../context.svelte';

  const { timers } = useIsland();
  const t = $derived(i18n.m.timers);
  const sw = $derived(timers.stopwatch);

  /** Lap durations (not split times), newest first, with extremes marked. */
  const laps = $derived.by(() => {
    const splits = sw.laps;
    const durations = splits.map((s, i) => s - (splits[i - 1] ?? 0));
    const fastest = durations.length > 1 ? Math.min(...durations) : -1;
    const slowest = durations.length > 1 ? Math.max(...durations) : -1;
    return durations
      .map((d, i) => ({ n: i + 1, d, total: splits[i]!, fastest: d === fastest, slowest: d === slowest }))
      .reverse();
  });
</script>

<div class="stopwatch">
  <div class="stopwatch__time num" role="timer" aria-label={t.modes.stopwatch}>{fmt.stopwatch(timers.stopwatchElapsed)}</div>
  <div class="stopwatch__buttons">
    {#if sw.running}
      <button class="btn btn--tonal has-state" onclick={() => timers.lap()}><Icon name="flag" />{t.lap}</button>
    {:else}
      <button class="btn btn--tonal has-state" disabled={timers.stopwatchElapsed === 0} onclick={() => timers.resetStopwatch()}>
        <Icon name="restart_alt" />{t.reset}
      </button>
    {/if}
    <button class="btn btn--filled has-state" onclick={() => timers.toggleStopwatch()}>
      <Icon name={sw.running ? 'pause' : 'play_arrow'} />{sw.running ? t.stopwatchStop : t.stopwatchStart}
    </button>
  </div>
</div>

{#if laps.length}
  <ul aria-label={t.lap}>
    {#each laps as lap (lap.n)}
      <li class="lap num" class:is-fastest={lap.fastest} class:is-slowest={lap.slowest}>
        <span>{t.lapN(lap.n)}{lap.fastest ? ` · ${t.fastest}` : lap.slowest ? ` · ${t.slowest}` : ''}</span>
        <span>{fmt.stopwatch(lap.d)}</span>
        <span>{fmt.stopwatch(lap.total)}</span>
      </li>
    {/each}
  </ul>
{/if}
