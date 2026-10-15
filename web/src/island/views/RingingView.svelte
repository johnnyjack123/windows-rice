<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';
  import { PHASE_ICON, type PomodoroPhase } from '../pomodoro.svelte';

  const { timers, pomodoro, backend } = useIsland();
  const ringing = $derived(timers.ringing);
  const t = $derived(i18n.m.timers);
  const p = $derived(i18n.m.pomodoro);
  /** For the pomodoro the label is the phase that comes next. */
  const next = $derived((ringing?.kind === 'pomodoro' ? ringing.label : 'focus') as PomodoroPhase);
</script>

{#if ringing?.kind === 'pomodoro'}
  <div class="ring-head">
    <span class="ring-icon" class:is-break={next !== 'focus'}><Icon name={PHASE_ICON[next]} /></span>
    <div class="truncate">
      <div class="live-sub">{next === 'focus' ? p.breakDone : p.focusDone}</div>
      <div class="ring-time">{p.phaseLength(p.phases[next], Math.round(pomodoro.lengthOf(next) / 60000))}</div>
    </div>
  </div>
  <div class="live-actions">
    <button class="btn btn--tonal has-state" onclick={() => pomodoro.stop()}><Icon name="stop" />{p.stop}</button>
    <button class="btn btn--filled has-state" onclick={() => pomodoro.startNext()}>
      <Icon name="play_arrow" />{next === 'focus' ? p.startFocus : p.startBreak}
    </button>
  </div>
{:else if ringing}
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
