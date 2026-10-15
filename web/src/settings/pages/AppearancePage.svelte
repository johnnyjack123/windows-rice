<script lang="ts">
  import { onMount } from 'svelte';
  import type { Scheme, SchemePreference, ThemeOverrides } from '$backend/contract';
  import { useBackend } from '$backend/index';
  import { contrastRatio, mixHex } from '$design/color';
  import { buildUserThemeCss, CONTROLS, readThemeDefaults, readThemeMaterial, type ControlDef, type ControlGroup, type EditorValues } from '$design/editor';
  import { useEngine } from '$design/engine.svelte';
  import { themeIdFrom } from '$design/themes/registry';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Icon from '$ui/Icon.svelte';
  import Segmented from '$ui/Segmented.svelte';
  import Slider from '$ui/Slider.svelte';
  import Tile from '$ui/Tile.svelte';
  import EditorControl from '../EditorControl.svelte';
  import MotionPreview from '../MotionPreview.svelte';
  import ThemeCard from '../ThemeCard.svelte';

  let { toast }: { toast: (message: string) => void } = $props();

  const backend = useBackend();
  const engine = useEngine();
  const a = $derived(i18n.m.settings.appearance);
  const ed = $derived(i18n.m.settings.editor);

  const settings = $derived(backend.settings.value);
  const themeId = $derived(engine.themeId);
  const scheme = $derived(engine.scheme);
  const meta = $derived(engine.themes.find((t) => t.id === themeId));
  const overrides: ThemeOverrides = $derived(settings.theme.overrides[themeId] ?? {});

  // Theme defaults are read from the CSS itself, so they always match it.
  // Defaults belong to the theme alone (the probe ignores overrides), so they are
  // re-read when the theme, scheme or a theme file changes – not on every slider step.
  const defaults: EditorValues = $derived.by(() => {
    void engine.themes;
    return readThemeDefaults(themeId, scheme);
  });

  const COLOR_KEYS = ['background', 'foreground', 'accent'] as const;
  type ColorKey = (typeof COLOR_KEYS)[number];
  const isColor = (key: string): key is ColorKey => (COLOR_KEYS as readonly string[]).includes(key);

  function valueOf(key: keyof EditorValues): EditorValues[keyof EditorValues] {
    if (isColor(key)) {
      return overrides.colors?.[scheme]?.[key] ?? defaults[key];
    }
    const v = (overrides as Record<string, unknown>)[key];
    return (v ?? defaults[key]) as EditorValues[keyof EditorValues];
  }

  function isModified(key: keyof EditorValues): boolean {
    return isColor(key) ? overrides.colors?.[scheme]?.[key] !== undefined : (overrides as Record<string, unknown>)[key] !== undefined;
  }

  function set(key: keyof EditorValues, value: EditorValues[keyof EditorValues] | null): void {
    const patch = isColor(key) ? { colors: { [scheme]: { [key]: value } } } : { [key]: value };
    void backend.settings.patch({ theme: { overrides: { [themeId]: patch } } });
  }

  function resetAll(): void {
    void backend.settings.patch({ theme: { overrides: { [themeId]: null } } });
  }

  const material = $derived.by(() => {
    void engine.themes;
    return readThemeMaterial(themeId, scheme);
  });
  const groups = $derived<ControlGroup[]>(['colors', 'type', 'shape', ...(material === 'glass' ? (['material'] as const) : []), 'motion', 'icons']);
  const byGroup = (g: ControlGroup): ControlDef[] => CONTROLS.filter((c) => c.group === g && (!c.material || c.material === material));
  const anyModified = $derived(Object.keys(overrides).length > 0);

  // Contrast of the user's colours, so unreadable combinations are flagged.
  const contrast = $derived.by(() => {
    const bg = String(valueOf('background'));
    const fg = String(valueOf('foreground'));
    const muted = mixHex(fg, bg, 0.66);
    return { main: contrastRatio(fg, bg), muted: contrastRatio(muted, bg) };
  });

  let newName = $state('');
  let confirmDelete = $state(false);

  async function saveAs(): Promise<void> {
    const name = newName.trim();
    if (!name || !meta) {
      return;
    }
    let id = themeIdFrom(name);
    const taken = new Set(engine.themes.map((t) => t.id));
    for (let i = 2; taken.has(id); i++) {
      id = `${themeIdFrom(name)}-${i}`;
    }
    const css = buildUserThemeCss(meta, id, overrides);
    await backend.themes.save(id, { name, author: a.you, version: '1.0.0', schemes: meta.schemes, basedOn: meta.id }, css);
    await backend.settings.patch({ theme: { id } });
    newName = '';
    toast(a.savedAs(name));
  }

  async function deleteTheme(): Promise<void> {
    if (!meta || meta.builtIn) {
      return;
    }
    if (!confirmDelete) {
      confirmDelete = true;
      return;
    }
    confirmDelete = false;
    await backend.settings.patch({ theme: { id: 'obsidian', overrides: { [meta.id]: null } } });
    await backend.themes.remove(meta.id);
  }

  onMount(() => void backend.system.loadFonts());

  const schemeOptions = $derived<Array<{ value: SchemePreference; label: string; disabled?: boolean }>>([
    { value: 'auto', label: a.schemeAuto },
    { value: 'dark', label: a.schemeDark, disabled: !meta?.schemes.includes('dark') },
    { value: 'light', label: a.schemeLight, disabled: !meta?.schemes.includes('light') },
  ]);
  const onlyScheme = $derived(meta && meta.schemes.length === 1 ? (meta.schemes[0] as Scheme) : null);
