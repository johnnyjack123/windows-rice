import type {
  AudioState, BluetoothState, ConnectResult, DisplayState, MediaSession, MediaState, NetworkState, PowerState,
  WifiNetwork,
} from '../contract';
import { Emitter, errorCode, type Transport } from '../transport';
import { extractColor } from '$design/color';

/** Media sessions (GSMTC) with extrapolated position and album colour. */
export class MediaStore {
  state = $state<MediaState>({ sessions: [], currentId: null });
  /** Characteristic colour of the current cover (drives `--media-color`). */
  color = $state('#8e8e93');
  /** Fires when the current track changes (title/artist), not on play/pause. */
  readonly trackChanged = new Emitter<MediaSession>();

  readonly current: MediaSession | null = $derived(
    this.state.sessions.find((s) => s.id === this.state.currentId) ?? this.state.sessions[0] ?? null,
  );

  private lastArt: string | null = null;
  private lastKey = '';

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    this.apply(await this.t.invoke('media_get'));
    this.t.listen('media://state', (s) => this.apply(s));
  }

  private apply(state: MediaState): void {
    this.state = state;
    const current = this.current;
    const key = current ? `${current.title}\u0000${current.artist}` : '';
    if (current && key !== this.lastKey && this.lastKey) {
      this.trackChanged.emit(current);
    }
    this.lastKey = key;
    const art = current?.artUrl ?? null;
    if (art !== this.lastArt) {
      this.lastArt = art;
      if (art) {
        void extractColor(art).then((c) => {
          if (this.lastArt === art) {
            this.color = c;
          }
        });
      }
    }
  }

  /**
   * Position of the current session, extrapolated while playing.
   *
   * @param now - Reference time in ms.
   * @returns Seconds.
   */
  position(now: number): number {
    const s = this.current;
    if (!s) {
      return 0;
    }
    const elapsed = s.playing ? (now - Date.parse(s.positionAt)) / 1000 : 0;
    return Math.max(0, Math.min(s.duration, s.position + elapsed));
  }

  /** @param action - Transport action for the current session. */
  control(action: 'play' | 'pause' | 'toggle' | 'next' | 'previous'): Promise<void> {
    return this.t.invoke('media_control', { sessionId: this.current?.id ?? null, action });
  }

  /** @param position - Target position in seconds. */
  seek(position: number): Promise<void> {
    return this.t.invoke('media_seek', { sessionId: this.current?.id ?? null, position });
  }
}

/** Output volume, devices and microphone privacy. */
export class AudioStore {
  state = $state<AudioState>({ volume: 0.5, muted: false, device: null, devices: [], micMuted: false, micAvailable: false });
  /** Fires when volume or mute changed (for the OSD). */
  readonly volumeChanged = new Emitter<AudioState>();

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    this.state = await this.t.invoke('audio_get');
    this.t.listen('audio://state', (s) => {
      const changed = s.volume !== this.state.volume || s.muted !== this.state.muted;
      this.state = s;
      if (changed) {
        this.volumeChanged.emit(s);
      }
    });
  }

  /** @param volume - 0…1. */
  setVolume(volume: number): Promise<void> {
    const v = Math.min(1, Math.max(0, volume));
    this.state = { ...this.state, volume: v, muted: false };
    return this.t.invoke('audio_set_volume', { volume: v });
  }

  /** @param muted - Mute output. */
  setMuted(muted: boolean): Promise<void> {
    return this.t.invoke('audio_set_muted', { muted });
  }

  /** @param muted - Mute the default microphone. */
  setMicMuted(muted: boolean): Promise<void> {
    this.state = { ...this.state, micMuted: muted };
    return this.t.invoke('audio_set_mic_muted', { muted });
  }
}

/** Brightness and night light. */
export class DisplayStore {
  state = $state<DisplayState>({ brightness: null, nightLight: null });
  /** Fires when brightness changed (for the OSD). */
  readonly brightnessChanged = new Emitter<number>();

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    this.state = await this.t.invoke('display_get');
    this.t.listen('display://state', (s) => {
      const changed = s.brightness !== this.state.brightness && s.brightness !== null;
      this.state = s;
      if (changed) {
        this.brightnessChanged.emit(s.brightness!);
      }
    });
  }

  /** @param value - 0…1. */
  setBrightness(value: number): Promise<void> {
    const v = Math.min(1, Math.max(0, value));
    this.state = { ...this.state, brightness: v };
    return this.t.invoke('display_set_brightness', { value: v });
  }

  /** @param enabled - Night light on/off. */
  setNightLight(enabled: boolean): Promise<void> {
    this.state = { ...this.state, nightLight: enabled };
    return this.t.invoke('display_set_night_light', { enabled });
  }
}

