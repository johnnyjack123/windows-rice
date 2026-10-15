/**
 * Theme editor model: the user-adjustable controls, how they map to tokens,
 * how theme defaults are read and how overrides become CSS.
 */
import type { Scheme, ThemeOverrides } from '$backend/contract';
import { parseSpring, type SpringToken } from '$motion/spring';
import { resolveColor } from './color';

/** Keys of all editor controls. */
export type ControlKey =
  | 'background' | 'foreground' | 'accent'
  | 'fontUi' | 'fontDisplay' | 'textScale'
  | 'islandHeight' | 'islandRadius' | 'panelRadius' | 'tileRadius' | 'density'
  | 'speed' | 'bounce' | 'easing' | 'blur' | 'liquid'
  | 'refraction' | 'dispersion' | 'frost'
  | 'iconStyle' | 'iconWeight';

/** Groups shown as sections in the editor. */
export type ControlGroup = 'colors' | 'type' | 'shape' | 'material' | 'motion' | 'icons';

/** Surface material of a theme's island (`--island-material`). */
export type IslandMaterial = 'solid' | 'glass';

/** Definition of one control. */
export interface ControlDef {
  key: ControlKey;
  group: ControlGroup;
  kind: 'color' | 'font' | 'range' | 'toggle' | 'select';
  min?: number;
  max?: number;
  step?: number;
  /** How the value is shown next to the slider. */
  format?: (value: number) => string;
  options?: readonly string[];
  /** Only offered for themes with this island material. */
  material?: IslandMaterial;
}

/** Concrete values of every control. */
export interface EditorValues {
  background: string;
  foreground: string;
  accent: string;
  fontUi: string;
  fontDisplay: string;
  textScale: number;
  islandHeight: number;
  islandRadius: number;
  panelRadius: number;
  tileRadius: number;
  density: number;
  speed: number;
  bounce: number;
  easing: number;
  blur: number;
  liquid: boolean;
  refraction: number;
  dispersion: number;
  frost: number;
  iconStyle: 'Rounded' | 'Sharp' | 'Outlined';
  iconWeight: number;
}

const px = (v: number): string => `${Math.round(v)} px`;
const pct = (v: number): string => `${Math.round(v * 100)} %`;
const times = (v: number): string => `${v.toFixed(2).replace(/0$/, '')}×`;

/** Radius values at or above this mean "pill" (full rounding). */
export const PILL = 999;

/** Every control, in display order. */
export const CONTROLS: readonly ControlDef[] = [
  { key: 'background', group: 'colors', kind: 'color' },
  { key: 'foreground', group: 'colors', kind: 'color' },
  { key: 'accent', group: 'colors', kind: 'color' },
  { key: 'fontUi', group: 'type', kind: 'font' },
  { key: 'fontDisplay', group: 'type', kind: 'font' },
  { key: 'textScale', group: 'type', kind: 'range', min: 0.85, max: 1.3, step: 0.05, format: pct },
  { key: 'islandHeight', group: 'shape', kind: 'range', min: 28, max: 44, step: 1, format: px },
  { key: 'islandRadius', group: 'shape', kind: 'range', min: 0, max: 22, step: 1, format: px },
  { key: 'panelRadius', group: 'shape', kind: 'range', min: 0, max: 48, step: 1, format: px },
  { key: 'tileRadius', group: 'shape', kind: 'range', min: 0, max: 34, step: 1, format: px },
  { key: 'density', group: 'shape', kind: 'range', min: 0.85, max: 1.2, step: 0.05, format: pct },
  { key: 'speed', group: 'motion', kind: 'range', min: 0.5, max: 2, step: 0.05, format: times },
  { key: 'bounce', group: 'motion', kind: 'range', min: 0, max: 0.5, step: 0.01, format: pct },
  { key: 'easing', group: 'motion', kind: 'range', min: 0, max: 1.5, step: 0.05, format: pct },
  { key: 'blur', group: 'motion', kind: 'range', min: 0, max: 16, step: 1, format: px },
  { key: 'liquid', group: 'motion', kind: 'toggle' },
  { key: 'refraction', group: 'material', kind: 'range', min: 0, max: 2, step: 0.05, format: pct, material: 'glass' },
  { key: 'dispersion', group: 'material', kind: 'range', min: 0, max: 1, step: 0.05, format: pct, material: 'glass' },
  { key: 'frost', group: 'material', kind: 'range', min: 0, max: 1, step: 0.05, format: pct, material: 'glass' },
  { key: 'iconStyle', group: 'icons', kind: 'select', options: ['Rounded', 'Sharp', 'Outlined'] },
  { key: 'iconWeight', group: 'icons', kind: 'range', min: 100, max: 700, step: 50, format: (v) => String(v) },
];

