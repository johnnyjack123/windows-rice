/**
 * Frontend ⇄ backend contract. SINGLE SOURCE OF TRUTH for every command and
 * event between the Svelte frontend and the Rust (Tauri) backend.
 *
 * Conventions (mirrored in docs/BACKEND.md):
 * - Field names are camelCase on the wire (`#[serde(rename_all = "camelCase")]`).
 * - Commands with input take exactly one parameter named `args`:
 *   JS `invoke('config_set', { args: { patch } })`, Rust `fn config_set(args: ConfigSetArgs)`.
 * - Commands reject with a {@link BackendError} payload.
 * - Times are ISO 8601 strings with offset; durations and positions are in
 *   seconds unless the field name says otherwise (`…Ms`, `…Bytes`).
 * - Fractions (volume, brightness, battery, signal) are 0…1.
 * - Lengths are CSS pixels (device-independent); the backend applies DPI.
 */

/** ISO 8601 date-time with offset, e.g. "2026-10-13T14:32:00+02:00". */
export type Iso = string;

/** Rectangle in CSS pixels. */
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** Error payload of a rejected command. */
export interface BackendError {
  code: ErrorCode;
  message: string;
}

/** Machine-readable error categories the UI knows how to present. */
export type ErrorCode =
  | 'not_supported'
  | 'permission_required'
  | 'not_found'
  | 'invalid_args'
  | 'auth_failed'
  | 'network'
  | 'timeout'
  | 'access_denied'
  | 'internal';

// ── App & settings ───────────────────────────────────────────────────────

/** Static facts about the running app. */
export interface AppInfo {
  version: string;
  /** Where the theme folder lives (shown in settings, opened via `shell_open`). */
  themesDir: string;
  configDir: string;
}

/** Starting with Windows. */
export interface AutostartState {
  enabled: boolean;
  /** Only the installed app can start with Windows; development builds report `false`. */
  supported: boolean;
}

/** Colour scheme preference. */
export type SchemePreference = 'auto' | 'dark' | 'light';

/** Concrete colour scheme. */
export type Scheme = 'dark' | 'light';

/** Per-scheme colour overrides of the theme editor (hex `#rrggbb`). */
export interface ThemeColorOverrides {
  background?: string;
  foreground?: string;
  accent?: string;
}

/** Everything the theme editor can change. Absent fields = theme default. */
export interface ThemeOverrides {
  colors?: Partial<Record<Scheme, ThemeColorOverrides>>;
  fontUi?: string;
  fontDisplay?: string;
  textScale?: number;
  islandHeight?: number;
  islandRadius?: number;
  panelRadius?: number;
  tileRadius?: number;
  density?: number;
  speed?: number;
  bounce?: number;
  easing?: number;
  blur?: number;
  liquid?: boolean;
  /** Liquid glass only: strength of the lensing at the rim (1 = theme). */
  refraction?: number;
  /** Liquid glass only: colour fringing of the refraction, 0…1. */
  dispersion?: number;
  /** Liquid glass only: how frosted the body is, 0…1. */
  frost?: number;
  iconStyle?: 'Rounded' | 'Sharp' | 'Outlined';
  iconWeight?: number;
}

/** Stored alarm (owned by the frontend, persisted through settings). */
export interface Alarm {
  id: string;
  /** Local time "HH:MM". */
  time: string;
  label: string;
  /** Weekdays 0 = Monday … 6 = Sunday; empty = once. */
  days: number[];
  enabled: boolean;
}

/** Pomodoro rhythm and what a break looks like. */
export interface PomodoroSettings {
  focusMinutes: number;
  shortBreakMinutes: number;
  longBreakMinutes: number;
  /** Focus sessions per cycle; the cycle ends with a long break. */
  sessionsUntilLongBreak: number;
  /** A break starts on its own when a focus session ends. */
  autoStartBreaks: boolean;
  /** The next focus session starts on its own when a break ends. */
  autoStartFocus: boolean;
  /** "notify": the island announces the break; "overlay": a full-screen break window covers every monitor. */
  breakMode: 'notify' | 'overlay';
}

