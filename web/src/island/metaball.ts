/**
 * Liquid bridge between the island's end cap and the split bubble, computed
 * as a vector path (metaball construction after Hiroyuki Sato / Varun
 * Vachhar). Replaces the blur+threshold SVG filter of the lab, which caused
 * aliased edges and a visible snap when it was switched off.
 */

/** A circle in shell coordinates. */
export interface Circle {
  x: number;
  y: number;
  r: number;
}

type Point = [number, number];

const dist = (a: Point, b: Point): number => Math.hypot(a[0] - b[0], a[1] - b[1]);
const angle = (a: Point, b: Point): number => Math.atan2(a[1] - b[1], a[0] - b[0]);
const vec = (c: Point, a: number, r: number): Point => [c[0] + r * Math.cos(a), c[1] + r * Math.sin(a)];
const pt = (p: Point): string => `${p[0].toFixed(2)} ${p[1].toFixed(2)}`;

/**
 * Builds the bridge path between two circles, or an empty string when they
 * are too far apart (the bridge has snapped) or one contains the other.
 *
 * @param a - Circle at the island's end cap.
 * @param b - Bubble circle.
 * @param maxDistance - Centre distance at which the bridge snaps.
 * @param spread - 0…1, how wide the neck attaches (0.5 looks liquid).
 * @param handle - Curvature of the neck.
 * @returns SVG path data.
 */
export function metaballPath(a: Circle, b: Circle, maxDistance: number, spread = 0.5, handle = 2.4): string {
  const c1: Point = [a.x, a.y];
  const c2: Point = [b.x, b.y];
  const d = dist(c1, c2);
  const r1 = a.r;
  const r2 = b.r;
  if (r1 <= 0 || r2 <= 0 || d > maxDistance || d <= Math.abs(r1 - r2)) {
    return '';
  }
  let u1 = 0;
  let u2 = 0;
  if (d < r1 + r2) {
    u1 = Math.acos(Math.min(1, Math.max(-1, (r1 * r1 + d * d - r2 * r2) / (2 * r1 * d))));
    u2 = Math.acos(Math.min(1, Math.max(-1, (r2 * r2 + d * d - r1 * r1) / (2 * r2 * d))));
  }
  const between = angle(c2, c1);
  const maxSpread = Math.acos(Math.min(1, Math.max(-1, (r1 - r2) / d)));
  const a1 = between + u1 + (maxSpread - u1) * spread;
  const a2 = between - u1 - (maxSpread - u1) * spread;
  const a3 = between + Math.PI - u2 - (Math.PI - u2 - maxSpread) * spread;
  const a4 = between - Math.PI + u2 + (Math.PI - u2 - maxSpread) * spread;
  const p1 = vec(c1, a1, r1);
  const p2 = vec(c1, a2, r1);
  const p3 = vec(c2, a3, r2);
  const p4 = vec(c2, a4, r2);
  const total = r1 + r2;
  const d2 = Math.min(spread * handle, dist(p1, p3) / total) * Math.min(1, (d * 2) / total);
  const h1 = vec(p1, a1 - Math.PI / 2, r1 * d2);
  const h2 = vec(p2, a2 + Math.PI / 2, r1 * d2);
  const h3 = vec(p3, a3 + Math.PI / 2, r2 * d2);
  const h4 = vec(p4, a4 - Math.PI / 2, r2 * d2);
  const large = d > r1 ? 1 : 0;
  return `M ${pt(p1)} C ${pt(h1)} ${pt(h3)} ${pt(p3)} A ${r2.toFixed(2)} ${r2.toFixed(2)} 0 ${large} 0 ${pt(p4)} C ${pt(h4)} ${pt(h2)} ${pt(p2)} Z`;
}
