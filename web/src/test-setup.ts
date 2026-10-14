// Browser APIs jsdom does not provide but the UI uses.
class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}

globalThis.ResizeObserver ??= ResizeObserverStub as unknown as typeof ResizeObserver;
globalThis.matchMedia ??= ((query: string) => ({
  matches: false,
  media: query,
  onchange: null,
  addEventListener() {},
  removeEventListener() {},
  addListener() {},
  removeListener() {},
  dispatchEvent: () => false,
})) as unknown as typeof matchMedia;
if (typeof CSS === 'undefined' || !CSS.escape) {
  (globalThis as unknown as { CSS: { escape: (v: string) => string } }).CSS = { escape: (v: string) => v.replace(/["\\]/g, '\\$&') };
}
globalThis.requestAnimationFrame ??= (cb: FrameRequestCallback): number =>
  setTimeout(() => cb(performance.now()), 16) as unknown as number;
globalThis.cancelAnimationFrame ??= (id: number): void => clearTimeout(id);
// jsdom has no canvas; colour helpers treat a missing context as "unknown".
HTMLCanvasElement.prototype.getContext = (() => null) as typeof HTMLCanvasElement.prototype.getContext;
window.open = (() => null) as typeof window.open;
Element.prototype.setPointerCapture ??= function setPointerCapture() {};
Element.prototype.releasePointerCapture ??= function releasePointerCapture() {};
Element.prototype.hasPointerCapture ??= () => false;
