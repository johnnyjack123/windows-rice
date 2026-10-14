<script lang="ts">
  import { onMount } from 'svelte';
  import type { MonitorInfo, SettingsPatch } from '$backend/contract';
  import { useBackend } from '$backend/index';
  import { i18n } from '$i18n/i18n.svelte';
  import Range from '$ui/Range.svelte';
  import Switch from '$ui/Switch.svelte';
  import Setting from '../Setting.svelte';

  const backend = useBackend();
  const s = $derived(i18n.m.settings.island);
  const island = $derived(backend.settings.value.island);
  let monitors = $state<MonitorInfo[]>([]);

  onMount(() => {
    void backend.transport.invoke('system_monitors').then((m) => (monitors = m));
  });

  const set = (patch: SettingsPatch['island']): void => void backend.settings.patch({ island: patch });
</script>

<div class="settings-page">
  <h1 class="settings-title">{i18n.m.settings.nav.island}</h1>

  <section class="settings-card">
    <div class="settings-card__title">{s.behaviour}</div>
    <Setting label={s.autoHide} hint={s.autoHideHint}>
      <Switch checked={island.autoHide} label={s.autoHide} onchange={(v) => set({ autoHide: v })} />
    </Setting>
    <Setting label={s.peek} hint={s.peekHint}>
      <Switch checked={island.peek} label={s.peek} disabled={!island.autoHide} onchange={(v) => set({ peek: v })} />
    </Setting>
    <Setting label={s.hideFullscreen}>
      <Switch checked={island.hideInFullscreen} label={s.hideFullscreen} onchange={(v) => set({ hideInFullscreen: v })} />
    </Setting>
    <Setting label={s.hoverPreview}>
      <Switch checked={island.hoverPreview} label={s.hoverPreview} onchange={(v) => set({ hoverPreview: v })} />
    </Setting>
    <Setting label={s.scrollVolume}>
      <Switch checked={island.scrollVolume} label={s.scrollVolume} onchange={(v) => set({ scrollVolume: v })} />
    </Setting>
  </section>

  <section class="settings-card">
    <div class="settings-card__title">{s.placement}</div>
    <Setting label={s.monitor}>
      <select
        class="select"
        aria-label={s.monitor}
        value={String(island.monitor)}
        onchange={(e) => set({ monitor: e.currentTarget.value === 'primary' ? 'primary' : Number(e.currentTarget.value) })}
      >
        <option value="primary">{s.primaryMonitor}</option>
        {#each monitors as m (m.index)}
          <option value={String(m.index)}>{s.monitorN(m.index + 1)} · {m.name} ({m.width}×{m.height})</option>
        {/each}
      </select>
    </Setting>
    <Setting label={s.offsetTop} range>
      <Range value={island.offsetTop} min={0} max={40} step={1} label={s.offsetTop} text={`${island.offsetTop} px`} oninput={(v) => set({ offsetTop: v })} />
      <span class="setting__value num">{island.offsetTop} px</span>
    </Setting>
    <Setting label={s.hotkey} hint={s.hotkeyHint}>
      <label class="field">
        <input value={island.hotkey} aria-label={s.hotkey} onchange={(e) => set({ hotkey: e.currentTarget.value.trim() })} />
      </label>
    </Setting>
  </section>
</div>
