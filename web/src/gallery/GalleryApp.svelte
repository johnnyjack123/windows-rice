<script lang="ts">
  import { provideBackend, type Backend } from '$backend/index';
  import { COVERS } from '$backend/mock/art';
  import { provideEngine, type ThemeEngine } from '$design/engine.svelte';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import AppIcon from '$ui/AppIcon.svelte';
  import Art from '$ui/Art.svelte';
  import Banner from '$ui/Banner.svelte';
  import Battery from '$ui/Battery.svelte';
  import Check from '$ui/Check.svelte';
  import ColorInput from '$ui/ColorInput.svelte';
  import Empty from '$ui/Empty.svelte';
  import Equalizer from '$ui/Equalizer.svelte';
  import Icon from '$ui/Icon.svelte';
  import Progress from '$ui/Progress.svelte';
  import Range from '$ui/Range.svelte';
  import Segmented from '$ui/Segmented.svelte';
  import Slider from '$ui/Slider.svelte';
  import SmallTile from '$ui/SmallTile.svelte';
  import Spark from '$ui/Spark.svelte';
  import Stepper from '$ui/Stepper.svelte';
  import Switch from '$ui/Switch.svelte';
  import Tabs from '$ui/Tabs.svelte';
  import Tile from '$ui/Tile.svelte';
  import Mascot from '$island/mascot/Mascot.svelte';

  interface Props {
    backend: Backend;
    engine: ThemeEngine;
  }

  const props: Props = $props();
  // svelte-ignore state_referenced_locally
  provideBackend(props.backend);
  // svelte-ignore state_referenced_locally
  provideEngine(props.engine);
  // svelte-ignore state_referenced_locally
  const engine = props.engine;

  const g = $derived(i18n.m.gallery);
  const m = $derived(i18n.m);
  const columns = $derived(engine.themes.flatMap((t) => t.schemes.map((scheme) => ({ theme: t, scheme }))));
  const samples = Array.from({ length: 32 }, (_, i) => 30 + Math.sin(i / 3) * 18 + Math.cos(i / 1.7) * 9);
</script>

<header class="g-head">
  <div>
    <h1>{g.title}</h1>
    <p>{g.intro}</p>
  </div>
  <a href="lab.html">← {g.backToLab}</a>
</header>

