/**
 * Pomodoro: focus sessions and breaks in a cycle that ends with a long
 * break. Owned by the island window like the other clock features; the
 * full-screen break window only displays what this store sends it.
 */
import type { BreakOverlay, PomodoroSettings, Settings } from '$backend/contract';
import type { Transport } from '$backend/transport';
import { playSignal } from './sound';
import type { TimersStore } from './timers.svelte';

/** Phase of the cycle. */
export type PomodoroPhase = 'focus' | 'short' | 'long';

/**
 * - idle: nothing running.
 * - running / paused: the phase counts down.
 * - waiting: the phase is ready but waits for the user (no auto-start).
 */
export type PomodoroStatus = 'idle' | 'running' | 'paused' | 'waiting';

/** Live state of the pomodoro. */
export interface PomodoroState {
  phase: PomodoroPhase;
  status: PomodoroStatus;
  /** End time while running. */
  endsAt: number | null;
  /** Remaining ms while paused or waiting. */
  remaining: number;
  /** Length of the phase in ms. */
  duration: number;
  /** Focus sessions completed in this cycle. */
  round: number;
}

/** A phase change the island announces briefly. */
export interface PomodoroChange {
  phase: PomodoroPhase;
  at: number;
}

/** Id of the pomodoro's signal in the shared ringing queue. */
export const POMODORO_SIGNAL = 'pomodoro';

/** Icon of each phase. */
export const PHASE_ICON: Record<PomodoroPhase, string> = {
  focus: 'target',
  short: 'coffee',
  long: 'self_improvement',
};

/**
 * Which phase follows and how the round counter moves on.
 *
 * @param phase - Phase that just ended.
 * @param round - Focus sessions completed before it ended.
 * @param rounds - Focus sessions per cycle.
 * @returns The next phase and the updated round counter.
 */
export function nextPhase(phase: PomodoroPhase, round: number, rounds: number): { phase: PomodoroPhase; round: number } {
  if (phase === 'focus') {
    const done = round + 1;
    return { phase: done >= Math.max(1, rounds) ? 'long' : 'short', round: done };
  }
  return { phase: 'focus', round: phase === 'long' ? 0 : round };
}

const IDLE: PomodoroState = { phase: 'focus', status: 'idle', endsAt: null, remaining: 0, duration: 0, round: 0 };

/** Pomodoro of one island. */
export class PomodoroStore {
  state = $state<PomodoroState>({ ...IDLE });
  /** Last automatic phase change (drives the island's short announcement). */
  change = $state<PomodoroChange | null>(null);
  /** Ticks while running (display only). */
  now = $state(Date.now());

  private ticker: ReturnType<typeof setInterval> | null = null;
  /** The break window is showing. */
  private overlayOpen = false;

  /**
   * @param settings - Returns current settings.
   * @param transport - Backend (break window).
   * @param signals - Shared ringing queue of the clock features.
   */
  constructor(
    private readonly settings: () => Settings,
    private readonly transport: Transport,
    private readonly signals: TimersStore,
  ) {
    transport.listen('break://finished', () => this.leaveBreak());
  }

  /** Current pomodoro settings. */
  get config(): PomodoroSettings {
    return this.settings().timers.pomodoro;
  }

  /** Remaining ms right now. */
  get remaining(): number {
    const s = this.state;
    return s.status === 'running' && s.endsAt !== null ? Math.max(0, s.endsAt - this.now) : s.remaining;
  }

  /** 0…1 of the phase still to go. */
  get progress(): number {
    return this.state.duration ? this.remaining / this.state.duration : 0;
  }

  /** Whether a cycle is in progress (anything but idle). */
  get active(): boolean {
    return this.state.status !== 'idle';
  }

  /** Whether the current phase is a break. */
  get onBreak(): boolean {
    return this.state.phase !== 'focus';
  }

  /**
   * @param phase - A phase.
   * @returns Its configured length in ms.
   */
  lengthOf(phase: PomodoroPhase): number {
    const c = this.config;
    const minutes = phase === 'focus' ? c.focusMinutes : phase === 'short' ? c.shortBreakMinutes : c.longBreakMinutes;
    return Math.max(1, minutes) * 60_000;
  }

  /** Starts a cycle, or the waiting phase. */
  start(): void {
    if (this.state.status === 'waiting') {
      this.startNext();
    } else if (this.state.status === 'idle') {
      this.begin('focus', 0);
    } else if (this.state.status === 'paused') {
      this.resume();
    }
  }

