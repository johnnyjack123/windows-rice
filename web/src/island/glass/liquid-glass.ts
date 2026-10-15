/**
 * Liquid glass: a WebGL2 renderer for the island material of glass themes.
 *
 * The island and its bubble are rounded rectangles described by a signed
 * distance field; a smooth union melts them together while the bubble splits
 * off. Inside a bezel along the edge the surface bends like a thick convex
 * lens: the wallpaper behind it is sampled further outwards (refraction),
 * once per colour channel with slightly different strength (dispersion), and
 * mixed with a pre-blurred copy (frost). A thin rim light follows the pointer.
 *
 * Honest limit: a shaped window cannot see the desktop behind it, so the
 * "background" is the pre-rendered wallpaper, aligned to screen coordinates
 * (see design/wallpaper.ts). With auto-hide on, the island rarely sits over
 * a window anyway.
 */
import type { GlassLayers } from '$design/wallpaper';
import type { GlassRenderer } from '../controller.svelte';
import type { GlassScene } from './scene';

/** Where the island window sits on its monitor. */
export interface GlassPlacement {
  monitorWidth: number;
  monitorHeight: number;
  windowX: number;
  windowY: number;
}

/** Optical parameters, read from the theme's tokens. */
export interface GlassParams {
  /** Width of the curved rim in px. */
  bezel: number;
  /** Lensing strength (1 = theme default). */
  refraction: number;
  /** Colour fringing 0…1. */
  dispersion: number;
  /** Frost of the body 0…1. */
  frost: number;
  /** Rim light strength 0…1. */
  edgeLight: number;
  /** Shadow opacity 0…1. */
  shadow: number;
  /** Tint of the compact island and of the open panel (straight RGBA, 0…1). */
  tint: [number, number, number, number];
  tintOpen: [number, number, number, number];
}

const VERTEX = `#version 300 es
in vec2 aPos;
void main() {
  gl_Position = vec4(aPos, 0.0, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision highp float;

uniform vec2 uRes;
uniform float uDpr;
uniform vec4 uMain;
uniform float uMainR;
uniform vec4 uBubble;
uniform float uBubbleR;
uniform float uK;
uniform vec2 uOrigin;
uniform vec2 uMonitor;
uniform sampler2D uSharp;
uniform sampler2D uFrost;
uniform float uHasTex;
uniform float uBend;
uniform float uBezel;
uniform float uDispersion;
uniform float uFrostAmount;
uniform vec4 uTint;
uniform vec4 uTintOpen;
uniform float uOpen;
uniform vec2 uLight;
uniform float uEdge;
uniform float uShadow;
uniform float uOpacity;

out vec4 outColor;

float sdRound(vec2 p, vec4 box, float r) {
  vec2 hs = box.zw * 0.5;
  vec2 q = abs(p - (box.xy + hs)) - hs + r;
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

float smin(float a, float b, float k) {
  if (k <= 0.0) {
    return min(a, b);
  }
  float h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
  return mix(b, a, h) - k * h * (1.0 - h);
}

float scene(vec2 p) {
  float d = sdRound(p, uMain, uMainR);
  if (uBubble.z > 0.0) {
    d = smin(d, sdRound(p, uBubble, uBubbleR), uK);
  }
  return d;
}

vec3 tap(sampler2D tex, vec2 p) {
  return texture(tex, clamp((uOrigin + p) / uMonitor, 0.0, 1.0)).rgb;
}

void main() {
  vec2 frag = gl_FragCoord.xy / uDpr;
  vec2 p = vec2(frag.x, uRes.y - frag.y);
  float d = scene(p);

  // Soft shadow under the body, offset downwards.
  float ds = scene(p - vec2(0.0, 5.0));
  float shadow = uShadow * pow(clamp(1.0 - ds / 26.0, 0.0, 1.0), 2.4);

  float cover = clamp(0.5 - d, 0.0, 1.0);
  if (cover <= 0.0) {
    outColor = vec4(0.0, 0.0, 0.0, shadow * uOpacity);
    return;
  }

  // Outward normal from the distance field.
  float e = 0.75;
  vec2 grad = vec2(scene(p + vec2(e, 0.0)) - scene(p - vec2(e, 0.0)), scene(p + vec2(0.0, e)) - scene(p - vec2(0.0, e)));
  vec2 n = dot(grad, grad) > 1e-8 ? normalize(grad) : vec2(0.0, -1.0);

  // 0 at the edge, 1 where the surface is flat again.
  float t = clamp(-d / uBezel, 0.0, 1.0);
  float bend = pow(1.0 - t, 2.4);
  vec2 offset = n * uBend * bend;

  vec3 col = uTint.rgb;
  if (uHasTex > 0.5) {
    vec2 oR = offset * (1.0 + uDispersion);
    vec2 oB = offset * (1.0 - uDispersion);
    vec3 sharp = vec3(tap(uSharp, p + oR).r, tap(uSharp, p + offset).g, tap(uSharp, p + oB).b);
    vec3 frost = vec3(tap(uFrost, p + oR).r, tap(uFrost, p + offset).g, tap(uFrost, p + oB).b);
    // The rim stays clear, the body is frosted.
    col = mix(sharp, frost, uFrostAmount * smoothstep(0.05, 0.75, t));
  }

  vec4 tint = mix(uTint, uTintOpen, uOpen);
  col = mix(col, tint.rgb, tint.a);
  // Thickness: the rim is a touch darker, like real glass seen at an angle.
  col *= 1.0 - 0.07 * bend;

  // Rim light: brightest where the edge faces the light, a faint back-light opposite.
  float facing = dot(n, uLight);
  float rim = exp(-(-d) / 1.15);
  float light = rim * (0.28 + 0.72 * pow(max(facing, 0.0), 2.0)) + rim * 0.22 * pow(max(-facing, 0.0), 3.0);
  light += bend * 0.16 * max(facing, 0.0);
  col += uEdge * light;

  float a = cover * uOpacity;
  float under = shadow * (1.0 - cover) * uOpacity;
  outColor = vec4(col * a, a + under);
}`;

