/**
 * Colour helpers: resolving CSS colours to hex, WCAG contrast and extracting
 * a characteristic colour from an image (album art, wallpaper).
 */

let probe: CanvasRenderingContext2D | null = null;

function probeContext(): CanvasRenderingContext2D | null {
  if (!probe && typeof document !== 'undefined') {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    probe = canvas.getContext('2d', { willReadFrequently: true });
  }
  return probe;
}

/**
 * Converts any CSS colour (including oklch() and color-mix()) to "#rrggbb"
 * by painting one pixel. Alpha is composited over black.
 *
 * @param css - Any CSS colour string.
 * @returns Hex colour, or null if the browser cannot parse it.
 */
export function toHex(css: string): string | null {
  const ctx = probeContext();
  if (!ctx) {
    return null;
  }
  ctx.clearRect(0, 0, 1, 1);
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, 1, 1);
  ctx.fillStyle = '#010203';
  ctx.fillStyle = css;
  if (ctx.fillStyle === '#010203' && css.trim().toLowerCase() !== '#010203') {
    return null;
  }
  ctx.fillRect(0, 0, 1, 1);
  const [r, g, b] = ctx.getImageData(0, 0, 1, 1).data;
  return rgbToHex(r ?? 0, g ?? 0, b ?? 0);
}

/**
 * Resolves a custom property (or any colour expression) in the context of an
 * element, e.g. `resolveColor(el, 'var(--base-bg)')`.
 *
 * @param el - Element whose cascade applies.
 * @param expression - CSS colour expression.
 * @returns Hex colour or null.
 */
export function resolveColor(el: Element, expression: string): string | null {
  const host = document.createElement('span');
  host.style.color = expression;
  host.style.display = 'none';
  el.append(host);
  const computed = getComputedStyle(host).color;
  host.remove();
  return toHex(computed);
}

/**
 * @param r - Red 0…255.
 * @param g - Green 0…255.
 * @param b - Blue 0…255.
 * @returns "#rrggbb".
 */
export function rgbToHex(r: number, g: number, b: number): string {
  return `#${[r, g, b].map((v) => Math.round(v).toString(16).padStart(2, '0')).join('')}`;
}

/**
 * @param hex - "#rgb" or "#rrggbb".
 * @returns Channels 0…255, or null for invalid input.
 */
export function hexToRgb(hex: string): [number, number, number] | null {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(hex.trim());
  if (!m) {
    return null;
  }
  const v = m[1]!.length === 3 ? m[1]!.split('').map((c) => c + c).join('') : m[1]!;
  return [parseInt(v.slice(0, 2), 16), parseInt(v.slice(2, 4), 16), parseInt(v.slice(4, 6), 16)];
}

function luminance([r, g, b]: [number, number, number]): number {
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

/**
 * WCAG 2 contrast ratio of two hex colours.
 *
 * @param a - First colour.
 * @param b - Second colour.
 * @returns Ratio ≥ 1, or 1 for invalid input.
 */
export function contrastRatio(a: string, b: string): number {
  const ra = hexToRgb(a);
  const rb = hexToRgb(b);
  if (!ra || !rb) {
    return 1;
  }
  const [hi, lo] = [luminance(ra), luminance(rb)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

/**
 * Mixes two hex colours in sRGB, like CSS `color-mix(in srgb, a p%, b)`.
 *
 * @param a - Colour weighted by `amount`.
 * @param b - Other colour.
 * @param amount - Weight of `a`, 0…1.
 * @returns Mixed hex colour.
 */
export function mixHex(a: string, b: string, amount: number): string {
  const ra = hexToRgb(a) ?? [0, 0, 0];
  const rb = hexToRgb(b) ?? [0, 0, 0];
  return rgbToHex(...(ra.map((v, i) => v * amount + rb[i]! * (1 - amount)) as [number, number, number]));
}

/**
 * Loads an image (CORS-safe sources: data:, blob:, same origin, Tauri asset).
 *
 * @param src - Image URL.
 * @returns The decoded image.
 */
export function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = src;
  });
}

/**
 * Finds a characteristic, reasonably saturated colour of an image by
 * bucketing hues weighted by chroma (a light-weight stand-in for Material's
 * quantiser + score).
 *
 * @param src - Image URL.
 * @param fallback - Returned for greyscale images or on errors.
 * @returns Hex colour.
 */
export async function extractColor(src: string, fallback = '#8e8e93'): Promise<string> {
  try {
    const img = await loadImage(src);
    const canvas = document.createElement('canvas');
    canvas.width = 48;
    canvas.height = 48;
    const ctx = canvas.getContext('2d', { willReadFrequently: true });
    if (!ctx) {
      return fallback;
    }
    ctx.drawImage(img, 0, 0, 48, 48);
    const data = ctx.getImageData(0, 0, 48, 48).data;
    const bins = Array.from({ length: 24 }, () => ({ w: 0, r: 0, g: 0, b: 0 }));
    for (let i = 0; i < data.length; i += 4) {
      const r = data[i]! / 255;
      const g = data[i + 1]! / 255;
      const b = data[i + 2]! / 255;
      const max = Math.max(r, g, b);
      const min = Math.min(r, g, b);
      const chroma = max - min;
      const light = (max + min) / 2;
      if (chroma < 0.08 || light < 0.1 || light > 0.94) {
        continue;
      }
      let hue: number;
      if (max === r) {
        hue = ((g - b) / chroma) % 6;
      } else if (max === g) {
        hue = (b - r) / chroma + 2;
      } else {
        hue = (r - g) / chroma + 4;
      }
      const bin = bins[Math.floor((((hue * 60) + 360) % 360) / 15)]!;
      const weight = chroma * chroma * (1 - Math.abs(light - 0.5));
      bin.w += weight;
      bin.r += data[i]! * weight;
      bin.g += data[i + 1]! * weight;
      bin.b += data[i + 2]! * weight;
    }
    const best = bins.reduce((a, b) => (b.w > a.w ? b : a));
    return best.w === 0 ? fallback : rgbToHex(best.r / best.w, best.g / best.w, best.b / best.w);
  } catch {
    return fallback;
  }
}
