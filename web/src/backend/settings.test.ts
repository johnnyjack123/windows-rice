import { describe, expect, it } from 'vitest';
import { applyPatch, DEFAULT_SETTINGS, resolveSettings } from './settings';
import { mergePatches } from './stores/core.svelte';
import type { StoredSettings } from './contract';

describe('settings', () => {
  it('merges objects, replaces arrays and deletes on null', () => {
    const base: StoredSettings = { panel: { sections: ['a', 'b'], lastSection: 'a' }, theme: { overrides: { x: { speed: 2 } } } };
    const out = applyPatch(base, { panel: { sections: ['c'] }, theme: { overrides: { x: null } } });
    expect(out.panel?.sections).toEqual(['c']);
    expect(out.panel?.lastSection).toBe('a');
    expect(out.theme?.overrides).toEqual({});
    expect(base.panel?.sections).toEqual(['a', 'b']);
  });

  it('resolves an empty store to the defaults', () => {
    expect(resolveSettings({})).toEqual(DEFAULT_SETTINGS);
    expect(resolveSettings(null)).toEqual(DEFAULT_SETTINGS);
  });

  it('drops values of the wrong type instead of breaking', () => {
    const bad = { island: { autoHide: 'yes', monitor: -3, offsetTop: 12 }, panel: { sections: 'notifications' } } as unknown as StoredSettings;
    const s = resolveSettings(bad);
    expect(s.island.autoHide).toBe(true);
    expect(s.island.monitor).toBe('primary');
    expect(s.island.offsetTop).toBe(12);
    expect(s.panel.sections).toEqual(DEFAULT_SETTINGS.panel.sections);
    expect(resolveSettings({ island: { monitor: 1 } }).island.monitor).toBe(1);
  });

  it('keeps free-form theme overrides', () => {
    const s = resolveSettings({ theme: { overrides: { paper: { bounce: 0.2 } } } });
    expect(s.theme.overrides.paper).toEqual({ bounce: 0.2 });
  });

  it('merges queued patches so applying once equals applying in order', () => {
    const a = { theme: { overrides: { x: { speed: 1.5 } } } };
    const b = { theme: { overrides: { x: { bounce: 0.1 } } }, locale: 'en' as const };
    type Loose = Record<string, unknown>;
    const merged = mergePatches(a, b) as Loose;
    const stepwise = applyPatch<Loose>(applyPatch<Loose>({}, a), b);
    expect(applyPatch<Loose>({}, merged)).toEqual(stepwise);
  });
});
