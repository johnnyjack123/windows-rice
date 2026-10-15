# rice — Design-System

> **Verbindlich** für jede Oberfläche in diesem Projekt (Island, Kontrollzentrum, Einstellungen, später Taskleiste und
> Launcher). Werte stehen in [`web/src/design/tokens.css`](../web/src/design/tokens.css), Themes in
> [`web/src/design/themes/`](../web/src/design/themes/), eigene Themes erklärt [`THEMING.md`](THEMING.md).
> Konsistenz wird in der **Komponenten-Galerie** geprüft: `pnpm dev` → <http://127.0.0.1:1420/gallery.html>.

---

## 1. Zweck und Geltung

Dieses Dokument verhindert, dass das Design über Monate „zufällig“ wird. Es gilt:

- **Keine Komponente ohne Tokens.** Ein Literalwert (Farbe, Abstand, Radius, Schriftgröße, Dauer, Kurve, Schatten)
  in einer Komponente ist ein Fehler. Fehlt ein Wert → Token in `tokens.css` **und** in allen Themes prüfen.
- **Kein UI-Text im Code.** Jeder sichtbare Text, jedes `aria-label` und jeder Tooltip kommt aus den Sprachkatalogen
  (§15). Daten (Titel, Namen aus dem Backend) sind keine UI-Texte.
- **Keine Komponente ohne Galerie.** Jede Komponente steht mit allen Zuständen in der Galerie und wird dort in allen
  Themes × Schemata geprüft.
- **Abweichungen** werden hier begründet nachgetragen, nicht still im Code gemacht.

---

## 2. Designprinzipien

1. **Ruhig, bis es etwas zu sagen gibt.** Die Island ist im Leerlauf klein und still – und verschwindet ganz, wenn sie
   ein Fenster verdecken würde. Informationen erscheinen, wenn sie relevant sind, und verschwinden wieder.
2. **Form zuerst, Inhalt danach.** Bei jedem Zustandswechsel bewegt sich zuerst die Form, dann erscheint der Inhalt.
   Ausgänge sind schneller als Eingänge.
3. **Physik statt Zeitleiste.** Alles Räumliche ist eine Feder: unterbrechbar und mit erhaltener Geschwindigkeit. Feste
   Dauern gibt es nur für Farbe und Deckkraft.
4. **Konzentrisch und aus dem Raster.** Radien verschachteln sich konzentrisch, Abstände kommen aus dem 4-px-Raster,
   Größen ergeben sich aus dem Inhalt.
5. **Ein System, viele Gesichter.** Themes ändern Tokens, nicht Komponenten. Der Theme-Editor ändert nur die Eingänge
   eines Themes – alles andere leitet sich ab.
6. **Windows respektieren.** Native Schrift als Standard, vollständige Tastaturbedienung, Deep-Links in die
   Einstellungen, wo wir etwas nicht besser können, und ehrliche Grenzen (z. B. Glas ohne Live-Blur).

---

## 3. Professionell statt „KI-generiert“

| Vermeiden | Stattdessen |
|---|---|
| Lila-blaue Verläufe als Standardfarbe, Verläufe als Deko | Verläufe nur mit Bedeutung (Albumcover, Wallpaper). Flächen sind flach |
| Glow, Neon, Schatten auf jedem Element | Trennung kommt aus **einem** Token: Schatten *oder* Haarlinie *oder* Fläche |
| Emojis und Icon-Mischmasch | Ein Icon-Set (Material Symbols), Achsen per Token |
| Glassmorphism auf jeder Karte | Glas ist ein Material **eines** Themes und betrifft nur die Island-Fläche |
| Willkürliche Radien (12, 16, 18, 20 gemischt) | Konzentrische Regel (§8), Radien aus der Skala |
| „Lorem ipsum“, „Item 1“, englische Platzhalter | Echte Inhalte in echter Länge, beide Sprachen |
| Gradient-Text, Sparkle-Icons, Glow-on-Hover | Hierarchie über Größe, Gewicht und Farbe |
| Alles in Boxen mit Schatten | Weißraum und Ausrichtung gruppieren; Karten nur, wo Inhalte wirklich zusammengehören |
| Animation als Effekt („alles wackelt“) | Animation erklärt Zustandswechsel; Federn mit Maß |

