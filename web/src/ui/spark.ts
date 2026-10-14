/** Curve styles a theme can choose via `--spark-style`. */
export type SparkStyle = 'smooth' | 'linear' | 'step';

/**
 * Builds sparkline paths in a 100×28 viewBox.
 *
 * @param values - Samples, oldest first.
 * @param max - Value mapped to the top edge.
 * @param style - Curve style.
 * @returns SVG path data for the line and the filled area.
 */
export function sparkPath(values: number[], max: number, style: SparkStyle): { line: string; area: string } {
  const W = 100;
  const H = 28;
  if (values.length < 2 || max <= 0) {
    return { line: `M0,${H} L${W},${H}`, area: '' };
  }
  const pts = values.map((v, i): [number, number] => [(i / (values.length - 1)) * W, H - 2 - (Math.min(v, max) / max) * (H - 4)]);
  const f = (n: number): string => n.toFixed(2);
  const first = pts[0]!;
  let line = `M${f(first[0])},${f(first[1])}`;
  if (style === 'step') {
    for (const p of pts.slice(1)) {
      line += ` H${f(p[0])} V${f(p[1])}`;
    }
  } else if (style === 'linear') {
    for (const p of pts.slice(1)) {
      line += ` L${f(p[0])},${f(p[1])}`;
    }
  } else {
    // Catmull-Rom converted to cubic Béziers.
    for (let i = 0; i < pts.length - 1; i++) {
      const p0 = pts[i - 1] ?? pts[i]!;
      const p1 = pts[i]!;
      const p2 = pts[i + 1]!;
      const p3 = pts[i + 2] ?? p2;
      const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
      const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
      line += ` C${f(c1[0]!)},${f(c1[1]!)} ${f(c2[0]!)},${f(c2[1]!)} ${f(p2[0])},${f(p2[1])}`;
    }
  }
  return { line, area: `${line} L${W},${H} L0,${H} Z` };
}
