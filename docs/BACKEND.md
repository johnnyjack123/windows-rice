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
| `app_autostart_get` / `app_autostart_set { enabled }` | `AutostartState { enabled, supported }`. Wert `rice` unter `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (Pfad zur laufenden Exe in Anführungszeichen). `enabled` heißt außerdem „im Task-Manager nicht deaktiviert“ (`…\Explorer\StartupApproved\Run`, erstes Byte ungerade = aus); Einschalten in rice hebt diese Sperre auf. Entwicklungs-Builds: `supported: false`, `set` → `not_supported` (sie bräuchten den Vite-Server). |
| `app_quit` | Beendet rice (`app.exit(0)`); dabei kommen die Windows-Banner zurück (`RunEvent::Exit`). |
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
| `power_get` | `GetSystemPowerStatus` (`battery: null`, wenn kein Akku: `BatteryFlag == 128`), Energiesparmodus über `GUID_POWER_SAVING_STATUS` lesen. `energySaverSupported` nur `true`, wenn du ihn zuverlässig schalten kannst. `updatePending` = Windows Update wartet auf einen Neustart (Windows Update Agent, `ISystemInformation::RebootRequired`). |
| `power_set_energy_saver` | Experimentell; sonst `not_supported`. |
| `power_action { action }` | `lock` → `LockWorkStation`; `sleep` → `SetSuspendState(false, false, false)`; `signout` → `ExitWindowsEx(EWX_LOGOFF)`; `restart`/`shutdown` → `ExitWindowsEx` mit `SE_SHUTDOWN_NAME`-Privileg; `update_restart`/`update_shutdown` → `InitiateShutdownW` mit `SHUTDOWN_INSTALL_UPDATES` (Windows installiert die wartenden Updates, dann Neustart bzw. Ausschalten – wie „Aktualisieren und …“ im Startmenü). |
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
| `notifications_open { id }` | Startet die sendende App bzw. holt sie nach vorn (`shell:AppsFolder\<AUMID>` per `ShellExecuteEx` ohne Fehlerdialog, vorher `AllowSetForegroundWindow`) und entfernt die Mitteilung wie ein Klick auf das Windows-Banner. Pfad-AUMIDs werden nie gestartet; Phone-Link-Unter-AUMIDs fallen auf die Haupt-App des Pakets zurück. Das eigentliche Ziel der Mitteilung (z. B. ein Chat) kann keine fremde App auslösen – Windows gibt die Aktivierung nur an den Absender. Geht es nicht: `not_supported`, die Mitteilung bleibt. → `notifications://removed`. |
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

**Schritte 1–7 umgesetzt** (Branch `feat/rust-backend`), damit alle Commands des Vertrags. Ein Test
(`commands::tests`) prüft, dass genau die Commands aus `contract.ts` registriert sind und alle gesendeten Event-Namen im
Vertrag stehen.

**Aufbau**

| Crate | Inhalt |
|---|---|
| `crates/rice-core` | Vertragstypen (serde), `applyPatch`-Semantik, Backend-Sicht der Einstellungen, Geometrie (Platzierung, Island-Zone, Hit-Test), Kanten-Zustandsautomat, Whitelist für `shell_open`, Theme-Manifest, Zeitstempel, Medien-/Geräte-Abbildungen, Regeln für Mitteilungen und Banner |
| `crates/rice-services` | `config.json` (atomar, Watcher), Theme-Ordner, entprellte Dateiüberwachung, Bild → JPEG/PNG-Data-URL, Banner ausblenden mit Backup |
| `crates/rice-caldav` | iCalendar lesen/schreiben, Wiederholungen, Zeitzonen, WebDAV-XML, HTTP-Client, Discovery, Sync, SQLite-Cache, `accounts.json` |
| `crates/rice-win` | Win32/WinRT-Wrapper: Monitore (EDID-Namen, eingebaut/extern), Wallpaper, Darstellung, Schriften, Überlappung, System-Events-Thread (inkl. Energie), Shell, GSMTC, Core Audio, Helligkeit (WMI, DDC/CI), Energie, Systemlast (PDH, DXGI, Netzwerktabelle), Prozesse, WLAN, Funk/Hotspot, Bluetooth, Anmeldeinformationen, Mitteilungen (`UserNotificationListener`), Banner-Schalter in der Registry, Konsolen-Handler |
| `src-tauri` | Fenster, Commands, Events, Hotkey, Dienste Medien/Audio/Anzeige/Energie/Systemmonitor/Netzwerk/Bluetooth/Kalender/Mitteilungen; `package/` (Sparse Package), `windows-app-manifest.xml` |

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
- **Wartende Updates:** Windows meldet keine Änderung von `RebootRequired`; rice fragt beim Start und bei jedem Öffnen
  des Kontrollzentrums (`island_set_expanded(true)`), höchstens einmal pro Minute, und sendet `power://state` nur bei
  Änderung. Updates, die heruntergeladen, aber noch nicht installiert sind, zählen nicht (das ginge nur über eine
  langsame Update-Suche).

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

