/**
 * Shared second-aligned clock. Components read `clock.now` to re-render once
 * per second without each running its own timer.
 */
class Clock {
  /** Current time in milliseconds, updated on every full second. */
  now = $state(Date.now());

  private timer: ReturnType<typeof setTimeout> | null = null;

  /** Starts ticking (idempotent). */
  start(): void {
    if (this.timer) {
      return;
    }
    const tick = (): void => {
      this.now = Date.now();
      this.timer = setTimeout(tick, 1000 - (this.now % 1000) + 5);
    };
    tick();
  }

  /** Current time as a Date. */
  get date(): Date {
    return new Date(this.now);
  }
}

/** The app-wide clock. */
export const clock = new Clock();
