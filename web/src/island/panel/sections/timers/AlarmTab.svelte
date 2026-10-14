<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import type { Alarm } from '$backend/contract';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Empty from '$ui/Empty.svelte';
  import Icon from '$ui/Icon.svelte';
  import Switch from '$ui/Switch.svelte';
  import { useIsland } from '../../../context.svelte';

  const { timers } = useIsland();
  const t = $derived(i18n.m.timers);
  const alarms = $derived([...timers.alarms].sort((a, b) => a.time.localeCompare(b.time)));
  const next = $derived.by(() => {
    void clock.now;
    return timers.nextAlarm;
  });

  let time = $state('07:00');
  let label = $state('');
  let days = $state<number[]>([0, 1, 2, 3, 4]);

  function summary(alarm: Alarm): string {
    const d = [...alarm.days].sort();
    let text: string;
    if (!d.length) {
      text = t.once;
    } else if (d.length === 7) {
      text = t.daily;
    } else if (d.join() === '0,1,2,3,4') {
      text = t.weekdays;
    } else if (d.join() === '5,6') {
      text = t.weekend;
    } else {
      text = d.map((x) => t.weekdayShort[x]).join(', ');
    }
    return alarm.label ? `${alarm.label} · ${text}` : text;
  }

  function toggleDay(day: number): void {
    days = days.includes(day) ? days.filter((x) => x !== day) : [...days, day];
  }

  function add(): void {
    if (!/^\d{2}:\d{2}$/.test(time)) {
      return;
    }
    void timers.addAlarm(time, label.trim(), [...days].sort());
    label = '';
  }
</script>

{#if next}
  <div class="section-label"><Icon name="alarm" />{t.nextAlarm(fmt.until(next.at, new Date(clock.now)))}</div>
{/if}

{#if alarms.length}
  <div class="timer-list">
    {#each alarms as alarm (alarm.id)}
      <div class="alarm-row" class:is-off={!alarm.enabled}>
        <span class="stack">
          <span class="alarm-row__time num">{alarm.time}</span>
          <span class="alarm-row__meta truncate">{summary(alarm)}</span>
        </span>
        <Switch checked={alarm.enabled} label={t.enableAlarm(alarm.time)} onchange={(v) => void timers.updateAlarm(alarm.id, { enabled: v })} />
        <button class="icon-btn has-state" aria-label={t.deleteAlarm} title={t.deleteAlarm} onclick={() => void timers.removeAlarm(alarm.id)}>
          <Icon name="delete" />
        </button>
      </div>
    {/each}
  </div>
{:else}
  <Empty icon="alarm" title={t.noAlarms} body={t.noAlarmsHint} />
{/if}

<div class="composer">
  <div class="section-label"><Icon name="alarm_add" />{t.addAlarm}</div>
  <input class="time-input num" type="time" bind:value={time} aria-label={t.addAlarm} />
  <div class="day-chips" role="group" aria-label={t.daily}>
    {#each t.weekdayShort as name, day (day)}
      <button class="day-chip" aria-pressed={days.includes(day)} onclick={() => toggleDay(day)}>{name}</button>
    {/each}
  </div>
  <label class="field">
    <Icon name="edit" class="ic--sm" />
    <input bind:value={label} placeholder={t.alarmLabel} aria-label={t.alarmLabel} />
  </label>
  <button class="btn btn--filled btn--block has-state" onclick={add}><Icon name="alarm_add" />{t.addAlarm}</button>
</div>
