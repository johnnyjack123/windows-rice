/**
 * Panel sections (tabs of the right-hand card). Extension point: register a
 * component with a label and icon; order and visibility come from the
 * settings (panel.sections / panel.hiddenSections).
 */
import type { Component } from 'svelte';
import type { Backend } from '$backend/index';
import { i18n } from '$i18n/i18n.svelte';
import type { TimersStore } from '../timers.svelte';
import CalendarSection from './sections/CalendarSection.svelte';
import NotificationsSection from './sections/NotificationsSection.svelte';
import SystemSection from './sections/SystemSection.svelte';
import TasksSection from './sections/TasksSection.svelte';
import TimersSection from './sections/TimersSection.svelte';

/** Definition of a panel section. */
export interface SectionDef {
  id: string;
  label: () => string;
  icon: string;
  /** Number shown on the tab, 0 hides it. */
  badge?: () => number;
  component: Component;
}

/**
 * Built-in sections, keyed by id.
 *
 * @param b - Backend stores.
 * @param timers - Timers store (badge).
 * @returns Section definitions.
 */
export function builtinSections(b: Backend, timers: TimersStore): Record<string, SectionDef> {
  const m = () => i18n.m.sections;
  const list: SectionDef[] = [
    { id: 'notifications', label: () => m().notifications, icon: 'notifications', badge: () => b.notifications.items.length, component: NotificationsSection },
    { id: 'calendar', label: () => m().calendar, icon: 'calendar_month', component: CalendarSection },
    { id: 'tasks', label: () => m().tasks, icon: 'task_alt', badge: () => b.tasks.list.filter((t) => !t.done && isDueOrOverdue(t.due)).length, component: TasksSection },
    { id: 'timers', label: () => m().timers, icon: 'timer', badge: () => timers.activeCount, component: TimersSection },
    { id: 'system', label: () => m().system, icon: 'monitoring', component: SystemSection },
  ];
  return Object.fromEntries(list.map((s) => [s.id, s]));
}

/**
 * @param due - "YYYY-MM-DD" or date-time, or null.
 * @returns Whether the task is due today or overdue.
 */
export function isDueOrOverdue(due: string | null): boolean {
  if (!due) {
    return false;
  }
  const today = new Date();
  const end = new Date(today.getFullYear(), today.getMonth(), today.getDate() + 1).getTime();
  return Date.parse(due.length === 10 ? `${due}T00:00:00` : due) < end;
}