**Details, die Qualität ausmachen:** tabellarische Ziffern für alles, was sich ändert · negatives Tracking bei großen
Schriftgraden · 0,5-px-Haarlinien statt 1 px auf dunklen Flächen · Icons optisch zentriert in ihren Kreisen ·
Inhalte, die sich beim Morphen nicht neu umbrechen · Fokusringe, die zum Theme passen.

---

## 4. Token-Architektur

| Ebene | Präfix / Beispiel | Wer definiert | Wer benutzt |
|---|---|---|---|
| 0 · Eingänge | `--base-bg`, `--base-fg`, `--accent`, `--mix-*`, `--text-scale`, `--density`, `--space-base` | Themes; der Theme-Editor überschreibt sie | Block B von `tokens.css` |
| 1 · Theme-Primitive | `--_name` | eine Theme-Datei | nur dieselbe Theme-Datei |
| 2 · Semantik | `--color-*`, `--type-*`, `--space-*`, `--radius-*`, `--elevation-*`, `--dur-*`, `--spring-*` | `tokens.css` (abgeleitet), Themes (gezielt) | Komponenten |
| 3 · Komponente | `--tile-*`, `--slider-*`, `--switch-*`, `--tabs-*`, `--media-*` … | `tokens.css`, Themes gezielt | die jeweilige Komponente |
| Laufzeit | `--media-color`, `--wallpaper-*`, `--ix/--iy/--r`, `--ease-spring-*` | App | Tokens, Glas-Material |

**Regeln**

1. Themes überschreiben **Werte**, nie Namen. Ein neuer Name entsteht nur in `tokens.css`.
2. Farben kommen **paarweise**: zu jeder Fläche gehört eine `on`-Farbe (`--color-accent` / `--color-on-accent`).
3. `tokens.css` hat zwei Blöcke auf `:root, [data-theme]`: **A** Eingänge und Konstanten, **B** alles Abgeleitete.
   Dadurch rechnen verschachtelte Theme-Bereiche (Galerie, Theme-Karten) korrekt neu.
4. Tokens, die von elementlokalen Werten abhängen (`--app-color`, `--c`), werden in der Komponente berechnet.
5. Komponenten lesen niemals `--_primitive` eines Themes.
6. **Ladereihenfolge** (einzig in `web/src/design/styles.ts` definiert): Tokens → Schriften → Basis → Komponenten →
   Features → Themes. Zur Laufzeit folgen eigene Themes und die Editor-Overrides. Gekapselte Theme-Overrides gewinnen
   dadurch Gleichstände mit Komponenten-CSS.
7. Komponenten-Styles stehen in globalen CSS-Dateien (`ui.css`, `island.css`, `panel.css`, `settings.css`), nicht in
   `<style>`-Blöcken von Svelte. So haben Themes stabile Klassennamen zum Ansprechen.

---

## 5. Farbe

Alle Flächen und Textfarben leiten sich aus drei Eingängen ab: **Hintergrund** (`--base-bg`), **Text** (`--base-fg`) und
**Akzent** (`--accent`). Die Abstufungen bestimmt das Theme über `--mix-*` (Anteil der Textfarbe in der Fläche).

