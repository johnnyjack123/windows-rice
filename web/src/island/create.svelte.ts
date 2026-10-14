import type { Backend } from '$backend/index';
import type { ThemeEngine } from '$design/engine.svelte';
import { motionSettings } from '$motion/spring';
import { ActivityRegistry } from './activities.svelte';
import { registerBuiltinActivities } from './builtins.svelte';
import { IslandUiState, PanelState, type IslandContext } from './context.svelte';
import { IslandController } from './controller.svelte';
import { TimersStore } from './timers.svelte';
import { IslandWindowState } from './window.svelte';

/**
 * Builds everything one island needs: registry with built-in activities,
 * controller, timers and UI state.
 *
 * @param backend - Backend of the window.
 * @param engine - Theme engine of the window.
 * @returns The island context and its window state.
 */
export function createIslandContext(backend: Backend, engine: ThemeEngine): { ctx: IslandContext; windowState: IslandWindowState } {
  const registry = new ActivityRegistry(() => motionSettings.timeScale);
  const ui = new IslandUiState();
  const panel = new PanelState();
  panel.section = backend.settings.value.panel.lastSection;
  const timers = new TimersStore(() => backend.settings.value, (p) => backend.settings.patch(p));

  let resetTimer: ReturnType<typeof setTimeout> | undefined;
  const controller: IslandController = new IslandController(registry, backend.transport, {
    onExpand: (section) => {
      clearTimeout(resetTimer);
      if (section) {
        panel.section = section;
      }
    },
    onCollapse: () => {
      // Drill-in pages reset once the panel is out of sight.
      resetTimer = setTimeout(() => {
        if (!controller.expanded) {
          panel.page = 'home';
        }
      }, 450 / motionSettings.timeScale);
    },
    hoverPreview: () => backend.settings.value.island.hoverPreview,
    glass: () => engine.glass,
  });

  const ctx: IslandContext = { backend, engine, registry, controller, timers, ui, panel };
  registerBuiltinActivities(registry, ctx);
  return { ctx, windowState: new IslandWindowState() };
}