const DEFAULT_LIGHT: [number, number] = [-0.45, -0.89];

/** The liquid glass renderer of one island window. */
export class LiquidGlass implements GlassRenderer {
  private gl: WebGL2RenderingContext | null = null;
  private program: WebGLProgram | null = null;
  private uniforms = new Map<string, WebGLUniformLocation | null>();
  private sharp: WebGLTexture | null = null;
  private frost: WebGLTexture | null = null;
  private hasTex = false;
  private enabled = false;
  private failed = false;
  private last: GlassScene | null = null;
  private pointer: [number, number] | null = null;
  private layersToken = 0;
  private placement: GlassPlacement = { monitorWidth: 1920, monitorHeight: 1080, windowX: 0, windowY: 0 };
  private params: GlassParams = {
    bezel: 16, refraction: 1, dispersion: 0.3, frost: 0.8, edgeLight: 0.6, shadow: 0.35,
    tint: [0.06, 0.06, 0.08, 0.3], tintOpen: [0.06, 0.06, 0.08, 0.52],
  };

  /**
   * @param canvas - Full-window canvas behind the island content.
   * @param onLost - Called when the GPU context is lost, so the CSS material can take over.
   */
  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly onLost: () => void = () => {},
  ) {}

  /** Whether the shader draws the material (glass theme on and WebGL2 available). */
  get active(): boolean {
    return this.enabled && this.gl !== null;
  }

  /**
   * Switches the renderer on for glass themes. Creates the GL context on
   * first use; without WebGL2 it stays off and the CSS material is used.
   *
   * @param on - Whether the active theme is a glass theme.
   * @returns Whether the renderer is active now.
   */
  setEnabled(on: boolean): boolean {
    this.enabled = on && !this.failed;
    if (this.enabled && !this.gl) {
      this.enabled = this.init();
    }
    if (!this.enabled) {
      this.clear();
    }
    return this.active;
  }

  /**
   * @param params - Optical parameters from the theme.
   */
  setParams(params: GlassParams): void {
    this.params = params;
    this.redraw();
  }

  /**
   * @param placement - Window position on its monitor.
   */
  setPlacement(placement: GlassPlacement): void {
    this.placement = placement;
    this.redraw();
  }

  /**
   * Loads the pre-rendered wallpaper layers as textures.
   *
   * @param layers - Blob URLs from the theme engine, or null.
   */
  async setLayers(layers: GlassLayers | null): Promise<void> {
    const token = ++this.layersToken;
    if (!layers) {
      this.hasTex = false;
      this.redraw();
      return;
    }
    try {
      const [sharp, frost] = await Promise.all([loadImage(layers.rim), loadImage(layers.blurred)]);
      if (token !== this.layersToken || !this.gl) {
        return;
      }
      this.sharp = this.upload(this.sharp, sharp);
      this.frost = this.upload(this.frost, frost);
      this.hasTex = true;
      this.redraw();
    } catch (error) {
      console.warn('[glass] wallpaper layers unavailable', error);
    }
  }

  /**
   * The rim light leans towards the pointer.
   *
   * @param x - Pointer x in viewport px, or null to reset.
   * @param y - Pointer y in viewport px.
   */
  setPointer(x: number | null, y = 0): void {
    this.pointer = x === null ? null : [x, y];
    this.redraw();
  }

  /** {@inheritDoc GlassRenderer.draw} */
  draw(scene: GlassScene): void {
    this.last = scene;
    const gl = this.gl;
    if (!this.active || !gl || !this.program) {
      return;
    }
    const dpr = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(window.innerWidth * dpr));
    const height = Math.max(1, Math.round(window.innerHeight * dpr));
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
    gl.viewport(0, 0, width, height);
    gl.disable(gl.SCISSOR_TEST);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    if (scene.opacity <= 0.001) {
      return;
    }

    // Only shade the area around the shapes (plus room for the shadow).
    const shapes = scene.bubble ? [scene.main, scene.bubble] : [scene.main];
    const pad = 40;
    const x0 = Math.min(...shapes.map((s) => s.x)) - pad;
    const y0 = Math.min(...shapes.map((s) => s.y)) - pad;
    const x1 = Math.max(...shapes.map((s) => s.x + s.w)) + pad;
    const y1 = Math.max(...shapes.map((s) => s.y + s.h)) + pad;
    gl.enable(gl.SCISSOR_TEST);
    gl.scissor(Math.floor(x0 * dpr), Math.floor(height - y1 * dpr), Math.ceil((x1 - x0) * dpr), Math.ceil((y1 - y0) * dpr));

    const p = this.params;
    const m = scene.main;
    const b = scene.bubble;
    const light = this.lightDirection(m.x + m.w / 2, m.y + m.h / 2);
    gl.useProgram(this.program);
    this.set2('uRes', window.innerWidth, window.innerHeight);
    this.set1('uDpr', dpr);
    this.set4('uMain', m.x, m.y, m.w, m.h);
    this.set1('uMainR', m.r);
    this.set4('uBubble', b?.x ?? 0, b?.y ?? 0, b?.w ?? 0, b?.h ?? 0);
    this.set1('uBubbleR', b?.r ?? 0);
    this.set1('uK', scene.liquid * 26);
    this.set2('uOrigin', this.placement.windowX, this.placement.windowY);
    this.set2('uMonitor', this.placement.monitorWidth, this.placement.monitorHeight);
    this.set1('uHasTex', this.hasTex ? 1 : 0);
    this.set1('uBend', p.bezel * 0.95 * p.refraction);
    this.set1('uBezel', Math.max(1, p.bezel));
    this.set1('uDispersion', p.dispersion * 0.35);
    this.set1('uFrostAmount', p.frost);
    this.set4('uTint', ...p.tint);
    this.set4('uTintOpen', ...p.tintOpen);
    this.set1('uOpen', scene.expanded);
    this.set2('uLight', light[0], light[1]);
    this.set1('uEdge', p.edgeLight);
    this.set1('uShadow', p.shadow);
    this.set1('uOpacity', scene.opacity);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.sharp);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.frost);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  /** Releases the GL context. */
  dispose(): void {
    this.gl?.getExtension('WEBGL_lose_context')?.loseContext();
    this.gl = null;
    this.program = null;
  }

  private redraw(): void {
    if (this.last) {
      this.draw(this.last);
    }
  }

  private clear(): void {
    const gl = this.gl;
    if (gl) {
      gl.disable(gl.SCISSOR_TEST);
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
  }

  private lightDirection(cx: number, cy: number): [number, number] {
    if (!this.pointer) {
      return DEFAULT_LIGHT;
    }
    // The light sits above the pointer, so the top rim always catches some of it.
    const dx = this.pointer[0] - cx;
    const dy = this.pointer[1] - cy - 120;
    const len = Math.hypot(dx, dy) || 1;
    return [dx / len, dy / len];
  }

  private init(): boolean {
    const gl = this.canvas.getContext('webgl2', { alpha: true, premultipliedAlpha: true, antialias: false }) as WebGL2RenderingContext | null;
    if (!gl) {
      this.failed = true;
      return false;
    }
    const program = link(gl, VERTEX, FRAGMENT);
    if (!program) {
      this.failed = true;
      return false;
    }
    this.gl = gl;
    this.program = program;
    gl.useProgram(program);
    // One oversized triangle covers the viewport; the scissor limits the work.
    const buffer = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(program, 'aPos');
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);
    gl.uniform1i(gl.getUniformLocation(program, 'uSharp'), 0);
    gl.uniform1i(gl.getUniformLocation(program, 'uFrost'), 1);
    this.sharp = this.upload(null, null);
    this.frost = this.upload(null, null);
    this.canvas.addEventListener('webglcontextlost', (e) => {
      e.preventDefault();
      this.gl = null;
      this.program = null;
      this.uniforms.clear();
      this.hasTex = false;
      this.failed = true;
      this.enabled = false;
      this.onLost();
    });
    return true;
  }

  private upload(texture: WebGLTexture | null, image: HTMLImageElement | null): WebGLTexture | null {
    const gl = this.gl;
    if (!gl) {
      return null;
    }
    const tex = texture ?? gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    if (image) {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
    } else {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([128, 128, 128, 255]));
    }
    return tex;
  }

  private location(name: string): WebGLUniformLocation | null {
    if (!this.uniforms.has(name) && this.gl && this.program) {
      this.uniforms.set(name, this.gl.getUniformLocation(this.program, name));
    }
    return this.uniforms.get(name) ?? null;
  }

  private set1(name: string, a: number): void {
    this.gl?.uniform1f(this.location(name), a);
  }

  private set2(name: string, a: number, b: number): void {
    this.gl?.uniform2f(this.location(name), a, b);
  }

  private set4(name: string, a: number, b: number, c: number, d: number): void {
    this.gl?.uniform4f(this.location(name), a, b, c, d);
  }
}

