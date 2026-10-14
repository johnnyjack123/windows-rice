import { getContext, setContext } from 'svelte';
import type { AppNotification, CalendarEvent } from '$backend/contract';
import type { Backend } from '$backend/index';
import type { ThemeEngine } from '$design/engine.svelte';
import type { ActivityRegistry } from './activities.svelte';
import type { IslandController } from './controller.svelte';
import type { TimersStore } from './timers.svelte';

/** Data the transient views display. */
export class IslandUiState {
  notification = $state<AppNotification | null>(null);
  osd = $state<{ kind: 'volume' | 'brightness'; value: number; muted: boolean }>({ kind: 'volume', value: 0.5, muted: false });
  event = $state<CalendarEvent | null>(null);
}

/** Panel navigation state. */
export class PanelState {
  /** Active section (tab). */
  section = $state('notifications');
  /** Page of the left column. */
  page = $state<'home' | 'wifi' | 'bluetooth'>('home');
}

/** Everything views, sections and tiles of one island need. */
export interface IslandContext {
  backend: Backend;
  engine: ThemeEngine;
  registry: ActivityRegistry;
  controller: IslandController;
  timers: TimersStore;
  ui: IslandUiState;
  panel: PanelState;
}

const KEY = Symbol('island');

/**
 * Provides the island context to child components.
 *
 * @param ctx - Context of this island.
 */
export function provideIsland(ctx: IslandContext): void {
  setContext(KEY, ctx);
}

/**
 * Reads the island context.
 *
 * @returns The context provided by the island root.
 */
export function useIsland(): IslandContext {
  return getContext<IslandContext>(KEY);
}
