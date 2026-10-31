/**
 * WebGL2 renderer of the voxel mascot. One hidden context draws every mascot
 * on the page and copies each frame into that mascot's own 2D canvas, so any
 * number of mascots (the island, the gallery's theme columns) cost one GPU
 * context. Frames run only while a mascot animates; an idle context is
 * released after a few seconds.
 */
import { hexToRgb, toHex } from '$design/color';
import { motionSettings } from '$motion/spring';
import { chain, lookAt, perspective, rotateX, rotateY, rotateZ, scale, translate, type Mat4 } from './mat4';
import { buildVoxels, CENTER, EYE_Y, GROUND, PART, PART_COUNT, SHOULDER } from './model';
import { pose, type MascotMood, type Pose } from './motion';

/** Width / height of every mascot canvas (the camera frames this). */
export const MASCOT_ASPECT = 1.5;

/**
 * Releases the GPU context this long after the last frame (ms): long enough
 * that a burst of requests reuses it, short enough that it is gone at rest.
 */
const RELEASE_AFTER = 90_000;

/**
 * Camera: slightly above, so the top and the shadow read. It frames about
 * 21 × 14 units: the figure (16 × 10) with room above for jumps.
 */
const TARGET: [number, number, number] = [CENTER.x, 6, CENTER.z];
const EYE: [number, number, number] = [CENTER.x, 6 + 4.5, CENTER.z + 32.6];
const VIEW_PROJ = chain(perspective((24 * Math.PI) / 180, MASCOT_ASPECT, 1, 100), lookAt(EYE, TARGET));

/** Shadow ellipse on the ground (grid units). */
const SHADOW = { rx: 7.2, rz: 3.8 };

/**
 * How far in front of the screen the mascot imagines the viewer (CSS px):
 * the cursor this far to the side turns its head by 45°.
 */
const LOOK_DISTANCE = 420;
/** How quickly the gaze catches up with the cursor (1/s). */
const LOOK_RATE = 7;

type Rgb = [number, number, number];

/** Colours of one mascot, 0…1. */
interface Colors {
  body: Rgb;
  eye: Rgb;
  shadow: Rgb;
  shadowAlpha: number;
}

/** Controls one mascot canvas. */
export interface MascotHandle {
  /** Starts the mood's motion (one-shots restart). */
  setMood(mood: MascotMood): void;
  /** Animates while true; otherwise the last frame stays. */
  setActive(active: boolean): void;
  /**
   * Turns towards a point, given relative to the mascot's centre in CSS px
   * (x right, y down); `null` returns to its own looking around.
   */
  lookAt(dx: number | null, dy?: number): void;
  /** Re-reads size, colours and the motion setting (theme changes). */
  refresh(): void;
  /** Stops drawing into the canvas. */
  dispose(): void;
}

/**
 * Transforms of the movable parts for one pose.
 *
 * @param p - Pose of the frame.
 * @returns `PART_COUNT` matrices, indexed by `PART`.
 */
export function partMatrices(p: Pose): Mat4[] {
  const { x: cx, y: cy, z: cz } = CENTER;
  const wide = 1 / Math.sqrt(Math.max(p.squash, 0.1));
  // Pitch last: the figure keeps looking down at the viewer while it turns.
  const root = chain(
    translate(0, p.y, 0),
    translate(cx, cy, cz),
    rotateX(p.pitch),
    rotateY(p.yaw),
    rotateZ(p.roll),
    translate(-cx, -cy, -cz),
    translate(cx, GROUND, cz),
    scale(wide, p.squash, wide),
    translate(-cx, -GROUND, -cz),
  );
  const eyes = chain(root, translate(0, EYE_Y, 0), scale(1, Math.max(p.blink, 0.12), 1), translate(0, -EYE_Y, 0));
  const arm = (pivot: { x: number; y: number }, angle: number): Mat4 =>
    chain(root, translate(pivot.x, pivot.y, 0), rotateZ(angle), translate(-pivot.x, -pivot.y, 0));
  const parts: Mat4[] = [];
  parts[PART.body] = root;
  parts[PART.eyeLeft] = eyes;
  parts[PART.eyeRight] = eyes;
  // The left arm points to −x: raising it turns clockwise.
  parts[PART.armLeft] = arm(SHOULDER.left, -p.armLeft);
  parts[PART.armRight] = arm(SHOULDER.right, p.armRight);
  p.legs.forEach((lift, i) => {
    parts[PART.leg + i] = chain(root, translate(0, lift, 0));
  });
  return parts;
}

