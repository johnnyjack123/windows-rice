import type {
  AppNotification, CalendarAccount, CalendarEvent, NotificationPermission, ProcessInfo, SystemSample, Task,
} from '../contract';
import { Emitter, type Transport } from '../transport';

/** Mirrored Windows notifications. */
export class NotificationsStore {
  items = $state<AppNotification[]>([]);
  permission = $state<NotificationPermission>('unknown');
  /** Fires for every new notification (banner in the island). */
  readonly posted = new Emitter<AppNotification>();

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    const state = await this.t.invoke('notifications_get');
    this.items = state.items;
    this.permission = state.permission;
    this.t.listen('notifications://posted', (n) => {
      this.items = [n, ...this.items.filter((x) => x.id !== n.id)];
      this.posted.emit(n);
    });
    this.t.listen('notifications://removed', ({ id }) => {
      this.items = this.items.filter((n) => n.id !== id);
    });
    this.t.listen('notifications://state', (s) => {
      this.items = s.items;
      this.permission = s.permission;
    });
  }

  /** @param id - Notification to remove (here and in Windows). */
  dismiss(id: string): Promise<void> {
    this.items = this.items.filter((n) => n.id !== id);
    return this.t.invoke('notifications_dismiss', { id });
  }

  /**
   * Opens the app that sent the notification; the notification then goes
   * away (the backend announces it). Windows cannot reopen the exact target.
   * @param id - Notification to open.
   */
  open(id: string): Promise<void> {
    return this.t.invoke('notifications_open', { id });
  }

  /** Removes all notifications. */
  clear(): Promise<void> {
    this.items = [];
    return this.t.invoke('notifications_clear');
  }

  /** Asks Windows for access (shows the system consent prompt). */
  async requestAccess(): Promise<void> {
    this.permission = await this.t.invoke('notifications_request_access');
  }
}

/** Range of days the calendar store keeps in memory. */
const RANGE_BEFORE_DAYS = 7;
const RANGE_AFTER_DAYS = 21;

/** CalDAV accounts and events around today. */
export class CalendarStore {
  accounts = $state<CalendarAccount[]>([]);
  events = $state<CalendarEvent[]>([]);
  loading = $state(false);
  /** Fires after events were refetched (reminders re-evaluate). */
  readonly refreshed = new Emitter<CalendarEvent[]>();

  /** Calendars of all accounts, enabled or not. */
  readonly calendars = $derived(this.accounts.flatMap((a) => a.calendars));

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads accounts (and events if wanted) and subscribes. */
  async init(withEvents = true): Promise<void> {
    this.accounts = await this.t.invoke('calendar_accounts');
    this.t.listen('calendar://accounts', (a) => {
      this.accounts = a;
    });
    if (withEvents) {
      await this.refetch();
      this.t.listen('calendar://changed', () => void this.refetch());
    }
  }

  /** Refetches the cached range of events. */
  async refetch(): Promise<void> {
    const now = new Date();
    const from = new Date(now.getFullYear(), now.getMonth(), now.getDate() - RANGE_BEFORE_DAYS);
    const to = new Date(now.getFullYear(), now.getMonth(), now.getDate() + RANGE_AFTER_DAYS);
    this.loading = true;
    try {
      this.events = await this.t.invoke('calendar_events', { from: from.toISOString(), to: to.toISOString() });
      this.refreshed.emit(this.events);
    } finally {
      this.loading = false;
    }
  }

  /**
   * Events intersecting a local day, sorted by start.
   *
   * @param day - Any time on that day.
   * @returns Events of the day.
   */
  eventsOn(day: Date): CalendarEvent[] {
    const start = new Date(day.getFullYear(), day.getMonth(), day.getDate()).getTime();
    const end = start + 86400000;
    return this.events.filter((e) => Date.parse(e.start) < end && Date.parse(e.end) > start);
  }

  /**
   * @param calendarId - Calendar id.
   * @returns Its colour, or a neutral grey.
   */
  colorOf(calendarId: string): string {
    return this.calendars.find((c) => c.id === calendarId)?.color ?? '#8e8e93';
  }

  /**
   * @param calendarId - Calendar id.
   * @returns Its display name.
   */
  nameOf(calendarId: string): string {
    return this.calendars.find((c) => c.id === calendarId)?.name ?? '';
  }

