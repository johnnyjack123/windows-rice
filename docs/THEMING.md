# Themes bauen und anpassen

> Ein Theme verändert, **wie** rice aussieht und sich bewegt, nie **was** es zeigt.
> Grundlagen: [`DESIGN.md`](DESIGN.md) · Vertrag: [`web/src/design/tokens.css`](../web/src/design/tokens.css) ·
> Vorlage: [`web/src/design/themes/_template.css`](../web/src/design/themes/_template.css)

Es gibt drei Wege, das Aussehen zu ändern – vom einfachsten zum mächtigsten:

| Weg | Wo | Was geht |
|---|---|---|
| **Theme-Editor** | Einstellungen → Darstellung → Anpassen | Farben, Schriften, Radien, Dichte, Tempo, Easing, Federung … pro Theme gespeichert, jederzeit zurücksetzbar |
| **Als eigenes Theme speichern** | Einstellungen → Darstellung → „Als eigenes Theme speichern“ | Das angepasste Theme wird ein eigenes, umbenennbares Theme (Ordner mit `theme.toml` + `theme.css`) |
| **Theme-Datei schreiben** | `%APPDATA%\rice\themes\<id>\theme.css` | Alles, was Tokens und gekapselte Overrides hergeben (dieses Dokument) |

---

## 1. Der Theme-Editor

Jedes Theme bringt eigene Standardwerte mit; der Editor zeigt sie an und überschreibt nur, was du änderst. Die Werte
liegen in den Einstellungen unter `theme.overrides.<theme-id>` und gelten nur für dieses Theme.

| Gruppe | Regler | Wirkt auf |
|---|---|---|
| Farben (pro Schema) | Hintergrund, Text, Akzent | `--base-bg`, `--base-fg`, `--accent` – alle Flächen, Linien und Zustände leiten sich daraus ab |
| Schrift | Oberfläche, Anzeige (Uhr, Zahlen), Textgröße | `--font-ui`, `--font-display`/`--font-numeric`, `--text-scale` |
| Form | Island-Höhe, Island-Radius, Panel-Radius, Kachel-Radius, Dichte | `--island-height`, `--radius-island`, `--radius-panel` (+ konzentrische Radien), `--radius-tile(-small)`, `--density` |
| Glas (nur Glas-Themes) | Lichtbrechung, Farbsaum, Milchglas | `--glass-refraction`, `--glass-dispersion`, `--glass-frost` |
| Bewegung | Tempo, Federung, Easing-Stärke, Weichzeichnen beim Überblenden, Flüssiges Abspalten | alle Dauern und Federn, Bounce, Bezier-Kurven, `--content-blur`, `--island-goo` |
| Icons | Stil, Strichstärke | `--icon-family`, `--icon-weight` |

- **Tempo** teilt alle Dauern und Feder-Dauern. **Federung** setzt den Bounce der Island; die anderen Federn behalten ihr
  Verhältnis dazu. **Easing-Stärke** interpoliert alle Bezier-Kurven zwischen linear (0 %) und dem Theme (100 %) bzw.
  darüber hinaus.
- Die Vorschau im Editor läuft live; die Island übernimmt die Änderung sofort.
- Der Editor warnt, wenn Text auf Hintergrund unter 4,5 : 1 Kontrast fällt.

## 2. Was ein Theme steuern kann

