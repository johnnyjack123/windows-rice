/**
 * Quick-setting tiles. Extension point: a tile is a plain object reading
 * reactive stores; which tiles show and in which order is configured in the
 * settings (panel.tilesLarge / panel.tilesSmall).
 */
import type { Backend } from '$backend/index';
import { i18n } from '$i18n/i18n.svelte';

/** Drill-in pages a tile may open. */
export type TilePage = 'wifi' | 'bluetooth';

/** Definition of a quick-setting tile. */
export interface TileDef {
  id: string;
  label: () => string;
  icon: () => string;
  /** Secondary line (large tiles only). */
  sub: () => string;
  on: () => boolean;
  /** False when the feature does not exist on this machine. */
  available: () => boolean;
  toggle: () => void;
  /** Opens a drill-in page from the text zone. */
  page?: TilePage;
  /** Windows settings page opened on right-click (`ms-settings:` URI). */
  settings?: string;
}

/**
 * Built-in tiles, keyed by id.
 *
 * @param b - Backend stores.
 * @returns Tile definitions.
 */
export function builtinTiles(b: Backend): Record<string, TileDef> {
  const m = () => i18n.m;
  const net = () => b.network.state;
  const list: TileDef[] = [
    {
      id: 'wifi',
      label: () => m().tiles.wifi,
      icon: () => (net().wifi.enabled ? 'wifi' : 'wifi_off'),
      sub: () => (!net().wifi.enabled ? m().common.off : net().wifi.connected?.ssid ?? m().tiles.notConnected),
      on: () => net().wifi.enabled,
      available: () => net().wifi.available,
      toggle: () => void b.network.setWifi(!net().wifi.enabled),
      page: 'wifi',
      settings: 'ms-settings:network-wifi',
    },
    {
      id: 'bluetooth',
      label: () => m().tiles.bluetooth,
      icon: () => (b.bluetooth.state.enabled ? 'bluetooth' : 'bluetooth_disabled'),
      sub: () => {
        if (!b.bluetooth.state.enabled) {
          return m().common.off;
        }
        const n = b.bluetooth.state.devices.filter((d) => d.connected).length;
        return n ? m().tiles.devicesConnected(n) : m().common.on;
      },
      on: () => b.bluetooth.state.enabled,
      available: () => b.bluetooth.state.available,
      toggle: () => void b.bluetooth.setEnabled(!b.bluetooth.state.enabled),
      page: 'bluetooth',
      settings: 'ms-settings:bluetooth',
    },
    {
      id: 'dnd',
      label: () => m().tiles.dnd,
      icon: () => 'do_not_disturb_on',
      sub: () => (b.settings.value.notifications.dnd ? m().tiles.dndOn : m().common.off),
      on: () => b.settings.value.notifications.dnd,
      available: () => true,
      toggle: () => void b.settings.patch({ notifications: { dnd: !b.settings.value.notifications.dnd } }),
      settings: 'ms-settings:notifications',
    },
    {
      id: 'night',
      label: () => m().tiles.night,
      icon: () => 'nightlight',
      sub: () => (b.display.state.nightLight === null ? m().tiles.unsupported : b.display.state.nightLight ? m().tiles.nightOn : m().tiles.nightOff),
      on: () => b.display.state.nightLight === true,
      available: () => b.display.state.nightLight !== null,
      toggle: () => void b.display.setNightLight(!b.display.state.nightLight),
      settings: 'ms-settings:nightlight',
    },
    {
      id: 'airplane',
      label: () => m().tiles.airplane,
      icon: () => 'airplanemode_active',
      sub: () => (net().airplane ? m().common.on : m().common.off),
      on: () => net().airplane,
      available: () => true,
      toggle: () => void b.network.setAirplane(!net().airplane),
      settings: 'ms-settings:network-airplanemode',
    },
    {
      id: 'saver',
      label: () => m().tiles.saver,
      icon: () => 'energy_savings_leaf',
      sub: () => (b.power.state.energySaver ? m().common.on : m().common.off),
      on: () => b.power.state.energySaver,
      available: () => b.power.state.energySaverSupported,
      toggle: () => void b.power.setEnergySaver(!b.power.state.energySaver),
      settings: 'ms-settings:powersleep',
    },
    {
      id: 'mic',
      label: () => m().tiles.mic,
      icon: () => (b.audio.state.micMuted ? 'mic_off' : 'mic'),
      sub: () => (b.audio.state.micMuted ? m().common.off : m().common.on),
      on: () => !b.audio.state.micMuted,
      available: () => b.audio.state.micAvailable,
      toggle: () => void b.audio.setMicMuted(!b.audio.state.micMuted),
      settings: 'ms-settings:sound',
    },
    {
      id: 'hotspot',
      label: () => m().tiles.hotspot,
      icon: () => 'wifi_tethering',
      sub: () => (net().hotspot.enabled ? m().common.on : m().common.off),
      on: () => net().hotspot.enabled,
      available: () => net().hotspot.supported,
      toggle: () => void b.network.setHotspot(!net().hotspot.enabled),
      settings: 'ms-settings:network-mobilehotspot',
    },
  ];
  return Object.fromEntries(list.map((t) => [t.id, t]));
}