  /**
   * Adds the Nextcloud account; rejects with `auth_failed` or `network`.
   *
   * @param input - Account data; the password goes to the credential store.
   * @returns The new account.
   */
  addAccount(input: { name: string; url: string; user: string; password: string }): Promise<CalendarAccount> {
    return this.t.invoke('calendar_add_account', input);
  }

  /** @param id - Account to remove. */
  removeAccount(id: string): Promise<void> {
    return this.t.invoke('calendar_remove_account', { id });
  }

  /**
   * @param calendarId - Calendar to show or hide.
   * @param enabled - Visibility.
   */
  setEnabled(calendarId: string, enabled: boolean): Promise<void> {
    return this.t.invoke('calendar_set_enabled', { calendarId, enabled });
  }

  /** Triggers a sync now. */
  sync(): Promise<void> {
    return this.t.invoke('calendar_sync');
  }
}

/** VTODO tasks of all enabled task lists. */
export class TasksStore {
  list = $state<Task[]>([]);

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    await this.refetch();
    this.t.listen('tasks://changed', () => void this.refetch());
    this.t.listen('calendar://changed', () => void this.refetch());
  }

  private async refetch(): Promise<void> {
    this.list = await this.t.invoke('tasks_list');
  }

  /**
   * @param calendarId - Task list.
   * @param title - Task title.
   * @param due - Due date "YYYY-MM-DD" or null.
   */
  async create(calendarId: string, title: string, due: string | null): Promise<void> {
    const task = await this.t.invoke('tasks_create', { calendarId, title, due });
    this.list = [task, ...this.list.filter((t) => t.id !== task.id)];
  }

  /**
   * @param id - Task id.
   * @param done - Completion state.
   */
  async setDone(id: string, done: boolean): Promise<void> {
    this.list = this.list.map((t) => (t.id === id ? { ...t, done } : t));
    await this.t.invoke('tasks_update', { id, patch: { done } });
  }

  /** @param id - Task to delete. */
  async remove(id: string): Promise<void> {
    this.list = this.list.filter((t) => t.id !== id);
    await this.t.invoke('tasks_delete', { id });
  }
}

/** Samples kept for sparklines. */
const HISTORY = 32;

/** System monitor. Samples flow only while someone is subscribed. */
export class SystemStore {
  latest = $state<SystemSample | null>(null);
  cpu = $state<number[]>(Array(HISTORY).fill(0));
  gpu = $state<number[]>(Array(HISTORY).fill(0));
  down = $state<number[]>(Array(HISTORY).fill(0));
  processes = $state<ProcessInfo[]>([]);

  private subscribed = 0;
  private processTimer: ReturnType<typeof setInterval> | null = null;

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {
    t.listen('system://sample', (s) => {
      this.latest = s;
      this.cpu = [...this.cpu.slice(1), s.cpu.usage];
      this.gpu = [...this.gpu.slice(1), s.gpu?.usage ?? 0];
      this.down = [...this.down.slice(1), s.network.downBytesPerSec];
    });
  }

  /**
   * Starts samples and process polling; call {@link SystemStore.unsubscribe}
   * when the view hides. Reference counted.
   */
  subscribe(): void {
    this.subscribed += 1;
    if (this.subscribed === 1) {
      void this.t.invoke('system_subscribe', { intervalMs: 1000 });
      void this.refreshProcesses();
      this.processTimer = setInterval(() => void this.refreshProcesses(), 2000);
    }
  }

  /** Stops samples when the last subscriber leaves. */
  unsubscribe(): void {
    this.subscribed = Math.max(0, this.subscribed - 1);
    if (this.subscribed === 0) {
      void this.t.invoke('system_unsubscribe');
      if (this.processTimer) {
        clearInterval(this.processTimer);
        this.processTimer = null;
      }
    }
  }

  private async refreshProcesses(): Promise<void> {
    this.processes = await this.t.invoke('system_processes', { limit: 12 });
  }

  /**
   * Ends a process; rejects with `access_denied` for elevated processes.
   *
   * @param pid - Process id.
   */
  async kill(pid: number): Promise<void> {
    await this.t.invoke('system_kill', { pid });
    this.processes = this.processes.filter((p) => p.pid !== pid);
  }
}