| Rolle | Token | Ableitung (Standard) | Partner |
|---|---|---|---|
| Island | `--color-island` | `--base-bg` + `--mix-island` | `--color-on-island(-muted)` |
| Panel-Fläche | `--color-surface` | `--base-bg` | `--color-text` |
| Fenster | `--color-window` | deckend, auch bei Glas | `--color-text` |
| Container 1–3 | `--color-surface-container(-high/-highest)` | `--base-fg` zu 8 / 12,5 / 18 % | `--color-text` |
| Text | `--color-text`, `-muted`, `-subtle` | `--base-fg`, 66 %, 53 % | – |
| Linien | `--color-border`, `-strong` | `--base-fg` zu 8 / 17 % transparent | – |
| Akzent | `--color-accent` | `--accent` | `--color-on-accent` |
| Akzent-Container | `--color-accent-container` | OKLCH aus `--accent` | `--color-on-accent-container` |
| Akzent als Text | `--color-accent-text` | OKLCH aus `--accent` | – |
| Status | `--color-positive/-negative/-warning` | fest pro Theme | `--color-on-*` |

- **Zustände** (Hover/Pressed) sind *State-Layer*: `currentColor` mit `--state-hover-opacity` /
  `--state-pressed-opacity`. Sie funktionieren dadurch auf jeder Fläche.
- **Dynamische Farbe:** `--wallpaper-seed` (Monet), `--media-color` (Albumcover, weich überblendet). Themes entscheiden,
  was sie damit tun.
- **Fremde Datenfarben** (Kalender, App-Icons) werden auf `--data-l/-c` bzw. `--app-icon-l/-c` normiert.
- **Kontrast:** Text ≥ 4,5 : 1, große Schrift ≥ 3 : 1 – in **jedem** Theme und Schema. Der Theme-Editor warnt, wenn
  gewählte Farben den Mindestkontrast unterschreiten.

---

## 6. Typografie

Komponenten setzen Schrift nur über Rollen: `font: var(--type-*)`. Alle Rollen skalieren mit `--text-scale`.

| Rolle | Standard (Obsidian) | Verwendung |
|---|---|---|
| `--type-display` | 600 40/1 | Uhr im Panel |
| `--type-timer` | 300 46/1 | Timer-, Stoppuhr-Anzeigen |
| `--type-page` | 600 28/1.15 | Seitentitel in Fenstern (Einstellungen) |
| `--type-headline` | 600 19/1.2 | Detailseiten (WLAN, Bluetooth) |
| `--type-title` | 600 13,5/1.3 | Kachel-, Zeilen-, Kartentitel |
| `--type-body` | 400 13/1.4 | Fließtext |
| `--type-label` | 500 12/1.25 | Buttons, Tabs, Chips, Abschnittslabels |
| `--type-caption` | 400 11,5/1.3 | Metadaten, Zeiten |
| `--type-island` / `-island-clock` | 600 13 / 13,5 | Text und Uhr in der kompakten Island |
| `--type-metric` | 600 22/1 | Große Messwerte |
| `--type-badge` | 700 10/1 | Zähler |

**Regeln:** Zahlen, die sich ändern, sind tabellarisch (`.num`) · einzeilige Titel kürzen mit Ellipse, Fließtext max.
zwei Zeilen · Deutsch ist ~30 % länger als Englisch, beide Sprachen testen · Großschreibung nur über
`--label-transform` · Datum/Zeit/Zahlen immer über die Formatierer in `i18n.svelte.ts` (`fmt.*`).

---

## 7. Raster und Abstände

- Basis: `--space-base` (4 px, Terminal 3,5 px) × `--density` (Editor) = `--space-unit`.
- Skala: `--space-half` · `-1` · `-1-5` · `-2` · `-2-5` · `-3` · `-4` · `-5` · `-6` · `-8` · `-10`.
- Feste Maße: `--island-height` (34), `--island-top` (10), Panel `--panel-width` × `--panel-height` (800 × 556),
  linke Spalte `--panel-column` (340).

---

## 8. Form

**Skala:** `--radius-xs` 6 · `-sm` 10 · `-md` 14 · `-lg` 20 · `-xl` 28 · `-full` 999.

