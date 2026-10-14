import { describe, expect, it } from 'vitest';
import { formatSpring, parseSpring, springConstants, springToLinear, SpringValue } from './spring';

describe('springs', () => {
  it('parses tokens and falls back on garbage', () => {
    expect(parseSpring('0.5 0.24')).toEqual([0.5, 0.24]);
    expect(parseSpring('nonsense', [0.3, 0])).toEqual([0.3, 0]);
    expect(formatSpring([0.5, 0.24])).toBe('0.5 0.24');
  });

  it('is critically damped at bounce 0', () => {
    const { stiffness, damping } = springConstants(0.5, 0);
    expect(damping).toBeCloseTo(2 * Math.sqrt(stiffness), 6);
  });

  it('settles on its target and keeps velocity when retargeted', () => {
    const s = new SpringValue(0, [0.4, 0.2]);
    s.to(100);
    for (let i = 0; i < 20; i++) {
      s.step(1 / 60);
    }
    const velocity = s.velocity;
    s.to(50);
    expect(s.velocity).toBe(velocity);
    for (let i = 0; i < 400 && !s.settled; i++) {
      s.step(1 / 60);
    }
    expect(s.settled).toBe(true);
    expect(s.value).toBeCloseTo(50, 0);
  });

  it('overshoots with bounce and generates a matching linear() curve', () => {
    const { easing, ms } = springToLinear([0.5, 0.3]);
    expect(easing.startsWith('linear(0, ')).toBe(true);
    expect(easing.endsWith(', 1)')).toBe(true);
    const values = [...easing.matchAll(/([\d.]+) [\d.]+%/g)].map((m) => Number(m[1]));
    expect(Math.max(...values)).toBeGreaterThan(1);
    expect(ms).toBeGreaterThan(300);
  });
});
