# CLAUDE.md

Windows-Desktop-Rice („rice“): Dynamic Island + Kontrollzentrum, später Taskleiste/Launcher.
Geplant: Tauri 2 + Rust + Svelte 5. **Aktuell Phase 0:** nur Browser-Mocks (`mocks/island/`) und Design-System
(`design/`). Architektur und Roadmap: `docs/PLAN.md`.

## Sprache
- Mit dem Nutzer Deutsch. UI-Texte Deutsch (Format: „14:32“, „82 %“, „vor 2 Min“).
- Code-Kommentare und JSDoc Englisch. Markdown-Doku Deutsch.

## Design – verbindlich
- Vor jeder UI-Änderung `design/DESIGN.md` lesen; Checkliste §13 anwenden.
- Komponenten benutzen **nur** Tokens aus `design/tokens/tokens.css`. Fehlt ein Wert → Token dort ergänzen und in
  allen Themes prüfen. Keine Literalwerte (Farbe, Abstand, Radius, Schrift, Dauer, Kurve, Schatten).
- Theme-spezifisches CSS nur in `design/themes/<id>.css`, präfixiert mit `[data-theme="<id>"]`
  (Regeln: `design/THEMING.md` §4). Themes werden **vor** dem Komponenten-CSS geladen → Overrides brauchen höhere
  Spezifität.
- Jede neue Komponente/jeder Zustand in `mocks/island/gallery.html` (`js/gallery.js`) aufnehmen.
- Neue Icons in `ICON_NAMES` (`mocks/island/js/icons.js`) eintragen (Font wird darauf reduziert).

## Code-Konventionen (JS/TS)
- JSDoc an allen Exporten bzw. globalen Funktionen/Objekten, Kommentare Englisch, immer geschweifte Klammern.
- Das Lab nutzt bewusst klassische Skripte ohne Build (läuft per Doppelklick); Reihenfolge in `index.html` beachten.
- Pro Frame nur Inline-Styles auf wenige kleine Elemente schreiben, nie geerbte Custom Properties auf große Teilbäume.

## Prüfen
- **Visuelle Tests macht der Nutzer selbst** – keine Browser-Automation. Nach Änderungen sagen, was er sich ansehen soll.
- Statisch: `node --check mocks/island/js/*.js`; Smoke-Test: `cd tools/lab-smoke && npm install && npm test`.
- Dateien nicht mit PowerShell `Get-Content`/`Set-Content` umschreiben (Windows PowerShell 5.1 zerstört UTF-8),
  sondern mit Edit/Write oder Python (`encoding="utf-8"`).
