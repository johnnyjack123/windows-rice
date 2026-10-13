/* Spring physics shared by JS-driven geometry and CSS transitions.
   Planned to become @rice/motion in the real app. */

/**
 * Global motion settings and spring helpers.
 * A spring is described like SwiftUI's `.spring(duration:bounce:)`:
 * `duration` is the perceived duration in seconds, `bounce` ranges from
 * 0 (critically damped) to ~0.4 (clearly springy).
 */
const Motion = {
  /** Playback speed multiplier for every animation (lab slow-motion). */
  timeScale: 1,

  /** When true, springs settle without overshoot and travel is minimised. */
  reduced: false,

  /**
   * Converts designer-facing spring parameters into physical constants for a
   * unit mass.
   *
   * @param {number} duration - Perceived duration in seconds.
   * @param {number} bounce - 0 = no overshoot, positive = springy.
   * @returns {{stiffness: number, damping: number}} Physical constants.
   */
  params(duration, bounce) {
    const d = Math.max(0.05, duration);
    const omega = (2 * Math.PI) / d;
    const zeta = bounce >= 0 ? 1 - bounce : 1 / (1 + bounce);
    return { stiffness: omega * omega, damping: 2 * zeta * omega };
  },

  /**
   * Parses a spring token such as "0.5 0.24". Under reduced motion every
   * spring becomes short and critically damped.
   *
   * @param {string} value - Raw custom property value.
   * @param {number[]} [fallback] - Used when the value cannot be parsed.
   * @returns {number[]} `[duration, bounce]`.
   */
  parseToken(value, fallback = [0.5, 0.2]) {
    const parts = String(value || '').trim().split(/\s+/).map(Number);
    const spring = parts.length >= 2 && parts.every(Number.isFinite) ? parts.slice(0, 2) : fallback;
    if (this.reduced) {
      return [Math.min(spring[0], 0.3), 0];
    }
    return spring;
  },

  /**
   * Reads a spring token from an element's computed style.
   *
   * @param {Element} el - Element whose computed style holds the token.
   * @param {string} name - Custom property name, e.g. "--spring-island".
   * @returns {number[]} `[duration, bounce]`.
   */
  readToken(el, name) {
    return this.parseToken(getComputedStyle(el).getPropertyValue(name));
  },

  /**
   * Simulates a spring from 0 to 1 and returns a CSS `linear()` easing that
   * reproduces it, plus the time it needs to settle. This keeps CSS
   * transitions and JS springs perceptually identical.
   *
   * @param {number} duration - Perceived duration in seconds.
   * @param {number} bounce - Bounce as in {@link Motion.params}.
   * @returns {{easing: string, ms: number}} Easing function and duration.
   */
  toLinear(duration, bounce) {
    const { stiffness, damping } = this.params(duration, bounce);
    const dt = 1 / 600;
    let x = 0;
    let v = 0;
    let t = 0;
    const samples = [];
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
      const index = Math.min(samples.length - 1, Math.round((i / count) * samples.length) - 1);
      points.push(`${samples[Math.max(0, index)].toFixed(4)} ${((i / count) * 100).toFixed(1)}%`);
    }
    points.push('1');
    return { easing: `linear(${points.join(', ')})`, ms: Math.round(t * 1000) };
  },
};

/**
 * A single animated number driven by a damped spring. Retargeting keeps the
 * current velocity, so interrupted animations stay continuous.
 */
class SpringValue {
  /**
   * @param {number} value - Initial value.
   * @param {number[]} spring - `[duration, bounce]`.
   * @param {number} [precision=0.05] - Distance at which the spring snaps to rest.
   */
  constructor(value, spring, precision = 0.05) {
    /** Current value. */
    this.value = value;
    /** Value the spring is moving towards. */
    this.target = value;
    /** Current velocity in units per second. */
    this.velocity = 0;
    /** Rest threshold. */
    this.precision = precision;
    this.setSpring(spring);
  }

  /**
   * Replaces the spring parameters without disturbing motion in flight.
   *
   * @param {number[]} spring - `[duration, bounce]`.
   */
  setSpring(spring) {
    this.config = Motion.params(spring[0], spring[1]);
  }

  /**
   * Moves the target; velocity is preserved.
   *
   * @param {number} target - New target value.
   */
  to(target) {
    this.target = target;
  }

  /**
   * Teleports to a value and stops.
   *
   * @param {number} value - New value and target.
   */
  jump(value) {
    this.value = value;
    this.target = value;
    this.velocity = 0;
  }

  /**
   * Adds velocity, e.g. for a physical "bump" on new content.
   *
   * @param {number} velocity - Velocity to add, in units per second.
   */
  impulse(velocity) {
    this.velocity += velocity;
  }

  /** Whether the spring is at rest on its target. */
  get settled() {
    return Math.abs(this.value - this.target) < this.precision
      && Math.abs(this.velocity) < this.precision * 10;
  }

  /**
   * Advances the simulation. Sub-steps keep stiff springs stable.
   *
   * @param {number} dt - Elapsed time in seconds.
   */
  step(dt) {
    const { stiffness, damping } = this.config;
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
class SpringSystem {
  /**
   * @param {function(): void} onFrame - Called after every simulation step.
   * @param {function(): void} [onRest] - Called once when everything settled.
   */
  constructor(onFrame, onRest = () => {}) {
    this.springs = [];
    this.onFrame = onFrame;
    this.onRest = onRest;
    this.raf = 0;
    this.last = 0;
    this.tick = this.tick.bind(this);
  }

  /**
   * Registers a spring.
   *
   * @param {SpringValue} spring - Spring to drive.
   * @returns {SpringValue} The same spring, for chaining.
   */
  add(spring) {
    this.springs.push(spring);
    return spring;
  }

  /** Starts the loop if it is not running. */
  kick() {
    if (!this.raf) {
      this.last = performance.now();
      this.raf = requestAnimationFrame(this.tick);
    }
  }

  /**
   * @param {number} now - Frame timestamp.
   * @private
   */
  tick(now) {
    const dt = Math.min(0.05, (now - this.last) / 1000) * Motion.timeScale;
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
  }
}