**Schritt 4 (Netzwerk, Bluetooth)**

- **WLAN** über `wlanapi` mit Benachrichtigungen (Scan fertig, Verbindung fertig/fehlgeschlagen, Funk, Schnittstellen).
  `detail` = Band aus der Kanalfrequenz des verbundenen BSSID bzw. des stärksten Access Points („2,4 GHz“ mit Komma bei
  deutscher Oberfläche) + Sicherheit („WPA2“, „WPA3“ …, offen → nur das Band).
- **Standort:** `ERROR_ACCESS_DENIED` der WLAN-Abfragen → `permission: 'location_required'` bzw. beim Scan
  `permission_required`. Name und Balken der aktuellen Verbindung kommen dann aus dem Verbindungsprofil
  (`NetworkInformation`), das keine Standortfreigabe braucht.
- **Verbinden:** bekanntes Netz ohne Schlüssel → vorhandenes Profil. Sonst neues Profil (WPA2-Personal, WPA3-SAE oder
  offen; SSID als Hex, Schlüssel XML-escaped) und Verbindung; Ergebnis aus der Verbindungsbenachrichtigung (max. 15 s).
  Sicherheitsfehler (`WLAN_REASON_CODE_MSMSEC_*`) → `wrong_key`; ein neu angelegtes Profil wird bei jedem Fehlschlag
  wieder gelöscht, sonst probiert Windows es endlos. Enterprise-Netze → `failed` (Anmeldedaten nur über Windows).
- **Funk/Flugmodus** über `Windows.Devices.Radios` (`StateChanged` → `network://state` und `bluetooth://state`).
  `airplane` = alle Funkmodule aus; Flugmodus an = alle aus (vorherige Zustände gemerkt), aus = gemerkte wieder an.
  WLAN einschalten beendet den Flugmodus.
- **Hotspot:** `supported` nur, wenn `GetTetheringCapabilityFromConnectionProfile` `Enabled` meldet.
- **Bluetooth:** gekoppelte Geräte (klassisch + LE, zusammengefasst nach Container-Id), `connected` aus
  `System.Devices.Aep.IsConnected`, `kind` aus Class of Device bzw. LE-Appearance (Name verfeinert „Buds“/„AirPods“ →
  Earbuds), `battery` aus `{104EA319…} 2` der Geräteknoten – nur für verbundene Geräte (Windows behält sonst den
  letzten Stand).
- **Verbinden/Trennen** für Audiogeräte mit eigenem Bluetooth-Audiofilter (`bthenum`): `IKsControl` über das
  Filtergerät (`IMMDeviceEnumerator::GetDevice(<Filter-Id>)`, nicht über `IPart`), Property `KSPROPSETID_BtAudio`
  `ONESHOT_RECONNECT/DISCONNECT`, danach bis 10 s auf den Verbindungsstatus warten. Geteilte Offload-Filter (z. B.
  Qualcomm `aucd` auf Snapdragon) und LE-Audio-Geräte → `canConnect: false` (das Frontend öffnet die Einstellungen).
- Diagnose (nur lesend): `cargo test -p rice-win prints_wifi -- --ignored --nocapture`, ebenso `prints_connectivity`.

**Schritt 5 (Kalender, Aufgaben)**

- **Bibliotheken:** eigener iCalendar-Leser/-Schreiber (unbekannte Eigenschaften anderer Apps bleiben beim Ändern
  erhalten), `rrule` + `chrono-tz`, `quick-xml`, `rusqlite` (SQLite gebündelt) und `reqwest` mit **Windows-TLS
  (Schannel)** statt rustls – rustls/aws-lc braucht auf ARM64-Windows zusätzliche Build-Werkzeuge, und Schannel vertraut
  denselben Zertifikaten wie Windows (selbst signierte Heimserver).
- **Discovery:** Adresse ohne Schema → `https://`. Kandidaten: angegebener DAV-Pfad, `/.well-known/caldav`,
  `<Pfad>/remote.php/dav/`, die Adresse selbst. Weiterleitungen werden nur innerhalb desselben Servers verfolgt – das
  Passwort geht nie an einen anderen Host.