| Achse | Wichtigste Tokens | Beispiel |
|---|---|---|
| Farbe | `--base-bg`, `--base-fg`, `--accent`, `--mix-*`, gezielt `--color-*`, `--data-l/-c` | Monet leitet alles aus dem Wallpaper-Seed ab |
| Typografie | `--font-*`, `--type-*`, `--tracking-*`, `--label-transform`, `--display-variation` | Paper: Serifen-Ziffern; Terminal: Mono + Versalien; Workbench: eine Pixelschrift in einer Größe |
| Dichte | `--space-base`, `--tile-height`, `--control-height`, `--row-height` | Terminal: 3,5-px-Raster |
| Form | `--radius-panel`, `--panel-padding`, `--radius-island`, `--radius-tile(-on/-pressed)` | Monet: Pille → Rechteck bei „An“; LCARS: Kachel-Ende rundet sich bei „An“; Cyberpunk: Radien als abgeschrägte Ecken (`corner-shape`) |
| Trennung | `--elevation-*`, `--surface-texture` | Terminal: Haarlinien, Paper: Regeln + Papierkorn, LCARS: Farbblöcke, Workbench: 2-px-Rahmen |
| Material | `--island-material`, `--island-goo`, `--glass-*` | Glass: Liquid-Glass-Shader; Obsidian, Monet, Paper: flüssiges Abspalten |
| Ikonografie | `--icon-family`, `--icon-weight`, `--icon-grade`, `--icon-fill-on` | Terminal: Sharp 300 |
| Bewegung | `--spring-*`, `--dur-*`, `--content-blur`, `--enter-*`, `--stagger`, `--eq-timing` | Terminal: ohne Federn, gestuft |
| Inhalt | `--island-clock-seconds`, `--island-track-display`, `--spark-style` | Terminal: Uhr mit Sekunden, gestufte Sparklines |
| Komponenten | `--tile-*`, `--slider-*`, `--switch-*`, `--media-*`, `--progress-*` … | Obsidian: aktive Kacheln invertiert |
| Pausen-Vollbild | `--break-*`, `--type-break` | Terminal: ASCII-Balken statt Ring; Glass: echtes, mattiertes Wallpaper |

## 3. Das Farbmodell

`tokens.css` rechnet alle Farbrollen aus drei Eingängen. Ein Theme setzt in der Regel **nur diese drei** – und
optional die Abstufungen:

```css
--base-bg: #0f1012;          /* Hintergrund */
--base-fg: #ececef;          /* Text */
--accent:  #7aa2f7;          /* Akzent */

--mix-island: 0%;            /* Island = Hintergrund; 6 % = minimal heller */
--mix-container: 8%;         /* Karten */
--mix-container-high: 12.5%; /* Kacheln aus */
--mix-container-highest: 18%;
--mix-muted: 66%;            /* Nebentext */
--mix-subtle: 53%;
--mix-border: 8%;
--mix-border-strong: 17%;
```

Weil der Theme-Editor genau diese Eingänge überschreibt, funktionieren eigene Farben in **jedem** Theme. Setzt ein Theme
eine Rolle direkt (`--color-surface-container: …`), sollte es sie aus den Eingängen ableiten (`color-mix`, OKLCH), sonst
reagiert sie nicht auf den Editor.

### Dynamische Farbe

Relative OKLCH-Farben machen Paletten zu Funktionen eines Seeds:

```css
--base-bg: oklch(from var(--wallpaper-seed) 0.17 calc(c * 0.1) h);
--accent:  oklch(from var(--wallpaper-seed) 0.83 min(calc(c * 0.8), 0.13) h);
```

- **Helligkeit fest, Chroma proportional, Farbton vom Seed.** Graue Seeds bleiben grau, sehr gesättigte werden über
  `min()` gezähmt.
- **Albumfarbe:** `--media-color` ist an der Wurzel gesetzt und wird weich überblendet. Ein Theme kann die
  Medien-Tokens daraus ableiten (Monet):
  ```css
  [data-theme="monet"] .media {
    --media-bg: oklch(from var(--media-color) 0.36 min(calc(c * 0.6), 0.09) h);
    --media-fg: oklch(from var(--media-color) 0.94 calc(c * 0.25) h);
  }
  ```
- Datenfarben (Kalender, App-Icons) über `--data-l/-c` bzw. `--app-icon-l/-c` an das Theme anpassen.

## 4. Aufbau einer Theme-Datei

