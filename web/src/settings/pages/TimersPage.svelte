<script lang="ts">
  import type { PomodoroSettings, Settings } from '$backend/contract';
  import { useBackend } from '$backend/index';
  import { i18n } from '$i18n/i18n.svelte';
  import { playSignal } from '$island/sound';
  import Icon from '$ui/Icon.svelte';
  import Range from '$ui/Range.svelte';
  import Segmented from '$ui/Segmented.svelte';
  import Stepper from '$ui/Stepper.svelte';
  import Switch from '$ui/Switch.svelte';
  import Setting from '../Setting.svelte';

  const backend = useBackend();
  const s = $derived(i18n.m.settings.timers);
  const timers = $derived(backend.settings.value.timers);
  const SOUNDS: Array<Settings['timers']['sound']> = ['chime', 'pulse', 'digital'];
  const p = $derived(i18n.m.pomodoro);
  const pomo = $derived(timers.pomodoro);
  const setPomo = (patch: Partial<PomodoroSettings>): void => void backend.settings.patch({ timers: { pomodoro: patch } });
  const minutes = (n: number): string => i18n.m.timers.minutesShort(n);

  function setPresets(text: string): void {
    const presets = text
      .split(/[,;\s]+/)
      .map(Number)
      .filter((n) => Number.isFinite(n) && n > 0 && n <= 600)
      .slice(0, 8);
    if (presets.length) {
      void backend.settings.patch({ timers: { presets } });
    }
  }
</script>

<div class="settings-page">
  <h1 class="settings-title">{i18n.m.settings.nav.timers}</h1>

  <section class="settings-card">
    <Setting label={s.sound}>
      <select
        class="select"
        aria-label={s.sound}
        value={timers.sound}
        onchange={(e) => void backend.settings.patch({ timers: { sound: e.currentTarget.value as Settings['timers']['sound'] } })}
      >
        {#each SOUNDS as sound (sound)}<option value={sound}>{s.sounds[sound]}</option>{/each}
      </select>
      <button class="btn btn--tonal has-state" onclick={() => playSignal(timers.sound, timers.soundVolume)}><Icon name="play_arrow" />{s.play}</button>
    </Setting>
    <Setting label={s.volume} range>
      <Range value={timers.soundVolume} min={0.05} max={1} step={0.05} label={s.volume} text={i18n.m.format.percent(timers.soundVolume * 100)} oninput={(v) => void backend.settings.patch({ timers: { soundVolume: v } })} />
      <span class="setting__value num">{i18n.m.format.percent(timers.soundVolume * 100)}</span>
    </Setting>
    <Setting label={s.snooze}>
      <select
        class="select"
        aria-label={s.snooze}
        value={String(timers.snoozeMinutes)}
        onchange={(e) => void backend.settings.patch({ timers: { snoozeMinutes: Number(e.currentTarget.value) } })}
      >
        {#each [5, 9, 10, 15] as minutes (minutes)}<option value={String(minutes)}>{i18n.m.timers.minutesShort(minutes)}</option>{/each}
      </select>
    </Setting>
    <Setting label={s.presets} hint={s.presetsHint}>
      <label class="field"><input value={timers.presets.join(', ')} aria-label={s.presets} onchange={(e) => setPresets(e.currentTarget.value)} /></label>
    </Setting>
  </section>

  <section class="settings-card">
    <div class="settings-card__title">{s.pomodoro}</div>
    <Setting label={p.focusMinutes}>
      <Stepper value={pomo.focusMinutes} min={5} max={120} step={5} label={p.focusMinutes} text={minutes(pomo.focusMinutes)} onchange={(v) => setPomo({ focusMinutes: v })} />
    </Setting>
    <Setting label={p.shortBreakMinutes}>
      <Stepper value={pomo.shortBreakMinutes} min={1} max={30} label={p.shortBreakMinutes} text={minutes(pomo.shortBreakMinutes)} onchange={(v) => setPomo({ shortBreakMinutes: v })} />
    </Setting>
    <Setting label={p.longBreakMinutes}>
      <Stepper value={pomo.longBreakMinutes} min={5} max={60} step={5} label={p.longBreakMinutes} text={minutes(pomo.longBreakMinutes)} onchange={(v) => setPomo({ longBreakMinutes: v })} />
    </Setting>
    <Setting label={p.sessionsUntilLongBreak}>
      <Stepper
        value={pomo.sessionsUntilLongBreak}
        min={2}
        max={8}
        label={p.sessionsUntilLongBreak}
        text={p.sessions(pomo.sessionsUntilLongBreak)}
        onchange={(v) => setPomo({ sessionsUntilLongBreak: v })}
      />
    </Setting>
    <Setting label={p.breakMode} hint={p.breakModeHint}>
      <Segmented
        label={p.breakMode}
        value={pomo.breakMode}
        options={[
          { value: 'notify', label: p.breakModes.notify },
          { value: 'overlay', label: p.breakModes.overlay },
        ]}
        onchange={(v) => setPomo({ breakMode: v })}
      />
    </Setting>
    <Setting label={p.autoStartBreaks}>
      <Switch checked={pomo.autoStartBreaks} label={p.autoStartBreaks} onchange={(v) => setPomo({ autoStartBreaks: v })} />
    </Setting>
    <Setting label={p.autoStartFocus}>
      <Switch checked={pomo.autoStartFocus} label={p.autoStartFocus} onchange={(v) => setPomo({ autoStartFocus: v })} />
    </Setting>
  </section>
</div>