**Konzentrische Regel.** Ein Element, das mit Abstand *p* in einem Element mit Radius *R* sitzt, bekommt *R − p*
(mindestens `--radius-min`). Die Regel ist in den Tokens eingebaut:

```
--radius-card = --radius-panel − --panel-padding      (40 − 16 = 24)
--radius-row  = --radius-card  − --card-padding       (24 −  8 = 16)
--radius-live = --radius-panel × 0,75                 (Banner, Player)
```

- **Pille:** `999px` heißt „halbe Höhe“. Die Island berechnet ihren Radius pro Frame als `min(Radius, Höhe/2)`.
- **Form als Zustand** ist erwünscht (Monet: Pille → abgerundetes Rechteck bei „An“) – auch für Barrierefreiheit.

---

## 9. Trennung und Material

| Strategie | Beispiel | Token-Wert |
|---|---|---|
| Fläche | Obsidian, Monet | `--elevation-card: none`, Container unterscheiden sich farblich |
| Haarlinie | Terminal | `--elevation-card: inset 0 0 0 1px var(--color-border)` |
| Regel (Linie) | Paper | Fläche = Papier, Linien über gekapselte Overrides |
| Schatten | Panel in allen Themes | `--elevation-panel` |

**Material** (`--island-material`): `solid` oder `glass`. Glas bekommt vorgerenderte Wallpaper-Ebenen aus der Laufzeit;
Live-Blur des Desktops ist für ein geformtes Fenster unter Windows nicht möglich (PLAN §4).

**Liquid Glass** (`glass` mit WebGL2, `web/src/island/glass/`): Ein Shader zeichnet Island und Bubble als Distanzfeld.
Im Rand (`--glass-bezel`) wölbt sich die Fläche wie eine dicke Linse: Das Wallpaper wird nach außen gebrochen
(`--glass-refraction`), je Farbkanal leicht verschieden (Farbsaum, `--glass-dispersion`), der Körper ist Milchglas
(`--glass-frost`), ein dünnes Randlicht folgt dem Mauszeiger (`--glass-edge-light`), darunter ein weicher Schatten
(`--glass-shadow`). Island und Bubble verschmelzen beim Abspalten weich miteinander. Ohne WebGL2 übernehmen die
CSS-Ebenen (`.mat-*`) – ebenso in Galerie und Theme-Karten.

---

## 10. Ikonografie

- **Ein Set:** Material Symbols, lokal als Subset-Schrift. Stil pro Theme über `--icon-family` (Rounded, Sharp,
  Outlined), Achsen über `--icon-weight`, `--icon-grade`, `--icon-fill`. **Aktiv = gefüllt**, animiert.
- **Größen:** `--icon-size-sm` 16 · `--icon-size` 20 · `--icon-size-lg` 24 · `--icon-size-xl` 34.
- **Neues Icon:** Name in `web/src/design/icons.json` eintragen, dann `pnpm icons` (lädt das Subset neu). Ein fehlender
  Name erzeugt im Dev-Modus eine Konsolenwarnung.
- Reine Icon-Buttons haben immer `aria-label` und `title` (aus dem Katalog). Keine Emojis in der Oberfläche.

---

## 11. Bewegung

### 11.1 Federn

Format `"<Dauer s> <Bounce>"`, wie SwiftUI `.spring(duration:bounce:)`.

| Token | Standard | Zweck |
|---|---|---|
| `--spring-island` | 0,5 · 0,24 | Form der Island (Breite, Höhe, Radius) |
| `--spring-bubble` | 0,55 · 0,3 | Abspalten/Zurückziehen der zweiten Aktivität |
| `--spring-content` | 0,36 · 0,08 | Eintretende Inhalte |
| `--spring-control` | 0,32 · 0,18 | Kacheln, Schalter, Tabs, Slider |

Die Theme-Engine erzeugt daraus CSS-Kurven (`--ease-spring-*`) und Dauern (`--dur-spring-*`). Der Editor wirkt so:
**Tempo** teilt alle Dauern, **Federung** setzt den Bounce der Island (die anderen Federn skalieren proportional),
**Easing-Stärke** interpoliert die Bezier-Kurven zwischen linear und Theme.

