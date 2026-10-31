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

/**
 * Front silhouette, top row first: `#` body, `e` eye socket, `a` arm, `l` leg.
 * The logo's quarter-block pixels are twice as tall as wide, so every logo
 * row becomes two voxel rows.
 */
const LOGO = [
  '...############...',
  '...##e######e##...',
  '.aa############aa.',
  '...############...',
  '....l.l....l.l....',
];
const ROWS = LOGO.flatMap((row) => [row, row]);

/** Body depth in voxels. */
const DEPTH = 6;
/** Depth layers of arms and legs (the middle of the body). */
const LIMB_LAYERS = [2, 3];

/** Model centre (grid units); the motion rotates around it. */
export const CENTER = { x: (ROWS[0]!.length - 1) / 2, y: (ROWS.length - 1) / 2, z: (DEPTH - 1) / 2 };

/** Underside of the feet (grid units); squashing keeps it on the ground. */
export const GROUND = -0.5;

/** Shoulder pivots of the arms (grid units, centre of the arm rows). */
export const SHOULDER = { left: { x: 2.5, y: ROWS.length - 1 - 4.5 }, right: { x: 14.5, y: ROWS.length - 1 - 4.5 } };

/** Centre height of the eyes (blinking scales around it). */
export const EYE_Y = ROWS.length - 1 - 2.5;

/**
 * Builds the voxel list. Eye sockets are recessed: the front layer is open,
 * the layer behind holds the dark eye, the rest of the depth stays body.
 *
 * @returns Every voxel of the model.
 */
export function buildVoxels(): Voxel[] {
  const voxels: Voxel[] = [];
  const legs: number[] = [];
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
        for (const z of LIMB_LAYERS) {
          voxels.push({ x, y, z, part, color: COLOR.body });
        }
      } else if (cell === 'l') {
        // Legs are numbered by column, left to right.
        if (!legs.includes(x)) {
          legs.push(x);
        }
        for (const z of LIMB_LAYERS) {
          voxels.push({ x, y, z, part: PART.leg + legs.indexOf(x), color: COLOR.body });
        }
      }
    });
  });
  return voxels;
}
