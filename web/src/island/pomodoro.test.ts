import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { PomodoroSettings, Settings, SettingsPatch } from '$backend/contract';
import type { Transport } from '$backend/transport';
import { applyPatch, DEFAULT_SETTINGS } from '$backend/settings';
import { nextPhase, POMODORO_SIGNAL, PomodoroStore } from './pomodoro.svelte';
import { TimersStore } from './timers.svelte';

const MIN = 60_000;

function setup(pomodoro: Partial<PomodoroSettings> = {}) {
  let current: Settings = applyPatch(structuredClone(DEFAULT_SETTINGS), { timers: { pomodoro } });
  const settings = (): Settings => current;
  const patch = (p: SettingsPatch): Promise<void> => {
    current = applyPatch(current, p);
    return Promise.resolve();
  };
  const calls: Array<[string, unknown]> = [];
  const listeners = new Map<string, (payload: unknown) => void>();
  const transport = {
    kind: 'mock',
    invoke: (command: string, args?: unknown) => {
      calls.push([command, args]);
      return Promise.resolve(undefined);
    },
    listen: (event: string, handler: (payload: unknown) => void) => {
      listeners.set(event, handler);
      return () => listeners.delete(event);
    },
  } as unknown as Transport;
  const timers = new TimersStore(settings, patch);
  const store = new PomodoroStore(settings, transport, timers);
  const commands = (): string[] => calls.map(([c]) => c);
  return { store, timers, calls, commands, emit: (event: string) => listeners.get(event)?.(null) };
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2026, 9, 15, 9, 0, 0));
});

afterEach(() => {
  vi.useRealTimers();
});

describe('pomodoro', () => {
  it('ends every cycle with a long break and starts over', () => {
    expect(nextPhase('focus', 0, 4)).toEqual({ phase: 'short', round: 1 });
    expect(nextPhase('short', 1, 4)).toEqual({ phase: 'focus', round: 1 });
    expect(nextPhase('focus', 3, 4)).toEqual({ phase: 'long', round: 4 });
    expect(nextPhase('long', 4, 4)).toEqual({ phase: 'focus', round: 0 });
  });

  it('starts breaks on its own and waits with a signal before the next focus', () => {
    const { store, timers } = setup({ focusMinutes: 25, shortBreakMinutes: 5, autoStartBreaks: true, autoStartFocus: false });
    store.start();
    expect(store.state).toMatchObject({ phase: 'focus', status: 'running', round: 0 });

    vi.advanceTimersByTime(25 * MIN + 300);
    expect(store.state).toMatchObject({ phase: 'short', status: 'running', round: 1 });
    expect(store.change?.phase).toBe('short');
    expect(timers.ringing).toBeNull();

    vi.advanceTimersByTime(5 * MIN + 300);
    expect(store.state).toMatchObject({ phase: 'focus', status: 'waiting', round: 1 });
    expect(timers.ringing).toMatchObject({ kind: 'pomodoro', id: POMODORO_SIGNAL, label: 'focus' });

    store.startNext();
    expect(timers.ringing).toBeNull();
    expect(store.state).toMatchObject({ phase: 'focus', status: 'running' });
  });

  it('pauses without losing time and skips to the next phase', () => {
    const { store } = setup({ focusMinutes: 25 });
    store.start();
    vi.advanceTimersByTime(10 * MIN);
    store.pause();
    vi.advanceTimersByTime(30 * MIN);
    expect(store.remaining).toBe(15 * MIN);
    store.resume();
    store.skip();
    expect(store.state).toMatchObject({ phase: 'short', status: 'running', round: 1 });
  });

  it('covers the screen during breaks in overlay mode and leaves it from the break window', () => {
    const { store, commands, emit } = setup({ breakMode: 'overlay', autoStartBreaks: true, autoStartFocus: false });
    store.start();
    expect(commands()).not.toContain('break_show');

    store.skip();
    expect(commands().at(-1)).toBe('break_show');

    // The user holds "skip" in the break window.
    emit('break://finished');
    expect(store.state).toMatchObject({ phase: 'focus', status: 'running' });
    expect(commands().at(-1)).toBe('break_hide');
  });

  it('keeps the break window open when the break ends and waits for the user', () => {
    const { store, calls, emit, timers } = setup({ breakMode: 'overlay', shortBreakMinutes: 5, autoStartFocus: false });
    store.start();
    store.skip();
    vi.advanceTimersByTime(5 * MIN + 300);
    const [command, overlay] = calls.at(-1)!;
    expect(command).toBe('break_show');
    expect(overlay).toMatchObject({ remaining: 0 });
    expect(timers.ringing?.kind).toBe('pomodoro');

    emit('break://finished');
    expect(store.state).toMatchObject({ phase: 'focus', status: 'running' });
    expect(timers.ringing).toBeNull();
  });

  it('stops cleanly from anywhere', () => {
    const { store, commands, timers } = setup({ breakMode: 'overlay', autoStartBreaks: false });
    store.start();
    vi.advanceTimersByTime(25 * MIN + 300);
    expect(timers.ringing?.kind).toBe('pomodoro');
    store.stop();
    expect(timers.ringing).toBeNull();
    expect(store.state.status).toBe('idle');
    expect(commands()).not.toContain('break_show');
  });
});