  /** Starts the phase that is waiting for the user. */
  startNext(): void {
    this.signals.clearSignal(POMODORO_SIGNAL);
    this.begin(this.state.phase, this.state.round);
  }

  /** Pauses the countdown. */
  pause(): void {
    const s = this.state;
    if (s.status !== 'running' || s.endsAt === null) {
      return;
    }
    this.state = { ...s, status: 'paused', remaining: Math.max(0, s.endsAt - Date.now()), endsAt: null };
    this.syncOverlay();
  }

  /** Continues a paused countdown. */
  resume(): void {
    const s = this.state;
    if (s.status !== 'paused') {
      return;
    }
    this.state = { ...s, status: 'running', endsAt: Date.now() + s.remaining };
    this.ensureTicker();
    this.syncOverlay();
  }

  /** Ends the current phase now and starts the next one. */
  skip(): void {
    const s = this.state;
    if (s.status === 'idle') {
      return;
    }
    if (s.status === 'waiting') {
      this.startNext();
      return;
    }
    const next = nextPhase(s.phase, s.round, this.config.sessionsUntilLongBreak);
    this.begin(next.phase, next.round);
  }

  /** Stops the cycle. */
  stop(): void {
    this.signals.clearSignal(POMODORO_SIGNAL);
    this.state = { ...IDLE };
    this.stopTicker();
    this.syncOverlay();
  }

  private begin(phase: PomodoroPhase, round: number): void {
    const duration = this.lengthOf(phase);
    this.state = { phase, status: 'running', endsAt: Date.now() + duration, remaining: duration, duration, round };
    this.ensureTicker();
    this.syncOverlay();
  }

  /** The countdown reached zero. */
  private finish(): void {
    const s = this.state;
    const c = this.config;
    const next = nextPhase(s.phase, s.round, c.sessionsUntilLongBreak);
    const auto = next.phase === 'focus' ? c.autoStartFocus : c.autoStartBreaks;
    if (auto) {
      playSignal(this.settings().timers.sound, this.settings().timers.soundVolume);
      this.begin(next.phase, next.round);
      this.change = { phase: next.phase, at: Date.now() };
      return;
    }
    const duration = this.lengthOf(next.phase);
    const brokeFromBreak = s.phase !== 'focus';
    this.state = { phase: next.phase, status: 'waiting', endsAt: null, remaining: duration, duration, round: next.round };
    this.stopTicker();
    this.signals.signal({ kind: 'pomodoro', id: POMODORO_SIGNAL, label: next.phase, time: '' });
    if (brokeFromBreak && this.overlayOpen) {
      // The break window stays and offers to get back to work.
      void this.transport.invoke('break_show', { ...this.overlayOf(s), endsAt: Date.now(), remaining: 0 });
    }
  }

  /** The user left the break in the break window. */
  private leaveBreak(): void {
    const s = this.state;
    if (s.status === 'waiting' && s.phase === 'focus') {
      this.startNext();
    } else if (s.phase !== 'focus' && (s.status === 'running' || s.status === 'paused')) {
      this.skip();
    }
  }

  private overlayOf(s: PomodoroState): BreakOverlay {
    return {
      endsAt: s.endsAt,
      remaining: s.status === 'running' && s.endsAt !== null ? Math.max(0, s.endsAt - Date.now()) : s.remaining,
      duration: s.duration,
      kind: s.phase === 'long' ? 'long' : 'short',
      round: s.round,
      rounds: this.config.sessionsUntilLongBreak,
    };
  }

  /** Shows, updates or closes the break window to match the state. */
  private syncOverlay(): void {
    const s = this.state;
    const breakRunning = s.phase !== 'focus' && (s.status === 'running' || s.status === 'paused');
    if (breakRunning && this.config.breakMode === 'overlay') {
      this.overlayOpen = true;
      void this.transport.invoke('break_show', this.overlayOf(s));
    } else if (this.overlayOpen && (s.phase === 'focus' || s.status === 'idle')) {
      this.overlayOpen = false;
      void this.transport.invoke('break_hide');
    }
  }

  private ensureTicker(): void {
    this.now = Date.now();
    if (this.ticker) {
      return;
    }
    this.ticker = setInterval(() => {
      this.now = Date.now();
      const s = this.state;
      if (s.status === 'running' && s.endsAt !== null && s.endsAt <= this.now) {
        this.finish();
      }
    }, 250);
  }

  private stopTicker(): void {
    if (this.ticker) {
      clearInterval(this.ticker);
      this.ticker = null;
    }
  }
}
