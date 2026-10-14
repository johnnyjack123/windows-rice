# rice — Umsetzungsplan

> Arbeitstitel „rice“ · Stand 14.10.2026 · aktuelle Phase: **1b – Rust-Backend** (Frontend fertig, läuft im Browser)
> Begleitdokumente: [`DESIGN.md`](DESIGN.md) (Gestaltungsregeln) · [`THEMING.md`](THEMING.md) (Themes bauen) ·
> [`BACKEND.md`](BACKEND.md) (Auftrag Rust-Backend) · Lab: `pnpm dev` → <http://127.0.0.1:1420/lab.html>

---

## 0. Kurzfassung

| Frage | Entscheidung |
|---|---|
| Framework | **Tauri 2** (Rust-Backend) + **Svelte 5** + TypeScript + Vite |
| Styling | Reines CSS mit **Design-Tokens** (CSS Custom Properties), kein CSS-Framework |
| Animation | Eigene **Feder-Engine** (`web/src/motion`) + aus denselben Federn erzeugte CSS-`linear()`-Kurven |
| Sprachen | Keine UI-Texte im Code: typisierte Kataloge **Deutsch + Englisch** (`web/src/i18n`), Test prüft Vollständigkeit |
| Prozessmodell | **Ein** Host-Prozess (`rice-shell`), jedes Modul (Island, später Taskleiste, Launcher) ist ein eigenes Fenster |
| Erweiterbarkeit | Rust: Services hinter einem gemeinsamen Trait. Frontend: drei Registries – **Aktivitäten**, **Panel-Bereiche**, **Kacheln** |
| Themes | Fünf Standard-Themes (Obsidian, Monet, Liquid Glass, Terminal, Paper) + eigene Themes aus `%APPDATA%\rice\themes` (Hot Reload). **Theme-Editor**: jedes Theme bringt Standardwerte mit, der Nutzer passt Farben, Schrift, Form und Bewegung an |
| Kalender/Aufgaben | **CalDAV** (VEVENT + VTODO) gegen **Nextcloud**, lokaler Cache. Das ist gleichzeitig unser „Task-Store“ |
| Arbeitsteilung | Frontend fertig (dieser Stand). Rust-Backend baut ein separater Agent nach [`BACKEND.md`](BACKEND.md); Vertrag: `web/src/backend/contract.ts` |
| Größtes Risiko | Benachrichtigungen lesen (braucht Paket-Identität) und Bluetooth-Audio verbinden (keine offizielle API) – Fallback ist jeweils „In Windows-Einstellungen öffnen“ |

---

## 1. Ziel und Leitplanken

**Vision.** Ein Windows-Desktop, der so ruhig und hochwertig wirkt wie ein gut gemachtes Android-/Apple-System: eine
Dynamic Island oben in der Mitte, die nur dann Platz einnimmt, wenn sie etwas zu sagen hat, und sich per Klick in ein
Kontrollzentrum verwandelt (Mitteilungen, Medien, Kalender, Aufgaben, Systemmonitor, Schnelleinstellungen). Später kommen
weitere Module dazu (Taskleiste, App-Launcher, …) und sprechen dieselbe Design-Sprache.

**Qualitätsziele (messbar)**

| Ziel | Messgröße |
|---|---|
| Flüssig | Island-Morph hält die Bildwiederholrate des Monitors (60/144 Hz) ohne verworfene Frames |
| Sparsam im Leerlauf | < 0,5 % CPU, keine Timer-Polls, solange nichts sichtbar ist |
| Speicher | ≤ ~150 MB für den gesamten Shell-Prozessbaum (WebView2 inklusive) – im Spike messen |
| Konsistent | Kein Literalwert in Komponenten; jede Komponente in der Galerie in allen Themes geprüft |
| Erweiterbar | Neue Kachel / Aktivität / Panel-Bereich = eine Datei + ein Registry-Eintrag |
| Anpassbar | Neues Theme ohne Code-Änderung, live umschaltbar |

**Bewusst nicht in v1:** kein Tiling-Window-Manager, kein Ersatz für den Explorer, keine Cloud-Synchronisation der
Einstellungen, keine Unterstützung für Windows 10.

---

## 2. Tech-Stack

### 2.1 Übersicht

