/**
 * Built-in activities and the sources that show or hide them. Extensions
 * follow the same pattern: `registry.define(...)` once, then call
 * `registry.show/hide` from their own data source.
 */
import { untrack } from 'svelte';
import { i18n } from '$i18n/i18n.svelte';
import { PRIORITY, type ActivityRegistry } from './activities.svelte';
import type { IslandContext } from './context.svelte';
import AgentStatusView from './views/AgentStatusView.svelte';
import AgentView from './views/AgentView.svelte';
import ChargingView from './views/ChargingView.svelte';
import EventBubble from './views/EventBubble.svelte';
import EventView from './views/EventView.svelte';
import MusicBubble from './views/MusicBubble.svelte';
import MusicPlayerView from './views/MusicPlayerView.svelte';
import MusicView from './views/MusicView.svelte';
import NotificationView from './views/NotificationView.svelte';
import OsdView from './views/OsdView.svelte';
import PomodoroBubble from './views/PomodoroBubble.svelte';
import PomodoroPhaseView from './views/PomodoroPhaseView.svelte';
import PomodoroPreview from './views/PomodoroPreview.svelte';
import PomodoroView from './views/PomodoroView.svelte';
import RingingView from './views/RingingView.svelte';
import StopwatchBubble from './views/StopwatchBubble.svelte';
import StopwatchPreview from './views/StopwatchPreview.svelte';
import StopwatchView from './views/StopwatchView.svelte';
import TimerBubble from './views/TimerBubble.svelte';
import TimerPreview from './views/TimerPreview.svelte';
import TimerView from './views/TimerView.svelte';

/** Extra classes per activity view (layout family). */
export const VIEW_CLASS: Record<string, string> = {
  alarm: 'view--live',
  timerDone: 'view--live',
  pomodoroDone: 'view--live',
  pomodoroPhase: 'view--live',
  agent: 'view--live',
};

/**
 * Registers every built-in activity.
 *
 * @param registry - Registry of the island.
 * @param ctx - Island context (labels read live data).
 */
export function registerBuiltinActivities(registry: ActivityRegistry, ctx: Pick<IslandContext, 'backend' | 'timers' | 'pomodoro' | 'ui'>): void {
  const m = () => i18n.m;
  registry.define({ id: 'alarm', priority: PRIORITY.alarm, view: RingingView, compact: false, transient: true, critical: true, section: 'timers', label: () => m().island.alarm });
  registry.define({ id: 'timerDone', priority: PRIORITY.timerDone, view: RingingView, compact: false, transient: true, critical: true, section: 'timers', label: () => m().island.timerDone });
  registry.define({
    id: 'pomodoroDone', priority: PRIORITY.pomodoroDone, view: RingingView, compact: false, transient: true, critical: true, section: 'timers',
    label: () => (ctx.pomodoro.state.phase === 'focus' ? m().pomodoro.breakDone : m().pomodoro.focusDone),
  });
  registry.define({
    id: 'pomodoroPhase', priority: PRIORITY.pomodoroPhase, view: PomodoroPhaseView, compact: false, transient: true, section: 'timers',
    label: () => m().island.pomodoro(m().pomodoro.phases[ctx.pomodoro.state.phase]),
  });
  registry.define({
    id: 'pomodoro', priority: PRIORITY.pomodoro, view: PomodoroView, expandedView: PomodoroPreview, bubble: PomodoroBubble, compact: true, section: 'timers',
    label: () => m().island.pomodoro(m().pomodoro.phases[ctx.pomodoro.state.phase]),
  });
  registry.define({
    id: 'notification', priority: PRIORITY.notification, view: NotificationView, compact: false, transient: true, section: 'notifications',
    label: () => (ctx.ui.notification ? `${ctx.ui.notification.appName}: ${ctx.ui.notification.title}` : m().sections.notifications),
    // Like a Windows banner: a click opens the sending app.
    open: async () => {
      const shown = ctx.ui.notification;
      if (!shown) {
        throw new Error('no notification on show');
      }
      await ctx.backend.notifications.open(shown.id);
      registry.hide('notification');
    },
  });
  registry.define({
    id: 'agent', priority: PRIORITY.agent, view: AgentView, compact: false, transient: true,
    label: () => {
      const request = ctx.ui.agentFarewell?.request ?? ctx.backend.agents.pending[0];
      return request ? `${m().agents.request(m().agents.names[request.agent], request.project)}: ${m().agents.asks[request.kind]}` : m().agents.names.claude;
    },
  });
  registry.define({
    id: 'agentStatus', priority: PRIORITY.agentStatus, view: AgentStatusView, compact: true, transient: true,
    label: () => {
      const status = ctx.ui.agentStatus;
      if (!status) {
        return m().agents.names.claude;
      }
      const name = m().agents.names[status.agent];
      return status.state === 'done' ? m().agents.done(name) : m().agents.waiting(name);
    },
  });
  registry.define({ id: 'osd', priority: PRIORITY.osd, view: OsdView, compact: false, transient: true, label: () => (ctx.ui.osd.kind === 'volume' ? m().osd.volume : m().osd.brightness) });
  registry.define({ id: 'charging', priority: PRIORITY.charging, view: ChargingView, compact: true, transient: true, label: () => m().island.charging });
  registry.define({
    id: 'event', priority: PRIORITY.event, view: EventView, bubble: EventBubble, compact: true, transient: true, section: 'calendar',
    label: () => (ctx.ui.event ? m().island.eventSoon(ctx.ui.event.title, '') : m().sections.calendar),
  });
  registry.define({
    id: 'timer', priority: PRIORITY.timer, view: TimerView, expandedView: TimerPreview, bubble: TimerBubble, compact: true, section: 'timers',
    label: () => (ctx.timers.activeCount > 1 ? m().island.timersRunning(ctx.timers.activeCount) : m().island.timerRunning),
  });
  registry.define({
    id: 'stopwatch', priority: PRIORITY.stopwatch, view: StopwatchView, expandedView: StopwatchPreview, bubble: StopwatchBubble, compact: true, section: 'timers',
    label: () => m().island.stopwatch,
  });
  registry.define({
    id: 'music', priority: PRIORITY.music, view: MusicView, expandedView: MusicPlayerView, bubble: MusicBubble, compact: true,
    label: () => {
      const s = ctx.backend.media.current;
      return s ? m().island.nowPlaying(s.title, s.artist) : m().media.region;
    },
  });
}