const VOXEL_VS = `#version 300 es
layout(location = 0) in vec3 a_corner;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec3 a_voxel;
layout(location = 3) in vec2 a_meta;
uniform mat4 u_viewProj;
uniform mat4 u_parts[${PART_COUNT}];
uniform vec3 u_colors[2];
out vec3 v_normal;
out vec3 v_local;
flat out vec3 v_face;
out vec3 v_color;
void main() {
  mat4 part = u_parts[int(a_meta.x)];
  v_normal = mat3(part) * a_normal;
  v_local = a_corner;
  v_face = a_normal;
  v_color = u_colors[int(a_meta.y)];
  gl_Position = u_viewProj * part * vec4(a_voxel + a_corner, 1.0);
}`;

const VOXEL_FS = `#version 300 es
precision mediump float;
in vec3 v_normal;
in vec3 v_local;
flat in vec3 v_face;
in vec3 v_color;
uniform vec3 u_light;
out vec4 o_color;
void main() {
  float diffuse = max(dot(normalize(v_normal), u_light), 0.0);
  float light = 0.62 + 0.38 * diffuse;
  // Slightly darker voxel edges keep the pixel look in 3D.
  vec3 across = abs(v_local) * (vec3(1.0) - abs(v_face));
  float edge = smoothstep(0.36, 0.5, max(across.x, max(across.y, across.z)));
  o_color = vec4(v_color * light * (1.0 - 0.1 * edge), 1.0);
}`;

const SHADOW_VS = `#version 300 es
layout(location = 0) in vec2 a_corner;
uniform mat4 u_viewProj;
uniform vec4 u_ellipse;
uniform float u_ground;
out vec2 v_uv;
void main() {
  v_uv = a_corner;
  gl_Position = u_viewProj * vec4(u_ellipse.x + a_corner.x * u_ellipse.z, u_ground, u_ellipse.y + a_corner.y * u_ellipse.w, 1.0);
}`;

const SHADOW_FS = `#version 300 es
precision mediump float;
in vec2 v_uv;
uniform vec4 u_shadow;
out vec4 o_color;
void main() {
  float a = u_shadow.a * (1.0 - smoothstep(0.3, 1.0, length(v_uv)));
  o_color = vec4(u_shadow.rgb * a, a);
}`;

/** Unit cube around the origin: 36 vertices of (corner, normal). */
function cube(): Float32Array {
  const data: number[] = [];
  for (let axis = 0; axis < 3; axis += 1) {
    for (const sign of [-1, 1]) {
      const b = (axis + 1) % 3;
      const c = (axis + 2) % 3;
      const corner = (u: number, v: number): void => {
        const p = [0, 0, 0];
        const n = [0, 0, 0];
        p[axis] = sign * 0.5;
        p[b] = u * 0.5;
        p[c] = v * 0.5;
        n[axis] = sign;
        data.push(...p, ...n);
      };
      for (const [u, v] of [[-1, -1], [1, -1], [1, 1], [-1, -1], [1, 1], [-1, 1]] as const) {
        corner(u, v);
      }
    }
  }
  return new Float32Array(data);
}

interface Programs {
  gl: WebGL2RenderingContext;
  voxel: WebGLProgram;
  voxelVao: WebGLVertexArrayObject;
  voxelCount: number;
  shadow: WebGLProgram;
  shadowVao: WebGLVertexArrayObject;
  u: Record<string, WebGLUniformLocation | null>;
}

/** Shaders handed to the driver, possibly still compiling. */
interface Compiling {
  gl: WebGL2RenderingContext;
  voxel: WebGLProgram;
  shadow: WebGLProgram;
  /** `KHR_parallel_shader_compile`: lets us ask instead of wait. */
  parallel: { COMPLETION_STATUS_KHR: number } | null;
}

/** Starts compiling and linking without waiting for the driver. */
function startPrograms(canvas: HTMLCanvasElement): Compiling | null {
  const gl = canvas.getContext('webgl2', { alpha: true, premultipliedAlpha: true, antialias: true, powerPreference: 'low-power' });
  if (!gl) {
    return null;
  }
  const parallel = gl.getExtension('KHR_parallel_shader_compile') as { COMPLETION_STATUS_KHR: number } | null;
  const program = (vs: string, fs: string): WebGLProgram => {
    const p = gl.createProgram();
    for (const [type, source] of [[gl.VERTEX_SHADER, vs], [gl.FRAGMENT_SHADER, fs]] as const) {
      const shader = gl.createShader(type)!;
      gl.shaderSource(shader, source);
      gl.compileShader(shader);
      gl.attachShader(p, shader);
    }
    gl.linkProgram(p);
    return p;
  };
  return { gl, voxel: program(VOXEL_VS, VOXEL_FS), shadow: program(SHADOW_VS, SHADOW_FS), parallel };
}

