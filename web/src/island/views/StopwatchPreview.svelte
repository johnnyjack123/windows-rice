<script lang="ts">
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';

  const { timers } = useIsland();
  const t = $derived(i18n.m.timers);
  const sw = $derived(timers.stopwatch);
</script>

<div class="qa-head">
  <span class="qa-icon"><Icon name="timer" /></span>
  <span class="stack">
    <span class="live-sub">{sw.laps.length ? t.lapN(sw.laps.length + 1) : t.modes.stopwatch}</span>
    <span class="qa-time num" role="timer">{fmt.stopwatch(timers.stopwatchElapsed)}</span>
  </span>
</div>
<div class="qa-actions">
  {#if sw.running}
    <button class="qa-btn has-state" onclick={() => timers.lap()}><Icon name="flag" />{t.lap}</button>
  {:else}
    <button class="qa-btn has-state" onclick={() => timers.resetStopwatch()}><Icon name="restart_alt" />{t.reset}</button>
  {/if}
  <button class="qa-btn has-state" onclick={() => timers.toggleStopwatch()}>
    <Icon name={sw.running ? 'pause' : 'play_arrow'} />{sw.running ? t.pause : t.resume}
  </button>
</div>
