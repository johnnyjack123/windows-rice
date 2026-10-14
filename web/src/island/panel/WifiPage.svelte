<script lang="ts">
  import type { WifiNetwork } from '$backend/contract';
  import { i18n } from '$i18n/i18n.svelte';
  import Banner from '$ui/Banner.svelte';
  import Empty from '$ui/Empty.svelte';
  import Icon from '$ui/Icon.svelte';
  import Switch from '$ui/Switch.svelte';
  import { useIsland } from '../context.svelte';

  const { backend, panel, controller } = useIsland();
  const net = backend.network;
  const wifi = $derived(net.state.wifi);
  const w = $derived(i18n.m.wifi);
  const SIGNAL = ['network_wifi_1_bar', 'network_wifi_1_bar', 'network_wifi_2_bar', 'network_wifi_3_bar', 'signal_wifi_4_bar'];

  let open = $state<string | null>(null);
  let keys = $state<Record<string, string>>({});

  // Scan whenever the page becomes visible.
  $effect(() => {
    if (panel.page === 'wifi' && controller.expanded && wifi.enabled && wifi.permission === 'granted') {
      void net.scan();
    }
  });

  function select(n: WifiNetwork): void {
    if (net.connecting) {
      return;
    }
    if (n.secure && !n.known) {
      open = open === n.ssid ? null : n.ssid;
    } else {
      void net.connect(n.ssid, null);
    }
  }

  async function submit(e: SubmitEvent, ssid: string): Promise<void> {
    e.preventDefault();
    const result = await net.connect(ssid, keys[ssid] ?? '');
    if (result.ok) {
      open = null;
      keys = { ...keys, [ssid]: '' };
    }
  }

  function status(n: WifiNetwork): string {
    if (net.connecting === n.ssid) {
      return w.connecting;
    }
    const error = net.errors[n.ssid];
    if (error) {
      return error === 'wrong_key' ? w.wrongKey : w.failed;
    }
    return n.secure ? w.secured : w.open;
  }

  const openSettings = (target: string) => void backend.transport.invoke('shell_open', { target });
</script>

<header class="detail-head">
  <button class="icon-btn has-state" aria-label={i18n.m.common.back} onclick={() => (panel.page = 'home')}><Icon name="arrow_back" /></button>
  <span class="detail-title">{w.title}</span>
  <Switch checked={wifi.enabled} label={w.title} onchange={(v) => void net.setWifi(v)} />
</header>

<div class="detail-body scroll">
  {#if !wifi.enabled}
    <Empty icon="wifi_off" title={w.off} body={w.offHint} />
  {:else if wifi.permission !== 'granted'}
    <Banner tone="warning" icon="location_on" title={w.permissionTitle} body={w.permissionHint}>
      <button class="btn btn--tonal has-state" onclick={() => openSettings('ms-settings:privacy-location')}>{w.permissionAction}</button>
    </Banner>
  {:else}
    {#if wifi.connected}
      <div class="section-label">{w.connected}</div>
      <div class="net is-connected">
        <div class="row">
          <Icon name={SIGNAL[wifi.connected.signal] ?? 'wifi'} />
          <span class="stack">
            <span class="row__title">{wifi.connected.ssid}</span>
            <span class="row__sub">{w.connectedDetail(wifi.connected.detail)}</span>
          </span>
          <button class="btn btn--text has-state" onclick={() => void backend.transport.invoke('network_disconnect')}>{w.disconnect}</button>
        </div>
      </div>
    {/if}
    <div class="section-label">
      {w.available}
      {#if net.scanning}<Icon name="progress_activity" class="spin" /><span class="sr-only">{w.scanning}</span>{/if}
    </div>
    {#each net.networks.filter((n) => n.ssid !== wifi.connected?.ssid) as n (n.ssid)}
      <div class="net" class:is-open={open === n.ssid}>
        <button class="row has-state" onclick={() => select(n)}>
          <Icon name={SIGNAL[n.signal] ?? 'wifi'} />
          <span class="stack">
            <span class="row__title">{n.ssid}</span>
            <span class="row__sub" class:is-error={Boolean(net.errors[n.ssid])}>{status(n)}</span>
          </span>
          <span class="row__trail">
            {#if net.connecting === n.ssid}
              <Icon name="progress_activity" class="spin ic--sm" />
            {:else if n.secure}
              <Icon name="lock" class="ic--sm" />
            {/if}
          </span>
        </button>
        {#if n.secure && !n.known}
          <div class="net__form">
            <div>
              <form class="net__form-inner" onsubmit={(e) => submit(e, n.ssid)}>
                <label class="field" class:is-error={net.errors[n.ssid] === 'wrong_key'}>
                  <Icon name="lock" class="ic--sm" />
                  <input
                    type="password"
                    placeholder={w.key}
                    aria-label={w.keyFor(n.ssid)}
                    autocomplete="off"
                    value={keys[n.ssid] ?? ''}
                    oninput={(e) => (keys = { ...keys, [n.ssid]: e.currentTarget.value })}
                  />
                </label>
                <button class="btn btn--filled has-state" type="submit" disabled={net.connecting !== null}>{w.connect}</button>
              </form>
            </div>
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<footer class="detail-foot">
  <button class="btn btn--text has-state" onclick={() => openSettings('ms-settings:network-wifi')}>
    {i18n.m.common.moreSettings}<Icon name="open_in_new" />
  </button>
</footer>