### 11.2 Dauern und Kurven (nur Farbe und Deckkraft)

`--dur-instant` 80 · `--dur-fast` 150 (Ausgänge) · `--dur-medium` 240 (Eingänge) · `--dur-slow` 400 ·
`--ease-standard`, `--ease-enter`, `--ease-exit`.

### 11.3 Choreografie

1. **Form zuerst:** Inhalt startet nach `--enter-delay-grow` (70 ms) bzw. `--enter-delay-shrink` (150 ms).
2. **Ausgang schneller als Eingang.**
3. **Inhalte lösen sich auf** über `--content-blur`, `--enter-scale` und Deckkraft (Terminal: gestufter Wisch).
4. **Staffelung** (`--stagger`) für höchstens ~8 Blöcke.
5. **Unterbrechbar:** Eingaben werden nie blockiert; neue Ziele übernehmen die aktuelle Geschwindigkeit.
6. **Bump** bei Aktualisierung an Ort und Stelle (neue Mitteilung, Titelwechsel), ca. 3 %.
7. **Hover** 1,035, **Drücken** 0,965 – beides als Feder.
8. **Flüssiges Abspalten:** Die zweite Aktivität wird aus der Island **herausgezogen**. Der Hals dehnt sich
   (`--island-split-pull`), reißt, die Bubble federt an ihren Platz zurück, und beide Körper wackeln kurz nach. Die
   Brücke ist ein berechneter Vektorpfad (Metaball) bzw. beim Liquid Glass die weiche Vereinigung im Shader. Beim
   Zurückziehen bildet sie sich neu. Schließt das Kontrollzentrum, wird die Island **erst** wieder eine Pille, **dann**
   spaltet sich die Bubble ab.
9. **Keine Endlosschleifen** außer echtem Fortschritt, laufender Wiedergabe (Equalizer), klingelndem Wecker und dem
   Pausen-Vollbild (Atemkreis, Ambiente; §16).
10. **Reduzierte Bewegung** (Windows oder Lab): Federn kritisch gedämpft ≤ 0,3 s, keine Unschärfe, Skalierung,
    Staffelung oder Wackeln. Überblendungen bleiben.

### 11.4 Performance

Animiert werden `transform` (inkl. `scale`/`translate`), `opacity`, `filter`, Farben. Größen nur an der kleinen
Island-Hülle; der Inhalt hat eine feste Endgröße. Pro Frame werden Werte nur auf wenige kleine Elemente geschrieben.

- **Kein Neu-Stylen nebenbei:** Die Theme-Engine reagiert nur auf Theme-Einstellungen, Systemdarstellung und
  Bewegungsmodus. Andere Einstellungen (z. B. der zuletzt offene Tab) dürfen nie das Theme neu anwenden – das restylt
  das ganze Dokument (Test: `design/engine.test.ts`). Häufige Zustände nicht bei jedem Klick speichern.
- **Eine Layout-Eigenschaft pro Animation:** Tabs animieren nur `flex-grow`; Beschriftungen werden beschnitten, nicht in
  der Breite animiert.
- **Isolieren:** Gestapelte Seiten/Bereiche haben `contain: size layout style` und während der Bewegung `will-change`,
  damit Aktualisierungen in einem Bereich (tickende Timer, Sparklines) die anderen nicht neu layouten.
- Abgeleitete Werte, die pro Sekunde neu entstehen (Datum aus `clock.now`), als Zahl ableiten – sonst rendert alles
  Abhängige jede Sekunde neu.

---

## 12. Komponenten, Zustände, Barrierefreiheit

