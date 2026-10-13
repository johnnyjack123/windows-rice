/* Component gallery ("storybook"): renders every primitive in every state
   for every theme and scheme side by side. */

/**
 * Markup builders for gallery specimens. Kept close to the markup used in
 * island-views.js and panel.js so the gallery shows the real components.
 */
const Specimen = {
  /**
   * @param {object} o - Options.
   * @returns {string} Large tile markup.
   */
  tile({ on = false, state = '', label = 'WLAN', sub = 'FRITZ!Box 7590 KL', icon = 'wifi', detail = true, tile = 'wifi', disabled = false }) {
    const dis = disabled ? 'disabled' : '';
    return `
      <div class="tile ${on ? 'is-on' : ''} ${state === 'pressed' ? 'is-pressed' : ''}" data-tile="${tile}">
        <button class="tile__toggle has-state ${disabled ? 'is-disabled' : ''}" ${dis} aria-pressed="${on}" aria-label="${label} umschalten">${ic(icon)}</button>
        <button class="tile__main has-state ${state === 'hover' ? 'is-hover' : ''} ${state === 'focus' ? 'is-focus' : ''} ${disabled ? 'is-disabled' : ''}" ${dis}>
          <span class="truncate"><span class="tile__label">${label}</span><span class="tile__sub">${sub}</span></span>
          ${detail ? ic('chevron_right', 'tile__chev') : ''}
        </button>
      </div>`;
  },

  /**
   * @param {string} icon - Icon name.
   * @param {string} tile - Tile id (for per-tile tints).
   * @param {string} [mods=''] - State classes.
   * @returns {string} Small tile markup.
   */
  smallTile(icon, tile, mods = '') {
    return `<button class="tile tile--small has-state ${mods}" data-tile="${tile}" ${mods.includes('is-disabled') ? 'disabled' : ''} aria-label="${tile}">${ic(icon)}</button>`;
  },

  /**
   * @param {string} icon - Icon name.
   * @param {number} value - 0…1.
   * @returns {string} Slider markup.
   */
  slider(icon, value) {
    return `
      <div class="slider" role="slider" tabindex="0" aria-valuenow="${Math.round(value * 100)}" style="--value:${value}">
        <div class="slider__track"><div class="slider__fill"></div></div>
        <span class="slider__icon">${ic(icon)}</span>
        <span class="slider__handle"></span>
      </div>`;
  },

  /**
   * @param {string} inner - View markup.
   * @param {boolean} [live=false] - Tall live activity.
   * @returns {string} Static island preview.
   */
  island(inner, live = false) {
    return `<div class="g-island ${live ? 'g-island--live' : ''}">${inner}</div>`;
  },
};

/**
 * Renders one gallery column.
 *
 * @param {object} theme - Theme meta.
 * @param {string} scheme - Colour scheme.
 * @returns {string} Column markup.
 */