/** All persisted settings. The backend stores them as JSON or TOML. */
export interface Settings {
  version: 1;
  locale: 'auto' | 'de' | 'en';
  theme: {
    id: string;
    scheme: SchemePreference;
    /** Keyed by theme id. */
    overrides: Record<string, ThemeOverrides>;
  };
  island: {
    /** "primary" or a 0-based monitor index. */
    monitor: 'primary' | number;
    offsetTop: number;
    autoHide: boolean;
    peek: boolean;
    hideInFullscreen: boolean;
    hoverPreview: boolean;
    scrollVolume: boolean;
    /** Accelerator string, e.g. "Super+Alt+Space". */
    hotkey: string;
  };
  panel: {
    sections: string[];
    hiddenSections: string[];
    tilesLarge: string[];
    tilesSmall: string[];
    lastSection: string;
  };
  notifications: {
    dnd: boolean;
    /** Suppress Windows' own toast banners while rice runs (the island shows them instead). */
    hideWindowsBanners: boolean;
  };
  calendar: {
    /** Minutes before an event when the island shows it. */
    reminderLead: number;
  };
  timers: {
    sound: 'chime' | 'pulse' | 'digital';
    soundVolume: number;
    snoozeMinutes: number;
    presets: number[];
    alarms: Alarm[];
    pomodoro: PomodoroSettings;
  };
}

/**
 * Recursive patch. Objects merge key by key, arrays and primitives replace,
 * `null` deletes the key (e.g. resetting one theme override).
 */
export type DeepPatch<T> = {
  [K in keyof T]?: T[K] extends readonly unknown[]
    ? T[K] | null
    : T[K] extends object
      ? DeepPatch<T[K]> | null
      : T[K] | null;
};

/**
 * What the backend persists: an opaque JSON object that may be partial or
 * empty on first start. The frontend merges it over its own defaults, so
 * defaults and migrations live in one place (web/src/backend/settings.ts).
 */
export type StoredSettings = DeepPatch<Settings>;

/** Patch accepted by `config_set`. */
export type SettingsPatch = DeepPatch<Settings>;

// ── System appearance & wallpaper ────────────────────────────────────────

/** Windows appearance settings. */
export interface Appearance {
  dark: boolean;
  /** Windows accent colour, "#rrggbb". */
  accent: string;
  reducedMotion: boolean;
  transparency: boolean;
}

/** Wallpaper of the monitor the island lives on. */
export interface WallpaperInfo {
  /** Loadable URL (Tauri asset protocol), null for a solid colour. */
  url: string | null;
  /** Solid background colour when there is no image. */
  color: string;
  fit: 'fill' | 'fit' | 'stretch' | 'tile' | 'center' | 'span';
  /** Monitor size in CSS px. */
  monitor: { width: number; height: number };
  /** Top-left of the island window relative to the monitor, CSS px. */
  window: { x: number; y: number };
}

/** A connected monitor. */
export interface MonitorInfo {
  /** 0-based index as used by `island.monitor`. */
  index: number;
  name: string;
  primary: boolean;
  /** Size in physical pixels. */
  width: number;
  height: number;
}

// ── Island window ────────────────────────────────────────────────────────

/** Occlusion state of the island zone. */
export interface Occlusion {
  /** A visible top-level window intersects the island zone. */
  occluded: boolean;
  /** The foreground app is full screen (game, video, presentation). */
  fullscreen: boolean;
}

// ── Media ────────────────────────────────────────────────────────────────

/** One media session (GSMTC). */
export interface MediaSession {
  id: string;
  appName: string;
  title: string;
  artist: string;
  album: string;
  /** Cover as data URL or asset URL; null if none. */
  artUrl: string | null;
  playing: boolean;
  /** Position in seconds at {@link MediaSession.positionAt}. */
  position: number;
  /** When the position was sampled; the UI extrapolates while playing. */
  positionAt: Iso;
  duration: number;
  canPrevious: boolean;
  canNext: boolean;
  canSeek: boolean;
}

/** All media sessions. */
export interface MediaState {
  sessions: MediaSession[];
  /** Session Windows considers current. */
  currentId: string | null;
}

// ── Audio, display, power ────────────────────────────────────────────────