function compile(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader | null {
  const shader = gl.createShader(type);
  if (!shader) {
    return null;
  }
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    console.warn('[glass] shader:', gl.getShaderInfoLog(shader));
    gl.deleteShader(shader);
    return null;
  }
  return shader;
}

function link(gl: WebGL2RenderingContext, vertex: string, fragment: string): WebGLProgram | null {
  const vs = compile(gl, gl.VERTEX_SHADER, vertex);
  const fs = compile(gl, gl.FRAGMENT_SHADER, fragment);
  const program = gl.createProgram();
  if (!vs || !fs || !program) {
    return null;
  }
  gl.attachShader(program, vs);
  gl.attachShader(program, fs);
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    console.warn('[glass] program:', gl.getProgramInfoLog(program));
    return null;
  }
  return program;
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error(`image failed: ${url}`));
    img.src = url;
  });
}

/**
 * Parses a computed CSS colour ("rgb(…)", "rgba(…)" or "color(srgb …)").
 *
 * @param value - Computed colour string.
 * @returns Straight RGBA in 0…1, or null if the format is unknown.
 */
export function parseCssColor(value: string): [number, number, number, number] | null {
  const v = value.trim();
  const rgb = /^rgba?\(\s*([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)(?:\s*[,/]\s*([\d.]+%?))?\s*\)$/i.exec(v);
  if (rgb) {
    return [Number(rgb[1]) / 255, Number(rgb[2]) / 255, Number(rgb[3]) / 255, alpha(rgb[4])];
  }
  const srgb = /^color\(\s*srgb\s+([\d.e+-]+)\s+([\d.e+-]+)\s+([\d.e+-]+)(?:\s*\/\s*([\d.]+%?))?\s*\)$/i.exec(v);
  if (srgb) {
    return [Number(srgb[1]), Number(srgb[2]), Number(srgb[3]), alpha(srgb[4])];
  }
  return null;
}

function alpha(raw: string | undefined): number {
  if (raw === undefined) {
    return 1;
  }
  return raw.endsWith('%') ? Number(raw.slice(0, -1)) / 100 : Number(raw);
}

/**
 * Reads the optical parameters of the active theme from its tokens.
 *
 * @param root - Element carrying the theme (the document root).
 * @returns Parameters for {@link LiquidGlass.setParams}.
 */
export function readGlassParams(root: HTMLElement = document.documentElement): GlassParams {
  const cs = getComputedStyle(root);
  const num = (name: string, fallback: number): number => {
    const value = parseFloat(cs.getPropertyValue(name));
    return Number.isFinite(value) ? value : fallback;
  };
  const probe = document.createElement('span');
  probe.style.display = 'none';
  root.append(probe);
  const color = (name: string, fallback: [number, number, number, number]): [number, number, number, number] => {
    probe.style.color = `var(${name})`;
    return parseCssColor(getComputedStyle(probe).color) ?? fallback;
  };
  const params: GlassParams = {
    bezel: num('--glass-bezel', 16),
    refraction: num('--glass-refraction', 1),
    dispersion: num('--glass-dispersion', 0.3),
    frost: num('--glass-frost', 0.8),
    edgeLight: num('--glass-edge-light', 0.6),
    shadow: num('--glass-shadow', 0.35),
    tint: color('--glass-tint', [0.06, 0.06, 0.08, 0.3]),
    tintOpen: color('--glass-tint-expanded', [0.06, 0.06, 0.08, 0.52]),
  };
  probe.remove();
  return params;
}
