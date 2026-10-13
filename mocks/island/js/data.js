/* Mock data and formatting helpers. Shapes mirror the payloads the Rust
   services will emit later (see docs/PLAN.md, section "Services"). */

const DE = 'de-DE';

/**
 * Formatting helpers for German UI strings.
 */
const Fmt = {
  /**
   * @param {Date} d - Point in time.
   * @returns {string} "14:32".
   */
  time(d) {
    return d.toLocaleTimeString(DE, { hour: '2-digit', minute: '2-digit' });
  },

  /**
   * @param {Date} d - Day.
   * @returns {string} "Dienstag, 13. Oktober".
   */
  longDate(d) {
    return d.toLocaleDateString(DE, { weekday: 'long', day: 'numeric', month: 'long' });
  },

  /**
   * @param {Date} d - Day.
   * @returns {string} Two-letter weekday, e.g. "Di".
   */
  weekday(d) {
    return d.toLocaleDateString(DE, { weekday: 'short' }).replace('.', '');
  },

  /**
   * @param {number} seconds - Duration in seconds.
   * @returns {string} "3:41".
   */
  duration(seconds) {
    const s = Math.max(0, Math.round(seconds));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  },

  /**
   * @param {number} minutes - Minutes in the past.
   * @returns {string} Relative label, e.g. "vor 14 Min".
   */
  ago(minutes) {
    if (minutes < 1) {
      return 'jetzt';
    }
    if (minutes < 60) {
      return `vor ${Math.round(minutes)} Min`;
    }
    return `vor ${Math.round(minutes / 60)} Std`;
  },

  /**
   * @param {number} minutes - Minutes until an event.
   * @returns {string} Relative label, e.g. "in 25 Min".
   */
  until(minutes) {
    if (minutes <= 0) {
      return 'jetzt';
    }
    if (minutes < 60) {
      return `in ${Math.round(minutes)} Min`;
    }
    const h = Math.floor(minutes / 60);
    const m = Math.round(minutes % 60);
    return m ? `in ${h} Std ${m} Min` : `in ${h} Std`;
  },

  /**
   * @param {number} value - Number to format.
   * @param {number} [digits=1] - Fraction digits.
   * @returns {string} German decimal, e.g. "11,4".
   */
  num(value, digits = 1) {
    return value.toLocaleString(DE, { minimumFractionDigits: digits, maximumFractionDigits: digits });
  },

  /**
   * @param {number} mb - Size in megabytes.
   * @returns {string} "1,2 GB" or "420 MB".
   */
  mem(mb) {
    return mb >= 1000 ? `${this.num(mb / 1024)} GB` : `${Math.round(mb)} MB`;
  },

  /**
   * @param {number} kbps - Throughput in KB/s.
   * @returns {string} "2,4 MB/s" or "120 KB/s".
   */
  rate(kbps) {
    return kbps >= 1000 ? `${this.num(kbps / 1024)} MB/s` : `${Math.round(kbps)} KB/s`;
  },
};

/**
 * Returns a copy of a date moved by whole days, at local midnight.
 *
 * @param {Date} base - Reference date.
 * @param {number} offset - Days to add.
 * @returns {Date} Midnight of the resulting day.
 */
function dayOffset(base, offset) {
  const d = new Date(base);
  d.setHours(0, 0, 0, 0);
  d.setDate(d.getDate() + offset);
  return d;
}

/**
 * Returns `count` samples of a bounded random walk (fake metrics).
 *
 * @param {number} count - Number of samples.
 * @param {number} start - First value.
 * @param {number} min - Lower bound.
 * @param {number} max - Upper bound.
 * @param {number} step - Maximum change per sample.
 * @returns {number[]} Samples.
 */
function randomWalk(count, start, min, max, step) {
  const out = [start];
  for (let i = 1; i < count; i++) {
    out.push(Math.min(max, Math.max(min, out[i - 1] + (Math.random() - 0.5) * 2 * step)));
  }
  return out;
}

/**
 * Static mock content for every service the island talks to.
 */
