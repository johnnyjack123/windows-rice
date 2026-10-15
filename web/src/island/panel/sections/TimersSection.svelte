<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import Segmented from '$ui/Segmented.svelte';
  import AlarmTab from './timers/AlarmTab.svelte';
  import PomodoroTab from './timers/PomodoroTab.svelte';
  import StopwatchTab from './timers/StopwatchTab.svelte';
  import TimerTab from './timers/TimerTab.svelte';

  type Mode = 'timer' | 'stopwatch' | 'alarm' | 'pomodoro';
  let mode = $state<Mode>('timer');
  const t = $derived(i18n.m.timers);
</script>

<div class="timers-mode">
  <Segmented
    block
    label={i18n.m.sections.timers}
    value={mode}
    options={[
      { value: 'timer', label: t.modes.timer },
      { value: 'stopwatch', label: t.modes.stopwatch },
      { value: 'alarm', label: t.modes.alarm },
      { value: 'pomodoro', label: t.modes.pomodoro },
    ]}
    onchange={(v) => (mode = v)}
  />
</div>

<div class="pane__scroll scroll">
  {#if mode === 'timer'}
    <TimerTab />
  {:else if mode === 'stopwatch'}
    <StopwatchTab />
  {:else if mode === 'alarm'}
    <AlarmTab />
  {:else}
    <PomodoroTab />
  {/if}
</div>