/** Output or input device. */
export interface AudioDevice {
  id: string;
  name: string;
  kind: 'speakers' | 'headphones' | 'hdmi' | 'bluetooth' | 'usb' | 'other';
}

/** Audio endpoint state. */
export interface AudioState {
  volume: number;
  muted: boolean;
  device: AudioDevice | null;
  devices: AudioDevice[];
  /** Default capture device muted (microphone privacy tile). */
  micMuted: boolean;
  micAvailable: boolean;
}

/** Display state; null fields are unsupported on this machine. */
export interface DisplayState {
  brightness: number | null;
  nightLight: boolean | null;
}

/** Power state. */
export interface PowerState {
  /** Null on machines without a battery. */
  battery: number | null;
  charging: boolean;
  energySaver: boolean;
  energySaverSupported: boolean;
  /** Windows Update waits for a restart to finish installing (the power menu offers "Update and …"). */
  updatePending: boolean;
}

/** Session actions of the power menu; `update_*` install pending updates first. */
export type PowerAction = 'lock' | 'sleep' | 'signout' | 'restart' | 'shutdown' | 'update_restart' | 'update_shutdown';

// ── Network ──────────────────────────────────────────────────────────────

/** A Wi-Fi network. */
export interface WifiNetwork {
  ssid: string;
  /** 0…4 bars. */
  signal: number;
  secure: boolean;
  /** Windows has a stored profile for it. */
  known: boolean;
  /** e.g. "5 GHz · WPA3"; empty if unknown. */
  detail: string;
}

/** Network state. */
export interface NetworkState {
  wifi: {
    available: boolean;
    enabled: boolean;
    connected: WifiNetwork | null;
    /** Wi-Fi scans need location access since Windows 11 24H2. */
    permission: 'granted' | 'location_required';
  };
  airplane: boolean;
  hotspot: { supported: boolean; enabled: boolean };
}

/** Result of a connection attempt. */
export interface ConnectResult {
  ok: boolean;
  error?: 'wrong_key' | 'timeout' | 'failed';
}

// ── Bluetooth ────────────────────────────────────────────────────────────

/** Device categories with icons and labels in the UI. */
export type BluetoothKind = 'headphones' | 'earbuds' | 'mouse' | 'keyboard' | 'speaker' | 'phone' | 'gamepad' | 'other';

/** A paired Bluetooth device. */
export interface BluetoothDevice {
  id: string;
  name: string;
  kind: BluetoothKind;
  connected: boolean;
  /** 0…1 or null if unknown. */
  battery: number | null;
  /** Whether connect/disconnect is possible from the app (else: open settings). */
  canConnect: boolean;
}

/** Bluetooth state. */
export interface BluetoothState {
  available: boolean;
  enabled: boolean;
  devices: BluetoothDevice[];
}

// ── Notifications ────────────────────────────────────────────────────────

/** A notification mirrored from Windows. */
export interface AppNotification {
  id: string;
  appId: string;
  appName: string;
  /** App icon as data URL or asset URL; null → letter avatar. */
  iconUrl: string | null;
  title: string;
  body: string;
  time: Iso;
}

/** Notification access state. */
export type NotificationPermission = 'granted' | 'denied' | 'unknown' | 'unsupported';

/** Notification centre state. */
export interface NotificationsState {
  permission: NotificationPermission;
  items: AppNotification[];
}

// ── Calendar & tasks (CalDAV, Nextcloud) ─────────────────────────────────

/** A calendar collection (events, tasks or both). */
export interface Calendar {
  id: string;
  accountId: string;
  name: string;
  /** "#rrggbb" from the server (Apple/Nextcloud calendar-color). */
  color: string;
  supportsEvents: boolean;
  supportsTasks: boolean;
  enabled: boolean;
}

/** A CalDAV account. Passwords never leave the backend. */
export interface CalendarAccount {
  id: string;
  name: string;
  url: string;
  user: string;
  status: 'ok' | 'syncing' | 'auth_failed' | 'offline';
  lastSync: Iso | null;
  calendars: Calendar[];
}