const MockData = {
  tracks: [
    { id: 'nachtfahrt', title: 'Nachtfahrt', artist: 'Kollektiv Ost', duration: 221, color: '#f08a4b' },
    { id: 'glasshouse', title: 'Glasshouse', artist: 'Mara Lind', duration: 252, color: '#e46d96' },
    { id: 'lowtide', title: 'Low Tide', artist: 'Harbour & Co.', duration: 185, color: '#4aa3c4' },
    { id: 'leipzig', title: 'Sommer in Leipzig', artist: 'Die Fernen', duration: 238, color: '#d9b23c' },
  ],

  notifications: [
    {
      app: 'Signal', icon: 'chat', color: '#3a76f0', minutesAgo: 2,
      title: 'Lea Hoffmann', body: 'Bist du heute Abend dabei? Wir wollten so gegen 19 Uhr los.',
      actions: ['Antworten', 'Als gelesen markieren'],
    },
    {
      app: 'Discord', icon: 'forum', color: '#5865f2', minutesAgo: 14,
      title: '#rice-dev · Tom', body: 'Hast du die neue Island schon getestet? Das Aufklappen ist absurd smooth.',
      actions: ['Antworten'],
    },
    {
      app: 'Thunderbird', icon: 'mail', color: '#1373d9', minutesAgo: 38,
      title: 'Paketankündigung', body: 'Dein Paket kommt morgen zwischen 10:00 und 13:00 Uhr.',
      actions: [],
    },
  ],

  notificationPool: [
    {
      app: 'Kalender', icon: 'event', color: '#3fb68b',
      title: 'Design-Review in 10 Minuten', body: 'Island-Konzepte · Jitsi', actions: ['Beitreten'],
    },
    {
      app: 'Steam', icon: 'sports_esports', color: '#66c0f4',
      title: 'Download abgeschlossen', body: 'Hades II ist bereit zum Spielen.', actions: ['Spielen'],
    },
    {
      app: 'Signal', icon: 'chat', color: '#3a76f0',
      title: 'Mama', body: 'Denkst du an Sonntag? Oma freut sich schon riesig auf euch!', actions: ['Antworten'],
    },
    {
      app: 'Windows Update', icon: 'restart_alt', color: '#0078d4',
      title: 'Neustart erforderlich', body: 'Updates werden beim nächsten Neustart installiert.',
      actions: ['Jetzt neu starten', 'Später'],
    },
    {
      app: 'GitHub', icon: 'commit', color: '#8250df',
      title: 'rice/shell · CI', body: 'Build #128 erfolgreich · alle Checks bestanden', actions: [],
    },
  ],

  networks: [
    { ssid: 'FRITZ!Box 7590 KL', signal: 4, secure: true, detail: '5 GHz · WPA3' },
    { ssid: 'Vodafone-3A2C', signal: 3, secure: true },
    { ssid: 'o2-WLAN61', signal: 2, secure: true },
    { ssid: 'Telekom_FON', signal: 2, secure: false },
    { ssid: 'DIRECT-7F-HP OfficeJet', signal: 1, secure: true },
    { ssid: 'Nachbarn 5G', signal: 1, secure: true },
  ],

  devices: [
    { id: 'xm5', name: 'WH-1000XM5', kind: 'Kopfhörer', icon: 'headphones', connected: true, battery: 78 },
    { id: 'mx', name: 'MX Master 3S', kind: 'Maus', icon: 'mouse', connected: true, battery: 55 },
    { id: 'buds', name: 'Pixel Buds Pro', kind: 'Ohrhörer', icon: 'earbuds', connected: false, battery: 0 },
    { id: 'k3', name: 'Keychron K3', kind: 'Tastatur', icon: 'keyboard', connected: false, battery: 0 },
    { id: 'jbl', name: 'JBL Flip 6', kind: 'Lautsprecher', icon: 'speaker', connected: false, battery: 0 },
  ],

  calendars: {
    work: { name: 'Arbeit', color: '#5b8def' },
    private: { name: 'Privat', color: '#e5774f' },
    rice: { name: 'Rice', color: '#3fb68b' },
  },

  taskLists: {
    rice: { name: 'Rice', color: '#3fb68b' },
    private: { name: 'Privat', color: '#e5774f' },
    shopping: { name: 'Einkauf', color: '#c9a227' },
  },

  tasks: [
    { id: 1, title: 'Island-Konzept auswählen', list: 'rice', due: 'today', done: false },
    { id: 2, title: 'CalDAV-Zugang (Nextcloud) einrichten', list: 'rice', due: 'tomorrow', done: false },
    { id: 3, title: 'Steuererklärung abgeben', list: 'private', due: 'overdue', done: false },
    { id: 4, title: 'Hafermilch, Kaffee, Zitronen', list: 'shopping', due: 'today', done: false },
    { id: 5, title: 'Geburtstagsgeschenk für Mama', list: 'private', due: 'sat', done: false },
    { id: 6, title: 'Tauri-2-Doku zu Fenstern lesen', list: 'rice', due: null, done: false },
    { id: 7, title: 'Wallpaper aussuchen', list: 'rice', due: null, done: true },
    { id: 8, title: 'Fahrrad zur Inspektion bringen', list: 'private', due: null, done: true },
  ],

  dueLabels: {
    today: { text: 'Heute', cls: 'is-today' },
    tomorrow: { text: 'Morgen', cls: '' },
    overdue: { text: 'Gestern', cls: 'is-overdue' },
    sat: { text: 'Sa', cls: '' },
  },

  processes: [
    { name: 'firefox.exe', color: '#ff7139', cpu: 6.2, ram: 1840 },
    { name: 'Code.exe', color: '#2f80ed', cpu: 3.1, ram: 1210 },
    { name: 'Discord.exe', color: '#5865f2', cpu: 1.1, ram: 610 },
    { name: 'Spotify.exe', color: '#1db954', cpu: 1.4, ram: 420 },
    { name: 'steamwebhelper.exe', color: '#66c0f4', cpu: 0.9, ram: 520 },
    { name: 'explorer.exe', color: '#f2c94c', cpu: 0.4, ram: 180 },
    { name: 'rice-shell.exe', color: '#b48ead', cpu: 0.6, ram: 96 },
  ],

  /**
   * Builds calendar events for a day. Today's events are placed around the
   * current time so the agenda always looks alive ("in 25 Min").
   *
   * @param {Date} day - Any time on the requested day.
   * @returns {Array<{title: string, cal: string, start: Date, end: Date, where: string}>}
   *   Events sorted by start.
   */
  eventsFor(day) {
    const now = new Date();
    const today = dayOffset(now, 0);
    const offset = Math.round((dayOffset(day, 0) - today) / 86400000);
    const at = (minutesFromMidnight, length) => {
      const start = new Date(dayOffset(day, 0).getTime() + minutesFromMidnight * 60000);
      return { start, end: new Date(start.getTime() + length * 60000) };
    };
    if (offset === 0) {
      const base = Math.floor((now.getHours() * 60 + now.getMinutes()) / 15) * 15;
      const rel = (delta, length) => {
        const start = new Date(today.getTime() + (base + delta) * 60000);
        return { start, end: new Date(start.getTime() + length * 60000) };
      };
      return [
        { title: 'Daily Stand-up', cal: 'work', where: 'Teams', ...rel(-285, 15) },
        { title: 'Design-Review: Island-Konzepte', cal: 'rice', where: 'Jitsi', ...rel(-120, 60) },
        { title: 'Pairing: Tauri-Fenster-Spike', cal: 'rice', where: 'Discord', ...rel(30, 45) },
        { title: 'Zahnarzt Dr. Weber', cal: 'private', where: 'Karl-Liebknecht-Str. 12', ...rel(150, 30) },
        { title: 'Pizza mit Lea & Tom', cal: 'private', where: 'Pizzeria Fiore', ...rel(300, 120) },
      ];
    }
    const plan = {
      1: [
        { title: 'Sprint Planning', cal: 'work', where: 'Raum 3.14', ...at(600, 90) },
        { title: 'Klettern', cal: 'private', where: 'Boulderhalle', ...at(1110, 120) },
      ],
      2: [{ title: 'CalDAV-Spike', cal: 'rice', where: '', ...at(840, 120) }],
      3: [
        { title: 'Retro', cal: 'work', where: 'Teams', ...at(900, 60) },
        { title: 'Kino: Dune', cal: 'private', where: 'Passage Kinos', ...at(1200, 170) },
      ],
      5: [{ title: 'Geburtstag Mama', cal: 'private', where: '', ...at(720, 240) }],
      [-1]: [{ title: 'Daily Stand-up', cal: 'work', where: 'Teams', ...at(570, 15) }],
    };
    return plan[offset] || [];
  },
};