/** A font the editor offers, with stacks for UI text and numerals. */
export interface FontOption {
  id: string;
  ui: string;
  display: string;
}

/** Fonts that ship with rice (or with every Windows 11). */
export const BUNDLED_FONTS: readonly FontOption[] = [
  {
    id: 'Segoe UI Variable',
    ui: '"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif',
    display: '"Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif',
  },
  { id: 'Roboto Flex', ui: '"Roboto Flex Variable", system-ui, sans-serif', display: '"Roboto Flex Variable", system-ui, sans-serif' },
  { id: 'Instrument Sans', ui: '"Instrument Sans Variable", system-ui, sans-serif', display: '"Instrument Sans Variable", system-ui, sans-serif' },
  { id: 'Instrument Serif', ui: '"Instrument Serif", Georgia, serif', display: '"Instrument Serif", Georgia, serif' },
  { id: 'JetBrains Mono', ui: '"JetBrains Mono Variable", Consolas, monospace', display: '"JetBrains Mono Variable", Consolas, monospace' },
  { id: 'Bahnschrift', ui: 'Bahnschrift, system-ui, sans-serif', display: 'Bahnschrift, system-ui, sans-serif' },
];

/**
 * CSS font stack for a font id (bundled or installed).
 *
 * @param id - Font id or installed family name.
 * @param role - UI text or display numerals.
 * @returns A font-family value.
 */
export function fontStack(id: string, role: 'ui' | 'display'): string {
  const bundled = BUNDLED_FONTS.find((f) => f.id === id);
  if (bundled) {
    return bundled[role];
  }
  return `"${id.replace(/"/g, '')}", system-ui, sans-serif`;
}

/**
 * Maps the first family of a stack back to a font id.
 *
 * @param stack - A font-family value.
 * @returns Matching bundled id, or the family name itself.
 */