<main class="g-matrix">
  {#each columns as { theme, scheme } (`${theme.id}-${scheme}`)}
    <section class="g-col theme-scope" data-theme={theme.id} data-scheme={scheme} aria-label="{theme.name} {scheme}">
      <header>
        <h2>{theme.name}</h2>
        <p>{scheme === 'dark' ? g.dark : g.light}</p>
      </header>

      <div class="g-section">
        <h3>{g.islandCompact}</h3>
        <div class="g-inline">
          <div class="g-island"><div class="view view--clock" data-compact><span class="island-clock num">14:32<span class="island-clock__sec">:07</span></span><span class="island-unread"></span></div></div>
          <div class="g-island"><div class="view view--timer" data-compact><Icon name="hourglass_top" /><span class="timer-time num">4:59</span></div></div>
          <div class="g-island">
            <div class="view view--pomodoro" data-compact>
              <span class="pomo-island"><Icon name="target" /><span class="pomo-island__phase">{m.pomodoro.phases.focus}</span><span class="timer-time num">18:42</span></span>
            </div>
          </div>
          <div class="g-island">
            <div class="view view--pomodoro" data-compact>
              <span class="pomo-island is-break"><Icon name="coffee" /><span class="pomo-island__phase">{m.pomodoro.phases.short}</span><span class="timer-time num">3:10</span></span>
            </div>
          </div>
          <div class="g-island">
            <div class="view view--music" data-compact>
              <Art src={COVERS.nachtfahrt ?? null} class="art--island" />
              <span class="island-track"><span class="island-track__title">Nachtfahrt</span><span class="island-track__artist">Kollektiv Ost</span></span>
              <Equalizer playing />
            </div>
          </div>
          <div class="g-island">
            <div class="view view--agentStatus" data-compact>
              <Mascot still class="mascot--compact" />
              <span class="agent-status"><span>{m.agents.done(m.agents.names.claude)}</span><span class="agent-status__project">rice</span></span>
            </div>
          </div>
        </div>
      </div>

      <div class="g-section">
        <h3>{g.islandLive}</h3>
        <div class="g-stack">
          <div class="g-island g-island--live">
            <div class="view view--notification">
              <div class="nv">
                <AppIcon name="Signal" />
                <div class="truncate">
                  <div class="nv-meta"><span>Signal</span><span>{m.common.justNow}</span></div>
                  <div class="nv-title truncate">Lea Hoffmann</div>
                  <div class="nv-body truncate">Bist du heute Abend dabei?</div>
                </div>
              </div>
            </div>
          </div>
          {#each [null, 'allow'] as decided (decided)}
            <div class="g-island g-island--live">
              <div class="view view--live view--agent">
                <div class="agent-head">
                  <Mascot still class="mascot--live" />
                  <div class="truncate">
                    <div class="nv-meta"><span class="truncate">{m.agents.names.claude} · rice</span><span>{m.agents.more(1)}</span></div>
                    <div class="live-title truncate">{m.agents.asks.command}</div>
                  </div>
                </div>
                <div class="agent-detail">pnpm verify &amp;&amp; cargo clippy --workspace --all-targets -- -D warnings &amp;&amp; cargo test --workspace</div>
                <div class="live-actions agent-actions" class:is-decided={decided !== null}>
                  <button class="btn btn--tonal has-state" disabled={decided !== null}><Icon name="terminal" />{m.agents.terminal}</button>
                  <button class="btn btn--tonal has-state" disabled={decided !== null}><Icon name="block" />{m.agents.deny}</button>
                  <button class="btn btn--filled has-state" class:is-chosen={decided !== null} disabled={decided !== null}><Icon name="check" />{m.agents.allow}</button>
                </div>
              </div>
            </div>
          {/each}
          <div class="g-island"><div class="view view--osd"><span class="osd-icon"><Icon name="volume_down" /></span><div class="osd-bar"><div class="osd__fill" style:--v={0.48}></div></div><span class="osd-value num">48</span></div></div>
          <div class="g-island g-island--live">
            <div class="view view--live view--timer-expanded">
              <div class="qa-head">
                <span class="qa-ring" style:--p={0.62}><Icon name="hourglass_top" class="ic--sm" /></span>
                <span class="stack"><span class="live-sub truncate">{m.timers.defaultName} · {m.timers.moreTimers(1)}</span><span class="qa-time num">3:06</span></span>
              </div>
              <div class="qa-actions">
                <button class="qa-btn has-state"><Icon name="more_time" />{m.timers.addMinute}</button>
                <button class="qa-btn has-state is-hover"><Icon name="pause" />{m.timers.pause}</button>
                <button class="qa-btn has-state"><Icon name="close" />{m.timers.cancel}</button>
              </div>
            </div>
          </div>
          <div class="g-island g-island--live">
            <div class="view view--live view--pomodoro-expanded">
              <div class="qa-head">
                <span class="qa-ring is-break" style:--p={0.35}><Icon name="coffee" class="ic--sm" /></span>
                <span class="stack"><span class="live-sub truncate">{m.pomodoro.phases.short}</span><span class="qa-time num">1:45</span></span>
              </div>
              <span class="pomo-dots"><i class="is-done"></i><i class="is-done"></i><i></i><i></i></span>
              <div class="qa-actions">
                <button class="qa-btn has-state"><Icon name="pause" />{m.pomodoro.pause}</button>
                <button class="qa-btn has-state"><Icon name="skip_next" />{m.pomodoro.skip}</button>
                <button class="qa-btn has-state" disabled><Icon name="stop" />{m.pomodoro.stop}</button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <div class="g-section">
        <h3>{g.tileStates}</h3>
        <div class="g-grid">
          <Tile id="wifi" label={m.tiles.wifi} sub="FRITZ!Box 7590" icon="wifi" on={false} detail />
          <Tile id="wifi" label={m.tiles.wifi} sub="FRITZ!Box 7590" icon="wifi" on detail />
          <Tile id="dnd" label={m.tiles.dnd} sub={m.common.off} icon="do_not_disturb_on" on={false} state="hover" />
          <Tile id="night" label={m.tiles.night} sub={m.tiles.nightOn} icon="nightlight" on state="pressed" />
          <Tile id="bluetooth" label={m.tiles.bluetooth} sub={m.tiles.devicesConnected(2)} icon="bluetooth" on={false} detail state="focus" />
          <Tile id="hotspot" label={m.tiles.hotspot} sub={m.tiles.unsupported} icon="wifi_tethering" on={false} unavailable />
        </div>
        <div class="g-grid g-grid--4" style:margin-top="var(--space-2)">
          <SmallTile id="airplane" label={m.tiles.airplane} icon="airplanemode_active" on={false} />
          <SmallTile id="saver" label={m.tiles.saver} icon="energy_savings_leaf" on />
          <SmallTile id="mic" label={m.tiles.mic} icon="mic" on state="hover" />
          <SmallTile id="hotspot" label={m.tiles.hotspot} icon="wifi_tethering" on={false} unavailable />
        </div>
      </div>

      <div class="g-section">
        <h3>{g.controls}</h3>
        <div class="g-stack">
          <Slider value={0.32} icon="volume_down" label={m.sliders.volume} onchange={() => {}} />
          <Slider value={0.82} icon="brightness_high" label={m.sliders.brightness} onchange={() => {}} />
          <div class="g-inline">
            <Switch checked={false} label={m.common.off} />
            <Switch checked label={m.common.on} />
            <Switch checked focus label={m.common.on} />
            <Switch checked={false} disabled label={m.common.notAvailable} />
          </div>
          <Range value={0.6} min={0} max={1} step={0.05} label={m.settings.editor.textScale} oninput={() => {}} />
          <div class="g-inline">
            <Stepper value={25} min={5} max={120} step={5} label={m.pomodoro.focusMinutes} text={m.timers.minutesShort(25)} onchange={() => {}} />
            <Stepper value={2} min={2} max={8} label={m.pomodoro.sessionsUntilLongBreak} text={m.pomodoro.sessions(2)} onchange={() => {}} />
          </div>
        </div>
      </div>

      <div class="g-section">
        <h3>{g.buttons}</h3>
        <div class="g-stack">
          <div class="g-inline">
            <button class="btn btn--filled has-state">{m.wifi.connect}</button>
            <button class="btn btn--tonal has-state">{m.timers.pause}</button>
            <button class="btn btn--text has-state">{m.notifications.clearAll}</button>
            <button class="btn btn--filled has-state" disabled>{m.timers.start}</button>
          </div>
          <div class="g-inline">
            <button class="icon-btn has-state" aria-label={m.header.settings}><Icon name="settings" /></button>
            <button class="icon-btn has-state is-hover" aria-label={m.header.editTiles}><Icon name="edit" /></button>
            <button class="icon-btn has-state is-focus" aria-label={m.header.power}><Icon name="power_settings_new" /></button>
            <button class="icon-btn has-state p-power has-update" aria-label={m.header.powerUpdate}><Icon name="power_settings_new" /></button>
            <span class="chip">{m.tasks.all}</span>
            <span class="chip is-selected">Rice</span>
            <span class="chip chip--accent">{m.format.until(25)}</span>
          </div>
          <Segmented
            label={m.sections.timers}
            value="timer"
            options={[
              { value: 'timer', label: m.timers.modes.timer },
              { value: 'stopwatch', label: m.timers.modes.stopwatch },
              { value: 'alarm', label: m.timers.modes.alarm },
              { value: 'pomodoro', label: m.timers.modes.pomodoro },
            ]}
            onchange={() => {}}
          />
        </div>
      </div>

      <div class="g-section">
        <h3>{g.tabs}</h3>
        <Tabs
          label={m.sections.label}
          active="calendar"
          onselect={() => {}}
          items={[
            { id: 'notifications', label: m.sections.notifications, icon: 'notifications', badge: 3 },
            { id: 'calendar', label: m.sections.calendar, icon: 'calendar_month' },
            { id: 'tasks', label: m.sections.tasks, icon: 'task_alt' },
            { id: 'timers', label: m.sections.timers, icon: 'timer' },
            { id: 'system', label: m.sections.system, icon: 'monitoring' },
          ]}
        />
      </div>

      <div class="g-section g-card">
        <div class="section-label">{m.wifi.connected}</div>
        <div class="net is-connected"><div class="row"><Icon name="signal_wifi_4_bar" /><span class="stack"><span class="row__title">FRITZ!Box 7590 KL</span><span class="row__sub">{m.wifi.connectedDetail('5 GHz · WPA3')}</span></span><span class="row__trail"><Icon name="lock" class="ic--sm" /></span></div></div>
        <div class="section-label">{m.wifi.available} <Icon name="progress_activity" class="spin" /></div>
        <div class="net"><div class="row has-state is-hover"><Icon name="network_wifi_2_bar" /><span class="stack"><span class="row__title">o2-WLAN61</span><span class="row__sub is-error">{m.wifi.wrongKey}</span></span><span class="row__trail"><Icon name="lock" class="ic--sm" /></span></div></div>
        <div class="net is-connected"><div class="row"><Icon name="headphones" /><span class="stack"><span class="row__title">WH-1000XM5</span><span class="row__sub">{m.bluetooth.statusConnected(m.bluetooth.kinds.headphones)}</span></span><span class="row__trail"><span class="device__battery num"><Battery level={0.78} />78</span></span></div></div>
      </div>

      <div class="g-section">
        <h3>{g.pomodoro}</h3>
        <div class="g-inline g-inline--center">
          <div class="pomo-dial" style:--p={0.64}>
            <span class="pomo-dial__phase"><Icon name="target" class="ic--sm" />{m.pomodoro.phases.focus}</span>
            <span class="pomo-dial__time num">16:02</span>
            <span class="pomo-dots"><i class="is-done"></i><i class="is-current"></i><i></i><i></i></span>
          </div>
        </div>
      </div>

      <div class="g-section">
        <h3>{g.notification}</h3>
        <article class="notif">
          <AppIcon name="Discord" />
          <div class="notif__main">
            <div class="notif__meta"><span>Discord</span><span>{m.format.ago(14)}</span></div>
            <div class="notif__title">#rice-dev · Tom</div>
            <div class="notif__body">Hast du die neue Island schon getestet? Das Aufklappen ist absurd smooth.</div>
          </div>
        </article>
      </div>

      <div class="g-section">
        <h3>{g.calendarTasks}</h3>
        <div class="g-stack">
          <div class="event-row is-past" style:--c="#5b8def"><span class="event-row__bar"></span><span class="event-row__time num"><span>09:30</span><span>09:45</span></span><span class="stack"><span class="event-row__title">Daily Stand-up</span><span class="event-row__meta">Arbeit · Teams</span></span></div>
          <div class="now-line num">14:32</div>
          <div class="event-row" style:--c="#3fb68b"><span class="event-row__bar"></span><span class="event-row__time num"><span>15:00</span><span>15:45</span></span><span class="stack"><span class="event-row__title">Pairing: Tauri-Fenster-Spike</span><span class="event-row__meta">Rice · Discord</span></span><span class="chip chip--accent">{m.format.until(28)}</span></div>
          <ul>
            <li class="task"><Check checked={false} label="Island-Konzept auswählen" /><div class="task__main"><div class="task__title">Island-Konzept auswählen</div><div class="task__meta"><span class="dot" style:--c="#3fb68b"></span>Rice · <span class="due is-today">{m.tasks.due.today}</span></div></div></li>
            <li class="task"><Check checked={false} label="Steuererklärung" /><div class="task__main"><div class="task__title">Steuererklärung abgeben</div><div class="task__meta"><span class="dot" style:--c="#e5774f"></span>Privat · <span class="due is-overdue">{m.tasks.due.overdue}</span></div></div></li>
            <li class="task is-done"><Check checked label="Wallpaper aussuchen" /><div class="task__main"><div class="task__title">Wallpaper aussuchen</div><div class="task__meta"><span class="dot" style:--c="#3fb68b"></span>Rice</div></div></li>
          </ul>
        </div>
      </div>

      <div class="g-section">
        <h3>{g.system}</h3>
        <div class="meters" style:padding="0">
          <div class="meter"><div class="meter__head"><span class="meter__label">{m.system.cpu}</span><span class="meter__value num">34<small>%</small></span></div><Spark values={samples} max={100} /><div class="meter__sub">AMD Ryzen 7 7800X3D · {fmt.num(4.7)} GHz</div></div>
          <div class="meter"><div class="meter__head"><span class="meter__label">{m.system.ram}</span><span class="meter__value num">{fmt.num(11.4)}<small>GB</small></span></div><div class="meter__bar"><div class="meter__fill" style:--p={0.36}></div></div><div class="meter__sub">{m.system.ofTotal(fmt.bytes(32 * 1024 ** 3), 36)}</div></div>
        </div>
      </div>

      <div class="g-section">
        <h3>{g.media}</h3>
        <section class="media" style:min-height="132px">
          <Art src={COVERS.glasshouse ?? null} class="media__backdrop" />
          <div class="media__row">
            <Art src={COVERS.glasshouse ?? null} class="media__art" />
            <div class="truncate"><div class="media__title truncate">Glasshouse</div><div class="media__artist truncate">Mara Lind</div></div>
            <div class="media__controls">
              <button class="icon-btn has-state" aria-label={m.media.previous}><Icon name="skip_previous" /></button>
              <button class="icon-btn has-state media__play" aria-label={m.media.playPause}><Icon name="pause" /></button>
              <button class="icon-btn has-state" aria-label={m.media.next}><Icon name="skip_next" /></button>
            </div>
          </div>
          <Progress position={84} duration={252} label={m.media.seek} />
        </section>
      </div>

      <div class="g-section">
        <h3>{g.fields}</h3>
        <div class="g-stack">
          <label class="field"><Icon name="add" /><input placeholder={m.tasks.newIn('Rice')} aria-label={m.tasks.newLabel} /></label>
          <ColorInput value="#0a84ff" label={m.settings.editor.accent} oninput={() => {}} />
          <Banner tone="warning" icon="location_on" title={m.wifi.permissionTitle} body={m.wifi.permissionHint}>
            <button class="btn btn--tonal has-state">{m.wifi.permissionAction}</button>
          </Banner>
        </div>
      </div>

      <div class="g-section g-card">
        <Empty icon="done_all" title={m.notifications.emptyTitle} body={m.notifications.emptyBody} />
      </div>
    </section>
  {/each}
</main>
