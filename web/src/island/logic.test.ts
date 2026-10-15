import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Component } from 'svelte';
import { ActivityRegistry, PRIORITY } from './activities.svelte';
import { metaballPath } from './metaball';
import { nextAlarmTime } from './timers.svelte';
import { shouldHide } from './window.svelte';

const View = (() => null) as unknown as Component;

function registry(): ActivityRegistry {
  const r = new ActivityRegistry();
  r.define({ id: 'music', priority: PRIORITY.music, view: View, bubble: View, compact: true, label: () => '' });
  r.define({ id: 'timer', priority: PRIORITY.timer, view: View, bubble: View, compact: true, label: () => '' });
  r.define({ id: 'notification', priority: PRIORITY.notification, view: View, compact: false, transient: true, label: () => '' });
  return r;
}

describe('activity registry', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('lets the highest priority own the island and splits the next one off', () => {
    const r = registry();
    r.show('timer');
    expect(r.main?.id).toBe('timer');
    expect(r.second).toBeNull();
    // Music keeps the main pill; the running timer moves to the bubble.
    r.show('music');
    expect(r.main?.id).toBe('music');
    expect(r.second?.id).toBe('timer');
  });

  it('never splits next to a tall activity', () => {
    const r = registry();
    r.show('music');
    r.show('notification', { ttl: 4800 });
    expect(r.main?.id).toBe('notification');
    expect(r.second).toBeNull();
  });

  it('expires transient activities and pauses expiry while held', () => {
    const r = registry();
    r.show('notification', { ttl: 1000 });
    r.hold('notification');
    vi.advanceTimersByTime(5000);
    expect(r.has('notification')).toBe(true);
    r.release('notification', 500);
    vi.advanceTimersByTime(600);
    expect(r.has('notification')).toBe(false);
  });

  it('counts bumps for in-place updates', () => {
    const r = registry();
    r.show('music');
    r.show('music', { bump: true });
    expect(r.main?.bumps).toBe(1);
  });
});

describe('auto-hide decision', () => {
  const base = { autoHide: true, peek: true, hideInFullscreen: true, expanded: false, transient: false, critical: false };
  const free = { occluded: false, fullscreen: false, edge: false };

  it('hides only when a window overlaps', () => {
    expect(shouldHide(free, base)).toBe(false);
    expect(shouldHide({ ...free, occluded: true }, base)).toBe(true);
    expect(shouldHide({ ...free, occluded: true }, { ...base, autoHide: false })).toBe(false);
  });

  it('peeks for transient events, reveals at the edge and while open', () => {
    const occluded = { ...free, occluded: true };
    expect(shouldHide(occluded, { ...base, transient: true })).toBe(false);
    expect(shouldHide(occluded, { ...base, transient: true, peek: false })).toBe(true);
    expect(shouldHide({ ...occluded, edge: true }, base)).toBe(false);
    expect(shouldHide(occluded, { ...base, expanded: true })).toBe(false);
  });

  it('stays hidden over full-screen apps except for alarms', () => {
    const fullscreen = { ...free, fullscreen: true };
    expect(shouldHide(fullscreen, { ...base, transient: true })).toBe(true);
    expect(shouldHide(fullscreen, { ...base, critical: true })).toBe(false);
  });
});

describe('liquid bridge', () => {
  it('connects close circles and snaps when they part', () => {
    const cap = { x: 100, y: 17, r: 17 };
    expect(metaballPath(cap, { x: 120, y: 17, r: 17 }, 40)).toMatch(/^M .* Z$/);
    expect(metaballPath(cap, { x: 160, y: 17, r: 17 }, 40)).toBe('');
    // One circle inside the other: nothing to bridge.
    expect(metaballPath(cap, { x: 101, y: 17, r: 5 }, 40)).toBe('');
  });
});

describe('alarms', () => {
  it('finds the next matching weekday', () => {
    // Monday 2026-10-12, 08:00
    const monday = new Date(2026, 9, 12, 8, 0);
    expect(nextAlarmTime({ id: 'a', time: '07:00', label: '', days: [0], enabled: true }, monday)?.getDate()).toBe(19);
    expect(nextAlarmTime({ id: 'a', time: '09:00', label: '', days: [], enabled: true }, monday)?.getHours()).toBe(9);
    expect(nextAlarmTime({ id: 'a', time: '07:00', label: '', days: [5, 6], enabled: true }, monday)?.getDay()).toBe(6);
    expect(nextAlarmTime({ id: 'a', time: '07:00', label: '', days: [], enabled: false }, monday)).toBeNull();
  });
});