/** One occurrence of an event (recurrences are expanded by the backend). */
export interface CalendarEvent {
  /** Unique per occurrence, e.g. `${uid}@${recurrenceId}`. */
  id: string;
  calendarId: string;
  title: string;
  start: Iso;
  end: Iso;
  allDay: boolean;
  location: string;
}

/** A VTODO. */
export interface Task {
  id: string;
  calendarId: string;
  title: string;
  /** Date "YYYY-MM-DD" or date-time; null if none. */
  due: string | null;
  done: boolean;
  completedAt: Iso | null;
}

// ── System monitor ───────────────────────────────────────────────────────

/** One sample of system load. */
export interface SystemSample {
  time: Iso;
  cpu: { usage: number; name: string; ghz: number | null };
  memory: { usedBytes: number; totalBytes: number };
  gpu: { usage: number; name: string; vramUsedBytes: number; vramTotalBytes: number } | null;
  network: { downBytesPerSec: number; upBytesPerSec: number };
}

/** A running process (top entries only). */
export interface ProcessInfo {
  pid: number;
  name: string;
  /** Percent of total CPU, 0…100. */
  cpu: number;
  memoryBytes: number;
  /** Icon as data URL; null → letter avatar. */
  iconUrl: string | null;
}

// ── Command map ──────────────────────────────────────────────────────────

/** Every command: `args` (void = no input) and `result`. */
export interface Commands {
  app_info: { args: void; result: AppInfo };
  app_autostart_get: { args: void; result: AutostartState };
  app_autostart_set: { args: { enabled: boolean }; result: AutostartState };
  /** Quits rice: windows close, Windows' notification banners come back. */
  app_quit: { args: void; result: void };

  config_get: { args: void; result: StoredSettings };
  config_set: { args: { patch: SettingsPatch }; result: StoredSettings };

  theme_list_user: { args: void; result: UserTheme[] };
  theme_save_user: { args: { id: string; manifest: UserThemeManifest; css: string }; result: UserTheme };
  theme_delete_user: { args: { id: string }; result: void };

  system_appearance: { args: void; result: Appearance };
  system_wallpaper: { args: void; result: WallpaperInfo };
  system_fonts: { args: void; result: string[] };
  system_monitors: { args: void; result: MonitorInfo[] };

  island_configure: { args: { width: number; height: number }; result: void };
  island_set_hit_regions: { args: { rects: Rect[] }; result: void };
  island_set_expanded: { args: { expanded: boolean }; result: void };
  island_occlusion: { args: void; result: Occlusion };

  window_open_settings: { args: { page?: string }; result: void };

  /** Shows (or updates) the full-screen break window on every monitor. */
  break_show: { args: BreakOverlay; result: void };
  /** Closes the break windows (after announcing `null` so they can fade out). */
  break_hide: { args: void; result: void };
  /** Current break, read by a break window when it loads. */
  break_state: { args: void; result: BreakOverlay | null };
  /** The user leaves the break from a break window. */
  break_finish: { args: void; result: void };
  shell_open: { args: { target: string }; result: void };

  media_get: { args: void; result: MediaState };
  media_control: { args: { sessionId: string | null; action: 'play' | 'pause' | 'toggle' | 'next' | 'previous' }; result: void };
  media_seek: { args: { sessionId: string | null; position: number }; result: void };

  audio_get: { args: void; result: AudioState };
  audio_set_volume: { args: { volume: number }; result: void };
  audio_set_muted: { args: { muted: boolean }; result: void };
  audio_set_device: { args: { id: string }; result: void };
  audio_set_mic_muted: { args: { muted: boolean }; result: void };

  display_get: { args: void; result: DisplayState };
  display_set_brightness: { args: { value: number }; result: void };
  display_set_night_light: { args: { enabled: boolean }; result: void };

  power_get: { args: void; result: PowerState };
  power_set_energy_saver: { args: { enabled: boolean }; result: void };
  power_action: { args: { action: PowerAction }; result: void };

  network_get: { args: void; result: NetworkState };
  network_scan: { args: void; result: WifiNetwork[] };
  network_set_wifi: { args: { enabled: boolean }; result: void };
  network_connect: { args: { ssid: string; key: string | null }; result: ConnectResult };
  network_disconnect: { args: void; result: void };
  network_set_airplane: { args: { enabled: boolean }; result: void };
  network_set_hotspot: { args: { enabled: boolean }; result: void };

