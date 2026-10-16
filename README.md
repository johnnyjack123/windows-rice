# rice

Ein Windows-Desktop-Rice: eine **Dynamic Island**, die per Klick zum Kontrollzentrum wird (Mitteilungen, Medien,
Kalender, Aufgaben, Timer, Systemmonitor, Schnelleinstellungen mit WLAN-/Bluetooth-Untermenüs). Später kommen
Taskleiste, App-Launcher und weitere Module dazu. Stack: Tauri 2 + Rust + Svelte 5.

> **Status: Phase 1 – Frontend fertig, Rust-Backend im Aufbau.** Das komplette Frontend läuft im Browser gegen ein
> Mock-Backend. Das Rust-Backend entsteht schrittweise nach [`docs/BACKEND.md`](docs/BACKEND.md) (Stand: §7); Plan und
> Roadmap [`docs/PLAN.md`](docs/PLAN.md).

## Starten

### Als Desktop-App (Tauri)

Voraussetzungen: Node 22+, pnpm, Rust (stable, MSVC), Visual Studio Build Tools mit C++-Workload, WebView2-Runtime.

```powershell
pnpm install
pnpm tauri dev
```

Startet Vite und die App. Die Island erscheint oben mittig auf dem Hauptbildschirm; Einstellungen und Daten liegen in
`%APPDATA%\rice`. Läuft `pnpm dev` schon, bricht `pnpm tauri dev` mit „Port 1420 is already in use“ ab – dann den
laufenden Server beenden oder nur das Backend starten: `cargo run -p rice`.

### Im Browser (Mock-Backend)

```powershell
pnpm install
pnpm dev
```

Dann in Chrome oder Edge:

| Seite | Adresse | Wofür |
|---|---|---|
| **Island Lab** | <http://127.0.0.1:1420/lab.html> | Island + Kontrollzentrum über einem Test-Desktop, Ereignisse auslösen |
| **Einstellungen** | <http://127.0.0.1:1420/settings.html> | Themes, Theme-Editor, Island, Kontrollzentrum, Kalender, Timer & Pomodoro, Sprache |
| **Galerie** | <http://127.0.0.1:1420/gallery.html> | Alle Bausteine in allen Zuständen, Themes und Schemata |

Lab und Einstellungen teilen sich das Mock-Backend: Eine Änderung in den Einstellungen wirkt sofort in einem offenen
Lab-Tab (wie später zwischen den beiden Fenstern).

### Tastenkürzel im Lab

| Taste | Aktion | Taste | Aktion |
|---|---|---|---|
| `1`–`5` | Theme wechseln | `Leertaste` | Kontrollzentrum öffnen/schließen |
| `D` | Hell/Dunkel | `Esc` | Schließen |
| `N` | Mitteilung | `M` | Play/Pause |
| `V` / `B` | Lautstärke / Helligkeit | `.` | Nächster Titel |
| `E` | Termin in Kürze | `T` | 5-Minuten-Timer (zeigt das Abspalten) |
| `C` | Laden | `A` | Wecker klingelt |
| `P` | Pomodoro starten / nächste Phase | `S` | Zeitlupe (1× → 0,5× → 0,2×) |
| `R` | Reduzierte Bewegung | `H` | Lab-Leiste ausblenden |

Außerdem: **Mausrad** über der Island ändert die Lautstärke, **Überfahren** der Musik-Island öffnet den Player.
**Auto-Hide testen:** Im Lab „Dahinter → Helles/Dunkles Fenster“ wählen – die Island versteckt sich. Maus oben in der
Mitte an den Rand drücken → sie kommt zurück; Bereich verlassen → sie verschwindet wieder.

**Zwei Pillen testen:** Musik läuft bereits; `T` startet einen Timer, er spaltet sich als zweite Pille ab. Über die
Musik-Pille fahren öffnet den Player, über die Timer-Pille die Schnellaktionen (+1 Min, Pause, Abbrechen); die andere
Pille weicht jeweils aus. Kontrollzentrum öffnen und schließen zeigt das erneute
Abspalten. Am deutlichsten in Zeitlupe (`S`) und mit den Themes Obsidian oder Liquid Glass.

