/* Lab panel: switches concepts, schemes, accents and wallpapers and fires
   simulated system events. Not part of the product. */

const LAB_STORE = 'rice-island-lab';

const ACCENTS = ['#0a84ff', '#30b07a', '#ff9f0a', '#e5484d', '#a78bfa', '#14b8c4'];

const LAB_EVENTS = [
  { key: 'n', label: 'Mitteilung', run: (app) => app.pushNotification() },
  { key: 'v', label: 'Lautstärke', run: (app) => app.setVolume(app.state.volume + (Math.random() > 0.5 ? 0.12 : -0.12), true) },
  { key: 'b', label: 'Helligkeit', run: (app) => app.setBrightness(app.state.brightness + (Math.random() > 0.5 ? 0.15 : -0.15), true) },
  { key: 'm', label: 'Play / Pause', run: (app) => app.togglePlay() },
  { key: '.', label: 'Nächster Titel', run: (app) => app.skip(1) },
  { key: 'e', label: 'Termin', run: (app) => app.triggerEvent() },
  { key: 't', label: 'Timer (Split)', run: (app) => app.startTimer() },
  { key: 'c', label: 'Laden', run: (app) => app.toggleCharging() },
];

const TEMPOS = [1, 0.5, 0.2];

/**
 * Lab controller.
 */
