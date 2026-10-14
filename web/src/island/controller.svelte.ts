/**
 * Island controller: spring-driven geometry, view switching, split bubble
 * with liquid bridge, auto-hide motion, hover preview and hit regions.
 * Geometry is written per frame to a handful of small elements only.
 */
import type { Rect } from '$backend/contract';
import type { Transport } from '$backend/transport';
import type { GlassGeometry } from '$design/engine.svelte';
import { motionSettings, parseSpring, SpringSystem, SpringValue } from '$motion/spring';
import type { ActivityRegistry } from './activities.svelte';
import { metaballPath } from './metaball';

/** DOM nodes the controller drives. */
export interface IslandElements {
  shell: HTMLElement;
  shadow: HTMLElement;
  bg: HTMLElement;
  clip: HTMLElement;
  bubble: HTMLElement;
  bubbleContent: HTMLElement;
  bridge: SVGPathElement;
  views: Record<string, HTMLElement | undefined>;
}

/** Callbacks into the app. */
export interface IslandHooks {
  /** Panel opened, optionally with a section to show. */
  onExpand: (section: string | null) => void;
  onCollapse: () => void;
  /** Whether hovering the music island should open the player. */
  hoverPreview: () => boolean;
  glass: () => GlassGeometry;
}

interface Tokens {
  top: number;
  bubbleSize: number;
  bubbleGap: number;
  bubbleRadius: number;
  goo: boolean;
  glass: boolean;
}

const clamp = (v: number, lo: number, hi: number): number => Math.min(hi, Math.max(lo, v));

/** Controller of one island. */
export class IslandController {
  /** Key of the visible view ("clock", "panel", an activity id or "<id>:expanded"). */
  view = $state('clock');
  expanded = $state(false);
  previewing = $state(false);
  hidden = $state(false);
  /** Activity shown in the split bubble. */
  bubbleId = $state<string | null>(null);

  private el: IslandElements | null = null;
  private readonly system = new SpringSystem(() => this.render(), () => this.rest());
  private readonly w: SpringValue;
  private readonly h: SpringValue;
  private readonly r: SpringValue;
  private readonly scale: SpringValue;
  private readonly bx: SpringValue;
  private readonly bs: SpringValue;
  private readonly hide: SpringValue;
  private tokens: Tokens = { top: 10, bubbleSize: 34, bubbleGap: 8, bubbleRadius: 17, goo: false, glass: false };
  private bridgeBroken = false;
  private lastRegions = '';
  private hovering = false;
  private previewTimer: ReturnType<typeof setTimeout> | undefined;

  /**
   * @param registry - Activities of this island.
   * @param transport - Backend (hit regions, focus).
   * @param hooks - App callbacks.
   */
  constructor(
    private readonly registry: ActivityRegistry,
    private readonly transport: Transport,
    private readonly hooks: IslandHooks,
  ) {
    const spring: [number, number] = [0.5, 0.25];
    this.w = this.system.add(new SpringValue(120, spring));
    this.h = this.system.add(new SpringValue(34, spring));
    this.r = this.system.add(new SpringValue(17, spring));
    this.scale = this.system.add(new SpringValue(1, [0.32, 0.45], 0.0004));
    this.bx = this.system.add(new SpringValue(-34, spring));
    this.bs = this.system.add(new SpringValue(0.6, spring, 0.001));
    this.hide = this.system.add(new SpringValue(0, [0.45, 0.12], 0.001));
  }

  /**
   * Connects the DOM and shows the initial state without animation.
   *
   * @param el - Elements of the island component.
   */
  attach(el: IslandElements): void {
    this.el = el;
    this.readTokens();
    this.resolve(true);
  }

  /** Stops the animation loop. */
  detach(): void {
    this.system.stop();
    this.el = null;
  }

  /** Re-reads tokens after a theme or override change and re-measures. */
  onThemeChange(): void {
    if (!this.el) {
      return;
    }
    this.readTokens();
    if (!this.bubbleId) {
      this.bx.jump(-this.tokens.bubbleSize);
    } else {
      this.bx.to(this.tokens.bubbleGap);
    }
    this.refresh();
  }

