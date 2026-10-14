/**
 * Timers, stopwatch and alarms. Owned by the frontend (the island window
 * runs permanently); alarms persist through the settings.
 */
import type { Alarm, Settings, SettingsPatch } from '$backend/contract';
import { loopSignal } from './sound';

/** A countdown timer. */
export interface TimerItem {
  id: string;
  label: string;
  /** Total length in ms. */
  duration: number;
  /** End time in ms while running. */
  endsAt: number | null;
  /** Remaining ms while paused. */
  remaining: number;
  state: 'running' | 'paused' | 'done';
}

/** Stopwatch state. */
export interface StopwatchState {
  running: boolean;
  /** Start of the current running stretch (ms). */
  startedAt: number;
  /** Time accumulated before the current stretch (ms). */
  elapsed: number;
  /** Lap split times (ms since start), oldest first. */
  laps: number[];
}

/** Something currently ringing. */
export interface Ringing {
  kind: 'timer' | 'alarm';
  id: string;
  label: string;
  /** "HH:MM" for alarms. */
  time: string;
}

/**
 * Next occurrence of an alarm.
 *
 * @param alarm - Alarm definition.
 * @param from - Reference time.
 * @returns Date of the next ring, or null if disabled.
 */
export function nextAlarmTime(alarm: Alarm, from = new Date()): Date | null {
  if (!alarm.enabled) {
    return null;
  }
  const [h, m] = alarm.time.split(':').map(Number);
  for (let offset = 0; offset < 8; offset++) {
    const d = new Date(from.getFullYear(), from.getMonth(), from.getDate() + offset, h ?? 0, m ?? 0, 0, 0);
    const weekday = (d.getDay() + 6) % 7;
    if (d > from && (alarm.days.length === 0 || alarm.days.includes(weekday))) {
      return d;
    }
  }
  return null;
}

/** Frontend-owned clock features. */
export class TimersStore {
  timers = $state<TimerItem[]>([]);
  stopwatch = $state<StopwatchState>({ running: false, startedAt: 0, elapsed: 0, laps: [] });
  ringing = $state<Ringing | null>(null);
  /** Ticks every 100 ms while anything runs (display only). */
  now = $state(Date.now());

  private ticker: ReturnType<typeof setInterval> | null = null;
  private stopSound: (() => void) | null = null;
  private snoozed: Array<{ alarm: Alarm; at: number }> = [];
  private lastAlarmCheck = '';

  /**
   * @param settings - Returns current settings (sound, alarms).
   * @param patch - Persists settings changes.
   */
  constructor(
    private readonly settings: () => Settings,
    private readonly patch: (p: SettingsPatch) => Promise<void>,
  ) {
    setInterval(() => this.checkAlarms(), 1000);
  }

  /** Running or paused timers, soonest end first. */
  get active(): TimerItem[] {
    return this.timers
      .filter((t) => t.state !== 'done')
      .sort((a, b) => this.remainingOf(a) - this.remainingOf(b));
  }

  /**
   * @param t - Timer.
   * @returns Remaining ms right now.
   */
  remainingOf(t: TimerItem): number {
    return t.state === 'running' && t.endsAt !== null ? Math.max(0, t.endsAt - this.now) : t.remaining;
  }

  /** Elapsed stopwatch time right now (ms). */
  get stopwatchElapsed(): number {
    const s = this.stopwatch;
    return s.elapsed + (s.running ? this.now - s.startedAt : 0);
  }

  // ── Timers ──────────────────────────────────────────────────────────────

  /**
   * Starts a new timer.
   *
   * @param duration - Length in ms.
   * @param label - Optional name.
   */
  start(duration: number, label: string): void {
    const now = Date.now();
    this.timers = [...this.timers, {
      id: `timer-${now}`, label, duration, endsAt: now + duration, remaining: duration, state: 'running',
    }];
    this.ensureTicker();
  }

  /** @param id - Timer to pause. */
  pause(id: string): void {
    this.update(id, (t) => ({ ...t, state: 'paused', remaining: this.remainingOf(t), endsAt: null }));
  }

  /** @param id - Timer to resume. */
  resume(id: string): void {
    this.update(id, (t) => ({ ...t, state: 'running', endsAt: Date.now() + t.remaining }));
    this.ensureTicker();
  }

  /** @param id - Timer to extend by one minute. */
  addMinute(id: string): void {
    this.update(id, (t) => (t.state === 'running' && t.endsAt !== null
      ? { ...t, endsAt: t.endsAt + 60_000, duration: t.duration + 60_000 }
      : { ...t, remaining: t.remaining + 60_000, duration: t.duration + 60_000 }));
  }

  /** @param id - Timer to remove (also stops its ringing). */
  cancel(id: string): void {
    this.timers = this.timers.filter((t) => t.id !== id);
    if (this.ringing?.id === id) {
      this.silence();
    }
  }

  // ── Stopwatch ───────────────────────────────────────────────────────────

  /** Starts or stops the stopwatch. */
  toggleStopwatch(): void {
    const s = this.stopwatch;
    const now = Date.now();
    this.stopwatch = s.running
      ? { ...s, running: false, elapsed: s.elapsed + now - s.startedAt }
      : { ...s, running: true, startedAt: now };
    this.ensureTicker();
  }

  /** Records a lap. */
  lap(): void {
    if (this.stopwatch.running) {
      this.stopwatch = { ...this.stopwatch, laps: [...this.stopwatch.laps, this.stopwatchElapsed] };
    }
  }

