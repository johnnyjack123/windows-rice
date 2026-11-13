/**
 * Minimal 4×4 matrices for the mascot, column-major as WebGL expects.
 */

/** A column-major 4×4 matrix. */
export type Mat4 = Float32Array;

/** @returns The identity matrix. */
export function identity(): Mat4 {
  const m = new Float32Array(16);
  m[0] = m[5] = m[10] = m[15] = 1;
  return m;
}

/**
 * Multiplies matrices left to right: `chain(a, b, c)` = a · b · c, so `c`
 * applies to a point first.
 *
 * @param matrices - Factors.
 * @returns A new matrix.
 */
export function chain(...matrices: Mat4[]): Mat4 {
  let result = identity();
  for (const m of matrices) {
    const next = new Float32Array(16);
    for (let col = 0; col < 4; col += 1) {
      for (let row = 0; row < 4; row += 1) {
        let sum = 0;
        for (let k = 0; k < 4; k += 1) {
          sum += result[k * 4 + row]! * m[col * 4 + k]!;
        }
        next[col * 4 + row] = sum;
      }
    }
    result = next;
  }
  return result;
}

/**
 * @param x - Offset along x.
 * @param y - Offset along y.
 * @param z - Offset along z.
 * @returns A translation.
 */
export function translate(x: number, y: number, z: number): Mat4 {
  const m = identity();
  m[12] = x;
  m[13] = y;
  m[14] = z;
  return m;
}

/**
 * @param x - Factor along x.
 * @param y - Factor along y.
 * @param z - Factor along z.
 * @returns A scale.
 */
export function scale(x: number, y: number, z: number): Mat4 {
  const m = identity();
  m[0] = x;
  m[5] = y;
  m[10] = z;
  return m;
}

/**
 * @param angle - Radians; positive tilts +y towards +z.
 * @returns A rotation around x.
 */
export function rotateX(angle: number): Mat4 {
  const m = identity();
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  m[5] = c;
  m[6] = s;
  m[9] = -s;
  m[10] = c;
  return m;
}

/**
 * @param angle - Radians; positive turns +z towards +x.
 * @returns A rotation around y.
 */
export function rotateY(angle: number): Mat4 {
  const m = identity();
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  m[0] = c;
  m[2] = -s;
  m[8] = s;
  m[10] = c;
  return m;
}

/**
 * @param angle - Radians; positive turns +x towards +y.
 * @returns A rotation around z.
 */
export function rotateZ(angle: number): Mat4 {
  const m = identity();
  const c = Math.cos(angle);
  const s = Math.sin(angle);
  m[0] = c;
  m[1] = s;
  m[4] = -s;
  m[5] = c;
  return m;
}

/**
 * @param fovY - Vertical field of view in radians.
 * @param aspect - Width / height.
 * @param near - Near plane distance.
 * @param far - Far plane distance.
 * @returns A perspective projection.
 */
export function perspective(fovY: number, aspect: number, near: number, far: number): Mat4 {
  const f = 1 / Math.tan(fovY / 2);
  const m = new Float32Array(16);
  m[0] = f / aspect;
  m[5] = f;
  m[10] = (far + near) / (near - far);
  m[11] = -1;
  m[14] = (2 * far * near) / (near - far);
  return m;
}

/**
 * @param eye - Camera position.
 * @param target - Point the camera looks at.
 * @returns A view matrix with +y up.
 */
export function lookAt(eye: [number, number, number], target: [number, number, number]): Mat4 {
  const norm = (v: number[]): number[] => {
    const len = Math.hypot(...v) || 1;
    return v.map((c) => c / len);
  };
  const z = norm([eye[0] - target[0], eye[1] - target[1], eye[2] - target[2]]);
  // x = up × z with up = (0, 1, 0).
  const x = norm([z[2]!, 0, -z[0]!]);
  const y = [z[1]! * x[2]! - z[2]! * x[1]!, z[2]! * x[0]! - z[0]! * x[2]!, z[0]! * x[1]! - z[1]! * x[0]!];
  const dot = (a: number[]): number => a[0]! * eye[0] + a[1]! * eye[1] + a[2]! * eye[2];
  const m = new Float32Array(16);
  m.set([x[0]!, y[0]!, z[0]!, 0, x[1]!, y[1]!, z[1]!, 0, x[2]!, y[2]!, z[2]!, 0, -dot(x), -dot(y), -dot(z), 1]);
  return m;
}

/**
 * Applies `m` to a point.
 *
 * @param m - Matrix.
 * @param p - Point.
 * @returns The transformed point (w divided out).
 */
export function apply(m: Mat4, p: [number, number, number]): [number, number, number] {
  const [x, y, z] = p;
  const w = m[3]! * x + m[7]! * y + m[11]! * z + m[15]!;
  return [
    (m[0]! * x + m[4]! * y + m[8]! * z + m[12]!) / w,
    (m[1]! * x + m[5]! * y + m[9]! * z + m[13]!) / w,
    (m[2]! * x + m[6]! * y + m[10]! * z + m[14]!) / w,
  ];
}
