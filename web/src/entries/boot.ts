import { createBackend, type Backend, type StoreKey } from '$backend/index';
import { clock } from '$backend/clock.svelte';
import { ThemeEngine } from '$design/engine.svelte';

/** Stores every island-like window needs. */
export const ISLAND_STORES: StoreKey[] = [
  'settings', 'system', 'themes', 'media', 'audio', 'display', 'power',
  'network', 'bluetooth', 'notifications', 'calendar', 'tasks', 'agents',
];

/**
 * Creates the backend, loads the requested stores and starts the theme
 * engine. Shared by every window entry.
 *
 * @param stores - Stores this window needs.
 * @returns Ready backend and engine.
 */
export async function boot(stores: StoreKey[]): Promise<{ backend: Backend; engine: ThemeEngine }> {
  const backend = createBackend();
  await backend.init(stores);
  const engine = new ThemeEngine(backend);
  engine.connect();
  clock.start();
  return { backend, engine };
}
