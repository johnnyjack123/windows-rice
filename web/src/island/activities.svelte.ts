/**
 * Activity registry: which live activities exist, which are active right now
 * and which one owns the island. Extension point — new activities register a
 * definition and call show/hide from wherever their data comes from.
 */
import type { Component } from 'svelte';

/**
 * Priorities of built-in activities; extensions pick values in between.
 * Music keeps the main pill while it plays; running clocks split off into
 * the bubble, where hovering shows their quick actions.
 */
export const PRIORITY = {
  alarm: 110,
  timerDone: 105,
  pomodoroDone: 104,
  notification: 100,
  pomodoroPhase: 95,
  osd: 90,
  charging: 80,
  event: 60,
  music: 57,
  pomodoro: 55,
  timer: 50,
  stopwatch: 45,
} as const;

/** Definition of an activity type. */
export interface ActivityDef {
  id: string;
  priority: number;
  /** Presentation when it owns the island. */
  view: Component;
  /** Height equals the island height (allows a split bubble next to it). */
  compact: boolean;
  /** Taller presentation shown after hovering (e.g. the music player). */
  expandedView?: Component;
  /** Presentation inside the split bubble when it is the second activity. */
  bubble?: Component;
  /** Panel section opened on click. */
  section?: string;
  /** Short-lived; may briefly show while the island is auto-hidden. */
  transient?: boolean;
  /** Also shows over full-screen apps (alarms, finished timers). */
  critical?: boolean;
  /** Accessible description of the current state. */
  label: () => string;
}

/** An active instance. */
export interface ActiveEntry {
  id: string;
  /** When it was (re)shown, for tie-breaking. */
  since: number;
  /** Increments on in-place updates that deserve a physical bump. */
  bumps: number;
}

/** Options for {@link ActivityRegistry.show}. */
export interface ShowOptions {
  /** Auto-hide after this many ms (0/undefined = until hidden). */
  ttl?: number;
  /** Play a bump if the activity is already showing. */
  bump?: boolean;
}

/** The registry of one island. */
export class ActivityRegistry {
  /** Active entries; reactive. */
  active = $state<ActiveEntry[]>([]);

  private readonly defs = new Map<string, ActivityDef>();
  private readonly timers = new Map<string, ReturnType<typeof setTimeout>>();
  private readonly ttls = new Map<string, number>();
  private readonly held = new Set<string>();

  /** @param timeScale - Returns the lab slow-motion factor (lifetimes stretch with it). */
  constructor(private readonly timeScale: () => number = () => 1) {}

  /**
   * Registers an activity type.
   *
   * @param def - Definition; ids must be unique.
   */
  define(def: ActivityDef): void {
    this.defs.set(def.id, def);
  }

  /** All registered definitions. */
  get definitions(): ActivityDef[] {
    return [...this.defs.values()];
  }

  /**
   * @param id - Activity id.
   * @returns Its definition.
   */
  def(id: string): ActivityDef | undefined {
    return this.defs.get(id);
  }

  /**
   * Shows (or refreshes) an activity.
   *
   * @param id - Registered activity id.
   * @param options - Lifetime and bump.
   */
  show(id: string, options: ShowOptions = {}): void {
    if (!this.defs.has(id)) {
      console.warn(`[island] unknown activity "${id}"`);
      return;
    }
    const existing = this.active.find((e) => e.id === id);
    if (existing) {
      if (options.bump) {
        existing.bumps += 1;
      }
    } else {
      this.active = [...this.active, { id, since: Date.now(), bumps: 0 }];
    }
    this.arm(id, options.ttl ?? 0);
  }

  /** @param id - Activity to remove. */
  hide(id: string): void {
    clearTimeout(this.timers.get(id));
    this.timers.delete(id);
    this.ttls.delete(id);
    this.held.delete(id);
    if (this.active.some((e) => e.id === id)) {
      this.active = this.active.filter((e) => e.id !== id);
    }
  }

  /**
   * @param id - Activity id.
   * @returns Whether it is active.
   */
  has(id: string): boolean {
    return this.active.some((e) => e.id === id);
  }

  /**
   * Pauses the expiry while the pointer rests on the island.
   *
   * @param id - Activity id.
   */
  hold(id: string): void {
    if (this.ttls.has(id)) {
      clearTimeout(this.timers.get(id));
      this.held.add(id);
    }
  }

  /**
   * Resumes a held expiry with a short grace period.
   *
   * @param id - Activity id.
   * @param grace - Remaining lifetime in ms.
   */
  release(id: string, grace = 1800): void {
    if (this.held.delete(id)) {
      this.arm(id, grace);
    }
  }

  /** Active entries, highest priority first (newest wins ties). */
  get ordered(): ActiveEntry[] {
    return [...this.active].sort((a, b) => {
      const pa = this.defs.get(a.id)?.priority ?? 0;
      const pb = this.defs.get(b.id)?.priority ?? 0;
      return pb - pa || b.since - a.since;
    });
  }

  /** The activity owning the island. */
  get main(): ActiveEntry | null {
    return this.ordered[0] ?? null;
  }

  /** The activity shown in the split bubble, if the main one is compact. */
  get second(): ActiveEntry | null {
    const main = this.main;
    if (!main || !this.defs.get(main.id)?.compact) {
      return null;
    }
    return this.ordered.find((e) => e !== main && this.defs.get(e.id)?.bubble) ?? null;
  }

  private arm(id: string, ttl: number): void {
    clearTimeout(this.timers.get(id));
    this.timers.delete(id);
    if (ttl > 0) {
      this.ttls.set(id, ttl);
      if (!this.held.has(id)) {
        this.timers.set(id, setTimeout(() => this.hide(id), ttl / this.timeScale()));
      }
    } else {
      this.ttls.delete(id);
    }
  }
}
