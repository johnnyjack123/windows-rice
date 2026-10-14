<script lang="ts">
  import type { BluetoothDevice } from '$backend/contract';
  import { i18n } from '$i18n/i18n.svelte';
  import Battery from '$ui/Battery.svelte';
  import Empty from '$ui/Empty.svelte';
  import Icon from '$ui/Icon.svelte';
  import Switch from '$ui/Switch.svelte';
  import { useIsland } from '../context.svelte';

  const { backend, panel } = useIsland();
  const bt = backend.bluetooth;
  const b = $derived(i18n.m.bluetooth);
  const connected = $derived(bt.state.devices.filter((d) => d.connected));
  const paired = $derived(bt.state.devices.filter((d) => !d.connected));

  const ICONS: Record<BluetoothDevice['kind'], string> = {
    headphones: 'headphones', earbuds: 'earbuds', mouse: 'mouse', keyboard: 'keyboard',
    speaker: 'speaker', phone: 'chat', gamepad: 'videogame_asset', other: 'bluetooth',
  };

  const openSettings = () => void backend.transport.invoke('shell_open', { target: 'ms-settings:bluetooth' });

  function activate(d: BluetoothDevice): void {
    // Devices the backend cannot switch fall back to Windows Settings.
    if (!d.canConnect) {
      openSettings();
    } else {
      void bt.toggle(d.id);
    }
  }

  function status(d: BluetoothDevice): string {
    if (bt.busy.includes(d.id)) {
      return d.connected ? b.disconnecting : b.connecting;
    }
    if (bt.failed.includes(d.id)) {
      return b.failed;
    }
    return d.connected ? b.statusConnected(b.kinds[d.kind]) : b.statusPaired(b.kinds[d.kind]);
  }
</script>

{#snippet device(d: BluetoothDevice)}
  <div class="net" class:is-connected={d.connected}>
    <button class="row has-state" onclick={() => activate(d)}>
      <Icon name={ICONS[d.kind]} />
      <span class="stack">
        <span class="row__title">{d.name}</span>
        <span class="row__sub" class:is-error={bt.failed.includes(d.id)}>{status(d)}</span>
      </span>
      <span class="row__trail">
        {#if bt.busy.includes(d.id)}
          <Icon name="progress_activity" class="spin ic--sm" />
        {:else if !d.canConnect}
          <Icon name="open_in_new" class="ic--sm" />
        {:else if d.connected && d.battery !== null}
          <span class="device__battery num" title={b.battery(Math.round(d.battery * 100))}>
            <Battery level={d.battery} />{Math.round(d.battery * 100)}
          </span>
        {/if}
      </span>
    </button>
  </div>
{/snippet}

<header class="detail-head">
  <button class="icon-btn has-state" aria-label={i18n.m.common.back} onclick={() => (panel.page = 'home')}><Icon name="arrow_back" /></button>
  <span class="detail-title">{b.title}</span>
  <Switch checked={bt.state.enabled} label={b.title} onchange={(v) => void bt.setEnabled(v)} />
</header>

<div class="detail-body scroll">
  {#if !bt.state.enabled}
    <Empty icon="bluetooth_disabled" title={b.off} body={b.offHint} />
  {:else}
    {#if connected.length}
      <div class="section-label">{b.connected}</div>
      {#each connected as d (d.id)}{@render device(d)}{/each}
    {/if}
    {#if paired.length}
      <div class="section-label">{b.paired}</div>
      {#each paired as d (d.id)}{@render device(d)}{/each}
    {/if}
    <button class="row has-state" onclick={openSettings}>
      <Icon name="add" />
      <span class="row__title">{b.pairNew}</span>
      <Icon name="open_in_new" class="ic--sm" />
    </button>
  {/if}
</div>

<footer class="detail-foot">
  <button class="btn btn--text has-state" onclick={openSettings}>{i18n.m.common.moreSettings}<Icon name="open_in_new" /></button>
</footer>