/** Battery and energy saver. */
export class PowerStore {
  state = $state<PowerState>({ battery: null, charging: false, energySaver: false, energySaverSupported: false });
  /** Fires when a charger is plugged in. */
  readonly plugged = new Emitter<PowerState>();

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    this.state = await this.t.invoke('power_get');
    this.t.listen('power://state', (s) => {
      const plugged = s.charging && !this.state.charging;
      this.state = s;
      if (plugged) {
        this.plugged.emit(s);
      }
    });
  }

  /** @param enabled - Energy saver on/off. */
  setEnergySaver(enabled: boolean): Promise<void> {
    this.state = { ...this.state, energySaver: enabled };
    return this.t.invoke('power_set_energy_saver', { enabled });
  }

  /** @param action - Session action (lock, sleep, sign out, restart, shut down). */
  action(action: 'lock' | 'sleep' | 'signout' | 'restart' | 'shutdown'): Promise<void> {
    return this.t.invoke('power_action', { action });
  }
}

/** Wi-Fi, airplane mode and hotspot. */
export class NetworkStore {
  state = $state<NetworkState>({
    wifi: { available: false, enabled: false, connected: null, permission: 'granted' },
    airplane: false,
    hotspot: { supported: false, enabled: false },
  });
  /** Last scan result (without the connected network). */
  networks = $state<WifiNetwork[]>([]);
  scanning = $state(false);
  /** SSID of a connection attempt in progress. */
  connecting = $state<string | null>(null);
  /** Error of the last attempt, keyed by SSID. */
  errors = $state<Record<string, NonNullable<ConnectResult['error']>>>({});

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    this.state = await this.t.invoke('network_get');
    this.t.listen('network://state', (s) => {
      this.state = s;
    });
    this.t.listen('network://scan', (list) => {
      this.networks = list;
    });
  }

  /** Scans for networks; permission problems surface in `state`. */
  async scan(): Promise<void> {
    if (this.scanning) {
      return;
    }
    this.scanning = true;
    try {
      this.networks = await this.t.invoke('network_scan');
    } catch (error) {
      if (errorCode(error) === 'permission_required') {
        this.state = { ...this.state, wifi: { ...this.state.wifi, permission: 'location_required' } };
      }
    } finally {
      this.scanning = false;
    }
  }

  /**
   * @param ssid - Network to join.
   * @param key - Network key for new secured networks.
   * @returns Whether it worked.
   */
  async connect(ssid: string, key: string | null): Promise<ConnectResult> {
    this.connecting = ssid;
    const { [ssid]: _previous, ...rest } = this.errors;
    this.errors = rest;
    try {
      const result = await this.t.invoke('network_connect', { ssid, key });
      if (!result.ok) {
        this.errors = { ...this.errors, [ssid]: result.error ?? 'failed' };
      }
      return result;
    } catch {
      this.errors = { ...this.errors, [ssid]: 'failed' };
      return { ok: false, error: 'failed' };
    } finally {
      this.connecting = null;
    }
  }

  /** @param enabled - Wi-Fi radio on/off. */
  setWifi(enabled: boolean): Promise<void> {
    this.state = { ...this.state, wifi: { ...this.state.wifi, enabled } };
    return this.t.invoke('network_set_wifi', { enabled });
  }

  /** @param enabled - Airplane mode on/off. */
  setAirplane(enabled: boolean): Promise<void> {
    this.state = { ...this.state, airplane: enabled };
    return this.t.invoke('network_set_airplane', { enabled });
  }

  /** @param enabled - Mobile hotspot on/off. */
  setHotspot(enabled: boolean): Promise<void> {
    this.state = { ...this.state, hotspot: { ...this.state.hotspot, enabled } };
    return this.t.invoke('network_set_hotspot', { enabled });
  }
}

/** Bluetooth radio and paired devices. */
export class BluetoothStore {
  state = $state<BluetoothState>({ available: false, enabled: false, devices: [] });
  /** Device ids with an operation in progress. */
  busy = $state<string[]>([]);
  /** Devices whose last connect attempt failed. */
  failed = $state<string[]>([]);

  /** @param t - Backend transport. */
  constructor(private readonly t: Transport) {}

  /** Loads and subscribes. */
  async init(): Promise<void> {
    this.state = await this.t.invoke('bluetooth_get');
    this.t.listen('bluetooth://state', (s) => {
      this.state = s;
    });
  }

  /** @param enabled - Bluetooth radio on/off. */
  setEnabled(enabled: boolean): Promise<void> {
    this.state = { ...this.state, enabled };
    return this.t.invoke('bluetooth_set_enabled', { enabled });
  }

  /**
   * Connects or disconnects a device.
   *
   * @param id - Device id.
   */
  async toggle(id: string): Promise<void> {
    const device = this.state.devices.find((d) => d.id === id);
    if (!device || this.busy.includes(id)) {
      return;
    }
    this.busy = [...this.busy, id];
    this.failed = this.failed.filter((x) => x !== id);
    try {
      if (device.connected) {
        await this.t.invoke('bluetooth_disconnect', { id });
      } else {
        const result = await this.t.invoke('bluetooth_connect', { id });
        if (!result.ok) {
          this.failed = [...this.failed, id];
        }
      }
    } catch {
      this.failed = [...this.failed, id];
    } finally {
      this.busy = this.busy.filter((x) => x !== id);
    }
  }
}