**Primitive** (`web/src/ui`): `Icon`, `Tile` (zwei Zonen), `SmallTile`, `Slider`, `Switch`, `Tabs`, `Segmented`, `Range`, `Stepper`,
`ColorInput`, `Progress`, `Check`, `AppIcon`, `Art`, `Equalizer`, `Spark`, `Battery`, `Empty`, `Banner` sowie die
CSS-Primitive `.btn`, `.icon-btn`, `.chip`, `.row`, `.field`, `.select`, `.section-label`.
**Fachkomponenten** bauen nur auf Primitiven auf (Island-Views, Panel-Bereiche, Einstellungsseiten).

| Zustand | Regel |
|---|---|
| Hover / Gedrückt | State-Layer; Gedrückt zusätzlich Einsinken (Feder) |
| Fokus | Fokusring aus `--color-focus`, an jedem interaktiven Element, nur bei Tastaturfokus |
| An / Ausgewählt | Farbe **und** Form/Icon-Füllung, nie nur Farbe |
| Deaktiviert / nicht verfügbar | 38 % Deckkraft + Erklärung („Wird nicht unterstützt“) |
| Lädt | Spinner nur an der betroffenen Stelle; das Layout springt nicht |
| Leer | Icon + Titel + ein Satz |
| Fehler | Ursache und nächster Schritt („Standortfreigabe nötig → Einstellungen öffnen“) |

**Barrierefreiheit:** semantische Rollen (`button`, `switch`, `slider`, `spinbutton`, `tab`, `checkbox`, `radio`), Labels
an allen Icon-Buttons, Treffer ≥ 32 px, Pfeiltasten in Tabs/Slidern/Zeitwählern, Esc schließt, Kontraste nach §5,
Bedeutung nie nur über Farbe.

---

## 13. Review-Checkliste (vor jedem Merge mit UI-Änderung)

- [ ] Keine Literalwerte in Komponenten (Farbe, Abstand, Radius, Schrift, Dauer, Kurve, Schatten)
- [ ] Keine UI-Texte im Code; neue Texte in `de.ts` **und** `en.ts` (der Paritätstest prüft das)
- [ ] Neue Tokens in `tokens.css` und in allen Themes geprüft
- [ ] Farben paarweise, Radien konzentrisch, Abstände aus der Skala
- [ ] Alle Zustände umgesetzt: Hover, Gedrückt, Fokus, Deaktiviert, Lädt, Leer, Fehler, Ausgewählt
- [ ] In der Galerie in **allen Themes × Schemata** angesehen, auch mit Editor-Anpassungen
- [ ] Mit längstem realistischem Text in **beiden** Sprachen getestet
- [ ] Bewegung über Tokens, Ausgang schneller als Eingang, reduzierte Bewegung geprüft
- [ ] `pnpm verify` grün (Typcheck, Tests inkl. Smoke-Test, Build)
- [ ] Ehrlicher Bericht: was geprüft wurde und was nicht

---

## 14. Island-Spezifikation

### 14.1 Ansichten

| Ansicht | Größe | Inhalt |
|---|---|---|
| `clock` (Leerlauf) | Inhalt × 34 | Uhrzeit (Terminal mit Sekunden), Punkt bei ungelesenen Mitteilungen |
| `music` / `music:expanded` | Inhalt × 34 / 392 × Inhalt | Cover, Titel · Interpret, Equalizer / Player mit Fortschritt |
| `notification` | 392 × Inhalt | App-Icon, App, Titel, eine Zeile Text |
| `osd` | 264 × 42 | Icon nach Stufe, Balken, Wert |
| `event` | Inhalt × 34 | Fortschrittsring in Kalenderfarbe, Titel, „in 5 Min“ |
| `timer` / `stopwatch` | Inhalt × 34 | Restzeit des nächsten Timers (+n weitere) / laufende Zeit |
| `pomodoro` | Inhalt × 34 | Phasen-Icon, Phase, Restzeit (Fokus in Akzentfarbe, Pausen in Grün) |
| `timer:expanded`, `stopwatch:expanded`, `pomodoro:expanded` | 304 × Inhalt | Schnellaktionen: Ring + Restzeit, darunter +1 Min/Pause/Abbrechen bzw. Runde/Pause bzw. Pause/Überspringen/Beenden |
| `pomodoroPhase` | 304 × Inhalt | Ankündigung eines automatischen Phasenwechsels („Zeit für eine Pause · 5 Min“) |
| `charging` | ≥ 264 × 34 | „Wird geladen“, Prozent, Akku |
| `alarm` / `timerDone` / `pomodoroDone` | 392 × Inhalt | Wackelnde Glocke, Uhrzeit bzw. Name, Schlummern/+1 Min und Ausschalten/Stopp; beim Pomodoro „Pause starten“/„Fokus starten“ und Beenden |
| `panel` | 800 × 556 | Kontrollzentrum |

