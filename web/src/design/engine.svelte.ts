/**
 * Applies the active theme, the user's editor overrides, user themes, the
 * language, wallpaper-derived tokens and motion settings to the document.
 * One engine per window; it reacts to settings changes from any window.
 */
import { getContext, setContext, untrack } from 'svelte';
import type { Scheme, Settings, ThemeOverrides, WallpaperInfo } from '$backend/contract';
import type { Backend } from '$backend/index';
import { i18n } from '$i18n/i18n.svelte';
import { formatSpring, motionSettings, parseSpring, springToLinear } from '$motion/spring';
import { adjustEasing, adjustSpring, overridesStylesheet, SPRING_TOKENS } from './editor';
import { BUILTIN_THEMES, type ThemeMeta } from './themes/registry';
import { extractColor } from './color';
import { prepareGlass, type GlassLayers } from './wallpaper';

const DURATIONS = ['--dur-instant', '--dur-fast', '--dur-medium', '--dur-slow', '--stagger', '--enter-delay-grow', '--enter-delay-shrink'];
const EASES = ['--ease-standard', '--ease-enter', '--ease-exit'];
const GENERATED = ['--ease-spring-content', '--dur-spring-content', '--ease-spring-control', '--dur-spring-control'];

/** Wallpaper geometry the glass material needs to align with the screen. */
export interface GlassGeometry {
  monitorWidth: number;
  monitorHeight: number;
  windowX: number;
  windowY: number;
}

/** The theme engine of one window. */
export class ThemeEngine {
  /** Active theme id after resolution. */
  themeId = $state('obsidian');
  /** Active colour scheme after resolution. */
  scheme = $state<Scheme>('dark');
  /** Increments whenever tokens were re-applied; readers re-measure. */
  version = $state(0);
  /** Lab override for reduced motion (null = follow Windows). */
  reducedOverride = $state<boolean | null>(null);
  glass = $state<GlassGeometry>({ monitorWidth: 1920, monitorHeight: 1080, windowX: 0, windowY: 0 });
  /** Pre-rendered wallpaper layers for glass materials (blob URLs), once ready. */
  glassLayers = $state<GlassLayers | null>(null);

  /** Built-in plus user themes. */
  readonly themes: ThemeMeta[] = $derived.by(() => [
    ...BUILTIN_THEMES,
    ...this.backend.themes.list.map((t) => ({
      id: t.id,
      name: t.manifest.name,
      author: t.manifest.author,
      schemes: t.manifest.schemes.length ? t.manifest.schemes : (['dark'] as Scheme[]),
      builtIn: false,
      css: t.css,
    })),
  ]);

  private readonly root = document.documentElement;
  private readonly userStyle = this.styleElement('rice-user-themes');
  private readonly overrideStyle = this.styleElement('rice-overrides');
  private glassUrls: string[] = [];
  private wallpaperToken = 0;

  /** @param backend - Backend of this window. */
  constructor(private readonly backend: Backend) {}

  /**
   * Starts reacting to settings, appearance, media and wallpaper.
   *
   * @returns Cleanup function.
   */
  connect(): () => void {
    return $effect.root(() => {
      // Settings are one object that is replaced on every patch (e.g. the last
      // panel tab). Re-applying a theme restyles the whole document, so only
      // the parts that matter are tracked, as strings that compare by value.
      const themeJson = $derived(JSON.stringify(this.backend.settings.value.theme));
      const locale = $derived(this.backend.settings.value.locale);
      const dark = $derived(this.backend.system.appearance.dark);
      const systemReduced = $derived(this.backend.system.appearance.reducedMotion);

      $effect(() => {
        this.userStyle.textContent = this.backend.themes.list.map((t) => t.css).join('\n');
      });

      $effect(() => {
        const theme = JSON.parse(themeJson) as Settings['theme'];
        const meta = this.themes.find((t) => t.id === theme.id) ?? BUILTIN_THEMES[0]!;
        const wanted: Scheme = theme.scheme === 'auto' ? (dark ? 'dark' : 'light') : theme.scheme;
        const scheme = meta.schemes.includes(wanted) ? wanted : meta.schemes[0]!;
        const reduced = this.reducedOverride ?? systemReduced;

        const css = overridesStylesheet(theme.overrides);
        if (this.overrideStyle.textContent !== css) {
          this.overrideStyle.textContent = css;
        }
        this.root.dataset.theme = meta.id;
        this.root.dataset.scheme = scheme;
        this.root.dataset.motion = reduced ? 'reduced' : 'full';
        motionSettings.reduced = reduced;
        this.themeId = meta.id;
        this.scheme = scheme;
        const overrides = theme.overrides[meta.id] ?? {};
        untrack(() => this.refreshMotion(overrides));
      });

      $effect(() => {
        i18n.apply(locale);
      });

      $effect(() => {
        this.root.style.setProperty('--media-color', this.backend.media.color);
      });

      $effect(() => {
        const wallpaper = this.backend.system.wallpaper;
        if (wallpaper) {
          untrack(() => void this.applyWallpaper(wallpaper));
        }
      });
    });
  }

