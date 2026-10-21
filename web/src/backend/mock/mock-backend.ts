/**
 * Simulated backend for the browser. Implements the complete contract with
 * realistic timings and failure cases, so the whole frontend can be built and
 * reviewed without Windows APIs. The lab drives it through `debug`.
 */
import type {
  AppNotification, Appearance, ArgsOf, AudioState, AutostartState, BackendError, BluetoothState, BreakOverlay, CalendarAccount,
  CalendarEvent, CommandName, DisplayState, EventName, Events, MediaState, NetworkState,
  NotificationsState, Occlusion, PowerState, ProcessInfo, Rect, ResultOf, StoredSettings,
  SystemSample, Task, UserTheme, WallpaperInfo, WifiNetwork,
} from '../contract';
import { Emitter, type InvokeArgs, type Transport } from '../transport';
import { applyPatch } from '../settings';
import { WALLPAPERS } from './art';
import {
  CALENDARS, DEVICES, eventsOfDay, INITIAL_NOTIFICATIONS, initialTasks, NETWORKS, NOTIFICATION_POOL,
  PROCESSES, SYSTEM_FONTS, TRACKS, dayStart,
} from './data';

type Handlers = { [C in CommandName]: (args: ArgsOf<C>) => ResultOf<C> | Promise<ResultOf<C>> };

/** Events that are relayed to other open pages (settings ↔ island). */
const SHARED_EVENTS = new Set<EventName>([
  'config://changed', 'theme://user-changed', 'calendar://accounts', 'calendar://changed', 'break://state', 'break://finished',
]);

const KEYS = {
  settings: 'rice-mock-settings',
  themes: 'rice-mock-themes',
  accounts: 'rice-mock-accounts',
  break: 'rice-mock-break',
};

const sleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

function fail(code: BackendError['code'], message: string): never {
  throw { code, message } satisfies BackendError;
}

function load<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key);
    return raw === null ? fallback : (JSON.parse(raw) as T);
  } catch {
    return fallback;
  }
}

function save(key: string, value: unknown): void {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    // Storage may be unavailable; the mock then simply forgets.
  }
}

function walk(value: number, min: number, max: number, step: number): number {
  return Math.min(max, Math.max(min, value + (Math.random() - 0.5) * 2 * step));
}

/** The simulated backend. */
export class MockBackend implements Transport {
  readonly kind = 'mock' as const;

  /** Fired when the frontend asks to open a URI (the lab shows a toast). */
  readonly onShellOpen = new Emitter<string>();

  private readonly listeners = new Map<EventName, Set<(payload: unknown) => void>>();
  private readonly channel: BroadcastChannel | null =
    typeof BroadcastChannel === 'undefined' ? null : new BroadcastChannel('rice-mock');

  private settings: StoredSettings = load(KEYS.settings, {});
  private userThemes: UserTheme[] = load(KEYS.themes, []);
  private accounts: CalendarAccount[] = load<CalendarAccount[] | null>(KEYS.accounts, null) ?? [this.demoAccount()];

  private track = 0;
  private playing = true;
  private position = 84;
  private positionAt = Date.now();