| Schicht | Wahl | Warum |
|---|---|---|
| Shell | Tauri 2 | Kleine Binaries, native Fenster, Rust-Zugriff auf Win32/WinRT; Web-Frontend für maximale Gestaltungsfreiheit |
| Rendering | WebView2 (Chromium) | Immer aktueller Chromium → modernes CSS sicher nutzbar: `linear()`, OKLCH-Relativfarben, `@property`, `:has()`, Container Queries, View Transitions |
| Windows-APIs | Crate `windows` (windows-rs) | Offizielle Microsoft-Bindings für Win32 **und** WinRT |
| Async | `tokio` | Hintergrund-Watcher, Netzwerk (CalDAV) |
| UI | Svelte 5 (Runes) + TypeScript (strict) | Siehe 2.2 |
| Build | Vite (Multi-Page: ein Einstieg pro Fenster), pnpm-Workspaces, Cargo-Workspace | |
| Typsichere IPC | `tauri-specta` | Erzeugt TS-Typen für Commands und Events aus Rust – Frontend und Backend können nicht auseinanderlaufen |
| Icons | Material Symbols (variable Font, beim Build auf genutzte Icons reduziert) | Achsen Füllung/Strichstärke/Grad sind über Tokens steuerbar und **animierbar** |
| Schriften | Lokal gebündelt (woff2) | Offline-fähig, keine Requests beim Start |

### 2.2 Warum Svelte 5 (und nicht React/Angular/…)

Die Island besteht aus mehreren kleinen Fenstern, die **dauerhaft** laufen. Dafür zählen: wenig Laufzeit-Overhead,
präzise Kontrolle über DOM und Animation, Scoped CSS, das sich gut mit Tokens verträgt, und eine Syntax nahe an
HTML/CSS (du hast gesagt, dass du dort am liebsten arbeitest).

| | Svelte 5 | React (+ Motion) | Angular | SolidJS | Vue 3 |
|---|---|---|---|---|---|
| Laufzeit-Overhead | sehr gering (kompiliert) | mittel (VDOM) | hoch | sehr gering | gering–mittel |
| Nähe zu HTML/CSS | sehr hoch | mittel (JSX) | mittel | mittel (JSX) | hoch |
| Scoped CSS eingebaut | ja | nein | ja | nein | ja |
| Animation | Springs/Transitions eingebaut | Motion ist Spitzenklasse | schwach | Bibliotheken | gut |
| Ökosystem | gut | sehr groß | groß | klein | groß |
| Passt zu dauerhaft laufenden Mini-Fenstern | **sehr gut** | gut | schlecht | sehr gut | gut |

React wäre die zweite Wahl (vor allem wegen Motion). Den Vorsprung von Motion gleichen wir aus, weil die
entscheidende Animation – das Morphen der Island – ohnehin von unserer eigenen Feder-Engine kommt (siehe Lab). Angular
ist für viele kleine, dauerhaft laufende Fenster zu schwer.

**Verworfen:** Electron (Speicherbedarf pro Fenster, eigener Chromium), WinUI 3 / WPF (weniger Freiheit bei Formen,
Morphing und Theming, kein CSS), Rainmeter/YASB (nicht tief genug für eigene Windows-Integration).

**Referenzprojekte zum Lernen** (keinen Code übernehmen, ohne die Lizenz zu prüfen):
[Seelen UI](https://github.com/eythaann/Seelen-UI) (Tauri + Rust, kompletter Desktop-Ersatz; die Lizenzangaben in den
Quellen widersprechen sich), [Zebar](https://github.com/glzr-io/zebar) (Tauri-basierte Statusleisten mit HTML-Widgets).

---

## 3. Architektur

### 3.1 Überblick

```
┌──────────────────────────── rice-shell.exe (ein Prozess) ─────────────────────────────┐
│                                                                                      │
│  Rust-Kern                                                                           │
│  ┌────────────┐  ┌──────────────────────── Services ──────────────────────────────┐  │
│  │ Config     │  │ media  audio  display  power  network  bluetooth  system       │  │
│  │ (rice.toml)│  │ notifications  calendar  tasks  theme  (später: windows, apps) │  │
│  ├────────────┤  └───────────────▲───────────────────────────┬───────────────────┘  │
│  │ Theme-     │                  │ Commands (invoke)         │ Events (emit)        │
│  │ Loader     │  ┌───────────────┴───────────────────────────▼───────────────────┐  │
│  ├────────────┤  │ IPC-Schicht (tauri-specta: generierte TS-Typen)               │  │
│  │ Window-    │  └───────────────▲───────────────────────────┬───────────────────┘  │
│  │ Manager    │                  │                           │                      │
│  │ (Widgets)  │   WebView2 ──────┼───────────────────────────┼──────────────────    │
│  └────────────┘  ┌───────────────┴───────┐  ┌───────────────┴───────┐  ┌─────────┐  │
│                  │ Fenster: island        │  │ Fenster: bar (später) │  │ settings│  │
│                  │ Svelte-App             │  │ Svelte-App            │  │         │  │
│                  │ @rice/ui · motion ·    │  │ dieselben Pakete      │  │         │  │
│                  │ design · services      │  │                       │  │         │  │
│                  └───────────────────────┘  └───────────────────────┘  └─────────┘  │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Prozess- und Fenstermodell

- **Ein Prozess, mehrere Fenster.** Alle Module laufen in `rice-shell`. Fenster einer Tauri-App teilen sich eine
  WebView2-Umgebung (Browser-Prozess, GPU-Prozess). Jedes weitere Fenster kostet also deutlich weniger als ein
  zusätzliches Programm. Services existieren nur einmal: der Medien-Watcher versorgt Island und Taskleiste gleichzeitig.
- **Ein Fenster pro Modul.** Jedes Modul hat seine eigene Vite-Seite (`island.html`, `bar.html`, …). Fällt ein Modul
  aus, laufen die anderen weiter. Module lassen sich einzeln abschalten.
- **Lazy Services.** Ein Service startet erst, wenn ein Fenster ihn abonniert. Teure Abfragen (z. B. Prozessliste)
  laufen nur, solange die zugehörige Ansicht sichtbar ist.

### 3.3 Rust-Kern: Services

Jeder Service ist ein eigenes Modul (später ggf. eigene Crate) mit einheitlicher Form:

```rust
/// Contract every backend service implements.
#[async_trait]
pub trait Service: Send + Sync + 'static {
    /// Stable id, also the event namespace ("media", "network", ...).
    const ID: &'static str;
    /// Typed snapshot sent to the UI (generated as a TS type via specta).
    type State: Serialize + specta::Type + Clone;

    /// Starts watchers. Called when the first window subscribes.
    async fn start(&self, ctx: ServiceContext) -> anyhow::Result<()>;
    /// Stops watchers when the last subscriber is gone.
    async fn stop(&self);
    /// Current state for late subscribers.
    fn snapshot(&self) -> Self::State;
}
```

**Konventionen**
- Events: `<service>://state` (kompletter, kleiner Snapshot) und `<service>://<ereignis>` (z. B. `notifications://posted`).
- Commands: `<service>_<verb>` (z. B. `media_play_pause`, `network_connect`). Alle sind über `tauri-specta` typisiert.
- Push statt Poll: wo Windows Callbacks bietet (Lautstärke, Medien, Netzwerk, Akku), werden sie genutzt. Gepollt wird
  nur, wo es nicht anders geht (CPU/GPU-Last), und nur bei sichtbarer Ansicht.
