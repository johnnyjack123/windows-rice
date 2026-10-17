# Backend-Auftrag: Rust/Tauri-Backend für rice

> **Für den Agenten, der das Backend baut.** Das Frontend ist fertig und läuft im Browser gegen ein simuliertes Backend.
> Deine Aufgabe ist das echte Windows-Backend in Rust (Tauri 2). Es muss **exakt** zum vorhandenen Frontend passen.
>
> **Quelle der Wahrheit:** [`web/src/backend/contract.ts`](../web/src/backend/contract.ts) (alle Commands, Events, Typen).
> **Verhaltensreferenz:** [`web/src/backend/mock/mock-backend.ts`](../web/src/backend/mock/mock-backend.ts) — so soll sich
> jeder Command anfühlen (Zeiten, Fehlerfälle, Folge-Events).
> Hintergrund: [`docs/PLAN.md`](PLAN.md) §4–§5 (Fenstertechnik, Machbarkeit pro Feature).
>
> **Start:** Arbeite die Schritte aus §6 der Reihe nach ab, beginnend mit Schritt 1, auf einem eigenen Branch
> (`feat/rust-backend`). Das Frontend im Browser (`pnpm dev` → `/lab.html`) zeigt jederzeit, wie es sich anfühlen soll.

---

## 0. Spielregeln

1. **Frontend nicht umbauen.** Du arbeitest in `src-tauri/` (und optional `crates/`). Änderungen unter `web/` nur, wenn der
   Vertrag sich ändern *muss*. Dann **alle drei** anpassen: `contract.ts`, `mock-backend.ts` (damit der Browser-Modus
   weiterläuft) und diese Datei. Danach `pnpm verify` (Typcheck, Tests, Build) grün.
2. **Kein Command darf panicken.** Jeder Command gibt `Result<T, BackendError>` zurück. Nicht unterstützte Features sind
   ein *Zustand* (`null`, `supported: false`, `permission: 'unsupported'`), kein Absturz.
3. **Push statt Poll.** Wo Windows Callbacks/Notifications bietet, nutze sie. Gepollt wird nur der Cursor (Klick-durch,
   Kantenerkennung) und die Systemlast, Letztere nur solange `system_subscribe` aktiv ist.
4. **Keine Geheimnisse in Dateien.** CalDAV-Passwörter ausschließlich in der Windows-Anmeldeinformationsverwaltung.
5. Commits nach Conventional Commits (englisch), Code-Kommentare englisch, Doku deutsch.

---

## 1. Projekt-Setup

```
rice/
├─ package.json, vite.config.ts      Frontend (fertig). Vite-Root ist web/, Build nach dist/
├─ web/                               Frontend (fertig)
├─ src-tauri/                         ← NEU: Tauri-2-App (Fenster, IPC-Registrierung, Setup)
└─ crates/                            ← empfohlen: rice-core, rice-win, rice-services
```

- **Tauri 2** (aktuelle stabile 2.x), Frontend-Paket `@tauri-apps/api` 2.x ist bereits installiert. Die CLI fehlt noch:
  `pnpm add -D @tauri-apps/cli@2` und im `package.json` das Script `"tauri": "tauri"` ergänzen (dann `pnpm tauri dev`).
  Voraussetzungen auf dem Rechner: Rust (stable, MSVC), Visual Studio Build Tools mit C++-Workload, WebView2-Runtime.
- `tauri.conf.json`:
  - `build.devUrl = "http://127.0.0.1:1420"`, `build.frontendDist = "../dist"`
  - `build.beforeDevCommand = "pnpm dev"`, `build.beforeBuildCommand = "pnpm build"`
  - Fenster siehe §3. Nur das Island-Fenster startet beim Boot; Einstellungen und Pausenfenster werden bei Bedarf
    erzeugt.
- Plugins: `tauri-plugin-single-instance`, `tauri-plugin-autostart`, `tauri-plugin-global-shortcut`, `tauri-plugin-log`.
  Shell-Öffnen machst du selbst (§4, `shell_open`), damit die Whitelist bei dir liegt.
- Empfohlene Crates: `windows` (Win32 + WinRT), `tokio`, `serde`/`serde_json`, `toml`, `notify`, `sysinfo`, `keyring`,
  `reqwest` (rustls), `quick-xml` oder `libdav` (EUPL-1.2), `icalendar`/`ical`, `rrule`, `chrono` + `chrono-tz`,
  `rusqlite`, `image`, `base64`, `tracing`.
- Empfohlene Struktur: ein `Service`-Trait in `rice-core` (siehe PLAN §3.3), je Service ein Modul in `rice-services`,
  dünne, getestete Windows-Wrapper in `rice-win`. Fachlogik (CalDAV-Parsing, Settings-Merge, Bars aus Signalstärke …)
  ohne Windows-Abhängigkeit, damit sie unter `cargo test` läuft.

---

## 2. IPC-Konventionen (verbindlich)

| Thema | Regel |
|---|---|
| Command mit Eingabe | genau **ein** Parameter namens `args`: `#[tauri::command] async fn config_set(args: ConfigSetArgs, …)`. Das Frontend ruft `invoke('config_set', { args: { patch } })`. |
| Command ohne Eingabe | keine Parameter (`invoke('media_get', {})`). |
| Feldnamen | `#[serde(rename_all = "camelCase")]` überall. Enum-Werte wie im Vertrag (meist `snake_case`/Kleinschreibung, z. B. `"location_required"`). |
| Fehler | `Err(BackendError { code, message })`, serialisiert als `{ "code": "...", "message": "..." }`. Codes: `not_supported`, `permission_required`, `not_found`, `invalid_args`, `auth_failed`, `network`, `timeout`, `access_denied`, `internal`. |
| Events | `app.emit("<name>", payload)` an **alle** Fenster. Namen exakt wie im Vertrag (`media://state` …). |
| Zeiten | ISO 8601 **mit Offset** (`chrono::DateTime<FixedOffset>` → RFC 3339). Tagesdaten ohne Zeit: `"YYYY-MM-DD"`. |
| Einheiten | Brüche 0…1 (Lautstärke, Helligkeit, Akku, Signal als 0–4 Balken), Sekunden für Medien, Bytes/Bytes pro Sekunde, Längen in **CSS-Pixeln** (logisch, DPI-unabhängig). |
| `null` | `Option<T>` → `null` (nicht weglassen, außer der Vertrag markiert das Feld mit `?`). |
| Bilder | Cover, App-Icons, Wallpaper als **Data-URL** (`data:image/jpeg;base64,…`). So bleibt das Canvas im Frontend „untainted“ (Farbextraktion, Glas-Vorrendern). Cover/Icons ≤ 300 px, Wallpaper ≤ Monitorauflösung, JPEG q≈85. |

