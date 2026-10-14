<script lang="ts">
  import { onMount } from 'svelte';
  import type { Backend } from '$backend/index';
  import { WALLPAPERS } from '$backend/mock/art';
  import type { ThemeEngine } from '$design/engine.svelte';
  import { BUILTIN_THEMES } from '$design/themes/registry';
  import { i18n } from '$i18n/i18n.svelte';
  import type { IslandContext } from '$island/context.svelte';
  import IslandApp from '$island/IslandApp.svelte';

  interface Props {
    backend: Backend;
    engine: ThemeEngine;
  }

  const props: Props = $props();
  // svelte-ignore state_referenced_locally
  const { backend, engine } = props;
  const mock = backend.mock;
  const l = $derived(i18n.m.lab);

  let ctx = $state<IslandContext | null>(null);
  let backdrop = $state<'none' | 'light' | 'dark'>('none');
  let wallpaper = $state(WALLPAPERS[0]!.id);
  let collapsed = $state(false);
  let hiddenLab = $state(false);
  let tempo = $state(1);
  let reduced = $state(false);
  let message = $state('');
  let toastTimer: ReturnType<typeof setTimeout> | undefined;

  const settings = $derived(backend.settings.value);
  const themes = $derived(engine.themes);
  const theme = $derived(themes.find((t) => t.id === engine.themeId) ?? BUILTIN_THEMES[0]!);

  function toast(text: string): void {
    message = text;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (message = ''), 1500);
  }

  function setTheme(id: string): void {
    void backend.settings.patch({ theme: { id } });
    toast(themes.find((t) => t.id === id)?.name ?? id);
  }

  function setScheme(scheme: 'dark' | 'light'): void {
    void backend.settings.patch({ theme: { scheme } });
  }

  /** A window behind the island also counts as overlapping it. */
  function setBackdrop(mode: 'none' | 'light' | 'dark'): void {
    backdrop = mode;
    mock?.debug.setOccluded(mode !== 'none');
  }

  function setWallpaper(id: string, url: string): void {
    wallpaper = id;
    mock?.debug.setWallpaper(url);
  }

  function upload(e: Event & { currentTarget: HTMLInputElement }): void {
    const file = e.currentTarget.files?.[0];
    if (file) {
      setWallpaper('upload', URL.createObjectURL(file));
    }
  }

  function setTempo(value: number): void {
    tempo = value;
    engine.setTimeScale(value);
    toast(l.tempoToast(String(value).replace('.', i18n.locale === 'de' ? ',' : '.')));
  }

  function setReduced(value: boolean): void {
    reduced = value;
    engine.reducedOverride = value;
  }

  const EVENTS: Array<{ key: string; label: () => string; run: () => void }> = [
    { key: 'n', label: () => l.notification, run: () => mock?.debug.pushNotification() },
    { key: 'v', label: () => l.volume, run: () => mock?.debug.changeVolume(Math.random() > 0.5 ? 0.12 : -0.12) },
    { key: 'b', label: () => l.brightness, run: () => mock?.debug.changeBrightness(Math.random() > 0.5 ? 0.15 : -0.15) },
    { key: 'm', label: () => l.playPause, run: () => mock?.debug.togglePlay() },
    { key: '.', label: () => l.nextTrack, run: () => mock?.debug.nextTrack() },
    { key: 'e', label: () => l.event, run: () => mock?.debug.addSoonEvent() },
    { key: 't', label: () => l.timer, run: () => ctx?.timers.start(5 * 60_000, '') },
    { key: 'c', label: () => l.charging, run: () => mock?.debug.toggleCharging() },
    {
      key: 'a',
      label: () => l.alarm,
      run: () => {
        const now = new Date();
        const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
        ctx?.timers.ringNow({ id: 'lab', time, label: l.alarm, days: [], enabled: true });
      },
    },
  ];
  const TEMPOS = [1, 0.5, 0.2];

  onMount(() => {
    const offShell = mock?.onShellOpen.on((target) => toast(l.opens(target)));
    const key = (e: KeyboardEvent): void => {
      const target = e.target instanceof Element ? e.target : document.body;
      if (e.ctrlKey || e.metaKey || e.altKey || target.closest('input, textarea, select, [contenteditable]')) {
        return;
      }
      const k = e.key.toLowerCase();
      const index = Number(k) - 1;
      if (index >= 0 && index < themes.length && index < 9) {
        setTheme(themes[index]!.id);
      } else if (k === 'h') {
        hiddenLab = !hiddenLab;
      } else if (k === 'd') {
        setScheme(engine.scheme === 'dark' ? 'light' : 'dark');
      } else if (k === 's') {
        setTempo(TEMPOS[(TEMPOS.indexOf(tempo) + 1) % TEMPOS.length]!);
      } else if (k === 'r') {
        setReduced(!reduced);
      } else if (k === ' ' && !target.closest('button, [role="slider"], [role="tab"], .island')) {
        e.preventDefault();
        ctx?.controller.toggle();
      } else {
        EVENTS.find((ev) => ev.key === k)?.run();
      }
    };
    document.addEventListener('keydown', key);
    return () => {
      offShell?.();
      document.removeEventListener('keydown', key);
    };
  });
