# Themes bauen

> Ein Theme verändert, **wie** rice aussieht und sich bewegt, nie **was** es zeigt.
> Grundlagen: [`DESIGN.md`](DESIGN.md) · Vertrag: [`tokens/tokens.css`](tokens/tokens.css) ·
> Vorlage: [`themes/_template.css`](themes/_template.css)

---

## 1. Was ein Theme steuern kann

| Achse | Wichtigste Tokens | Beispiel |
|---|---|---|
| Farbe | `--accent`, `--color-*`, `--data-l/-c` | Monet leitet alles aus einem Seed ab |
| Typografie | `--font-*`, `--type-*`, `--tracking-*`, `--label-transform`, `--display-variation` | Paper: Serifen-Ziffern; Terminal: Mono + Versalien |
| Dichte | `--space-unit`, `--tile-height`, `--control-height`, `--row-height` | Terminal: 3,5-px-Raster |
| Form | `--radius-panel`, `--panel-padding`, `--radius-island`, `--radius-tile(-on/-pressed)` | Monet: Pille → Rechteck bei „An“ |
| Trennung | `--elevation-*`, `--surface-texture` | Terminal: Haarlinien, Paper: Regeln + Papierkorn |
| Material | `--island-material`, `--island-goo` | Glass: vorgerendertes Glas; Obsidian: flüssiges Abspalten |
| Ikonografie | `--icon-family`, `--icon-weight`, `--icon-grade`, `--icon-fill-on` | Terminal: Sharp 300 |
| Bewegung | `--spring-*`, `--dur-*`, `--content-blur`, `--enter-*`, `--stagger`, `--eq-timing` | Terminal: ohne Federn, gestuft |
| Inhalt (Dichte) | `--island-clock-seconds`, `--island-track-display`, `--spark-style` | Terminal: Uhr mit Sekunden, gestufte Sparklines |

## 2. Aufbau einer Theme-Datei

```css
[data-theme="mein-theme"] {
  color-scheme: dark;

  /* 1. Primitive: privat, nur in dieser Datei */
  --_bg: #101114;
  --_ink: #ecebe6;

  /* 2. Eingänge */
  --accent: #7aa2f7;

  /* 3. Semantik: nur Werte überschreiben, die anders sein sollen */
  --color-surface: var(--_bg);
  --color-text: var(--_ink);

  /* 4. Form, Typo, Bewegung … */
  --radius-panel: 28px;
  --spring-island: 0.45 0.15;
}

/* 5. Optionales zweites Schema */
[data-theme="mein-theme"][data-scheme="light"] { … }

/* 6. Optionale gekapselte Overrides (siehe §4) */
[data-theme="mein-theme"] .tab.is-active { box-shadow: inset 0 -2px 0 var(--color-accent); }
```

Der Selektor ist ein **Attribut-Selektor ohne `html`**, damit ein Theme auch auf einem Teilbaum funktioniert (Galerie,
Vorschauen).

## 3. Schritt für Schritt

1. `themes/_template.css` kopieren, `mein-theme` ersetzen.
2. Im Lab (`mocks/island/js/themes.js`) eintragen: `id`, `name`, `schemes`, `tagline`. In der echten App übernimmt das
   die `theme.toml`, siehe §8.
3. **Identität zuerst:** Akzent, Island-Farbe, Panel-Fläche, Schrift. Damit steht 80 % des Eindrucks.
4. **Form und Trennung:** `--radius-panel` + `--panel-padding` (der Rest folgt konzentrisch), dann *eine*
   Trennungsstrategie.
5. **Bewegung:** Federn passend zum Charakter (verspielt ↔ sachlich). Bounce > 0,35 wirkt schnell albern.
6. **Prüfen** im Lab (alle Ereignisse, Panel, Drill-ins, Zeitlupe 0,2×) und in der **Galerie** (alle Zustände).
7. **Kontrast messen**: `--color-text-muted` auf `--color-surface` und auf `--color-surface-container-highest` ≥ 4,5 : 1.
8. Checkliste in §9 abhaken.

## 4. Gekapselte Overrides – was erlaubt ist

Manche Charakterzüge gehen über Tokens hinaus (Terminal-Eckmarken, Paper-Regeln, Glas-Ebenen). Dafür gilt:

- **Erlaubt:** Dekoration, Trennung, Darstellung von Unterelementen, Ein-/Ausblendanimation der Views
  (`.view`, `.view.is-active`), Glas-Ebenen (`.mat-*`).
