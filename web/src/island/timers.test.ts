import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Settings, SettingsPatch } from '$backend/contract';
import { applyPatch, DEFAULT_SETTINGS } from '$backend/settings';
import { TimersStore } from './timers.svelte';

function createStore(): { store: TimersStore; settings: () => Settings } {
  let current: Settings = structuredClone(DEFAULT_SETTINGS);
  const settings = (): Settings => current;
  const patch = (p: SettingsPatch): Promise<void> => {
    current = applyPatch(current, p);
    return Promise.resolve();
  };
  return { store: new TimersStore(settings, patch), settings };
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2026, 9, 14, 9, 0, 0));
});

afterEach(() => {
  vi.useRealTimers();
});

describe('timers', () => {
  it('runs several timers in parallel, soonest first, with unique ids', () => {
    const { store } = createStore();
    store.start(5 * 60_000, 'Tea');
    store.start(60_000, 'Eggs');
    store.start(10 * 60_000, '');
    expect(new Set(store.timers.map((t) => t.id)).size).toBe(3);
    expect(store.active.map((t) => t.label)).toEqual(['Eggs', 'Tea', '']);
    expect(store.activeCount).toBe(3);

    vi.advanceTimersByTime(30_000);
    const tea = store.timers.find((t) => t.label === 'Tea')!;
    store.pause(tea.id);
    vi.advanceTimersByTime(60_000);
    expect(store.remainingOf(store.timers.find((t) => t.id === tea.id)!)).toBe(4.5 * 60_000);
    store.resume(tea.id);
    store.addMinute(tea.id);
    expect(store.remainingOf(store.timers.find((t) => t.id === tea.id)!)).toBe(5.5 * 60_000);
  });

  it('rings timers that finish together one after another', () => {
    const { store } = createStore();
    store.start(60_000, 'A');
    store.start(60_000, 'B');
    vi.advanceTimersByTime(60_100);
    expect(store.ringing?.label).toBe('A');
    expect(store.active).toEqual([]);

    store.dismiss();
    expect(store.ringing?.label).toBe('B');
    store.extendRinging();
    expect(store.ringing).toBeNull();
    expect(store.active.map((t) => [t.label, t.duration])).toEqual([['B', 60_000]]);
    expect(store.timers).toHaveLength(1);
  });

  it('queues an alarm that rings while a timer is ringing', () => {
    const { store } = createStore();
    store.start(1000, 'Pasta');
    vi.advanceTimersByTime(1100);
    store.ringNow({ id: 'wake', time: '09:00', label: 'Wake up', days: [], enabled: true });
    expect(store.ringing?.kind).toBe('timer');
    store.dismiss();
    expect(store.ringing?.kind).toBe('alarm');
  });

  it('never shows a negative or stale stopwatch after an idle stretch', () => {
    const { store } = createStore();
    vi.advanceTimersByTime(60 * 60_000);
    store.toggleStopwatch();
    expect(store.stopwatchElapsed).toBe(0);
    vi.advanceTimersByTime(1500);
    store.lap();
    vi.advanceTimersByTime(500);
    store.toggleStopwatch();
    expect(store.stopwatch.laps).toEqual([1500]);
    expect(store.stopwatchElapsed).toBe(2000);
    store.resetStopwatch();
    expect(store.stopwatchElapsed).toBe(0);
  });

  it('rings every alarm due at the same time, snoozes and disables one-off alarms', async () => {
    const { store, settings } = createStore();
    await store.addAlarm('09:01', 'Stand-up', []);
    await store.addAlarm('09:01', 'Daily', [0, 1, 2, 3, 4]);
    expect(store.nextAlarm?.at.getHours()).toBe(9);

    vi.advanceTimersByTime(61_000);
    expect(store.ringing?.label).toBe('Stand-up');
    store.snooze();
    expect(store.ringing?.label).toBe('Daily');
    store.dismiss();
    expect(store.ringing).toBeNull();

    vi.advanceTimersByTime(settings().timers.snoozeMinutes * 60_000 + 1000);
    expect(store.ringing?.label).toBe('Stand-up');
    store.dismiss();
    const [once, daily] = settings().timers.alarms;
    expect(once?.enabled).toBe(false);
    expect(daily?.enabled).toBe(true);
  });
});
