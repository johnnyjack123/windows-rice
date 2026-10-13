# rice

Ein Windows-Desktop-Rice: eine **Dynamic Island**, die per Klick zum Kontrollzentrum wird (Mitteilungen, Medien,
Kalender, Aufgaben, Systemmonitor, Schnelleinstellungen mit WLAN-/Bluetooth-Untermenüs). Später kommen Taskleiste,
App-Launcher und weitere Module dazu. Geplanter Stack: Tauri 2 + Rust + Svelte 5.

> **Status: Phase 0 – Design Lab.** Es gibt noch keine Desktop-App, sondern Browser-Mocks und das Design-System.
> Plan und Roadmap: [`docs/PLAN.md`](docs/PLAN.md)

## Island Lab öffnen

Empfohlen ist ein kleiner lokaler Server (das Glas-Theme braucht ihn für das Vorrendern des Wallpapers):

```powershell
python -m http.server 5173 --bind 127.0.0.1
```

Dann in Chrome oder Edge: <http://127.0.0.1:5173/mocks/island/>. Ein Doppelklick auf `mocks/island/index.html`
funktioniert für die meisten Dinge auch. Schriften und Icons kommen von Google Fonts, dafür braucht es Internet.

| Taste | Aktion | Taste | Aktion |
|---|---|---|---|
| `1`–`5` | Konzept wechseln | `Leertaste` | Kontrollzentrum öffnen/schließen |
| `D` | Hell/Dunkel | `Esc` | Schließen |
| `N` | Mitteilung | `M` | Play/Pause |
| `V` / `B` | Lautstärke / Helligkeit | `.` | Nächster Titel |
| `E` | Termin | `T` | Timer (zeigt das Abspalten) |
| `C` | Laden | `S` | Zeitlupe (1× → 0,5× → 0,2×) |
| `R` | Reduzierte Bewegung | `H` | Lab ausblenden |

Außerdem: **Mausrad** über der Island ändert die Lautstärke, **Überfahren** der Musik-Island öffnet den Player.
Die **Komponenten-Galerie** (`mocks/island/gallery.html`) zeigt alle Bausteine in allen Zuständen und Themes
nebeneinander.

## Die fünf Konzepte

| | Konzept | Charakter |
|---|---|---|
| 1 | **Obsidian** | Apple-nah: reines Schwarz, kräftige Federn, die zweite Aktivität spaltet sich flüssig ab |
| 2 | **Monet** | Material 3 Expressive: alle Farben aus einem Seed (Wallpaper/Album), Formen verwandeln sich je nach Zustand |
| 3 | **Liquid Glass** | Lichtbrechendes Glas über dem Wallpaper, Glanzkante folgt der Maus |
| 4 | **Terminal** | Monospace, Haarlinien, Bernstein; ohne Federn, Inhalte „rendern“ sich ein |
| 5 | **Paper** | Editorial: warmes Papier, Serifen-Ziffern, Tinte und Zinnoberrot |

Alle fünf benutzen **denselben** HTML-/CSS-/JS-Code, nur die Tokens unterscheiden sich. Genau darauf baut später das
Theme-System der App auf.

## Struktur

```
design/                 Design-System (wird später zu packages/design)
  DESIGN.md             verbindliche Gestaltungsregeln
  THEMING.md            eigene Themes bauen
  tokens/tokens.css     Token-Vertrag
  themes/*.css          Obsidian, Monet, Glass, Terminal, Paper + Vorlage
docs/PLAN.md            Architektur, Machbarkeit, Roadmap
mocks/island/           Island Lab + Komponenten-Galerie (ohne Build-Schritt)
tools/lab-smoke/        Headless-Smoke-Test für das Lab (Laufzeitfehler, unbekannte Icons)
```

Smoke-Test ausführen (prüft keine Optik):

```powershell
cd tools/lab-smoke; npm install; npm test
```