  private audio: AudioState = {
    volume: 0.62, muted: false, micMuted: false, micAvailable: true,
    device: { id: 'xm5', name: 'WH-1000XM5', kind: 'bluetooth' },
    devices: [
      { id: 'xm5', name: 'WH-1000XM5', kind: 'bluetooth' },
      { id: 'speakers', name: 'Realtek Lautsprecher', kind: 'speakers' },
      { id: 'hdmi', name: 'LG UltraGear (HDMI)', kind: 'hdmi' },
    ],
  };
  private autostart: AutostartState = { enabled: true, supported: true };
  private display: DisplayState = { brightness: 0.7, nightLight: false };
  private power: PowerState = { battery: 0.82, charging: false, energySaver: false, energySaverSupported: true };
  private network: NetworkState = {
    wifi: { available: true, enabled: true, connected: NETWORKS[0]!, permission: 'granted' },
    airplane: false,
    hotspot: { supported: true, enabled: false },
  };
  private bluetooth: BluetoothState = { available: true, enabled: true, devices: DEVICES.map((d) => ({ ...d })) };
  private notifications: NotificationsState = {
    permission: 'granted',
    items: INITIAL_NOTIFICATIONS.map(({ minutesAgo, ...n }, i) => ({
      ...n, id: `n${i}`, time: new Date(Date.now() - minutesAgo * 60000).toISOString(),
    })),
  };
  private pool = 0;
  private tasks: Task[] = initialTasks();
  private extraEvents: CalendarEvent[] = [];
  private processes: ProcessInfo[] = PROCESSES.map((p) => ({ ...p }));
  private sample = { cpu: 18, gpu: 9, down: 900_000, up: 120_000, mem: 11.4 * 1024 ** 3 };
  private systemTimer: ReturnType<typeof setInterval> | null = null;
  private systemSubscribers = 0;
  private occlusion: Occlusion = { occluded: false, fullscreen: false };
  private edge = false;
  private hitRects: Rect[] = [];
  private wallpaperUrl = WALLPAPERS[0]!.url;

  constructor() {
    this.channel?.addEventListener('message', (e: MessageEvent<{ event: EventName; payload: unknown }>) => {
      this.syncFromChannel(e.data.event, e.data.payload);
      this.deliver(e.data.event, e.data.payload);
    });
    setInterval(() => this.tickMedia(), 1000);
    if (typeof window !== 'undefined') {
      window.addEventListener('resize', () => this.emit('system://wallpaper', this.wallpaper()));
      document.addEventListener('mousemove', (e) => this.trackEdge(e));
    }
  }

  // ── Transport ───────────────────────────────────────────────────────────

  async invoke<C extends CommandName>(command: C, ...args: InvokeArgs<C>): Promise<ResultOf<C>> {
    await sleep(12);
    const handler = this.handlers[command] as (a: ArgsOf<C>) => ResultOf<C> | Promise<ResultOf<C>>;
    return structuredClone(await handler(args[0] as ArgsOf<C>));
  }

  listen<E extends EventName>(event: E, handler: (payload: Events[E]) => void): () => void {
    let set = this.listeners.get(event);
    if (!set) {
      set = new Set();
      this.listeners.set(event, set);
    }
    const fn = handler as (payload: unknown) => void;
    set.add(fn);
    return () => set.delete(fn);
  }

  private emit<E extends EventName>(event: E, payload: Events[E]): void {
    const copy = structuredClone(payload);
    this.deliver(event, copy);
    if (SHARED_EVENTS.has(event)) {
      this.channel?.postMessage({ event, payload: copy });
    }
  }

  private deliver(event: EventName, payload: unknown): void {
    for (const fn of this.listeners.get(event) ?? []) {
      fn(payload);
    }
  }

  private syncFromChannel(event: EventName, payload: unknown): void {
    if (event === 'config://changed') {
      this.settings = payload as StoredSettings;
    } else if (event === 'theme://user-changed') {
      this.userThemes = payload as UserTheme[];
    } else if (event === 'calendar://accounts') {
      this.accounts = payload as CalendarAccount[];
    } else if (event === 'break://state') {
      this.setBreak(payload as BreakOverlay | null);
    }
  }

  /** Break state is shared through storage so a frame loaded later can read it. */
  private setBreak(overlay: BreakOverlay | null): void {
    save(KEYS.break, overlay);
  }

  // ── Command handlers (must cover the whole contract) ────────────────────

