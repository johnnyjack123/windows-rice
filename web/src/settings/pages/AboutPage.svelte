<script lang="ts">
  import { useBackend } from '$backend/index';
  import { i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';

  const backend = useBackend();
  const a = $derived(i18n.m.settings.about);
  const info = $derived(backend.system.info);
  const open = (target: string | undefined): void => {
    if (target) {
      void backend.transport.invoke('shell_open', { target });
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
</div>
