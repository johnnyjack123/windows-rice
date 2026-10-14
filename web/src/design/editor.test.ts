import { describe, expect, it } from 'vitest';
import { adjustEasing, adjustSpring, buildUserThemeCss, fontIdFromStack, fontStack, overridesStylesheet, PILL } from './editor';

describe('theme editor', () => {
  it('writes overrides scoped to the theme, colours per scheme', () => {
    const css = overridesStylesheet({
      monet: { tileRadius: 12, islandRadius: PILL, colors: { light: { accent: '#ff0000' } } },
    });
    const common = css.split('\n')[0] ?? '';
    expect(common.startsWith(':is(:root, .theme-scope)[data-theme="monet"] {')).toBe(true);
    expect(common).toContain('--radius-tile: 12px');
    expect(common).toContain('--radius-tile-small: 12px');
    expect(common).toContain('--radius-island: 999px');
    expect(css).toContain('[data-theme="monet"][data-scheme="light"] { --accent: #ff0000; }');
    expect(css).not.toContain('data-scheme="dark"');
  });

  it('scales speed and bounce while keeping the theme character', () => {
    const base: [number, number] = [0.5, 0.2];
    expect(adjustSpring([0.5, 0.2], '--spring-island', base, { speed: 2 })).toEqual([0.25, 0.2]);
    const control = adjustSpring([0.3, 0.1], '--spring-control', base, { bounce: 0.4 });
    expect(control[1]).toBeCloseTo(0.2);
    // A bounce-free theme gets the absolute value, content stays calmer.
    expect(adjustSpring([0.3, 0], '--spring-content', [0.3, 0], { bounce: 0.4 })[1]).toBeCloseTo(0.12);
  });

  it('interpolates easing curves between linear and the theme', () => {
    expect(adjustEasing('cubic-bezier(0.2, 0, 0, 1)', 1)).toBe('cubic-bezier(0.2, 0, 0, 1)');
    expect(adjustEasing('cubic-bezier(0.2, 0, 0, 1)', 0)).toBe('cubic-bezier(0.25, 0.25, 0.75, 0.75)');
    expect(adjustEasing('steps(3)', 1)).toBe('steps(3)');
  });

  it('maps font ids to stacks and back', () => {
    expect(fontIdFromStack(fontStack('Roboto Flex', 'ui'))).toBe('Roboto Flex');
    expect(fontIdFromStack('"Segoe UI Variable Display", system-ui')).toBe('Segoe UI Variable');
    expect(fontStack('Bahnschrift', 'ui')).toContain('Bahnschrift');
    expect(fontStack('My "Font"', 'ui')).toBe('"My Font", system-ui, sans-serif');
  });

  it('derives a user theme by renaming the selector and baking overrides in', () => {
    const css = buildUserThemeCss(
      { id: 'paper', css: '[data-theme="paper"] { --accent: red; }\n[data-theme="paper"] .tab { color: blue; }' },
      'mein-papier',
      { panelRadius: 30, colors: { light: { background: '#ffffff' } } },
    );
    expect(css).not.toContain('[data-theme="paper"]');
    expect(css).toContain('[data-theme="mein-papier"] .tab');
    expect(css).toContain('--radius-panel: 30px');
    expect(css).toContain('[data-theme="mein-papier"][data-scheme="light"][data-theme="mein-papier"]');
  });
});
