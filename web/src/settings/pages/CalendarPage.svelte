<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import { useBackend } from '$backend/index';
  import { errorCode } from '$backend/transport';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Banner from '$ui/Banner.svelte';
  import Icon from '$ui/Icon.svelte';
  import Switch from '$ui/Switch.svelte';
  import Setting from '../Setting.svelte';

  const backend = useBackend();
  const cal = backend.calendar;
  const c = $derived(i18n.m.settings.calendar);
  const account = $derived(cal.accounts[0]);

  let url = $state('');
  let user = $state('');
  let password = $state('');
  let busy = $state(false);
  let error = $state<string | null>(null);

  async function connect(e: SubmitEvent): Promise<void> {
    e.preventDefault();
    busy = true;
    error = null;
    try {
      await cal.addAccount({ name: 'Nextcloud', url: url.trim(), user: user.trim(), password });
      password = '';
    } catch (err) {
      error = errorCode(err) === 'auth_failed' ? c.errorAuth : c.errorNetwork;
    } finally {
      busy = false;
    }
  }

  const statusClass = (status: string): string => (status === 'ok' ? '' : status === 'syncing' ? 'is-warn' : 'is-error');
</script>

<div class="settings-page">
  <h1 class="settings-title">{i18n.m.settings.nav.calendar}</h1>

  <section class="settings-card">
    <div class="settings-card__title">{c.account}</div>
    <div class="settings-card__hint">{c.intro}</div>

    {#if account}
      <div class="account-head">
        <span class="status-dot {statusClass(account.status)}" aria-hidden="true"></span>
        <span class="stack">
          <span class="setting__label">{account.user} · {account.url}</span>
          <span class="setting__hint">
            {c.status[account.status]}{account.lastSync ? ` · ${c.lastSync(fmt.ago(new Date(account.lastSync), new Date(clock.now)))}` : ''}
          </span>
        </span>
        <span class="order-row__actions">
          <button class="btn btn--tonal has-state" disabled={account.status === 'syncing'} onclick={() => void cal.sync()}>
            <Icon name="sync" class={account.status === 'syncing' ? 'spin' : ''} />{c.syncNow}
          </button>
          <button class="btn btn--text has-state" onclick={() => void cal.removeAccount(account.id)}>{c.removeAccount}</button>
        </span>
      </div>
      {#if account.status === 'auth_failed'}
        <Banner tone="error" icon="error" title={c.status.auth_failed} body={c.errorAuth} />
      {/if}
    {:else}
      <form class="form-grid" onsubmit={connect}>
        <label class="form-field">
          <span class="form-field__label">{c.server}</span>
          <span class="field"><input bind:value={url} type="url" required placeholder={c.serverPlaceholder} autocomplete="url" /></span>
        </label>
        <label class="form-field">
          <span class="form-field__label">{c.user}</span>
          <span class="field"><input bind:value={user} required autocomplete="username" /></span>
        </label>
        <label class="form-field">
          <span class="form-field__label">{c.password}</span>
          <span class="field"><input bind:value={password} type="password" required autocomplete="current-password" /></span>
          <span class="form-field__hint">{c.passwordHint}</span>
        </label>
        {#if error}<Banner tone="error" icon="error" title={error} />{/if}
        <div class="form-actions">
          <button class="btn btn--filled has-state" type="submit" disabled={busy}>
            {#if busy}<Icon name="progress_activity" class="spin" />{c.connecting}{:else}{c.connect}{/if}
          </button>
        </div>
      </form>
    {/if}
  </section>

  {#if account}
    <section class="settings-card">
      <div class="settings-card__title">{c.calendars} & {c.taskLists}</div>
      {#each account.calendars as calendar (calendar.id)}
        <div class="order-row">
          <span class="cal-swatch" style:--c={calendar.color} aria-hidden="true"></span>
          <span class="stack">
            <span class="setting__label">{calendar.name}</span>
            <span class="setting__hint">{[calendar.supportsEvents ? c.calendars : '', calendar.supportsTasks ? c.taskLists : ''].filter(Boolean).join(' · ')}</span>
          </span>
          <Switch checked={calendar.enabled} label={calendar.name} onchange={(v) => void cal.setEnabled(calendar.id, v)} />
        </div>
      {/each}
    </section>

    <section class="settings-card">
      <Setting label={c.reminders} hint={c.reminderLead}>
        <select
          class="select"
          aria-label={c.reminderLead}
          value={String(backend.settings.value.calendar.reminderLead)}
          onchange={(e) => void backend.settings.patch({ calendar: { reminderLead: Number(e.currentTarget.value) } })}
        >
          {#each [5, 10, 15, 30] as minutes (minutes)}<option value={String(minutes)}>{i18n.m.timers.minutesShort(minutes)}</option>{/each}
        </select>
      </Setting>
    </section>
  {/if}
</div>
