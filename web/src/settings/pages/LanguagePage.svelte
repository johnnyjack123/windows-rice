<script lang="ts">
  import { useBackend } from '$backend/index';
  import { i18n, LOCALES } from '$i18n/i18n.svelte';

  const backend = useBackend();
  const l = $derived(i18n.m.settings.language);
  const current = $derived(backend.settings.value.locale);
  const options = $derived([
    { value: 'auto' as const, label: l.auto },
    ...LOCALES.map((locale) => ({ value: locale, label: i18n.catalogue(locale).meta.name })),
  ]);
</script>

<div class="settings-page">
  <h1 class="settings-title">{i18n.m.settings.nav.language}</h1>
  <section class="settings-card" aria-label={l.label}>
    <div class="settings-card__title">{l.label}</div>
    <div class="settings-card__hint">{l.hint}</div>
    {#each options as option (option.value)}
      <label class="lang-option has-state">
        <input
          type="radio"
          name="locale"
          value={option.value}
          checked={current === option.value}
          onchange={() => void backend.settings.patch({ locale: option.value })}
        />
        <span class="setting__label">{option.label}</span>
      </label>
    {/each}
  </section>
</div>