</script>

<main class="stage" data-backdrop={backdrop}>
  <div class="wallpaper"></div>
  {#each ['light', 'dark'] as kind (kind)}
    <div class="test-window test-window--{kind}" aria-hidden="true">
      <div class="test-window__bar"><div class="test-window__tab"></div></div>
      <div class="test-window__body">
        <div class="test-window__h"></div>
        {#each [92, 86, 95, 64, 88, 79] as width, i (i)}<div class="test-window__line" style:width="{width}%"></div>{/each}
      </div>
    </div>
  {/each}
  <IslandApp {backend} {engine} onready={(c) => (ctx = c)} />
</main>

<aside class="lab" class:is-collapsed={collapsed} hidden={hiddenLab} data-lab aria-label={l.title}>
  <div class="lab__head">
    <div class="lab__title">{l.title} <span>· rice</span></div>
    <button class="lab__collapse" aria-label={l.collapse} onclick={() => (collapsed = !collapsed)}>
      <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"><path d="M4 6l4 4 4-4" /></svg>
    </button>
  </div>
  <div class="lab__body">
    <h3>{l.theme}</h3>
    <div class="lab-concepts">
      {#each themes as t, i (t.id)}
        <button class="lab-concept" class:is-active={t.id === engine.themeId} onclick={() => setTheme(t.id)}>
          <kbd>{i + 1}</kbd><span class="lab-concept__name">{t.name}</span>
          <span class="lab-concept__tag">{t.builtIn ? i18n.m.themes[t.id as keyof typeof i18n.m.themes] : i18n.m.themes.customTagline}</span>
        </button>
      {/each}
    </div>
    <div class="lab-row">
      <span>{l.scheme}</span>
      <div class="lab-seg">
        <button class:is-active={engine.scheme === 'dark'} disabled={!theme.schemes.includes('dark')} onclick={() => setScheme('dark')}>
          {i18n.m.settings.appearance.schemeDark}
        </button>
        <button class:is-active={engine.scheme === 'light'} disabled={!theme.schemes.includes('light')} onclick={() => setScheme('light')}>
          {i18n.m.settings.appearance.schemeLight}
        </button>
      </div>
    </div>

    <h3>{l.background}</h3>
    <div class="lab-walls">
      {#each WALLPAPERS as w (w.id)}
        <button class="lab-wall" class:is-active={wallpaper === w.id} style:--thumb="url('{w.url}')" title={w.name} aria-label={w.name} onclick={() => setWallpaper(w.id, w.url)}></button>
      {/each}
      <label class="lab-wall lab-wall--upload" class:is-active={wallpaper === 'upload'} title={l.uploadWallpaper}>
        +<input type="file" accept="image/*" aria-label={l.uploadWallpaper} onchange={upload} />
      </label>
    </div>
    <div class="lab-row">
      <span>{l.behind}</span>
      <div class="lab-seg">
        <button class:is-active={backdrop === 'none'} onclick={() => setBackdrop('none')}>{l.desktop}</button>
        <button class:is-active={backdrop === 'light'} onclick={() => setBackdrop('light')}>{l.lightWindow}</button>
        <button class:is-active={backdrop === 'dark'} onclick={() => setBackdrop('dark')}>{l.darkWindow}</button>
      </div>
    </div>
    {#if backdrop !== 'none'}<p class="lab-note">{l.overlapHint}</p>{/if}
    {#if engine.themeId === 'glass'}<p class="lab-note">{l.glassNote}</p>{/if}

    <h3>{l.events}</h3>
    <div class="lab-events">
      {#each EVENTS as ev (ev.key)}
        <button class="lab-btn" onclick={ev.run}>{ev.label()}<kbd>{ev.key.toUpperCase()}</kbd></button>
      {/each}
      <button class="lab-btn lab-btn--wide" onclick={() => ctx?.controller.toggle()}>{l.togglePanel}<kbd>␣</kbd></button>
    </div>
    <p class="lab-note">{l.hint}</p>

    <h3>{l.motion}</h3>
    <div class="lab-row">
      <span>{l.tempo}</span>
      <div class="lab-seg">
        {#each TEMPOS as t (t)}
          <button class:is-active={tempo === t} onclick={() => setTempo(t)}>{String(t).replace('.', i18n.locale === 'de' ? ',' : '.')}×</button>
        {/each}
      </div>
    </div>
    <label class="lab-check">{l.reduced} <input type="checkbox" checked={reduced} onchange={(e) => setReduced(e.currentTarget.checked)} /></label>
    <label class="lab-check">
      {i18n.m.settings.island.hoverPreview}
      <input type="checkbox" checked={settings.island.hoverPreview} onchange={(e) => void backend.settings.patch({ island: { hoverPreview: e.currentTarget.checked } })} />
    </label>

    <div class="lab-foot">
      <a href="settings.html" target="rice-settings">{l.openSettings} →</a>
      <a href="gallery.html">{l.gallery} →</a>
    </div>
    <p class="lab-note">{l.shortcuts}</p>
  </div>
</aside>

<div class="lab-toast" class:is-visible={Boolean(message)} role="status" aria-live="polite">{message}</div>
