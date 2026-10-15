/**
 * What the liquid glass renderer draws per frame: the island and its bubble
 * as rounded rectangles in viewport pixels, plus how strongly they melt
 * into each other.
 */

/** A rounded rectangle in viewport pixels. */
export interface GlassShape {
  x: number;
  y: number;
  w: number;
  h: number;
  /** Corner radius (at most half the shorter side). */
  r: number;
}

/** One frame of the glass material. */
export interface GlassScene {
  main: GlassShape;
  /** The split bubble, or null while it is hidden inside the island. */
  bubble: GlassShape | null;
  /** 0…1: how strongly island and bubble melt together (liquid neck). */
  liquid: number;
  /** 0…1: the island is open as control centre (denser tint). */
  expanded: number;
  /** 0…1: overall opacity (auto-hide fades the island out). */
  opacity: number;
}
