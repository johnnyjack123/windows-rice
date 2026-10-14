# CLAUDE.md

Windows-Desktop-Rice („rice“): Dynamic Island + Kontrollzentrum, später Taskleiste/Launcher.
Stack: Tauri 2 + Rust + Svelte 5 (Vite, TypeScript). **Stand:** Frontend in `web/` fertig und läuft gegen ein
Mock-Backend; das Rust-Backend (`src-tauri/`) wird nach `docs/BACKEND.md` gebaut. Architektur und Roadmap:
`docs/PLAN.md`.

## Sprache
- Mit dem Nutzer Deutsch. Code-Kommentare und JSDoc Englisch. Markdown-Doku Deutsch.
- **Keine UI-Texte im Code.** Alles Sichtbare (auch `aria-label`, `title`) kommt aus
  `web/src/i18n/messages/de.ts` **und** `en.ts`. Zahlen/Daten/Zeiten über `fmt.*` aus `i18n.svelte.ts`.

## Design – verbindlich
- Vor jeder UI-Änderung `docs/DESIGN.md` lesen; Checkliste §13 anwenden.
- Komponenten benutzen **nur** Tokens aus `web/src/design/tokens.css`. Fehlt ein Wert → Token dort ergänzen und in
  allen Themes prüfen. Keine Literalwerte (Farbe, Abstand, Radius, Schrift, Dauer, Kurve, Schatten).
- Komponenten-Styles in den globalen CSS-Dateien (`ui.css`, `island.css`, `panel.css`, `settings.css`), nicht in
  Svelte-`<style>`-Blöcken.
- Theme-spezifisches CSS nur in `web/src/design/themes/<id>.css`, präfixiert mit `[data-theme="<id>"]`
  (Regeln: `docs/THEMING.md` §6). Themes laden **nach** dem Komponenten-CSS (Reihenfolge nur in `design/styles.ts`).
- Jede neue Komponente/jeder Zustand in die Galerie (`web/src/gallery/GalleryApp.svelte`).
- Neue Icons in `web/src/design/icons.json` eintragen, dann `pnpm icons`.

## Architektur
- `web/src/backend/contract.ts` ist die Quelle der Wahrheit für alle Commands und Events. Änderungen dort immer
  gleichzeitig in Mock-Backend und `docs/BACKEND.md` nachziehen.
- Island-Aktivitäten über die Registry (`island/activities.svelte.ts`), Kontrollzentrum-Bereiche über
  `island/panel/sections.ts`, Kacheln über `island/panel/tiles.ts` – neue Module dort registrieren statt Sonderfälle.
- Rust-Code nach `docs/BACKEND.md` (Fenster, Commands, Sicherheitsregeln: Passwörter nur im Windows-Tresor,
  `shell_open` nur per Whitelist).

## Code-Konventionen (TS/Svelte)
- JSDoc an allen Exporten, Kommentare Englisch, immer geschweifte Klammern.
- Pro Frame nur Inline-Styles auf wenige kleine Elemente schreiben, nie geerbte Custom Properties auf große Teilbäume.

## Prüfen
- **Visuelle Tests macht der Nutzer selbst** – keine Browser-Automation. Nach Änderungen sagen, was er sich ansehen soll.
- Statisch: `pnpm verify` (svelte-check ohne Warnungen, Vitest inkl. Smoke-Test, Build).
- Dateien nicht mit PowerShell `Get-Content`/`Set-Content` umschreiben (Windows PowerShell 5.1 zerstört UTF-8),
  sondern mit Edit/Write oder Python (`encoding="utf-8"`).
