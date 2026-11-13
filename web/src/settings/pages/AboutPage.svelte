<script lang="ts">
  import { onMount } from 'svelte';
  import type { AutostartState } from '$backend/contract';
  import { useBackend } from '$backend/index';
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import Switch from '$ui/Switch.svelte';
  import Setting from '../Setting.svelte';

  const backend = useBackend();
  const a = $derived(i18n.m.settings.about);
  const info = $derived(backend.system.info);
  let autostart = $state<AutostartState | null>(null);

  onMount(() => {
    void backend.transport.invoke('app_autostart_get').then((s) => (autostart = s));
  });

  const open = (target: string | undefined): void => {
    if (target) {
      void backend.transport.invoke('shell_open', { target });
    }
  };

  /** Flips the switch at once; the backend's answer (or the old state on failure) follows. */
  const setAutostart = async (enabled: boolean): Promise<void> => {
    const before = autostart;
    if (before) {
      autostart = { ...before, enabled };
    }
    try {
      autostart = await backend.transport.invoke('app_autostart_set', { enabled });
    } catch {
      autostart = before;
    }
  };
</script>

<div class="settings-page">
  <h1 class="settings-title">{i18n.m.settings.nav.about}</h1>
  <section class="settings-card">
    <div class="setting">
      <div>
        <div class="setting__label">rice</div>
        <div class="setting__hint">{a.version(info?.version ?? '–')} · {a.backend(backend.transport.kind === 'mock' ? a.mock : a.native)}</div>
      </div>
    </div>
    <div class="form-actions">
      <button class="btn btn--tonal has-state" onclick={() => open(info?.themesDir)}><Icon name="folder_open" />{a.themesFolder}</button>
      <button class="btn btn--tonal has-state" onclick={() => open(info?.configDir)}><Icon name="folder_open" />{a.configFolder}</button>
    </div>
  </section>

  <section class="settings-card">
    <div class="settings-card__title">{a.app}</div>
    <Setting label={a.autostart} hint={autostart?.supported === false ? a.autostartUnsupported : a.autostartHint}>
      <Switch
        checked={autostart?.enabled ?? false}
        label={a.autostart}
        disabled={!autostart?.supported}
        onchange={(v) => void setAutostart(v)}
      />
    </Setting>
    <Setting label={a.quit} hint={a.quitHint}>
      <button class="btn btn--tonal has-state" onclick={() => void backend.transport.invoke('app_quit')}>
        <Icon name="power_settings_new" />{a.quitAction}
      </button>
    </Setting>
  </section>
</div>