function renderColumn(theme, scheme) {
  const track = MockData.tracks[0];
  const art = Art.covers[track.id];
  const eq = '<span class="eq" data-playing="true"><i></i><i></i><i></i><i></i><i></i></span>';
  const n = MockData.notifications[0];
  const cal = MockData.calendars.work;

  return `
    <section class="g-col" data-theme="${theme.id}" data-scheme="${scheme}" aria-label="${theme.name} ${scheme}">
      <header>
        <h2>${theme.name}</h2>
        <p>${scheme === 'dark' ? 'Dunkel' : 'Hell'} · ${theme.tagline}</p>
      </header>

      <div class="g-section">
        <h3>Island · kompakt</h3>
        <div class="g-inline">
          ${Specimen.island(`<div class="view view--clock" data-compact><span class="island-clock num">14:32<span class="island-clock__sec">:07</span></span><span class="island-unread"></span></div>`)}
          ${Specimen.island(`<div class="view view--timer" data-compact>${ic('timer')}<span class="timer-time num">04:59</span></div>`)}
          ${Specimen.island(`<div class="view view--music" data-compact><span class="art art--island" style="--art:${art}"></span><span class="island-track"><span class="island-track__title">${track.title}</span><span class="island-track__artist">${track.artist}</span></span>${eq}</div>`)}
          ${Specimen.island(`<div class="view view--charging" data-compact><span class="ch-label">${ic('bolt')} Wird geladen</span><span class="ch-right"><span class="num">82 %</span><span class="battery is-charging" style="--level:.82"><i></i></span></span></div>`)}
        </div>
      </div>

      <div class="g-section">
        <h3>Island · Live-Aktivitäten</h3>
        <div class="g-stack">
          ${Specimen.island(`<div class="view view--notification"><div class="nv"><span class="app-icon" style="--app-color:${n.color}">${ic(n.icon)}</span><div class="truncate"><div class="nv-meta"><span>${n.app}</span><span>jetzt</span></div><div class="nv-title truncate">${n.title}</div><div class="nv-body truncate">${n.body}</div></div></div></div>`, true)}
          ${Specimen.island(`<div class="view view--osd"><span class="osd-icon">${ic('volume_down')}</span><div class="osd-bar"><div class="osd__fill" style="--v:.48"></div></div><span class="osd-value num">48</span></div>`)}
        </div>
      </div>

      <div class="g-section">
        <h3>Tiles · Aus · An · Hover · Gedrückt · Fokus · Deaktiviert</h3>
        <div class="g-grid">
          ${Specimen.tile({})}
          ${Specimen.tile({ on: true })}
          ${Specimen.tile({ state: 'hover', label: 'Nicht stören', sub: 'Aus', icon: 'do_not_disturb_on', detail: false, tile: 'dnd' })}
          ${Specimen.tile({ on: true, state: 'pressed', label: 'Nachtlicht', sub: 'An · bis 07:00', icon: 'nightlight', detail: false, tile: 'night' })}
          ${Specimen.tile({ state: 'focus', label: 'Bluetooth', sub: '2 Geräte verbunden', icon: 'bluetooth', tile: 'bluetooth' })}
          ${Specimen.tile({ disabled: true, label: 'Mobiler Hotspot', sub: 'Nicht verfügbar', icon: 'wifi_tethering', detail: false, tile: 'hotspot' })}
        </div>
        <div class="g-grid g-grid--4" style="margin-top:var(--space-2)">
          ${Specimen.smallTile('airplanemode_active', 'airplane')}
          ${Specimen.smallTile('energy_savings_leaf', 'saver', 'is-on')}
          ${Specimen.smallTile('mic', 'mic', 'is-on is-hover')}
          ${Specimen.smallTile('wifi_tethering', 'hotspot', 'is-disabled')}
        </div>
      </div>

      <div class="g-section">
        <h3>Regler & Schalter</h3>
        <div class="g-stack">
          ${Specimen.slider('volume_down', 0.32)}
          ${Specimen.slider('brightness_high', 0.82)}
          <div class="g-inline">
            <button class="switch" role="switch" aria-checked="false" aria-label="Aus"></button>
            <button class="switch" role="switch" aria-checked="true" aria-label="An"></button>
            <button class="switch is-focus" role="switch" aria-checked="true" aria-label="Fokus"></button>
            <span class="g-caption">Aus · An · Fokus</span>
          </div>
        </div>
      </div>

      <div class="g-section">
        <h3>Buttons & Chips</h3>
        <div class="g-stack">
          <div class="g-inline">
            <button class="btn btn--filled has-state">Verbinden</button>
            <button class="btn btn--tonal has-state">Später</button>
            <button class="btn btn--text has-state">Alle löschen</button>
            <button class="btn btn--filled has-state is-pressed">Gedrückt</button>
          </div>
          <div class="g-inline">
            <button class="icon-btn has-state" aria-label="Einstellungen">${ic('settings')}</button>
            <button class="icon-btn has-state is-hover" aria-label="Bearbeiten">${ic('edit')}</button>
            <button class="icon-btn has-state is-focus" aria-label="Energie">${ic('power_settings_new')}</button>
            <span class="chip has-state">Alle</span>
            <span class="chip is-selected">Rice</span>
            <span class="chip chip--accent">in 25 Min</span>
          </div>
        </div>
      </div>

      <div class="g-section">
        <h3>Tabs</h3>
        <div class="tabs" role="tablist">
          ${TABS.map((t, i) => `<button class="tab has-state ${i === 1 ? 'is-active' : ''}" role="tab">${ic(t.icon)}<span class="tab__label">${t.label}</span>${i === 0 ? '<span class="tab__badge num">3</span>' : ''}</button>`).join('')}
        </div>
      </div>

      <div class="g-section g-card">
        <div class="section-label">Verbunden</div>
        <div class="net is-connected"><div class="row">${ic('signal_wifi_4_bar')}<span class="truncate"><span class="row__title">FRITZ!Box 7590 KL</span><span class="row__sub">Verbunden · 5 GHz · WPA3</span></span><span class="row__trail">${ic('lock', 'net__lock')}</span></div></div>
        <div class="section-label">Verfügbar ${ic('progress_activity', 'spin')}</div>
        <div class="net"><div class="row has-state is-hover">${ic('network_wifi_2_bar')}<span class="truncate"><span class="row__title">o2-WLAN61</span><span class="row__sub">Gesichert</span></span><span class="row__trail">${ic('lock', 'net__lock')}</span></div></div>
        <div class="net is-connected"><div class="row">${ic('headphones')}<span class="truncate"><span class="row__title">WH-1000XM5</span><span class="row__sub">Verbunden · Kopfhörer</span></span><span class="row__trail"><span class="device__battery num"><span class="battery" style="--level:.78"><i></i></span>78 %</span></span></div></div>
      </div>

      <div class="g-section">
        <h3>Mitteilung</h3>
        <article class="notif">
          <span class="app-icon" style="--app-color:${n.color}">${ic(n.icon)}</span>
          <div class="notif__main">
            <div class="notif__meta"><span>${n.app}</span><span>vor 2 Min</span></div>
            <div class="notif__title">${n.title}</div>
            <div class="notif__body">${n.body}</div>
            <div class="notif__actions">${n.actions.map((a) => `<span class="chip">${a}</span>`).join('')}</div>
          </div>
        </article>
      </div>

      <div class="g-section">
        <h3>Kalender & Aufgaben</h3>
        <div class="g-stack">
          <div class="event-row is-past" style="--c:${cal.color}"><span class="event-row__bar"></span><span class="event-row__time num"><span>09:30</span><span>09:45</span></span><span class="truncate"><span class="event-row__title">Daily Stand-up</span><span class="event-row__meta">Arbeit · Teams</span></span></div>
          <div class="now-line num">14:32</div>
          <div class="event-row" style="--c:${MockData.calendars.rice.color}"><span class="event-row__bar"></span><span class="event-row__time num"><span>15:00</span><span>15:45</span></span><span class="truncate"><span class="event-row__title">Pairing: Tauri-Fenster-Spike</span><span class="event-row__meta">Rice · Discord</span></span><span class="chip chip--accent">in 28 Min</span></div>
          <ul class="tasks">
            <li class="task"><span class="check">${CHECK_SVG}</span><div class="task__main"><div class="task__title">Island-Konzept auswählen</div><div class="task__meta"><span class="dot" style="--c:#3fb68b"></span>Rice · <span class="due is-today">Heute</span></div></div></li>
            <li class="task"><span class="check">${CHECK_SVG}</span><div class="task__main"><div class="task__title">Steuererklärung abgeben</div><div class="task__meta"><span class="dot" style="--c:#e5774f"></span>Privat · <span class="due is-overdue">Gestern</span></div></div></li>
            <li class="task is-done"><span class="check">${CHECK_SVG}</span><div class="task__main"><div class="task__title">Wallpaper aussuchen</div><div class="task__meta"><span class="dot" style="--c:#3fb68b"></span>Rice</div></div></li>
          </ul>
          <label class="field">${ic('add')}<input placeholder="Neue Aufgabe in „Rice“ …" aria-label="Neue Aufgabe"></label>
        </div>
      </div>

      <div class="g-section">
        <h3>System</h3>
        <div class="meters" style="padding:0">
          <div class="meter"><div class="meter__head"><span class="meter__label">CPU</span><span class="meter__value num">34<small>%</small></span></div><svg class="spark" viewBox="0 0 100 28" preserveAspectRatio="none" data-spark><path class="area"/><path class="line"/></svg><div class="meter__sub">8 Kerne · 4,7 GHz</div></div>
          <div class="meter"><div class="meter__head"><span class="meter__label">RAM</span><span class="meter__value num">11,4<small>GB</small></span></div><div class="meter__bar"><div class="meter__fill" style="--p:.36"></div></div><div class="meter__sub">von 32 GB · 36 %</div></div>
        </div>
      </div>

      <div class="g-section">
        <h3>Media</h3>
        <section class="media reseed" style="min-height:132px">
          <span class="art media__backdrop" style="--art:${art}"></span>
          <div class="media__row">
            <span class="art media__art" style="--art:${art}"></span>
            <div class="truncate"><div class="media__title truncate">${track.title}</div><div class="media__artist truncate">${track.artist}</div></div>
            <div class="media__controls">
              <button class="icon-btn has-state" aria-label="Zurück">${ic('skip_previous')}</button>
              <button class="icon-btn has-state media__play" aria-label="Pause">${ic('pause')}</button>
              <button class="icon-btn has-state" aria-label="Weiter">${ic('skip_next')}</button>
            </div>
          </div>
          <div class="progress" style="--p:.38"><span class="num">1:24</span><div class="progress__track"><div class="progress__fill"></div></div><span class="num">-2:17</span></div>
        </section>
      </div>

      <div class="g-section g-card">
        <div class="empty">${ic('done_all')}<div class="empty__title">Alles erledigt</div><div>Keine neuen Mitteilungen</div></div>
      </div>
    </section>`;
}

/** Builds the gallery and draws the sparklines in each theme's style. */
function initGallery() {
  loadIconFonts();
  const root = document.documentElement;
  root.style.setProperty('--grain', Art.grain);
  root.style.setProperty('--media-color', MockData.tracks[0].color);
  root.style.setProperty('--wallpaper-src', `url("${Art.svgUrl(Art.wallpapers[0].svg)}")`);
  root.style.setProperty('--wallpaper-seed', Art.wallpapers[0].seed);
  ThemeEngine.refreshMotion();

  const matrix = document.querySelector('.g-matrix');
  matrix.innerHTML = THEMES.flatMap((t) => t.schemes.map((s) => renderColumn(t, s))).join('');

  const samples = randomWalk(32, 30, 5, 80, 12);
  for (const svg of matrix.querySelectorAll('[data-spark]')) {
    const style = getComputedStyle(svg).getPropertyValue('--spark-style').trim() || 'smooth';
    const { line, area } = Panel.sparkPath.call({ sparkStyle: style }, samples, 100);
    svg.querySelector('.line').setAttribute('d', line);
    svg.querySelector('.area').setAttribute('d', area);
  }
}