**Typ-Abgleich (empfohlen):** `tauri-specta` erzeugt TS-Typen aus Rust. Generiere nach `web/src/backend/generated.ts` und
lege einen Typtest an, der prüft, dass die generierten Typen den Vertragstypen zuweisbar sind. So fällt jede Abweichung
im `pnpm check` auf.

**Capabilities (Tauri 2 ACL):** Alle Fenster (`island`, `settings`, `break-*`) brauchen mindestens `core:default` und
`core:event:default` (das Frontend nutzt `listen`). Prüfe beim Setup, ob deine App-Commands zusätzliche Freigaben
brauchen (abhängig davon, ob du ein App-Manifest in `build.rs` definierst).

---

## 3. Fenster

### 3.1 `island` (startet beim Boot)

| Eigenschaft | Wert |
|---|---|
| URL | `island.html` |
| Stil | `transparent: true`, `decorations: false`, `shadow: false`, `alwaysOnTop: true`, `skipTaskbar: true`, `resizable: false`, `focus: false` |
| Browser-Argumente | `additionalBrowserArgs`: Tauris Standard-Flags **plus** `--autoplay-policy=no-user-gesture-required` (Alarmtöne werden per Web Audio ohne Nutzerinteraktion abgespielt). Beim Überschreiben die Tauri-Defaults (`--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection`) mitgeben. |
| Größe | kommt vom Frontend über `island_configure { width, height }` (logische px), Aufruf bei Start und nach Theme-Wechseln. |
| Position | horizontal zentriert auf dem konfigurierten Monitor (`settings.island.monitor`: `"primary"` oder Index), oben bündig mit der **Arbeitsfläche** dieses Monitors. Bei Monitor-/DPI-/Taskleisten-Änderungen neu positionieren und `system://wallpaper` senden (Fensterposition gehört zu dessen Payload). |
| Erweiterte Stile | kompakt: `WS_EX_NOACTIVATE` (stiehlt keinen Fokus). Bei `island_set_expanded(true)`: `NOACTIVATE` entfernen und Fenster aktivieren (Tastatur/Eingaben im Panel). Bei `false`: zurück. |

**Klick-durch (kritisch).** Das Fenster ist größer als die sichtbare Island. Grundzustand: `set_ignore_cursor_events(true)`.
Das Frontend meldet über `island_set_hit_regions { rects }` die klickbaren Rechtecke (logische px, relativ zum
Fenster-Client-Bereich; leere Liste = komplett durchklickbar). Ein Thread prüft ~60× pro Sekunde `GetCursorPos`, rechnet
in Fensterkoordinaten um (DPI beachten) und schaltet `ignore_cursor_events` **nur bei Wechseln** um.

**Fokusverlust.** Ist die Island aufgeklappt und das Fenster wird deaktiviert (Klick daneben, Alt-Tab), sende
`island://focus-lost` (Payload `null`). Das Frontend klappt dann zu.

**Überlappung (`island://occlusion`).** Die „Island-Zone“ ist ein Rechteck oben mittig auf dem Island-Monitor:
480 × (offsetTop + 50) logische px. `occluded = true`, wenn ein sichtbares Top-Level-Fenster diese Zone schneidet —
ausgenommen eigene Fenster, Desktop (`Progman`/`WorkerW`), Taskleiste (`Shell_TrayWnd`), minimierte und geclockte
(`DWMWA_CLOAKED`) Fenster sowie Fenster mit `WS_EX_TOOLWINDOW`. Neu berechnen bei WinEvents
(`EVENT_SYSTEM_FOREGROUND`, `EVENT_OBJECT_LOCATIONCHANGE` gefiltert auf Top-Level, `EVENT_SYSTEM_MINIMIZESTART/END`,
`EVENT_OBJECT_SHOW/HIDE`, `EVENT_OBJECT_CLOAKED/UNCLOAKED`), entprellt (~50 ms), Event **nur bei Änderung**.
`fullscreen = true`, wenn `SHQueryUserNotificationState` `QUNS_BUSY`, `QUNS_RUNNING_D3D_FULL_SCREEN` oder
`QUNS_PRESENTATION_MODE` liefert, oder das Vordergrundfenster den ganzen Monitor bedeckt. `island_occlusion` liefert den
aktuellen Zustand.

**Kante (`island://edge`).** Steht der Cursor ≥ 120 ms am oberen Monitorrand (y ≤ Monitor-Oberkante + 1) und horizontal
höchstens 160 px von der Monitormitte entfernt → `{ active: true }`. Aktiv bleibt es, bis der Cursor die Vereinigung der
aktuellen Hit-Regions plus 32 px Rand verlässt **und** tiefer als 40 px unter der Oberkante ist → `{ active: false }`.
(Referenzlogik: `trackEdge()` im Mock.)

**Hotkey.** `settings.island.hotkey` (Standard `"Super+Alt+Space"`) global registrieren → `island://toggle`. Bei
Änderung der Einstellung neu registrieren.

**Vollbild/Sperrbildschirm:** Bei Sperre (`WTS_SESSION_LOCK`) nichts senden; das Fenster bleibt einfach unsichtbar.

**GPU.** Bei Glas-Themes zeichnet das Frontend das Material per **WebGL2** in eine Canvas des transparenten Fensters
(vormultipliziertes Alpha). Hardwarebeschleunigung von WebView2 also nicht abschalten (keine `--disable-gpu`-Flags). Fehlt
WebGL2, fällt das Frontend selbst auf CSS zurück.

