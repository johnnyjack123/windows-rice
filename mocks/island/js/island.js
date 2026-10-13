/* Island controller: activity arbitration, spring-driven morphing between
   views, the split bubble and pointer interaction. Rendering of the views'
   content lives in island-views.js and panel.js. */

/**
 * @typedef {Object} Activity
 * @property {string} id - Unique id, e.g. "music".
 * @property {number} priority - Higher wins the island; see ISLAND_PRIORITY.
 * @property {string} view - View shown when this activity owns the island.
 * @property {number} [ttl] - Auto-dismiss after this many ms (transient).
 * @property {string} [tab] - Panel tab to open when the island is clicked.
 * @property {string} [label] - Accessible description.
 * @property {function(): string} [bubble] - Markup when shown as split bubble.
 * @property {boolean} [bump] - Play a physical bump when updated in place.
 */

/** Priorities of built-in activities. Plugins pick values in between. */
const ISLAND_PRIORITY = {
  notification: 100,
  osd: 90,
  charging: 80,
  event: 60,
  timer: 50,
  music: 40,
};

/**
 * Drives the island element. Exactly one view is active at a time; the shell
 * springs to the measured size of that view.
 */
class Island {
  /**
   * @param {HTMLElement} anchor - The `.island-anchor` element.
   * @param {object} hooks - Callbacks into the app.
   * @param {function(string=): void} hooks.onExpand - Panel opened with a tab.
   * @param {function(): void} hooks.onCollapse - Panel closed.
   * @param {function(number): void} hooks.onWheel - Wheel over the compact island.
   */
  constructor(anchor, hooks) {
    this.anchor = anchor;
    this.hooks = hooks;
    this.root = document.documentElement;
    this.shell = anchor.querySelector('.island');
    this.shadow = this.shell.querySelector('.island__shadow');
    this.shapes = this.shell.querySelector('.island__shapes');
    this.bg = this.shell.querySelector('.island__bg');
    this.bubble = this.shell.querySelector('.island__bubble');
    this.clip = this.shell.querySelector('.island__clip');
    this.bubbleContent = this.shell.querySelector('.bubble-content');
    this.views = new Map([...this.clip.querySelectorAll('[data-view]')].map((v) => [v.dataset.view, v]));

    /** @type {Map<string, Activity>} */
    this.activities = new Map();
    this.timers = new Map();
    this.main = null;
    this.current = null;
    this.bubbleId = null;
    this.expanded = false;
    this.hovering = false;
    this.previewing = false;
    /** Expand music into the player on hover. */
    this.hoverPreview = true;

    this.system = new SpringSystem(() => this.render(), () => this.rest());
    const spring = [0.5, 0.25];
    this.w = this.system.add(new SpringValue(120, spring));
    this.h = this.system.add(new SpringValue(34, spring));
    this.r = this.system.add(new SpringValue(17, spring));
    this.scale = this.system.add(new SpringValue(1, [0.32, 0.45], 0.0004));
    this.bx = this.system.add(new SpringValue(-34, spring));
    this.bs = this.system.add(new SpringValue(0.6, spring, 0.001));

    this.readTheme();
    this.bindEvents();
    this.resolve(true);
  }

  /** The activity currently owning the island, if any. */
  get mainActivity() {
    return this.main;
  }

  /**
   * Re-reads every token the controller depends on. Call after a theme
   * change; geometry is re-measured because fonts and paddings change.
   */
  readTheme() {
    const cs = getComputedStyle(this.root);
    const island = Motion.parseToken(cs.getPropertyValue('--spring-island'));
    const bubble = Motion.parseToken(cs.getPropertyValue('--spring-bubble'));
    this.w.setSpring(island);
    this.h.setSpring(island);
    this.r.setSpring(island);
    this.bx.setSpring(bubble);
    this.bs.setSpring(bubble);
    this.goo = cs.getPropertyValue('--island-goo').trim() === '1';
    this.glass = cs.getPropertyValue('--island-material').trim() === 'glass';
    this.top = parseFloat(cs.getPropertyValue('--island-top')) || 10;
    this.bubbleSize = parseFloat(cs.getPropertyValue('--island-height')) || 34;
    this.bubbleGap = parseFloat(cs.getPropertyValue('--island-bubble-gap')) || 7;
    this.bubbleRadius = Math.min(parseFloat(cs.getPropertyValue('--radius-island')) || 999, this.bubbleSize / 2);
    if (!this.bubbleId) {
      this.bx.jump(-this.bubbleSize);
    }
    this.shapes.classList.remove('is-gooey');
  }

