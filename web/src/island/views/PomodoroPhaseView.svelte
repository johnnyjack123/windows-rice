<script lang="ts">
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';
  import { PHASE_ICON } from '../pomodoro.svelte';

  const { pomodoro } = useIsland();
  const p = $derived(i18n.m.pomodoro);
  const phase = $derived(pomodoro.change?.phase ?? pomodoro.state.phase);
  const title = $derived(phase === 'focus' ? p.focusNow : phase === 'long' ? p.longBreakNow : p.breakNow);
</script>

<div class="ring-head">
  <span class="ring-icon pomo-phase-icon" class:is-break={phase !== 'focus'}><Icon name={PHASE_ICON[phase]} /></span>
  <div class="truncate">
    <div class="live-title">{title}</div>
    <div class="live-sub">{p.phaseLength(p.phases[phase], Math.round(pomodoro.lengthOf(phase) / 60000))}</div>
  </div>
</div>
