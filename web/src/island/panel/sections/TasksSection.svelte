<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import type { Task } from '$backend/contract';
  import { i18n } from '$i18n/i18n.svelte';
  import Check from '$ui/Check.svelte';
  import Empty from '$ui/Empty.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../../context.svelte';

  const { backend } = useIsland();
  const tasks = backend.tasks;
  const cal = backend.calendar;
  const t = $derived(i18n.m.tasks);

  const lists = $derived(cal.calendars.filter((c) => c.supportsTasks && c.enabled));
  let filter = $state('all');
  let draft = $state('');
  /** Tasks checked a moment ago stay in place briefly so the tick is visible. */
  let settling = $state<string[]>([]);
  const created = new Set<string>();

  const visible = $derived(tasks.list.filter((x) => filter === 'all' || x.calendarId === filter));
  const open = $derived(visible.filter((x) => !x.done || settling.includes(x.id)));
  const done = $derived(visible.filter((x) => x.done && !settling.includes(x.id)));
  const target = $derived(filter === 'all' ? lists[0] : lists.find((l) => l.id === filter));

  function toggle(task: Task): void {
    const next = !task.done;
    if (next) {
      settling = [...settling, task.id];
      setTimeout(() => (settling = settling.filter((id) => id !== task.id)), 900);
    }
    void tasks.setDone(task.id, next);
  }

  async function add(e: SubmitEvent): Promise<void> {
    e.preventDefault();
    const title = draft.trim();
    if (!title || !target) {
      return;
    }
    draft = '';
    const d = new Date(clock.now);
    const due = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
    await tasks.create(target.id, title, due);
    const newest = tasks.list[0];
    if (newest) {
      created.add(newest.id);
    }
  }

  function due(task: Task): { text: string; cls: string } | null {
    if (!task.due || task.done) {
      return null;
    }
    const now = new Date(clock.now);
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    const day = Date.parse(task.due.length === 10 ? `${task.due}T00:00:00` : task.due);
    const diff = Math.floor((day - today) / 86400000);
    if (diff < 0) {
      return { text: t.due.overdue, cls: 'is-overdue' };
    }
    if (diff === 0) {
      return { text: t.due.today, cls: 'is-today' };
    }
    if (diff === 1) {
      return { text: t.due.tomorrow, cls: '' };
    }
    return { text: new Date(day).toLocaleDateString(i18n.tag, { weekday: 'short', day: 'numeric', month: 'numeric' }), cls: '' };
  }
</script>

{#snippet row(task: Task)}
  <li class="task has-state" class:is-done={task.done} class:is-new={created.has(task.id)}>
    <Check checked={task.done} label={t.markDone(task.title)} onchange={() => toggle(task)} />
    <div class="task__main">
      <div class="task__title">{task.title}</div>
      <div class="task__meta">
        <span class="dot" style:--c={cal.colorOf(task.calendarId)}></span>{cal.nameOf(task.calendarId)}
        {#if due(task)}<span aria-hidden="true">·</span><span class="due {due(task)?.cls}">{due(task)?.text}</span>{/if}
      </div>
    </div>
  </li>
{/snippet}

{#if !cal.accounts.length}
  <Empty icon="task_alt" title={i18n.m.calendar.notConnected} body={i18n.m.calendar.notConnectedHint}>
    <button class="btn btn--filled has-state" onclick={() => void backend.transport.invoke('window_open_settings', { page: 'calendar' })}>{i18n.m.calendar.connect}</button>
  </Empty>
{:else}
  <div class="task-lists" role="group" aria-label={i18n.m.sections.tasks}>
    <button class="chip has-state" class:is-selected={filter === 'all'} aria-pressed={filter === 'all'} onclick={() => (filter = 'all')}>{t.all}</button>
    {#each lists as list (list.id)}
      <button class="chip has-state" class:is-selected={filter === list.id} aria-pressed={filter === list.id} onclick={() => (filter = list.id)}>{list.name}</button>
    {/each}
  </div>

  <form class="field task-add" onsubmit={add}>
    <Icon name="add" />
    <input bind:value={draft} placeholder={t.newIn(target?.name ?? '')} aria-label={t.newLabel} autocomplete="off" />
  </form>

  <div class="pane__scroll scroll">
    {#if open.length}
      <ul>{#each open as task (task.id)}{@render row(task)}{/each}</ul>
    {:else}
      <Empty icon="task_alt" title={t.nothingOpen} body={t.nothingOpenHint} />
    {/if}
    {#if done.length}
      <div class="section-label">{t.done(done.length)}</div>
      <ul>{#each done as task (task.id)}{@render row(task)}{/each}</ul>
    {/if}
  </div>

  <div class="pane-foot"><Icon name="sync" /><span>{t.footer}</span></div>
{/if}