  // ── State ───────────────────────────────────────────────────────────────

  /**
   * Decides which view owns the island and what splits into the bubble.
   *
   * @param instant - Jump instead of animating.
   */
  resolve(instant = false): void {
    if (!this.el) {
      return;
    }
    const main = this.registry.main;
    const def = main ? this.registry.def(main.id) : undefined;
    let view = 'clock';
    if (this.expanded) {
      view = 'panel';
    } else if (main && def) {
      view = this.previewing && def.expandedView ? `${main.id}:expanded` : main.id;
    }
    this.show(view, instant);
    const second = this.expanded ? null : this.registry.second;
    this.setBubble(second?.id ?? null);
    this.updateA11y();
  }

  /**
   * Opens the panel.
   *
   * @param section - Section to show, or null to keep the last one.
   */
  expand(section: string | null = null): void {
    if (this.expanded) {
      return;
    }
    this.expanded = true;
    this.previewing = false;
    this.scale.to(1);
    this.hooks.onExpand(section);
    void this.transport.invoke('island_set_expanded', { expanded: true });
    this.resolve();
  }

  /** Closes the panel. */
  collapse(): void {
    if (!this.expanded) {
      return;
    }
    this.expanded = false;
    this.hooks.onCollapse();
    void this.transport.invoke('island_set_expanded', { expanded: false });
    this.resolve();
  }

  /** Toggles the panel, opening the main activity's section. */
  toggle(): void {
    if (this.expanded) {
      this.collapse();
    } else {
      const main = this.registry.main;
      this.expand(main ? this.registry.def(main.id)?.section ?? null : null);
    }
  }

  /**
   * Slides the island out of view (auto-hide) or back in.
   *
   * @param hidden - Target state.
   */
  setHidden(hidden: boolean): void {
    if (this.hidden === hidden) {
      return;
    }
    this.hidden = hidden;
    this.el?.shell.toggleAttribute('inert', hidden);
    this.hide.to(hidden ? 1 : 0);
    this.system.kick();
    this.reportRegions(true);
  }

  /** Physical bump when the owning activity updates in place. */
  bump(): void {
    if (motionSettings.reduced || !this.el) {
      return;
    }
    this.scale.impulse(0.7);
    this.system.kick();
    const view = this.el.views[this.view];
    if (view) {
      view.classList.remove('is-bumped');
      void view.offsetWidth;
      view.classList.add('is-bumped');
    }
  }

  // ── Pointer ─────────────────────────────────────────────────────────────

  /** Pointer entered the island. */
  pointerEnter(): void {
    this.hovering = true;
    if (this.expanded || this.hidden) {
      return;
    }
    this.scale.to(1.035);
    this.system.kick();
    const main = this.registry.main;
    if (main) {
      this.registry.hold(main.id);
    }
    clearTimeout(this.previewTimer);
    if (main && this.registry.def(main.id)?.expandedView && this.hooks.hoverPreview() && !this.previewing) {
      this.previewTimer = setTimeout(() => {
        this.previewing = true;
        this.resolve();
      }, 420 / motionSettings.timeScale);
    }
  }

  /** Pointer left the island. */
  pointerLeave(): void {
    this.hovering = false;
    this.scale.to(1);
    this.system.kick();
    clearTimeout(this.previewTimer);
    const main = this.registry.main;
    if (main) {
      this.registry.release(main.id);
    }
    if (this.previewing) {
      this.previewTimer = setTimeout(() => {
        this.previewing = false;
        this.resolve();
      }, 260 / motionSettings.timeScale);
    }
  }

  /** Pointer pressed on the compact island. */
  press(): void {
    if (!this.expanded) {
      this.scale.to(0.965);
      this.system.kick();
    }
  }

  /** Pointer released. */
  release(): void {
    if (!this.expanded) {
      this.scale.to(this.hovering ? 1.035 : 1);
      this.system.kick();
    }
  }

  // ── Morphing ────────────────────────────────────────────────────────────

