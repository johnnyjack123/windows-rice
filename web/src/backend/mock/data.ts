/**
 * Seed data of the simulated backend. Content (titles, names) is data, not
 * UI text, and therefore not translated.
 */
import type { AgentRequest, AgentStatus, BluetoothDevice, Calendar, CalendarEvent, ProcessInfo, Task, WifiNetwork } from '../contract';
import { appIcon, COVERS } from './art';

/** A simulated track. */
export interface MockTrack {
  id: string;
  title: string;
  artist: string;
  album: string;
  duration: number;
  art: string;
}

/** Tracks of the simulated media session. */
export const TRACKS: MockTrack[] = [
  { id: 'nachtfahrt', title: 'Nachtfahrt', artist: 'Kollektiv Ost', album: 'Autobahn bei Nacht', duration: 221, art: COVERS.nachtfahrt },
  { id: 'glasshouse', title: 'Glasshouse', artist: 'Mara Lind', album: 'Greenhouse Effect', duration: 252, art: COVERS.glasshouse },
  { id: 'lowtide', title: 'Low Tide', artist: 'Harbour & Co.', album: 'Coastlines', duration: 185, art: COVERS.lowtide },
  { id: 'leipzig', title: 'Sommer in Leipzig', artist: 'Die Fernen', album: 'Ostwärts', duration: 238, art: COVERS.leipzig },
];

/** Notification template without id and time. */
export interface MockNotification {
  appId: string;
  appName: string;
  iconUrl: string | null;
  title: string;
  body: string;
}

const SIGNAL = appIcon('#3a76f0', 'chat');

/** Notifications present at start, with their age in minutes. */
export const INITIAL_NOTIFICATIONS: Array<MockNotification & { minutesAgo: number }> = [
  { appId: 'signal', appName: 'Signal', iconUrl: SIGNAL, title: 'Lea Hoffmann', body: 'Bist du heute Abend dabei? Wir wollten so gegen 19 Uhr los.', minutesAgo: 2 },
  { appId: 'discord', appName: 'Discord', iconUrl: appIcon('#5865f2', 'forum'), title: '#rice-dev · Tom', body: 'Hast du die neue Island schon getestet? Das Aufklappen ist absurd smooth.', minutesAgo: 14 },
  { appId: 'thunderbird', appName: 'Thunderbird', iconUrl: appIcon('#1373d9', 'mail'), title: 'Paketankündigung', body: 'Dein Paket kommt morgen zwischen 10:00 und 13:00 Uhr.', minutesAgo: 38 },
];

/** Notifications the lab can deliver, in order. */
export const NOTIFICATION_POOL: MockNotification[] = [
  { appId: 'calendar', appName: 'Kalender', iconUrl: appIcon('#3fb68b', 'calendar'), title: 'Design-Review in 10 Minuten', body: 'Island-Konzepte · Jitsi' },
  { appId: 'steam', appName: 'Steam', iconUrl: appIcon('#1b2838', 'game'), title: 'Download abgeschlossen', body: 'Hades II ist bereit zum Spielen.' },
  { appId: 'signal', appName: 'Signal', iconUrl: SIGNAL, title: 'Mama', body: 'Denkst du an Sonntag? Oma freut sich schon riesig auf euch!' },
  { appId: 'windows-update', appName: 'Windows Update', iconUrl: appIcon('#0078d4', 'update'), title: 'Neustart erforderlich', body: 'Updates werden beim nächsten Neustart installiert.' },
  { appId: 'github', appName: 'GitHub', iconUrl: null, title: 'rice/shell · CI', body: 'Build #128 erfolgreich · alle Checks bestanden' },
];

/** Permission requests the lab can deliver, in order (without id and time). */
export const AGENT_REQUESTS: Array<Omit<AgentRequest, 'id' | 'time'>> = [
  { agent: 'claude', project: 'rice', tool: 'Bash', kind: 'command', detail: 'pnpm verify && cargo clippy --workspace -- -D warnings' },
  { agent: 'claude', project: 'rice', tool: 'Edit', kind: 'edit', detail: 'C:\\Users\\demo\\Projekte\\rice\\web\\src\\island\\island.css' },
  { agent: 'claude', project: 'nextcloud-sync', tool: 'WebFetch', kind: 'fetch', detail: 'https://docs.nextcloud.com/server/latest/developer_manual/' },
  { agent: 'claude', project: 'rice', tool: 'mcp__github__create_pull_request', kind: 'mcp', detail: 'github · create_pull_request' },
];

