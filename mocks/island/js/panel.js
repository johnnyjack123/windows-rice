/* The expanded panel. Sections are described declaratively (TILES, TABS)
   so new ones can be added without touching the layout — the same idea as
   the plugin registries planned for the real app. */

/**
 * Quick-setting tiles. `detail` names a drill-in page; tiles without one
 * simply toggle. Labels and icons may depend on state.
 */
const TILES = {
  large: [
    {
      id: 'wifi', label: 'WLAN', detail: 'wifi',
      icon: (s) => (s.wifi ? 'wifi' : 'wifi_off'),
      sub: (s) => (s.wifi ? (s.network || 'Nicht verbunden') : 'Aus'),
    },
    {
      id: 'bluetooth', label: 'Bluetooth', detail: 'bluetooth',
      icon: (s) => (s.bluetooth ? 'bluetooth' : 'bluetooth_disabled'),
      sub: (s) => {
        if (!s.bluetooth) {
          return 'Aus';
        }
        const n = s.devices.filter((d) => d.connected).length;
        return n ? `${n} ${n === 1 ? 'Gerät' : 'Geräte'} verbunden` : 'Ein';
      },
    },
    {
      id: 'dnd', label: 'Nicht stören',
      icon: () => 'do_not_disturb_on',
      sub: (s) => (s.dnd ? 'An · bis 18:00' : 'Aus'),
    },
    {
      id: 'night', label: 'Nachtlicht',
      icon: () => 'nightlight',
      sub: (s) => (s.night ? 'An · bis 07:00' : 'Ab 21:00'),
    },
  ],
  small: [
    { id: 'airplane', label: 'Flugmodus', icon: () => 'airplanemode_active' },
    { id: 'saver', label: 'Energiesparen', icon: () => 'energy_savings_leaf' },
    { id: 'mic', label: 'Mikrofon', icon: (s) => (s.mic ? 'mic' : 'mic_off') },
    { id: 'hotspot', label: 'Mobiler Hotspot', icon: () => 'wifi_tethering' },
  ],
};

/** Tabs of the right-hand card. */
const TABS = [
  { id: 'notifications', label: 'Mitteilungen', icon: 'notifications' },
  { id: 'calendar', label: 'Kalender', icon: 'calendar_month' },
  { id: 'tasks', label: 'Aufgaben', icon: 'task_alt' },
  { id: 'system', label: 'System', icon: 'monitoring' },
];

const CHECK_SVG = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.2 4.2L19 7.5"/></svg>';
const SIGNAL_ICONS = ['network_wifi_1_bar', 'network_wifi_1_bar', 'network_wifi_2_bar', 'network_wifi_3_bar', 'signal_wifi_4_bar'];

/**
 * Escapes text for safe interpolation into markup.
 *
 * @param {string} value - Raw text.
 * @returns {string} Escaped text.
 */
function esc(value) {
  return String(value).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}

/**
 * Switches between two stacked panels (pages or panes) with a direction-
 * aware slide. The incoming panel is placed on the correct side without a
 * transition first, so it never slides in from the wrong direction.
 *
 * @param {HTMLElement} container - Parent that carries `--dir`.
 * @param {HTMLElement} from - Currently visible panel.
 * @param {HTMLElement} to - Panel to show.
 * @param {number} dir - 1 = forward, -1 = backward.
 */
function swapPanels(container, from, to, dir) {
  if (from === to) {
    return;
  }
  container.style.setProperty('--dir', String(dir));
  for (const el of container.children) {
    if (el !== from) {
      el.classList.remove('is-leaving');
    }
  }
  to.style.transition = 'none';
  to.classList.remove('is-current');
  void to.offsetWidth;
  to.style.transition = '';
  from.classList.remove('is-current');
  from.classList.add('is-leaving');
  from.inert = true;
  to.classList.add('is-current');
  to.inert = false;
}

/**
 * Expanded panel controller.
 */
