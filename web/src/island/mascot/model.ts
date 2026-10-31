/**
 * Voxel model of the coding-agent mascot, extruded from the pixel logo
 * Claude Code prints in the terminal (▐▛███▜▌ / ▝▜█████▛▘ / ▘▘ ▝▝).
 * Grid units: x to the right, y up, z towards the viewer.
 */

/** Movable parts; each gets its own transform per frame. */
export const PART = {
  body: 0,
  eyeLeft: 1,
  eyeRight: 2,
  armLeft: 3,
  armRight: 4,
  /** Legs are 5…8, left to right. */
  leg: 5,
} as const;

/** Number of parts (body, two eyes, two arms, four legs). */
export const PART_COUNT = 9;

/** Palette slots. */
export const COLOR = { body: 0, eye: 1 } as const;

/** One cube of the model, centred on integer grid positions. */
export interface Voxel {
  x: number;
  y: number;
  z: number;
  part: number;
  color: number;
}

/** Front silhouette, top row first: `#` body, `e` eye socket, `a` arm, `l` leg. */
const ROWS = [
  '...############...',
  '...##e######e##...',
  '.aa############aa.',
  '...############...',
  '....l.l....l.l....',
];

/** Body depth in voxels; arms and legs sit in the middle two layers. */
const DEPTH = 4;

/** Model centre (grid units); the motion rotates around it. */
export const CENTER = { x: (ROWS[0]!.length - 1) / 2, y: (ROWS.length - 1) / 2, z: (DEPTH - 1) / 2 };

/** Underside of the feet (grid units); squashing keeps it on the ground. */
export const GROUND = -0.5;

/** Shoulder pivots of the arms (grid units). */
export const SHOULDER = { left: { x: 2.5, y: 2 }, right: { x: 14.5, y: 2 } };

/** Height of the eye row (blinking scales around it). */
export const EYE_Y = 3;

/**
 * Builds the voxel list. Eye sockets are recessed: the front layer is open,
 * the layer behind holds the dark eye, the rest of the depth stays body.
 *
 * @returns Every voxel of the model.
 */
export function buildVoxels(): Voxel[] {
  const voxels: Voxel[] = [];
  let leg = 0;
  ROWS.forEach((row, index) => {
    const y = ROWS.length - 1 - index;
    [...row].forEach((cell, x) => {
      if (cell === '#') {
        for (let z = 0; z < DEPTH; z += 1) {
          voxels.push({ x, y, z, part: PART.body, color: COLOR.body });
        }
      } else if (cell === 'e') {
        const part = x < CENTER.x ? PART.eyeLeft : PART.eyeRight;
        voxels.push({ x, y, z: DEPTH - 2, part, color: COLOR.eye });
        for (let z = 0; z < DEPTH - 2; z += 1) {
          voxels.push({ x, y, z, part: PART.body, color: COLOR.body });
        }
      } else if (cell === 'a') {
        const part = x < CENTER.x ? PART.armLeft : PART.armRight;
        for (const z of [1, 2]) {
          voxels.push({ x, y, z, part, color: COLOR.body });
        }
      } else if (cell === 'l') {
        for (const z of [1, 2]) {
          voxels.push({ x, y, z, part: PART.leg + leg, color: COLOR.body });
        }
        leg += 1;
      }
    });
  });
  return voxels;
}
