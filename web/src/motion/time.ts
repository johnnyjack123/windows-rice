/**
 * Parses a CSS time ("1600ms", "8s", " 0.4s ") to milliseconds. Plain
 * `parseFloat` would read "8s" as 8 ms.
 *
 * @param value - Token value as returned by `getPropertyValue`.
 * @returns Milliseconds, or null if the value is not a time.
 */
export function cssTimeMs(value: string): number | null {
  const match = /^\s*(-?\d*\.?\d+)\s*(ms|s)\s*$/i.exec(value);
  if (!match) {
    return null;
  }
  const amount = Number(match[1]);
  return match[2]!.toLowerCase() === 's' ? amount * 1000 : amount;
}

/**
 * Reads a time token of `element` in milliseconds.
 *
 * @param element - Element whose computed style holds the token.
 * @param name - Custom property, e.g. `--break-hold`.
 * @param fallback - Milliseconds if the token is missing or not a time.
 * @returns Milliseconds.
 */
export function timeToken(element: Element, name: string, fallback: number): number {
  return cssTimeMs(getComputedStyle(element).getPropertyValue(name)) ?? fallback;
}
