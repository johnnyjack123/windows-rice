import { describe, expect, it } from 'vitest';
import { flushSync, tick } from 'svelte';
import { createBackend } from '$backend/index';
import { ISLAND_STORES } from '../entries/boot';
import { ThemeEngine } from './engine.svelte';

const settle = async (ms = 40): Promise<void> => {
  await tick();
  await new Promise((r) => setTimeout(r, ms));
  flushSync();
};

describe('theme engine', () => {
  it('re-applies tokens only when theme-related settings change', async () => {
    localStorage.clear();
    const backend = createBackend();
    await backend.init(ISLAND_STORES);
    const engine = new ThemeEngine(backend);
    const stop = engine.connect();
    await settle();
    const before = engine.version;

    // Switching panel tabs persists the last section; that must not restyle the document.
    await backend.settings.patch({ panel: { lastSection: 'calendar' } });
    await backend.settings.patch({ island: { scrollVolume: false } });
    await settle(450);
    expect(engine.version).toBe(before);

    await backend.settings.patch({ theme: { overrides: { obsidian: { speed: 1.5 } } } });
    await settle(450);
    expect(engine.version).toBeGreaterThan(before);
    stop();
  });
});