  private readTokens(): void {
    const cs = getComputedStyle(document.documentElement);
    const island = parseSpring(cs.getPropertyValue('--spring-island'));
    const bubble = parseSpring(cs.getPropertyValue('--spring-bubble'));
    for (const s of [this.w, this.h, this.r]) {
      s.setSpring(island);
    }
    this.bx.setSpring(bubble);
    this.bs.setSpring(bubble);
    const size = parseFloat(cs.getPropertyValue('--island-height')) || 34;
    this.tokens = {
      top: parseFloat(cs.getPropertyValue('--island-top')) || 10,
      bubbleSize: size,
      bubbleGap: parseFloat(cs.getPropertyValue('--island-bubble-gap')) || 8,
      bubbleRadius: Math.min(parseFloat(cs.getPropertyValue('--radius-island')) || 999, size / 2),
      goo: cs.getPropertyValue('--island-goo').trim() === '1' && !motionSettings.reduced,
      glass: cs.getPropertyValue('--island-material').trim() === 'glass',
    };
  }

  private measure(view: HTMLElement): { w: number; h: number; r: number } {
    const w = view.offsetWidth;
    const h = view.offsetHeight;
    const radius = parseFloat(getComputedStyle(view).getPropertyValue('--view-radius')) || 0;
    return { w, h, r: Math.min(radius, h / 2) };
  }

  private show(name: string, instant: boolean): void {
    const el = this.el;
    if (!el) {
      return;
    }
    const next = el.views[name] ?? el.views.clock;
    if (!next) {
      return;
    }
    if (name === this.view && next.classList.contains('is-active')) {
      this.refresh();
      return;
    }
    const prev = el.views[this.view];
    const size = this.measure(next);
    const growing = size.w * size.h >= this.w.target * this.h.target;
    if (prev && prev !== next) {
      prev.classList.remove('is-active');
      prev.inert = true;
    }
    next.style.setProperty('--enter-delay', instant ? '0ms' : `var(--enter-delay-${growing ? 'grow' : 'shrink'})`);
    next.classList.add('is-active');
    next.inert = false;
    this.view = name;
    this.applyTarget(size, instant);
  }

  /** Re-measures the active view, e.g. after its content changed. */
  refresh(): void {
    const view = this.el?.views[this.view];
    if (view) {
      this.applyTarget(this.measure(view), false);
    }
  }

  private applyTarget(size: { w: number; h: number; r: number }, instant: boolean): void {
    if (instant) {
      this.w.jump(size.w);
      this.h.jump(size.h);
      this.r.jump(size.r);
      this.render();
    } else {
      this.w.to(size.w);
      this.h.to(size.h);
      this.r.to(size.r);
      this.system.kick();
    }
    this.reportRegions(false);
  }

  private setBubble(id: string | null): void {
    if (id === this.bubbleId || !this.el) {
      return;
    }
    this.bubbleId = id;
    this.bridgeBroken = false;
    this.el.bubbleContent.tabIndex = id ? 0 : -1;
    this.bx.to(id ? this.tokens.bubbleGap : -this.tokens.bubbleSize);
    this.bs.to(id ? 1 : 0.55);
    this.system.kick();
    this.reportRegions(false);
  }

  private render(): void {
    const el = this.el;
    if (!el) {
      return;
    }
    const { top, bubbleSize: size, bubbleGap: gap } = this.tokens;
    const w = this.w.value;
    const h = this.h.value;
    const r = clamp(this.r.value, 0, Math.min(w, h) / 2);
    const radius = `${r}px`;
    const hy = this.hide.value;

    const s = el.shell.style;
    s.width = `${w}px`;
    s.height = `${h}px`;
    s.scale = String(this.scale.value * (1 - hy * 0.12));
    s.translate = `-50% ${-hy * (top + h + 24)}px`;
    s.opacity = String(1 - clamp((hy - 0.55) / 0.45, 0, 1));
    el.shadow.style.borderRadius = radius;
    el.bg.style.borderRadius = radius;
    el.clip.style.borderRadius = radius;

    const bx = this.bx.value;
    const bs = this.bs.value;
    const transform = `translate(${w + bx}px, 0) scale(${bs})`;
    el.bubble.style.transform = transform;
    el.bubbleContent.style.transform = transform;
    const reveal = clamp((bx + size) / (size + gap), 0, 1);
    el.bubble.style.opacity = this.tokens.goo ? '1' : String(reveal);
    el.bubbleContent.classList.toggle('is-visible', Boolean(this.bubbleId) && reveal > 0.85);

    this.renderBridge(w, h, r, bx, bs);

    if (this.tokens.glass) {
      const g = this.hooks.glass();
      const vw = window.innerWidth;
      const x = g.windowX + (vw - w) / 2;
      const y = g.windowY + top;
      this.setGlass(el.bg, x, y, w, h, r, g);
      this.setGlass(el.bubble, x + w + bx, y, size, size, this.tokens.bubbleRadius, g);
    }
  }