const Panel = {
  tab: 'notifications',
  page: 'home',
  day: new Date(),
  taskFilter: 'all',
  procSort: 'cpu',
  sparkStyle: 'smooth',
  scanning: false,
  connecting: null,
  searching: false,

  /**
   * Renders the panel and wires up its interactions.
   *
   * @param {HTMLElement} root - The panel view element.
   * @param {object} app - The App object (state + actions).
   */
  mount(root, app) {
    this.root = root;
    this.app = app;
    this.s = app.state;
    root.innerHTML = this.template();
    const $ = (sel) => root.querySelector(sel);
    this.el = {
      clock: $('[data-p-clock]'),
      date: $('[data-p-date]'),
      battery: $('[data-p-battery]'),
      batteryPct: $('[data-p-battery-pct]'),
      stack: $('[data-stack]'),
      large: $('[data-tiles-large]'),
      small: $('[data-tiles-small]'),
      tabs: $('[data-tabs]'),
      panes: $('[data-panes]'),
      nList: $('[data-n-list]'),
      nEmpty: $('[data-n-empty]'),
      nCount: $('[data-n-count]'),
      nClear: $('[data-clear]'),
      week: $('[data-week]'),
      agenda: $('[data-agenda]'),
      taskLists: $('[data-task-lists]'),
      taskForm: $('[data-task-form]'),
      taskInput: $('[data-task-input]'),
      tasksOpen: $('[data-tasks-open]'),
      tasksDone: $('[data-tasks-done]'),
      doneLabel: $('[data-done-label]'),
      meters: $('[data-meters]'),
      procs: $('[data-procs]'),
      procSort: $('[data-proc-sort]'),
      wifiBody: $('[data-wifi-body]'),
      btBody: $('[data-bt-body]'),
      media: $('[data-media]'),
    };
    for (const page of root.querySelectorAll('.page:not(.is-current), .pane:not(.is-current)')) {
      page.inert = true;
    }
    this.renderTiles();
    this.renderNotifications();
    this.renderWeek();
    this.renderAgenda();
    this.renderTaskLists();
    this.renderTasks();
    this.renderProcs();
    this.renderWifi();
    this.renderBluetooth();
    this.bind();
  },

  /** @returns {string} Static panel markup. @private */
  template() {
    const slider = (key, label, icon) => `
      <div class="slider" data-slider="${key}" role="slider" tabindex="0" aria-label="${label}" aria-valuemin="0" aria-valuemax="100">
        <div class="slider__track"><div class="slider__fill"></div></div>
        <span class="slider__icon" data-slider-icon>${ic(icon)}</span>
        <span class="slider__handle"></span>
      </div>`;
    const tabs = TABS.map((t, i) => `
      <button class="tab has-state ${t.id === this.tab ? 'is-active' : ''}" role="tab" data-tab="${t.id}"
        aria-selected="${t.id === this.tab}" title="${t.label}">
        ${ic(t.icon)}<span class="tab__label">${t.label}</span>${i === 0 ? '<span class="tab__badge num" data-badge></span>' : ''}
      </button>`).join('');
    const meter = (id, label, sub, unit, bar) => `
      <div class="meter" data-meter="${id}">
        <div class="meter__head"><span class="meter__label">${label}</span><span class="meter__value num"><span data-v>0</span><small>${unit}</small></span></div>
        ${bar
          ? '<div class="meter__bar"><div class="meter__fill" data-fill></div></div>'
          : '<svg class="spark" viewBox="0 0 100 28" preserveAspectRatio="none" aria-hidden="true"><path class="area"/><path class="line"/></svg>'}
        <div class="meter__sub" data-sub>${sub}</div>
      </div>`;

    return `
      <header class="p-header" data-stagger style="--i:0">
        <div class="p-time">
          <span class="p-clock num" data-p-clock>--:--</span>
          <span class="p-date" data-p-date></span>
        </div>
        <div class="p-status">
          <span class="p-battery"><span class="battery" data-p-battery><i></i></span><span class="num" data-p-battery-pct></span></span>
          <button class="icon-btn has-state" aria-label="Kacheln bearbeiten" title="Kacheln bearbeiten">${ic('edit')}</button>
          <button class="icon-btn has-state" aria-label="Einstellungen" title="Einstellungen">${ic('settings')}</button>
          <button class="icon-btn has-state" aria-label="Energieoptionen" title="Energieoptionen">${ic('power_settings_new')}</button>
        </div>
      </header>

      <div class="p-body">
        <div class="p-left" data-stack>
          <div class="page is-current" data-page="home">
            <div class="tiles" data-tiles-large data-stagger style="--i:1"></div>
            <div class="tiles tiles--small" data-tiles-small data-stagger style="--i:2"></div>
            <div class="sliders" data-stagger style="--i:3">
              ${slider('volume', 'Lautstärke', 'volume_up')}
              ${slider('brightness', 'Helligkeit', 'brightness_medium')}
            </div>
            <section class="media reseed" data-media data-stagger style="--i:4" aria-label="Medienwiedergabe">
              <span class="art media__backdrop" data-media-art></span>
              <div class="media__row">
                <span class="art media__art" data-media-art></span>
                <div class="truncate">
                  <div class="media__title truncate" data-media-title></div>
                  <div class="media__artist truncate" data-media-artist></div>
                </div>
                <div class="media__controls">
                  <button class="icon-btn has-state" data-action="prev" aria-label="Vorheriger Titel">${ic('skip_previous')}</button>
                  <button class="icon-btn has-state media__play" data-action="play" aria-label="Wiedergabe umschalten">${ic('pause')}</button>
                  <button class="icon-btn has-state" data-action="next" aria-label="Nächster Titel">${ic('skip_next')}</button>
                </div>
              </div>
              <div class="progress" data-media-progress>
                <span class="num" data-pos>0:00</span>
                <div class="progress__track" data-seek><div class="progress__fill"></div></div>
                <span class="num" data-rem>-0:00</span>
              </div>
              <div class="media__foot">${ic('headphones')}<span data-output>WH-1000XM5</span><span>·</span><span>Spotify</span></div>
            </section>
          </div>

          <div class="page page--detail" data-page="wifi">
            <header class="detail-head">
              <button class="icon-btn has-state" data-back aria-label="Zurück">${ic('arrow_back')}</button>
              <span class="detail-title">WLAN</span>
              <button class="switch" role="switch" data-switch="wifi" aria-label="WLAN"></button>
            </header>
            <div class="detail-body scroll" data-wifi-body></div>
            <footer class="detail-foot">
              <button class="btn btn--text has-state" data-settings="ms-settings:network-wifi">Weitere Einstellungen ${ic('open_in_new')}</button>
            </footer>
          </div>

          <div class="page page--detail" data-page="bluetooth">
            <header class="detail-head">
              <button class="icon-btn has-state" data-back aria-label="Zurück">${ic('arrow_back')}</button>
              <span class="detail-title">Bluetooth</span>
              <button class="switch" role="switch" data-switch="bluetooth" aria-label="Bluetooth"></button>
            </header>
            <div class="detail-body scroll" data-bt-body></div>
            <footer class="detail-foot">
              <button class="btn btn--text has-state" data-settings="ms-settings:bluetooth">Weitere Einstellungen ${ic('open_in_new')}</button>
            </footer>
          </div>
        </div>

        <section class="p-right" data-stagger style="--i:2">
          <div class="tabs" role="tablist" aria-label="Bereiche" data-tabs>${tabs}</div>
          <div class="panes" data-panes>
            <div class="pane is-current" data-pane="notifications" role="tabpanel" aria-label="Mitteilungen">
              <div class="pane-toolbar">
                <span class="pane-title" data-n-count></span>
                <button class="btn btn--text has-state" data-clear>Alle löschen</button>
              </div>
              <div class="pane__scroll scroll">
                <ul class="notif-list" data-n-list></ul>
                <div class="empty" data-n-empty hidden>
                  ${ic('done_all')}
                  <div class="empty__title">Alles erledigt</div>
                  <div>Keine neuen Mitteilungen</div>
                </div>
              </div>
            </div>

            <div class="pane" data-pane="calendar" role="tabpanel" aria-label="Kalender">
              <div class="week" data-week></div>
              <div class="pane__scroll scroll"><div class="agenda" data-agenda></div></div>
              <div class="pane-foot">${ic('sync')}<span>Nextcloud · 3 Kalender · vor 2 Min synchronisiert</span></div>
            </div>

            <div class="pane" data-pane="tasks" role="tabpanel" aria-label="Aufgaben">
              <div class="task-lists" data-task-lists></div>
              <form class="field task-add" data-task-form>
                ${ic('add')}
                <input data-task-input placeholder="Neue Aufgabe …" aria-label="Neue Aufgabe" autocomplete="off">
              </form>
              <div class="pane__scroll scroll">
                <ul class="tasks" data-tasks-open></ul>
                <div class="section-label" data-done-label></div>
                <ul class="tasks" data-tasks-done></ul>
              </div>
              <div class="pane-foot">${ic('sync')}<span>CalDAV · VTODO · synchron mit tasks.org</span></div>
            </div>

            <div class="pane" data-pane="system" role="tabpanel" aria-label="System">
              <div class="meters" data-meters>
                ${meter('cpu', 'CPU', 'Ryzen 7 7800X3D · 4,7 GHz', '%')}
                ${meter('ram', 'RAM', '', 'GB', true)}
                ${meter('gpu', 'GPU', 'RTX 4070 · 2,1 / 12 GB', '%')}
                ${meter('net', 'Netz', '', 'MB/s')}
              </div>
              <div class="section-label">Prozesse
                <span class="seg" data-proc-sort>
                  <button class="is-active" data-sort="cpu">CPU</button><button data-sort="ram">RAM</button>
                </span>
              </div>
              <div class="pane__scroll scroll"><ul class="procs" data-procs></ul></div>
            </div>
          </div>
        </section>
      </div>`;
  },

  // ── Lifecycle ───────────────────────────────────────────────────────────

  /**
   * Called when the panel opens.
   *
   * @param {string} [tab] - Tab to show.
   */
  onOpen(tab) {
    if (tab) {
      this.setTab(tab, true);
    }
    this.day = new Date();
    this.renderWeek();
    this.renderAgenda();
    this.renderNotifications();
  },

  /** Called when the panel closes; drill-ins reset once it is hidden. */
  onClose() {
    clearTimeout(this.resetTimer);
    this.resetTimer = setTimeout(() => {
      if (!this.app.island.expanded) {
        this.openPage('home', true);
      }
    }, 450 / Motion.timeScale);
  },

  /** Re-reads theme-dependent settings. */
  onThemeChange() {
    this.sparkStyle = ThemeEngine.token('--spark-style') || 'smooth';
    this.renderSparks();
  },

  // ── Header ──────────────────────────────────────────────────────────────

  /** @param {Date} now - Current time. */
  updateClock(now) {
    this.el.clock.textContent = Fmt.time(now);
    this.el.date.textContent = Fmt.longDate(now);
  },

  /**
   * @param {number} level - 0…1.
   * @param {boolean} charging - Charger connected.
   */
  updateBattery(level, charging) {
    this.el.battery.style.setProperty('--level', String(level));
    this.el.battery.classList.toggle('is-charging', charging);
    this.el.batteryPct.textContent = `${Math.round(level * 100)} %`;
  },

  // ── Tiles & sliders ─────────────────────────────────────────────────────

  /** @private */
  renderTiles() {
    const s = this.s;
    this.el.large.innerHTML = TILES.large.map((t) => `
      <div class="tile ${s[t.id] ? 'is-on' : ''}" data-tile="${t.id}">
        <button class="tile__toggle has-state" data-toggle="${t.id}" aria-pressed="${Boolean(s[t.id])}" aria-label="${t.label} umschalten">${ic(t.icon(s))}</button>
        <button class="tile__main has-state" ${t.detail ? `data-open="${t.detail}"` : `data-toggle="${t.id}"`} aria-label="${t.label}${t.detail ? ' – Details' : ''}">
          <span class="truncate"><span class="tile__label">${t.label}</span><span class="tile__sub" data-sub>${esc(t.sub(s))}</span></span>
          ${t.detail ? ic('chevron_right', 'tile__chev') : ''}
        </button>
      </div>`).join('');
    this.el.small.innerHTML = TILES.small.map((t) => `
      <button class="tile tile--small has-state ${s[t.id] ? 'is-on' : ''}" data-tile="${t.id}" data-toggle="${t.id}"
        aria-pressed="${Boolean(s[t.id])}" aria-label="${t.label}" title="${t.label}">${ic(t.icon(s))}</button>`).join('');
    this.updateSwitches();
  },

  /** Updates every tile in place so state transitions animate. */
  updateTiles() {
    const s = this.s;
    for (const t of [...TILES.large, ...TILES.small]) {
      const el = this.root.querySelector(`[data-tile="${t.id}"]`);
      el.classList.toggle('is-on', Boolean(s[t.id]));
      el.querySelector('.ic').textContent = t.icon(s);
      for (const b of el.querySelectorAll('[aria-pressed]')) {
        b.setAttribute('aria-pressed', String(Boolean(s[t.id])));
      }
      if (el.matches('[aria-pressed]')) {
        el.setAttribute('aria-pressed', String(Boolean(s[t.id])));
      }
      const sub = el.querySelector('[data-sub]');
      if (sub) {
        sub.textContent = t.sub(s);
      }
    }
    this.updateSwitches();
  },

  /** @private */
  updateSwitches() {
    for (const sw of this.root.querySelectorAll('[data-switch]')) {
      sw.setAttribute('aria-checked', String(Boolean(this.s[sw.dataset.switch])));
    }
  },

  /**
   * @param {'volume'|'brightness'} key - Slider id.
   * @param {number} value - 0…1.
   */
  updateSlider(key, value) {
    const el = this.root.querySelector(`[data-slider="${key}"]`);
    el.style.setProperty('--value', String(value));
    el.setAttribute('aria-valuenow', String(Math.round(value * 100)));
    let icon;
    if (key === 'volume') {
      icon = value === 0 ? 'volume_off' : value < 0.5 ? 'volume_down' : 'volume_up';
    } else {
      icon = value < 0.34 ? 'brightness_low' : value < 0.67 ? 'brightness_medium' : 'brightness_high';
    }
    el.querySelector('.ic').textContent = icon;
  },

  /**
   * Makes a slider draggable, scrollable and keyboard-operable.
   *
   * @param {HTMLElement} el - Slider element.
   * @param {function(number): void} onChange - Receives values 0…1.
   * @private
   */
  bindSlider(el, onChange) {
    const valueAt = (clientX) => {
      const rect = el.getBoundingClientRect();
      const h = rect.height;
      return Math.min(1, Math.max(0, (clientX - rect.left - h / 2) / (rect.width - h)));
    };
    const current = () => parseFloat(el.style.getPropertyValue('--value')) || 0;
    el.addEventListener('pointerdown', (e) => {
      el.setPointerCapture(e.pointerId);
      el.classList.add('is-dragging');
      onChange(valueAt(e.clientX));
    });
    el.addEventListener('pointermove', (e) => {
      if (el.hasPointerCapture(e.pointerId)) {
        onChange(valueAt(e.clientX));
      }
    });
    const end = (e) => {
      if (el.hasPointerCapture(e.pointerId)) {
        el.releasePointerCapture(e.pointerId);
      }
      el.classList.remove('is-dragging');
    };
    el.addEventListener('pointerup', end);
    el.addEventListener('pointercancel', end);
    el.addEventListener('wheel', (e) => {
      e.preventDefault();
      onChange(Math.min(1, Math.max(0, current() - Math.sign(e.deltaY) * 0.05)));
    }, { passive: false });
    el.addEventListener('keydown', (e) => {
      const steps = { ArrowRight: 0.05, ArrowUp: 0.05, ArrowLeft: -0.05, ArrowDown: -0.05, PageUp: 0.2, PageDown: -0.2 };
      if (e.key in steps) {
        e.preventDefault();
        onChange(Math.min(1, Math.max(0, current() + steps[e.key])));
      } else if (e.key === 'Home' || e.key === 'End') {
        e.preventDefault();
        onChange(e.key === 'Home' ? 0 : 1);
      }
    });
  },

  // ── Media ───────────────────────────────────────────────────────────────

  /**
   * @param {object} track - Current track.
   * @param {boolean} animate - Animate the cover swap.
   */
  updateMedia(track, animate) {
    for (const art of this.root.querySelectorAll('[data-media-art]')) {
      art.style.setProperty('--art', Art.covers[track.id]);
      if (animate) {
        art.classList.remove('is-swapping');
        void art.offsetWidth;
        art.classList.add('is-swapping');
      }
    }
    this.root.querySelector('[data-media-title]').textContent = track.title;
    this.root.querySelector('[data-media-artist]').textContent = track.artist;
  },

  /** @param {boolean} playing - Playback state. */
  updatePlaying(playing) {
    this.root.querySelector('.media__play .ic').textContent = playing ? 'pause' : 'play_arrow';
  },

  /**
   * @param {number} position - Seconds played.
   * @param {number} duration - Track length.
   * @param {boolean} [jump=false] - Disable smoothing.
   */
  updateProgress(position, duration, jump = false) {
    const p = this.root.querySelector('[data-media-progress]');
    p.classList.toggle('is-jump', jump);
    p.style.setProperty('--p', String(Math.min(1, position / duration)));
    p.querySelector('[data-pos]').textContent = Fmt.duration(position);
    p.querySelector('[data-rem]').textContent = `-${Fmt.duration(duration - position)}`;
  },

  // ── Navigation ──────────────────────────────────────────────────────────

  /**
   * Shows a tab of the right card.
   *
   * @param {string} id - Tab id.
   * @param {boolean} [instant=false] - Skip the slide (used while hidden).
   */
  setTab(id, instant = false) {
    if (id === this.tab) {
      return;
    }
    const order = TABS.map((t) => t.id);
    const dir = order.indexOf(id) > order.indexOf(this.tab) ? 1 : -1;
    const from = this.root.querySelector(`[data-pane="${this.tab}"]`);
    const to = this.root.querySelector(`[data-pane="${id}"]`);
    this.tab = id;
    for (const tab of this.el.tabs.children) {
      const active = tab.dataset.tab === id;
      tab.classList.toggle('is-active', active);
      tab.setAttribute('aria-selected', String(active));
    }
    swapPanels(this.el.panes, from, to, instant ? 0 : dir);
    if (id === 'system') {
      this.renderSparks();
    }
  },

  /**
   * Pushes or pops a page of the left column.
   *
   * @param {string} id - Page id ("home", "wifi", "bluetooth").
   * @param {boolean} [instant=false] - Skip the slide.
   */
  openPage(id, instant = false) {
    if (id === this.page) {
      return;
    }
    const from = this.root.querySelector(`[data-page="${this.page}"]`);
    const to = this.root.querySelector(`[data-page="${id}"]`);
    const dir = id === 'home' ? -1 : 1;
    this.page = id;
    if (id === 'wifi') {
      this.scan();
    }
    swapPanels(this.el.stack, from, to, instant ? 0 : dir);
    if (!instant) {
      to.querySelector('[data-back]')?.focus({ preventScroll: true });
    }
  },

  // ── Notifications ───────────────────────────────────────────────────────

  /** Rebuilds the notification list. */
  renderNotifications() {
    const list = this.s.notifications;
    this.el.nList.innerHTML = list.map((n) => this.notifMarkup(n)).join('');
    this.updateNotifMeta();
  },

  /**
   * @param {object} n - Notification.
   * @param {boolean} [isNew=false] - Animate in.
   * @returns {string} Markup.
   * @private
   */
  notifMarkup(n, isNew = false) {
    const minutes = (Date.now() - n.at) / 60000;
    return `
      <li class="notif-wrap ${isNew ? 'is-new' : ''}" data-nid="${n.id}">
        <div class="notif-clip">
          <article class="notif" tabindex="0" aria-label="${esc(n.app)}: ${esc(n.title)}">
            <span class="app-icon" style="--app-color:${n.color}">${ic(n.icon)}</span>
            <div class="notif__main">
              <div class="notif__meta"><span>${esc(n.app)}</span><span data-ago>${Fmt.ago(minutes)}</span></div>
              <div class="notif__title">${esc(n.title)}</div>
              <div class="notif__body">${esc(n.body)}</div>
              ${n.actions.length ? `<div class="notif__actions">${n.actions.map((a) => `<button class="chip has-state" data-dismiss>${esc(a)}</button>`).join('')}</div>` : ''}
            </div>
            <button class="icon-btn has-state notif__close" data-dismiss aria-label="Mitteilung schließen">${ic('close')}</button>
          </article>
        </div>
      </li>`;
  },

  /**
   * Inserts a notification at the top of the list with an animation.
   *
   * @param {object} n - Notification (already in state).
   */
  addNotification(n) {
    this.el.nList.insertAdjacentHTML('afterbegin', this.notifMarkup(n, true));
    this.updateNotifMeta();
  },

  /** Refreshes counters, empty state and relative times. */
  updateNotifMeta() {
    const count = this.s.notifications.length;
    this.el.nCount.textContent = count === 1 ? '1 Mitteilung' : `${count} Mitteilungen`;
    this.el.nEmpty.hidden = count > 0;
    this.el.nClear.hidden = count === 0;
    const badge = this.root.querySelector('[data-badge]');
    badge.textContent = String(count);
    badge.hidden = count === 0;
    for (const li of this.el.nList.children) {
      const n = this.s.notifications.find((x) => String(x.id) === li.dataset.nid);
      if (n) {
        li.querySelector('[data-ago]').textContent = Fmt.ago((Date.now() - n.at) / 60000);
      }
    }
  },

  /**
   * Animates a notification away and removes it from state.
   *
   * @param {HTMLElement} wrap - The `.notif-wrap` element.
   * @param {number} [direction=1] - Fly-out direction.
   * @private
   */
  dismiss(wrap, direction = 1) {
    if (wrap.classList.contains('is-removing')) {
      return;
    }
    const card = wrap.querySelector('.notif');
    card.style.translate = `${direction * 110}% 0`;
    card.style.opacity = '0';
    wrap.classList.add('is-removing');
    this.app.removeNotification(Number(wrap.dataset.nid));
    setTimeout(() => wrap.remove(), 420 / Motion.timeScale);
    this.updateNotifMeta();
  },

  /** @private */
  bindSwipe() {
    let drag = null;
    this.el.nList.addEventListener('pointerdown', (e) => {
      const card = e.target.closest('.notif');
      if (!card || e.target.closest('button')) {
        return;
      }
      drag = { card, x: e.clientX, dx: 0, id: e.pointerId };
      card.setPointerCapture(e.pointerId);
    });
    this.el.nList.addEventListener('pointermove', (e) => {
      if (!drag || e.pointerId !== drag.id) {
        return;
      }
      drag.dx = e.clientX - drag.x;
      if (Math.abs(drag.dx) > 4) {
        drag.card.classList.add('is-dragging');
      }
      drag.card.style.translate = `${drag.dx}px 0`;
      drag.card.style.opacity = String(1 - Math.min(0.7, Math.abs(drag.dx) / 320));
    });
    const end = () => {
      if (!drag) {
        return;
      }
      const { card, dx } = drag;
      card.classList.remove('is-dragging');
      if (Math.abs(dx) > 90) {
        this.dismiss(card.closest('.notif-wrap'), Math.sign(dx));
      } else {
        card.style.translate = '';
        card.style.opacity = '';
      }
      drag = null;
    };
    this.el.nList.addEventListener('pointerup', end);
    this.el.nList.addEventListener('pointercancel', end);
  },

  // ── Calendar ────────────────────────────────────────────────────────────

  /** @private */
  renderWeek() {
    const today = dayOffset(new Date(), 0);
    const monday = dayOffset(today, -((today.getDay() + 6) % 7));
    const selected = dayOffset(this.day, 0).getTime();
    this.el.week.innerHTML = Array.from({ length: 7 }, (_, i) => {
      const d = dayOffset(monday, i);
      const cals = [...new Set(MockData.eventsFor(d).map((e) => e.cal))];
      const cls = [d.getTime() === today.getTime() ? 'is-today' : '', d.getTime() === selected ? 'is-selected' : ''].join(' ');
      return `
        <button class="day has-state ${cls}" data-day="${d.getTime()}" aria-label="${Fmt.longDate(d)}" aria-pressed="${d.getTime() === selected}">
          <span class="day__dow">${Fmt.weekday(d)}</span>
          <span class="day__num num">${d.getDate()}</span>
          <span class="day__dots">${cals.slice(0, 3).map((c) => `<i style="--c:${MockData.calendars[c].color}"></i>`).join('')}</span>
        </button>`;
    }).join('');
  },

  /** Renders the agenda for the selected day, including the "now" line. */
  renderAgenda() {
    const now = new Date();
    const day = dayOffset(this.day, 0);
    const offset = Math.round((day - dayOffset(now, 0)) / 86400000);
    const labels = { 0: 'Heute', 1: 'Morgen', [-1]: 'Gestern' };
    const label = labels[offset] || Fmt.longDate(day);
    const events = MockData.eventsFor(day);
    if (!events.length) {
      this.el.agenda.innerHTML = `
        <div class="agenda__label">${label}</div>
        <div class="empty">${ic('event')}<div class="empty__title">Keine Termine</div><div>Ein freier Tag.</div></div>`;
      return;
    }
    let nowPlaced = offset !== 0;
    let nextMarked = false;
    const rows = [];
    for (const e of events) {
      if (!nowPlaced && e.start > now) {
        rows.push(`<div class="now-line num">${Fmt.time(now)}</div>`);
        nowPlaced = true;
      }
      const cal = MockData.calendars[e.cal];
      const past = e.end < now;
      const running = e.start <= now && e.end > now;
      let chip = '';
      if (running) {
        chip = '<span class="chip chip--accent">Jetzt</span>';
      } else if (!past && offset === 0 && !nextMarked) {
        chip = `<span class="chip chip--accent">${Fmt.until((e.start - now) / 60000)}</span>`;
        nextMarked = true;
      }
      rows.push(`
        <div class="event-row ${past ? 'is-past' : ''}" style="--c:${cal.color}">
          <span class="event-row__bar"></span>
          <span class="event-row__time num"><span>${Fmt.time(e.start)}</span><span>${Fmt.time(e.end)}</span></span>
          <span class="truncate">
            <span class="event-row__title">${esc(e.title)}</span>
            <span class="event-row__meta">${cal.name}${e.where ? ` · ${esc(e.where)}` : ''}</span>
          </span>
          ${chip}
        </div>`);
    }
    if (!nowPlaced) {
      rows.push(`<div class="now-line num">${Fmt.time(now)}</div>`);
    }
    this.el.agenda.innerHTML = `<div class="agenda__label">${label}</div>${rows.join('')}`;
  },

  // ── Tasks ───────────────────────────────────────────────────────────────

  /** @private */
  renderTaskLists() {
    const lists = [['all', 'Alle'], ...Object.entries(MockData.taskLists).map(([id, l]) => [id, l.name])];
    this.el.taskLists.innerHTML = lists.map(([id, name]) => `
      <button class="chip has-state ${id === this.taskFilter ? 'is-selected' : ''}" data-filter="${id}" aria-pressed="${id === this.taskFilter}">${name}</button>`).join('');
    const target = this.taskFilter === 'all' ? 'Rice' : MockData.taskLists[this.taskFilter].name;
    this.el.taskInput.placeholder = `Neue Aufgabe in „${target}“ …`;
  },

  /**
   * @param {object} t - Task.
   * @param {boolean} [isNew=false] - Animate in.
   * @returns {string} Markup.
   * @private
   */
  taskMarkup(t, isNew = false) {
    const list = MockData.taskLists[t.list];
    const due = t.due && !t.done ? MockData.dueLabels[t.due] : null;
    return `
      <li class="task has-state ${t.done ? 'is-done' : ''} ${isNew ? 'is-new' : ''}" data-task="${t.id}">
        <button class="check" role="checkbox" aria-checked="${t.done}" aria-label="Erledigt: ${esc(t.title)}">${CHECK_SVG}</button>
        <div class="task__main">
          <div class="task__title">${esc(t.title)}</div>
          <div class="task__meta"><span class="dot" style="--c:${list.color}"></span>${list.name}${due ? ` · <span class="due ${due.cls}">${due.text}</span>` : ''}</div>
        </div>
      </li>`;
  },

  /** Renders open and completed tasks for the current filter. */
  renderTasks() {
    const visible = this.s.tasks.filter((t) => this.taskFilter === 'all' || t.list === this.taskFilter);
    const open = visible.filter((t) => !t.done);
    const done = visible.filter((t) => t.done);
    this.el.tasksOpen.innerHTML = open.length
      ? open.map((t) => this.taskMarkup(t)).join('')
      : `<li class="empty">${ic('task_alt')}<div class="empty__title">Nichts offen</div><div>Gönn dir eine Pause.</div></li>`;
    this.el.doneLabel.textContent = `Erledigt · ${done.length}`;
    this.el.doneLabel.hidden = done.length === 0;
    this.el.tasksDone.innerHTML = done.map((t) => this.taskMarkup(t)).join('');
  },

  /**
   * Toggles a task with a short delay, so the check animation is visible
   * before the row moves to the other section.
   *
   * @param {number} id - Task id.
   * @private
   */
  toggleTask(id) {
    const task = this.s.tasks.find((t) => t.id === id);
    const row = this.root.querySelector(`[data-task="${id}"]`);
    task.done = !task.done;
    row.classList.toggle('is-done', task.done);
    row.querySelector('.check').setAttribute('aria-checked', String(task.done));
    clearTimeout(this.taskTimer);
    this.taskTimer = setTimeout(() => {
      row.classList.add('is-leaving');
      setTimeout(() => this.renderTasks(), 300 / Motion.timeScale);
    }, 650 / Motion.timeScale);
  },

  // ── System ──────────────────────────────────────────────────────────────

  /**
   * Updates the resource meters with a new sample. Rendering only happens
   * while the System tab is visible; history is always recorded.
   */
  updateSystem() {
    const sys = this.s.system;
    if (!this.app.island.expanded || this.tab !== 'system') {
      return;
    }
    const meter = (id) => this.el.meters.querySelector(`[data-meter="${id}"]`);
    meter('cpu').querySelector('[data-v]').textContent = Math.round(sys.cpu.at(-1));
    meter('gpu').querySelector('[data-v]').textContent = Math.round(sys.gpu.at(-1));
    const ram = meter('ram');
    ram.querySelector('[data-v]').textContent = Fmt.num(sys.ram);
    ram.querySelector('[data-fill]').style.setProperty('--p', String(sys.ram / sys.ramTotal));
    ram.querySelector('[data-sub]').textContent = `von ${sys.ramTotal} GB · ${Math.round((sys.ram / sys.ramTotal) * 100)} %`;
    const net = meter('net');
    net.querySelector('[data-v]').textContent = Fmt.num(sys.down.at(-1) / 1024);
    net.querySelector('[data-sub]').innerHTML = `↓ ${Fmt.rate(sys.down.at(-1))} · ↑ ${Fmt.rate(sys.up)}`;
    this.renderSparks();
    for (const row of this.el.procs.children) {
      const p = this.s.processes.find((x) => x.name === row.dataset.proc);
      if (p) {
        const [cpu, ram] = row.querySelectorAll('.proc__vals span');
        cpu.textContent = `${Fmt.num(p.cpu)} %`;
        ram.textContent = Fmt.mem(p.ram);
      }
    }
  },

  /** @private */
  renderSparks() {
    const sys = this.s.system;
    const draw = (id, values, max) => {
      const svg = this.el.meters.querySelector(`[data-meter="${id}"] svg`);
      const { line, area } = this.sparkPath(values, max);
      svg.querySelector('.line').setAttribute('d', line);
      svg.querySelector('.area').setAttribute('d', area);
    };
    draw('cpu', sys.cpu, 100);
    draw('gpu', sys.gpu, 100);
    draw('net', sys.down, Math.max(3000, ...sys.down));
  },

  /**
   * Builds sparkline paths in the theme's curve style.
   *
   * @param {number[]} values - Samples, oldest first.
   * @param {number} max - Value mapped to the top edge.
   * @returns {{line: string, area: string}} SVG path data.
   * @private
   */
  sparkPath(values, max) {
    const W = 100;
    const H = 28;
    const pts = values.map((v, i) => [(i / (values.length - 1)) * W, H - 2 - (Math.min(v, max) / max) * (H - 4)]);
    const f = (n) => n.toFixed(2);
    let line = `M${f(pts[0][0])},${f(pts[0][1])}`;
    if (this.sparkStyle === 'step') {
      for (let i = 1; i < pts.length; i++) {
        line += ` H${f(pts[i][0])} V${f(pts[i][1])}`;
      }
    } else if (this.sparkStyle === 'linear') {
      for (let i = 1; i < pts.length; i++) {
        line += ` L${f(pts[i][0])},${f(pts[i][1])}`;
      }
    } else {
      for (let i = 0; i < pts.length - 1; i++) {
        const p0 = pts[i - 1] || pts[i];
        const p1 = pts[i];
        const p2 = pts[i + 1];
        const p3 = pts[i + 2] || p2;
        const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6];
        const c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
        line += ` C${f(c1[0])},${f(c1[1])} ${f(c2[0])},${f(c2[1])} ${f(p2[0])},${f(p2[1])}`;
      }
    }
    return { line, area: `${line} L${W},${H} L0,${H} Z` };
  },

  /** @private */
  renderProcs() {
    const key = this.procSort;
    const procs = [...this.s.processes].sort((a, b) => b[key] - a[key]);
    this.el.procs.innerHTML = procs.map((p) => `
      <li class="row proc has-state" data-proc="${p.name}">
        <span class="proc__icon" style="--app-color:${p.color}">${p.name[0].toUpperCase()}</span>
        <span class="row__title">${p.name}</span>
        <span class="proc__vals num"><span>${Fmt.num(p.cpu)} %</span><span>${Fmt.mem(p.ram)}</span></span>
        <button class="icon-btn has-state proc__kill" data-kill="${p.name}" aria-label="${p.name} beenden" title="Beenden">${ic('close')}</button>
      </li>`).join('');
  },

  // ── WLAN ────────────────────────────────────────────────────────────────

  /** Simulates a network scan when the WLAN page opens. */
  scan() {
    this.scanning = true;
    this.renderWifi();
    clearTimeout(this.scanTimer);
    this.scanTimer = setTimeout(() => {
      this.scanning = false;
      this.renderWifi();
    }, 1600 / Motion.timeScale);
  },

  /** Renders the WLAN drill-in page. */
  renderWifi() {
    const s = this.s;
    if (!s.wifi) {
      this.el.wifiBody.innerHTML = `
        <div class="detail-empty">${ic('wifi_off')}<div class="empty__title">WLAN ist aus</div><div>Schalte WLAN ein, um Netzwerke in der Nähe zu sehen.</div></div>`;
      return;
    }
    const net = (n, connected) => {
      const connecting = this.connecting === n.ssid;
      let sub = n.secure ? 'Gesichert' : 'Offen';
      if (connected) {
        sub = `Verbunden${n.detail ? ` · ${n.detail}` : ''}`;
      } else if (connecting) {
        sub = 'Verbinde …';
      }
      return `
        <div class="net ${connected ? 'is-connected' : ''}" data-ssid="${esc(n.ssid)}">
          <button class="row has-state" data-net="${esc(n.ssid)}">
            ${ic(SIGNAL_ICONS[n.signal])}
            <span class="truncate"><span class="row__title">${esc(n.ssid)}</span><span class="row__sub">${sub}</span></span>
            <span class="row__trail">${connecting ? ic('progress_activity', 'spin ic--sm') : n.secure ? ic('lock', 'net__lock') : ''}</span>
          </button>
          ${n.secure && !connected ? `
          <div class="net__form"><div>
            <form class="net__form-inner" data-net-form="${esc(n.ssid)}">
              <label class="field">${ic('lock', 'ic--sm')}<input type="password" placeholder="Netzwerkschlüssel" aria-label="Netzwerkschlüssel für ${esc(n.ssid)}"></label>
              <button class="btn btn--filled" type="submit">Verbinden</button>
            </form>
          </div></div>` : ''}
        </div>`;
    };
    const current = MockData.networks.find((n) => n.ssid === s.network);
    const others = MockData.networks.filter((n) => n !== current);
    this.el.wifiBody.innerHTML = `
      ${current ? `<div class="section-label">Verbunden</div>${net(current, true)}` : ''}
      <div class="section-label">Verfügbare Netzwerke ${this.scanning ? ic('progress_activity', 'spin') : ''}</div>
      ${others.map((n) => net(n, false)).join('')}`;
  },

  /**
   * Starts a (simulated) connection to a network.
   *
   * @param {string} ssid - Network name.
   * @private
   */
  connect(ssid) {
    this.connecting = ssid;
    this.renderWifi();
    setTimeout(() => {
      this.connecting = null;
      this.s.network = ssid;
      this.renderWifi();
      this.updateTiles();
    }, 1400 / Motion.timeScale);
  },

  // ── Bluetooth ───────────────────────────────────────────────────────────

  /** Renders the Bluetooth drill-in page. */
  renderBluetooth() {
    const s = this.s;
    if (!s.bluetooth) {
      this.el.btBody.innerHTML = `
        <div class="detail-empty">${ic('bluetooth_disabled')}<div class="empty__title">Bluetooth ist aus</div><div>Schalte Bluetooth ein, um deine Geräte zu verbinden.</div></div>`;
      return;
    }
    const row = (d) => {
      const busy = this.connecting === d.id;
      let sub = d.connected ? `Verbunden · ${d.kind}` : `Nicht verbunden · ${d.kind}`;
      if (busy) {
        sub = d.connected ? 'Trenne …' : 'Verbinde …';
      }
      let trail = '';
      if (busy) {
        trail = ic('progress_activity', 'spin ic--sm');
      } else if (d.connected && d.battery) {
        trail = `<span class="device__battery num"><span class="battery" style="--level:${d.battery / 100}"><i></i></span>${d.battery} %</span>`;
      }
      return `
        <div class="net ${d.connected ? 'is-connected' : ''}">
          <button class="row has-state" data-device="${d.id}">
            ${ic(d.icon)}
            <span class="truncate"><span class="row__title">${esc(d.name)}</span><span class="row__sub">${sub}</span></span>
            <span class="row__trail">${trail}</span>
          </button>
        </div>`;
    };
    const connected = s.devices.filter((d) => d.connected);
    const paired = s.devices.filter((d) => !d.connected);
    this.el.btBody.innerHTML = `
      ${connected.length ? `<div class="section-label">Verbunden</div>${connected.map(row).join('')}` : ''}
      <div class="section-label">Gekoppelt</div>
      ${paired.map(row).join('')}
      <div class="section-label">Neue Geräte ${this.searching ? ic('progress_activity', 'spin') : ''}</div>
      ${this.searching
        ? '<div class="row"><span></span><span class="row__sub">Suche nach Geräten in der Nähe …</span><span></span></div>'
        : `<button class="row has-state" data-pair>${ic('add')}<span class="row__title">Neues Gerät koppeln</span><span></span></button>`}`;
  },

  /**
   * Toggles a Bluetooth device connection (simulated).
   *
   * @param {string} id - Device id.
   * @private
   */
  toggleDevice(id) {
    const d = this.s.devices.find((x) => x.id === id);
    this.connecting = id;
    this.renderBluetooth();
    setTimeout(() => {
      this.connecting = null;
      d.connected = !d.connected;
      if (d.connected && !d.battery) {
        d.battery = 40 + Math.round(Math.random() * 55);
      }
      this.renderBluetooth();
      this.updateTiles();
    }, 1200 / Motion.timeScale);
  },

  /** Simulated discovery of a new device. @private */
  pair() {
    this.searching = true;
    this.renderBluetooth();
    setTimeout(() => {
      this.searching = false;
      if (!this.s.devices.some((d) => d.id === 'galaxy')) {
        this.s.devices.push({ id: 'galaxy', name: 'Galaxy Buds2 Pro', kind: 'Ohrhörer', icon: 'earbuds', connected: false, battery: 0 });
      }
      this.renderBluetooth();
    }, 2200 / Motion.timeScale);
  },

  // ── Events ──────────────────────────────────────────────────────────────

  /** @private */
  bind() {
    const root = this.root;
    const app = this.app;

    root.addEventListener('click', (e) => {
      const t = e.target;
      const toggle = t.closest('[data-toggle]');
      const open = t.closest('[data-open]');
      const tab = t.closest('[data-tab]');
      if (toggle) {
        app.toggleSetting(toggle.dataset.toggle);
      } else if (open) {
        this.openPage(open.dataset.open);
      } else if (t.closest('[data-back]')) {
        this.openPage('home');
      } else if (t.closest('[data-switch]')) {
        app.toggleSetting(t.closest('[data-switch]').dataset.switch);
      } else if (tab) {
        this.setTab(tab.dataset.tab);
      } else if (t.closest('[data-clear]')) {
        [...this.el.nList.children].forEach((w, i) => setTimeout(() => this.dismiss(w, 1), (i * 45) / Motion.timeScale));
      } else if (t.closest('[data-dismiss]')) {
        this.dismiss(t.closest('.notif-wrap'), 1);
      } else if (t.closest('[data-day]')) {
        this.day = new Date(Number(t.closest('[data-day]').dataset.day));
        this.renderWeek();
        this.renderAgenda();
      } else if (t.closest('[data-filter]')) {
        this.taskFilter = t.closest('[data-filter]').dataset.filter;
        this.renderTaskLists();
        this.renderTasks();
      } else if (t.closest('.check')) {
        this.toggleTask(Number(t.closest('[data-task]').dataset.task));
      } else if (t.closest('[data-sort]')) {
        this.procSort = t.closest('[data-sort]').dataset.sort;
        for (const b of this.el.procSort.children) {
          b.classList.toggle('is-active', b.dataset.sort === this.procSort);
        }
        this.renderProcs();
      } else if (t.closest('[data-kill]')) {
        const row = t.closest('.proc');
        row.classList.add('is-leaving');
        setTimeout(() => {
          this.s.processes = this.s.processes.filter((p) => p.name !== row.dataset.proc);
          row.remove();
        }, 380 / Motion.timeScale);
      } else if (t.closest('[data-net]')) {
        const ssid = t.closest('[data-net]').dataset.net;
        const n = MockData.networks.find((x) => x.ssid === ssid);
        if (ssid !== this.s.network && !this.connecting) {
          if (n.secure) {
            const wrap = t.closest('.net');
            const opening = !wrap.classList.contains('is-open');
            for (const other of this.el.wifiBody.querySelectorAll('.net.is-open')) {
              other.classList.remove('is-open');
            }
            wrap.classList.toggle('is-open', opening);
            if (opening) {
              setTimeout(() => wrap.querySelector('input')?.focus(), 120);
            }
          } else {
            this.connect(ssid);
          }
        }
      } else if (t.closest('[data-device]')) {
        if (!this.connecting) {
          this.toggleDevice(t.closest('[data-device]').dataset.device);
        }
      } else if (t.closest('[data-pair]')) {
        this.pair();
      } else if (t.closest('[data-seek]')) {
        const track = t.closest('[data-seek]').getBoundingClientRect();
        app.seek((e.clientX - track.left) / track.width);
      } else if (t.closest('[data-settings]')) {
        app.toast(`Öffnet ${t.closest('[data-settings]').dataset.settings}`);
      }
    });

    root.addEventListener('submit', (e) => {
      e.preventDefault();
      if (e.target.matches('[data-net-form]')) {
        this.connect(e.target.dataset.netForm);
      } else if (e.target === this.el.taskForm) {
        const title = this.el.taskInput.value.trim();
        if (title) {
          const list = this.taskFilter === 'all' ? 'rice' : this.taskFilter;
          const task = { id: Date.now(), title, list, due: 'today', done: false };
          this.s.tasks.unshift(task);
          this.el.taskInput.value = '';
          this.el.tasksOpen.querySelector('.empty')?.remove();
          this.el.tasksOpen.insertAdjacentHTML('afterbegin', this.taskMarkup(task, true));
        }
      }
    });

    this.el.tabs.addEventListener('keydown', (e) => {
      if (e.key !== 'ArrowRight' && e.key !== 'ArrowLeft') {
        return;
      }
      const ids = TABS.map((t) => t.id);
      const next = ids[(ids.indexOf(this.tab) + (e.key === 'ArrowRight' ? 1 : ids.length - 1)) % ids.length];
      this.setTab(next);
      this.el.tabs.querySelector(`[data-tab="${next}"]`).focus();
    });

    this.bindSlider(root.querySelector('[data-slider="volume"]'), (v) => app.setVolume(v, false));
    this.bindSlider(root.querySelector('[data-slider="brightness"]'), (v) => app.setBrightness(v, false));
    this.bindSwipe();
  },
};