/** Whether the driver finished (always "yes" without the extension: then the first query waits). */
function compiled(c: Compiling): boolean {
  const done = (p: WebGLProgram): boolean => !c.parallel || Boolean(c.gl.getProgramParameter(p, c.parallel.COMPLETION_STATUS_KHR));
  return done(c.voxel) && done(c.shadow);
}

/** Checks the compiled programs and uploads the model. */
function finishPrograms({ gl, voxel, shadow }: Compiling): Programs | null {
  for (const p of [voxel, shadow]) {
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) {
      const logs = gl.getAttachedShaders(p)?.map((s) => gl.getShaderInfoLog(s)) ?? [];
      console.warn('[mascot]', gl.getProgramInfoLog(p), ...logs);
      return null;
    }
  }

  const voxels = buildVoxels();
  const voxelVao = gl.createVertexArray();
  gl.bindVertexArray(voxelVao);
  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, cube(), gl.STATIC_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 24, 0);
  gl.enableVertexAttribArray(1);
  gl.vertexAttribPointer(1, 3, gl.FLOAT, false, 24, 12);
  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(voxels.flatMap((v) => [v.x, v.y, v.z, v.part, v.color])), gl.STATIC_DRAW);
  gl.enableVertexAttribArray(2);
  gl.vertexAttribPointer(2, 3, gl.FLOAT, false, 20, 0);
  gl.vertexAttribDivisor(2, 1);
  gl.enableVertexAttribArray(3);
  gl.vertexAttribPointer(3, 2, gl.FLOAT, false, 20, 12);
  gl.vertexAttribDivisor(3, 1);

  const shadowVao = gl.createVertexArray();
  gl.bindVertexArray(shadowVao);
  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, 1, 1, -1, -1, 1, 1, -1, 1]), gl.STATIC_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
  gl.bindVertexArray(null);

  const u: Programs['u'] = {};
  for (const name of ['u_viewProj', 'u_parts', 'u_colors', 'u_light']) {
    u[`voxel.${name}`] = gl.getUniformLocation(voxel, name);
  }
  for (const name of ['u_viewProj', 'u_ellipse', 'u_ground', 'u_shadow']) {
    u[`shadow.${name}`] = gl.getUniformLocation(shadow, name);
  }
  return { gl, voxel, voxelVao, voxelCount: voxels.length, shadow, shadowVao, u };
}

/** Light from the upper left front, normalised. */
const LIGHT = (() => {
  const v = [-0.45, 0.8, 0.55];
  const len = Math.hypot(...v);
  return new Float32Array(v.map((c) => c / len));
})();

/** One mascot on the page. */
class Slot implements MascotHandle {
  ctx: CanvasRenderingContext2D | null;
  mood: MascotMood = 'idle';
  /** Seconds of motion since it was attached / since the mood began. */
  clock = 0;
  moodClock = 0;
  active = false;
  dirty = true;
  /** Where the cursor is (radians), and the smoothed gaze following it. */
  private gazeTarget: { yaw: number; pitch: number } | null = null;
  private gaze = { yaw: 0, pitch: 0, weight: 0 };
  colors: Colors = { body: [0.85, 0.47, 0.34], eye: [0.1, 0.08, 0.07], shadow: [0, 0, 0], shadowAlpha: 0.3 };

  constructor(
    readonly canvas: HTMLCanvasElement,
    private readonly stage: Stage,
    /** Drawn once and never animated (gallery); otherwise drawn only while active. */
    readonly still: boolean,
  ) {
    this.ctx = canvas.getContext('2d');
    this.refresh();
  }

  setMood(mood: MascotMood): void {
    this.mood = mood;
    this.moodClock = 0;
    this.dirty = true;
    this.stage.request();
  }

  setActive(active: boolean): void {
    this.active = active;
    this.dirty = true;
    this.stage.request();
  }

  lookAt(dx: number | null, dy = 0): void {
    const clamp = (v: number, min: number, max: number): number => Math.min(max, Math.max(min, v));
    // Below the island is where the cursor usually is: looking down may go further than up.
    this.gazeTarget =
      dx === null ? null : { yaw: clamp(Math.atan2(dx, LOOK_DISTANCE), -1, 1), pitch: clamp(Math.atan2(dy, LOOK_DISTANCE) * 0.6, -0.25, 0.45) };
  }

