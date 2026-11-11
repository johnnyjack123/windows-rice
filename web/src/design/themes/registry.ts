import type { Scheme } from '$backend/contract';
import obsidian from './obsidian.css?raw';
import monet from './monet.css?raw';
import glass from './glass.css?raw';
import terminal from './terminal.css?raw';
import paper from './paper.css?raw';
import lcars from './lcars.css?raw';
import workbench from './workbench.css?raw';
import cyberpunk from './cyberpunk.css?raw';

/** Metadata of a theme (built-in or from the themes folder). */
export interface ThemeMeta {
  id: string;
  name: string;
  author: string;
  schemes: Scheme[];
  builtIn: boolean;
  /** Source CSS; used when deriving a user theme from it. */
  css: string;
}

/** Themes shipped with the app. Their CSS is also bundled globally. */
export const BUILTIN_THEMES: ThemeMeta[] = [
  { id: 'obsidian', name: 'Obsidian', author: 'rice', schemes: ['dark'], builtIn: true, css: obsidian },
  { id: 'monet', name: 'Monet', author: 'rice', schemes: ['dark', 'light'], builtIn: true, css: monet },
  { id: 'glass', name: 'Liquid Glass', author: 'rice', schemes: ['dark', 'light'], builtIn: true, css: glass },
  { id: 'terminal', name: 'Terminal', author: 'rice', schemes: ['dark'], builtIn: true, css: terminal },
  { id: 'paper', name: 'Paper', author: 'rice', schemes: ['light'], builtIn: true, css: paper },
  { id: 'lcars', name: 'LCARS', author: 'rice', schemes: ['dark'], builtIn: true, css: lcars },
  { id: 'workbench', name: 'Workbench 1.3', author: 'rice', schemes: ['dark'], builtIn: true, css: workbench },
  { id: 'cyberpunk', name: 'Cyberpunk 2077', author: 'rice', schemes: ['dark', 'light'], builtIn: true, css: cyberpunk },
];

/**
 * Turns a display name into a theme id usable as folder name and selector.
 *
 * @param name - Display name.
 * @returns Lowercase kebab-case id.
 */
export function themeIdFrom(name: string): string {
  const id = name
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
  return id || 'custom-theme';
}
