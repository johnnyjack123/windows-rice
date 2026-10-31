/**
 * One shared cursor subscription for every mascot on show: the backend
 * reports the cursor only while at least one mascot follows it.
 */
import type { Transport } from '$backend/transport';

/** Cursor position in CSS px of the island window. */
type Listener = (point: { x: number; y: number }) => void;

class CursorFeed {
  private readonly listeners = new Set<Listener>();
  private off: (() => void) | null = null;

  constructor(private readonly transport: Transport) {}

  add(listener: Listener): () => void {
    this.listeners.add(listener);
    if (!this.off) {
      this.off = this.transport.listen('island://cursor', (point) => {
        for (const l of this.listeners) {
          l(point);
        }
      });
      void this.transport.invoke('island_track_cursor', { enabled: true }).catch(() => undefined);
    }
    return () => {
      this.listeners.delete(listener);
      if (this.listeners.size === 0 && this.off) {
        this.off();
        this.off = null;
        void this.transport.invoke('island_track_cursor', { enabled: false }).catch(() => undefined);
      }
    };
  }
}

const feeds = new WeakMap<Transport, CursorFeed>();

/**
 * Calls `listener` whenever the cursor moves, anywhere on screen.
 *
 * @param transport - Backend transport of the island window.
 * @param listener - Receives the cursor in CSS px of the window.
 * @returns Stops following.
 */
export function followCursor(transport: Transport, listener: Listener): () => void {
  let feed = feeds.get(transport);
  if (!feed) {
    feed = new CursorFeed(transport);
    feeds.set(transport, feed);
  }
  return feed.add(listener);
}
