/* App state and actions. In the real app this state lives in Rust services
   and reaches the UI as events; here everything is simulated. */

/**
 * Central app object: state, actions and the 1 s tick.
 */
const App = {
  state: {
    wifi: true,
    bluetooth: true,
    dnd: false,
    night: false,
    airplane: false,
    saver: false,
    mic: true,
    hotspot: false,
    network: MockData.networks[0].ssid,
    devices: MockData.devices.map((d) => ({ ...d })),
    volume: 0.62,
    brightness: 0.7,
    track: 0,
    position: 84,
    playing: true,
    battery: 0.82,
    charging: false,
    notifications: MockData.notifications.map((n, i) => ({ ...n, id: i + 1, at: Date.now() - n.minutesAgo * 60000 })),
    tasks: MockData.tasks.map((t) => ({ ...t })),
    processes: MockData.processes.map((p) => ({ ...p })),
    system: {
      cpu: randomWalk(32, 18, 4, 70, 9),
      gpu: randomWalk(32, 9, 2, 60, 6),
      down: randomWalk(32, 900, 20, 5200, 700),
      up: 120,
      ram: 11.4,
      ramTotal: 32,
    },
    timer: 0,
    pool: 0,
    beforeAirplane: null,
  },

  /** The island controller, created in {@link App.init}. */
  island: null,

  /** Current track object. */
  get track() {
    return MockData.tracks[this.state.track];
  },

  /** Boots the lab: renders views, creates the island, starts the tick. */
  init() {
    loadIconFonts();
    const root = document.documentElement;
    root.style.setProperty('--grain', Art.grain);

    IslandViews.mount(document.querySelector('.island__clip'));
    Panel.mount(document.querySelector('[data-view="panel"]'), this);

    this.island = new Island(document.querySelector('.island-anchor'), {
      onExpand: (tab) => Panel.onOpen(tab),
      onCollapse: () => Panel.onClose(),
      onWheel: (dy) => this.setVolume(this.state.volume - Math.sign(dy) * 0.04, true),
    });

    Lab.init(this);
    ThemeEngine.onChange(() => {
      this.island.onThemeChange();
      Panel.onThemeChange();
    });
    Lab.restore();

    this.applyTrack(false);
    this.setPlaying(true);
    this.setVolume(this.state.volume, false);
    this.setBrightness(this.state.brightness, false);
    this.updateBattery();
    this.tick();
    setInterval(() => this.tick(), 1000);

    // Fonts change metrics; re-measure once they are in.
    document.fonts.ready.then(() => this.island.refresh());
    document.fonts.addEventListener('loadingdone', () => this.island.refresh());

    this.bindGlobal();
  },

  /** @private */
  bindGlobal() {
    document.querySelector('.island').addEventListener('click', (e) => {
      const action = e.target.closest('[data-action]');
      if (!action) {
        return;
      }
      e.stopPropagation();
      if (action.dataset.action === 'play') {
        this.togglePlay();
      } else if (action.dataset.action === 'next') {
        this.skip(1);
      } else if (action.dataset.action === 'prev') {
        this.skip(-1);
      }
    });

    document.addEventListener('pointerdown', (e) => {
      if (this.island.expanded && !e.target.closest('.island, .lab')) {
        this.island.collapse();
      }
    });

    document.addEventListener('keydown', (e) => {
      if (e.key === 'Escape' && this.island.expanded) {
        this.island.collapse();
        document.querySelector('.island').focus({ preventScroll: true });
      }
    });
  },

  // ── Settings ────────────────────────────────────────────────────────────

  /**
   * Flips a quick setting, including Windows-like side effects (airplane
   * mode switches radios off and restores them afterwards).
   *
   * @param {string} id - Setting id, e.g. "wifi".
   */
  toggleSetting(id) {
    const s = this.state;
    s[id] = !s[id];
    if (id === 'airplane') {
      if (s.airplane) {
        s.beforeAirplane = { wifi: s.wifi, bluetooth: s.bluetooth, hotspot: s.hotspot };
        s.wifi = false;
        s.bluetooth = false;
        s.hotspot = false;
      } else if (s.beforeAirplane) {
        Object.assign(s, s.beforeAirplane);
      }
    }
    if ((id === 'wifi' || id === 'bluetooth') && s[id] && s.airplane) {
      s.airplane = false;
    }
    if (id === 'hotspot' && s.hotspot && !s.wifi) {
      s.wifi = true;
    }
    Panel.updateTiles();
    Panel.renderWifi();
    Panel.renderBluetooth();
  },

  /**
   * @param {number} value - 0…1.
   * @param {boolean} showOsd - Show the island OSD (only when collapsed).
   */
  setVolume(value, showOsd) {
    this.state.volume = Math.min(1, Math.max(0, value));
    Panel.updateSlider('volume', this.state.volume);
    IslandViews.setOsd('volume', this.state.volume);
    if (showOsd && !this.island.expanded) {
      this.island.upsert({ id: 'osd', priority: ISLAND_PRIORITY.osd, view: 'osd', ttl: 1700, label: 'Lautstärke' });
    }
  },

  /**
   * @param {number} value - 0…1.
   * @param {boolean} showOsd - Show the island OSD (only when collapsed).
   */
  setBrightness(value, showOsd) {
    this.state.brightness = Math.min(1, Math.max(0, value));
    Panel.updateSlider('brightness', this.state.brightness);
    IslandViews.setOsd('brightness', this.state.brightness);
    if (showOsd && !this.island.expanded) {
      this.island.upsert({ id: 'osd', priority: ISLAND_PRIORITY.osd, view: 'osd', ttl: 1700, label: 'Helligkeit' });
    }
  },

  // ── Media ───────────────────────────────────────────────────────────────

  /**
   * Pushes the current track to every view and recolours the UI.
   *
   * @param {boolean} animate - Animate the cover swap.
   * @private
   */
  applyTrack(animate) {
    const track = this.track;
    document.documentElement.style.setProperty('--media-color', track.color);
    IslandViews.setTrack(track, animate);
    Panel.updateMedia(track, animate);
    this.updateProgress(true);
  },

  /** @private */
  musicActivity() {
    const track = this.track;
    return {
      id: 'music',
      priority: ISLAND_PRIORITY.music,
      view: 'music',
      ttl: this.state.playing ? 0 : 12000,
      label: `${track.title} – ${track.artist}`,
      bubble: () => IslandViews.musicBubble(this.track),
    };
  },

  /** @param {boolean} playing - New playback state. */
  setPlaying(playing) {
    this.state.playing = playing;
    IslandViews.setPlaying(playing);
    Panel.updatePlaying(playing);
    this.island.upsert(this.musicActivity());
  },

  /** Toggles play/pause. */
  togglePlay() {
    this.setPlaying(!this.state.playing);
  },

  /**
   * Skips to the next or previous track.
   *
   * @param {number} dir - 1 or -1.
   */
  skip(dir) {
    const count = MockData.tracks.length;
    this.state.track = (this.state.track + dir + count) % count;
    this.state.position = 0;
    this.applyTrack(true);
    this.island.upsert({ ...this.musicActivity(), bump: true });
  },

  /**
   * Seeks within the current track.
   *
   * @param {number} fraction - 0…1.
   */
  seek(fraction) {
    this.state.position = Math.round(Math.min(1, Math.max(0, fraction)) * this.track.duration);
    this.updateProgress(true);
  },

  /** @private */
  updateProgress(jump = false) {
    IslandViews.setProgress(this.state.position, this.track.duration, jump);
    Panel.updateProgress(this.state.position, this.track.duration, jump);
  },

  // ── Notifications ───────────────────────────────────────────────────────

  /**
   * Delivers the next mock notification. With "Nicht stören" it only lands
   * in the list; otherwise it also gets a banner in the island.
   */
  pushNotification() {
    const pool = MockData.notificationPool;
    const template = pool[this.state.pool % pool.length];
    this.state.pool += 1;
    const n = { ...template, id: Date.now(), at: Date.now() };
    this.state.notifications.unshift(n);
    Panel.addNotification(n);
    this.updateUnread();
    if (this.island.expanded) {
      return;
    }
    if (this.state.dnd) {
      this.toast('Nicht stören: nur in der Liste zugestellt');
      return;
    }
    IslandViews.setNotification(n);
    this.island.upsert({
      id: 'notification',
      priority: ISLAND_PRIORITY.notification,
      view: 'notification',
      ttl: 4800,
      tab: 'notifications',
      label: `${n.app}: ${n.title}`,
      bump: true,
    });
  },

  /**
   * @param {number} id - Notification id.
   */
  removeNotification(id) {
    this.state.notifications = this.state.notifications.filter((n) => n.id !== id);
    this.updateUnread();
  },

  /** @private */
  updateUnread() {
    IslandViews.setClock(new Date(), this.state.notifications.length);
    if (this.island.current === 'clock') {
      this.island.refresh();
    }
  },

  // ── Other activities ────────────────────────────────────────────────────

  /** Starts a 5-minute timer (demonstrates the split bubble with music). */
  startTimer() {
    this.state.timer = 300;
    IslandViews.setTimer(this.state.timer);
    this.island.upsert({
      id: 'timer',
      priority: ISLAND_PRIORITY.timer,
      view: 'timer',
      tab: 'system',
      label: 'Timer',
      bubble: () => IslandViews.timerBubble(this.state.timer),
    });
  },

  /** Shows an upcoming calendar event in the island. */
  triggerEvent() {
    const events = MockData.eventsFor(new Date());
    const next = events.find((e) => e.start > new Date()) || events[0];
    IslandViews.setEvent(next, 5);
    this.island.upsert({
      id: 'event',
      priority: ISLAND_PRIORITY.event,
      view: 'event',
      ttl: 14000,
      tab: 'calendar',
      label: `${next.title} in 5 Minuten`,
      bubble: () => `<span class="ev-ring" style="--c:${MockData.calendars[next.cal].color};--p:.7"></span>`,
    });
  },

  /** Plugs or unplugs the (simulated) charger. */
  toggleCharging() {
    this.state.charging = !this.state.charging;
    this.updateBattery();
    if (this.state.charging) {
      IslandViews.setCharging(this.state.battery);
      this.island.upsert({ id: 'charging', priority: ISLAND_PRIORITY.charging, view: 'charging', ttl: 3200, label: 'Wird geladen' });
    } else {
      this.island.remove('charging');
    }
  },

  /** @private */
  updateBattery() {
    Panel.updateBattery(this.state.battery, this.state.charging);
  },

  /**
   * Shows a short message at the bottom of the screen.
   *
   * @param {string} message - Text to show.
   */
  toast(message) {
    Lab.toast(message);
  },

  // ── Tick ────────────────────────────────────────────────────────────────

  /** Advances clocks, playback, timers and system metrics. @private */
  tick() {
    const s = this.state;
    const now = new Date();
    IslandViews.setClock(now, s.notifications.length);
    Panel.updateClock(now);

    if (s.playing) {
      s.position += 1;
      if (s.position >= this.track.duration) {
        this.skip(1);
      } else {
        this.updateProgress();
      }
    }

    if (s.timer > 0) {
      s.timer -= 1;
      IslandViews.setTimer(s.timer);
      if (this.island.bubbleId === 'timer') {
        this.island.upsert({ id: 'timer' });
      }
      if (s.timer === 0) {
        this.island.remove('timer');
      }
    }

    const sys = s.system;
    const push = (arr, v) => {
      arr.push(v);
      arr.shift();
    };
    push(sys.cpu, Math.min(92, Math.max(3, sys.cpu.at(-1) + (Math.random() - 0.5) * 18)));
    push(sys.gpu, Math.min(85, Math.max(1, sys.gpu.at(-1) + (Math.random() - 0.5) * 12)));
    push(sys.down, Math.min(9000, Math.max(10, sys.down.at(-1) + (Math.random() - 0.5) * 1400)));
    sys.up = Math.max(8, sys.up + (Math.random() - 0.5) * 60);
    sys.ram = Math.min(sys.ramTotal - 2, Math.max(8, sys.ram + (Math.random() - 0.5) * 0.3));
    for (const p of s.processes) {
      p.cpu = Math.max(0, p.cpu + (Math.random() - 0.5) * 1.6);
      p.ram = Math.max(40, p.ram + (Math.random() - 0.5) * 24);
    }
    Panel.updateSystem();

    s.battery = Math.min(1, Math.max(0.05, s.battery + (s.charging ? 0.004 : -0.0002)));
    this.updateBattery();
    if (this.island.current === 'charging') {
      IslandViews.setCharging(s.battery);
    }

    if (now.getSeconds() === 0) {
      Panel.updateNotifMeta();
      if (this.island.expanded) {
        Panel.renderAgenda();
      }
    }
  },
};
