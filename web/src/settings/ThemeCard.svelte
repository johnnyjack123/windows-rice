<script lang="ts">
  import type { Scheme } from '$backend/contract';
  import type { ThemeMeta } from '$design/themes/registry';
  import { i18n } from '$i18n/i18n.svelte';
  import Equalizer from '$ui/Equalizer.svelte';

  interface Props {
    theme: ThemeMeta;
    scheme: Scheme;
    selected: boolean;
    onselect: () => void;
  }

  let { theme, scheme, selected, onselect }: Props = $props();
  const shownScheme = $derived(theme.schemes.includes(scheme) ? scheme : theme.schemes[0]!);
  const tagline = $derived(
    theme.builtIn ? (i18n.m.themes[theme.id as keyof typeof i18n.m.themes] ?? '') : i18n.m.themes.customTagline,
  );
</script>

<button class="theme-card" role="radio" aria-checked={selected} onclick={onselect}>
  <span class="theme-card__stage theme-scope" data-theme={theme.id} data-scheme={shownScheme}>
    <span class="theme-card__pill">
      <span class="art art--island" style:--art="none" style:background="var(--accent)"></span>
      <span class="num">14:32</span>
      <Equalizer playing />
    </span>
    <span class="theme-card__swatches" aria-hidden="true">
      <i style:background="var(--color-surface-container-highest)"></i>
      <i style:background="var(--color-accent)"></i>
      <i style:background="var(--color-text)"></i>
    </span>
  </span>
  <span class="theme-card__name">{theme.name}</span>
  <span class="theme-card__meta">{theme.builtIn ? tagline : i18n.m.settings.appearance.by(theme.author)}</span>
</button>
