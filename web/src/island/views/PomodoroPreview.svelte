<script lang="ts">
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';
  import { PHASE_ICON } from '../pomodoro.svelte';
  import PomodoroDots from './PomodoroDots.svelte';

  const { pomodoro } = useIsland();
  const p = $derived(i18n.m.pomodoro);
  const s = $derived(pomodoro.state);
</script>

<div class="qa-head">
  <span class="qa-ring" class:is-break={s.phase !== 'focus'} class:is-paused={s.status !== 'running'} style:--p={pomodoro.progress}>
    <Icon name={PHASE_ICON[s.phase]} class="ic--sm" />
  </span>
  <span class="stack">
    <span class="live-sub truncate">{p.phases[s.phase]}</span>
    <span class="qa-time num" role="timer">{fmt.duration(Math.ceil(pomodoro.remaining / 1000))}</span>
  </span>
</div>
<PomodoroDots round={s.round} rounds={pomodoro.config.sessionsUntilLongBreak} focus={s.phase === 'focus'} />
<div class="qa-actions">
  {#if s.status === 'running'}
    <button class="qa-btn has-state" onclick={() => pomodoro.pause()}><Icon name="pause" />{p.pause}</button>
  {:else}
    <button class="qa-btn has-state" onclick={() => pomodoro.start()}><Icon name="play_arrow" />{p.resume}</button>
  {/if}
  <button class="qa-btn has-state" onclick={() => pomodoro.skip()}><Icon name="skip_next" />{p.skip}</button>
  <button class="qa-btn has-state" onclick={() => pomodoro.stop()}><Icon name="stop" />{p.stop}</button>
</div>