  /** Resets the stopwatch. */
  resetStopwatch(): void {
    this.stopwatch = { running: false, startedAt: 0, elapsed: 0, laps: [] };
  }

  // ── Alarms ──────────────────────────────────────────────────────────────

  /** Alarms from the settings. */
  get alarms(): Alarm[] {
    return this.settings().timers.alarms;
  }

  /** The next alarm that will ring. */
  get nextAlarm(): { alarm: Alarm; at: Date } | null {
    let best: { alarm: Alarm; at: Date } | null = null;
    for (const alarm of this.alarms) {
      const at = nextAlarmTime(alarm);
      if (at && (!best || at < best.at)) {
        best = { alarm, at };
      }
    }
    return best;
  }

  /** @param alarms - New alarm list (persisted). */
  saveAlarms(alarms: Alarm[]): Promise<void> {
    return this.patch({ timers: { alarms } });
  }

  /**
   * @param time - "HH:MM".
   * @param label - Optional label.
   * @param days - Weekdays 0 = Monday; empty = once.
   */
  addAlarm(time: string, label: string, days: number[]): Promise<void> {
    return this.saveAlarms([...this.alarms, { id: `alarm-${Date.now()}`, time, label, days, enabled: true }]);
  }

  /**
   * @param id - Alarm id.
   * @param patch - Fields to change.
   */
  updateAlarm(id: string, patch: Partial<Alarm>): Promise<void> {
    return this.saveAlarms(this.alarms.map((a) => (a.id === id ? { ...a, ...patch } : a)));
  }

  /** @param id - Alarm to delete. */
  removeAlarm(id: string): Promise<void> {
    return this.saveAlarms(this.alarms.filter((a) => a.id !== id));
  }

  /** Rings an alarm immediately (lab). */
  ringNow(alarm: Alarm): void {
    this.ring({ kind: 'alarm', id: alarm.id, label: alarm.label, time: alarm.time });
  }

  // ── Ringing ─────────────────────────────────────────────────────────────

  /** Snoozes the ringing alarm (or dismisses a finished timer). */
  snooze(): void {
    const ringing = this.ringing;
    if (ringing?.kind === 'alarm') {
      const alarm = this.alarms.find((a) => a.id === ringing.id) ?? {
        id: ringing.id, time: ringing.time, label: ringing.label, days: [], enabled: true,
      };
      this.snoozed.push({ alarm, at: Date.now() + this.settings().timers.snoozeMinutes * 60_000 });
    }
    this.dismiss();
  }

  /** Silences a finished timer and restarts it for one more minute. */
  extendRinging(): void {
    const ringing = this.ringing;
    if (ringing?.kind !== 'timer') {
      return;
    }
    this.dismiss();
    this.start(60_000, ringing.label);
  }

  /** Stops the signal; one-off alarms disable themselves. */
  dismiss(): void {
    const ringing = this.ringing;
    if (ringing?.kind === 'timer') {
      this.timers = this.timers.filter((t) => t.id !== ringing.id);
    } else if (ringing?.kind === 'alarm') {
      const alarm = this.alarms.find((a) => a.id === ringing.id);
      if (alarm && alarm.days.length === 0) {
        void this.updateAlarm(alarm.id, { enabled: false });
      }
    }
    this.silence();
  }

  private ring(ringing: Ringing): void {
    this.silence();
    this.ringing = ringing;
    const { sound, soundVolume } = this.settings().timers;
    this.stopSound = loopSignal(sound, soundVolume);
  }

  private silence(): void {
    this.stopSound?.();
    this.stopSound = null;
    this.ringing = null;
  }

  private update(id: string, fn: (t: TimerItem) => TimerItem): void {
    this.timers = this.timers.map((t) => (t.id === id ? fn(t) : t));
  }

  private ensureTicker(): void {
    if (this.ticker) {
      return;
    }
    this.ticker = setInterval(() => {
      this.now = Date.now();
      for (const t of this.timers) {
        if (t.state === 'running' && t.endsAt !== null && t.endsAt <= this.now) {
          this.update(t.id, (x) => ({ ...x, state: 'done', remaining: 0, endsAt: null }));
          this.ring({ kind: 'timer', id: t.id, label: t.label, time: '' });
        }
      }
      const busy = this.stopwatch.running || this.timers.some((t) => t.state === 'running');
      if (!busy && this.ticker) {
        clearInterval(this.ticker);
        this.ticker = null;
      }
    }, 100);
  }

  private checkAlarms(): void {
    const now = new Date();
    const key = `${now.getHours()}:${now.getMinutes()}`;
    const due = this.snoozed.filter((s) => s.at <= now.getTime());
    if (due.length) {
      this.snoozed = this.snoozed.filter((s) => s.at > now.getTime());
      const first = due[0]!.alarm;
      this.ring({ kind: 'alarm', id: first.id, label: first.label, time: first.time });
    }
    if (key === this.lastAlarmCheck) {
      return;
    }
    this.lastAlarmCheck = key;
    const weekday = (now.getDay() + 6) % 7;
    const hhmm = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
    const alarm = this.alarms.find((a) => a.enabled && a.time === hhmm && (a.days.length === 0 || a.days.includes(weekday)));
    if (alarm) {
      this.ring({ kind: 'alarm', id: alarm.id, label: alarm.label, time: alarm.time });
    }
  }
}
