import { describe, expect, it } from 'vitest';
import { apply, chain, rotateZ, translate } from './mat4';
import { buildVoxels, GROUND, PART, PART_COUNT, SHOULDER } from './model';
import { ONE_SHOT, pose, type MascotMood } from './motion';
import { partMatrices } from './renderer';

const MOODS: MascotMood[] = ['idle', 'ask', 'waiting', 'yes', 'no', 'done'];

describe('mascot model', () => {
  it('has a body, two eyes, two arms and four legs', () => {
    const voxels = buildVoxels();
    const parts = new Set(voxels.map((v) => v.part));
    expect(parts.size).toBe(PART_COUNT);
    for (let leg = 0; leg < 4; leg += 1) {
      expect(voxels.filter((v) => v.part === PART.leg + leg)).toHaveLength(2);
    }
    // Eyes sit one layer behind the front, so they read as sockets.
    const front = Math.max(...voxels.map((v) => v.z));
    expect(voxels.filter((v) => v.part === PART.eyeLeft).every((v) => v.z < front)).toBe(true);
  });
});

describe('mascot motion', () => {
  it('stays finite in every mood', () => {
    for (const mood of MOODS) {
      for (let t = 0; t < 12; t += 0.37) {
        const p = pose(mood, t, t % 2);
        expect(Object.values(p).flat().every(Number.isFinite), `${mood} at ${t}`).toBe(true);
        expect(p.blink).toBeGreaterThanOrEqual(0);
        expect(p.blink).toBeLessThanOrEqual(1);
      }
    }
  });

  it('settles one-shots into idle', () => {
    for (const [mood, duration] of Object.entries(ONE_SHOT)) {
      expect(pose(mood as MascotMood, 3, duration + 0.01)).toEqual(pose('idle', 3, 0));
    }
  });

  it('jumps on yes and turns once', () => {
    const top = pose('yes', 0, 0.16 + 0.31);
    expect(top.y).toBeGreaterThan(2);
    expect(pose('yes', 0, 0.16 + 0.62).yaw).toBeCloseTo(Math.PI * 2, 3);
  });
});

describe('mascot rig', () => {
  const rest = { ...pose('idle', 0, 0), y: 0, squash: 1, yaw: 0, pitch: 0, roll: 0 };

  it('raises arms around the shoulders', () => {
    const lifted = partMatrices({ ...rest, armLeft: 1, armRight: 1 });
    const hand = (side: 'left' | 'right'): [number, number, number] => [SHOULDER[side].x + (side === 'left' ? -2 : 2), SHOULDER[side].y, 1];
    expect(apply(lifted[PART.armLeft]!, hand('left'))[1]).toBeGreaterThan(SHOULDER.left.y + 1);
    expect(apply(lifted[PART.armRight]!, hand('right'))[1]).toBeGreaterThan(SHOULDER.right.y + 1);
  });

  it('keeps the feet on the ground while squashing and lifts single legs', () => {
    const squashed = partMatrices({ ...rest, squash: 0.8 });
    expect(apply(squashed[PART.body]!, [8.5, GROUND, 1.5])[1]).toBeCloseTo(GROUND, 5);
    const tapping = partMatrices({ ...rest, legs: [0.3, 0, 0, 0] });
    expect(apply(tapping[PART.leg]!, [4, 0, 1])[1]).toBeCloseTo(0.3, 5);
    expect(apply(tapping[PART.leg + 1]!, [6, 0, 1])[1]).toBeCloseTo(0, 5);
  });

  it('composes matrices right to left', () => {
    const m = chain(translate(1, 0, 0), rotateZ(Math.PI / 2));
    const [x, y] = apply(m, [1, 0, 0]);
    expect(x).toBeCloseTo(1, 5);
    expect(y).toBeCloseTo(1, 5);
  });
});
