<script lang="ts">
  import { clock } from '$backend/clock.svelte';
  import { fmt, i18n } from '$i18n/i18n.svelte';
  import Battery from '$ui/Battery.svelte';
  import Icon from '$ui/Icon.svelte';
  import { useIsland } from '../context.svelte';

  const { backend, controller } = useIsland();
  const now = $derived(new Date(clock.now));
  const power = $derived(backend.power.state);
  const h = $derived(i18n.m.header);
  let menuOpen = $state(false);

  const actions = ['lock', 'sleep', 'signout', 'restart', 'shutdown'] as const;
  const icons: Record<(typeof actions)[number], string> = {
    lock: 'lock',
    sleep: 'nightlight',
    signout: 'arrow_back',
    restart: 'restart_alt',
    shutdown: 'power_settings_new',
  };

  $effect(() => {
    if (!controller.expanded) {
      menuOpen = false;
    }
  });

  function openSettings(page?: string): void {
    void backend.transport.invoke('window_open_settings', { page });
  }
</script>

<div class="p-time">
  <span class="p-clock num">{fmt.time(now)}</span>
  <span class="p-date">{fmt.longDate(now)}</span>
</div>
<div class="p-status">
  {#if power.battery !== null}
    <span class="p-battery" role="img" aria-label={h.battery(Math.round(power.battery * 100), power.charging)}>
      <Battery level={power.battery} charging={power.charging} />
      <span class="num" aria-hidden="true">{i18n.m.format.percent(power.battery * 100)}</span>
    </span>
  {/if}
  <button class="icon-btn has-state" aria-label={h.editTiles} title={h.editTiles} onclick={() => openSettings('panel')}>
    <Icon name="edit" />
  </button>
  <button class="icon-btn has-state" aria-label={h.settings} title={h.settings} onclick={() => openSettings()}>
    <Icon name="settings" />
  </button>
  <button
    class="icon-btn has-state"
    aria-label={h.power}
    title={h.power}
    aria-haspopup="menu"
    aria-expanded={menuOpen}
    onclick={() => (menuOpen = !menuOpen)}
  >
    <Icon name="power_settings_new" />
  </button>
</div>

{#if menuOpen}
  <div class="power-menu" role="menu">
    {#each actions as action (action)}
      <button class="row has-state" role="menuitem" onclick={() => { menuOpen = false; void backend.power.action(action); }}>
        <Icon name={icons[action]} />
        <span class="row__title">{h.powerMenu[action]}</span>
      </button>
    {/each}
  </div>
{/if}
