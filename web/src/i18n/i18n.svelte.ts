import { de, type Messages } from './messages/de';
import { en } from './messages/en';

/** Supported UI languages. */
export type Locale = 'de' | 'en';

/** Language preference as stored in the settings. */
export type LocalePreference = Locale | 'auto';

const CATALOGUES: Record<Locale, Messages> = { de, en };

/** Every supported locale with its catalogue, e.g. for language pickers. */
export const LOCALES: readonly Locale[] = ['de', 'en'];

/**
 * Resolves "auto" against the system language (WebView2 reports the
 * Windows display language).
 *
 * @param preference - Stored preference.
 * @returns A concrete locale.
 */
export function resolveLocale(preference: LocalePreference): Locale {
  if (preference !== 'auto') {
    return preference;
  }
  const system = typeof navigator === 'undefined' ? 'de' : navigator.language.toLowerCase();
  return system.startsWith('de') ? 'de' : 'en';
}

/**
 * Reactive i18n state. Reading `i18n.m` inside a component or effect
 * subscribes to language changes.
 */
class I18n {
  /** Active language. */
  locale = $state<Locale>(resolveLocale('auto'));

  /** Messages of the active language. */
  get m(): Messages {
    return CATALOGUES[this.locale];
  }

  /** BCP 47 tag used for Intl formatting. */
  get tag(): string {
    return this.m.meta.tag;
  }

  /**
   * Applies a stored preference and updates `<html lang>`.
   *
   * @param preference - "auto" or a locale.
   */
  apply(preference: LocalePreference): void {
    this.locale = resolveLocale(preference);
    if (typeof document !== 'undefined') {
      document.documentElement.lang = this.locale;
    }
  }

  /**
   * Catalogue of a specific locale (used by the language picker).
   *
   * @param locale - Locale to read.
   * @returns Its messages.
   */
  catalogue(locale: Locale): Messages {
    return CATALOGUES[locale];
  }
}

/** The app-wide i18n instance. */
export const i18n = new I18n();

/**
 * Locale-aware formatting helpers. All of them follow {@link i18n.locale}.
 */
export const fmt = {
  /**
   * @param date - Point in time.
   * @returns Time like "14:32".
   */
  time(date: Date): string {
    return date.toLocaleTimeString(i18n.tag, { hour: '2-digit', minute: '2-digit', hour12: false });
  },

  /**
   * @param date - Day.
   * @returns Long date like "Dienstag, 13. Oktober".
   */
  longDate(date: Date): string {
    return date.toLocaleDateString(i18n.tag, { weekday: 'long', day: 'numeric', month: 'long' });
  },

  /**
   * @param date - Day.
   * @returns Short weekday like "Di" / "Tue".
   */
  weekday(date: Date): string {
    return date.toLocaleDateString(i18n.tag, { weekday: 'short' }).replace('.', '');
  },

  /**
   * @param seconds - Duration in seconds.
   * @returns "3:41" or "1:02:03".
   */
  duration(seconds: number): string {
    const s = Math.max(0, Math.round(seconds));
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const rest = String(s % 60).padStart(2, '0');
    return h ? `${h}:${String(m).padStart(2, '0')}:${rest}` : `${m}:${rest}`;
  },

  /**
   * @param ms - Duration in milliseconds.
   * @returns Stopwatch format "01:23,45" (centiseconds).
   */
  stopwatch(ms: number): string {
    const total = Math.max(0, Math.floor(ms / 10));
    const cs = String(total % 100).padStart(2, '0');
    const s = Math.floor(total / 100);
    const sep = i18n.locale === 'de' ? ',' : '.';
    const h = Math.floor(s / 3600);
    const mm = String(Math.floor((s % 3600) / 60)).padStart(2, '0');
    const ss = String(s % 60).padStart(2, '0');
    return h ? `${h}:${mm}:${ss}${sep}${cs}` : `${mm}:${ss}${sep}${cs}`;
  },

  /**
   * @param value - Number.
   * @param digits - Fraction digits.
   * @returns Localised number, e.g. "11,4".
   */
  num(value: number, digits = 1): string {
    return value.toLocaleString(i18n.tag, { minimumFractionDigits: digits, maximumFractionDigits: digits });
  },

  /**
   * @param bytes - Size in bytes.
   * @returns "1,2 GB", "420 MB".
   */
  bytes(bytes: number): string {
    const mb = bytes / (1024 * 1024);
    return mb >= 1000 ? `${this.num(mb / 1024)} GB` : `${Math.round(mb)} MB`;
  },

  /**
   * @param bytesPerSecond - Throughput.
   * @returns "2,4 MB/s", "120 KB/s".
   */
  rate(bytesPerSecond: number): string {
    const kb = bytesPerSecond / 1024;
    return kb >= 1000 ? `${this.num(kb / 1024)} MB/s` : `${Math.round(kb)} KB/s`;
  },

  /**
   * @param date - Point in the past.
   * @param now - Reference time.
   * @returns Relative label, e.g. "vor 2 Min".
   */
  ago(date: Date, now = new Date()): string {
    return i18n.m.format.ago((now.getTime() - date.getTime()) / 60000);
  },

  /**
   * @param date - Point in the future.
   * @param now - Reference time.
   * @returns Relative label, e.g. "in 25 Min".
   */
  until(date: Date, now = new Date()): string {
    return i18n.m.format.until((date.getTime() - now.getTime()) / 60000);
  },
};
