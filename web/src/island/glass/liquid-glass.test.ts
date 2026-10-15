import { describe, expect, it } from 'vitest';
import { LiquidGlass, parseCssColor } from './liquid-glass';

describe('liquid glass', () => {
  it('parses computed colours in every format browsers return', () => {
    expect(parseCssColor('rgb(255, 0, 51)')).toEqual([1, 0, 0.2, 1]);
    expect(parseCssColor('rgba(16, 16, 20, 0.3)')).toEqual([16 / 255, 16 / 255, 20 / 255, 0.3]);
    expect(parseCssColor('rgb(16 16 20 / 30%)')).toEqual([16 / 255, 16 / 255, 20 / 255, 0.3]);
    expect(parseCssColor('color(srgb 0.0627 0.0627 0.0784 / 0.52)')).toEqual([0.0627, 0.0627, 0.0784, 0.52]);
    expect(parseCssColor('oklch(0.5 0.1 200)')).toBeNull();
  });

  it('stays off without WebGL2 so the CSS material takes over', () => {
    const glass = new LiquidGlass(document.createElement('canvas'));
    expect(glass.setEnabled(true)).toBe(false);
    expect(glass.active).toBe(false);
    // Drawing while inactive is a no-op, not an error.
    glass.draw({ main: { x: 0, y: 0, w: 120, h: 34, r: 17 }, bubble: null, liquid: 0, expanded: 0, opacity: 1 });
  });
});
