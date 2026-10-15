<script lang="ts">
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';

  const { timers } = useIsland();
  const t = $derived(i18n.m.timers);
  const soonest = $derived(timers.active[0]);
  const more = $derived(timers.active.length - 1);
  const remaining = $derived(soonest ? timers.remainingOf(soonest) : 0);
</script>

{#if soonest}
  <div class="qa-head">
    <span class="qa-ring" class:is-paused={soonest.state === 'paused'} style:--p={soonest.duration ? remaining / soonest.duration : 0}>
      <Icon name="hourglass_top" class="ic--sm" />
    </span>
    <span class="stack">
      <span class="live-sub truncate">{soonest.label || t.defaultName}{more > 0 ? ` · ${t.moreTimers(more)}` : ''}</span>
      <span class="qa-time num" role="timer">{fmt.duration(Math.ceil(remaining / 1000))}</span>
    </span>
  </div>
  <div class="qa-actions">
    <button class="qa-btn has-state" onclick={() => timers.addMinute(soonest.id)}><Icon name="more_time" />{t.addMinute}</button>
    {#if soonest.state === 'running'}
      <button class="qa-btn has-state" onclick={() => timers.pause(soonest.id)}><Icon name="pause" />{t.pause}</button>
    {:else}
      <button class="qa-btn has-state" onclick={() => timers.resume(soonest.id)}><Icon name="play_arrow" />{t.resume}</button>
    {/if}
    <button class="qa-btn has-state" onclick={() => timers.cancel(soonest.id)}><Icon name="close" />{t.cancel}</button>
  </div>
{/if}