```css
[data-theme="mein-theme"] {
  color-scheme: dark;

  /* 1. Eingänge */
  --base-bg: #101114;
  --base-fg: #ecebe6;
  --accent: #7aa2f7;

  /* 2. Primitive: privat, nur in dieser Datei */
  --_highlight: #ffd166;

  /* 3. Gezielte Rollen und Komponenten-Tokens */
  --tile-bg-on: var(--accent);

  /* 4. Form, Typo, Bewegung … */
  --radius-panel: 28px;
  --spring-island: 0.45 0.15;
}

/* 5. Optionales zweites Schema */
[data-theme="mein-theme"][data-scheme="light"] {
  color-scheme: light;
  --base-bg: #f6f6f8;
  --base-fg: #17181b;
}

/* 6. Optionale gekapselte Overrides (siehe §6) */
[data-theme="mein-theme"] .tab.is-active { box-shadow: inset 0 -2px 0 var(--color-accent); }
```

Der Selektor ist ein **Attribut-Selektor ohne `html`**, damit ein Theme auch auf einem Teilbaum funktioniert (Galerie,
Theme-Karten in den Einstellungen).

## 5. Schritt für Schritt

1. **Startpunkt wählen:** Ein Standard-Theme im Editor anpassen und „Als eigenes Theme speichern“ – oder
   `_template.css` kopieren und `my-theme` ersetzen.
2. **Identität zuerst:** Hintergrund, Text, Akzent, Schrift. Damit steht 80 % des Eindrucks.
3. **Form und Trennung:** `--radius-panel` + `--panel-padding` (der Rest folgt konzentrisch), dann *eine*
   Trennungsstrategie.
4. **Bewegung:** Federn passend zum Charakter (verspielt ↔ sachlich). Bounce > 0,35 wirkt schnell albern.
5. **Prüfen** im Lab (alle Ereignisse, Kontrollzentrum, Detailseiten, Zeitlupe 0,2×) und in der **Galerie** (alle
   Zustände, beide Schemata).
6. **Kontrast messen:** `--color-text-muted` auf `--color-surface` und auf `--color-surface-container-highest`
   ≥ 4,5 : 1.
7. Checkliste in §10 abhaken.

## 6. Gekapselte Overrides – was erlaubt ist

Manche Charakterzüge gehen über Tokens hinaus (Terminal-Eckmarken, Paper-Regeln, Glas-Ebenen, Kachelfarben in
Obsidian). Dafür gilt:

- **Erlaubt:** Dekoration, Trennung, Darstellung von Unterelementen, Ein-/Ausblendanimation der Views
  (`.view`, `.view.is-active`), Glas-Ebenen (`.mat-*`), Komponenten-Tokens auf Unterelementen.
- **Nicht erlaubt:** Informationen verstecken, Reihenfolge oder Bedienlogik ändern, Trefferflächen verkleinern,
  Fokusringe entfernen, Literal-Farben *außerhalb* der Theme-Primitive.
- Immer mit `[data-theme="id"]` präfixen. Overrides gehören **nur** in die Theme-Datei, nie in Komponenten-CSS.
- Weitreichende Overrides (Layout, Deko) in einen Donut-Scope legen, damit sie nicht in verschachtelte Vorschauen
  anderer Themes (Galerie-Spalten, Theme-Karten) durchschlagen:
  `@scope ([data-theme="id"]) to ([data-theme]:not([data-theme="id"])) { :scope .tile { … } }` – `:scope` hat dieselbe
  Spezifität wie das Attribut. So machen es LCARS, Enterprise NX-01, Workbench 1.3 und Cyberpunk 2077.
- Wenn ein Override in zwei Themes nötig wird, ist es ein fehlendes Token → in `tokens.css` aufnehmen.

**Techniken aus den Standard-Themes**

