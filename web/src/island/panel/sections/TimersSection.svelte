<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import Segmented from '$ui/Segmented.svelte';
  import AlarmTab from './timers/AlarmTab.svelte';
  import StopwatchTab from './timers/StopwatchTab.svelte';
  import TimerTab from './timers/TimerTab.svelte';

  type Mode = 'timer' | 'stopwatch' | 'alarm';
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
    ]}
    onchange={(v) => (mode = v)}
  />
</div>

<div class="pane__scroll scroll">
  {#if mode === 'timer'}
    <TimerTab />
  {:else if mode === 'stopwatch'}
    <StopwatchTab />
  {:else}
    <AlarmTab />
  {/if}
</div>
