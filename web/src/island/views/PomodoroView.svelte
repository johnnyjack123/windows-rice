<script lang="ts">
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';
  import { PHASE_ICON } from '../pomodoro.svelte';

  const { pomodoro } = useIsland();
  const s = $derived(pomodoro.state);
</script>

<span class="pomo-island" class:is-break={s.phase !== 'focus'} class:is-paused={s.status !== 'running'}>
  <Icon name={PHASE_ICON[s.phase]} />
  <span class="pomo-island__phase">{i18n.m.pomodoro.phases[s.phase]}</span>
  <span class="timer-time num">{fmt.duration(Math.ceil(pomodoro.remaining / 1000))}</span>
</span>