  bluetooth_get: { args: void; result: BluetoothState };
  bluetooth_set_enabled: { args: { enabled: boolean }; result: void };
  bluetooth_connect: { args: { id: string }; result: ConnectResult };
  bluetooth_disconnect: { args: { id: string }; result: void };

  notifications_get: { args: void; result: NotificationsState };
  notifications_request_access: { args: void; result: NotificationPermission };
  notifications_dismiss: { args: { id: string }; result: void };
  notifications_clear: { args: void; result: void };

  calendar_accounts: { args: void; result: CalendarAccount[] };
  calendar_add_account: { args: { name: string; url: string; user: string; password: string }; result: CalendarAccount };
  calendar_remove_account: { args: { id: string }; result: void };
  calendar_set_enabled: { args: { calendarId: string; enabled: boolean }; result: void };
  calendar_sync: { args: void; result: void };
  calendar_events: { args: { from: Iso; to: Iso }; result: CalendarEvent[] };

  tasks_list: { args: void; result: Task[] };
  tasks_create: { args: { calendarId: string; title: string; due: string | null }; result: Task };
  tasks_update: { args: { id: string; patch: { title?: string; due?: string | null; done?: boolean } }; result: Task };
  tasks_delete: { args: { id: string }; result: void };

  system_subscribe: { args: { intervalMs: number }; result: void };
  system_unsubscribe: { args: void; result: void };
  system_processes: { args: { limit: number }; result: ProcessInfo[] };
  system_kill: { args: { pid: number }; result: void };
}

/** Every event and its payload. */
export interface Events {
  'config://changed': StoredSettings;
  'theme://user-changed': UserTheme[];
  'system://appearance': Appearance;
  'system://wallpaper': WallpaperInfo;

  'island://occlusion': Occlusion;
  /** Cursor pushes against the top edge in the island zone (true) or left it (false). */
  'island://edge': { active: boolean };
  /** The expanded island lost focus (click elsewhere). */
  'island://focus-lost': null;
  /** Global hotkey pressed. */
  'island://toggle': null;

  /** Break shown or updated; null = closing (fade out). */
  'break://state': BreakOverlay | null;
  /** The user left the break in a break window. */
  'break://finished': null;

  'media://state': MediaState;
  'audio://state': AudioState;
  'display://state': DisplayState;
  'power://state': PowerState;
  'network://state': NetworkState;
  'network://scan': WifiNetwork[];
  'bluetooth://state': BluetoothState;

  'notifications://posted': AppNotification;
  'notifications://removed': { id: string };
  'notifications://state': NotificationsState;

  'calendar://accounts': CalendarAccount[];
  /** Events or tasks changed after a sync; the UI refetches its range. */
  'calendar://changed': null;
  'tasks://changed': null;

  'system://sample': SystemSample;
}

/** What the full-screen break window shows. */
export interface BreakOverlay {
  /** Epoch ms when the break ends; null while it is paused. */
  endsAt: number | null;
  /** Remaining ms (used while paused). */
  remaining: number;
  /** Length of the break in ms. */
  duration: number;
  kind: 'short' | 'long';
  /** Focus sessions completed in this cycle. */
  round: number;
  /** Focus sessions per cycle. */
  rounds: number;
}

/** Manifest of a user theme (theme.toml in the theme folder). */
export interface UserThemeManifest {
  name: string;
  author: string;
  version: string;
  schemes: Scheme[];
  /** Theme this one was derived from, if any. */
  basedOn: string | null;
}

/** A user theme found in the themes folder. */
export interface UserTheme {
  id: string;
  manifest: UserThemeManifest;
  /** Full CSS text of theme.css. */
  css: string;
}

/** Command names. */
export type CommandName = keyof Commands;

/** Event names. */
export type EventName = keyof Events;

/** Argument type of a command. */
export type ArgsOf<C extends CommandName> = Commands[C]['args'];

/** Result type of a command. */
export type ResultOf<C extends CommandName> = Commands[C]['result'];
