/**
 * Generated artwork for the simulated backend: wallpapers, album covers and
 * app icons as SVG data URLs, so the browser build needs no image assets.
 */

/**
 * Wraps SVG markup into a data URL.
 *
 * @param svg - SVG markup.
 * @returns Data URL usable as image source.
 */
export function svgUrl(svg: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

const NOISE = '<filter id="n" x="0" y="0" width="100%" height="100%"><feTurbulence type="fractalNoise" baseFrequency=".75" numOctaves="2" stitchTiles="stitch"/><feColorMatrix type="saturate" values="0"/><feComponentTransfer><feFuncA type="linear" slope=".1"/></feComponentTransfer></filter>';

/** A preset wallpaper. */
export interface Wallpaper {
  id: string;
  name: string;
  url: string;
}

/** Preset wallpapers of the lab. */
export const WALLPAPERS: Wallpaper[] = [
  {
    id: 'dunes',
    name: 'Dünen',
    url: svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" preserveAspectRatio="xMidYMid slice"><defs><linearGradient id="s" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#1d1730"/><stop offset=".36" stop-color="#55334f"/><stop offset=".6" stop-color="#c06a5d"/><stop offset=".76" stop-color="#eea27a"/></linearGradient><radialGradient id="g" cx="1240" cy="650" r="560" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#ffd9aa" stop-opacity=".9"/><stop offset=".2" stop-color="#ffb784" stop-opacity=".42"/><stop offset="1" stop-color="#ff9a6a" stop-opacity="0"/></radialGradient><linearGradient id="d1" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#dc8a69"/><stop offset="1" stop-color="#a5524b"/></linearGradient><linearGradient id="d2" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ad574e"/><stop offset="1" stop-color="#6c3040"/></linearGradient><linearGradient id="d3" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#6c3040"/><stop offset="1" stop-color="#3b1a2a"/></linearGradient><linearGradient id="d4" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#37192a"/><stop offset="1" stop-color="#1b0e18"/></linearGradient>${NOISE}</defs><rect width="1920" height="1080" fill="url(#s)"/><rect width="1920" height="1080" fill="url(#g)"/><circle cx="1240" cy="652" r="54" fill="#ffe3bf"/><path d="M0 702 C 240 642 480 652 760 692 S 1300 762 1600 692 S 1840 652 1920 662 V1080 H0Z" fill="url(#d1)"/><path d="M0 792 C 300 732 620 802 960 772 S 1500 702 1920 762 V1080 H0Z" fill="url(#d2)"/><path d="M0 882 C 420 822 820 902 1180 862 S 1700 832 1920 872 V1080 H0Z" fill="url(#d3)"/><path d="M0 982 C 500 932 980 1002 1400 962 S 1800 952 1920 972 V1080 H0Z" fill="url(#d4)"/><rect width="1920" height="1080" filter="url(#n)"/></svg>`),
  },
  {
    id: 'fjord',
    name: 'Fjord',
    url: svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" preserveAspectRatio="xMidYMid slice"><defs><linearGradient id="s" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0a1424"/><stop offset=".34" stop-color="#1b344f"/><stop offset=".6" stop-color="#4b6f8c"/><stop offset=".76" stop-color="#a9bfcc"/></linearGradient><radialGradient id="m" cx="560" cy="300" r="300" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#e6eff4" stop-opacity=".55"/><stop offset="1" stop-color="#e6eff4" stop-opacity="0"/></radialGradient><linearGradient id="f" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#5a7690"/><stop offset="1" stop-color="#2f4862"/></linearGradient><linearGradient id="k" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2b4159"/><stop offset="1" stop-color="#172a3d"/></linearGradient><linearGradient id="n2" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#132131"/><stop offset="1" stop-color="#0a1420"/></linearGradient><linearGradient id="w" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2f4b66"/><stop offset="1" stop-color="#08121d"/></linearGradient>${NOISE}</defs><rect width="1920" height="1080" fill="url(#s)"/><rect width="1920" height="1080" fill="url(#m)"/><circle cx="560" cy="300" r="24" fill="#f0f5f7"/><path d="M0 640 L160 560 L260 600 L420 470 L560 590 L700 520 L880 610 L1040 480 L1200 580 L1380 500 L1560 600 L1720 530 L1920 590 V820 H0Z" fill="url(#f)"/><path d="M0 700 L120 650 L300 720 L470 600 L640 700 L820 640 L980 730 L1160 620 L1340 720 L1500 660 L1700 740 L1920 670 V820 H0Z" fill="url(#k)"/><path d="M0 770 L200 712 L380 784 L560 726 L760 800 L1000 744 L1240 810 L1460 752 L1700 800 L1920 744 V820 H0Z" fill="url(#n2)"/><rect y="820" width="1920" height="260" fill="url(#w)"/><g transform="translate(0 1640) scale(1 -1)" opacity=".22"><path d="M0 770 L200 712 L380 784 L560 726 L760 800 L1000 744 L1240 810 L1460 752 L1700 800 L1920 744 V820 H0Z" fill="#9fb6c7"/></g><g fill="#cfe0ea" opacity=".18"><rect x="420" y="868" width="300" height="1.5"/><rect x="980" y="900" width="420" height="1.5"/><rect x="260" y="944" width="220" height="1.5"/><rect x="1300" y="980" width="260" height="1.5"/></g><rect width="1920" height="1080" filter="url(#n)"/></svg>`),
  },
  {
    id: 'graphite',
    name: 'Graphit',
    url: svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" preserveAspectRatio="xMidYMid slice"><defs><radialGradient id="a" cx="420" cy="140" r="1100" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#3a3c42"/><stop offset=".55" stop-color="#18191c"/><stop offset="1" stop-color="#0e0f11"/></radialGradient><radialGradient id="b" cx="1640" cy="980" r="700" gradientUnits="userSpaceOnUse"><stop offset="0" stop-color="#26353a" stop-opacity=".8"/><stop offset="1" stop-color="#26353a" stop-opacity="0"/></radialGradient>${NOISE}</defs><rect width="1920" height="1080" fill="url(#a)"/><rect width="1920" height="1080" fill="url(#b)"/><rect width="1920" height="1080" filter="url(#n)"/></svg>`),
  },
  {
    id: 'bloom',
    name: 'Bloom',
    url: svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" preserveAspectRatio="xMidYMid slice"><defs><filter id="b" x="-30%" y="-30%" width="160%" height="160%"><feGaussianBlur stdDeviation="130"/></filter>${NOISE}</defs><rect width="1920" height="1080" fill="#eee7dc"/><g filter="url(#b)"><circle cx="380" cy="260" r="360" fill="#f1b39a"/><circle cx="1460" cy="240" r="420" fill="#b8ccb2"/><circle cx="1120" cy="840" r="440" fill="#cdbfe0"/><circle cx="260" cy="920" r="320" fill="#f4d7a4"/></g><rect width="1920" height="1080" filter="url(#n)"/></svg>`),
  },
];