### 14.2 Arbitrierung

| Aktivität | Priorität | Lebensdauer | Klick öffnet |
|---|---|---|---|
| Wecker klingelt | 110 | bis Schlummern/Aus | Timer |
| Timer abgelaufen | 105 | bis Stopp | Timer |
| Pomodoro-Phase vorbei (ohne Autostart) | 104 | bis Start/Beenden | Timer |
| Mitteilung | 100 | 4,8 s (pausiert bei Hover) | Mitteilungen |
| Pomodoro-Phasenwechsel | 95 | 5,2 s | Timer |
| OSD | 90 | 1,7 s nach letzter Änderung | – |
| Laden | 80 | 3,2 s | – |
| Termin | 60 | bis 1 min nach Beginn (max. 15 min) | Kalender |
| Musik | 57 | solange sie läuft, 12 s nach Pause | – |
| Pomodoro | 55 | solange ein Zyklus läuft | Timer |
| Timer | 50 | solange einer läuft oder pausiert ist | Timer |
| Stoppuhr | 45 | solange sie läuft oder pausiert ist (bis Zurücksetzen) | Timer |

Die höchste Priorität besitzt die Island. Die nächsthöhere Aktivität mit Bubble-Darstellung spaltet sich ab, aber nur
neben kompakten Ansichten. Laufende Musik behält die Haupt-Pille; Timer, Stoppuhr und Pomodoro spalten sich daneben ab
(Hover auf die Bubble zeigt ihre Schnellaktionen). Klingelnde Signale (Wecker, Timer, Pomodoro) stehen in einer Warteschlange: Nichts geht
verloren, wenn mehrere gleichzeitig fällig werden.

### 14.3 Sichtbarkeit (Auto-Hide)

| Situation | Island |
|---|---|
| Ein Fenster überlappt die Island-Zone | gleitet nach oben aus dem Bild (abschaltbar) |
| Maus oben mittig gegen die Kante drücken | kommt zurück, bis die Maus den Bereich verlässt |
| Kurzlebiges Ereignis (Mitteilung, OSD, Laden, Termin) | blendet für seine Dauer ein (abschaltbar) |
| Vollbild-App | versteckt, nur Wecker und abgelaufene Timer kommen durch |
| Kontrollzentrum offen | immer sichtbar |

Versteckt ist die Island vollständig durchklickbar.

### 14.4 Interaktion

| Eingabe | Wirkung |
|---|---|
| Hover über einer Pille | Feder auf 1,035 (Bubble 1,06); nach 420 ms öffnet ihre Vorschau (Musik-Player, Schnellaktionen), nach Verlassen 260 ms zurück |
| Vorschau der Bubble | wächst um ihre eigene Mitte (bleibt unter dem Zeiger) und schiebt die Island zur Seite; wächst die Island, weicht die Bubble nach rechts – beides als Feder, immer innerhalb des Fensters |
| Klick | Kontrollzentrum (Bereich der Aktivität) |
| Mausrad | Lautstärke ± 4 % mit OSD (abschaltbar) |
| Enter / Leertaste (fokussiert), globales Tastenkürzel | Kontrollzentrum umschalten |
| Esc, Klick daneben, Fokusverlust | schließen |
| Klick auf Bubble | Kontrollzentrum mit dem Bereich dieser Aktivität (Schnellaktionen in der Vorschau wirken direkt) |