| Technik | Wie | Beispiel |
|---|---|---|
| Rahmen ums Kontrollzentrum | Pseudo-Elemente an `.p-header`/`.p-body`, Platz über `padding-left` der `.view--panel` und `--panel-column` | LCARS: Ellbogen = Element mit dickem `border-left` + `border-top` und elliptischem Eckradius (die Innenkurve entsteht automatisch) |
| Deko an der Pille | `.island__bg::before` (wird mit der Form beschnitten) oder `.view[data-compact]::after` (wird mitgemessen) | LCARS: Endkappen; Workbench: Tiefen-Gadget |
| Abgeschrägte Ecken | `corner-shape: bevel` bzw. pro Ecke (`square square bevel bevel`); Radien werden zur Fasenbreite, Rahmen und Schatten folgen | Cyberpunk; NX-01 auch an den Ellbogen (aus der runden wird eine schräge Kurve); ohne Unterstützung bleiben kleine Rundungen |
| Farbwechsel je Fläche | Rollen lokal neu setzen (`--color-text`, `--color-focus` …) auf dem Element mit der anderen Fläche | Workbench: weiße Titelleisten und Requester mit blauer Schrift |
| Einmaliger Effekt beim Einblenden | `animation` auf `.view.is-active` (Name wechseln bei `.is-bumped`), bei reduzierter Bewegung `none` | Cyberpunk: Glitch |
| Text als Deko | nur Zeichen/Codes ohne Sprache, mit leerem Alternativtext: `content: "47-1138" / ""` | LCARS-Rahmen, Cyberpunk `//` |

## 7. Schemata, Ladereihenfolge und Laufzeitwerte

- **Hell/Dunkel:** zweites Schema als `[data-theme="id"][data-scheme="light"]`. Die App setzt `data-scheme` (manuell
  oder „Automatisch“ nach Windows). Ein Theme mit nur einem Schema nutzt immer dieses.
- **Ladereihenfolge** (`web/src/design/styles.ts`): Tokens → Schriften → Basis → Komponenten → Standard-Themes. Danach
  zur Laufzeit: eigene Themes, dann die Editor-Overrides. Gekapselte Overrides gewinnen dadurch Gleichstände mit
  Komponenten-CSS; der Editor gewinnt gegen das Theme.
- **Reduzierte Bewegung** setzt `data-motion="reduced"` an der Wurzel; `base.css` dämpft dann alle Federn. Ein Theme
  muss dafür nichts tun.
- **Laufzeitwerte – nie im Theme setzen, nur lesen:** `--media-color`, `--wallpaper-src`, `--wallpaper-seed`,
  `--wallpaper-blurred`, `--wallpaper-rim`, `--ix/--iy/--icx/--icy/--vw/--vh/--r` (Glas-Ausrichtung), `--mx`
  (Glanzpunkt), `--ease-spring-*`/`--dur-spring-*` (aus den Federn erzeugt), `--panel-width/-height` am Fenster.

## 8. Bewegung im Theme

- Federn als `"<Dauer> <Bounce>"`. Die Laufzeit erzeugt daraus `linear()`-Kurven für CSS. Ein Theme schreibt also nur
  die Feder, nie eine Bezier-Kurve für räumliche Bewegung.
- `--content-blur: 0px` + `--enter-scale: 1` schaltet die weiche Auflösung ab (Terminal, Paper).
- Eigene Ein-/Ausblendtechnik über gekapselte `.view`-Overrides (Terminal: `clip-path` mit `steps()`).
- `--island-goo: 1` aktiviert das flüssige Abspalten der zweiten Aktivität (Metaball-Brücke bzw. weiche Vereinigung im
  Glas-Shader); `--island-split-pull` bestimmt, wie weit sich der Hals dehnt, bevor er reißt.

## 8a. Liquid Glass

Ein Theme mit `--island-material: glass` bekommt den WebGL-Shader (`web/src/island/glass/liquid-glass.ts`). Er liest:

