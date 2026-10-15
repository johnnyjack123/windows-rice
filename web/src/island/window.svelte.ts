/**
 * Island window behaviour: auto-hide when a window overlaps the island zone,
 * reveal on pushing against the top edge, brief "peeks" for transient
 * events, focus loss, global hotkey and the window size the panel needs.
 */
import { untrack } from 'svelte';
import type { IslandContext } from './context.svelte';

/** Live window state reported by the backend. */
export class IslandWindowState {
  occluded = $state(false);
  fullscreen = $state(false);
  edge = $state(false);
}

/**
 * Decides whether the island hides.
 *
 * @param s - Window state.
 * @param opts - Settings and the owning activity.
 * @returns True when the island should slide out of view.
 */
export function shouldHide(
  s: { occluded: boolean; fullscreen: boolean; edge: boolean },
  opts: { autoHide: boolean; peek: boolean; hideInFullscreen: boolean; expanded: boolean; transient: boolean; critical: boolean },
): boolean {
  if (opts.expanded || s.edge || opts.critical) {
    return false;
  }
  if (s.fullscreen && opts.hideInFullscreen) {
    return true;
  }
  if (s.occluded && opts.autoHide) {
    return !(opts.peek && opts.transient);
  }
  return false;
}

/**
 * Connects window events of the backend to the island.
 *
 * @param ctx - Island context.
 * @param state - Shared window state (also read by the lab).
 * @returns Disconnect function.
 */
export function connectWindow(ctx: IslandContext, state: IslandWindowState): () => void {
  const { backend, controller, registry, engine } = ctx;
  const t = backend.transport;

  void t.invoke('island_occlusion').then((o) => {
    state.occluded = o.occluded;
    state.fullscreen = o.fullscreen;
  });
  const offs = [
    t.listen('island://occlusion', (o) => {
      state.occluded = o.occluded;
      state.fullscreen = o.fullscreen;
    }),
    t.listen('island://edge', (e) => {
      state.edge = e.active;
    }),
    t.listen('island://focus-lost', () => controller.collapse()),
    t.listen('island://toggle', () => controller.toggle()),
  ];

  let configured = '';
  const stop = $effect.root(() => {
    $effect(() => {
      const settings = backend.settings.value.island;
      const main = registry.main;
      const def = main ? registry.def(main.id) : undefined;
      const hidden = shouldHide(state, {
        autoHide: settings.autoHide,
        peek: settings.peek,
        hideInFullscreen: settings.hideInFullscreen,
        expanded: controller.expanded,
        transient: def?.transient ?? false,
        critical: def?.critical ?? false,
      });
      untrack(() => controller.setHidden(hidden));
    });

    // The window must fit the largest panel of the active theme.
    $effect(() => {
      void engine.version;
      untrack(() => {
        const cs = getComputedStyle(document.documentElement);
        const width = (parseFloat(cs.getPropertyValue('--panel-width')) || 800) + 96;
        const height = (parseFloat(cs.getPropertyValue('--island-top')) || 10) + (parseFloat(cs.getPropertyValue('--panel-height')) || 556) + 96;
        const size = { width: Math.ceil(width), height: Math.ceil(height) };
        // Theme edits bump the version constantly; resizing a native window each time is expensive.
        const key = `${size.width}x${size.height}`;
        if (key !== configured) {
          configured = key;
          void t.invoke('island_configure', size);
        }
      });
    });
  });

  return () => {
    for (const off of offs) {
      off();
    }
    stop();
  };
}