/**
 * Wires backend events and frontend stores to the built-in activities.
 *
 * @param ctx - Island context.
 * @returns Disconnect function.
 */
export function connectSources(ctx: IslandContext): () => void {
  const { backend, registry, timers, pomodoro, ui, controller } = ctx;
  const offs: Array<() => void> = [];

  offs.push(backend.notifications.posted.on((n) => {
    if (backend.settings.value.notifications.dnd || controller.expanded) {
      return;
    }
    ui.notification = n;
    registry.show('notification', { ttl: 4800, bump: true });
  }));

  offs.push(backend.audio.volumeChanged.on((s) => {
    if (!controller.expanded) {
      ui.osd = { kind: 'volume', value: s.volume, muted: s.muted };
      registry.show('osd', { ttl: 1700 });
    }
  }));

  offs.push(backend.display.brightnessChanged.on((value) => {
    if (!controller.expanded) {
      ui.osd = { kind: 'brightness', value, muted: false };
      registry.show('osd', { ttl: 1700 });
    }
  }));

  offs.push(backend.agents.status.on((status) => {
    if (controller.expanded) {
      return;
    }
    ui.agentStatus = status;
    registry.show('agentStatus', { ttl: 4200, bump: true });
  }));

  offs.push(backend.power.plugged.on(() => registry.show('charging', { ttl: 3200 })));
  offs.push(backend.media.trackChanged.on(() => registry.show('music', { bump: true })));

  const stopEffects = $effect.root(() => {
    $effect(() => {
      const session = backend.media.current;
      const playing = session?.playing ?? false;
      untrack(() => {
        if (!session) {
          registry.hide('music');
        } else {
          registry.show('music', { ttl: playing ? 0 : 12_000 });
        }
      });
    });

    // activeCount, not active: active re-sorts with every 100 ms tick of a running timer.
    $effect(() => {
      const anyTimer = timers.activeCount > 0;
      untrack(() => (anyTimer ? registry.show('timer') : registry.hide('timer')));
    });

    // A paused stopwatch stays until it is reset, so it can be resumed from the island.
    $effect(() => {
      const visible = timers.stopwatch.running || timers.stopwatch.elapsed > 0;
      untrack(() => (visible ? registry.show('stopwatch') : registry.hide('stopwatch')));
    });

    $effect(() => {
      const kind = timers.ringing?.kind;
      untrack(() => {
        if (kind === 'alarm') {
          registry.show('alarm');
        } else {
          registry.hide('alarm');
        }
        if (kind === 'timer') {
          registry.show('timerDone');
        } else {
          registry.hide('timerDone');
        }
        if (kind === 'pomodoro') {
          registry.show('pomodoroDone');
        } else {
          registry.hide('pomodoroDone');
        }
      });
    });

    $effect(() => {
      const active = pomodoro.active;
      untrack(() => (active ? registry.show('pomodoro') : registry.hide('pomodoro')));
    });

    // A coding agent waits for an answer: shown until it is answered here or in
    // the terminal; the next waiting request follows with a bump.
    $effect(() => {
      const shown = ui.agentFarewell?.request.id ?? backend.agents.pending[0]?.id;
      untrack(() => {
        if (!shown) {
          registry.hide('agent');
        } else {
          registry.show('agent', { bump: registry.has('agent') });
        }
      });
    });

    // Automatic phase changes are announced briefly (also over other windows).
    $effect(() => {
      const change = pomodoro.change;
      untrack(() => {
        if (change && !controller.expanded) {
          registry.show('pomodoroPhase', { ttl: 5200, bump: true });
        }
      });
    });
  });

  // Calendar reminders: each upcoming event is announced once.
  const announced = new Set<string>();
  const checkReminders = (): void => {
    const lead = backend.settings.value.calendar.reminderLead * 60_000;
    const now = Date.now();
    const next = backend.calendar.events.find((e) => {
      const start = Date.parse(e.start);
      return !e.allDay && start > now && start - now <= lead && !announced.has(e.id);
    });
    if (next) {
      announced.add(next.id);
      ui.event = next;
      registry.show('event', { ttl: Math.min(Date.parse(next.start) - now + 60_000, 15 * 60_000), bump: true });
    }
  };
  const reminderTimer = setInterval(checkReminders, 20_000);
  offs.push(backend.calendar.refreshed.on(() => checkReminders()));
  checkReminders();

  return () => {
    for (const off of offs) {
      off();
    }
    stopEffects();
    clearInterval(reminderTimer);
  };
}
