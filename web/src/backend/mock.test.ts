import { describe, expect, it, vi } from 'vitest';
import { MockBackend } from './mock/mock-backend';

describe('simulated backend', () => {
  it('persists settings patches and echoes them as an event', async () => {
    const mock = new MockBackend();
    const seen = vi.fn();
    mock.listen('config://changed', seen);
    const stored = await mock.invoke('config_set', { patch: { locale: 'en', island: { autoHide: false } } });
    expect(stored).toMatchObject({ locale: 'en', island: { autoHide: false } });
    expect(seen).toHaveBeenCalledOnce();
  });

  it('rejects wrong Wi-Fi keys and connects with a valid one', async () => {
    const mock = new MockBackend();
    expect(await mock.invoke('network_connect', { ssid: 'o2-WLAN61', key: 'falsch123' })).toEqual({ ok: false, error: 'wrong_key' });
    expect(await mock.invoke('network_connect', { ssid: 'o2-WLAN61', key: 'richtig123' })).toEqual({ ok: true });
  }, 10_000);

  it('returns events of a range sorted and only from enabled calendars', async () => {
    const mock = new MockBackend();
    const now = new Date();
    const from = new Date(now.getFullYear(), now.getMonth(), now.getDate()).toISOString();
    const to = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 2).toISOString();
    const events = await mock.invoke('calendar_events', { from, to });
    expect(events.length).toBeGreaterThan(0);
    expect([...events].sort((a, b) => a.start.localeCompare(b.start))).toEqual(events);
    await mock.invoke('calendar_set_enabled', { calendarId: 'private', enabled: false });
    const filtered = await mock.invoke('calendar_events', { from, to });
    expect(filtered.some((e) => e.calendarId === 'private')).toBe(false);
  });

  it('refuses to end elevated processes', async () => {
    const mock = new MockBackend();
    await expect(mock.invoke('system_kill', { pid: 4 })).rejects.toMatchObject({ code: 'access_denied' });
  });
});