  /** Lets the gaze catch up with the cursor (frame-rate independent). */
  follow(dt: number): void {
    const k = 1 - Math.exp(-dt * LOOK_RATE);
    const target = this.gazeTarget;
    this.gaze.weight += ((target ? 1 : 0) - this.gaze.weight) * k;
    if (target) {
      this.gaze.yaw += (target.yaw - this.gaze.yaw) * k;
      this.gaze.pitch += (target.pitch - this.gaze.pitch) * k;
    }
  }

  refresh(): void {
    const dpr = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.round(this.canvas.clientWidth * dpr));
    const height = Math.max(1, Math.round(width / MASCOT_ASPECT));
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
    // One probe element for all tokens: a single style recalculation.
    const probe = document.createElement('span');
    probe.style.display = 'none';
    (this.canvas.parentElement ?? document.body).append(probe);
    const style = getComputedStyle(probe);
    const rgb = (token: string, fallback: Rgb): Rgb => {
      probe.style.color = `var(${token})`;
      const hex = toHex(style.color);
      const channels = hex ? hexToRgb(hex) : null;
      return channels ? (channels.map((c) => c / 255) as Rgb) : fallback;
    };
    const alpha = parseFloat(style.getPropertyValue('--mascot-shadow-opacity'));
    this.colors = {
      body: rgb('--mascot-body', this.colors.body),
      eye: rgb('--mascot-eye', this.colors.eye),
      shadow: rgb('--mascot-shadow', this.colors.shadow),
      shadowAlpha: Number.isFinite(alpha) ? alpha : this.colors.shadowAlpha,
    };
    probe.remove();
    this.dirty = true;
    this.stage.request();
  }

  dispose(): void {
    this.stage.remove(this);
  }

  /** Whether this mascot moves on its own right now. */
  get moving(): boolean {
    return this.active && !this.still && !motionSettings.reduced;
  }

  /** Whether a pending change must be drawn (hidden island views wait until shown). */
  get needsFrame(): boolean {
    return this.moving || (this.dirty && (this.active || this.still));
  }

  /** The pose to draw: still mascots stand calmly with open eyes. */
  get pose(): Pose {
    if (!this.moving) {
      return { ...pose('idle', 0, 0), blink: 1 };
    }
    const p = pose(this.mood, this.clock, this.moodClock);
    const w = this.gaze.weight;
    if (w > 0.001) {
      // The head follows the cursor; the mood's own turning stays as a hint,
      // except for the jump's spin and the head shake, which play in full.
      const own = this.mood === 'yes' || this.mood === 'no' ? 1 : this.mood === 'waiting' ? 0.15 : 0.35;
      p.yaw = p.yaw * (1 - w + w * own) + this.gaze.yaw * w;
      p.pitch += this.gaze.pitch * w;
    }
    return p;
  }
}

/** The shared context and the frame loop. */
class Stage {
  private readonly slots = new Set<Slot>();
  private programs: Programs | null = null;
  private compiling: Compiling | null = null;
  private canvas: HTMLCanvasElement | null = null;
  private frame = 0;
  private last = 0;
  private releaseTimer: ReturnType<typeof setTimeout> | undefined;
  private failed = false;

  attach(canvas: HTMLCanvasElement, still: boolean): Slot {
    const slot = new Slot(canvas, this, still);
    this.slots.add(slot);
    return slot;
  }

  remove(slot: Slot): void {
    this.slots.delete(slot);
  }

  /** Schedules a frame. */
  request(): void {
    clearTimeout(this.releaseTimer);
    if (!this.frame && !this.failed) {
      this.frame = requestAnimationFrame(this.tick);
    }
  }

  private readonly tick = (now: number): void => {
    this.frame = 0;
    const dt = this.last ? Math.min(now - this.last, 100) / 1000 : 0;
    this.last = now;
    let animating = false;
    let waiting = false;
    for (const slot of this.slots) {
      const moving = slot.moving;
      if (moving) {
        slot.clock += dt * motionSettings.timeScale;
        slot.moodClock += dt * motionSettings.timeScale;
        slot.follow(dt * motionSettings.timeScale);
      }
      if (slot.needsFrame) {
        const result = this.draw(slot);
        slot.dirty &&= result !== 'drawn';
        waiting ||= result === 'compiling';
      }
      animating ||= moving;
    }
    if (animating || waiting) {
      this.request();
    } else {
      this.last = 0;
      if (this.programs || this.compiling) {
        this.releaseTimer = setTimeout(() => this.release(), RELEASE_AFTER);
      }
    }
  };

