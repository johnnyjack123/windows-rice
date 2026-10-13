/* Markup and update functions for the island's compact views. Each view is
   rendered once; updates only touch text and custom properties. */

/**
 * Compact views of the island (clock, music, banners, OSD, ...).
 */
const IslandViews = {
  /** Cached element references, filled by {@link IslandViews.mount}. */
  el: {},

  /**
   * Renders all views into the island's clip container.
   *
   * @param {HTMLElement} clip - The `.island__clip` element.
   */
  mount(clip) {
    clip.innerHTML = `
      <div class="view view--clock" data-view="clock" data-compact inert>
        <span class="island-clock num"><span data-hm>--:--</span><span class="island-clock__sec">:<span data-s>00</span></span></span>
        <span class="island-unread" data-unread hidden></span>
      </div>

      <div class="view view--music" data-view="music" data-compact inert>
        <span class="art art--island" data-art></span>
        <span class="island-track">
          <span class="island-track__title" data-title></span>
          <span class="island-track__artist" data-artist></span>
        </span>
        <span class="eq" data-eq data-playing="true"><i></i><i></i><i></i><i></i><i></i></span>
      </div>

      <div class="view view--music-expanded" data-view="music-expanded" inert>
        <div class="mx-head">
          <span class="art mx-art" data-art></span>
          <div class="truncate">
            <div class="mx-title truncate" data-title></div>
            <div class="mx-artist truncate" data-artist></div>
          </div>
          <span class="eq eq--lg" data-eq data-playing="true"><i></i><i></i><i></i><i></i><i></i></span>
        </div>
        <div class="progress" data-progress>
          <span class="num" data-pos>0:00</span>
          <div class="progress__track"><div class="progress__fill"></div></div>
          <span class="num" data-rem>-0:00</span>
        </div>
        <div class="mx-controls">
          <button class="icon-btn has-state" data-action="prev" aria-label="Vorheriger Titel">${ic('skip_previous')}</button>
          <button class="icon-btn has-state mx-play" data-action="play" aria-label="Wiedergabe umschalten">${ic('pause')}</button>
          <button class="icon-btn has-state" data-action="next" aria-label="Nächster Titel">${ic('skip_next')}</button>
        </div>
      </div>

      <div class="view view--notification" data-view="notification" inert>
        <div class="nv">
          <span class="app-icon" data-app-icon>${ic('chat')}</span>
          <div class="truncate">
            <div class="nv-meta"><span data-app></span><span>jetzt</span></div>
            <div class="nv-title truncate" data-title></div>
            <div class="nv-body truncate" data-body></div>
          </div>
        </div>
      </div>

      <div class="view view--osd" data-view="osd" inert>
        <span class="osd-icon" data-icon>${ic('volume_up')}</span>
        <div class="osd-bar"><div class="osd__fill" data-fill></div></div>
        <span class="osd-value num" data-value>0</span>
      </div>

      <div class="view view--event" data-view="event" data-compact inert>
        <span class="ev-ring" data-ring></span>
        <span data-title></span>
        <span class="ev-when" data-when></span>
      </div>

      <div class="view view--timer" data-view="timer" data-compact inert>
        ${ic('timer')}
        <span class="timer-time num" data-time>00:00</span>
      </div>

      <div class="view view--charging" data-view="charging" data-compact inert>
        <span class="ch-label">${ic('bolt')} Wird geladen</span>
        <span class="ch-right">
          <span class="num" data-pct>0 %</span>
          <span class="battery is-charging" data-battery><i></i></span>
        </span>
      </div>

      <div class="view view--panel" data-view="panel" inert></div>
    `;
    const q = (view, sel) => clip.querySelector(`[data-view="${view}"] ${sel}`);
    const views = (sel) => [...clip.querySelectorAll(`[data-view="music"] ${sel}, [data-view="music-expanded"] ${sel}`)];
    this.el = {
      hm: q('clock', '[data-hm]'),
      s: q('clock', '[data-s]'),
      unread: q('clock', '[data-unread]'),
      arts: views('[data-art]'),
      titles: views('[data-title]'),
      artists: views('[data-artist]'),
      eqs: views('[data-eq]'),
      progress: q('music-expanded', '[data-progress]'),
      pos: q('music-expanded', '[data-pos]'),
      rem: q('music-expanded', '[data-rem]'),
      playIcon: q('music-expanded', '[data-action="play"] .ic'),
      nIcon: q('notification', '[data-app-icon]'),
      nApp: q('notification', '[data-app]'),
      nTitle: q('notification', '[data-title]'),
      nBody: q('notification', '[data-body]'),
      osdIcon: q('osd', '[data-icon]'),
      osdFill: q('osd', '[data-fill]'),
      osdValue: q('osd', '[data-value]'),
      evRing: q('event', '[data-ring]'),
      evTitle: q('event', '[data-title]'),
      evWhen: q('event', '[data-when]'),
      timer: q('timer', '[data-time]'),
      chPct: q('charging', '[data-pct]'),
      chBattery: q('charging', '[data-battery]'),
    };
  },

  /**
   * @param {Date} now - Current time.
   * @param {number} unread - Number of unread notifications.
   */
  setClock(now, unread) {
    this.el.hm.textContent = Fmt.time(now);
    this.el.s.textContent = String(now.getSeconds()).padStart(2, '0');
    this.el.unread.hidden = unread === 0;
  },

  /**
   * @param {object} track - Track from {@link MockData.tracks}.
   * @param {boolean} animate - Play the cover swap animation.
   */
  setTrack(track, animate) {
    for (const art of this.el.arts) {
      art.style.setProperty('--art', Art.covers[track.id]);
      if (animate) {
        art.classList.remove('is-swapping');
        void art.offsetWidth;
        art.classList.add('is-swapping');
      }
    }
    for (const t of this.el.titles) {
      t.textContent = track.title;
    }
    for (const a of this.el.artists) {
      a.textContent = track.artist;
    }
  },

  /** @param {boolean} playing - Playback state. */
  setPlaying(playing) {
    for (const eq of this.el.eqs) {
      eq.dataset.playing = String(playing);
    }
    this.el.playIcon.textContent = playing ? 'pause' : 'play_arrow';
  },

  /**
   * @param {number} position - Seconds played.
   * @param {number} duration - Track length in seconds.
   * @param {boolean} [jump=false] - Disable the smoothing transition.
   */
  setProgress(position, duration, jump = false) {
    this.el.progress.classList.toggle('is-jump', jump);
    this.el.progress.style.setProperty('--p', String(Math.min(1, position / duration)));
    this.el.pos.textContent = Fmt.duration(position);
    this.el.rem.textContent = `-${Fmt.duration(duration - position)}`;
  },

  /** @param {object} n - Notification. */
  setNotification(n) {
    this.el.nIcon.style.setProperty('--app-color', n.color);
    this.el.nIcon.innerHTML = ic(n.icon);
    this.el.nApp.textContent = n.app;
    this.el.nTitle.textContent = n.title;
    this.el.nBody.textContent = n.body;
  },

  /**
   * @param {'volume'|'brightness'} kind - What is being changed.
   * @param {number} value - 0…1.
   * @param {boolean} [muted=false] - Volume muted.
   */
  setOsd(kind, value, muted = false) {
    let icon;
    if (kind === 'volume') {
      if (muted || value === 0) {
        icon = 'volume_off';
      } else if (value < 0.34) {
        icon = 'volume_mute';
      } else if (value < 0.67) {
        icon = 'volume_down';
      } else {
        icon = 'volume_up';
      }
    } else if (value < 0.34) {
      icon = 'brightness_low';
    } else if (value < 0.67) {
      icon = 'brightness_medium';
    } else {
      icon = 'brightness_high';
    }
    this.el.osdIcon.innerHTML = ic(icon);
    this.el.osdFill.style.setProperty('--v', String(muted ? 0 : value));
    this.el.osdValue.textContent = String(Math.round((muted ? 0 : value) * 100));
  },

  /**
   * @param {object} event - Calendar event.
   * @param {number} minutes - Minutes until it starts.
   */
  setEvent(event, minutes) {
    this.el.evRing.style.setProperty('--c', MockData.calendars[event.cal].color);
    this.el.evRing.style.setProperty('--p', String(Math.max(0.04, 1 - minutes / 15)));
    this.el.evTitle.textContent = event.title.length > 24 ? `${event.title.slice(0, 23)}…` : event.title;
    this.el.evWhen.textContent = Fmt.until(minutes);
  },

  /** @param {number} seconds - Remaining timer seconds. */
  setTimer(seconds) {
    this.el.timer.textContent = `${String(Math.floor(seconds / 60)).padStart(2, '0')}:${String(seconds % 60).padStart(2, '0')}`;
  },

  /** @param {number} level - Battery level 0…1. */
  setCharging(level) {
    this.el.chPct.textContent = `${Math.round(level * 100)} %`;
    this.el.chBattery.style.setProperty('--level', String(level));
  },

  /**
   * Markup for the music bubble (split view).
   *
   * @param {object} track - Current track.
   * @returns {string} HTML.
   */
  musicBubble(track) {
    return `<span class="art art--bubble" style="--art: ${Art.covers[track.id]}"></span>`;
  },

  /**
   * Markup for the timer bubble (split view).
   *
   * @param {number} seconds - Remaining seconds.
   * @returns {string} HTML.
   */
  timerBubble(seconds) {
    return `<span class="bubble-timer num">${Math.ceil(seconds / 60)}m</span>`;
  },
};
