/**
 * Island controller: spring-driven geometry, view switching, split bubble
 * with liquid bridge, auto-hide motion, hover previews and hit regions.
 * Geometry is written per frame to a handful of small elements only.
 */
import type { Rect } from '$backend/contract';
import type { Transport } from '$backend/transport';
import type { GlassGeometry } from '$design/engine.svelte';
import { motionSettings, parseSpring, SpringSystem, SpringValue } from '$motion/spring';
import type { ActivityRegistry } from './activities.svelte';
import type { GlassScene, GlassShape } from './glass/scene';
import { metaballPath } from './metaball';

/** Renderer of the refractive glass material (see glass/liquid-glass.ts). */
export interface GlassRenderer {
  /** Whether the renderer draws the material (WebGL available, glass theme active). */
  readonly active: boolean;
  /** Draws one frame. */
  draw(scene: GlassScene): void;
}

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
  /** Views inside the bubble: its compact face and the hover preview. */
  bubbleViews: { compact?: HTMLElement; expanded?: HTMLElement };
  glass?: GlassRenderer;
}

/** Callbacks into the app. */
export interface IslandHooks {
  /** Panel opened, optionally with a section to show. */
  onExpand: (section: string | null) => void;
  onCollapse: () => void;
  /** Whether hovering an activity should open its larger preview. */
  hoverPreview: () => boolean;
  glass: () => GlassGeometry;
}

/** Which part of the island the pointer is on. */
export type HoverZone = 'main' | 'bubble';

interface Tokens {
  top: number;
  bubbleSize: number;
  bubbleGap: number;
  bubbleRadius: number;
  /** Extra distance the bubble is pulled out before the liquid neck tears. */
  splitPull: number;
  goo: boolean;
  glass: boolean;
}

const clamp = (v: number, lo: number, hi: number): number => Math.min(hi, Math.max(lo, v));

/** Hover delays (ms, before the lab time scale). */
const PREVIEW_OPEN = 420;
const PREVIEW_CLOSE = 260;
/** Space kept free between the island group and the window edge. */
const EDGE_MARGIN = 12;

/** Controller of one island. */
export class IslandController {
  /** Key of the visible view ("clock", "panel", an activity id or "<id>:expanded"). */
  view = $state('clock');
  expanded = $state(false);
  /** The main activity shows its hover preview. */
  previewing = $state(false);
  hidden = $state(false);
  /** Activity shown in the split bubble. */
  bubbleId = $state<string | null>(null);
  /** The bubble shows its hover preview (and pushes the main island aside). */
  bubbleExpanded = $state(false);

