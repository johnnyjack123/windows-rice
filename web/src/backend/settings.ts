import type { DeepPatch, Settings, StoredSettings } from './contract';

/**
 * Defaults for every setting. The backend persists only what differs, so
 * this object is the single source of truth for defaults.
 */
export const DEFAULT_SETTINGS: Settings = {
  version: 1,
  locale: 'auto',
  theme: {
    id: 'obsidian',
    scheme: 'auto',
    overrides: {},
  },
  island: {
    monitor: 'primary',
    offsetTop: 10,
    autoHide: true,
    peek: true,
    hideInFullscreen: true,
    hoverPreview: true,
    scrollVolume: true,
    hotkey: 'Super+Alt+Space',
  },
  panel: {
    sections: ['notifications', 'calendar', 'tasks', 'timers', 'system'],
    hiddenSections: [],
    tilesLarge: ['wifi', 'bluetooth', 'dnd', 'night'],
    tilesSmall: ['airplane', 'saver', 'mic', 'hotspot'],
    lastSection: 'notifications',
  },
  notifications: {
    dnd: false,
    hideWindowsBanners: true,
  },
  calendar: {
    reminderLead: 10,
  },
  timers: {
    sound: 'chime',
    soundVolume: 0.7,
    snoozeMinutes: 9,
    presets: [1, 3, 5, 10, 15, 25],
    alarms: [],
    pomodoro: {
      focusMinutes: 25,
      shortBreakMinutes: 5,
      longBreakMinutes: 15,
      sessionsUntilLongBreak: 4,
      autoStartBreaks: true,
      autoStartFocus: false,
      breakMode: 'notify',
    },
  },
};

type Json = Record<string, unknown>;

function isPlainObject(value: unknown): value is Json {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * Applies a patch with the contract's semantics: objects merge key by key,
 * arrays and primitives replace, `null` deletes the key. Inputs are not
 * mutated. The Rust backend implements the same rules for `config_set`.
 *
 * @param base - Object to patch.
 * @param patch - Patch to apply.
 * @returns A new, patched object.
 */
export function applyPatch<T>(base: T, patch: DeepPatch<T> | undefined | null): T {
  if (!isPlainObject(patch)) {
    return base;
  }
  const out: Json = isPlainObject(base) ? { ...base } : {};
  for (const [key, value] of Object.entries(patch)) {
    if (value === null) {
      delete out[key];
    } else if (isPlainObject(value) && isPlainObject(out[key])) {
      out[key] = applyPatch(out[key], value as DeepPatch<unknown>);
    } else if (isPlainObject(value)) {
      out[key] = applyPatch({}, value as DeepPatch<unknown>);
    } else {
      out[key] = value;
    }
  }
  return out as T;
}

/**
 * Merges stored settings over the defaults and drops values whose type does
 * not match the default (corrupt or outdated config never breaks the UI).
 *
 * @param stored - Whatever the backend persisted, possibly partial or empty.
 * @returns Complete, type-safe settings.
 */
export function resolveSettings(stored: StoredSettings | null | undefined): Settings {
  return sanitize(DEFAULT_SETTINGS, applyPatch(DEFAULT_SETTINGS, stored ?? {}), '') as Settings;
}

/** Settings whose valid values have more than one type. */
const MIXED_TYPES: Record<string, (value: unknown) => boolean> = {
  'island.monitor': (v) => v === 'primary' || (typeof v === 'number' && Number.isInteger(v) && v >= 0),
  'timers.pomodoro.breakMode': (v) => v === 'notify' || v === 'overlay',
};

function sanitize(defaults: unknown, value: unknown, path: string): unknown {
  const mixed = MIXED_TYPES[path];
  if (mixed) {
    return mixed(value) ? value : defaults;
  }
  if (isPlainObject(defaults)) {
    if (!isPlainObject(value)) {
      return defaults;
    }
    // Records with free keys (theme overrides) have an empty default.
    if (Object.keys(defaults).length === 0) {
      return value;
    }
    const out: Json = {};
    for (const key of Object.keys(defaults)) {
      out[key] = sanitize(defaults[key], value[key], path ? `${path}.${key}` : key);
    }
    return out;
  }
  if (Array.isArray(defaults)) {
    return Array.isArray(value) ? value : defaults;
  }
  return typeof value === typeof defaults ? value : defaults;
}