  /**
   * The context, once the driver has compiled the shaders. Compiling runs in
   * the background where the driver allows it (KHR_parallel_shader_compile),
   * so a mascot appearing never stalls the island's animation; it shows up a
   * few frames later instead.
   */
  private context(): Programs | 'compiling' | null {
    if ((this.programs ?? this.compiling)?.gl.isContextLost()) {
      this.release();
    }
    if (this.programs) {
      return this.programs;
    }
    if (this.failed) {
      return null;
    }
    if (!this.compiling) {
      const canvas = document.createElement('canvas');
      this.compiling = startPrograms(canvas);
      this.canvas = canvas;
    }
    if (this.compiling && !compiled(this.compiling)) {
      return 'compiling';
    }
    this.programs = this.compiling ? finishPrograms(this.compiling) : null;
    this.compiling = null;
    if (!this.programs) {
      // No WebGL2 or broken shaders: mascots stay empty rather than retrying every frame.
      this.failed = true;
      this.canvas = null;
    }
    return this.programs;
  }

  /** Frees the GPU memory; the next frame creates a new context. */
  private release(): void {
    (this.programs ?? this.compiling)?.gl.getExtension('WEBGL_lose_context')?.loseContext();
    this.programs = null;
    this.compiling = null;
    this.canvas = null;
  }

  private draw(slot: Slot): 'drawn' | 'compiling' | 'failed' {
    const p = this.context();
    const canvas = this.canvas;
    if (p === 'compiling') {
      return p;
    }
    if (!p || !canvas || !slot.ctx) {
      return 'failed';
    }
    const { gl, u } = p;
    const { width, height } = slot.canvas;
    if (canvas.width < width || canvas.height < height) {
      canvas.width = Math.max(canvas.width, width);
      canvas.height = Math.max(canvas.height, height);
    }
    gl.viewport(0, 0, width, height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

    const current = slot.pose;
    const c = slot.colors;

    // Shadow: smaller and fainter the higher the figure is.
    const lift = 1 / (1 + 0.28 * Math.max(0, current.y));
    gl.disable(gl.DEPTH_TEST);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.useProgram(p.shadow);
    gl.uniformMatrix4fv(u['shadow.u_viewProj']!, false, VIEW_PROJ);
    gl.uniform4f(u['shadow.u_ellipse']!, CENTER.x, CENTER.z, SHADOW.rx * lift, SHADOW.rz * lift);
    gl.uniform1f(u['shadow.u_ground']!, GROUND);
    gl.uniform4f(u['shadow.u_shadow']!, c.shadow[0], c.shadow[1], c.shadow[2], c.shadowAlpha * lift);
    gl.bindVertexArray(p.shadowVao);
    gl.drawArrays(gl.TRIANGLES, 0, 6);

    gl.disable(gl.BLEND);
    gl.enable(gl.DEPTH_TEST);
    gl.useProgram(p.voxel);
    gl.uniformMatrix4fv(u['voxel.u_viewProj']!, false, VIEW_PROJ);
    const parts = new Float32Array(PART_COUNT * 16);
    partMatrices(current).forEach((m, i) => parts.set(m, i * 16));
    gl.uniformMatrix4fv(u['voxel.u_parts']!, false, parts);
    gl.uniform3fv(u['voxel.u_colors']!, new Float32Array([...c.body, ...c.eye]));
    gl.uniform3fv(u['voxel.u_light']!, LIGHT);
    gl.bindVertexArray(p.voxelVao);
    gl.drawArraysInstanced(gl.TRIANGLES, 0, 36, p.voxelCount);
    gl.bindVertexArray(null);

    // WebGL draws from the bottom left of its (possibly larger) canvas.
    slot.ctx.clearRect(0, 0, width, height);
    slot.ctx.drawImage(canvas, 0, canvas.height - height, width, height, 0, 0, width, height);
    return 'drawn';
  }
}

let stage: Stage | null = null;

/**
 * Lets the shared renderer draw a mascot into `canvas`. The canvas's CSS
 * width sets the size; its height follows {@link MASCOT_ASPECT}.
 *
 * @param canvas - Target canvas (2D context is taken).
 * @param still - Draw one calm frame and never animate (gallery).
 * @returns Controls for this mascot.
 */
export function attachMascot(canvas: HTMLCanvasElement, still = false): MascotHandle {
  stage ??= new Stage();
  return stage.attach(canvas, still);
}