const COVER = (body: string, defs = ''): string =>
  svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" width="300" height="300" viewBox="0 0 300 300"><defs>${defs}</defs>${body}</svg>`);

/** Album covers keyed by track id. */
export const COVERS: Record<string, string> = {
  nachtfahrt: COVER(
    '<rect width="300" height="300" fill="url(#s)"/><rect width="300" height="300" fill="url(#l)"/><circle cx="198" cy="156" r="54" fill="#f28a4a"/><circle cx="198" cy="156" r="36" fill="#ffd296"/><rect y="201" width="300" height="99" fill="#0b111b"/>',
    '<linearGradient id="s" x1="0" y1="0" x2=".2" y2="1"><stop offset="0" stop-color="#2a3a55"/><stop offset=".62" stop-color="#141e30"/><stop offset="1" stop-color="#0b111b"/></linearGradient><pattern id="l" width="300" height="5" patternUnits="userSpaceOnUse"><rect width="300" height="1" fill="#fff" fill-opacity=".06"/></pattern>',
  ),
  glasshouse: COVER(
    '<rect width="300" height="300" fill="url(#g)"/><rect width="300" height="300" fill="url(#b)"/><rect width="300" height="300" fill="url(#a)"/>',
    '<linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#f7a9be"/><stop offset=".68" stop-color="#a03c7d"/><stop offset="1" stop-color="#4a1946"/></linearGradient><radialGradient id="a" cx=".3" cy=".28" r=".3"><stop offset="0" stop-color="#ffe4ea"/><stop offset="1" stop-color="#ffe4ea" stop-opacity="0"/></radialGradient><radialGradient id="b" cx=".72" cy=".72" r=".52"><stop offset=".3" stop-color="#c3306b"/><stop offset="1" stop-color="#c3306b" stop-opacity="0"/></radialGradient>',
  ),
  lowtide: COVER(
    '<rect width="300" height="300" fill="url(#g)"/><ellipse cx="150" cy="174" rx="110" ry="30" fill="#8fd6e8" fill-opacity=".7" filter="url(#f)"/><rect y="172" width="300" height="3" fill="#f0faff" fill-opacity=".85"/>',
    '<linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0c2236"/><stop offset=".58" stop-color="#1f5b7b"/><stop offset=".58" stop-color="#0f2f45"/><stop offset="1" stop-color="#08192a"/></linearGradient><filter id="f"><feGaussianBlur stdDeviation="14"/></filter>',
  ),
  leipzig: COVER(
    '<rect width="100" height="300" fill="#d9b23c"/><rect x="100" width="100" height="300" fill="#2f6b4f"/><rect x="200" width="100" height="300" fill="#c9442f"/><circle cx="150" cy="150" r="63" fill="#ece5d3"/>',
  ),
};