/** Status reports the lab can deliver, in order (without time). */
export const AGENT_STATUS: Array<Omit<AgentStatus, 'time'>> = [
  { agent: 'claude', project: 'rice', state: 'done', message: 'Der Equalizer hat jetzt ein eigenes Tempo; alle Checks sind grün.' },
  { agent: 'claude', project: 'nextcloud-sync', state: 'waiting', message: null },
];

/** Wi-Fi networks in range. */
export const NETWORKS: WifiNetwork[] = [
  { ssid: 'FRITZ!Box 7590 KL', signal: 4, secure: true, known: true, detail: '5 GHz · WPA3' },
  { ssid: 'Vodafone-3A2C', signal: 3, secure: true, known: false, detail: '' },
  { ssid: 'o2-WLAN61', signal: 2, secure: true, known: false, detail: '' },
  { ssid: 'Telekom_FON', signal: 2, secure: false, known: false, detail: '' },
  { ssid: 'DIRECT-7F-HP OfficeJet', signal: 1, secure: true, known: false, detail: '' },
  { ssid: 'Nachbarn 5G', signal: 1, secure: true, known: false, detail: '' },
];

/** Paired Bluetooth devices. */
export const DEVICES: BluetoothDevice[] = [
  { id: 'xm5', name: 'WH-1000XM5', kind: 'headphones', connected: true, battery: 0.78, canConnect: true },
  { id: 'mx', name: 'MX Master 3S', kind: 'mouse', connected: true, battery: 0.55, canConnect: true },
  { id: 'buds', name: 'Pixel Buds Pro', kind: 'earbuds', connected: false, battery: null, canConnect: true },
  { id: 'k3', name: 'Keychron K3', kind: 'keyboard', connected: false, battery: null, canConnect: true },
  { id: 'jbl', name: 'JBL Flip 6', kind: 'speaker', connected: false, battery: null, canConnect: false },
];

/** Calendars of the demo Nextcloud account. */
export const CALENDARS: Calendar[] = [
  { id: 'work', accountId: 'demo', name: 'Arbeit', color: '#5b8def', supportsEvents: true, supportsTasks: false, enabled: true },
  { id: 'private', accountId: 'demo', name: 'Privat', color: '#e5774f', supportsEvents: true, supportsTasks: true, enabled: true },
  { id: 'rice', accountId: 'demo', name: 'Rice', color: '#3fb68b', supportsEvents: true, supportsTasks: true, enabled: true },
  { id: 'shopping', accountId: 'demo', name: 'Einkauf', color: '#c9a227', supportsEvents: false, supportsTasks: true, enabled: true },
];

/**
 * Midnight of a day relative to another.
 *
 * @param base - Reference date.
 * @param offset - Days to add.
 * @returns Local midnight of that day.
 */
export function dayStart(base: Date, offset = 0): Date {
  const d = new Date(base);
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() + offset);
  return d;
}

/**
 * Formats a local date as "YYYY-MM-DD".
 *
 * @param d - Date.
 * @returns Date string.
 */
