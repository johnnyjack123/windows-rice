/**
 * Pre-renders the glass layers from the wallpaper once per change, so the
 * GPU never blurs live (a shaped window cannot blur the desktop anyway).
 */
import type { WallpaperInfo } from '$backend/contract';
import { loadImage } from './color';

/** Blob URLs of the two pre-rendered glass layers. */
export interface GlassLayers {
  blurred: string;
  rim: string;
}

function drawFitted(
  ctx: CanvasRenderingContext2D,
  img: HTMLImageElement,
  width: number,
  height: number,
  fit: WallpaperInfo['fit'],
  pad: number,
): void {
  const iw = img.naturalWidth || width;
  const ih = img.naturalHeight || height;
  let scaleX: number;
  let scaleY: number;
  if (fit === 'stretch') {
    scaleX = width / iw;
    scaleY = height / ih;
  } else {
    const s = fit === 'fit' ? Math.min(width / iw, height / ih) : fit === 'center' ? 1 : Math.max(width / iw, height / ih);
    scaleX = s;
    scaleY = s;
  }
  const dw = iw * scaleX;
  const dh = ih * scaleY;
  // Enlarged copy underneath keeps the blur from fading at the edges.
  ctx.drawImage(img, (width - dw) / 2 - pad, (height - dh) / 2 - pad, dw + pad * 4, dh + pad * 4);
  ctx.drawImage(img, (width - dw) / 2 + pad, (height - dh) / 2 + pad, dw, dh);
}

function render(
  img: HTMLImageElement,
  width: number,
  height: number,
  fit: WallpaperInfo['fit'],
  filter: string,
  pad: number,
): HTMLCanvasElement {
  const w = Math.max(1, Math.round(width));
  const h = Math.max(1, Math.round(height));
  const padded = document.createElement('canvas');
  padded.width = w + pad * 2;
  padded.height = h + pad * 2;
  const p = padded.getContext('2d');
  const out = document.createElement('canvas');
  out.width = w;
  out.height = h;
  const o = out.getContext('2d');
  if (!p || !o) {
    return out;
  }
  drawFitted(p, img, w, h, fit, pad);
  o.filter = filter;
  o.drawImage(padded, -pad, -pad);
  return out;
}

function toBlobUrl(canvas: HTMLCanvasElement): Promise<string> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => (blob ? resolve(URL.createObjectURL(blob)) : reject(new Error('toBlob failed'))), 'image/jpeg', 0.9);
  });
}

/**
 * Renders a strongly blurred body and a lightly blurred rim at quarter and
 * half the monitor resolution.
 *
 * @param info - Wallpaper of the island's monitor.
 * @returns Blob URLs, or null when there is no wallpaper image.
 */
export async function prepareGlass(info: WallpaperInfo): Promise<GlassLayers | null> {
  if (!info.url) {
    return null;
  }
  const img = await loadImage(info.url);
  const { width, height } = info.monitor;
  const blurred = render(img, width / 4, height / 4, info.fit, 'blur(9px) saturate(1.7) brightness(1.03)', 24);
  const rim = render(img, width / 2, height / 2, info.fit, 'blur(1.5px) saturate(1.9) contrast(1.05)', 4);
  return { blurred: await toBlobUrl(blurred), rim: await toBlobUrl(rim) };
}