- **Nicht erlaubt:** Informationen verstecken, Reihenfolge oder Bedienlogik ändern, Trefferflächen verkleinern,
  Fokusringe entfernen, Literal-Farben *außerhalb* der Theme-Primitive.
- Immer mit `[data-theme="id"]` präfixen. Overrides gehören **nur** in die Theme-Datei, nie in Komponenten-CSS.
- Wenn ein Override in zwei Themes nötig wird, ist es ein fehlendes Token → in `tokens.css` aufnehmen.

## 5. Schemata, Neuableitung und Laufzeitwerte

- **Hell/Dunkel:** zweites Schema als `[data-theme="id"][data-scheme="light"]`. Die App setzt `data-scheme` (manuell oder
  „auto“ nach Windows).
- **Lokale Neueinfärbung (`.reseed`):** Ein Theme kann seine abgeleiteten Farben zusätzlich auf
  `[data-theme="id"] .reseed` deklarieren. Setzt eine Komponente lokal `--accent` (z. B. die Medienkarte auf die
  Albumfarbe), färbt sich ihr Teilbaum komplett neu. Monet nutzt das.
- **Laufzeitwerte – nie im Theme setzen:** `--media-color`, `--wallpaper-src`, `--wallpaper-seed`,
  `--wallpaper-blurred`, `--wallpaper-rim`, `--ix/--iy/--icx/--icy/--vw/--vh/--r` (Glas-Ausrichtung), `--mx`
  (Glanzpunkt), `--ease-spring-*`/`--dur-spring-*` (aus den Federn erzeugt). Ein Theme darf sie **lesen**.

## 6. Dynamische Farbe

Relative OKLCH-Farben machen ganze Paletten zu Funktionen eines Seeds:

```css
--color-surface:   oklch(from var(--accent) 0.17 calc(c * 0.1) h);
--color-accent:    oklch(from var(--accent) 0.83 min(calc(c * 0.8), 0.13) h);
```

- **Helligkeit fest, Chroma proportional, Farbton vom Seed.** Graue Seeds bleiben grau (kein Rosastich), sehr
  gesättigte werden über `min()` gezähmt.
- Datenfarben (Kalender, App-Icons) über `--data-l/-c` bzw. `--app-icon-l/-c` an das Theme anpassen.

## 7. Bewegung im Theme

- Federn als `"<Dauer> <Bounce>"`. Die Laufzeit erzeugt daraus `linear()`-Kurven für CSS. Ein Theme schreibt also nur
  die Feder, nie eine Bezier-Kurve für räumliche Bewegung.
- `--content-blur: 0px` + `--enter-scale: 1` schaltet die weiche Auflösung ab (Terminal, Paper).
- Eigene Ein-/Ausblendtechnik über gekapselte `.view`-Overrides (Terminal: `clip-path` mit `steps()`).

## 8. Themes in der echten App (ab Phase 1)

```
%APPDATA%\rice\themes\
└─ mein-theme\
   ├─ theme.toml        Manifest
   ├─ theme.css         genau diese Datei-Struktur
   └─ fonts\ …          optionale Schriften (woff2)
```

```toml
id = "mein-theme"
name = "Mein Theme"
author = "Ich"
version = "1.0.0"
schemes = ["dark", "light"]
fonts = ["fonts/Inter-Variable.woff2"]   # optional, werden per @font-face registriert
requires = "rice >= 0.1"                 # Token-Vertrag-Version
```

Der `theme`-Service überwacht den Ordner. Beim Speichern wird `theme.css` ohne Neustart ersetzt (**Hot Reload**).
Das ist ideal zum Live-Ricen.

## 9. Theme-Checkliste

- [ ] Nur Werte überschrieben, keine neuen Namen (außer `--_primitive`)
- [ ] Jede Fläche hat eine passende `on`-Farbe; Kontraste ≥ 4,5 : 1 (auch `-muted`)
- [ ] Fokusring sichtbar auf allen Flächen
- [ ] Konzentrische Radien stimmen (Panel → Karte → Zeile)
- [ ] Genau eine Trennungsstrategie
- [ ] Federn geprüft in Zeitlupe; reduzierte Bewegung geprüft
- [ ] Alle Ereignisse im Lab, alle Zustände in der Galerie, beide Schemata
- [ ] Helles **und** dunkles Fenster hinter der Island getestet (Lab → „Dahinter“)