**Pomodoro und Pausen-Vollbild:** Im Lab „Pomodoro-Pause im Vollbild“ anhaken, dann `P` (Start) und nochmal `P`
(Fokus überspringen) – die Pause legt sich über die Seite. Direkt ansehen: <http://127.0.0.1:1420/break.html?preview>
(auch `?preview=long`, `?preview=over`).

## Die fünf Standard-Themes

| | Theme | Charakter |
|---|---|---|
| 1 | **Obsidian** | Apple-nah: reines Schwarz, kräftige Federn, die zweite Aktivität spaltet sich flüssig ab |
| 2 | **Monet** | Material 3 Expressive: alle Farben aus einem Seed (Wallpaper/Album), Formen verwandeln sich je nach Zustand |
| 3 | **Liquid Glass** | Echtes Glas per WebGL: Lichtbrechung und Farbsaum am Rand, Milchglas, Randlicht folgt der Maus, Pillen verschmelzen flüssig |
| 4 | **Terminal** | Monospace, Haarlinien, Bernstein; ohne Federn, Inhalte „rendern“ sich ein |
| 5 | **Paper** | Editorial: warmes Papier, Serifen-Ziffern, Tinte und Zinnoberrot |

Jedes Theme lässt sich im **Theme-Editor** anpassen (Farben, Schriften, Radien, Tempo, Easing, Federung …) und als
eigenes Theme speichern. Eigene Themes von Hand: [`docs/THEMING.md`](docs/THEMING.md).

## Befehle

| Befehl | Was |
|---|---|
| `pnpm dev` | Entwicklungsserver auf Port 1420 |
| `pnpm check` | Typprüfung (svelte-check, Warnungen sind Fehler) |
| `pnpm test` | Unit-Tests + Smoke-Test (mountet alle Seiten headless) |
| `pnpm build` | Produktions-Build nach `dist/` |
| `pnpm verify` | alles drei nacheinander – vor jedem Commit |
| `pnpm icons` | Icon-Subset neu erzeugen (nach Änderungen an `web/src/design/icons.json`, braucht Python mit `fonttools` + `brotli`) |
| `pnpm tauri dev` | Desktop-App mit Vite-Server starten |
| `cargo clippy -- -D warnings` | Rust-Lints (Warnungen sind Fehler) |
| `cargo test` | Rust-Tests (Vertragsabgleich, Geometrie, Einstellungen, Dateien) |

## Struktur

```
docs/
  PLAN.md               Architektur, Machbarkeit, Roadmap, Entscheidungen
  DESIGN.md             verbindliche Gestaltungsregeln
  THEMING.md            Themes anpassen und bauen
  BACKEND.md            Auftrag und Schnittstelle für das Rust-Backend
web/
  *.html                Einstiegsseiten (island, settings, break, lab, gallery)
  src/
    design/             Tokens, Themes, Theme-Engine, Theme-Editor, Icons, Schriften
    motion/             Federn (Physik + CSS-Kurven)
    i18n/               Sprachkataloge Deutsch/Englisch, Formatierer
    backend/            Vertrag (contract.ts), Tauri-Anbindung, Mock-Backend, Stores
    ui/                 Bausteine (Tile, Slider, Switch, Tabs …)
    island/             Island, Aktivitäten, Auto-Hide, Timer, Pomodoro, Glas-Shader (glass/), Kontrollzentrum (panel/)
    break/              Pausen-Vollbild des Pomodoro
    settings/           Einstellungsfenster
    lab/ gallery/       Entwicklungsseiten
scripts/                Hilfsskripte (Icon-Subset)
src-tauri/              Tauri-App: Fenster, Commands, Events (Rust)
crates/
  rice-core/            Vertragstypen und Logik ohne Windows-Abhängigkeit
  rice-services/        Konfiguration, Themes, Bildkodierung
  rice-win/             Windows-API-Wrapper
```