export function fontIdFromStack(stack: string): string {
  const first = (stack.split(',')[0] ?? '').trim().replace(/^["']|["']$/g, '');
  const bundled = BUNDLED_FONTS.find((f) => {
    const ui = (f.ui.split(',')[0] ?? '').replace(/"/g, '').trim();
    const display = (f.display.split(',')[0] ?? '').replace(/"/g, '').trim();
    return ui === first || display === first;
  });
  return bundled?.id ?? first;
}

/** Spring tokens the motion controls act on. */
export const SPRING_TOKENS = ['--spring-island', '--spring-bubble', '--spring-content', '--spring-control'] as const;

/**
 * Reads the default editor values of a theme by rendering an invisible probe
 * with only the theme applied (no user overrides).
 *
 * @param themeId - Theme to inspect.
 * @param scheme - Colour scheme.
 * @returns The theme's own values.
 */
export function readThemeDefaults(themeId: string, scheme: Scheme): EditorValues {
  return withProbe(themeId, scheme, (probe, cs) => {
    const token = (name: string): string => cs.getPropertyValue(name).trim();
    const length = (name: string): number => {
      const el = document.createElement('div');
      el.style.width = `var(${name})`;
      probe.append(el);
      const value = parseFloat(getComputedStyle(el).width);
      el.remove();
      return Number.isFinite(value) ? value : 0;
    };
    const number = (name: string, fallback: number): number => {
      const value = parseFloat(token(name));
      return Number.isFinite(value) ? value : fallback;
    };
    const islandHeight = length('--island-height');
    const islandRadius = length('--radius-island');
    const tileRadius = length('--radius-tile');
    const family = token('--icon-family').replace(/["']/g, '');
    return {
      background: resolveColor(probe, 'var(--base-bg)') ?? '#000000',
      foreground: resolveColor(probe, 'var(--base-fg)') ?? '#ffffff',
      accent: resolveColor(probe, 'var(--accent)') ?? '#0a84ff',
      fontUi: fontIdFromStack(token('--font-ui')),
      fontDisplay: fontIdFromStack(token('--font-numeric')),
      textScale: number('--text-scale', 1),
      islandHeight,
      islandRadius: islandRadius >= islandHeight / 2 ? PILL : islandRadius,
      panelRadius: length('--radius-panel'),
      tileRadius: tileRadius >= PILL ? PILL : tileRadius,
      density: number('--density', 1),
      speed: 1,
      bounce: parseSpring(token('--spring-island'))[1],
      easing: 1,
      blur: length('--content-blur'),
      liquid: token('--island-goo') === '1',
      refraction: number('--glass-refraction', 1),
      dispersion: number('--glass-dispersion', 0.3),
      frost: number('--glass-frost', 0.8),
      iconStyle: family.endsWith('Sharp') ? 'Sharp' : family.endsWith('Outlined') ? 'Outlined' : 'Rounded',
      iconWeight: number('--icon-weight', 400),
    };
  });
}

/**
 * Reads which island material a theme uses.
 *
 * @param themeId - Theme to inspect.
 * @param scheme - Colour scheme.
 * @returns "glass" for refractive themes, otherwise "solid".
 */
export function readThemeMaterial(themeId: string, scheme: Scheme): IslandMaterial {
  return withProbe(themeId, scheme, (_probe, cs) => (cs.getPropertyValue('--island-material').trim() === 'glass' ? 'glass' : 'solid'));
}

/**
 * Renders an invisible element with only a theme applied (no user
 * overrides, which target `:root` and `.theme-scope` only) and reads it.
 *
 * @param themeId - Theme to apply.
 * @param scheme - Colour scheme.
 * @param read - Reads values from the probe.
 * @returns Whatever `read` returns.
 */
function withProbe<T>(themeId: string, scheme: Scheme, read: (probe: HTMLElement, cs: CSSStyleDeclaration) => T): T {
  const probe = document.createElement('div');
  probe.dataset.theme = themeId;
  probe.dataset.scheme = scheme;
  probe.setAttribute('aria-hidden', 'true');
  probe.style.cssText = 'position:absolute;left:-9999px;top:0;visibility:hidden;pointer-events:none;';
  document.body.append(probe);
  try {
    return read(probe, getComputedStyle(probe));
  } finally {
    probe.remove();
  }
}

const MOTION_DURATIONS = ['--dur-instant', '--dur-fast', '--dur-medium', '--dur-slow', '--stagger', '--enter-delay-grow', '--enter-delay-shrink'];
const MOTION_EASES = ['--ease-standard', '--ease-enter', '--ease-exit'];

/**
 * Bakes speed, bounce and easing overrides into token declarations, based on
 * a theme's own motion tokens.
 *
 * @param themeId - Base theme.
 * @param o - Overrides.
 * @returns CSS declarations.
 */
export function motionDeclarations(themeId: string, o: ThemeOverrides): string[] {
  if (o.speed === undefined && o.bounce === undefined && o.easing === undefined) {
    return [];
  }
  return withProbe(themeId, 'dark', (_probe, cs) => {
    const d: string[] = [];
    const base = parseSpring(cs.getPropertyValue('--spring-island'), undefined, false);
    for (const name of SPRING_TOKENS) {
      const raw = parseSpring(cs.getPropertyValue(name), undefined, false);
      const [duration, bounce] = adjustSpring(raw, name, base, o);
      d.push(`${name}: ${Number(duration.toFixed(3))} ${Number(bounce.toFixed(3))}`);
    }
    if (o.speed !== undefined) {
      for (const name of MOTION_DURATIONS) {
        const ms = parseFloat(cs.getPropertyValue(name));
        if (Number.isFinite(ms)) {
          d.push(`${name}: ${Math.round(ms / o.speed)}ms`);
        }
      }
    }
    if (o.easing !== undefined) {
      for (const name of MOTION_EASES) {
        d.push(`${name}: ${adjustEasing(cs.getPropertyValue(name), o.easing)}`);
      }
    }
    return d;
  });
}

/**
 * Builds the CSS of a user theme derived from a base theme plus editor
 * overrides. The result is self-contained: the overrides are baked in.
 *
 * @param base - Base theme (its source CSS and id).
 * @param id - New theme id.
 * @param o - Overrides to bake in.
 * @returns Complete theme.css text.
 */
export function buildUserThemeCss(base: { id: string; css: string }, id: string, o: ThemeOverrides): string {
  const scope = `[data-theme="${cssEscape(id)}"]`;
  const css = base.css.split(`[data-theme="${base.id}"]`).join(scope);
  const blocks: string[] = [];
  const common = [...commonDeclarations(o), ...motionDeclarations(base.id, o)];
  if (common.length) {
    blocks.push(`${scope} {\n  ${common.join(';\n  ')};\n}`);
  }
  for (const scheme of ['dark', 'light'] as const) {
    const colors = colorDeclarations(o, scheme);
    if (colors.length) {
      // Two attribute selectors beat the base theme's scheme blocks.
      blocks.push(`${scope}[data-scheme="${scheme}"]${scope} {\n  ${colors.join(';\n  ')};\n}`);
    }
  }
  return `${css.trim()}\n\n/* Customisations baked in by the rice theme editor (based on "${base.id}"). */\n${blocks.join('\n')}\n`;
}

/**
 * Turns the non-colour overrides into token declarations.
 *
 * @param o - Overrides of one theme.
 * @returns CSS declarations ("--token: value").
 */
export function commonDeclarations(o: ThemeOverrides): string[] {
  const d: string[] = [];
  if (o.fontUi) {
    d.push(`--font-ui: ${fontStack(o.fontUi, 'ui')}`);
  }
  if (o.fontDisplay) {
    const stack = fontStack(o.fontDisplay, 'display');
    d.push(`--font-display: ${stack}`, `--font-numeric: ${stack}`);
  }
  if (o.textScale !== undefined) {
    d.push(`--text-scale: ${o.textScale}`);
  }
  if (o.islandHeight !== undefined) {
    d.push(`--island-height: ${o.islandHeight}px`);
  }
  if (o.islandRadius !== undefined) {
    d.push(`--radius-island: ${o.islandRadius >= PILL ? 999 : o.islandRadius}px`);
  }
  if (o.panelRadius !== undefined) {
    d.push(`--radius-panel: ${o.panelRadius}px`, `--radius-live: ${Math.round(o.panelRadius * 0.75)}px`);
  }
  if (o.tileRadius !== undefined) {
    const r = o.tileRadius >= PILL ? 999 : o.tileRadius;
    d.push(`--radius-tile: ${r}px`, `--radius-tile-small: ${r}px`);
  }
  if (o.density !== undefined) {
    d.push(`--density: ${o.density}`);
  }
  if (o.blur !== undefined) {
    d.push(`--content-blur: ${o.blur}px`);
  }
  if (o.liquid !== undefined) {
    d.push(`--island-goo: ${o.liquid ? 1 : 0}`);
  }
  if (o.refraction !== undefined) {
    d.push(`--glass-refraction: ${o.refraction}`);
  }
  if (o.dispersion !== undefined) {
    d.push(`--glass-dispersion: ${o.dispersion}`);
  }
  if (o.frost !== undefined) {
    d.push(`--glass-frost: ${o.frost}`);
  }
  if (o.iconStyle) {
    d.push(`--icon-family: "Material Symbols ${o.iconStyle}"`);
  }
  if (o.iconWeight !== undefined) {
    d.push(`--icon-weight: ${o.iconWeight}`);
  }
  return d;
}

/**
 * Colour overrides of one scheme as token declarations.
 *
 * @param o - Overrides of one theme.
 * @param scheme - Scheme to read.
 * @returns CSS declarations.
 */
export function colorDeclarations(o: ThemeOverrides, scheme: Scheme): string[] {
  const c = o.colors?.[scheme];
  const d: string[] = [];
  if (c?.background) {
    d.push(`--base-bg: ${c.background}`);
  }
  if (c?.foreground) {
    d.push(`--base-fg: ${c.foreground}`);
  }
  if (c?.accent) {
    d.push(`--accent: ${c.accent}`);
  }
  return d;
}

/**
 * Builds the stylesheet that applies user overrides on top of themes. It
 * targets the document root and `.theme-scope` previews.
 *
 * @param all - Overrides keyed by theme id.
 * @returns CSS text.
 */
export function overridesStylesheet(all: Record<string, ThemeOverrides>): string {
  const rules: string[] = [];
  for (const [id, o] of Object.entries(all)) {
    const scope = `:is(:root, .theme-scope)[data-theme="${cssEscape(id)}"]`;
    const common = commonDeclarations(o);
    if (common.length) {
      rules.push(`${scope} { ${common.join('; ')}; }`);
    }
    for (const scheme of ['dark', 'light'] as const) {
      const colors = colorDeclarations(o, scheme);
      if (colors.length) {
        rules.push(`${scope}[data-scheme="${scheme}"] { ${colors.join('; ')}; }`);
      }
    }
  }
  return rules.join('\n');
}

/** Motion values read from a theme before user adjustments. */
export interface MotionBase {
  springs: Record<(typeof SPRING_TOKENS)[number], SpringToken>;
}

/**
 * Applies speed and bounce to a spring. Bounce is absolute for the island
 * spring; the others scale proportionally so the theme's character stays.
 *
 * @param spring - Theme spring.
 * @param name - Token name.
 * @param base - The theme's island spring.
 * @param o - User overrides.
 * @returns Adjusted spring.
 */
export function adjustSpring(
  spring: SpringToken,
  name: (typeof SPRING_TOKENS)[number],
  base: SpringToken,
  o: Pick<ThemeOverrides, 'speed' | 'bounce'>,
): SpringToken {
  const speed = o.speed ?? 1;
  let bounce = spring[1];
  if (o.bounce !== undefined) {
    if (base[1] > 0.01) {
      bounce = spring[1] * (o.bounce / base[1]);
    } else {
      bounce = name === '--spring-content' ? o.bounce * 0.3 : o.bounce;
    }
  }
  const cap = name === '--spring-content' ? 0.3 : 0.6;
  return [spring[0] / speed, Math.max(0, Math.min(cap, bounce))];
}

/**
 * Interpolates a cubic-bezier between linear (strength 0), the theme's curve
 * (1) and an exaggerated version (1.5).
 *
 * @param curve - Theme curve, e.g. "cubic-bezier(0.2, 0, 0, 1)".
 * @param strength - 0…1.5.
 * @returns Adjusted curve, or the input if it is not a cubic-bezier.
 */
export function adjustEasing(curve: string, strength: number): string {
  const m = /cubic-bezier\(([^)]+)\)/.exec(curve);
  if (!m) {
    return curve;
  }
  const p = m[1]!.split(',').map(Number);
  const linear = [0.25, 0.25, 0.75, 0.75];
  const out = p.map((v, i) => {
    const value = linear[i]! + strength * (v - linear[i]!);
    // x coordinates must stay within 0…1
    return i % 2 === 0 ? Math.min(1, Math.max(0, value)) : value;
  });
  return `cubic-bezier(${out.map((v) => Number(v.toFixed(3))).join(', ')})`;
}

function cssEscape(value: string): string {
  return value.replace(/["\\]/g, '\\$&');
}
