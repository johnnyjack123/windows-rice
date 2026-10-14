import { describe, expect, it } from 'vitest';
import { de } from './messages/de';
import { en } from './messages/en';
import { fmt, i18n } from './i18n.svelte';

/** Flattens a catalogue into "path → kind/arity" for shape comparison. */
function shape(obj: unknown, path = ''): Record<string, string> {
  const out: Record<string, string> = {};
  if (typeof obj === 'function') {
    out[path] = `fn/${obj.length}`;
  } else if (Array.isArray(obj)) {
    out[path] = `array/${obj.length}`;
  } else if (obj && typeof obj === 'object') {
    for (const [k, v] of Object.entries(obj)) {
      Object.assign(out, shape(v, path ? `${path}.${k}` : k));
    }
  } else {
    out[path] = typeof obj;
  }
  return out;
}

describe('i18n', () => {
  it('has the same keys, kinds and arities in every language', () => {
    expect(shape(en)).toEqual(shape(de));
  });

  it('has no empty strings', () => {
    for (const catalogue of [de, en]) {
      const empty = Object.entries(shape(catalogue)).filter(([path, kind]) => kind === 'string'
        && path.split('.').reduce<unknown>((o, k) => (o as Record<string, unknown>)[k], catalogue) === '');
      expect(empty).toEqual([]);
    }
  });

  it('formats per locale', () => {
    i18n.locale = 'de';
    expect(fmt.num(11.4)).toBe('11,4');
    expect(fmt.duration(221)).toBe('3:41');
    expect(fmt.stopwatch(83_450)).toBe('01:23,45');
    expect(i18n.m.format.ago(14)).toBe('vor 14 Min');
    i18n.locale = 'en';
    expect(fmt.num(11.4)).toBe('11.4');
    expect(fmt.stopwatch(83_450)).toBe('01:23.45');
    expect(i18n.m.format.until(90)).toBe('in 1 h 30 min');
    i18n.locale = 'de';
  });
});