  /** Re-applies theme-dependent state and re-measures the current view. */
  onThemeChange() {
    this.readTheme();
    if (this.bubbleId) {
      this.bx.to(this.bubbleGap);
    }
    this.refresh();
  }

  // ── Activities ──────────────────────────────────────────────────────────

  /**
   * Adds or updates an activity and re-arbitrates the island.
   *
   * @param {Activity} activity - Activity to show.
   */
  upsert(activity) {
    const existing = this.activities.get(activity.id);
    const next = { ...existing, ...activity };
    this.activities.set(activity.id, next);
    this.arm(next);
    const wasMain = this.main && this.main.id === activity.id;
    this.resolve();
    if (existing && wasMain && !this.expanded) {
      this.refresh();
      if (activity.bump) {
        this.bump();
      }
    }
  }

  /**
   * Removes an activity.
   *
   * @param {string} id - Activity id.
   */
  remove(id) {
    clearTimeout(this.timers.get(id));
    this.timers.delete(id);
    if (this.activities.delete(id)) {
      this.resolve();
    }
  }

  /**
   * @param {string} id - Activity id.
   * @returns {boolean} Whether the activity is active.
   */
  has(id) {
    return this.activities.has(id);
  }

  /**
   * (Re)starts the expiry timer of a transient activity. Slow motion
   * stretches lifetimes so they stay watchable.
   *
   * @param {Activity} activity - Activity to arm.
   * @param {number} [ttl] - Override lifetime in ms.
   * @private
   */
  arm(activity, ttl = activity.ttl) {
    clearTimeout(this.timers.get(activity.id));
    if (ttl) {
      this.timers.set(activity.id, setTimeout(() => this.remove(activity.id), ttl / Motion.timeScale));
    }
  }

  /**
   * Decides which view owns the island and which activity, if any, splits
   * off into the bubble.
   *
   * @param {boolean} [instant=false] - Skip animation (initial render).
   */
  resolve(instant = false) {
    const list = [...this.activities.values()].sort((a, b) => b.priority - a.priority);
    this.main = list[0] || null;

    let view = 'clock';
    if (this.expanded) {
      view = 'panel';
    } else if (this.main) {
      view = this.main.view === 'music' && this.previewing ? 'music-expanded' : this.main.view;
    }
    this.show(view, instant);

    const compact = this.views.get(view).hasAttribute('data-compact');
    const second = !this.expanded && compact && this.main
      ? list.find((a) => a !== this.main && typeof a.bubble === 'function')
      : null;
    this.setBubble(second || null);

    // Compact it behaves as a button; expanded it is a non-modal dialog.
    this.shell.setAttribute('role', this.expanded ? 'dialog' : 'button');
    if (this.expanded) {
      this.shell.removeAttribute('aria-expanded');
    } else {
      this.shell.setAttribute('aria-expanded', 'false');
    }
    this.shell.setAttribute('aria-label', this.expanded ? 'Kontrollzentrum' : (this.main?.label || 'Dynamic Island'));
  }

  // ── Panel ───────────────────────────────────────────────────────────────

  /**
   * Opens the panel.
   *
   * @param {string} [tab] - Tab to show.
   */
  expand(tab) {
    if (this.expanded) {
      return;
    }
    this.expanded = true;
    this.previewing = false;
    this.scale.to(1);
    this.hooks.onExpand(tab);
    this.resolve();
  }

  /** Closes the panel. */
  collapse() {
    if (!this.expanded) {
      return;
    }
    this.expanded = false;
    this.hooks.onCollapse();
    this.resolve();
  }

