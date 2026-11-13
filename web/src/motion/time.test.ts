import { describe, expect, it } from 'vitest';
import { cssTimeMs } from './time';

describe('cssTimeMs', () => {
  it('reads milliseconds and seconds', () => {
    expect(cssTimeMs('1600ms')).toBe(1600);
    expect(cssTimeMs('8s')).toBe(8000);
    expect(cssTimeMs(' 0.4s ')).toBe(400);
    expect(cssTimeMs('.25S')).toBe(250);
  });

  it('rejects values that are not times', () => {
    expect(cssTimeMs('')).toBeNull();
    expect(cssTimeMs('8')).toBeNull();
    expect(cssTimeMs('8px')).toBeNull();
    expect(cssTimeMs('calc(1s * 2)')).toBeNull();
  });
});
