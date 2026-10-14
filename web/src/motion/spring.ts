/**
 * Spring physics shared by JS-driven geometry and CSS transitions.
 *
 * A spring is described like SwiftUI's `.spring(duration:bounce:)`:
 * `duration` is the perceived duration in seconds, `bounce` goes from 0
 * (critically damped) to ~0.5 (very springy).
 */

/** Designer-facing spring parameters: `[duration in s, bounce]`. */
export type SpringToken = readonly [duration: number, bounce: number];

/** Physical constants of a spring with unit mass. */
export interface SpringConstants {
  stiffness: number;
  damping: number;
}

/** Global playback settings for every spring and duration. */
export const motionSettings = {
  /** Lab slow-motion multiplier (1 = real time, 0.2 = five times slower). */
  timeScale: 1,
  /** Reduced motion: springs settle without overshoot and travel is minimised. */
  reduced: false,
};

/**
 * Converts designer-facing parameters into physical constants.
 *
 * @param duration - Perceived duration in seconds.
 * @param bounce - 0 = no overshoot, positive = springy.
 * @returns Stiffness and damping for a unit mass.
 */
export function springConstants(duration: number, bounce: number): SpringConstants {
  const d = Math.max(0.05, duration);
  const omega = (2 * Math.PI) / d;
  const zeta = bounce >= 0 ? 1 - bounce : 1 / (1 + bounce);
  return { stiffness: omega * omega, damping: 2 * zeta * omega };
}

/**
 * Parses a spring token such as `"0.5 0.24"`. Under reduced motion every
 * spring becomes short and critically damped.
 *
 * @param value - Raw custom property value.
 * @param fallback - Used when the value cannot be parsed.
 * @param respectReduced - Apply the reduced-motion clamp (default true).
 * @returns The parsed spring.
 */
export function parseSpring(
  value: string | null | undefined,
  fallback: SpringToken = [0.5, 0.2],
  respectReduced = true,
): SpringToken {
  const parts = String(value ?? '').trim().split(/\s+/).map(Number);
  const [duration, bounce] = parts;
  const spring: SpringToken = parts.length >= 2 && Number.isFinite(duration) && Number.isFinite(bounce)
    ? [duration!, bounce!]
    : fallback;
  if (respectReduced && motionSettings.reduced) {
    return [Math.min(spring[0], 0.3), 0];
  }
  return spring;
}

/**
 * Serialises a spring back into token form.
 *
 * @param spring - Spring to serialise.
 * @returns Token string, e.g. `"0.5 0.24"`.
 */
export function formatSpring(spring: SpringToken): string {
  return `${round(spring[0], 3)} ${round(spring[1], 3)}`;
}

/**
 * Simulates a spring from 0 to 1 and returns a CSS `linear()` easing that
 * reproduces it, plus the time it needs to settle. Keeps CSS transitions and
 * JS springs perceptually identical.
 *
 * @param spring - Spring to sample.
 * @returns The easing function and its duration in milliseconds.
 */
export function springToLinear(spring: SpringToken): { easing: string; ms: number } {
  const { stiffness, damping } = springConstants(spring[0], spring[1]);
  const dt = 1 / 600;
  let x = 0;
  let v = 0;
  let t = 0;
  const samples: number[] = [];
  while (t < 4) {
    const a = -stiffness * (x - 1) - damping * v;
    v += a * dt;
    x += v * dt;
    t += dt;
    samples.push(x);
    if (Math.abs(1 - x) < 0.002 && Math.abs(v) < 0.02) {
      break;
    }
  }
  const count = 40;
  const points = ['0'];
  for (let i = 1; i < count; i++) {
    const index = Math.max(0, Math.min(samples.length - 1, Math.round((i / count) * samples.length) - 1));
    points.push(`${(samples[index] ?? 1).toFixed(4)} ${((i / count) * 100).toFixed(1)}%`);
  }
  points.push('1');
  return { easing: `linear(${points.join(', ')})`, ms: Math.round(t * 1000) };
}