export function isoDay(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

/**
 * Builds the events of one day. Today's events sit around the current time
 * so the agenda always looks alive ("in 30 Min").
 *
 * @param day - Any time on the requested day.
 * @returns Events of that day.
 */
export function eventsOfDay(day: Date): CalendarEvent[] {
  const now = new Date();
  const today = dayStart(now);
  const start = dayStart(day);
  const offset = Math.round((start.getTime() - today.getTime()) / 86400000);
  const make = (id: string, calendarId: string, title: string, location: string, startMinute: number, length: number): CalendarEvent => {
    const s = new Date(start.getTime() + startMinute * 60000);
    return { id: `${id}@${isoDay(start)}`, calendarId, title, location, allDay: false, start: s.toISOString(), end: new Date(s.getTime() + length * 60000).toISOString() };
  };
  if (offset === 0) {
    const base = Math.floor((now.getHours() * 60 + now.getMinutes()) / 15) * 15;
    return [
      make('standup', 'work', 'Daily Stand-up', 'Teams', base - 285, 15),
      make('review', 'rice', 'Design-Review: Island-Konzepte', 'Jitsi', base - 120, 60),
      make('pairing', 'rice', 'Pairing: Tauri-Fenster-Spike', 'Discord', base + 30, 45),
      make('dentist', 'private', 'Zahnarzt Dr. Weber', 'Karl-Liebknecht-Str. 12', base + 150, 30),
      make('pizza', 'private', 'Pizza mit Lea & Tom', 'Pizzeria Fiore', base + 300, 120),
    ];
  }
  const plan: Record<number, CalendarEvent[]> = {
    [-1]: [make('standup', 'work', 'Daily Stand-up', 'Teams', 570, 15)],
    1: [make('planning', 'work', 'Sprint Planning', 'Raum 3.14', 600, 90), make('climbing', 'private', 'Klettern', 'Boulderhalle', 1110, 120)],
    2: [make('caldav', 'rice', 'CalDAV-Spike', '', 840, 120)],
    3: [make('retro', 'work', 'Retro', 'Teams', 900, 60), make('cinema', 'private', 'Kino: Dune', 'Passage Kinos', 1200, 170)],
    5: [{ ...make('birthday', 'private', 'Geburtstag Mama', '', 0, 1440), allDay: true }],
  };
  return plan[offset] ?? [];
}

/**
 * Initial tasks, with due dates relative to today.
 *
 * @returns Fresh task list.
 */
export function initialTasks(): Task[] {
  const today = new Date();
  const day = (offset: number) => isoDay(dayStart(today, offset));
  const task = (id: string, calendarId: string, title: string, due: string | null, done = false): Task => ({
    id, calendarId, title, due, done, completedAt: done ? new Date().toISOString() : null,
  });
  return [
    task('t1', 'rice', 'Island-Konzept auswählen', day(0)),
    task('t2', 'rice', 'Nextcloud-Zugang einrichten', day(1)),
    task('t3', 'private', 'Steuererklärung abgeben', day(-1)),
    task('t4', 'shopping', 'Hafermilch, Kaffee, Zitronen', day(0)),
    task('t5', 'private', 'Geburtstagsgeschenk für Mama', day(4)),
    task('t6', 'rice', 'Tauri-2-Doku zu Fenstern lesen', null),
    task('t7', 'rice', 'Wallpaper aussuchen', null, true),
    task('t8', 'private', 'Fahrrad zur Inspektion bringen', null, true),
  ];
}

/** Simulated processes (memory in bytes). */
export const PROCESSES: ProcessInfo[] = [
  { pid: 4120, name: 'firefox.exe', cpu: 6.2, memoryBytes: 1_930_000_000, iconUrl: null },
  { pid: 8812, name: 'Code.exe', cpu: 3.1, memoryBytes: 1_270_000_000, iconUrl: null },
  { pid: 9932, name: 'Discord.exe', cpu: 1.1, memoryBytes: 640_000_000, iconUrl: null },
  { pid: 7720, name: 'Spotify.exe', cpu: 1.4, memoryBytes: 440_000_000, iconUrl: null },
  { pid: 6604, name: 'steamwebhelper.exe', cpu: 0.9, memoryBytes: 545_000_000, iconUrl: null },
  { pid: 1288, name: 'explorer.exe', cpu: 0.4, memoryBytes: 189_000_000, iconUrl: null },
  { pid: 3004, name: 'rice-shell.exe', cpu: 0.6, memoryBytes: 101_000_000, iconUrl: null },
  { pid: 4, name: 'System', cpu: 0.3, memoryBytes: 8_000_000, iconUrl: null },
];

/** Installed fonts reported by the simulated backend. */
export const SYSTEM_FONTS = [
  'Arial', 'Bahnschrift', 'Calibri', 'Cambria', 'Candara', 'Cascadia Code', 'Cascadia Mono', 'Consolas',
  'Constantia', 'Corbel', 'Georgia', 'Segoe UI', 'Segoe UI Variable Display', 'Segoe UI Variable Text',
  'Sitka Text', 'Trebuchet MS', 'Verdana',
];