const Lab = {
  settings: { theme: 'obsidian', scheme: 'dark', wall: 'dunes', accent: null },
  blobUrls: [],

  /**
   * Builds the lab panel and binds shortcuts.
   *
   * @param {object} app - The App object.
   */
  init(app) {
    this.app = app;
    this.el = document.querySelector('.lab');
    this.stage = document.querySelector('.stage');
    this.toastEl = document.querySelector('.lab-toast');
    this.el.innerHTML = this.template();
    this.bind();
    window.addEventListener('resize', () => {
      clearTimeout(this.resizeTimer);
      this.resizeTimer = setTimeout(() => this.prepareGlass(this.wallSrc), 300);
    });
  },

  /** @returns {string} Lab markup. @private */
  template() {
    const concepts = THEMES.map((t, i) => `
      <button class="lab-concept" data-theme-id="${t.id}">
        <kbd>${i + 1}</kbd><span class="lab-concept__name">${t.name}</span>
        <span class="lab-concept__tag">${t.tagline}</span>
      </button>`).join('');
    const swatches = ACCENTS.map((c) => `<button class="lab-swatch" data-accent="${c}" style="--sw:${c}" aria-label="Akzent ${c}"></button>`).join('');
    const walls = Art.wallpapers.map((w) => `
      <button class="lab-wall" data-wall="${w.id}" style="--thumb:url('${Art.svgUrl(w.svg)}')" aria-label="Wallpaper ${w.name}" title="${w.name}"></button>`).join('');
    const events = LAB_EVENTS.map((e) => `<button class="lab-btn" data-event="${e.key}">${e.label}<kbd>${e.key === '.' ? '.' : e.key.toUpperCase()}</kbd></button>`).join('');
    return `
      <div class="lab__head">
        <div class="lab__title">Island Lab <span>· rice</span></div>
        <button class="lab__collapse" data-collapse aria-label="Lab einklappen">
          <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"><path d="M4 6l4 4 4-4"/></svg>
        </button>
      </div>
      <div class="lab__body">
        <h3>Konzept</h3>
        <div class="lab-concepts">${concepts}</div>

        <h3>Erscheinung</h3>
        <div class="lab-row"><span>Schema</span>
          <div class="lab-seg" data-schemes>
            <button data-scheme="dark">Dunkel</button><button data-scheme="light">Hell</button>
          </div>
        </div>
        <div class="lab-row"><span>Akzent</span>
          <div class="lab-swatches">
            <button class="lab-swatch lab-swatch--auto" data-accent="">Theme</button>
            ${swatches}
            <label class="lab-swatch lab-swatch--picker" title="Eigene Farbe"><input type="color" data-accent-picker aria-label="Eigene Akzentfarbe"></label>
          </div>
        </div>

        <h3>Hintergrund</h3>
        <div class="lab-walls">
          ${walls}
          <label class="lab-wall lab-wall--upload" title="Eigenes Wallpaper">+<input type="file" accept="image/*" data-wall-upload aria-label="Eigenes Wallpaper"></label>
        </div>
        <div class="lab-row"><span>Dahinter</span>
          <div class="lab-seg" data-backdrops>
            <button data-backdrop="none">Desktop</button><button data-backdrop="light">Helles Fenster</button><button data-backdrop="dark">Dunkles</button>
          </div>
        </div>
        <p class="lab-note" data-glass-note hidden>Liquid Glass zeigt – wie in der echten App – das vorgerenderte Wallpaper, nicht das Fenster dahinter.</p>

        <h3>Ereignisse</h3>
        <div class="lab-events">
          ${events}
          <button class="lab-btn lab-btn--wide" data-event="space">Kontrollzentrum öffnen / schließen<kbd>Leertaste</kbd></button>
        </div>
        <p class="lab-note">Scrollen über der Island ändert die Lautstärke. Musik-Island überfahren zeigt den Player.</p>

        <h3>Bewegung</h3>
        <div class="lab-row"><span>Tempo</span>
          <div class="lab-seg" data-tempos>${TEMPOS.map((t) => `<button data-tempo="${t}">${String(t).replace('.', ',')}×</button>`).join('')}</div>
        </div>
        <label class="lab-check">Reduzierte Bewegung <input type="checkbox" data-reduced></label>
        <label class="lab-check">Player bei Hover <input type="checkbox" data-hover-preview checked></label>

        <div class="lab-foot">
          <a href="gallery.html">Komponenten-Galerie →</a>
          <span><kbd>H</kbd> Lab ausblenden</span>
        </div>
      </div>`;
  },

  /** @private */
  bind() {
    const el = this.el;
    el.addEventListener('click', (e) => {
      const t = e.target;
      const concept = t.closest('[data-theme-id]');
      const scheme = t.closest('[data-scheme]');
      const accent = t.closest('[data-accent]');
      const wall = t.closest('[data-wall]');
      const backdrop = t.closest('[data-backdrop]');
      const event = t.closest('[data-event]');
      const tempo = t.closest('[data-tempo]');
      if (concept) {
        this.setTheme(concept.dataset.themeId);
      } else if (scheme && !scheme.disabled) {
        this.setScheme(scheme.dataset.scheme);
      } else if (accent) {
        this.setAccent(accent.dataset.accent || null);
      } else if (wall) {
        this.setWall(wall.dataset.wall);
      } else if (backdrop) {
        this.setBackdrop(backdrop.dataset.backdrop);
      } else if (event) {
        this.fire(event.dataset.event);
      } else if (tempo) {
        this.setTempo(Number(tempo.dataset.tempo));
      } else if (t.closest('[data-collapse]')) {
        el.classList.toggle('is-collapsed');
      }
    });
    el.querySelector('[data-accent-picker]').addEventListener('input', (e) => this.setAccent(e.target.value));
    el.querySelector('[data-wall-upload]').addEventListener('change', (e) => {
      const file = e.target.files[0];
      if (file) {
        this.setWall(null, URL.createObjectURL(file));
      }
    });
    el.querySelector('[data-reduced]').addEventListener('change', (e) => this.setReduced(e.target.checked));
    el.querySelector('[data-hover-preview]').addEventListener('change', (e) => {
      this.app.island.hoverPreview = e.target.checked;
    });

    document.addEventListener('keydown', (e) => {
      const target = e.target instanceof Element ? e.target : document.body;
      if (e.ctrlKey || e.metaKey || e.altKey || target.closest('input, textarea, [contenteditable]')) {
        return;
      }
      const key = e.key.toLowerCase();
      const index = Number(key) - 1;
      if (index >= 0 && index < THEMES.length) {
        this.setTheme(THEMES[index].id);
      } else if (key === 'h') {
        el.hidden = !el.hidden;
      } else if (key === 'd') {
        this.setScheme(this.settings.scheme === 'dark' ? 'light' : 'dark');
      } else if (key === 's') {
        this.setTempo(TEMPOS[(TEMPOS.indexOf(Motion.timeScale) + 1) % TEMPOS.length]);
      } else if (key === 'r') {
        const box = el.querySelector('[data-reduced]');
        box.checked = !box.checked;
        this.setReduced(box.checked);
      } else if (key === ' ' && !target.closest('button, [role="slider"], [role="tab"], .island')) {
        e.preventDefault();
        this.fire('space');
      } else if (LAB_EVENTS.some((ev) => ev.key === key)) {
        this.fire(key);
      }
    });
  },

  /**
   * Runs a lab event.
   *
   * @param {string} key - Event key from LAB_EVENTS or "space".
   * @private
   */
  fire(key) {
    if (key === 'space') {
      this.app.island.toggle();
      return;
    }
    LAB_EVENTS.find((e) => e.key === key)?.run(this.app);
  },

  // ── Settings ────────────────────────────────────────────────────────────

  /** Restores the last lab settings and applies them. */
  restore() {
    try {
      Object.assign(this.settings, JSON.parse(localStorage.getItem(LAB_STORE) || '{}'));
    } catch {
      // Storage may be unavailable (private mode, file:// policies).
    }
    if (!Art.wallpapers.some((w) => w.id === this.settings.wall)) {
      this.settings.wall = 'dunes';
    }
    this.setWall(this.settings.wall);
    this.setAccent(this.settings.accent, false);
    this.setTheme(this.settings.theme, false);
    this.setBackdrop('none');
    this.setTempo(1, false);
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      this.el.querySelector('[data-reduced]').checked = true;
      this.setReduced(true);
    }
  },

  /** @private */
  save() {
    try {
      localStorage.setItem(LAB_STORE, JSON.stringify(this.settings));
    } catch {
      // Ignore: persistence is a convenience only.
    }
  },

  /**
   * @param {string} id - Theme id.
   * @param {boolean} [announce=true] - Show a toast.
   */
  setTheme(id, announce = true) {
    const meta = ThemeEngine.meta(id);
    this.settings.theme = meta.id;
    const scheme = meta.schemes.includes(this.settings.scheme) ? this.settings.scheme : meta.schemes[0];
    ThemeEngine.apply(meta.id, scheme);
    for (const b of this.el.querySelectorAll('[data-theme-id]')) {
      b.classList.toggle('is-active', b.dataset.themeId === meta.id);
    }
    for (const b of this.el.querySelectorAll('[data-scheme]')) {
      b.disabled = !meta.schemes.includes(b.dataset.scheme);
      b.classList.toggle('is-active', b.dataset.scheme === ThemeEngine.scheme);
    }
    this.el.querySelector('[data-glass-note]').hidden = meta.id !== 'glass';
    if (announce) {
      this.toast(meta.name);
    }
    this.save();
  },

  /** @param {string} scheme - "dark" or "light". */
  setScheme(scheme) {
    const meta = ThemeEngine.meta(this.settings.theme);
    if (!meta.schemes.includes(scheme)) {
      this.toast(`${meta.name} gibt es nur ${meta.schemes[0] === 'dark' ? 'dunkel' : 'hell'}`);
      return;
    }
    this.settings.scheme = scheme;
    this.setTheme(meta.id, false);
  },

  /**
   * @param {string|null} color - Hex colour or null for the theme default.
   * @param {boolean} [persist=true] - Save to storage.
   */
  setAccent(color, persist = true) {
    this.settings.accent = color;
    const root = document.documentElement;
    if (color) {
      root.style.setProperty('--accent', color);
    } else {
      root.style.removeProperty('--accent');
    }
    for (const b of this.el.querySelectorAll('[data-accent]')) {
      b.classList.toggle('is-active', (b.dataset.accent || null) === color);
    }
    if (persist) {
      this.save();
    }
  },

  /**
   * Sets a preset or uploaded wallpaper, pre-renders the glass layers and
   * derives the seed colour for dynamic themes.
   *
   * @param {string|null} id - Preset id, or null for an uploaded image.
   * @param {string} [url] - Object URL of an uploaded image.
   */
  async setWall(id, url) {
    const root = document.documentElement;
    const preset = Art.wallpapers.find((w) => w.id === id);
    const src = preset ? Art.svgUrl(preset.svg) : url;
    this.wallSrc = src;
    root.style.setProperty('--wallpaper-src', `url("${src}")`);
    for (const b of this.el.querySelectorAll('[data-wall]')) {
      b.classList.toggle('is-active', b.dataset.wall === id);
    }
    if (preset) {
      this.settings.wall = id;
      this.save();
    }
    const seed = preset ? preset.seed : await Art.extractSeed(src);
    root.style.setProperty('--wallpaper-seed', seed);
    await this.prepareGlass(src);
  },

  /**
   * @param {string} src - Wallpaper URL.
   * @private
   */
  async prepareGlass(src) {
    if (!src) {
      return;
    }
    const { blurred, rim } = await Art.prepareGlass(src);
    const root = document.documentElement;
    root.style.setProperty('--wallpaper-blurred', `url("${blurred}")`);
    root.style.setProperty('--wallpaper-rim', `url("${rim}")`);
    for (const old of this.blobUrls) {
      URL.revokeObjectURL(old);
    }
    this.blobUrls = [blurred, rim];
  },

  /** @param {string} mode - "none", "light" or "dark". */
  setBackdrop(mode) {
    this.stage.dataset.backdrop = mode;
    for (const b of this.el.querySelectorAll('[data-backdrop]')) {
      b.classList.toggle('is-active', b.dataset.backdrop === mode);
    }
  },

  /**
   * @param {number} tempo - Animation speed multiplier.
   * @param {boolean} [announce=true] - Show a toast.
   */
  setTempo(tempo, announce = true) {
    Motion.timeScale = tempo;
    ThemeEngine.refreshMotion();
    for (const b of this.el.querySelectorAll('[data-tempo]')) {
      b.classList.toggle('is-active', Number(b.dataset.tempo) === tempo);
    }
    if (announce) {
      this.toast(`Tempo ${String(tempo).replace('.', ',')}×`);
    }
  },

  /** @param {boolean} on - Reduced motion. */
  setReduced(on) {
    Motion.reduced = on;
    document.documentElement.dataset.motion = on ? 'reduced' : 'full';
    ThemeEngine.refreshMotion();
    this.app.island.onThemeChange();
    this.toast(on ? 'Reduzierte Bewegung an' : 'Volle Bewegung');
  },

  /**
   * Shows a short message.
   *
   * @param {string} message - Text.
   */
  toast(message) {
    this.toastEl.textContent = message;
    this.toastEl.classList.add('is-visible');
    clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => this.toastEl.classList.remove('is-visible'), 1400);
  },
};