- **Passwort:** generischer Eintrag `rice/caldav/<accountId>` in der Windows-Anmeldeinformationsverwaltung (direkt per
  `CredWriteW`, ohne `keyring`). Konto-Id = Hash aus Benutzer und DAV-Adresse (erneutes Hinzufügen findet den Cache
  wieder), Kalender-Id = `<Konto>:<letztes Pfadsegment>`.
- **Sync:** 3 s nach dem Start, alle 5 min und per `calendar_sync`. Unveränderter Sync-Token bzw. CTag in der
  Kalenderliste → keine weitere Anfrage. Sonst `sync-collection` (abgelaufener Token → vollständiger Neuabgleich),
  Fallback CTag + ETag-Liste; Objekte per `calendar-multiget` in 50er-Paketen. Auf dem Server gelöschte Kalender
  verschwinden aus dem Cache.
- **Termine:** Serien über `rrule` (`UNTIL` wird an `DTSTART` angeglichen, wie RFC 5545 verlangt), `EXDATE`,
  verschobene Einzeltermine (`RECURRENCE-ID`), abgesagte entfallen. Zeitzonen: IANA, Windows-Namen aus
  Outlook-Einladungen (Tabelle der häufigsten), Mozilla-Präfixe; Unbekanntes gilt als Ortszeit statt zu verschwinden.
- **Aufgaben:** `id` = Pfad des Objekts auf dem Server. Sortierung: offen vor erledigt, mit Fälligkeit vor ohne, dann
  Datum und Titel. Änderungen landen im gespeicherten Text (`SEQUENCE` + 1); bei 412 werden sie auf die aktuelle
  Server-Version angewendet, Änderungen vom Handy gehen also nicht verloren.
- **Offline:** Schreibvorgänge kommen in die Warteschlange im Cache (auch „Fälligkeit entfernen“), Status `offline`;
  der nächste Sync spielt sie vor dem Abgleich aus.
- **Tests:** `crates/rice-caldav/tests/fake_server.rs` spielt Discovery, Sync und Schreibzugriffe gegen einen Server, der
  wie Nextcloud antwortet (Weiterleitung, Basic Auth, Sync-Token, ETags).

**Schritt 6 (Mitteilungen, Sparse Package)**

- **Paket-Identität:** `src-tauri/package/AppxManifest.xml` ist ein Sparse Package nach Microsofts Vorlage („packaging
  with external location“: `AllowExternalContent`, `runFullTrust`, `unvirtualizedResources`, `AppListEntry="none"`)
  plus Capability `userNotificationListener`. `rice.exe` trägt dazu ein eigenes Manifest
  (`src-tauri/windows-app-manifest.xml`, eingebettet von `build.rs`) mit dem `msix`-Element; Name, Publisher und
  Application-Id müssen in beiden gleich sein. Ohne registriertes Paket läuft rice wie bisher ohne Identität →
  `permission: 'unsupported'`, die Registry bleibt unangetastet. **Entwicklung:** eigene Identität
  `io.github.johnnyjack123.rice.dev` („rice (dev)“; `build.rs` bettet sie in Debug-Builds ein), damit installierte
  Version und `pnpm tauri dev` sich nicht gegenseitig die Identität nehmen. `scripts/sparse-package.ps1` schreibt die
  Dev-Variante des Manifests nach `%LOCALAPPDATA%\rice-dev\package` und registriert sie unsigniert mit `target\debug`
  als externem Ort (Entwicklermodus nötig, `-Remove` entfernt sie wieder). **Release:** siehe Schritt 7.
- **Listener und Threads:** `UserNotificationListener` ist nicht agil, sondern gehört dem COM-Apartment, in dem er
  entsteht. Er wird deshalb auf einem eigenen MTA-Thread erzeugt (`CoIncrementMTAUsage` hält das MTA am Leben) und nur
  von Worker-Threads benutzt; vom UI-Thread (STA) aus erzeugt, scheitern alle Aufrufe aus dem Hintergrund mit
  `RPC_E_WRONG_THREAD` (0x8001010E). Nur der Zugriffsdialog (`RequestAccessAsync`) läuft auf dem UI-Thread mit eigener
  Instanz; die Antwort kommt über einen Completed-Handler. Diagnose ohne Fenster: `rice.exe --check-notifications`
  (Identität, Zugriff, Events, Anzahl – fragt nichts an, ändert nichts).
- **Abgleich:** Zugriff (`GetAccessStatus`) wird bei jedem Abgleich neu gelesen; ist er nicht lesbar, gilt er als nicht
  erteilt. Fehler beim Lesen landen einmal je Fehlerserie im Log. Abgleich über `NotificationChanged` plus alle 10 s als
  Sicherheitsnetz (bemerkt auch entzogenen Zugriff); findet ein Abgleich Änderungen, die kein Event gemeldet hat, wird
  auf 2 s umgestellt. Die erste Liste nach dem Start bzw. nach erteiltem Zugriff geht als `notifications://state` raus,
  nicht als `posted` – sonst würde der Bestand des Infocenters als Banner-Flut in der Island landen.