**Zwei Pillen.** Die Hit-Regions enthalten bis zu zwei Rechtecke: Island und Bubble. Die Bubble kann beim Überfahren zu
einer Vorschau mit Schnellaktionen wachsen und schiebt die Island dabei zur Seite – die gemeldeten Rechtecke decken das
ab. Für das Backend ändert sich dadurch nichts.

### 3.2 `settings` (bei Bedarf)

`window_open_settings { page? }`: existiert das Fenster, fokussieren und auf `settings.html#<page>` navigieren; sonst
erzeugen: Titel „rice“, 1120 × 780, min. 900 × 640, normale Dekoration (Mica optional). Das Frontend ist im
Einstellungsfenster vollständig selbstständig.

### 3.3 `break-<n>` (Pomodoro-Pause im Vollbild, bei Bedarf)

Ist in den Pomodoro-Einstellungen „Pause anzeigen als: Vollbild“ gewählt, legt das Frontend während einer Pause ein
Fenster über **jeden** Monitor. Der Pomodoro selbst läuft im Island-Fenster; die Pausenfenster zeigen nur an.

| Eigenschaft | Wert |
|---|---|
| URL | `break.html` (je Monitor ein Fenster, Label `break-0`, `break-1` …) |
| Stil | `transparent: true` (die Seite blendet sich selbst ein), `decorations: false`, `alwaysOnTop: true`, `skipTaskbar: true`, `resizable: false`, deckt den **ganzen** Monitor ab (nicht nur die Arbeitsfläche), Fokus auf das Fenster des Island-Monitors |
| Schließen | Alt+F4/Schließen-Anfragen während der Pause ignorieren (`CloseRequested` → `prevent_close`). Der Ausweg ist der Button „Gedrückt halten zum Überspringen“ in der Seite (`break_finish`); Strg+Alt+Entf und Task-Manager funktionieren wie immer. |

| Command / Event | Verhalten |
|---|---|
| `break_show(BreakOverlay)` | Zustand merken, fehlende Fenster öffnen (Monitore per `EnumDisplayMonitors`), dann → `break://state` mit dem Zustand. Erneute Aufrufe aktualisieren nur (Pause angehalten, Pause vorbei). |
| `break_hide` | → `break://state` mit `null` (die Seiten blenden in ~400 ms aus), nach **450 ms** alle Pausenfenster schließen, Zustand löschen. |
| `break_state` | Letzter Zustand oder `null` – ein Pausenfenster liest ihn beim Laden. |
| `break_finish` | Vom Pausenfenster: wie `break_hide`, zusätzlich → `break://finished` (das Island-Fenster startet dann die nächste Fokuszeit). |
| Monitorwechsel | Während eine Pause läuft, Fenster für neue Monitore öffnen bzw. für entfernte schließen. |

---

## 4. Command- und Event-Referenz

Typen siehe `contract.ts`. „→“ = Event, das nach dem Command (oder bei externer Änderung) gesendet wird.

### App, Einstellungen, Themes

| Command | Verhalten |
|---|---|
| `app_info` | `{ version, themesDir, configDir }` — `configDir = %APPDATA%\rice`, `themesDir = %APPDATA%\rice\themes` (absolute Pfade). |
| `config_get` | Liefert das **gespeicherte** JSON-Objekt aus `%APPDATA%\rice\config.json` (oder `{}`). Keine Defaults einfügen — die stehen im Frontend (`web/src/backend/settings.ts`). |
| `config_set { patch }` | Tiefes Mergen: Objekte schlüsselweise, Arrays und Primitive ersetzen, `null` löscht den Schlüssel (identisch zu `applyPatch()` in `settings.ts`). Atomar schreiben (temp + rename). Rückgabe: neues gespeichertes Objekt. → `config://changed` (gesamtes Objekt). Externe Dateiänderungen (Watcher) ebenfalls → `config://changed`. |
| `theme_list_user` | Alle Ordner unter `themes/` mit gültigem `theme.toml` + `theme.css` → `UserTheme[]`. |
| `theme_save_user { id, manifest, css }` | `id` muss `^[a-z0-9-]{1,48}$` sein und darf kein eingebautes Theme sein (`obsidian`, `monet`, `glass`, `terminal`, `paper`) → sonst `invalid_args`. Ordner anlegen/überschreiben, `theme.toml` (Manifest) + `theme.css` schreiben. → `theme://user-changed` (komplette Liste). |
| `theme_delete_user { id }` | Ordner löschen. → `theme://user-changed`. Der Ordner wird zusätzlich überwacht (Hot Reload beim Bearbeiten von `theme.css`). |

`theme.toml`:

```toml
name = "Mein Papier"
author = "Du"
version = "1.0.0"
schemes = ["light"]
based_on = "paper"        # im JSON: basedOn
```

Was das Backend aus den Einstellungen selbst braucht (mit diesen Defaults, falls der Schlüssel fehlt):
`island.monitor = "primary"`, `island.offsetTop = 10`, `island.hotkey = "Super+Alt+Space"`, `locale = "auto"`,
`notifications.hideWindowsBanners = true` (siehe Mitteilungen).

### System