const GLYPHS: Record<string, string> = {
  chat: '<path d="M14 15h20a3 3 0 0 1 3 3v11a3 3 0 0 1-3 3H21l-7 5V18a3 3 0 0 1 3-3z"/>',
  mail: '<rect x="11" y="15" width="26" height="18" rx="3"/><path d="m12 17 12 9 12-9"/>',
  forum: '<path d="M11 14h17a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H17l-6 4V16a2 2 0 0 1 2-2z"/><path d="M33 20h2a2 2 0 0 1 2 2v13l-5-3H21"/>',
  game: '<path d="M16 19h16a6 6 0 0 1 6 6v1a5 5 0 0 1-9 3l-1-1h-8l-1 1a5 5 0 0 1-9-3v-1a6 6 0 0 1 6-6z"/><path d="M18 23v5M15.5 25.5h5"/>',
  update: '<path d="M34 22a10 10 0 0 0-18-4l-2 3"/><path d="M14 14v7h7"/><path d="M14 26a10 10 0 0 0 18 4l2-3"/><path d="M34 34v-7h-7"/>',
  branch: '<circle cx="17" cy="14" r="3"/><circle cx="17" cy="34" r="3"/><circle cx="31" cy="20" r="3"/><path d="M17 17v14M31 23c0 5-6 6-11 9"/>',
  calendar: '<rect x="12" y="14" width="24" height="22" rx="3"/><path d="M12 21h24M19 11v6M29 11v6"/>',
};

/**
 * Builds a simple app icon: coloured squircle with a white line glyph.
 *
 * @param color - Background colour.
 * @param glyph - Key of a built-in glyph.
 * @returns Data URL.
 */
export function appIcon(color: string, glyph: keyof typeof GLYPHS): string {
  return svgUrl(`<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48"><rect width="48" height="48" rx="13" fill="${color}"/><g fill="none" stroke="#fff" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round">${GLYPHS[glyph]}</g></svg>`);
}