- **Abbildung:** `title`/`body` = erster bzw. übrige nicht leere Texte der `ToastGeneric`-Bindung (ohne Text: App-Name
  als Titel). `appName` aus `AppInfo.DisplayInfo`, sonst `GetFromAppUserModelId`, sonst aus der AUMID abgeleitet.
  `iconUrl` = App-Logo bzw. Startmenü-Icon (`shell:AppsFolder\<AUMID>`, auch für klassische Apps), 48 px als
  PNG-Data-URL, je App gecacht. Liste neueste zuerst.
- **Banner ausblenden:** Regeln in `rice-core::notifications`, Backup-Logik in `rice-services::banners`, Registry in
  `rice-win::banners`. Geschrieben wird nur `ShowBanner` in vorhandenen App-Schlüsseln. Unberührt bleiben Apps mit
  `Enabled = 0` sowie **Wecker (`Microsoft.WindowsAlarms_…`) und Anrufe (`Windows.SystemToast.Calling…`)**, die auch bei
  verdeckter Island durchkommen sollen. Das Backup (`%APPDATA%\rice\banner-backup.json`, Originalwert je Schlüssel,
  `null` = fehlte) wird vor jedem Schreiben gesichert; lässt es sich nicht schreiben, wird nichts unterdrückt.
- **Zurücksetzen** (nur Werte, die noch 0 sind; was scheitert, bleibt im Backup für den nächsten Versuch): beim Beenden
  (`RunEvent::Exit`), bei `WM_ENDSESSION`, bei Strg+C bzw. Schließen der Konsole (Entwicklung), im Panic-Hook (danach
  blendet rice in dieser Sitzung nichts mehr aus), wenn das Setting ausgeht oder der Zugriff nicht (mehr) erteilt ist,
  bei jedem Start und per `rice.exe --restore-banners` (für den Deinstaller).
- **Neue Apps:** Bei der ersten Mitteilung einer App in dieser Sitzung wird ihr Schlüssel nachgezogen.
- **Öffnen** (`notifications_open`, nachgereicht): Ein Klick auf das Island-Banner oder ein Antippen in der Liste öffnet
  die App (Aktivitäts-Registry: neues optionales `open` an `ActivityDef`; scheitert es, öffnet das Banner wie vorher
  den Mitteilungsbereich). In der Liste ist der Titel dafür ein Knopf (Tastatur), Wischen entfernt weiterhin.

**Schritt 7 (Installer, Autostart, Einzelinstanz)**

- **Bauen:** `pnpm release` = `tauri build` mit `src-tauri/tauri.release.conf.json` (Bundling, Ressourcen, NSIS). Die
  Basiskonfiguration bündelt nicht (`bundle.active: false`), weil `tauri-build` Bundle-Ressourcen bei *jedem* Build
  kopiert und ohne gebautes Paket auch `cargo build` scheitern würde. Ergebnis:
  `target\release\bundle\nsis\rice_<Version>_<Arch>-setup.exe`.