| Command | Verhalten |
|---|---|
| `system_appearance` | `dark` aus `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize\AppsUseLightTheme` (0 = dunkel), `accent` aus `UISettings.GetColorValue(Accent)`, `reducedMotion = !SPI_GETCLIENTAREAANIMATION`, `transparency` aus `EnableTransparency`. → `system://appearance` bei `UISettings.ColorValuesChanged`/`WM_SETTINGCHANGE`. |
| `system_wallpaper` | Wallpaper des Island-Monitors über `IDesktopWallpaper` (`GetWallpaper`, `GetPosition` → `fit`: Fill→`fill`, Fit→`fit`, Stretch→`stretch`, Tile→`tile`, Center→`center`, Span→`span`), Bild auf Monitorgröße herunterskalieren, als JPEG-Data-URL. `color` = Hintergrundfarbe. `monitor` = Monitorgröße in logischen px, `window` = Island-Fensterposition relativ zum Monitor (logische px). → `system://wallpaper` bei Wallpaperwechsel (`WM_SETTINGCHANGE` mit `SPI_SETDESKWALLPAPER`), Monitor- oder Positionsänderung. |
| `system_fonts` | Familiennamen der Systemschriften (DirectWrite-Systemkollektion), sortiert, eindeutig. |
| `system_monitors` | `EnumDisplayMonitors` + `GetMonitorInfoW` + Anzeigename → `MonitorInfo[]` (`index` = Reihenfolge, wie sie `island.monitor` verwendet). |
| `island_configure { width, height }` | Fenstergröße setzen (logische px) und neu positionieren. |
| `island_set_hit_regions { rects }` | Siehe §3.1. |
| `island_set_expanded { expanded }` | Siehe §3.1 (Aktivierung/`NOACTIVATE`). |
| `island_occlusion` | Aktueller `Occlusion`-Zustand. |
| `window_open_settings { page? }` | Siehe §3.2. |
| `shell_open { target }` | **Whitelist:** `ms-settings:`-URIs, `https://`-URLs und Pfade innerhalb von `configDir`. Alles andere → `invalid_args`. Öffnen per `ShellExecuteW`. |

### Medien (GSMTC)

| Command / Event | Verhalten |
|---|---|
| `media_get` | `GlobalSystemMediaTransportControlsSessionManager`: alle Sitzungen; `currentId` = `GetCurrentSession`. Je Sitzung: `TryGetMediaPropertiesAsync` (Titel, Interpret, Album, Thumbnail → Data-URL ≤ 300 px; nach Inhalts-Hash cachen), `GetPlaybackInfo` (Status, `Controls.IsPrevious/NextEnabled`, `IsPlaybackPositionEnabled` → `canSeek`), `GetTimelineProperties` (`Position`, `EndTime`, `LastUpdatedTime` → `positionAt`). `appName` aus der AUMID (`SourceAppUserModelId` → `AppInfo.DisplayInfo.DisplayName`, Fallback: AUMID). |
| `media_control { sessionId, action }` | `null` = aktuelle Sitzung. `play`/`pause`/`toggle`/`next`/`previous` → `TryPlayAsync` … `TrySkipPreviousAsync`. |
| `media_seek { sessionId, position }` | `TryChangePlaybackPositionAsync` (100-ns-Ticks). |
| → `media://state` | Bei `SessionsChanged`, `CurrentSessionChanged`, je Sitzung `MediaPropertiesChanged`, `PlaybackInfoChanged`, `TimelinePropertiesChanged`; entprellt ~50 ms. **Kein** sekündliches Senden — das Frontend extrapoliert die Position aus `position` + `positionAt`. |

### Audio, Anzeige, Energie

| Command / Event | Verhalten |
|---|---|
| `audio_get` | Standard-Wiedergabegerät (`IMMDeviceEnumerator`, eConsole) → `IAudioEndpointVolume` (`GetMasterVolumeLevelScalar`, `GetMute`); `devices` = aktive Render-Endpunkte; `kind` aus FormFactor/Bus (Bluetooth am Geräteinstanzpfad erkennbar). Mikrofon: Standard-Aufnahmegerät, `micMuted` = dessen Mute, `micAvailable` = Gerät vorhanden. |
| `audio_set_volume { volume }` | `SetMasterVolumeLevelScalar` (hebt Mute auf). |
| `audio_set_muted`, `audio_set_mic_muted` | `SetMute` auf Render- bzw. Capture-Endpunkt. |
| `audio_set_device { id }` | Standardgerät für eConsole/eMultimedia/eCommunications über `IPolicyConfig` (inoffiziell, hinter Feature-Flag; bei Fehler `not_supported`). |
| → `audio://state` | `IAudioEndpointVolumeCallback` (auch Hardware-Lautstärketasten!) und `IMMNotificationClient`. Das Frontend zeigt bei jeder Lautstärkeänderung das OSD in der Island. |
| `display_get` | Helligkeit des Island-Monitors: intern per WMI (`WmiMonitorBrightness`), extern per DDC/CI (`GetMonitorBrightness`, dxva2). Geht beides nicht → `brightness: null`. `nightLight`: Registry-Blob unter `…\CloudStore\Store\DefaultAccount\Current\default$windows.data.bluelightreduction.bluelightreductionstate\…` lesen; bei Unsicherheit `null` (Kachel zeigt „Wird nicht unterstützt“). |
| `display_set_brightness`, `display_set_night_light` | Entsprechend; Nachtlicht experimentell. |
| → `display://state` | `WmiMonitorBrightnessEvent` bzw. nach eigenen Änderungen. |
| `power_get` | `GetSystemPowerStatus` (`battery: null`, wenn kein Akku: `BatteryFlag == 128`), Energiesparmodus über `GUID_POWER_SAVING_STATUS` lesen. `energySaverSupported` nur `true`, wenn du ihn zuverlässig schalten kannst. |
| `power_set_energy_saver` | Experimentell; sonst `not_supported`. |
| `power_action { action }` | `lock` → `LockWorkStation`; `sleep` → `SetSuspendState(false, false, false)`; `signout` → `ExitWindowsEx(EWX_LOGOFF)`; `restart`/`shutdown` → `ExitWindowsEx` mit `SE_SHUTDOWN_NAME`-Privileg. |
| → `power://state` | `RegisterPowerSettingNotification` (`GUID_BATTERY_PERCENTAGE_REMAINING`, `GUID_ACDC_POWER_SOURCE`, `GUID_POWER_SAVING_STATUS`). Das Frontend zeigt beim Einstecken („charging“ false → true) die Lade-Aktivität. |

### Netzwerk