| Token | Bedeutung | Glass (Standard) |
|---|---|---|
| `--glass-bezel` | Breite des gewölbten Rands | `16px` |
| `--glass-refraction` | Stärke der Lichtbrechung (Faktor) | `1` |
| `--glass-dispersion` | Farbsaum der Brechung, 0…1 | `0.35` |
| `--glass-frost` | Milchglas des Körpers, 0…1 | `0.78` |
| `--glass-edge-light` | Randlicht, 0…1 | `0.55` (hell `0.75`) |
| `--glass-shadow` | Deckkraft des Schattens | `0.38` (hell `0.22`) |
| `--glass-tint`, `--glass-tint-expanded` | Tönung kompakt / als Kontrollzentrum | aus `--base-bg` |

Ohne WebGL2 (oder in Vorschauen) zeichnen die CSS-Ebenen `.mat-rim/-core/-tint/-sheen` das Glas. Die Laufzeit setzt
`data-glass="webgl"` bzw. `"css"` an der Wurzel; ein Theme kann beide Fälle gezielt ansprechen.

## 9. Theme-Pakete

```
%APPDATA%\rice\themes\
└─ mein-theme\
   ├─ theme.toml        Manifest
   └─ theme.css         Aufbau wie §4, Selektor [data-theme="mein-theme"]
```

```toml
name = "Mein Theme"
author = "Ich"
version = "1.0.0"
schemes = ["dark", "light"]
based_on = "obsidian"     # optional: Theme, aus dem es abgeleitet wurde
```

- Die Ordner-ID (`^[a-z0-9-]{1,48}$`) ist auch die Theme-ID; die Standard-IDs (`obsidian`, `monet`, `glass`,
  `terminal`, `paper`, `lcars`, `nx01`, `workbench`, `cyberpunk`) sind reserviert.
- Das Backend überwacht den Ordner. Beim Speichern von `theme.css` wird das Theme ohne Neustart ersetzt (**Hot Reload**)
  – ideal zum Live-Ricen. Einstellungen → Über rice → „Theme-Ordner öffnen“.
- „Als eigenes Theme speichern“ schreibt genau so ein Paket: das Basis-Theme mit neuem Selektor plus einem Block mit den
  Editor-Werten.
- **Schriften:** Ein Theme kann jede installierte Windows-Schrift über `--font-ui` / `--font-display` nutzen. Mitgeliefert
  sind Roboto Flex, Instrument Sans/Serif, JetBrains Mono, Antonio, Saira, Rajdhani und Topaz (im Editor auswählbar).
  Saira hat eine Breitenachse: In den Rollen genügt `expanded`, `semi-expanded` usw. im `font`-Kurzschreibwert.
  Topaz ist eine Pixelschrift: scharf nur bei 16 px und Vielfachen, nur ein Schnitt – dazu `font-synthesis: none`, sonst
  verschmiert der künstliche Fettdruck die Pixel.

## 10. Theme-Checkliste

- [ ] Nur Werte überschrieben, keine neuen Namen (außer `--_primitive`)
- [ ] Farben aus `--base-bg`/`--base-fg`/`--accent` abgeleitet, damit der Editor wirkt
- [ ] Jede Fläche hat eine passende `on`-Farbe; Kontraste ≥ 4,5 : 1 (auch `-muted`)
- [ ] Fokusring sichtbar auf allen Flächen
- [ ] Konzentrische Radien stimmen (Panel → Karte → Zeile)
- [ ] Genau eine Trennungsstrategie
- [ ] Federn in Zeitlupe geprüft; reduzierte Bewegung geprüft
- [ ] Alle Ereignisse im Lab, alle Zustände in der Galerie, alle Schemata
- [ ] Helles **und** dunkles Fenster hinter der Island getestet (Lab → „Dahinter“)
- [ ] Mit Editor-Anpassungen (andere Farben, Radien, Tempo) gegengeprüft