- **Signiertes Sparse Package:** `scripts/identity-package.ps1` (läuft vor dem Kompilieren) packt `src-tauri/package`
  mit der App-Version (`x.y.z.0`), signiert es mit SignTool und exportiert den öffentlichen Teil des Zertifikats nach
  `target\identity\`. Zertifikat: selbst signiert, `CN=rice` (= Publisher im Manifest), Code-Signing-EKU, 10 Jahre,
  einmalig angelegt in `Cert:\CurrentUser\My` mit **nicht exportierbarem** privatem Schlüssel – nie als Datei, nie im
  Repo. Kostenpflichtige Alternativen (Microsoft Artifact Signing: für Einzelpersonen nur USA/Kanada) sind für ein
  privates Projekt unverhältnismäßig.
- **Installer (NSIS, pro Benutzer),** Hooks in `src-tauri/windows/hooks.nsh`:
  - *Vor der Installation:* läuft rice schon (Update), beendet `rice.exe --quit` es geordnet.
  - *Danach:* `identity.ps1 -Install` vertraut dem Zertifikat in `Cert:\LocalMachine\TrustedPeople` und registriert das
    Paket (`Add-AppxPackage -ExternalLocation <Installationsordner>`); ein vorhandenes Paket gleichen Namens wird vorher
    entfernt. **Nur der Zertifikatsschritt läuft erhöht (eine UAC-Abfrage, entfällt, wenn das Zertifikat schon
    vertraut ist):** Der Bereitstellungsdienst akzeptiert ausschließlich den Maschinenspeicher – ein Vertrauen nur für
    den Benutzer (`CurrentUser\TrustedPeople`) scheitert mit `0x800B0109`, anders als Microsofts Anleitung
    nahelegt (am echten System nachgewiesen). PowerShell wird über `Sysnative` als 64-Bit-Prozess gestartet (der
    Installer ist 32-Bit, dort fehlen die Appx-Cmdlets). Abgelehnte UAC oder ein anderer Fehler → rice läuft ohne
    Mitteilungen weiter.
  - *Autostart:* einmalig gesetzt (Merker `HKCU\Software\io.github.johnnyjack123.rice\AutostartOffered`); wer ihn in rice
    ausschaltet, behält das über Updates.
  - *Deinstallation:* `--quit`, `--restore-banners`, `identity.ps1 -Uninstall` (Paket weg; das Zertifikat entfernt eine
    weitere UAC-Abfrage aus dem Maschinenspeicher), Autostart-Werte und Merker löschen. Läuft vor Tauris eigener „App
    läuft noch“-Prüfung.
- **`rice.exe --quit`:** sucht das Systemereignis-Fenster und schickt die registrierte Nachricht `rice-quit`; rice
  beendet sich über `app.exit(0)` (Banner zurück), der Aufrufer wartet bis 5 s. Debug- und Release-Builds haben
  getrennte Fensterklassen und treffen sich nie.
- **Einzelinstanz** (`tauri-plugin-single-instance`, nur Release): ein zweiter Start öffnet die Einstellungen. In
  Entwicklungs-Builds aus, damit `pnpm tauri dev` neben der installierten Version läuft (beide haben eigene Identitäten;
  das Banner-Backup teilen sie sich – beim Entwickeln die installierte Version am besten beenden).
- **Autostart** über den `Run`-Schlüssel selbst statt `tauri-plugin-autostart`: wenige Zeilen mit dem vorhandenen
  Registry-Helfer, keine weiteren Crates, und das Verhalten bei der Task-Manager-Sperre bleibt in eigener Hand. Beim
  Start wird ein vorhandener Eintrag auf die aktuelle Exe korrigiert, ohne die Task-Manager-Einstellung anzufassen.
- **CSP:** `default-src 'self'`, `script-src 'self'`, `style-src 'self' 'unsafe-inline'` (Svelte-Stilattribute,
  Nutzer-Themes als `<style>`), `img-src 'self' data: blob:` (Wallpaper, Cover, Icons, Glasebenen), `font-src 'self'
  data:`, `connect-src ipc: http://ipc.localhost`, `object-src 'none'`. `dangerousDisableAssetCspModification:
  ["style-src"]`, weil Tauri sonst Hashes einträgt und Browser dann `'unsafe-inline'` ignorieren. `devCsp` erlaubt
  zusätzlich Vites HMR (`ws://127.0.0.1:1420`). Nutzer-Themes können damit keine Schriften oder Bilder aus dem Netz
  laden.

**Bekannt / offen**

- Ein hart beendeter Prozess (Task-Manager, `pnpm tauri dev` beim Neubau) kann die Banner nicht zurücksetzen; das
  passiert beim nächsten Start oder mit `rice.exe --restore-banners`.
- Noch am echten System zu prüfen: ob der Listener Mitteilungen klassischer Desktop-Apps (Discord, Outlook, Signal)
  sieht und ob `NotificationChanged` für rice feuert (sonst greift das 2-s-Polling automatisch).

- Das Frontend setzt `settings.island.offsetTop` noch nicht um (`--island-top` ist ein festes Token). Das Backend nutzt
  den Wert nur für die Island-Zone.
- Der Standard-Hotkey `Super+Alt+Space` kann belegt sein (z. B. PowerToys Command Palette); das Backend loggt dann eine
  Warnung, die Island funktioniert weiter.
- Wenig freier Arbeitsspeicher: Der erste Build kompiliert die große `windows`-Crate; bei „Die Auslagerungsdatei ist zu
  klein“ (os error 1455) mit weniger Jobs bauen (`$env:CARGO_BUILD_JOBS=4`).
- Der Installer ist nicht signiert: Windows SmartScreen warnt beim ersten Start („Unbekannter Herausgeber“ → „Weitere
  Informationen“ → „Trotzdem ausführen“). Das verhindert nur ein gekauftes Code-Signing-Zertifikat.
- Der Installer passt zur Architektur des Build-Rechners (hier ARM64).
