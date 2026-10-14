<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';

  const { timers, backend } = useIsland();
  const ringing = $derived(timers.ringing);
  const t = $derived(i18n.m.timers);
</script>

{#if ringing}
  <div class="ring-head">
    <span class="ring-icon"><Icon name={ringing.kind === 'alarm' ? 'alarm' : 'hourglass_top'} /></span>
    <div class="truncate">
      <div class="live-sub">{ringing.kind === 'alarm' ? i18n.m.island.alarm : i18n.m.island.timerDone}</div>
      <div class="ring-time num">{ringing.kind === 'alarm' ? ringing.time : ringing.label || t.defaultName}</div>
      {#if ringing.kind === 'alarm' && ringing.label}<div class="live-sub truncate">{ringing.label}</div>{/if}
    </div>
  </div>
  <div class="live-actions">
    {#if ringing.kind === 'alarm'}
      <button class="btn btn--tonal has-state" onclick={() => timers.snooze()}>
        <Icon name="snooze" />{t.snooze(backend.settings.value.timers.snoozeMinutes)}
      </button>
      <button class="btn btn--filled has-state" onclick={() => timers.dismiss()}>{t.dismissAlarm}</button>
    {:else}
      <button class="btn btn--tonal has-state" onclick={() => timers.extendRinging()}>
        <Icon name="more_time" />{t.addMinute}
      </button>
      <button class="btn btn--filled has-state" onclick={() => timers.dismiss()}>{t.stop}</button>
    {/if}
  </div>
{/if}