| Command / Event | Verhalten |
|---|---|
| `network_get` | WLAN über `wlanapi`: `WlanEnumInterfaces`, aktuelle Verbindung (`WlanQueryInterface` `wlan_intf_opcode_current_connection`) → `connected` (`signal` = Qualität 0–100 → 0–4 Balken, `detail` z. B. „5 GHz · WPA3“). `enabled` aus dem Radio (`Windows.Devices.Radios`). `permission = 'location_required'`, wenn WLAN-APIs wegen fehlender Standortfreigabe (Windows 11 24H2) verweigert werden. `airplane` = alle Radios aus. `hotspot` über `NetworkOperatorTetheringManager` (vorher prüfen, ob es ohne Paket-Identität funktioniert; sonst `supported: false`). |
| `network_scan` | `WlanScan`, auf `wlan_notification_acm_scan_complete` warten (Timeout 4 s), `WlanGetAvailableNetworkList`; pro SSID das stärkste Netz; versteckte Netze weglassen; ohne das verbundene. `known` = Profil vorhanden. Fehlende Standortfreigabe → Fehler `permission_required`. → `network://scan`. |
| `network_set_wifi { enabled }` | WLAN-Radio schalten. Einschalten beendet einen aktiven Flugmodus (Referenz: Mock). |
| `network_connect { ssid, key }` | Bekanntes Netz: `WlanConnect` mit Profil. Neues gesichertes Netz: Profil-XML (WPA2/WPA3-Personal, AES, `keyMaterial`) per `WlanSetProfile`, dann verbinden. Auf Verbindungsnotification warten (max. 15 s). Ergebnis `{ ok }` bzw. `{ ok: false, error: 'wrong_key' | 'timeout' | 'failed' }` (Authentifizierungsfehler → `wrong_key`, das neu angelegte Profil dann wieder löschen). |
| `network_disconnect` | `WlanDisconnect`. |
| `network_set_airplane { enabled }` | Alle Radios aus (vorherige Zustände merken) bzw. wiederherstellen; WLAN- und Bluetooth-Zustand entsprechend. |
| `network_set_hotspot { enabled }` | `StartTetheringAsync`/`StopTetheringAsync`. |
| → `network://state` | WLAN-Notifications (Verbindung, Radio), `NetworkInformation.NetworkStatusChanged`. |

### Bluetooth

| Command / Event | Verhalten |
|---|---|
| `bluetooth_get` | Radio-Zustand; gekoppelte Geräte (klassisch + BLE) per `DeviceWatcher`; `connected` aus `ConnectionStatus`; `battery` aus der Geräte-Eigenschaft `{104EA319-6EE2-4701-BD47-8DDBF425BBE5} 2` (falls vorhanden, sonst `null`); `kind` aus Class of Device/Appearance. `canConnect = true` nur für Audiogeräte, die du per Kernel-Streaming-Property (`KSPROPERTY_ONESHOT_RECONNECT/DISCONNECT`, Ansatz wie „ToothTray“) schalten kannst. Alle anderen `false` → das Frontend öffnet dann `ms-settings:bluetooth`. |
| `bluetooth_set_enabled { enabled }` | Bluetooth-Radio schalten. |
| `bluetooth_connect { id }` / `bluetooth_disconnect { id }` | Wie oben; Ergebnis als `ConnectResult`. Koppeln neuer Geräte macht das Frontend absichtlich über die Windows-Einstellungen. |
| → `bluetooth://state` | DeviceWatcher-Ereignisse, Radio-`StateChanged`. |

### Mitteilungen

`UserNotificationListener` benötigt **Paket-Identität**. Plane ein *Sparse Package* („packaging with external location“)
mit der Capability `userNotificationListener`, das der Installer registriert. Ohne Identität: `permission:
'unsupported'` — nichts darf abstürzen.

| Command / Event | Verhalten |
|---|---|
| `notifications_get` | `GetAccessStatus` → `granted`/`denied`/`unknown` (bzw. `unsupported`). Bei `granted`: `GetNotificationsAsync(Toast)` → `AppNotification` (`id` = Id als String, `appId` = AUMID, `appName` = Anzeigename, `iconUrl` = Logo 48 px als Data-URL, `title` = erster Text des Toast-Bindings, `body` = restliche Texte, `time` = `CreationTime`). |
| `notifications_request_access` | `RequestAccessAsync` (UI-Thread). → `notifications://state`. |
| `notifications_dismiss { id }` / `notifications_clear` | `RemoveNotification` / `ClearNotifications`. → `notifications://removed` je Id. |
| → `notifications://posted` / `removed` | Über `NotificationChanged`; falls das Event in deiner Konfiguration nicht feuert, alle 2 s abgleichen (Diff nach Id). |

**Windows-Banner ausblenden (`notifications.hideWindowsBanners`, Standard `true`).** Solange rice läuft, zeigt nur die
Island neue Mitteilungen; im Infocenter von Windows bleiben sie. Recherche (Oktober 2026), Ergebnis und Vorgaben:

