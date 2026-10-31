/**
 * Moods and poses of the mascot: pure functions of time, so the renderer only
 * evaluates them per frame and tests can check every mood.
 */

/** What the mascot expresses. One-shots (`yes`, `no`, `done`) settle into `idle`. */
export type MascotMood = 'idle' | 'ask' | 'waiting' | 'yes' | 'no' | 'done';

/** Pose of one frame (grid units and radians). */
export interface Pose {
  /** Lift of the whole figure. */
  y: number;
  /** Vertical scale around the feet (volume kept: 1 / sqrt sideways). */
  squash: number;
  yaw: number;
  pitch: number;
  roll: number;
  /** Eye height, 1 open … 0 closed. */
  blink: number;
  /** Arm angles, positive raises the arm. */
  armLeft: number;
  armRight: number;
  /** Lift of each leg, left to right. */
  legs: [number, number, number, number];
}

/** How long the one-shot moods play (s). */
export const ONE_SHOT: Partial<Record<MascotMood, number>> = { yes: 0.95, no: 0.75, done: 1.1 };

const TAU = Math.PI * 2;
/** The figure looks slightly down at the viewer from the island. */
const BASE_PITCH = 0.16;

const clamp01 = (v: number): number => Math.min(1, Math.max(0, v));
const smooth = (v: number): number => {
  const t = clamp01(v);
  return t * t * (3 - 2 * t);
};

/** Parabolic hop over `duration` seconds: 0 → height → 0. */
function arc(t: number, duration: number, height: number): number {
  const p = clamp01(t / duration);
  return 4 * height * p * (1 - p);
}

/**
 * Squash and stretch of one hop: crouch (anticipation), stretch on the way up,
 * squash on landing.
 */
function hopSquash(t: number, crouch: number, air: number): number {
  if (t < crouch) {
    return 1 - 0.16 * Math.sin((t / crouch) * Math.PI);
  }
  const inAir = t - crouch;
  if (inAir < air) {
    return 1 + 0.12 * Math.sin((inAir / air) * Math.PI) * (1 - inAir / air);
  }
  const landed = inAir - air;
  return landed < 0.16 ? 1 - 0.12 * Math.sin((landed / 0.16) * Math.PI) : 1;
}

/** Eyes close for 140 ms every ~3.4 s (a second blink now and then). */
function blinkAt(t: number): number {
  const phase = t % 3.4;
  const one = phase < 0.14 ? Math.abs(Math.cos((phase / 0.14) * Math.PI)) : 1;
  const second = Math.floor(t / 3.4) % 3 === 2 && phase > 0.26 && phase < 0.4 ? Math.abs(Math.cos(((phase - 0.26) / 0.14) * Math.PI)) : 1;
  return Math.min(one, second);
}

/** Calm breathing and looking around. */
function idle(t: number): Pose {
  return {
    y: 0.2 * Math.sin((t * TAU) / 2.6),
    squash: 1 + 0.025 * Math.sin((t * TAU) / 1.3),
    yaw: 0.32 * Math.sin((t * TAU) / 7),
    pitch: BASE_PITCH,
    roll: 0.04 * Math.sin((t * TAU) / 5.2),
    blink: blinkAt(t),
    armLeft: 0.1 + 0.08 * Math.sin((t * TAU) / 2.6),
    armRight: 0.1 + 0.08 * Math.sin((t * TAU) / 2.6 + 1),
    legs: [0, 0, 0, 0],
  };
}

/** Asking: curious head tilt, a hop every 1.9 s, one arm waving. */
function ask(t: number): Pose {
  const cycle = t % 1.9;
  const crouch = 0.18;
  const air = 0.42;
  const lift = cycle > crouch ? arc(cycle - crouch, air, 2) : 0;
  const tap = (offset: number): number => Math.max(0, Math.sin(((t + offset) * TAU) / 0.5)) * 0.6;
  return {
    y: lift,
    squash: hopSquash(cycle, crouch, air),
    yaw: 0.22 * Math.sin((t * TAU) / 3.8),
    pitch: BASE_PITCH,
    roll: 0.16 * Math.sin((t * TAU) / 3.6),
    blink: blinkAt(t + 1.1),
    armLeft: 0.18,
    armRight: 1.05 + 0.4 * Math.sin(t * TAU * 1.6),
    legs: lift > 0.1 ? [0.4, 0.4, 0.4, 0.4] : [tap(0), tap(0.25), tap(0.12), tap(0.37)],
  };
}

/** Waiting for input: looks left, centre, right, with pauses. */
function waiting(t: number): Pose {
  const base = idle(t);
  const step = Math.floor(t / 1.6) % 4;
  const target = [0, -0.65, 0, 0.65][step]!;
  const previous = [0.65, 0, -0.65, 0][step]!;
  const look = previous + (target - previous) * smooth((t % 1.6) / 0.35);
  return { ...base, yaw: look, roll: 0.08 * Math.sin((t * TAU) / 3.2) };
}

/**
 * Pose of `mood` at time `t`.
 *
 * @param mood - Current mood.
 * @param t - Seconds since the mascot appeared (drives the loops).
 * @param since - Seconds since the mood started (drives the one-shots).
 * @returns The pose to render.
 */
export function pose(mood: MascotMood, t: number, since: number): Pose {
  const duration = ONE_SHOT[mood];
  if (duration !== undefined && since >= duration) {
    return idle(t);
  }
  switch (mood) {
    case 'ask':
      return ask(t);
    case 'waiting':
      return waiting(t);
    case 'yes': {
      // Crouch, then a big jump with a full turn, arms up.
      const crouch = 0.16;
      const air = 0.62;
      const inAir = clamp01((since - crouch) / air);
      return {
        ...idle(t),
        y: since > crouch ? arc(since - crouch, air, 3.2) : 0,
        squash: hopSquash(since, crouch, air),
        yaw: TAU * smooth(inAir),
        armLeft: 1.25 * smooth(since / 0.2),
        armRight: 1.25 * smooth(since / 0.2),
        legs: [0.5, 0.5, 0.5, 0.5],
        blink: since > crouch && since < crouch + air ? 0.25 : 1,
      };
    }
    case 'no': {
      // Shakes its head, sinks a little, arms down.
      const fade = 1 - since / duration!;
      return {
        ...idle(t),
        y: -0.3 * Math.sin(clamp01(since / duration!) * Math.PI),
        yaw: 0.5 * Math.sin(since * TAU * 3.6) * fade,
        armLeft: -0.35,
        armRight: -0.35,
        blink: 0.55,
      };
    }
    case 'done': {
      // Two happy hops, arms up.
      const first = since < 0.5;
      const local = first ? since : since - 0.5;
      return {
        ...idle(t),
        y: arc(local, 0.45, first ? 1.6 : 1.1),
        squash: hopSquash(local + 0.12, 0.12, 0.45),
        armLeft: 1.1,
        armRight: 1.1,
      };
    }
    default:
      return idle(t);
  }
}