  /** Overrides of the active theme. */
  get overrides(): ThemeOverrides {
    return this.backend.settings.value.theme.overrides[this.themeId] ?? {};
  }

  /**
   * Lab slow motion: rescales every duration and spring.
   *
   * @param scale - 1 = real time, 0.2 = five times slower.
   */
  setTimeScale(scale: number): void {
    motionSettings.timeScale = scale;
    this.refreshMotion(this.overrides);
  }

  /**
   * Re-derives motion tokens from the theme, the user's speed/bounce/easing
   * and the lab time scale, then signals readers via {@link version}.
   *
   * @param o - Overrides of the active theme.
   */
  refreshMotion(o: ThemeOverrides): void {
    const style = this.root.style;
    for (const name of [...DURATIONS, ...EASES, ...SPRING_TOKENS, ...GENERATED]) {
      style.removeProperty(name);
    }
    const cs = getComputedStyle(this.root);
    const slow = 1 / motionSettings.timeScale;
    const speed = o.speed ?? 1;

    for (const name of DURATIONS) {
      const ms = parseFloat(cs.getPropertyValue(name));
      if (Number.isFinite(ms) && (slow !== 1 || speed !== 1)) {
        style.setProperty(name, `${(ms * slow) / speed}ms`);
      }
    }
    if (o.easing !== undefined) {
      for (const name of EASES) {
        style.setProperty(name, adjustEasing(cs.getPropertyValue(name), o.easing));
      }
    }

    const base = parseSpring(cs.getPropertyValue('--spring-island'), undefined, false);
    for (const name of SPRING_TOKENS) {
      if (o.speed !== undefined || o.bounce !== undefined) {
        const raw = parseSpring(cs.getPropertyValue(name), undefined, false);
        style.setProperty(name, formatSpring(adjustSpring(raw, name, base, o)));
      }
    }
    const after = getComputedStyle(this.root);
    for (const name of ['content', 'control'] as const) {
      const { easing, ms } = springToLinear(parseSpring(after.getPropertyValue(`--spring-${name}`)));
      style.setProperty(`--ease-spring-${name}`, easing);
      style.setProperty(`--dur-spring-${name}`, `${Math.round(ms * slow)}ms`);
    }
    this.version += 1;
  }

  private async applyWallpaper(info: WallpaperInfo): Promise<void> {
    const token = ++this.wallpaperToken;
    this.root.style.setProperty('--wallpaper-src', info.url ? `url("${info.url}")` : 'none');
    this.glass = {
      monitorWidth: info.monitor.width,
      monitorHeight: info.monitor.height,
      windowX: info.window.x,
      windowY: info.window.y,
    };
    const seed = info.url ? await extractColor(info.url, info.color) : info.color;
    if (token !== this.wallpaperToken) {
      return;
    }
    this.root.style.setProperty('--wallpaper-seed', seed);
    try {
      const layers = await prepareGlass(info);
      if (token !== this.wallpaperToken || !layers) {
        return;
      }
      this.root.style.setProperty('--wallpaper-blurred', `url("${layers.blurred}")`);
      this.root.style.setProperty('--wallpaper-rim', `url("${layers.rim}")`);
      this.glassLayers = layers;
      const stale = this.glassUrls;
      this.glassUrls = [layers.blurred, layers.rim];
      // Give image loaders (CSS, WebGL) a moment to switch before revoking.
      setTimeout(() => {
        for (const url of stale) {
          URL.revokeObjectURL(url);
        }
      }, 2000);
    } catch (error) {
      console.warn('[theme] glass layers unavailable', error);
    }
  }

  private styleElement(id: string): HTMLStyleElement {
    const existing = document.getElementById(id);
    if (existing instanceof HTMLStyleElement) {
      return existing;
    }
    const el = document.createElement('style');
    el.id = id;
    document.head.append(el);
    return el;
  }
}

const KEY = Symbol('theme-engine');

/**
 * Provides the engine to child components.
 *
 * @param engine - Engine of this window.
 */
export function provideEngine(engine: ThemeEngine): void {
  setContext(KEY, engine);
}

/**
 * Reads the engine provided by {@link provideEngine}.
 *
 * @returns The window's theme engine.
 */
export function useEngine(): ThemeEngine {
  return getContext<ThemeEngine>(KEY);
}
