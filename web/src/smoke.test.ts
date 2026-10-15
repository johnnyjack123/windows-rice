/**
 * Headless smoke test: mounts the real windows against the simulated backend
 * and drives every feature. It catches runtime errors, not visual problems —
 * appearance is reviewed by hand in the lab and the gallery.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import { createBackend } from '$backend/index';
import { ThemeEngine } from '$design/engine.svelte';
import { CONTROLS, PILL } from '$design/editor';
import type { IslandContext } from '$island/context.svelte';
import IslandApp from '$island/IslandApp.svelte';
import BreakApp from '$break/BreakApp.svelte';
import LabApp from '$lab/LabApp.svelte';
import SettingsApp from '$settings/SettingsApp.svelte';
import GalleryApp from './gallery/GalleryApp.svelte';
import { ISLAND_STORES } from './entries/boot';

const settle = async (ms = 50): Promise<void> => {
  await tick();
  await new Promise((r) => setTimeout(r, ms));
  flushSync();
};

let errors: unknown[] = [];

beforeEach(() => {
  errors = [];
  vi.spyOn(console, 'error').mockImplementation((...args) => errors.push(args));
  vi.spyOn(console, 'warn').mockImplementation((...args) => errors.push(args));
  window.addEventListener('error', (e) => errors.push(e.error ?? e.message));
  localStorage.clear();
});

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = '';
});

async function boot() {
  const backend = createBackend();
  await backend.init(ISLAND_STORES);
  const engine = new ThemeEngine(backend);
  engine.connect();
  return { backend, engine };
}

describe('island', () => {
  it('runs every activity, section and page without errors', async () => {
    const { backend, engine } = await boot();
    const mock = backend.mock!;
    let ctx: IslandContext | null = null;
    const app = mount(IslandApp, { target: document.body, props: { backend, engine, onready: (c) => (ctx = c) } });
    await settle();
    expect(ctx).not.toBeNull();
    const c = ctx!;

    // Activities
    mock.debug.pushNotification();
    mock.debug.changeVolume(-0.2);
    mock.debug.changeBrightness(0.1);
    mock.debug.toggleCharging();
    mock.debug.addSoonEvent();
    c.timers.start(60_000, 'Tee');
    c.timers.toggleStopwatch();
    c.timers.lap();
    mock.debug.nextTrack();
    mock.debug.togglePlay();
    await settle(120);
    expect(c.registry.active.length).toBeGreaterThan(0);
    c.controller.pointerEnter();
    c.controller.pointerLeave();

    // Auto-hide: a window overlaps, the island hides, the panel overrides it.
    for (const id of ['notification', 'osd', 'charging', 'event']) {
      c.registry.hide(id);
    }
    mock.debug.setOccluded(true);
    await settle();
    expect(c.controller.hidden).toBe(true);
    c.controller.expand('notifications');
    await settle();
    expect(c.controller.hidden).toBe(false);

    // Every section and drill-in page
    for (const section of ['notifications', 'calendar', 'tasks', 'timers', 'system']) {
      c.panel.section = section;
      await settle();
    }
    // Drill-in pages; their footer hands off to the Windows settings.
    const opened: string[] = [];
    const offShell = mock.onShellOpen.on((target) => opened.push(target));
    for (const page of ['wifi', 'bluetooth'] as const) {
      c.panel.page = page;
      await settle();
      document.querySelector<HTMLButtonElement>(`[data-page="${page}"].is-current .detail-foot .btn`)?.click();
      await settle();
    }
    offShell();
    expect(opened).toEqual(['ms-settings:network-wifi', 'ms-settings:bluetooth']);
    c.panel.page = 'home';
    await settle();
    document.querySelectorAll<HTMLButtonElement>('.tile__toggle, .tile--small, .seg__btn, .day, .chip, .check').forEach((b) => b.click());
    await settle();

    // Second pill: hovering it opens its quick actions, leaving closes them.
    mock.debug.setOccluded(false);
    c.controller.collapse();
    await settle(700);
    expect(c.controller.bubbleId).not.toBeNull();
    c.controller.pointerEnter('bubble');
    await settle(520);
    expect(c.controller.bubbleExpanded).toBe(true);
    document.querySelectorAll<HTMLButtonElement>('.bubble-content .qa-btn').forEach((b) => b.click());
    c.controller.pointerLeave('bubble');
    await settle(320);
    expect(c.controller.bubbleExpanded).toBe(false);

    // Pomodoro tab: start, settings face, steppers and switches, stop.
    c.controller.expand('timers');
    await settle();
    [...document.querySelectorAll<HTMLButtonElement>('.timers-mode .seg__btn')].at(-1)?.click();
    await settle();
    document.querySelector<HTMLButtonElement>('.pomo__buttons .btn--filled')?.click();
    await settle();
    expect(c.pomodoro.state.status).toBe('running');
    expect(c.registry.has('pomodoro')).toBe(true);
    document.querySelector<HTMLButtonElement>('.pomo__settings')?.click();
    await settle();
    document.querySelectorAll<HTMLButtonElement>('.pomo-settings .stepper .icon-btn, .pomo-settings .switch, .pomo-settings .seg__btn').forEach((b) => b.click());
    await settle();
    c.pomodoro.skip();
    await settle();
    c.pomodoro.stop();
    await settle();
    expect(c.registry.has('pomodoro')).toBe(false);

    // Alarm rings and is dismissed
    c.timers.ringNow({ id: 'x', time: '07:00', label: 'Test', days: [], enabled: true });
    await settle();
    expect(c.registry.has('alarm')).toBe(true);
    c.timers.snooze();
    c.controller.collapse();
    await settle();

    unmount(app);
    expect(errors).toEqual([]);
  }, 20_000);

  it('applies every theme, scheme and editor override', async () => {
    const { backend, engine } = await boot();
    const app = mount(IslandApp, { target: document.body, props: { backend, engine } });
    await settle();
    for (const theme of engine.themes) {
      for (const scheme of theme.schemes) {
        await backend.settings.patch({ theme: { id: theme.id, scheme } });
        await settle(10);
        expect(document.documentElement.dataset.theme).toBe(theme.id);
        expect(document.documentElement.dataset.scheme).toBe(scheme);
      }
    }
    const overrides = Object.fromEntries(CONTROLS.filter((d) => d.kind === 'range').map((d) => [d.key, d.max === 22 ? PILL : d.max]));
    await backend.settings.patch({ theme: { overrides: { paper: { ...overrides, liquid: true, iconStyle: 'Sharp', fontUi: 'JetBrains Mono', colors: { light: { accent: '#123456' } } } } } });
    await settle();
    expect(document.getElementById('rice-overrides')?.textContent).toContain('--accent: #123456');
    engine.setTimeScale(0.2);
    engine.reducedOverride = true;
    await settle();
    unmount(app);
    expect(errors).toEqual([]);
  }, 20_000);
});

describe('other windows', () => {
  it('renders every settings page and the lab and gallery', async () => {
    const { backend, engine } = await boot();
    const settings = mount(SettingsApp, { target: document.body, props: { backend, engine } });
    for (const page of ['appearance', 'island', 'panel', 'calendar', 'timers', 'language', 'about']) {
      location.hash = page;
      window.dispatchEvent(new HashChangeEvent('hashchange'));
      await settle();
      expect(document.querySelector('.settings-title')).not.toBeNull();
    }
    unmount(settings);

    const lab = mount(LabApp, { target: document.body, props: { backend, engine } });
    await settle();
    document.querySelectorAll<HTMLButtonElement>('.lab-btn').forEach((b) => b.click());
    await settle(100);
    unmount(lab);

    const gallery = mount(GalleryApp, { target: document.body, props: { backend, engine } });
    await settle();
    expect(document.querySelectorAll('.g-col').length).toBe(7);
    unmount(gallery);

    // Break window: shows the countdown, then the "back to work" face.
    const now = Date.now();
    await backend.transport.invoke('break_show', { endsAt: now + 300_000, remaining: 300_000, duration: 300_000, kind: 'short', round: 2, rounds: 4 });
    const breakApp = mount(BreakApp, { target: document.body, props: { backend, engine } });
    await settle(60);
    expect(document.querySelector('.break__time')?.textContent).toMatch(/\d/);
    await backend.transport.invoke('break_show', { endsAt: now - 1, remaining: 0, duration: 300_000, kind: 'long', round: 4, rounds: 4 });
    await settle(60);
    expect(document.querySelector('.break__back')).not.toBeNull();
    unmount(breakApp);
    await backend.transport.invoke('break_hide');
    expect(errors).toEqual([]);
  }, 20_000);
});