- Fehler sind Teil des Zustands (`"permission_required"`, `"unavailable"`), damit die UI sie gestalten kann – z. B.
  „WLAN-Liste braucht die Standortfreigabe → Einstellungen öffnen“.

| Service | Aufgaben |
|---|---|
| `media` | Aktive Sitzungen, Titel/Interpret/Cover, Fortschritt, Steuerung; Coverfarbe → `--media-color` |
| `audio` | Lautstärke/Stumm, Standard-Ausgabegerät, Geräteliste, Lautstärke pro App |
| `display` | Helligkeit (intern + extern), Nachtlicht (experimentell) |
| `power` | Akku, Laden, Energiesparmodus |
| `network` | WLAN (Scan, Verbinden, Trennen), Funk ein/aus, Hotspot |
| `bluetooth` | Funk ein/aus, gekoppelte Geräte, Verbindungsstatus, Akkustand, Verbinden |
| `system` | CPU/RAM/GPU/Netz/Datenträger, Prozesse, Beenden |
| `notifications` | Windows-Mitteilungen lesen, entfernen; eigener „Nicht stören“-Modus |
| `calendar` | CalDAV-Konten, Synchronisation, lokaler Cache, Erinnerungen → Island-Aktivität |
| `tasks` | VTODO über CalDAV (Provider-Trait, später ggf. Microsoft To Do) |
| `theme` | Themes laden/überwachen, Wallpaper vorbereiten (Vorab-Blur, Seed-Farbe), System hell/dunkel |
| `config` | `rice.toml` laden, validieren, überwachen |

