import { getContext, setContext } from 'svelte';
import { MockBackend } from './mock/mock-backend';
import { createTauriTransport, isTauri } from './tauri';
import type { Transport } from './transport';
import { SettingsStore, SystemInfoStore, UserThemesStore } from './stores/core.svelte';
import { AudioStore, BluetoothStore, DisplayStore, MediaStore, NetworkStore, PowerStore } from './stores/devices.svelte';
import { CalendarStore, NotificationsStore, SystemStore, TasksStore } from './stores/data.svelte';

/** Stores a window can initialise. */
export type StoreKey =
  | 'settings' | 'system' | 'themes' | 'media' | 'audio' | 'display' | 'power'
  | 'network' | 'bluetooth' | 'notifications' | 'calendar' | 'tasks';

/** Everything a window needs from the backend, as reactive stores. */
export class Backend {
  readonly settings: SettingsStore;
  readonly system: SystemInfoStore;
  readonly themes: UserThemesStore;
  readonly media: MediaStore;
  readonly audio: AudioStore;
  readonly display: DisplayStore;
  readonly power: PowerStore;
  readonly network: NetworkStore;
  readonly bluetooth: BluetoothStore;
  readonly notifications: NotificationsStore;
  readonly calendar: CalendarStore;
  readonly tasks: TasksStore;
  readonly monitor: SystemStore;

  /** @param transport - Tauri or simulated transport. */
  constructor(readonly transport: Transport) {
    this.settings = new SettingsStore(transport);
    this.system = new SystemInfoStore(transport);
    this.themes = new UserThemesStore(transport);
    this.media = new MediaStore(transport);
    this.audio = new AudioStore(transport);
    this.display = new DisplayStore(transport);
    this.power = new PowerStore(transport);
    this.network = new NetworkStore(transport);
    this.bluetooth = new BluetoothStore(transport);
    this.notifications = new NotificationsStore(transport);
    this.calendar = new CalendarStore(transport);
    this.tasks = new TasksStore(transport);
    this.monitor = new SystemStore(transport);
  }

  /** The simulated backend, when running in a browser (lab hooks). */
  get mock(): MockBackend | null {
    return this.transport instanceof MockBackend ? this.transport : null;
  }

  /**
   * Initialises the given stores in parallel. A failing store is logged and
   * keeps its empty default, so one broken service never blanks the UI.
   *
   * @param keys - Stores this window needs.
   */
  async init(keys: StoreKey[]): Promise<void> {
    const run: Record<StoreKey, () => Promise<void>> = {
      settings: () => this.settings.init(),
      system: () => this.system.init(),
      themes: () => this.themes.init(),
      media: () => this.media.init(),
      audio: () => this.audio.init(),
      display: () => this.display.init(),
      power: () => this.power.init(),
      network: () => this.network.init(),
      bluetooth: () => this.bluetooth.init(),
      notifications: () => this.notifications.init(),
      calendar: () => this.calendar.init(),
      tasks: () => this.tasks.init(),
    };
    const results = await Promise.allSettled(keys.map((k) => run[k]()));
    results.forEach((r, i) => {
      if (r.status === 'rejected') {
        console.error(`[backend] ${keys[i]} failed to initialise`, r.reason);
      }
    });
  }
}

/**
 * Creates the backend for this window: Tauri inside the app, simulated in a
 * plain browser.
 *
 * @returns A new backend.
 */
export function createBackend(): Backend {
  return new Backend(isTauri() ? createTauriTransport() : new MockBackend());
}

const KEY = Symbol('backend');

/**
 * Provides the backend to all child components.
 *
 * @param backend - Backend of this window.
 */
export function provideBackend(backend: Backend): void {
  setContext(KEY, backend);
}

/**
 * Reads the backend provided by {@link provideBackend}.
 *
 * @returns The window's backend.
 */
export function useBackend(): Backend {
  return getContext<Backend>(KEY);
}
