/* Generated artwork (wallpapers, album covers) and wallpaper processing.
   Everything is vector or CSS so the lab works offline and without assets. */

const GRAIN_SVG = "<svg xmlns='http://www.w3.org/2000/svg' width='200' height='200'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='.85' numOctaves='3' stitchTiles='stitch'/><feColorMatrix type='saturate' values='0'/></filter><rect width='100%' height='100%' filter='url(#n)'/></svg>";

const NOISE_FILTER = '<filter id="n" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency=".75" numOctaves="2" stitchTiles="stitch"/><feColorMatrix type="saturate" values="0"/><feComponentTransfer><feFuncA type="linear" slope=".1"/></feComponentTransfer></filter>';

/**
 * Artwork catalogue and helpers for wallpapers and covers.
 */
const Art = {
  /** CSS url() of a monochrome grain texture used on album art. */
  grain: `url("data:image/svg+xml,${encodeURIComponent(GRAIN_SVG)}")`,

  /** Album covers as layered CSS gradients, keyed by track id. */
  covers: {
    nachtfahrt: [
      'linear-gradient(0deg, #0b111b 0 33%, transparent 33%)',
      'radial-gradient(circle at 66% 52%, #ffd296 0 12%, #f28a4a 12.5% 18%, transparent 18.5%)',
      'repeating-linear-gradient(0deg, rgb(255 255 255 / 0.06) 0 1px, transparent 1px 5px)',
      'linear-gradient(170deg, #2a3a55 0%, #141e30 62%, #0b111b 100%)',
    ].join(', '),
    glasshouse: [
      'radial-gradient(circle at 30% 28%, #ffe4ea 0 7%, transparent 30%)',
      'radial-gradient(circle at 72% 72%, #c3306b 0 16%, transparent 52%)',
      'linear-gradient(140deg, #f7a9be, #a03c7d 68%, #4a1946)',
    ].join(', '),
    lowtide: [
      'linear-gradient(180deg, transparent 0 57.5%, rgb(240 250 255 / 0.85) 57.5% 58.5%, transparent 58.5%)',
      'radial-gradient(ellipse 70% 26% at 50% 58%, rgb(143 214 232 / 0.75), transparent 70%)',
      'linear-gradient(180deg, #0c2236 0%, #1f5b7b 58%, #0f2f45 58%, #08192a 100%)',
    ].join(', '),
    leipzig: [
      'radial-gradient(circle at 50% 50%, #ece5d3 0 21%, transparent 21.6%)',
      'linear-gradient(90deg, #d9b23c 0 33.3%, #2f6b4f 33.3% 66.6%, #c9442f 66.6%)',
    ].join(', '),
  },

  /** Preset wallpapers with a curated seed colour for dynamic themes. */
  wallpapers: [
    {
      id: 'dunes',
      name: 'Dünen',
      seed: '#c4705a',
      svg: `<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" preserveAspectRatio="xMidYMid slice">
<defs>
<linearGradient id="s" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#1d1730"/><stop offset=".36" stop-color="#55334f"/><stop offset=".6" stop-color="#c06a5d"/><stop offset=".76" stop-color="#eea27a"/></linearGradient>
<radialGradient id="g" cx="1240" cy="650" r="560" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#ffd9aa" stop-opacity=".9"/><stop offset=".2" stop-color="#ffb784" stop-opacity=".42"/><stop offset="1" stop-color="#ff9a6a" stop-opacity="0"/></radialGradient>
<linearGradient id="d1" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#dc8a69"/><stop offset="1" stop-color="#a5524b"/></linearGradient>
<linearGradient id="d2" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ad574e"/><stop offset="1" stop-color="#6c3040"/></linearGradient>
<linearGradient id="d3" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#6c3040"/><stop offset="1" stop-color="#3b1a2a"/></linearGradient>
<linearGradient id="d4" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#37192a"/><stop offset="1" stop-color="#1b0e18"/></linearGradient>
${NOISE_FILTER}
</defs>
<rect width="1920" height="1080" fill="url(#s)"/>
<rect width="1920" height="1080" fill="url(#g)"/>
<circle cx="1240" cy="652" r="54" fill="#ffe3bf"/>
<path d="M0 702 C 240 642 480 652 760 692 S 1300 762 1600 692 S 1840 652 1920 662 V1080 H0Z" fill="url(#d1)"/>
<path d="M0 792 C 300 732 620 802 960 772 S 1500 702 1920 762 V1080 H0Z" fill="url(#d2)"/>
<path d="M0 882 C 420 822 820 902 1180 862 S 1700 832 1920 872 V1080 H0Z" fill="url(#d3)"/>
<path d="M0 982 C 500 932 980 1002 1400 962 S 1800 952 1920 972 V1080 H0Z" fill="url(#d4)"/>
<rect width="1920" height="1080" filter="url(#n)"/>
</svg>`,
    },
    {
      id: 'fjord',
      name: 'Fjord',
      seed: '#4b6f8c',
      svg: `<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" preserveAspectRatio="xMidYMid slice">
<defs>
<linearGradient id="s" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0a1424"/><stop offset=".34" stop-color="#1b344f"/><stop offset=".6" stop-color="#4b6f8c"/><stop offset=".76" stop-color="#a9bfcc"/></linearGradient>
<radialGradient id="m" cx="560" cy="300" r="300" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#e6eff4" stop-opacity=".55"/><stop offset="1" stop-color="#e6eff4" stop-opacity="0"/></radialGradient>
<linearGradient id="f" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#5a7690"/><stop offset="1" stop-color="#2f4862"/></linearGradient>
<linearGradient id="k" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2b4159"/><stop offset="1" stop-color="#172a3d"/></linearGradient>
<linearGradient id="n2" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#132131"/><stop offset="1" stop-color="#0a1420"/></linearGradient>
<linearGradient id="w" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2f4b66"/><stop offset="1" stop-color="#08121d"/></linearGradient>
${NOISE_FILTER}
</defs>
<rect width="1920" height="1080" fill="url(#s)"/>
<rect width="1920" height="1080" fill="url(#m)"/>
<circle cx="560" cy="300" r="24" fill="#f0f5f7"/>
<path d="M0 640 L160 560 L260 600 L420 470 L560 590 L700 520 L880 610 L1040 480 L1200 580 L1380 500 L1560 600 L1720 530 L1920 590 V820 H0Z" fill="url(#f)"/>
<path d="M0 700 L120 650 L300 720 L470 600 L640 700 L820 640 L980 730 L1160 620 L1340 720 L1500 660 L1700 740 L1920 670 V820 H0Z" fill="url(#k)"/>
<path d="M0 770 L200 712 L380 784 L560 726 L760 800 L1000 744 L1240 810 L1460 752 L1700 800 L1920 744 V820 H0Z" fill="url(#n2)"/>
<rect y="820" width="1920" height="260" fill="url(#w)"/>
<g transform="translate(0 1640) scale(1 -1)" opacity=".22"><path d="M0 770 L200 712 L380 784 L560 726 L760 800 L1000 744 L1240 810 L1460 752 L1700 800 L1920 744 V820 H0Z" fill="#9fb6c7"/></g>
<g fill="#cfe0ea" opacity=".18"><rect x="420" y="868" width="300" height="1.5"/><rect x="980" y="900" width="420" height="1.5"/><rect x="260" y="944" width="220" height="1.5"/><rect x="1300" y="980" width="260" height="1.5"/></g>
<rect width="1920" height="1080" filter="url(#n)"/>
</svg>`,
    },
    {
      id: 'graphite',
      name: 'Graphit',
      seed: '#7d8590',
      svg: `<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" preserveAspectRatio="xMidYMid slice">
<defs>
<radialGradient id="a" cx="420" cy="140" r="1100" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#3a3c42"/><stop offset=".55" stop-color="#18191c"/><stop offset="1" stop-color="#0e0f11"/></radialGradient>
<radialGradient id="b" cx="1640" cy="980" r="700" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#26353a" stop-opacity=".8"/><stop offset="1" stop-color="#26353a" stop-opacity="0"/></radialGradient>
<linearGradient id="c" x1="0" y1="0" x2="1" y2="1"><stop offset=".42" stop-color="#fff" stop-opacity="0"/><stop offset=".5" stop-color="#fff" stop-opacity=".035"/><stop offset=".58" stop-color="#fff" stop-opacity="0"/></linearGradient>
${NOISE_FILTER}
</defs>
<rect width="1920" height="1080" fill="url(#a)"/>
<rect width="1920" height="1080" fill="url(#b)"/>
<rect width="1920" height="1080" fill="url(#c)"/>
<rect width="1920" height="1080" filter="url(#n)"/>
</svg>`,
    },
    {
      id: 'bloom',
      name: 'Bloom',
      seed: '#8fae8b',
      svg: `<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" preserveAspectRatio="xMidYMid slice">
<defs>
<filter id="b" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="130"/></filter>
${NOISE_FILTER}
</defs>
<rect width="1920" height="1080" fill="#eee7dc"/>
<g filter="url(#b)">
<circle cx="380" cy="260" r="360" fill="#f1b39a"/>
<circle cx="1460" cy="240" r="420" fill="#b8ccb2"/>
<circle cx="1120" cy="840" r="440" fill="#cdbfe0"/>
<circle cx="260" cy="920" r="320" fill="#f4d7a4"/>
</g>
<rect width="1920" height="1080" filter="url(#n)"/>
</svg>`,
    },
  ],

  /**
   * Wraps an SVG string into a data URL.
   *
   * @param {string} svg - SVG markup.
   * @returns {string} Data URL usable as image source.
   */
  svgUrl(svg) {
    return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  },

  /**
   * Loads an image element from a URL.
   *
   * @param {string} src - Image URL (data:, blob: or http:).
   * @returns {Promise<HTMLImageElement>} The decoded image.
   */
  loadImage(src) {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = reject;
      img.src = src;
    });
  },

  /**
   * Draws an image "object-fit: cover" into a canvas of the given size and
   * applies a canvas filter. The source is first painted slightly enlarged
   * underneath so the blur does not fade towards transparent at the edges.
   *
   * @param {HTMLImageElement} img - Source image.
   * @param {number} width - Output width in px.
   * @param {number} height - Output height in px.
   * @param {string} filter - Canvas filter string.
   * @param {number} pad - Overscan in px, should be ~2× the blur radius.
   * @returns {HTMLCanvasElement} The rendered canvas.
   */
  renderCover(img, width, height, filter, pad) {
    const w = Math.max(1, Math.round(width));
    const h = Math.max(1, Math.round(height));
    const scale = Math.max(w / img.naturalWidth, h / img.naturalHeight);
    const dw = img.naturalWidth * scale;
    const dh = img.naturalHeight * scale;

    const padded = document.createElement('canvas');
    padded.width = w + pad * 2;
    padded.height = h + pad * 2;
    const p = padded.getContext('2d');
    p.drawImage(img, (w - dw) / 2 - pad, (h - dh) / 2 - pad, dw + pad * 4, dh + pad * 4);
    p.drawImage(img, (w - dw) / 2 + pad, (h - dh) / 2 + pad, dw, dh);

    const out = document.createElement('canvas');
    out.width = w;
    out.height = h;
    const o = out.getContext('2d');
    o.filter = filter;
    o.drawImage(padded, -pad, -pad);
    return out;
  },

  /**
   * Converts a canvas to an object URL.
   *
   * @param {HTMLCanvasElement} canvas - Canvas to encode.
   * @returns {Promise<string>} Blob URL of a JPEG.
   */
  canvasUrl(canvas) {
    return new Promise((resolve) => {
      canvas.toBlob((blob) => resolve(URL.createObjectURL(blob)), 'image/jpeg', 0.9);
    });
  },

  /**
   * Pre-renders the two glass layers for a wallpaper at viewport size: a
   * strongly blurred body and a lightly blurred rim. In the real app Rust
   * does this once per wallpaper change, so the GPU never blurs live.
   *
   * @param {string} src - Wallpaper URL.
   * @returns {Promise<{blurred: string, rim: string}>} Blob URLs.
   */
  async prepareGlass(src) {
    const img = await this.loadImage(src);
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const blurred = this.renderCover(img, vw / 4, vh / 4, 'blur(9px) saturate(1.7) brightness(1.03)', 24);
    const rim = this.renderCover(img, vw / 2, vh / 2, 'blur(1.5px) saturate(1.9) contrast(1.05)', 4);
    return { blurred: await this.canvasUrl(blurred), rim: await this.canvasUrl(rim) };
  },

  /**
   * Finds a characteristic, reasonably saturated colour of an image by
   * bucketing hues weighted by chroma. Mirrors what material-colors does in
   * the real app, in a much simpler form.
   *
   * @param {string} src - Image URL.
   * @returns {Promise<string>} Hex colour.
   */
  async extractSeed(src) {
    const img = await this.loadImage(src);
    const canvas = this.renderCover(img, 64, 36, 'none', 0);
    const data = canvas.getContext('2d').getImageData(0, 0, 64, 36).data;
    const bins = Array.from({ length: 24 }, () => ({ w: 0, r: 0, g: 0, b: 0 }));
    for (let i = 0; i < data.length; i += 4) {
      const r = data[i] / 255;
      const g = data[i + 1] / 255;
      const b = data[i + 2] / 255;
      const max = Math.max(r, g, b);
      const min = Math.min(r, g, b);
      const chroma = max - min;
      const light = (max + min) / 2;
      if (chroma < 0.08 || light < 0.12 || light > 0.92) {
        continue;
      }
      let hue;
      if (max === r) {
        hue = ((g - b) / chroma) % 6;
      } else if (max === g) {
        hue = (b - r) / chroma + 2;
      } else {
        hue = (r - g) / chroma + 4;
      }
      const bin = bins[Math.floor((((hue * 60) + 360) % 360) / 15)];
      const weight = chroma * chroma * (1 - Math.abs(light - 0.5));
      bin.w += weight;
      bin.r += data[i] * weight;
      bin.g += data[i + 1] * weight;
      bin.b += data[i + 2] * weight;
    }
    const best = bins.reduce((a, b) => (b.w > a.w ? b : a));
    if (best.w === 0) {
      return '#8a8f98';
    }
    const hex = (v) => Math.round(v / best.w).toString(16).padStart(2, '0');
    return `#${hex(best.r)}${hex(best.g)}${hex(best.b)}`;
  },
};