</script>

<div class="settings-page">
  <h1 class="settings-title">{i18n.m.settings.nav.appearance}</h1>

  <section class="settings-card" aria-label={a.themes}>
    <div class="settings-card__title">{a.themes}</div>
    <div class="theme-grid" role="radiogroup" aria-label={a.themes}>
      {#each engine.themes as theme (theme.id)}
        <ThemeCard {theme} {scheme} selected={theme.id === themeId} onselect={() => void backend.settings.patch({ theme: { id: theme.id } })} />
      {/each}
    </div>
    <div class="setting">
      <div>
        <div class="setting__label">{a.scheme}</div>
        {#if onlyScheme}<div class="setting__hint">{a.schemeOnly(onlyScheme === 'dark' ? a.schemeOnlyDark : a.schemeOnlyLight)}</div>{/if}
      </div>
      <Segmented label={a.scheme} value={settings.theme.scheme} options={schemeOptions} onchange={(v) => void backend.settings.patch({ theme: { scheme: v } })} />
    </div>
  </section>

  <div class="editor">
    <div class="form-grid">
      <section class="settings-card">
        <div class="settings-card__title">{a.customize} · {meta?.name}</div>
        <div class="settings-card__hint">{a.customizeHint}</div>
      </section>

      {#each groups as group (group)}
        <section class="settings-card" aria-label={ed.groups[group]}>
          <div class="settings-card__title">{ed.groups[group]}</div>
          {#each byGroup(group) as def (def.key)}
            <EditorControl
              {def}
              value={valueOf(def.key)}
              fallback={defaults[def.key]}
              modified={isModified(def.key)}
              systemFonts={backend.system.fonts}
              label={def.key === 'background' && settings.theme.id === 'glass' ? ed.glassTint : undefined}
              onchange={(v) => set(def.key, v)}
              onreset={() => set(def.key, null)}
            />
            {#if def.key === 'foreground'}
              <p class="contrast-note" class:is-low={contrast.muted < 4.5}>
                {contrast.muted < 4.5 ? ed.contrastLow(fmt.num(contrast.muted)) : ed.contrastOk(fmt.num(contrast.main))}
              </p>
            {/if}
          {/each}
        </section>
      {/each}

      <section class="settings-card">
        <div class="form-actions">
          <button class="btn btn--tonal has-state" disabled={!anyModified} onclick={resetAll}><Icon name="restart_alt" />{a.resetAll}</button>
        </div>
        <div class="setting">
          <label class="field">
            <Icon name="palette" class="ic--sm" />
            <input bind:value={newName} placeholder={a.themeName} aria-label={a.themeName} />
          </label>
          <button class="btn btn--filled has-state" disabled={!newName.trim()} onclick={() => void saveAs()}><Icon name="save" />{a.saveAs}</button>
        </div>
        {#if meta && !meta.builtIn}
          <div class="form-actions">
            <button class="btn btn--danger has-state" onclick={() => void deleteTheme()}>
              <Icon name="delete" />{confirmDelete ? a.deleteConfirm(meta.name) : a.deleteTheme}
            </button>
          </div>
        {/if}
      </section>
    </div>

    <aside class="editor__preview" aria-label={a.preview}>
      <MotionPreview />
      <div class="preview-panel">
        <div class="tiles">
          <Tile id="wifi" label={i18n.m.tiles.wifi} sub="FRITZ!Box 7590" icon="wifi" on detail />
          <Tile id="dnd" label={i18n.m.tiles.dnd} sub={i18n.m.common.off} icon="do_not_disturb_on" on={false} />
        </div>
        <Slider value={0.62} icon="volume_up" label={i18n.m.sliders.volume} onchange={() => {}} />
        <div class="form-actions">
          <span class="chip is-selected">{i18n.m.tasks.all}</span>
          <button class="btn btn--filled">{i18n.m.timers.start}</button>
        </div>
      </div>
    </aside>
  </div>
</div>