  private el: IslandElements | null = null;
  private readonly system = new SpringSystem(() => this.render(), () => this.rest());
  private readonly w: SpringValue;
  private readonly h: SpringValue;
  private readonly r: SpringValue;
  private readonly scale: SpringValue;
  private readonly shift: SpringValue;
  private readonly bx: SpringValue;
  private readonly bs: SpringValue;
  private readonly bw: SpringValue;
  private readonly bh: SpringValue;
  private readonly br: SpringValue;
  private readonly bscale: SpringValue;
  private readonly liquid: SpringValue;
  private readonly open: SpringValue;
  private readonly hide: SpringValue;
  private tokens: Tokens = { top: 10, bubbleSize: 34, bubbleGap: 8, bubbleRadius: 17, splitPull: 18, goo: false, glass: false };
  /** The neck between island and bubble has torn and stays torn. */
  private bridgeBroken = false;
  /** The bubble is being pulled out and the neck still holds. */
  private splitting = false;
  /** After closing the panel the bubble waits until the island is one pill again. */
  private pendingSplit = false;
  private bubbleView: 'compact' | 'expanded' = 'compact';
  private lastRegions = '';
  private hoverZone: HoverZone | null = null;
  private mainTimer: ReturnType<typeof setTimeout> | undefined;
  private bubbleTimer: ReturnType<typeof setTimeout> | undefined;

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
    this.shift = this.system.add(new SpringValue(0, spring));
    this.bx = this.system.add(new SpringValue(-34, spring));
    this.bs = this.system.add(new SpringValue(0.55, spring, 0.001));
    this.bw = this.system.add(new SpringValue(34, spring));
    this.bh = this.system.add(new SpringValue(34, spring));
    this.br = this.system.add(new SpringValue(17, spring));
    this.bscale = this.system.add(new SpringValue(1, [0.32, 0.45], 0.0004));
    this.liquid = this.system.add(new SpringValue(0, [0.16, 0], 0.002));
    this.open = this.system.add(new SpringValue(0, [0.4, 0], 0.002));
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
    clearTimeout(this.mainTimer);
    clearTimeout(this.bubbleTimer);
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
   * Decides which view owns the island, what splits into the bubble and
   * where both sit.
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
    this.setBubble(second?.id ?? null, instant);
    this.showBubbleView(instant);
    this.layout(instant);
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
    this.bubbleExpanded = false;
    this.pendingSplit = false;
    clearTimeout(this.mainTimer);
    clearTimeout(this.bubbleTimer);
    this.scale.to(1);
    this.bscale.to(1);
    this.open.to(1);
    this.hooks.onExpand(section);
    void this.transport.invoke('island_set_expanded', { expanded: true });
    this.resolve();
  }

  /** Closes the panel. A second activity splits off once the island is a pill again. */
  collapse(): void {
    if (!this.expanded) {
      return;
    }
    this.expanded = false;
    this.pendingSplit = true;
    this.open.to(0);
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
    if (hidden && (this.previewing || this.bubbleExpanded)) {
      this.previewing = false;
      this.bubbleExpanded = false;
      this.resolve();
    }
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

  /**
   * Pointer entered a part of the island. Resting on an activity with a
   * preview opens it after a short delay.
   *
   * @param zone - The main island or the split bubble.
   */
  pointerEnter(zone: HoverZone = 'main'): void {
    this.hoverZone = zone;
    if (this.expanded || this.hidden) {
      return;
    }
    const id = zone === 'main' ? this.registry.main?.id : this.bubbleId;
    if (id) {
      this.registry.hold(id);
    }
    const hasPreview = Boolean(id && this.registry.def(id)?.expandedView) && this.hooks.hoverPreview();
    if (zone === 'main') {
      this.scale.to(1.035);
      clearTimeout(this.mainTimer);
      if (hasPreview && !this.previewing) {
        this.mainTimer = setTimeout(() => {
          this.previewing = true;
          this.bubbleExpanded = false;
          this.resolve();
        }, PREVIEW_OPEN / motionSettings.timeScale);
      }
    } else {
      this.bscale.to(1.06);
      clearTimeout(this.bubbleTimer);
      if (hasPreview && !this.bubbleExpanded) {
        this.bubbleTimer = setTimeout(() => {
          this.bubbleExpanded = true;
          this.previewing = false;
          this.resolve();
        }, PREVIEW_OPEN / motionSettings.timeScale);
      }
    }
    this.system.kick();
  }

  /**
   * Pointer left a part of the island; its preview closes after a grace period.
   *
   * @param zone - The main island or the split bubble.
   */
  pointerLeave(zone: HoverZone = 'main'): void {
    if (this.hoverZone === zone) {
      this.hoverZone = null;
    }
    if (zone === 'main') {
      this.scale.to(1);
      clearTimeout(this.mainTimer);
      const main = this.registry.main;
      if (main) {
        this.registry.release(main.id);
      }
      if (this.previewing) {
        this.mainTimer = setTimeout(() => {
          this.previewing = false;
          this.resolve();
        }, PREVIEW_CLOSE / motionSettings.timeScale);
      }
    } else {
      this.bscale.to(1);
      clearTimeout(this.bubbleTimer);
      if (this.bubbleId) {
        this.registry.release(this.bubbleId);
      }
      if (this.bubbleExpanded) {
        this.bubbleTimer = setTimeout(() => {
          this.bubbleExpanded = false;
          this.resolve();
        }, PREVIEW_CLOSE / motionSettings.timeScale);
      }
    }
    this.system.kick();
  }

  /**
   * Pointer pressed on the compact island or the bubble.
   *
   * @param zone - Where it was pressed.
   */
  press(zone: HoverZone = 'main'): void {
    if (!this.expanded) {
      (zone === 'main' ? this.scale : this.bscale).to(0.965);
      this.system.kick();
    }
  }

  /** Pointer released. */
  release(): void {
    if (!this.expanded) {
      this.scale.to(this.hoverZone === 'main' ? 1.035 : 1);
      this.bscale.to(this.hoverZone === 'bubble' ? 1.06 : 1);
      this.system.kick();
    }
  }

  // ── Morphing ────────────────────────────────────────────────────────────

  private readTokens(): void {
    const cs = getComputedStyle(document.documentElement);
    const island = parseSpring(cs.getPropertyValue('--spring-island'));
    const bubble = parseSpring(cs.getPropertyValue('--spring-bubble'));
    for (const s of [this.w, this.h, this.r, this.shift]) {
      s.setSpring(island);
    }
    for (const s of [this.bx, this.bs, this.bw, this.bh, this.br]) {
      s.setSpring(bubble);
    }
    const size = parseFloat(cs.getPropertyValue('--island-height')) || 34;
    // 0 is a valid radius (square islands), so only a missing value falls back to a pill.
    const radius = parseFloat(cs.getPropertyValue('--radius-island'));
    this.tokens = {
      top: parseFloat(cs.getPropertyValue('--island-top')) || 10,
      bubbleSize: size,
      bubbleGap: parseFloat(cs.getPropertyValue('--island-bubble-gap')) || 8,
      bubbleRadius: Math.min(Number.isFinite(radius) ? radius : 999, size / 2),
      splitPull: parseFloat(cs.getPropertyValue('--island-split-pull')) || 18,
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

  /** Re-measures the active views, e.g. after their content changed. */
  refresh(): void {
    const view = this.el?.views[this.view];
    if (view) {
      this.applyTarget(this.measure(view), false);
    }
    if (this.bubbleView === 'expanded') {
      this.showBubbleView(false);
    }
    this.layout(false);
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

  private setBubble(id: string | null, instant: boolean): void {
    if (id === this.bubbleId || !this.el) {
      return;
    }
    const previous = this.bubbleId;
    this.bubbleId = id;
    this.bubbleExpanded = false;
    clearTimeout(this.bubbleTimer);
    this.el.bubbleContent.tabIndex = id ? 0 : -1;
    const { bubbleSize: size, bubbleGap: gap, goo } = this.tokens;
    if (id && previous) {
      // Another activity takes over the bubble; it stays where it is.
      return;
    }
    this.bridgeBroken = false;
    this.splitting = false;
    if (id) {
      if (instant) {
        this.bx.jump(gap);
        this.bs.jump(1);
        this.bridgeBroken = true;
      } else if (!this.pendingSplit) {
        this.startSplit();
      }
    } else {
      this.pendingSplit = false;
      this.bx.to(-size);
      this.bs.to(0.55);
      this.liquid.to(goo ? 1 : 0);
    }
    this.system.kick();
    this.reportRegions(false);
  }

  /**
   * Pulls the bubble out of the island. With the liquid material the neck
   * stretches until it tears ({@link tear}); without it the bubble glides out.
   */
  private startSplit(): void {
    const { bubbleGap: gap, splitPull, goo } = this.tokens;
    this.pendingSplit = false;
    if (goo) {
      this.splitting = true;
      this.liquid.jump(1);
      this.bx.to(gap + splitPull);
    } else {
      this.bridgeBroken = true;
      this.bx.to(gap);
    }
    this.bs.to(1);
    this.system.kick();
  }

  /** The neck tears: the bubble recoils into place and both bodies wobble. */
  private tear(): void {
    this.splitting = false;
    this.bridgeBroken = true;
    this.liquid.to(0);
    this.bx.to(this.tokens.bubbleGap);
    this.scale.impulse(0.45);
    this.bscale.impulse(-0.9);
    this.system.kick();
  }

  /** Switches the bubble between its compact face and the hover preview. */
  private showBubbleView(instant: boolean): void {
    const el = this.el;
    if (!el) {
      return;
    }
    const { compact, expanded } = el.bubbleViews;
    const want = this.bubbleId && this.bubbleExpanded && expanded ? 'expanded' : 'compact';
    const next = want === 'expanded' ? expanded : compact;
    const prev = want === 'expanded' ? compact : expanded;
    if (prev) {
      prev.classList.remove('is-active');
      prev.inert = true;
    }
    if (next) {
      next.style.setProperty('--enter-delay', instant ? '0ms' : `var(--enter-delay-${want === 'expanded' ? 'grow' : 'shrink'})`);
      next.classList.add('is-active');
      next.inert = false;
    }
    this.bubbleView = want;
    const { bubbleSize: size, bubbleRadius } = this.tokens;
    const target = want === 'expanded' && expanded ? this.measure(expanded) : { w: size, h: size, r: bubbleRadius };
    for (const [spring, value] of [[this.bw, target.w], [this.bh, target.h], [this.br, target.r]] as const) {
      if (instant) {
        spring.jump(value);
      } else {
        spring.to(value);
      }
    }
    this.system.kick();
  }

  /**
   * Horizontal placement: an expanding bubble grows around its own centre
   * and pushes the island aside; everything stays inside the window.
   */
  private layout(instant: boolean): void {
    const { bubbleSize: size, bubbleGap: gap } = this.tokens;
    let shift = 0;
    if (!this.expanded && this.bubbleId) {
      shift = -(this.bw.target - size) / 2;
      const half = window.innerWidth / 2 - EDGE_MARGIN;
      const w = this.w.target;
      shift = clamp(shift, -half + w / 2, half - (w / 2 + gap + this.bw.target));
    }
    if (instant) {
      this.shift.jump(shift);
    } else {
      this.shift.to(shift);
      this.system.kick();
    }
    this.reportRegions(false);
  }

  private render(): void {
    const el = this.el;
    if (!el) {
      return;
    }
    const { top, bubbleSize: size, bubbleGap: gap, splitPull } = this.tokens;
    const w = this.w.value;
    const h = this.h.value;
    const r = clamp(this.r.value, 0, Math.min(w, h) / 2);
    const radius = `${r}px`;
    const hy = this.hide.value;
    const shift = this.shift.value;
    const scale = this.scale.value * (1 - hy * 0.12);
    const lift = -hy * (top + h + 24);
    const opacity = 1 - clamp((hy - 0.55) / 0.45, 0, 1);

    // Once the island is a single pill again, the waiting bubble splits off.
    if (this.pendingSplit && this.bubbleId && Math.abs(w - this.w.target) < 6 && Math.abs(h - this.h.target) < 3) {
      this.startSplit();
    }
    if (this.splitting && this.bx.value >= gap + splitPull * 0.72) {
      this.tear();
    }

    const s = el.shell.style;
    s.width = `${w}px`;
    s.height = `${h}px`;
    s.scale = String(scale);
    s.translate = `calc(-50% + ${shift}px) ${lift}px`;
    s.opacity = String(opacity);
    el.shadow.style.borderRadius = radius;
    el.bg.style.borderRadius = radius;
    el.clip.style.borderRadius = radius;

    const bx = this.bx.value;
    const bs = this.bs.value * this.bscale.value;
    const bw = this.bw.value;
    const bh = this.bh.value;
    const br = clamp(this.br.value, 0, Math.min(bw, bh) / 2);
    const transform = `translate(${w + bx}px, 0) scale(${bs})`;
    for (const node of [el.bubble, el.bubbleContent]) {
      node.style.transform = transform;
      node.style.width = `${bw}px`;
      node.style.height = `${bh}px`;
      node.style.borderRadius = `${br}px`;
    }
    const reveal = clamp((bx + size) / (size + gap), 0, 1);
    el.bubble.style.opacity = this.tokens.goo ? '1' : String(reveal);
    el.bubbleContent.classList.toggle('is-visible', Boolean(this.bubbleId) && reveal > 0.85);

    const webglGlass = this.tokens.glass && el.glass?.active === true;
    this.renderBridge(w, h, r, bx, bs, bw, bh, webglGlass);

    if (this.tokens.glass) {
      const vw = window.innerWidth;
      // Shell box in viewport pixels (scaled around its top centre).
      const cx = vw / 2 + shift;
      const y0 = top + lift;
      const main: GlassShape = { x: cx - (w * scale) / 2, y: y0, w: w * scale, h: h * scale, r: r * scale };
      // Bubble centre in shell coordinates is (w + bx + bw / 2, bh / 2).
      const bubbleCx = cx + (w / 2 + bx + bw / 2) * scale;
      const bubbleCy = y0 + (bh / 2) * scale;
      const bsz = { w: bw * bs * scale, h: bh * bs * scale };
      const bubble: GlassShape = { x: bubbleCx - bsz.w / 2, y: bubbleCy - bsz.h / 2, w: bsz.w, h: bsz.h, r: br * bs * scale };
      // A bubble parked inside the island (waiting to split) is not drawn.
      const bubbleVisible = bx > -size + 0.5 && (this.tokens.goo || reveal > 0.02);
      if (webglGlass) {
        el.glass!.draw({ main, bubble: bubbleVisible ? bubble : null, liquid: this.liquid.value, expanded: this.open.value, opacity });
      } else {
        const g = this.hooks.glass();
        this.setGlass(el.bg, g.windowX + main.x, g.windowY + top, w, h, r, g);
        this.setGlass(el.bubble, g.windowX + bubble.x, g.windowY + top, bw, bh, br, g);
      }
    }
  }

  private renderBridge(w: number, h: number, r: number, bx: number, bs: number, bw: number, bh: number, webglGlass: boolean): void {
    const el = this.el!;
    const { bubbleGap: gap, splitPull, goo } = this.tokens;
    const pill = r >= h / 2 - 0.5;
    const round = Math.abs(bw - bh) < 1;
    if (!goo || webglGlass || !pill || !round || this.expanded || bs <= 0.01) {
      el.bridge.setAttribute('d', '');
      return;
    }
    const cap = { x: w - h / 2, y: h / 2, r: h / 2 };
    const bubble = { x: w + bx + bw / 2, y: bh / 2, r: (bh / 2) * bs };
    const reach = cap.r + bubble.r + gap;
    let path = '';
    if (this.bubbleId) {
      // Splitting: the neck holds until it tears, then stays torn.
      path = this.bridgeBroken ? '' : metaballPath(cap, bubble, reach + splitPull + 4);
    } else {
      // Merging back: the neck re-forms as the bubble comes close.
      path = metaballPath(cap, bubble, reach - gap * 0.45);
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
    if (this.pendingSplit && this.bubbleId && !this.expanded) {
      this.startSplit();
      return;
    }
    this.pendingSplit = false;
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
      const { top, bubbleGap: gap } = this.tokens;
      const pick = (s: SpringValue): number => (exact ? s.value : Math.max(s.value, s.target));
      const width = pick(this.w);
      const height = pick(this.h);
      const shift = exact ? this.shift.value : this.shift.target;
      const scale = this.expanded ? 1 : 1.04;
      rects.push({ x: (vw - width * scale) / 2 + shift, y: top, width: width * scale, height: height * scale });
      if (this.bubbleId) {
        const bw = pick(this.bw) * 1.06;
        rects.push({ x: vw / 2 + shift + width / 2 + gap, y: top, width: bw, height: pick(this.bh) * 1.06 });
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
