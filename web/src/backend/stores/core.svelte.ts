import type {
  AppInfo, Appearance, Settings, SettingsPatch, StoredSettings, UserTheme, UserThemeManifest, WallpaperInfo,
} from '../contract';
import type { Transport } from '../transport';
import { applyPatch, resolveSettings } from '../settings';

/** Persisted settings: stored values merged over the defaults. */
export class SettingsStore {
  /** Raw stored object as the backend keeps it. */
  stored = $state<StoredSettings>({});
  /** Complete settings, always valid. */
  readonly value: Settings = $derived(resolveSettings(this.stored));

  private pending: SettingsPatch | null = null;
  private flushTimer: ReturnType<typeof setTimeout> | undefined;

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    this.stored = await this.t.invoke('config_get');
    this.t.listen('config://changed', (stored) => {
      // Local changes that are not flushed yet stay on top of the echo.
      this.stored = this.pending ? applyPatch(stored, this.pending) : stored;
    });
  }

  /**
   * Applies a patch immediately and persists it shortly after. Rapid patches
   * (sliders) are merged into one backend write.
   *
   * @param patch - Deep patch; `null` deletes a key.
   */
  patch(patch: SettingsPatch): Promise<void> {
    this.stored = applyPatch(this.stored, patch);
    this.pending = mergePatches(this.pending, patch);
    clearTimeout(this.flushTimer);
    return new Promise((resolve) => {
      this.flushTimer = setTimeout(() => {
        const queued = this.pending;
        this.pending = null;
        if (queued) {
          void this.t.invoke('config_set', { patch: queued }).finally(resolve);
        } else {
          resolve();
        }
      }, 150);
    });
  }
}

/**
 * Merges two patches so applying the result equals applying both in order.
 *
 * @param a - Earlier patch (may be null).
 * @param b - Later patch.
 * @returns Combined patch.
 */
export function mergePatches(a: SettingsPatch | null, b: SettingsPatch): SettingsPatch {
  if (!a) {
    return b;
  }
  const out: Record<string, unknown> = { ...(a as Record<string, unknown>) };
  for (const [key, value] of Object.entries(b as Record<string, unknown>)) {
    const prev = out[key];
    const isObj = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
    out[key] = isObj(prev) && isObj(value) ? mergePatches(prev as SettingsPatch, value as SettingsPatch) : value;
  }
  return out as SettingsPatch;
}

/** Windows appearance, wallpaper, fonts and static app facts. */
export class SystemInfoStore {
  appearance = $state<Appearance>({ dark: true, accent: '#0078d4', reducedMotion: false, transparency: true });
  wallpaper = $state<WallpaperInfo | null>(null);
  fonts = $state<string[]>([]);
  info = $state<AppInfo | null>(null);

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    const [appearance, wallpaper, info] = await Promise.all([
      this.t.invoke('system_appearance'),
      this.t.invoke('system_wallpaper'),
      this.t.invoke('app_info'),
    ]);
    this.appearance = appearance;
    this.wallpaper = wallpaper;
    this.info = info;
    this.t.listen('system://appearance', (a) => {
      this.appearance = a;
    });
    this.t.listen('system://wallpaper', (w) => {
      this.wallpaper = w;
    });
  }

  /** Loads the installed font families once (settings only). */
  async loadFonts(): Promise<void> {
    if (!this.fonts.length) {
      this.fonts = await this.t.invoke('system_fonts');
    }
  }
}

/** User themes from the themes folder. */
export class UserThemesStore {
  list = $state<UserTheme[]>([]);

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    this.list = await this.t.invoke('theme_list_user');
    this.t.listen('theme://user-changed', (list) => {
      this.list = list;
    });
  }

  /**
   * Writes a theme folder (theme.toml + theme.css).
   *
   * @param id - Folder name, lowercase kebab case.
   * @param manifest - Manifest data.
   * @param css - Complete theme.css text.
   * @returns The stored theme.
   */
  save(id: string, manifest: UserThemeManifest, css: string): Promise<UserTheme> {
    return this.t.invoke('theme_save_user', { id, manifest, css });
  }

  /** @param id - Theme to delete. */
  remove(id: string): Promise<void> {
    return this.t.invoke('theme_delete_user', { id });
  }
}
