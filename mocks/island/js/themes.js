/* Theme registry and runtime theme engine. In the real app the registry
   comes from theme manifests (theme.toml) found on disk. */

/**
 * Available themes. `schemes` lists supported colour schemes, the first one
 * is the default.
 */
const THEMES = [
  {
    id: 'obsidian',
    name: 'Obsidian',
    schemes: ['dark'],
    tagline: 'Apple-nah. Reines Schwarz, kräftige Federn, flüssiges Abspalten der zweiten Aktivität.',
  },
  {
    id: 'monet',
    name: 'Monet',
    schemes: ['dark', 'light'],
    tagline: 'Material 3 Expressive. Tonale Farben aus Wallpaper und Albumcover, Formen, die sich verwandeln.',
  },
  {
    id: 'glass',
    name: 'Liquid Glass',
    schemes: ['dark', 'light'],
    tagline: 'Lichtbrechendes Glas über dem Wallpaper, Glanzkante folgt dem Mauszeiger, gelartige Bewegung.',
  },
  {
    id: 'terminal',
    name: 'Terminal',
    schemes: ['dark'],
    tagline: 'Monospace, Haarlinien, Bernstein. Dicht und präzise, ohne Federn, Inhalte „rendern“ sich ein.',
  },
  {
    id: 'paper',
    name: 'Paper',
    schemes: ['light'],
    tagline: 'Editorial. Warmes Papier, Serifen-Ziffern, Tinte und Zinnoberrot, ruhige Bewegung.',
  },
];

/** Duration tokens that are rescaled for slow-motion playback. */
const SCALED_DURATIONS = [
  '--dur-instant', '--dur-fast', '--dur-medium', '--dur-slow',
  '--stagger', '--enter-delay-grow', '--enter-delay-shrink',
];

/** Spring tokens that get a generated CSS linear() counterpart. */
const CSS_SPRINGS = ['content', 'control'];

/**
 * Applies themes to the document and keeps runtime-derived motion tokens
 * (spring easings, slow-motion durations) in sync.
 */
const ThemeEngine = {
  root: document.documentElement,
  listeners: [],

  /** Currently applied theme id. */
  get theme() {
    return this.root.dataset.theme;
  },

  /** Currently applied colour scheme. */
  get scheme() {
    return this.root.dataset.scheme;
  },

  /**
   * Looks up theme metadata.
   *
   * @param {string} id - Theme id.
   * @returns {object} Theme entry from {@link THEMES}.
   */
  meta(id) {
    return THEMES.find((t) => t.id === id) || THEMES[0];
  },

  /**
   * Applies a theme and scheme. Unsupported schemes fall back to the theme's
   * default scheme.
   *
   * @param {string} id - Theme id.
   * @param {string} [scheme] - Requested colour scheme.
   */
  apply(id, scheme) {
    const meta = this.meta(id);
    const resolved = meta.schemes.includes(scheme) ? scheme : meta.schemes[0];
    this.root.dataset.theme = meta.id;
    this.root.dataset.scheme = resolved;
    this.refreshMotion();
    for (const fn of this.listeners) {
      fn(meta, resolved);
    }
  },

  /**
   * Registers a callback for theme changes.
   *
   * @param {function(object, string): void} fn - Receives theme meta and scheme.
   */
  onChange(fn) {
    this.listeners.push(fn);
  },

  /**
   * Reads a token from the root's computed style.
   *
   * @param {string} name - Custom property name.
   * @returns {string} Trimmed value.
   */
  token(name) {
    return getComputedStyle(this.root).getPropertyValue(name).trim();
  },

  /**
   * Regenerates CSS spring easings from the theme's spring tokens and
   * rescales durations for the current {@link Motion.timeScale}.
   */
  refreshMotion() {
    const style = this.root.style;
    for (const name of SCALED_DURATIONS) {
      style.removeProperty(name);
    }
    const computed = getComputedStyle(this.root);
    const slow = 1 / Motion.timeScale;
    if (slow !== 1) {
      for (const name of SCALED_DURATIONS) {
        const ms = parseFloat(computed.getPropertyValue(name));
        if (Number.isFinite(ms)) {
          style.setProperty(name, `${ms * slow}ms`);
        }
      }
    }
    for (const name of CSS_SPRINGS) {
      const [duration, bounce] = Motion.parseToken(computed.getPropertyValue(`--spring-${name}`));
      const { easing, ms } = Motion.toLinear(duration, bounce);
      style.setProperty(`--ease-spring-${name}`, easing);
      style.setProperty(`--dur-spring-${name}`, `${Math.round(ms * slow)}ms`);
    }
  },
};