### 14.5 Kontrollzentrum

Kopf: große Uhr + Datum · Akku, Kacheln bearbeiten (öffnet Einstellungen), Einstellungen, Energie-Menü (Sperren,
Energie sparen, Abmelden, Neu starten, Herunterfahren).
Links: große und kleine Kacheln (konfigurierbar), Lautstärke, Helligkeit, Medienkarte. Kacheln mit Details haben zwei
Zonen: **Icon schaltet, Text öffnet** die Detailseite. Was dort zu frickelig wird, öffnet die Windows-Einstellungen.
**Rechtsklick** (oder Kontextmenü-Taste) auf eine Kachel öffnet die passende Seite der Windows-Einstellungen.
Rechts: Bereiche **Mitteilungen · Kalender · Aufgaben · Timer · System** (Reihenfolge und Sichtbarkeit konfigurierbar).
Der Timer-Bereich hat vier Tabs: **Timer · Stoppuhr · Wecker · Pomodoro**. Der Pomodoro zeigt ein großes Zifferblatt
mit Sessionpunkten; seine Einstellungen (Längen, lange Pause nach n Sessions, Autostart, Pause als Mitteilung oder
Vollbild) liegen auf einer zweiten Fläche hinter dem Regler-Icon und zusätzlich im Einstellungsfenster.
Der aktive Tab wächst und zeigt seine Beschriftung.

---

## 15. Sprachen

- Kataloge: `web/src/i18n/messages/de.ts` ist die Referenz, `en.ts` hat exakt dieselbe Form (TypeScript erzwingt das,
  ein Test vergleicht zusätzlich Schlüssel und Funktionsstellen).
- Interpolation und Plural über Funktionen im Katalog (`count: (n) => …`), nie durch Zusammensetzen im Komponenten-Code.
- Zahlen, Daten, Uhrzeiten, Dauern über `fmt.*` (folgen der aktiven Sprache).
- Sprache: „Wie Windows“, Deutsch oder Englisch (Einstellungen → Sprache), wirkt live in allen Fenstern.
- Neue Sprache: Katalog anlegen, in `LOCALES` eintragen – fertig.

---

## 16. Pausen-Vollbild (Pomodoro)

Ein eigenes Fenster je Monitor (`break.html`, siehe BACKEND §3.3). Es soll zum Aufstehen einladen, nicht bestrafen.

- **Aufbau:** Uhrzeit oben · Zifferblatt mit Atemkreis, Fortschrittsring und Countdown (`--type-break`) · Sessionpunkte ·
  ein Tipp für die Pause · unten „Gedrückt halten zum Überspringen“ (`--break-hold`, auch per Leertaste/Enter halten).
  Ist die Pause vorbei: Haken, „Bereit für die nächste Runde?“ und „Weiter arbeiten“.
- **Bewegung:** Hintergrund blendet ein, Inhalte steigen gestaffelt auf, Ausgang ist ein Ausblenden. Der Atemkreis atmet
  in `--break-breath` (4 s ein, 4 s aus, Beschriftung wechselt mit), das Ambiente-Licht driftet langsam
  (`--break-drift`). Reduzierte Bewegung: beides steht still.
- **Themes** färben es über `--break-*`: Obsidian echtes Schwarz mit weißem Ring, Monet Seed-Farben und ein Atemkreis,
  der seine Form wandelt, Liquid Glass das echte Wallpaper mattiert mit Glas-Zifferblatt, Terminal Scanlines, ASCII-Balken
  und Cursor, Paper Papierkorn, Tuschering und kursiver Tipp.
- **Vorschau:** `break.html?preview` (auch `?preview=long`, `?preview=over`).