  private readonly handlers: Handlers = {
    app_info: () => ({ version: '0.1.0', themesDir: '%APPDATA%\\rice\\themes', configDir: '%APPDATA%\\rice' }),
    app_autostart_get: () => this.autostart,
    app_autostart_set: async ({ enabled }) => {
      await sleep(150);
      this.autostart = { ...this.autostart, enabled };
      return this.autostart;
    },
    app_quit: () => {
      // A browser tab cannot quit the app; only log the request.
      console.info('[mock] app_quit');
    },

    config_get: () => this.settings,
    config_set: ({ patch }) => {
      this.settings = applyPatch(this.settings, patch);
      save(KEYS.settings, this.settings);
      this.emit('config://changed', this.settings);
      return this.settings;
    },

    theme_list_user: () => this.userThemes,
    theme_save_user: ({ id, manifest, css }) => {
      const theme: UserTheme = { id, manifest, css };
      this.userThemes = [...this.userThemes.filter((t) => t.id !== id), theme];
      save(KEYS.themes, this.userThemes);
      this.emit('theme://user-changed', this.userThemes);
      return theme;
    },
    theme_delete_user: ({ id }) => {
      this.userThemes = this.userThemes.filter((t) => t.id !== id);
      save(KEYS.themes, this.userThemes);
      this.emit('theme://user-changed', this.userThemes);
    },

    system_appearance: () => this.appearance(),
    system_wallpaper: () => this.wallpaper(),
    system_fonts: () => SYSTEM_FONTS,
    system_monitors: () => [
      { index: 0, name: 'LG UltraGear 27GP850', primary: true, width: 2560, height: 1440 },
      { index: 1, name: 'Dell P2419H', primary: false, width: 1920, height: 1080 },
    ],

    island_configure: () => undefined,
    island_set_hit_regions: ({ rects }) => {
      this.hitRects = rects;
    },
    island_set_expanded: () => undefined,
    island_occlusion: () => this.occlusion,

    window_open_settings: ({ page }) => {
      window.open(`settings.html${page ? `#${page}` : ''}`, 'rice-settings', 'width=1120,height=780');
    },
    shell_open: ({ target }) => {
      console.info(`[mock] shell_open ${target}`);
      this.onShellOpen.emit(target);
    },

    // The lab shows the break window as a full-page frame (see LabApp).
    break_show: (overlay) => {
      this.setBreak(overlay);
      this.emit('break://state', overlay);
    },
    break_hide: () => {
      this.setBreak(null);
      this.emit('break://state', null);
    },
    break_state: () => load<BreakOverlay | null>(KEYS.break, null),
    break_finish: () => {
      this.setBreak(null);
      this.emit('break://state', null);
      this.emit('break://finished', null);
    },

    media_get: () => this.mediaState(),
    media_control: ({ action }) => {
      if (action === 'next' || action === 'previous') {
        this.skip(action === 'next' ? 1 : -1);
      } else {
        this.position = this.currentPosition();
        this.positionAt = Date.now();
        this.playing = action === 'toggle' ? !this.playing : action === 'play';
      }
      this.emit('media://state', this.mediaState());
    },
    media_seek: ({ position }) => {
      this.position = Math.max(0, Math.min(position, TRACKS[this.track]!.duration));
      this.positionAt = Date.now();
      this.emit('media://state', this.mediaState());
    },

    audio_get: () => this.audio,
    audio_set_volume: ({ volume }) => {
      this.audio = { ...this.audio, volume: Math.min(1, Math.max(0, volume)), muted: false };
      this.emit('audio://state', this.audio);
    },
    audio_set_muted: ({ muted }) => {
      this.audio = { ...this.audio, muted };
      this.emit('audio://state', this.audio);
    },
    audio_set_device: ({ id }) => {
      const device = this.audio.devices.find((d) => d.id === id) ?? fail('not_found', `no device ${id}`);
      this.audio = { ...this.audio, device };
      this.emit('audio://state', this.audio);
    },
    audio_set_mic_muted: ({ muted }) => {
      this.audio = { ...this.audio, micMuted: muted };
      this.emit('audio://state', this.audio);
    },

    display_get: () => this.display,
    display_set_brightness: ({ value }) => {
      this.display = { ...this.display, brightness: Math.min(1, Math.max(0, value)) };
      this.emit('display://state', this.display);
    },
    display_set_night_light: ({ enabled }) => {
      this.display = { ...this.display, nightLight: enabled };
      this.emit('display://state', this.display);
    },

    power_get: () => this.power,
    power_set_energy_saver: ({ enabled }) => {
      this.power = { ...this.power, energySaver: enabled };
      this.emit('power://state', this.power);
    },
    power_action: ({ action }) => {
      console.info(`[mock] power_action ${action}`);
      this.onShellOpen.emit(`power:${action}`);
    },

    network_get: () => this.network,
    network_scan: async () => {
      if (!this.network.wifi.enabled) {
        return [];
      }
      if (this.network.wifi.permission !== 'granted') {
        fail('permission_required', 'location access required for Wi-Fi scans');
      }
      await sleep(1400);
      const list = NETWORKS.filter((n) => n.ssid !== this.network.wifi.connected?.ssid)
        .map((n) => ({ ...n, signal: Math.max(1, Math.min(4, n.signal + Math.round(Math.random() - 0.4))) }));
      this.emit('network://scan', list);
      return list;
    },
    network_set_wifi: ({ enabled }) => {
      this.network = {
        ...this.network,
        airplane: enabled ? false : this.network.airplane,
        wifi: { ...this.network.wifi, enabled, connected: enabled ? NETWORKS[0]! : null },
      };
      this.emit('network://state', this.network);
    },
    network_connect: async ({ ssid, key }) => {
      const target: WifiNetwork = NETWORKS.find((n) => n.ssid === ssid) ?? fail('not_found', `unknown network ${ssid}`);
      await sleep(1500);
      if (target.secure && !target.known && (!key || key.length < 8 || /^(falsch|wrong)/i.test(key))) {
        return { ok: false, error: 'wrong_key' };
      }
      this.network = { ...this.network, wifi: { ...this.network.wifi, connected: { ...target, known: true } } };
      this.emit('network://state', this.network);
      return { ok: true };
    },
    network_disconnect: () => {
      this.network = { ...this.network, wifi: { ...this.network.wifi, connected: null } };
      this.emit('network://state', this.network);
    },
    network_set_airplane: ({ enabled }) => {
      this.network = {
        ...this.network,
        airplane: enabled,
        wifi: { ...this.network.wifi, enabled: !enabled, connected: enabled ? null : NETWORKS[0]! },
      };
      this.bluetooth = { ...this.bluetooth, enabled: !enabled };
      this.emit('network://state', this.network);
      this.emit('bluetooth://state', this.bluetooth);
    },
    network_set_hotspot: ({ enabled }) => {
      this.network = { ...this.network, hotspot: { ...this.network.hotspot, enabled } };
      this.emit('network://state', this.network);
    },

    bluetooth_get: () => this.bluetooth,
    bluetooth_set_enabled: ({ enabled }) => {
      this.bluetooth = {
        ...this.bluetooth,
        enabled,
        devices: this.bluetooth.devices.map((d) => ({ ...d, connected: enabled && d.connected })),
      };
      this.emit('bluetooth://state', this.bluetooth);
    },
    bluetooth_connect: async ({ id }) => {
      await sleep(1300);
      const device = this.bluetooth.devices.find((d) => d.id === id) ?? fail('not_found', `no device ${id}`);
      if (!device.canConnect) {
        return { ok: false, error: 'failed' };
      }
      this.setDevice(id, { connected: true, battery: device.battery ?? 0.4 + Math.random() * 0.55 });
      return { ok: true };
    },
    bluetooth_disconnect: async ({ id }) => {
      await sleep(700);
      this.setDevice(id, { connected: false });
    },

    notifications_get: () => this.notifications,
    notifications_request_access: async () => {
      await sleep(400);
      this.notifications = { ...this.notifications, permission: 'granted' };
      this.emit('notifications://state', this.notifications);
      return 'granted' as const;
    },
    notifications_dismiss: ({ id }) => {
      this.notifications = { ...this.notifications, items: this.notifications.items.filter((n) => n.id !== id) };
      this.emit('notifications://removed', { id });
    },
    notifications_clear: () => {
      const ids = this.notifications.items.map((n) => n.id);
      this.notifications = { ...this.notifications, items: [] };
      for (const id of ids) {
        this.emit('notifications://removed', { id });
      }
    },

    calendar_accounts: () => this.accounts,
    calendar_add_account: async ({ name, url, user, password }) => {
      await sleep(1600);
      if (!/^https?:\/\//i.test(url) || /offline/i.test(url)) {
        fail('network', 'server not reachable');
      }
      if (!user || /^(falsch|wrong)/i.test(password)) {
        fail('auth_failed', 'wrong user name or app password');
      }
      const account: CalendarAccount = { ...this.demoAccount(), name: name || 'Nextcloud', url, user };
      this.setAccounts([account]);
      this.emit('calendar://changed', null);
      return account;
    },
    calendar_remove_account: ({ id }) => {
      this.setAccounts(this.accounts.filter((a) => a.id !== id));
      this.emit('calendar://changed', null);
    },
    calendar_set_enabled: ({ calendarId, enabled }) => {
      this.setAccounts(this.accounts.map((a) => ({
        ...a, calendars: a.calendars.map((c) => (c.id === calendarId ? { ...c, enabled } : c)),
      })));
      this.emit('calendar://changed', null);
    },
    calendar_sync: async () => {
      this.setAccounts(this.accounts.map((a) => ({ ...a, status: 'syncing' })));
      await sleep(1200);
      this.setAccounts(this.accounts.map((a) => ({ ...a, status: 'ok', lastSync: new Date().toISOString() })));
      this.emit('calendar://changed', null);
    },
    calendar_events: ({ from, to }) => {
      if (!this.accounts.length) {
        return [];
      }
      const enabled = new Set(this.enabledCalendars());
      const out: CalendarEvent[] = [];
      for (let d = dayStart(new Date(from)); d < new Date(to); d = dayStart(d, 1)) {
        out.push(...eventsOfDay(d));
      }
      out.push(...this.extraEvents);
      return out
        .filter((e) => enabled.has(e.calendarId) && e.end > from && e.start < to)
        .sort((a, b) => a.start.localeCompare(b.start));
    },

    tasks_list: () => {
      const enabled = new Set(this.enabledCalendars());
      return this.accounts.length ? this.tasks.filter((t) => enabled.has(t.calendarId)) : [];
    },
    tasks_create: ({ calendarId, title, due }) => {
      const task: Task = { id: `t${Date.now()}`, calendarId, title, due, done: false, completedAt: null };
      this.tasks = [task, ...this.tasks];
      this.emit('tasks://changed', null);
      return task;
    },
    tasks_update: ({ id, patch }) => {
      const current = this.tasks.find((t) => t.id === id) ?? fail('not_found', `no task ${id}`);
      const next: Task = {
        ...current,
        ...patch,
        completedAt: patch.done === undefined ? current.completedAt : patch.done ? new Date().toISOString() : null,
      };
      this.tasks = this.tasks.map((t) => (t.id === id ? next : t));
      this.emit('tasks://changed', null);
      return next;
    },
    tasks_delete: ({ id }) => {
      this.tasks = this.tasks.filter((t) => t.id !== id);
      this.emit('tasks://changed', null);
    },

    system_subscribe: ({ intervalMs }) => {
      this.systemSubscribers += 1;
      if (!this.systemTimer) {
        this.emit('system://sample', this.nextSample());
        this.systemTimer = setInterval(() => this.emit('system://sample', this.nextSample()), Math.max(250, intervalMs));
      }
    },
    system_unsubscribe: () => {
      this.systemSubscribers = Math.max(0, this.systemSubscribers - 1);
      if (!this.systemSubscribers && this.systemTimer) {
        clearInterval(this.systemTimer);
        this.systemTimer = null;
      }
    },
    system_processes: ({ limit }) => {
      this.processes = this.processes.map((p) => ({
        ...p, cpu: walk(p.cpu, 0, 40, 0.9), memoryBytes: walk(p.memoryBytes, 4e6, 4e9, 2.5e7),
      }));
      return this.processes.slice(0, limit);
    },
    system_kill: async ({ pid }) => {
      await sleep(250);
      const proc = this.processes.find((p) => p.pid === pid) ?? fail('not_found', `no process ${pid}`);
      if (proc.name === 'System' || proc.name === 'explorer.exe') {
        fail('access_denied', 'process is elevated');
      }
      this.processes = this.processes.filter((p) => p.pid !== pid);
    },
  };

  // ── Lab hooks ───────────────────────────────────────────────────────────

  /** Simulated system events, triggered from the lab. */
  readonly debug = {
    pushNotification: (): void => {
      const template = NOTIFICATION_POOL[this.pool % NOTIFICATION_POOL.length]!;
      this.pool += 1;
      const n: AppNotification = { ...template, id: `n${Date.now()}`, time: new Date().toISOString() };
      this.notifications = { ...this.notifications, items: [n, ...this.notifications.items] };
      this.emit('notifications://posted', n);
    },
    changeVolume: (delta: number): void => {
      this.audio = { ...this.audio, volume: Math.min(1, Math.max(0, this.audio.volume + delta)), muted: false };
      this.emit('audio://state', this.audio);
    },
    changeBrightness: (delta: number): void => {
      const current = this.display.brightness ?? 0.5;
      this.display = { ...this.display, brightness: Math.min(1, Math.max(0, current + delta)) };
      this.emit('display://state', this.display);
    },
    togglePlay: (): void => {
      void this.invoke('media_control', { sessionId: null, action: 'toggle' });
    },
    nextTrack: (): void => {
      void this.invoke('media_control', { sessionId: null, action: 'next' });
    },
    toggleCharging: (): void => {
      this.power = { ...this.power, charging: !this.power.charging };
      this.emit('power://state', this.power);
    },
    /** Adds an event that starts in five minutes, so a reminder fires. */
    addSoonEvent: (): void => {
      const start = new Date(Date.now() + 5 * 60000);
      this.extraEvents = [{
        id: `soon${Date.now()}`, calendarId: 'work', title: 'Abstimmung Tauri-Backend', location: 'Teams',
        allDay: false, start: start.toISOString(), end: new Date(start.getTime() + 30 * 60000).toISOString(),
      }];
      this.emit('calendar://changed', null);
    },
    setOccluded: (occluded: boolean): void => {
      this.occlusion = { ...this.occlusion, occluded };
      this.emit('island://occlusion', this.occlusion);
    },
    setWallpaper: (url: string): void => {
      this.wallpaperUrl = url;
      this.emit('system://wallpaper', this.wallpaper());
    },
    setWifiPermission: (granted: boolean): void => {
      this.network = { ...this.network, wifi: { ...this.network.wifi, permission: granted ? 'granted' : 'location_required' } };
      this.emit('network://state', this.network);
    },
  };

  // ── Internals ───────────────────────────────────────────────────────────

  private demoAccount(): CalendarAccount {
    return {
      id: 'demo', name: 'Nextcloud', url: 'https://cloud.example.org/remote.php/dav', user: 'demo',
      status: 'ok', lastSync: new Date(Date.now() - 2 * 60000).toISOString(),
      calendars: CALENDARS.map((c) => ({ ...c })),
    };
  }

  private setAccounts(accounts: CalendarAccount[]): void {
    this.accounts = accounts;
    save(KEYS.accounts, accounts);
    this.emit('calendar://accounts', accounts);
  }

  private enabledCalendars(): string[] {
    return this.accounts.flatMap((a) => a.calendars.filter((c) => c.enabled).map((c) => c.id));
  }

  private setDevice(id: string, patch: Partial<BluetoothState['devices'][number]>): void {
    this.bluetooth = {
      ...this.bluetooth,
      devices: this.bluetooth.devices.map((d) => (d.id === id ? { ...d, ...patch } : d)),
    };
    this.emit('bluetooth://state', this.bluetooth);
  }

  private appearance(): Appearance {
    const media = (q: string) => typeof matchMedia !== 'undefined' && matchMedia(q).matches;
    return {
      dark: !media('(prefers-color-scheme: light)'),
      accent: '#0078d4',
      reducedMotion: media('(prefers-reduced-motion: reduce)'),
      transparency: true,
    };
  }

  private wallpaper(): WallpaperInfo {
    const width = typeof window === 'undefined' ? 1920 : window.innerWidth;
    const height = typeof window === 'undefined' ? 1080 : window.innerHeight;
    return { url: this.wallpaperUrl, color: '#1d1730', fit: 'fill', monitor: { width, height }, window: { x: 0, y: 0 } };
  }

  private currentPosition(): number {
    const elapsed = this.playing ? (Date.now() - this.positionAt) / 1000 : 0;
    return Math.min(TRACKS[this.track]!.duration, this.position + elapsed);
  }

  private mediaState(): MediaState {
    const t = TRACKS[this.track]!;
    return {
      currentId: 'spotify',
      sessions: [{
        id: 'spotify', appName: 'Spotify', title: t.title, artist: t.artist, album: t.album, artUrl: t.art,
        playing: this.playing, position: this.currentPosition(), positionAt: new Date().toISOString(),
        duration: t.duration, canPrevious: true, canNext: true, canSeek: true,
      }],
    };
  }

  private skip(dir: number): void {
    this.track = (this.track + dir + TRACKS.length) % TRACKS.length;
    this.position = 0;
    this.positionAt = Date.now();
  }

  private tickMedia(): void {
    if (this.playing && this.currentPosition() >= TRACKS[this.track]!.duration) {
      this.skip(1);
      this.emit('media://state', this.mediaState());
    }
  }

  private nextSample(): SystemSample {
    const s = this.sample;
    s.cpu = walk(s.cpu, 3, 92, 9);
    s.gpu = walk(s.gpu, 1, 85, 6);
    s.down = walk(s.down, 10_000, 9_000_000, 700_000);
    s.up = walk(s.up, 8_000, 900_000, 60_000);
    s.mem = walk(s.mem, 8 * 1024 ** 3, 30 * 1024 ** 3, 0.15 * 1024 ** 3);
    return {
      time: new Date().toISOString(),
      cpu: { usage: s.cpu, name: 'AMD Ryzen 7 7800X3D', ghz: 4.7 },
      memory: { usedBytes: s.mem, totalBytes: 32 * 1024 ** 3 },
      gpu: { usage: s.gpu, name: 'NVIDIA GeForce RTX 4070', vramUsedBytes: 2.1 * 1024 ** 3, vramTotalBytes: 12 * 1024 ** 3 },
      network: { downBytesPerSec: s.down, upBytesPerSec: s.up },
    };
  }

  /**
   * Simulates the Windows-side edge detection: pushing the cursor against
   * the top edge near the centre activates, leaving the island area ends it.
   */
  private trackEdge(e: MouseEvent): void {
    const nearTop = e.clientY <= 2 && Math.abs(e.clientX - window.innerWidth / 2) <= 160;
    if (!this.edge && nearTop) {
      this.edge = true;
      this.emit('island://edge', { active: true });
      return;
    }
    if (this.edge && !nearTop) {
      const margin = 32;
      const inside = this.hitRects.some((r) =>
        e.clientX >= r.x - margin && e.clientX <= r.x + r.width + margin && e.clientY <= r.y + r.height + margin);
      if (!inside && e.clientY > 40) {
        this.edge = false;
        this.emit('island://edge', { active: false });
      }
    }
  }
}
