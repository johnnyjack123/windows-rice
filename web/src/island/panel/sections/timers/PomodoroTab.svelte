<script lang="ts">
  import type { PomodoroSettings } from '$backend/contract';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import Segmented from '$ui/Segmented.svelte';
  import Stepper from '$ui/Stepper.svelte';
  import Switch from '$ui/Switch.svelte';
  import { useIsland } from '../../../context.svelte';
  import { PHASE_ICON } from '../../../pomodoro.svelte';
  import PomodoroDots from '../../../views/PomodoroDots.svelte';

  const { pomodoro, backend } = useIsland();
  const p = $derived(i18n.m.pomodoro);
  const s = $derived(pomodoro.state);
  const cfg = $derived(backend.settings.value.timers.pomodoro);
  const idle = $derived(s.status === 'idle');
  const shown = $derived(idle ? pomodoro.lengthOf('focus') : pomodoro.remaining);
  let editing = $state(false);

  const set = (patch: Partial<PomodoroSettings>): void => void backend.settings.patch({ timers: { pomodoro: patch } });
  const minutes = (n: number): string => i18n.m.timers.minutesShort(n);
</script>

{#if editing}
  <div class="pomo-settings">
    <div class="pomo-settings__head">
      <span class="section-label">{p.settings}</span>
      <button class="btn btn--tonal has-state" onclick={() => (editing = false)}><Icon name="check" />{p.done}</button>
    </div>
    <div class="row">
      <span class="row__title">{p.focusMinutes}</span>
      <span></span>
      <Stepper value={cfg.focusMinutes} min={5} max={120} step={5} label={p.focusMinutes} text={minutes(cfg.focusMinutes)} onchange={(v) => set({ focusMinutes: v })} />
    </div>
    <div class="row">
      <span class="row__title">{p.shortBreakMinutes}</span>
      <span></span>
      <Stepper value={cfg.shortBreakMinutes} min={1} max={30} label={p.shortBreakMinutes} text={minutes(cfg.shortBreakMinutes)} onchange={(v) => set({ shortBreakMinutes: v })} />
    </div>
    <div class="row">
      <span class="row__title">{p.longBreakMinutes}</span>
      <span></span>
      <Stepper value={cfg.longBreakMinutes} min={5} max={60} step={5} label={p.longBreakMinutes} text={minutes(cfg.longBreakMinutes)} onchange={(v) => set({ longBreakMinutes: v })} />
    </div>
    <div class="row">
      <span class="row__title">{p.sessionsUntilLongBreak}</span>
      <span></span>
      <Stepper
        value={cfg.sessionsUntilLongBreak}
        min={2}
        max={8}
        label={p.sessionsUntilLongBreak}
        text={p.sessions(cfg.sessionsUntilLongBreak)}
        onchange={(v) => set({ sessionsUntilLongBreak: v })}
      />
    </div>
    <div class="pomo-settings__mode">
      <span class="row__title">{p.breakMode}</span>
      <Segmented
        block
        label={p.breakMode}
        value={cfg.breakMode}
        options={[
          { value: 'notify', label: p.breakModes.notify },
          { value: 'overlay', label: p.breakModes.overlay },
        ]}
        onchange={(v) => set({ breakMode: v })}
      />
      <span class="pomo-settings__hint">{p.breakModeHint}</span>
    </div>
    <div class="row">
      <span class="row__title">{p.autoStartBreaks}</span>
      <span></span>
      <Switch checked={cfg.autoStartBreaks} label={p.autoStartBreaks} onchange={(v) => set({ autoStartBreaks: v })} />
    </div>
    <div class="row">
      <span class="row__title">{p.autoStartFocus}</span>
      <span></span>
      <Switch checked={cfg.autoStartFocus} label={p.autoStartFocus} onchange={(v) => set({ autoStartFocus: v })} />
    </div>
  </div>
{:else}
  <div class="pomo">
    <button class="icon-btn has-state pomo__settings" aria-label={p.settings} title={p.settings} onclick={() => (editing = true)}>
      <Icon name="tune" />
    </button>
    <div class="pomo-dial" class:is-break={!idle && s.phase !== 'focus'} class:is-paused={s.status === 'paused'} style:--p={idle ? 1 : pomodoro.progress}>
      <span class="pomo-dial__phase">
        <Icon name={PHASE_ICON[s.phase]} class="ic--sm" />{idle ? p.idle : p.phases[s.phase]}
      </span>
      <span class="pomo-dial__time num" role="timer">{fmt.duration(Math.ceil(shown / 1000))}</span>
      <PomodoroDots round={s.round} rounds={cfg.sessionsUntilLongBreak} focus={!idle && s.phase === 'focus'} />
    </div>
    <div class="pomo__buttons">
      {#if idle}
        <button class="btn btn--filled has-state" onclick={() => pomodoro.start()}><Icon name="play_arrow" />{p.start}</button>
      {:else if s.status === 'waiting'}
        <button class="btn btn--tonal has-state" onclick={() => pomodoro.stop()}><Icon name="stop" />{p.stop}</button>
        <button class="btn btn--filled has-state" onclick={() => pomodoro.startNext()}>
          <Icon name="play_arrow" />{s.phase === 'focus' ? p.startFocus : p.startBreak}
        </button>
      {:else}
        {#if s.status === 'running'}
          <button class="btn btn--tonal has-state" onclick={() => pomodoro.pause()}><Icon name="pause" />{p.pause}</button>
        {:else}
          <button class="btn btn--filled has-state" onclick={() => pomodoro.resume()}><Icon name="play_arrow" />{p.resume}</button>
        {/if}
        <button class="btn btn--tonal has-state" onclick={() => pomodoro.skip()}><Icon name="skip_next" />{p.skip}</button>
        <button class="icon-btn has-state" aria-label={p.stop} title={p.stop} onclick={() => pomodoro.stop()}><Icon name="stop" /></button>
      {/if}
    </div>
    <p class="pomo__hint">
      {idle ? p.idleHint : p.session(Math.min(cfg.sessionsUntilLongBreak, s.phase === 'focus' ? s.round + 1 : Math.max(1, s.round)), cfg.sessionsUntilLongBreak)}
    </p>
  </div>
{/if}