  /** Toggles the panel. */
  toggle() {
    if (this.expanded) {
      this.collapse();
    } else {
      this.expand(this.main?.tab);
    }
  }

  // ── Morphing ────────────────────────────────────────────────────────────

  /**
   * Measures a view's natural size and its corner radius token.
   *
   * @param {HTMLElement} view - View element.
   * @returns {{w: number, h: number, r: number}} Target geometry.
   * @private
   */
  measure(view) {
    const w = view.offsetWidth;
    const h = view.offsetHeight;
    const radius = parseFloat(getComputedStyle(view).getPropertyValue('--view-radius')) || 0;
    return { w, h, r: Math.min(radius, h / 2) };
  }

  /**
   * Activates a view and springs the shell to its size. Content of the
   * previous view exits immediately; the new content enters after the
   * shape has started to move (shape first, content second).
   *
   * @param {string} name - View name.
   * @param {boolean} [instant=false] - Jump instead of animating.
   * @private
   */
  show(name, instant = false) {
    if (name === this.current) {
      return;
    }
    const next = this.views.get(name);
    const prev = this.views.get(this.current);
    const size = this.measure(next);
    const growing = size.w * size.h >= this.w.target * this.h.target;

    if (prev) {
      prev.classList.remove('is-active');
      prev.inert = true;
    }
    next.style.setProperty('--enter-delay', instant ? '0ms' : `var(--enter-delay-${growing ? 'grow' : 'shrink'})`);
    next.classList.add('is-active');
    next.inert = false;

    this.current = name;
    this.shell.dataset.view = name;
    this.shell.classList.toggle('is-expanded', name === 'panel');
    this.applyTarget(size, instant);
  }

  /**
   * @param {{w: number, h: number, r: number}} size - Target geometry.
   * @param {boolean} [instant=false] - Jump instead of animating.
   * @private
   */
  applyTarget(size, instant = false) {
    if (instant) {
      this.w.jump(size.w);
      this.h.jump(size.h);
      this.r.jump(size.r);
      this.render();
      return;
    }
    this.w.to(size.w);
    this.h.to(size.h);
    this.r.to(size.r);
    this.system.kick();
  }

  /** Re-measures the active view, e.g. after its text changed. */
  refresh() {
    const view = this.views.get(this.current);
    if (view) {
      this.applyTarget(this.measure(view));
    }
  }

  /** Physical "bump" when an activity updates in place. */
  bump() {
    if (Motion.reduced) {
      return;
    }
    this.scale.impulse(0.7);
    this.system.kick();
    const view = this.views.get(this.current);
    view.classList.remove('is-bumped');
    void view.offsetWidth;
    view.classList.add('is-bumped');
  }

  /**
   * Shows or hides the split bubble for a secondary activity.
   *
   * @param {Activity|null} activity - Secondary activity or null.
   * @private
   */
  setBubble(activity) {
    const id = activity ? activity.id : null;
    if (activity) {
      this.bubbleContent.innerHTML = activity.bubble();
      this.bubbleContent.setAttribute('aria-label', activity.label || activity.id);
    }
    if (id === this.bubbleId) {
      return;
    }
    this.bubbleId = id;
    this.bubbleContent.classList.toggle('is-visible', Boolean(activity));
    this.bubbleContent.tabIndex = activity ? 0 : -1;
    this.bx.to(activity ? this.bubbleGap : -this.bubbleSize);
    this.bs.to(activity ? 1 : 0.6);
    if (this.goo && !Motion.reduced) {
      this.shapes.classList.add('is-gooey');
    }
    this.system.kick();
  }