**Crates (Vorschlag):** `windows`, `tokio`, `serde`, `toml`, `notify` (Dateiüberwachung), `sysinfo`, `keyring`
(Windows-Anmeldeinformationsverwaltung), `reqwest` (rustls), `quick-xml`, `icalendar`, `rrule` (Wiederholungen),
`rusqlite` (Cache), `image` + [`material-colors`](https://docs.rs/material-colors) (Seed-Farbe aus Wallpaper/Cover),
`tauri-specta`, `tracing`. Tauri-Plugins: `autostart`, `single-instance`, `global-shortcut`, `log`.

### 3.4 Frontend-Module (umgesetzt)

Eine Vite-App, eine HTML-Seite pro Fenster (`island.html`, `settings.html`; dazu die Dev-Seiten `lab.html` und
`gallery.html`). Statt separater npm-Pakete gibt es klar getrennte Ordner mit Pfad-Aliasen – spätere Module
(Taskleiste, Launcher) werden weitere Seiten derselben App und nutzen dieselben Module.

| Ordner (Alias) | Inhalt |
|---|---|
| `web/src/design` (`$design`) | `tokens.css` (Vertrag), `themes/*.css`, Schriften, Icon-Subset, Theme-Engine, Theme-Editor-Modell, Farb- und Wallpaper-Helfer |
| `web/src/motion` (`$motion`) | Feder-Engine (`SpringValue`, `SpringSystem`, `springToLinear`) |
| `web/src/i18n` (`$i18n`) | Kataloge `de.ts` (Referenz) und `en.ts`, reaktive Sprache, Formatierer |
| `web/src/ui` (`$ui`) | Primitive ohne Fachlogik: `Tile`, `SmallTile`, `Slider`, `Switch`, `Tabs`, `Segmented`, `Range`, `ColorInput`, `Progress`, `Check`, `AppIcon`, `Art`, `Equalizer`, `Spark`, `Battery`, `Empty`, `Banner`, `Icon` + `ui.css` |
| `web/src/backend` (`$backend`) | Vertrag `contract.ts`, Transports (Tauri/Mock), reaktive Stores je Service, Einstellungs-Defaults und Merge-Logik, simuliertes Backend |
| `web/src/island` (`$island`) | Island: Controller, Aktivitäten-Registry, Views, Auto-Hide, Timer/Stoppuhr/Wecker, Panel mit Kacheln, Drill-ins und Bereichen |
| `web/src/settings` (`$settings`) | Einstellungsfenster inkl. Theme-Editor |
| `web/src/lab`, `web/src/gallery` | Dev-Werkzeuge |

Regel aus dem Design-System: **Fachkomponenten** bauen nur auf `$ui` auf und greifen nie an den Tokens vorbei.

### 3.5 Die drei Erweiterungspunkte (umgesetzt)

| Erweiterungspunkt | Datei | So kommt etwas Neues dazu |
|---|---|---|
| Aktivitäten (was die Island zeigen kann) | `island/activities.svelte.ts` | `registry.define({ id, priority, view, compact, bubble?, expandedView?, section?, transient?, critical?, label })`, dann `registry.show(id, { ttl })` / `hide(id)` aus der eigenen Datenquelle |
| Panel-Bereiche (Tabs) | `island/panel/sections.ts` | Eintrag `{ id, label, icon, badge?, component }`; Reihenfolge/Sichtbarkeit in `settings.panel.sections` |
| Kacheln | `island/panel/tiles.ts` | Eintrag `{ id, label, icon, sub, on, available, toggle, page? }`; Auswahl/Reihenfolge in `settings.panel.tilesLarge/tilesSmall` |

Die eingebauten Aktivitäten (Musik, Mitteilung, OSD, Laden, Termin, Timer, Stoppuhr, Wecker, abgelaufener Timer) und
der Timer-Bereich wurden genau so hinzugefügt – das ist der Beweis, dass die Erweiterungspunkte tragen.

### 3.6 Konfiguration

Das Backend speichert ein **opakes JSON-Objekt** (`%APPDATA%
ice\config.json`) und mergt Patches (Objekte
schlüsselweise, Arrays ersetzen, `null` löscht). Defaults, Typprüfung und Migrationen liegen an **einer** Stelle im
Frontend (`web/src/backend/settings.ts`). Geheimnisse (CalDAV-Passwörter) liegen nie in der Datei, sondern in der
Windows-Anmeldeinformationsverwaltung. Schema: Typ `Settings` in `contract.ts`.

### 3.7 Verzeichnisstruktur

```
rice/
├─ package.json, vite.config.ts …   Frontend-Tooling (pnpm, Vite 8, Svelte 5, TS 6, Vitest)
├─ web/                             Frontend (Vite-Root)
│  ├─ island.html, settings.html    Fenster der App
│  ├─ lab.html, gallery.html        Dev-Werkzeuge
│  └─ src/…                         Module, siehe 3.4
├─ src-tauri/                       ← Rust-Backend (folgt, siehe BACKEND.md)
├─ crates/                          ← optional: rice-core, rice-win, rice-services
├─ scripts/fetch-icons.mjs          lädt das Icon-Subset (pnpm icons)
└─ docs/                            PLAN, DESIGN, THEMING, BACKEND
```

---

## 4. Die Island-Fenstertechnik (kritischster Teil)

Die Animationen im Lab funktionieren nur dann genauso im echten Desktop, wenn das Fenster richtig gebaut ist.

1. **Festes, transparentes Fenster statt Fenster-Resize.** Das Fenster ist so groß wie das maximal geöffnete Panel plus
   Schattenrand und sitzt fest oben mittig. Die Island morpht **innerhalb** der WebView (wie im Lab). Fenster bei jedem
   Frame neu zu skalieren ruckelt unter Windows sichtbar; das machen wir nicht.
   Tauri: `transparent: true`, `decorations: false`, `shadow: false`, `alwaysOnTop: true`, `skipTaskbar: true`,
   `resizable: false`.
2. **Klicks durch die transparente Fläche.** `set_ignore_cursor_events(true)` macht das ganze Fenster durchklickbar.
   Ein Rust-Thread prüft ~60× pro Sekunde `GetCursorPos` gegen das aktuelle Island-Rechteck (vom Frontend beim
   Morph-Start und -Ende gemeldet). Liegt der Cursor darüber, wird das Fenster klickbar, sonst nicht. → **Spike S1.**
3. **Kein Fokus-Klau.** Kompakt hat das Fenster `WS_EX_NOACTIVATE`: Hover und Klicks stehlen dem aktiven Programm nicht
   den Fokus. Aufgeklappt wird es aktiviert, damit Tastatur und Eingaben (Aufgabe anlegen, WLAN-Passwort) funktionieren.
   Verliert es den Fokus, klappt es zu (= „Klick daneben schließt“).
4. **Vollbild.** `SHQueryUserNotificationState` erkennt Vollbild-D3D und Präsentationen; dann blendet sich die Island
   aus (konfigurierbar). Exklusives Vollbild lässt sich ohnehin nicht überlagern.
5. **Monitore und DPI.** Per-Monitor-DPI v2; alle Maße sind CSS-px und skalieren. Monitorwahl über die Konfiguration,
   eine Island pro Monitor ist später möglich.
6. **Material.** Ein geformtes, transparentes Fenster kann unter Windows **nicht live weichzeichnen, was dahinter
   liegt** (Acrylic/Mica gelten immer für das ganze Rechteck; Fensterregionen sind nicht kantengeglättet). Deshalb:
   - Standard sind deckende Flächen (wie die echte Dynamic Island).
   - Glas-Themes bekommen ein **vorab weichgezeichnetes Wallpaper**, das der `theme`-Service einmal pro Wallpaperwechsel
     rendert und bildschirmgenau ausrichtet. Das Lab macht es bereits genauso. Fenster hinter der Island werden dabei
     nicht „gebrochen“, das Lab zeigt diese Grenze ehrlich (Hintergrund „Helles Fenster“).
7. **Windows-eigene Einblendungen.** Lautstärke- und Helligkeits-OSD von Windows erscheinen weiterhin. Sie lassen sich
   nur über inoffizielle Fenster-Tricks verstecken → optional, experimentell.

**Performance-Regeln für die WebView** (gelten für alle Fenster, stehen auch in DESIGN.md):
- Animiert werden `transform`, `opacity`, `filter`, Farbe. Größen nur am kleinen Island-Shell, dessen Inhalt eine feste
  Endgröße hat. Der Inhalt wird beim Morph also nicht neu umbrochen.
- Kein `backdrop-filter` über große Flächen; Glas nutzt vorgerenderte Bilder.
- Keine Endlos-Animationen außer bei echtem Fortschritt bzw. laufender Wiedergabe (Equalizer).
- Bei verborgener Island (Vollbild, Sperrbildschirm) werden Ticks und Polls pausiert.

---

## 5. Windows-Integration: Machbarkeit pro Feature

✅ offiziell und stabil · ⚠️ machbar mit Einschränkung/Workaround · ❌ nicht sinnvoll machbar → Fallback

| Feature | API / Weg | Status | Anmerkungen |
|---|---|---|---|
| Medien (Titel, Cover, Steuerung) | WinRT `GlobalSystemMediaTransportControlsSessionManager` | ✅ | Spotify, Browser, Media Player usw.; Cover als Stream; Fortschritt teils ungenau (App-abhängig) |
| Lautstärke, Stumm | Core Audio `IAudioEndpointVolume` (+ Callback) | ✅ | Push-Events, sehr zuverlässig |
| Lautstärke pro App | `IAudioSessionManager2` | ✅ | |
| Ausgabegerät wechseln | `IPolicyConfig` | ⚠️ | Inoffizielle COM-Schnittstelle, seit Jahren stabil und weit verbreitet |
| Helligkeit intern | WMI `WmiMonitorBrightnessMethods` | ✅ | Laptops |
| Helligkeit extern | DDC/CI (`SetMonitorBrightness`, dxva2) | ⚠️ | Monitorabhängig, manche reagieren träge |
| Nachtlicht | Registry-Blob | ⚠️ | Inoffiziell und versionsabhängig → als experimentell kennzeichnen |
| Akku, Laden | `GetSystemPowerStatus` / WinRT `Battery` | ✅ | |
| Energiesparmodus | Power-Overlay-API | ⚠️ | Zum Teil undokumentiert; Fallback: `ms-settings:batterysaver` |
| WLAN an/aus | WinRT `Windows.Devices.Radios` | ✅ | |
| WLAN-Liste, Verbinden | Native WLAN-API (`wlanapi`) | ⚠️ | Seit **Windows 11 24H2 nur mit Standortfreigabe** („Desktop-Apps Zugriff auf den Standort erlauben“) – UI muss das erklären ([Quelle](https://kb.devicetrust.com/changes-to-for-wi-fi-access-and-location-windows-11-24h2), [Beispiel](https://netsetman.com/support/viewtopic.php?pid=4861)). Neue Netze mit Passwort über Profil-XML |
| Mobiler Hotspot | WinRT `NetworkOperatorTetheringManager` | ⚠️ | Im Spike prüfen, ob es ohne Paket-Capabilities geht |
| Flugmodus | Alle Funkmodule über Radios-API schalten | ⚠️ | Der „echte“ Systemschalter ist nicht öffentlich; Funk-Toggle ist praktisch gleichwertig |
| Bluetooth an/aus | Radios-API | ✅ | |
| BT-Geräte, Status, Akku | `Windows.Devices.Enumeration` + Geräteeigenschaften | ✅/⚠️ | Akku zuverlässig vor allem bei BLE und Headsets mit HFP-Akkuanzeige |
| BT-Audio verbinden/trennen | Kernel-Streaming-Property `KSPROPERTY_ONESHOT_RECONNECT/DISCONNECT` | ⚠️ | Keine offizielle API; Ansatz von Tools wie ToothTray. Fallback: `ms-settings:bluetooth` |
| Neues BT-Gerät koppeln | `DeviceInformation.Pairing` | ⚠️ | Machbar, aber viele Sonderfälle (PIN, SSP) → zuerst Deep-Link zu den Einstellungen |
| Mitteilungen lesen | WinRT `UserNotificationListener` | ⚠️ | Braucht **Paket-Identität** → Sparse Package („packaging with external location“, ab Win10 2004) ([Microsoft](https://learn.microsoft.com/en-my/WINDOWS/APPS/desktop/modernize/grant-identity-to-nonpackaged-apps)) + Nutzerfreigabe. Lesen und Entfernen gehen; Aktions-Buttons fremder Mitteilungen lassen sich **nicht** auslösen |
| Nicht stören (Windows) | – | ❌ | Keine öffentliche API → eigener Modus, der Island-Banner unterdrückt; Windows-Banner pro App in den Einstellungen abschaltbar |
| CPU, RAM, Netz, Datenträger | `sysinfo` | ✅ | |
| GPU-Last, VRAM | PDH-Zähler „GPU Engine“, DXGI `QueryVideoMemoryInfo` | ✅ | |
| Temperaturen | – | ❌/⚠️ | Nur mit Treiber/Fremdtool (LibreHardwareMonitor) → optionales Plugin |
| Prozess beenden | `TerminateProcess` | ✅/⚠️ | Erhöhte Prozesse nur mit Adminrechten → UI zeigt das an |
| Kalender | CalDAV (eigener Client oder [`libdav`](https://docs.rs/crate/libdav/0.8.0)) | ✅ | Nextcloud, iCloud (App-Passwort), Radicale, Baïkal ✅ · Google ⚠️ (OAuth) · Microsoft 365 ❌ (kein CalDAV → später Graph-Provider). `libdav` steht unter EUPL-1.2 |
| Aufgaben | CalDAV VTODO | ✅ | Siehe 5.1 |
| Vollbild erkennen | `SHQueryUserNotificationState` | ✅ | |
| Autostart, Einzelinstanz | Tauri-Plugins | ✅ | |
| Später: Taskleiste | AppBar (`SHAppBarMessage`), `EnumWindows` + WinEvent-Hooks | ✅/⚠️ | Windows-Taskleiste ausblenden ist inoffiziell (Explorer zeigt sie gern wieder); Tray-Icons spiegeln ist schwierig |
| Später: App-Launcher | Startmenü-Verknüpfungen + `shell:AppsFolder` (UWP) | ✅ | Win-Taste abfangen erfordert einen Low-Level-Keyboard-Hook ⚠️ |

**Spikes (Backend-Schritt 1)** – je ein kleiner Wegwerf-Beweis, bevor Zeit in UI fließt:

| Spike | Frage | Erfolg wenn |
|---|---|---|
| S1 Fenster | Klick-durch, kein Fokus-Klau, Always-on-Top, DPI | Desktop-Icons unter der transparenten Fläche bleiben klickbar, Fokus bleibt beim aktiven Programm |
| S2 Performance | Morph in transparenter WebView2 bei 144 Hz | Keine Frame-Drops im Performance-Profil, CPU im Leerlauf ≈ 0 |
| S3 Identität | Sparse Package + `UserNotificationListener` | Mitteilungen einer Test-App erscheinen im Log |
| S4 WLAN | Scan unter 24H2 inkl. Standortdialog | Netzwerkliste mit Signalstärke, sauberer Fehlerzustand ohne Freigabe |
| S5 Medien | GSMTC inkl. Cover und Farbe | Titel/Cover von Spotify und Browser live |

### 5.1 „Gibt es auf Windows einen Task-Store wie auf Android?“

Kurz: **Nein.** Und auch auf Android ist es kein Systemdienst. Dort stellen Apps wie *OpenTasks* einen Speicher
bereit, den *DAVx⁵*, *tasks.org* oder *jtx Board* nutzen, und synchronisiert wird über **CalDAV (VTODO)**. Windows hat
nichts Vergleichbares; am nächsten kommt *Microsoft To Do* über die Graph-API (nur mit Microsoft-Konto).

**Empfehlung:** Aufgaben als **CalDAV-VTODO auf demselben Server wie der Kalender**. Damit sind sie automatisch synchron
mit Android (DAVx⁵ + tasks.org/jtx Board), Thunderbird usw. Unser `tasks`-Service hält einen lokalen SQLite-Cache
(offline-fähig, schnell) und ist damit **unser eigener Task-Store**, den später auch Taskleiste oder Launcher nutzen.
Ein Provider-Trait hält die Tür für Microsoft To Do offen.

---

## 6. Design-System und Theming (Kurzfassung)

Die Details stehen in [`DESIGN.md`](DESIGN.md) und [`THEMING.md`](THEMING.md). Die
Kernideen:

- **Ein Vertrag, viele Gesichter.** `web/src/design/tokens.css` definiert *jedes* Token. Themes überschreiben Werte,
  nie Namen. Komponenten benutzen nur Tokens. Die fünf Lab-Konzepte sind derselbe Code mit verschiedenen Tokens; das ist
  der Beweis, dass es funktioniert.
- **Theme-Achsen:** Farbe · Typografie · Dichte · Form · Trennung (Schatten/Haarlinie/Fläche) · Material
  (deckend/Glas) · Ikonografie · Bewegung (Federn, Dauern, Choreografie).
- **Dynamische Farbe:** Seed aus dem Wallpaper (im Frontend aus dem Bild gerechnet, Windows-Akzent als Rückfall) oder
  dem Albumcover; Paletten entstehen per OKLCH-Relativfarben direkt im CSS. Komponenten wie die Medienkarte haben eigene
  Tokens (`--media-*`), die ein Theme aus `--media-color` ableiten kann (Monet).
- **Theme-Pakete in der App:** `%APPDATA%\rice\themes\<id>\theme.toml` (Name, Autor, Version, Schemata, Basis-Theme) +
  `theme.css`. Das Backend überwacht den Ordner → Hot Reload. Die fünf Standard-Themes sind ins Frontend gebündelt.
- **Hell/Dunkel „auto“** folgt Windows (`AppsUseLightTheme` + `UISettings.ColorValuesChanged`).

---

## 7. Roadmap

Größen: S ≈ wenige Abende, M ≈ ein bis zwei Wochenenden, L ≈ mehrere Wochenenden.

| Phase | Inhalt | Stand |
|---|---|---|
| **0 Design Lab** | Fünf Konzepte, Tokens, Themes, Galerie, Regeln | ✅ erledigt (13.10.) – alle fünf bleiben Standard-Themes |
| **1a Frontend** | Svelte-App: Island (Morph, Aktivitäten, Split-Bubble mit flüssiger Brücke, Auto-Hide mit Kanten-Reveal), Kontrollzentrum (Kacheln, WLAN-/Bluetooth-Seiten, Medien, Mitteilungen, Kalender, Aufgaben, Timer/Stoppuhr/Wecker, System), Einstellungen mit Theme-Editor, DE/EN, Vertrag + simuliertes Backend, Tests | ✅ erledigt (14.10.) |
| **1b Rust-Backend** | Nach [`BACKEND.md`](BACKEND.md) §6, Schritte 1–7 (Fenster & Klick-durch → Medien/Audio → System → Netz/Bluetooth → CalDAV → Mitteilungen → Installer) | ⏳ nächster Schritt (separater Agent) |
| **2 Politur** | Feinschliff nach echtem Desktop-Test, Performance-Pass, Playwright-Screenshots der Galerie als visuelle Regression | offen |
| **3+ Weitere Module** | Taskleiste, App-Launcher, Desktop-Widgets … als weitere Seiten derselben App | offen |

## 8. Qualitätssicherung

- **Design:** Jede Komponente steht in der Galerie, in allen Themes × Schemata. Review-Checkliste aus `DESIGN.md` §13
  vor jedem Merge. Ab Phase 2 erzeugt Playwright (gleiche Engine wie WebView2) Screenshots der Galerie pro Theme als
  visuelle Regressionstests.
- **Frontend:** TypeScript strict, `svelte-check`, ESLint, Prettier, Vitest für Logik (Arbitrierung, Federn,
  Formatierung). JSDoc an allen Exporten, Kommentare auf Englisch, geschweifte Klammern immer.
- **Rust:** `rustfmt`, `clippy -D warnings`, Unit-Tests für Parser (ICS, CalDAV-XML, WLAN-Profile), dünne `rice-win`-
  Wrapper, damit Fachlogik ohne Windows testbar bleibt.
- **Performance:** Idle-CPU und RAM werden pro Release gemessen (Abschnitt 1); ein Frame-Profil des Morphs gehört zur
  Definition of Done von Backend-Schritt 1 und Phase 2.
- **Entscheidungen** landen als kurze ADRs in `docs/adr/` (Kontext, Entscheidung, Konsequenzen).

---

## 9. Risiken

| Risiko | Auswirkung | Gegenmaßnahme |
|---|---|---|
| Klick-durch/Fokus verhalten sich anders als erwartet | Island fühlt sich „im Weg“ an | Spike S1 zuerst; Fallback: Fenster nur zwischen Zuständen (nicht pro Frame) vergrößern |
| Paket-Identität für Mitteilungen ist umständlich (Signatur, Installer) | Backend-Schritt 6 verzögert sich | Spike S3 früh; Mitteilungen sind das letzte Feature, alles andere geht ohne |
| WLAN-API ohne Standortfreigabe leer (24H2) | Netzwerkliste fehlt | Expliziter Zustand `permission_required` mit Erklärung und Deep-Link |
| BT-Audio-Verbinden ohne offizielle API | Kachel kann nicht alles | KS-Property-Weg, sonst Deep-Link; klar gekennzeichnet |
| Inoffizielle APIs brechen mit Windows-Updates (Nachtlicht, IPolicyConfig) | Einzelne Kacheln fallen aus | Pro Feature Feature-Flag + Selbsttest beim Start; Kachel deaktiviert sich mit Hinweis |
| Design driftet über die Zeit | Wirkt „zusammengewürfelt“ | Token-Vertrag, Galerie als Pflicht, Review-Checkliste, `CLAUDE.md`-Regeln |
| WebView2-Speicherbedarf | Höher als nativ | Ein Prozess für alle Module, Messung im Spike, Lazy Services |

---

## 10. Entscheidungen

| Datum | Entscheidung |
|---|---|
| 13.10. | Stack Tauri 2 + Rust + Svelte 5; Design-Tokens als CSS Custom Properties |
| 14.10. | Alle fünf Konzepte bleiben als Standard-Themes; eigene Themes jederzeit möglich |
| 14.10. | Theme-Editor mit Standardwerten pro Theme: Hintergrund, Text, Akzent, Schriften, Textgröße, Island-/Panel-/Kachel-Rundung, Dichte, Tempo, Federung, Easing, Weichzeichnen, flüssiges Abspalten, Icon-Stil/-Stärke |
| 14.10. | Alle UI-Texte über Kataloge, Deutsch und Englisch von Anfang an |
| 14.10. | Island versteckt sich, wenn ein Fenster die Island-Zone überlappt; Maus oben mittig an die Kante → kommt zurück; kurzlebige Ereignisse blenden kurz ein |
| 14.10. | Untermenüs, die zu frickelig werden (BT koppeln, Spezialfälle), öffnen die passende Windows-Einstellungsseite |
| 14.10. | Timer-Menü mit Timern (parallel), Stoppuhr und Wecker |
| 14.10. | Kalender und Aufgaben: Nextcloud per CalDAV |
| 14.10. | Git-Repository; Frontend und Backend getrennt, Rust-Backend durch einen eigenen Agenten nach `BACKEND.md` |

**Noch offen:** Island auf mehreren Monitoren gleichzeitig? · Windows-eigene Mitteilungsbanner langfristig abschalten?
· Endgültiger Name statt „rice“.

---

## 11. Quellen

- Windows 11 24H2 – WLAN und Standort: [DeviceTrust KB](https://kb.devicetrust.com/changes-to-for-wi-fi-access-and-location-windows-11-24h2),
  [Citrix Docs](https://docs.citrix.com/en-us/device-trust/25-7/kb/properties/location_and_network_improvements_in_windows11_24h2),
  [NetSetMan-Forum](https://netsetman.com/support/viewtopic.php?pid=4861)
- Paket-Identität für nicht gepackte Apps: [Microsoft Learn](https://learn.microsoft.com/en-my/WINDOWS/APPS/desktop/modernize/grant-identity-to-nonpackaged-apps),
  [Windows Developer Blog](https://blogs.windows.com/windowsdeveloper/2019/10/29/identity-registration-and-activation-of-non-packaged-win32-apps/)
- Rust: [`material-colors`](https://docs.rs/material-colors), [`libdav`](https://docs.rs/crate/libdav/0.8.0)
- Referenzprojekte: [Seelen UI](https://github.com/eythaann/Seelen-UI), [Zebar](https://github.com/glzr-io/zebar)
