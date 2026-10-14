<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import type { CalendarEvent } from '$backend/contract';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Empty from '$ui/Empty.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../../context.svelte';

  const { backend } = useIsland();
  const cal = backend.calendar;
  const c = $derived(i18n.m.calendar);

  const startOfDay = (d: Date): Date => new Date(d.getFullYear(), d.getMonth(), d.getDate());
  let selected = $state(startOfDay(new Date()));

  const today = $derived(startOfDay(new Date(clock.now)));
  const week = $derived.by(() => {
    const monday = new Date(today);
    monday.setDate(today.getDate() - ((today.getDay() + 6) % 7));
    return Array.from({ length: 7 }, (_, i) => new Date(monday.getFullYear(), monday.getMonth(), monday.getDate() + i));
  });
  const events = $derived(cal.eventsOn(selected));
  const offset = $derived(Math.round((selected.getTime() - today.getTime()) / 86400000));
  const dayLabel = $derived(offset === 0 ? c.today : offset === 1 ? c.tomorrow : offset === -1 ? c.yesterday : fmt.longDate(selected));
  const account = $derived(cal.accounts[0]);
  const nextId = $derived(offset === 0 ? events.find((e) => Date.parse(e.start) > clock.now)?.id : undefined);
  /** Index before which the "now" line is drawn (today only). */
  const nowIndex = $derived(offset === 0 ? events.findIndex((e) => Date.parse(e.start) > clock.now) : -2);

  function calendarsOn(day: Date): string[] {
    return [...new Set(cal.eventsOn(day).map((e) => e.calendarId))].slice(0, 3);
  }

  function chip(e: CalendarEvent): string | null {
    const start = Date.parse(e.start);
    const end = Date.parse(e.end);
    if (start <= clock.now && end > clock.now) {
      return c.now;
    }
    return e.id === nextId ? fmt.until(new Date(start), new Date(clock.now)) : null;
  }
</script>

{#if !account}
  <Empty icon="calendar_month" title={c.notConnected} body={c.notConnectedHint}>
    <button class="btn btn--filled has-state" onclick={() => void backend.transport.invoke('window_open_settings', { page: 'calendar' })}>{c.connect}</button>
  </Empty>
{:else}
  <div class="week">
    {#each week as day (day.getTime())}
      <button
        class="day has-state"
        class:is-today={day.getTime() === today.getTime()}
        class:is-selected={day.getTime() === selected.getTime()}
        aria-pressed={day.getTime() === selected.getTime()}
        aria-label={fmt.longDate(day)}
        onclick={() => (selected = day)}
      >
        <span class="day__dow">{fmt.weekday(day)}</span>
        <span class="day__num num">{day.getDate()}</span>
        <span class="day__dots">
          {#each calendarsOn(day) as id (id)}<i style:--c={cal.colorOf(id)}></i>{/each}
        </span>
      </button>
    {/each}
  </div>

  <div class="pane__scroll scroll">
    <div class="agenda">
      <div class="agenda__label">{dayLabel}</div>
      {#if !events.length}
        <Empty icon="event" title={c.noEvents} body={c.freeDay} />
      {:else}
        {#each events as e, i (e.id)}
          {#if i === nowIndex}<div class="now-line num">{fmt.time(new Date(clock.now))}</div>{/if}
          <div class="event-row" class:is-past={Date.parse(e.end) < clock.now} style:--c={cal.colorOf(e.calendarId)}>
            <span class="event-row__bar"></span>
            <span class="event-row__time num">
              {#if e.allDay}<span>{c.allDay}</span>{:else}<span>{fmt.time(new Date(e.start))}</span><span>{fmt.time(new Date(e.end))}</span>{/if}
            </span>
            <span class="stack">
              <span class="event-row__title">{e.title}</span>
              <span class="event-row__meta">{cal.nameOf(e.calendarId)}{e.location ? ` · ${e.location}` : ''}</span>
            </span>
            {#if chip(e)}<span class="chip chip--accent">{chip(e)}</span>{/if}
          </div>
        {/each}
        {#if nowIndex === -1}<div class="now-line num">{fmt.time(new Date(clock.now))}</div>{/if}
      {/if}
    </div>
  </div>

  <div class="pane-foot">
    <Icon name={account.status === 'syncing' ? 'progress_activity' : account.status === 'ok' ? 'sync' : 'cloud_off'} class={account.status === 'syncing' ? 'spin' : ''} />
    <span class="truncate">
      {#if account.status === 'syncing'}
        {c.syncing}
      {:else if account.status !== 'ok'}
        {c.syncFailed}
      {:else}
        {c.footer(account.name, account.calendars.filter((x) => x.supportsEvents && x.enabled).length, account.lastSync ? fmt.ago(new Date(account.lastSync), new Date(clock.now)) : '–')}
      {/if}
    </span>
  </div>
{/if}