  /**
   * Writes the current spring values to the DOM. Only a few small elements
   * are touched; nothing is written as an inherited custom property on the
   * large panel subtree.
   *
   * @private
   */
  render() {
    const w = this.w.value;
    const h = this.h.value;
    const r = Math.max(0, Math.min(this.r.value, h / 2, w / 2));
    const radius = `${r}px`;

    this.shell.style.width = `${w}px`;
    this.shell.style.height = `${h}px`;
    this.shell.style.scale = String(this.scale.value);
    this.shadow.style.borderRadius = radius;
    this.bg.style.borderRadius = radius;
    this.clip.style.borderRadius = radius;

    const size = this.bubbleSize;
    const bx = this.bx.value;
    const transform = `translate(${w + bx}px, 0) scale(${this.bs.value})`;
    this.bubble.style.transform = transform;
    this.bubbleContent.style.transform = transform;
    const reveal = Math.min(1, Math.max(0, (bx + size) / (size + this.bubbleGap)));
    this.bubble.style.opacity = this.goo ? '1' : String(reveal);

    if (this.glass) {
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      const x = (vw - w) / 2;
      this.setGlass(this.bg, x, this.top, w, h, r, vw, vh);
      this.setGlass(this.bubble, x + w + bx, this.top, size, size, this.bubbleRadius, vw, vh);
    }
  }

  /**
   * Gives a glass layer its position in viewport coordinates so the
   * pre-blurred wallpaper lines up with the real one.
   *
   * @private
   */
  setGlass(el, x, y, w, h, r, vw, vh) {
    const s = el.style;
    s.setProperty('--ix', `${x}px`);
    s.setProperty('--iy', `${y}px`);
    s.setProperty('--icx', `${x + w / 2}px`);
    s.setProperty('--icy', `${y + h / 2}px`);
    s.setProperty('--vw', `${vw}px`);
    s.setProperty('--vh', `${vh}px`);
    s.setProperty('--r', `${r}px`);
  }

  /** Called when all springs are at rest. @private */
  rest() {
    this.shapes.classList.remove('is-gooey');
  }

  // ── Interaction ─────────────────────────────────────────────────────────

  /** @private */
  bindEvents() {
    const shell = this.shell;

    shell.addEventListener('pointerenter', () => {
      this.hovering = true;
      if (this.expanded) {
        return;
      }
      this.scale.to(1.035);
      this.system.kick();
      clearTimeout(this.previewTimer);
      if (this.hoverPreview && this.main?.view === 'music' && !this.previewing) {
        this.previewTimer = setTimeout(() => {
          this.previewing = true;
          this.resolve();
        }, 420 / Motion.timeScale);
      }
      if (this.main?.ttl) {
        clearTimeout(this.timers.get(this.main.id));
      }
    });

    shell.addEventListener('pointerleave', () => {
      this.hovering = false;
      this.scale.to(1);
      this.system.kick();
      clearTimeout(this.previewTimer);
      if (this.previewing) {
        this.previewTimer = setTimeout(() => {
          this.previewing = false;
          this.resolve();
        }, 260 / Motion.timeScale);
      }
      if (this.main?.ttl) {
        this.arm(this.main, 1800);
      }
    });

    shell.addEventListener('pointerdown', () => {
      if (!this.expanded) {
        this.scale.to(0.965);
        this.system.kick();
      }
    });

    const release = () => {
      if (!this.expanded) {
        this.scale.to(this.hovering ? 1.035 : 1);
        this.system.kick();
      }
    };
    shell.addEventListener('pointerup', release);
    shell.addEventListener('pointercancel', release);

    shell.addEventListener('click', (event) => {
      if (this.expanded || event.target.closest('button:not(.bubble-content)')) {
        return;
      }
      this.expand(this.main?.tab);
    });

    shell.addEventListener('keydown', (event) => {
      if (event.target !== shell) {
        return;
      }
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        this.toggle();
      }
    });

    shell.addEventListener('wheel', (event) => {
      if (this.expanded) {
        return;
      }
      event.preventDefault();
      this.hooks.onWheel(event.deltaY);
    }, { passive: false });

    this.bubbleContent.addEventListener('click', (event) => {
      event.stopPropagation();
      const activity = this.activities.get(this.bubbleId);
      this.expand(activity?.tab);
    });

    shell.addEventListener('pointermove', (event) => {
      if (this.glass) {
        const rect = shell.getBoundingClientRect();
        this.bg.style.setProperty('--mx', `${((event.clientX - rect.left) / rect.width) * 100}%`);
      }
    });

    window.addEventListener('resize', () => this.render());
  }
}