/**
 * A single animated number driven by a damped spring. Retargeting keeps the
 * current velocity, so interrupted animations stay continuous.
 */
export class SpringValue {
  /** Current value. */
  value: number;
  /** Value the spring is moving towards. */
  target: number;
  /** Current velocity in units per second. */
  velocity = 0;
  /** Distance at which the spring snaps to rest. */
  readonly precision: number;
  private constants: SpringConstants;

  /**
   * @param value - Initial value.
   * @param spring - Spring parameters.
   * @param precision - Rest threshold in value units.
   */
  constructor(value: number, spring: SpringToken, precision = 0.05) {
    this.value = value;
    this.target = value;
    this.precision = precision;
    this.constants = springConstants(spring[0], spring[1]);
  }

  /**
   * Replaces the spring parameters without disturbing motion in flight.
   *
   * @param spring - New spring parameters.
   */
  setSpring(spring: SpringToken): void {
    this.constants = springConstants(spring[0], spring[1]);
  }

  /**
   * Moves the target; velocity is preserved.
   *
   * @param target - New target value.
   */
  to(target: number): void {
    this.target = target;
  }

  /**
   * Teleports to a value and stops.
   *
   * @param value - New value and target.
   */
  jump(value: number): void {
    this.value = value;
    this.target = value;
    this.velocity = 0;
  }

  /**
   * Adds velocity, e.g. for a physical "bump" on new content.
   *
   * @param velocity - Velocity to add, in units per second.
   */
  impulse(velocity: number): void {
    this.velocity += velocity;
  }

  /** Whether the spring is at rest on its target. */
  get settled(): boolean {
    return Math.abs(this.value - this.target) < this.precision
      && Math.abs(this.velocity) < this.precision * 10;
  }

  /**
   * Advances the simulation. Sub-steps keep stiff springs stable.
   *
   * @param dt - Elapsed time in seconds.
   */
  step(dt: number): void {
    const { stiffness, damping } = this.constants;
    const steps = Math.max(1, Math.ceil(dt / (1 / 240)));
    const h = dt / steps;
    for (let i = 0; i < steps; i++) {
      const a = -stiffness * (this.value - this.target) - damping * this.velocity;
      this.velocity += a * h;
      this.value += this.velocity * h;
    }
  }
}

/**
 * Runs one requestAnimationFrame loop for any number of springs and stops
 * as soon as all of them are at rest.
 */
export class SpringSystem {
  private readonly springs: SpringValue[] = [];
  private raf = 0;
  private last = 0;

  /**
   * @param onFrame - Called after every simulation step.
   * @param onRest - Called once when every spring has settled.
   */
  constructor(
    private readonly onFrame: () => void,
    private readonly onRest: () => void = () => {},
  ) {}

  /**
   * Registers a spring.
   *
   * @param spring - Spring to drive.
   * @returns The same spring, for chaining.
   */
  add(spring: SpringValue): SpringValue {
    this.springs.push(spring);
    return spring;
  }

  /** Whether the loop is currently running. */
  get running(): boolean {
    return this.raf !== 0;
  }

  /** Starts the loop if it is not running. */
  kick(): void {
    if (!this.raf) {
      this.last = performance.now();
      this.raf = requestAnimationFrame(this.tick);
    }
  }

  /** Stops the loop without settling (e.g. on unmount). */
  stop(): void {
    cancelAnimationFrame(this.raf);
    this.raf = 0;
  }

  private readonly tick = (now: number): void => {
    const dt = Math.min(0.05, (now - this.last) / 1000) * motionSettings.timeScale;
    this.last = now;
    let moving = false;
    for (const spring of this.springs) {
      if (spring.settled) {
        spring.value = spring.target;
        spring.velocity = 0;
      } else {
        spring.step(dt);
        moving = true;
      }
    }
    this.onFrame();
    if (moving) {
      this.raf = requestAnimationFrame(this.tick);
    } else {
      this.raf = 0;
      this.onRest();
    }
  };
}

function round(value: number, digits: number): number {
  const f = 10 ** digits;
  return Math.round(value * f) / f;
}