- „Nicht stören“ lässt sich **nicht** dokumentiert per Programm schalten: `FocusSessionManager.TryStartFocusSession` /
  `DeactivateFocus` sind ein *Limited Access Feature* mit Freischalt-Token von Microsoft
  ([Doku](https://learn.microsoft.com/en-us/uwp/api/windows.ui.shell.focussessionmanager.trystartfocussession)).
  WNF-/CloudStore-Tricks sind undokumentiert und brechen zwischen Windows-Versionen → **nicht verwenden**. Ebenso **nicht**
  `NOC_GLOBAL_SETTING_TOASTS_ENABLED` (globaler Schalter, Wirkung auf Infocenter und Listener ungeklärt).
- **Umsetzung:** pro App den Wert „Benachrichtigungsbanner anzeigen“ ausschalten: DWORD `ShowBanner = 0` unter
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\<AUMID>`. Die Einstellung selbst ist
  [dokumentiert](https://support.microsoft.com/windows/8942c744-6198-fe56-4639-34320cf9444e), der Registry-Wert nicht.
  Die Tauri/Rust-App Catrace nutzt genau das unter Windows 11 (26200): Mitteilungen landen weiter im Infocenter und beim
  Listener, wirksam ab der nächsten Mitteilung, ohne Explorer-Neustart
  ([PR](https://github.com/lanxiuyun/Catrace/pull/87)). **`Enabled` nie anfassen.**
- **AUMIDs:** alle Unterschlüssel von `…\Notifications\Settings` (rekursiv, Pfad-AUMIDs sind verschachtelt). Ein
  Unterschlüssel entsteht erst mit der ersten Mitteilung einer App → bei jeder neuen AUMID aus `NotificationChanged`
  sofort nachziehen (die allererste Mitteilung einer neuen App erscheint daher einmal als Windows-Banner).
- **Wiederherstellen:** vor jedem Schreiben den Originalzustand („fehlte“ oder alter DWORD) in
  `%APPDATA%\rice\banner-backup.json` sichern. Zurücksetzen beim Beenden, bei Abmelden/Herunterfahren
  (`WM_ENDSESSION`), aus einem Panic-Hook und **bei jedem Start** (Reste eines Absturzes), bevor erneut unterdrückt wird.
  Nur Einträge zurücksetzen, die noch `0` sind (eine zwischenzeitliche Änderung des Nutzers bleibt). Der Deinstaller ruft
  das Zurücksetzen ebenfalls auf. Setting auf `false` → sofort zurücksetzen.
- **Sicherheitsnetz:** Ist der Listener-Zugriff nicht `granted` (oder wird entzogen), **nicht** unterdrücken bzw. sofort
  zurücksetzen – sonst sieht der Nutzer gar keine Mitteilungen mehr.
- **Offen, früh testen:** ob `ShowBanner = 0` auch Wecker-/Anruf-Toasts betrifft, und ob der Listener Mitteilungen
  klassischer Desktop-Apps (Discord, Outlook, Signal) sieht.

### Kalender und Aufgaben (CalDAV, Nextcloud)

| Command / Event | Verhalten |
|---|---|
| `calendar_accounts` | Konten aus `%APPDATA%\rice\accounts.json` (ohne Passwort) inkl. Kalendern, Status, `lastSync`. Das Frontend zeigt nur ein Konto (Nextcloud), die Struktur erlaubt mehrere. |
| `calendar_add_account { name, url, user, password }` | Discovery: `<url>/.well-known/caldav` bzw. `remote.php/dav` → `current-user-principal` → `calendar-home-set` → Collections per `PROPFIND` (`displayname`, Apple `calendar-color`, `supported-calendar-component-set` VEVENT/VTODO). 401 → `auth_failed`, Verbindungsfehler → `network`. Passwort in die Anmeldeinformationsverwaltung (`keyring`, Dienst `rice`, Konto `caldav/<accountId>`). Erster Sync sofort. → `calendar://accounts`, danach `calendar://changed`. |
| `calendar_remove_account { id }` | Konto, Cache und Credential löschen. → `calendar://accounts`, `calendar://changed`. |
| `calendar_set_enabled { calendarId, enabled }` | In `accounts.json` speichern. → `calendar://accounts`, `calendar://changed`. |
| `calendar_sync` | Status `syncing` (→ `calendar://accounts`), Sync per `sync-collection` (RFC 6578), Fallback CTag/ETag; Status `ok` + `lastSync`; bei Änderungen → `calendar://changed`. Zusätzlich automatisch alle 5 min und beim Start. Offline → Status `offline`. |
| `calendar_events { from, to }` | Aus dem SQLite-Cache (`%APPDATA%\rice\cache.sqlite`): alle **Vorkommen** im Bereich (Wiederholungen mit `rrule` expandieren, `EXDATE`/`RECURRENCE-ID` beachten, Zeitzonen über `VTIMEZONE`/`chrono-tz`), nur aktivierte Kalender, nach `start` sortiert. Ganztägig: `allDay: true`, `start`/`end` = lokale Mitternacht. |
| `tasks_list` | Alle VTODO aktivierter Aufgabenlisten. `due` als `"YYYY-MM-DD"` (DATE) oder Date-Time, `done` = `STATUS:COMPLETED`. |
| `tasks_create { calendarId, title, due }` | Neues VTODO (`UID` = UUID, `SUMMARY`, `DUE;VALUE=DATE`, `STATUS:NEEDS-ACTION`) per `PUT` (`If-None-Match: *`). → `tasks://changed`. |
| `tasks_update { id, patch }` | `SUMMARY`/`DUE`/`STATUS` + `COMPLETED`/`PERCENT-COMPLETE` setzen, `PUT` mit `If-Match: <etag>`; bei 412 neu laden und erneut anwenden. → `tasks://changed`. |
| `tasks_delete { id }` | `DELETE` mit ETag. → `tasks://changed`. |

Offline-Änderungen in einer Warteschlange im Cache halten und beim nächsten Sync ausspielen. Erinnerungen berechnet das
Frontend selbst aus `calendar_events` (Vorlauf in `settings.calendar.reminderLead`).

### Systemmonitor

| Command / Event | Verhalten |
|---|---|
| `system_subscribe { intervalMs }` | Referenzgezählt. Beim ersten Abonnenten einen Takt starten, der `system://sample` sendet: CPU-Auslastung, Name, aktuelle GHz (`sysinfo`); RAM belegt/gesamt; GPU-Auslastung über PDH (`\GPU Engine(*engtype_3D)\Utilization Percentage`, pro Adapter summiert, max. 100) + VRAM über DXGI `QueryVideoMemoryInfo` + Adaptername (ohne GPU: `null`); Netz ↓/↑ in Bytes/s (Summe der Schnittstellen). |
| `system_unsubscribe` | Zähler runter; bei 0 Takt stoppen. |
| `system_processes { limit }` | Top-Prozesse nach CPU (Prozent der Gesamt-CPU), `memoryBytes` = Working Set, `iconUrl` optional (Exe-Icon, nach Pfad gecacht). |
| `system_kill { pid }` | `TerminateProcess`. Zugriff verweigert → `access_denied`. PID 0/4 und der eigene Prozess → `access_denied`. |

---

## 5. Was das Frontend selbst erledigt (nicht im Backend bauen)

Timer, Stoppuhr, Wecker, Pomodoro und Alarmtöne (Wecker und Pomodoro-Rhythmus werden über `settings.timers`
gespeichert; das Backend öffnet für den Pomodoro nur die Pausenfenster), Erinnerungen an Termine, Theme-Anwendung und
Theme-Editor (inkl. Erzeugung der `theme.css`), Glas-Vorrendern aus der Wallpaper-Data-URL und der Liquid-Glass-Shader,
Farbextraktion aus Cover/Wallpaper, Übersetzungen (Deutsch/Englisch), „Nicht stören“ (unterdrückt Island-Banner).

---

## 6. Reihenfolge und Abnahme

| Schritt | Inhalt | Fertig, wenn … |
|---|---|---|
| 1 | Tauri-Gerüst, Fenster, Config, Themes, Appearance/Wallpaper/Monitore, Island-Fensterverhalten (Klick-durch, Fokus, Überlappung, Kante, Hotkey), Einstellungsfenster, Pausenfenster, `shell_open` | `pnpm tauri dev` zeigt die Island; Desktop-Icons unter der transparenten Fläche bleiben klickbar; ein maximiertes Fenster blendet die Island aus; Maus an die obere Kante holt sie zurück; Theme-Wechsel im Einstellungsfenster wirkt sofort in der Island; Pomodoro mit „Vollbild“ legt in der Pause über jeden Monitor ein Fenster, „Gedrückt halten“ beendet es; Liquid Glass zeigt Lichtbrechung (WebGL aktiv) |
| 2 | Medien, Audio, Anzeige, Energie | Spotify/Browser-Medien erscheinen live; Lautstärketasten zeigen das Island-OSD; Mausrad über der Island ändert die Lautstärke |
| 3 | Systemmonitor | Werte stimmen grob mit dem Task-Manager; ohne geöffneten System-Tab keine Samples |
| 4 | Netzwerk, Bluetooth | WLAN-Liste (inkl. Standort-Fehlerzustand), Verbinden mit falschem/richtigem Schlüssel, Kopfhörer verbinden/trennen oder sauberer Fallback |
| 5 | Kalender, Aufgaben | Nextcloud-Konto einrichten, Termine/Aufgaben synchron mit Android (DAVx⁵), Aufgabe abhaken wirkt in Nextcloud |
| 6 | Mitteilungen + Sparse Package | Mitteilungen anderer Apps erscheinen als Banner in der Island und in der Liste; Windows-Banner bleiben aus und kommen nach dem Beenden (auch nach einem Absturz beim nächsten Start) zurück |
| 7 | Installer (NSIS/MSI), Autostart, Einzelinstanz | Installierbar, startet mit Windows |

Bei jedem Schritt: `cargo clippy -- -D warnings`, `cargo test`, `pnpm verify` grün. Leistungsbudget aus PLAN §1:
< 0,5 % CPU im Leerlauf, kein Polling außer Cursor (60 Hz) und abonniertem Systemmonitor.

---

## 7. Umsetzungsstand

**Schritte 1–3 umgesetzt** (Branch `feat/rust-backend`). Noch nicht gebaute Commands (Schritte 4–6) sind
registriert und liefern neutrale Zustände (`network_get` mit `available: false`, `notifications_get` mit
`permission: 'unsupported'` …) bzw. `not_supported` für Aktionen – siehe `src-tauri/src/commands/pending.rs`. Jeder
Schritt ersetzt dort seinen Block. Ein Test (`commands::tests`) prüft, dass genau die Commands aus `contract.ts`
registriert sind und alle gesendeten Event-Namen im Vertrag stehen.

**Aufbau**

| Crate | Inhalt |
|---|---|
| `crates/rice-core` | Vertragstypen (serde), `applyPatch`-Semantik, Backend-Sicht der Einstellungen, Geometrie (Platzierung, Island-Zone, Hit-Test), Kanten-Zustandsautomat, Whitelist für `shell_open`, Theme-Manifest, Zeitstempel, Medien-/Geräte-Abbildungen |
| `crates/rice-services` | `config.json` (atomar, Watcher), Theme-Ordner, entprellte Dateiüberwachung, Bild → JPEG-Data-URL |
| `crates/rice-win` | Win32/WinRT-Wrapper: Monitore (EDID-Namen, eingebaut/extern), Wallpaper, Darstellung, Schriften, Überlappung, System-Events-Thread (inkl. Energie), Shell, GSMTC, Core Audio, Helligkeit (WMI, DDC/CI), Energie, Systemlast (PDH, DXGI, Netzwerktabelle), Prozesse |
| `src-tauri` | Fenster, Commands, Events, Hotkey, Dienste Medien/Audio/Anzeige/Energie/Systemmonitor |

**Präzisierungen gegenüber §3/§4**

- **Fenster entstehen im Code**, nicht in `tauri.conf.json` (`app.windows` ist leer). Alle Fenster benutzen dieselben
  Browser-Argumente (`webview::BROWSER_ARGS`) – WebView2 teilt sich einen Browser-Prozess, abweichende Argumente lassen
  das Erzeugen weiterer Fenster scheitern.
- **Island und Pausenfenster gehören einem unsichtbaren Besitzerfenster.** `skipTaskbar` allein lässt sie in Alt+Tab
  stehen (tao setzt `WS_EX_APPWINDOW`); als „owned windows“ erscheinen sie weder in der Taskleiste noch in Alt+Tab.
- **`NOACTIVATE` und Klick-durch** laufen über `set_focusable` bzw. `set_ignore_cursor_events`, weil tao die erweiterten
  Fensterstile bei jeder Änderung komplett neu schreibt. Solange eine Maustaste gedrückt ist, bleibt die Island
  klickbar (Regler ziehen über den Rand hinaus).
- **Aufklappen/Zuklappen:** Beim Zuklappen bekommt das zuvor aktive Fenster die Tastatur zurück. `island://focus-lost`
  wird erst gesendet, wenn nach 40 ms wirklich ein anderes Fenster im Vordergrund ist (der Fokuswechsel in das
  WebView-Kindfenster löst sonst Fehlalarme aus).
- **Überlappung:** Zusätzlich zu §3.1 zählen Fenster mit `WS_EX_TRANSPARENT` nicht (durchklickbare Overlays von Spielen
  und Aufnahme-Tools). Die Zone beginnt an der Oberkante der Arbeitsfläche. `fullscreen` gilt außer im
  Präsentationsmodus nur, wenn das Vordergrundfenster auf dem Island-Monitor liegt.
- **`shell_open` mit Pfaden:** innerhalb von `configDir` nur Ordner und Textdateien (`.css`, `.json`, `.log`, `.toml`,
  `.txt`), damit nie ein Programm gestartet wird.
- **Wallpaper:** Data-URL wird nach Datei, Änderungszeit und Zielgröße gecacht, damit wiederholte Events denselben String
  tragen. Unlesbare Formate (HEIC …) fallen auf Windows' JPEG-Kopie `TranscodedWallpaper` zurück. `system://wallpaper`
  geht nur bei echten Änderungen raus.
- **Alt+F4** schließt die Island nie (sie ist die App); Pausenfenster nur außerhalb einer Pause.

**Schritt 2 (Medien, Audio, Anzeige, Energie)**

- **Medien:** Sitzungs-Id = AUMID (bei mehreren Sitzungen derselben App `AUMID#2` …). `appName` aus
  `AppInfo.GetFromAppUserModelId`, sonst aus der AUMID abgeleitet (`Spotify.exe` → „Spotify“). Titel und Cover werden
  nur bei `MediaPropertiesChanged` neu gelesen; Cover ≤ 300 px, nach Inhalts-Hash gecacht. `media://state` geht nur
  raus, wenn sich etwas ändert, das das Frontend nicht selbst hochrechnen kann – Timeline-Meldungen, die weniger als
  1,5 s von der hochgerechneten Position abweichen, werden verschluckt (manche Apps melden sekündlich).
- **Audio:** Ein eigener Thread besitzt alle Core-Audio-Objekte (in windows-rs nicht threadsicher); Commands schicken
  ihm Aufträge. Bursts (Lautstärketasten) werden zu einer Meldung zusammengefasst. Lautstärke auf drei Nachkommastellen
  gerundet, damit `0.62` als `0.62` zurückkommt. `kind`: Bluetooth/USB aus der Geräte-Id des verbundenen Treibers
  (Gerätetopologie), sonst aus dem Formfaktor. `audio_set_device` über `IPolicyConfig` hinter dem Cargo-Feature
  `policy-config` (Standard an); das Frontend ruft es derzeit nicht auf.
- **Helligkeit:** Eingebautes Display → WMI inkl. `WmiMonitorBrightnessEvent` (Helligkeitstasten zeigen das OSD), externes
  → DDC/CI (keine Events). Die Quelle wird je Island-Monitor einmal ermittelt. Eigene Änderungen werden mit genau dem
  angefragten Wert bestätigt; ihr WMI-Echo wird 1,5 s lang ignoriert.
- **Nachtlicht: `null`** (Kachel „Wird nicht unterstützt“). Der einzige Schalter ist ein undokumentiertes
  CloudStore-Blob, dessen Format zwischen Windows-Versionen wechselt – nach §4 „bei Unsicherheit null“.
- **Energie:** `charging` heißt „Akku vorhanden und Netzteil angeschlossen“ (auch bei 100 %), damit Einstecken immer die
  Lade-Aktivität zeigt. Energiesparen wird gelesen (`SystemStatusFlag`), lässt sich aber nicht dokumentiert schalten →
  `energySaverSupported: false`. Herunterfahren mit `EWX_POWEROFF` (ohne Schnellstart-Hybrid).

**Schritt 3 (Systemmonitor)** – ohne `sysinfo`, direkt über dokumentierte APIs:

- **CPU:** Auslastung aus `GetSystemTimes`-Deltas, Name aus der Registry (`ProcessorNameString`). `ghz` wie im
  Task-Manager: PDH `Processor Frequency` × `% Processor Performance` (`sysinfo` liefert unter Windows nur den
  Nenntakt). **RAM:** `GlobalMemoryStatusEx`, belegt = gesamt − verfügbar.
- **Netz:** `GetIfTable2`, nur Hardware-Schnittstellen, die aktiv sind; Filtertreiber (QoS, WFP) und Loopback zählen
  nicht, sonst wäre der Verkehr doppelt.
- **GPU:** Adapter = Hardware-Adapter mit dem meisten dedizierten Speicher (DXGI, Name und Größe). Auslastung = PDH
  `GPU Engine(*engtype_3D)` je Adapter summiert (max. 100). **Belegter VRAM über PDH `GPU Adapter Memory`**
  (`Dedicated Usage`, bei integrierten GPUs `Shared Usage`), nicht über DXGI `QueryVideoMemoryInfo` – das misst nur
  den eigenen Prozess. Gesamt = dedizierter bzw. bei integrierten GPUs geteilter Speicher.
- **Abos:** je Fenster gezählt; lädt ein Fenster neu oder schließt es, verfallen seine Abos (sonst liefe der Takt nach
  einem Reload ewig weiter). Takt 250 ms–10 s, erstes Sample nach 300 ms.
- **Prozesse:** Toolhelp-Snapshot + `GetProcessTimes`/`GetProcessMemoryInfo` mit `PROCESS_QUERY_LIMITED_INFORMATION`;
  Prozesse, die sich nicht öffnen lassen, fehlen. CPU-Anteil seit dem letzten Aufruf (ist der älter als 10 s, wird
  300 ms gemessen). Icons 48 px als **PNG**-Data-URL (Transparenz), nach Exe-Pfad gecacht.
- **Beenden:** `TerminateProcess`; PID 0, 4 und rice selbst → `access_denied`, unbekannte PID → `not_found`.
- Diagnose: `cargo test -p rice-win prints_a_sample -- --ignored --nocapture` gibt einen Messpunkt aus.

**Bekannt / offen**

- Das Frontend setzt `settings.island.offsetTop` noch nicht um (`--island-top` ist ein festes Token). Das Backend nutzt
  den Wert nur für die Island-Zone.
- Der Standard-Hotkey `Super+Alt+Space` kann belegt sein (z. B. PowerToys Command Palette); das Backend loggt dann eine
  Warnung, die Island funktioniert weiter.
- Wenig freier Arbeitsspeicher: Der erste Build kompiliert die große `windows`-Crate; bei „Die Auslagerungsdatei ist zu
  klein“ (os error 1455) mit weniger Jobs bauen (`$env:CARGO_BUILD_JOBS=4`).
- CSP ist noch aus (`csp: null`); sie kommt mit dem Installer (Schritt 7).