  private renderBridge(w: number, h: number, r: number, bx: number, bs: number): void {
    const el = this.el!;
    const { bubbleSize: size, bubbleGap: gap, goo } = this.tokens;
    const pill = r >= h / 2 - 0.5;
    if (!goo || !pill || this.expanded || bs <= 0.01) {
      el.bridge.setAttribute('d', '');
      return;
    }
    const cap = { x: w - h / 2, y: h / 2, r: h / 2 };
    const bubble = { x: w + bx + size / 2, y: size / 2, r: (size / 2) * bs };
    const appearing = this.bubbleId !== null;
    const path = this.bridgeBroken && appearing ? '' : metaballPath(cap, bubble, cap.r + bubble.r + gap * 0.55);
    if (appearing && !path && bx > -size / 2) {
      // Once the neck has snapped it stays snapped, even when the spring rebounds.
      this.bridgeBroken = true;
    }
    el.bridge.setAttribute('d', path);
  }

  private setGlass(node: HTMLElement, x: number, y: number, w: number, h: number, r: number, g: GlassGeometry): void {
    const s = node.style;
    s.setProperty('--ix', `${x}px`);
    s.setProperty('--iy', `${y}px`);
    s.setProperty('--icx', `${x + w / 2}px`);
    s.setProperty('--icy', `${y + h / 2}px`);
    s.setProperty('--vw', `${g.monitorWidth}px`);
    s.setProperty('--vh', `${g.monitorHeight}px`);
    s.setProperty('--r', `${r}px`);
  }

  private rest(): void {
    this.reportRegions(true);
  }

  /**
   * Tells the backend where the island is clickable. While a morph runs the
   * region covers both the current and the target size.
   *
   * @param exact - Use only the settled geometry.
   */
  private reportRegions(exact: boolean): void {
    if (!this.el) {
      return;
    }
    const rects: Rect[] = [];
    if (!this.hidden) {
      const vw = window.innerWidth;
      const { top, bubbleSize: size, bubbleGap: gap } = this.tokens;
      const width = exact ? this.w.value : Math.max(this.w.value, this.w.target);
      const height = exact ? this.h.value : Math.max(this.h.value, this.h.target);
      const scale = this.expanded ? 1 : 1.04;
      rects.push({ x: (vw - width * scale) / 2, y: top, width: width * scale, height: height * scale });
      if (this.bubbleId) {
        rects.push({ x: (vw + width) / 2 + gap, y: top, width: size, height: size });
      }
    }
    const key = JSON.stringify(rects.map((r) => [Math.round(r.x), Math.round(r.y), Math.round(r.width), Math.round(r.height)]));
    if (key !== this.lastRegions) {
      this.lastRegions = key;
      void this.transport.invoke('island_set_hit_regions', { rects });
    }
  }

  private updateA11y(): void {
    const shell = this.el?.shell;
    if (!shell) {
      return;
    }
    shell.setAttribute('role', this.expanded ? 'dialog' : 'button');
    if (this.expanded) {
      shell.removeAttribute('aria-expanded');
    } else {
      shell.setAttribute('aria-expanded', 'false');
    }
    const main = this.registry.main;
    shell.dataset.label = main ? this.registry.def(main.id)?.label() ?? '' : '';
  }
}
