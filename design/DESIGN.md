# rice — Design-System

> **Verbindlich** für jede Oberfläche in diesem Projekt (Island, später Taskleiste, Launcher, Einstellungen).
> Werte stehen in [`tokens/tokens.css`](tokens/tokens.css), Themes in [`themes/`](themes/), Anleitung für
> eigene Themes in [`THEMING.md`](THEMING.md). Die Komponenten-Galerie
> ([`mocks/island/gallery.html`](../mocks/island/gallery.html)) ist der Ort, an dem Konsistenz geprüft wird.

---

## 1. Zweck und Geltung

Dieses Dokument verhindert, dass das Design über Monate „zufällig“ wird. Es gilt:

- **Keine Komponente ohne Tokens.** Ein Literalwert (Farbe, Abstand, Radius, Schriftgröße, Dauer, Kurve, Schatten)
  in einer Komponente ist ein Fehler. Fehlt ein Wert → Token in `tokens.css` **und** allen Themes ergänzen.
- **Keine Komponente ohne Galerie.** Jede Komponente steht mit allen Zuständen in der Galerie und wird dort in allen
  Themes × Schemata geprüft.
- **Abweichungen** von diesem Dokument werden hier begründet nachgetragen, nicht still im Code gemacht.

---

## 2. Designprinzipien

1. **Ruhig, bis es etwas zu sagen gibt.** Die Island ist im Leerlauf klein und still. Informationen erscheinen, wenn sie
   relevant sind, und verschwinden wieder. Nichts blinkt, nichts wirbt um Aufmerksamkeit.
2. **Form zuerst, Inhalt danach.** Bei jedem Zustandswechsel bewegt sich zuerst die Form, dann erscheint der Inhalt.
   Ausgänge sind schneller als Eingänge.
3. **Physik statt Zeitleiste.** Alles Räumliche ist eine Feder: unterbrechbar und mit erhaltener Geschwindigkeit. Feste
   Dauern gibt es nur für Farbe und Deckkraft.
4. **Konzentrisch und aus dem Raster.** Radien verschachteln sich konzentrisch, Abstände kommen aus dem 4-px-Raster,
   Größen ergeben sich aus dem Inhalt.
5. **Ein System, viele Gesichter.** Themes ändern Tokens, nicht Komponenten. Was ein Theme nicht über Tokens erreicht,
   ist entweder ein fehlendes Token oder gehört nicht ins Theme.
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
| „Lorem ipsum“, „Item 1“, englische Platzhalter | Echte deutsche Inhalte in echter Länge |
| Gradient-Text, Sparkle-Icons, Glow-on-Hover | Hierarchie über Größe, Gewicht und Farbe |
| Alles in Boxen mit Schatten | Weißraum und Ausrichtung gruppieren; Karten nur, wo Inhalte wirklich zusammengehören |
| Animation als Effekt („alles wackelt“) | Animation erklärt Zustandswechsel; Federn mit Maß |

**Details, die Qualität ausmachen:** tabellarische Ziffern für alles, was sich ändert · negatives Tracking bei großen
Schriftgraden · 0,5-px-Haarlinien statt 1 px auf dunklen Flächen · Icons optisch zentriert in ihren Kreisen ·
Inhalte, die sich beim Morphen nicht neu umbrechen · Fokusringe, die zum Theme passen.

---

## 4. Token-Architektur

| Ebene | Präfix | Wer definiert | Wer benutzt |
|---|---|---|---|
| Theme-Primitive | `--_name` | eine Theme-Datei | nur dieselbe Theme-Datei |
| Semantik | `--color-*`, `--type-*`, `--space-*`, `--radius-*`, `--elevation-*`, `--dur-*`, `--spring-*` … | `tokens.css` (Standard), Themes (Overrides) | Komponenten |
| Komponente | `--tile-*`, `--slider-*`, `--switch-*`, `--tabs-*`, `--media-*`, `--island-*` … | `tokens.css` (zeigt auf Semantik), Themes gezielt | die jeweilige Komponente |
| Laufzeit | `--accent`, `--wallpaper-*`, `--media-color`, `--ix/--iy/--r` … | App/Services | Tokens und Glas-Material |

**Regeln**

1. Themes überschreiben **Werte**, nie Namen. Ein neuer Name entsteht nur in `tokens.css`.
2. Farben kommen **paarweise**: zu jeder Fläche gehört eine `on`-Farbe (`--color-accent` / `--color-on-accent`).
   Schriftfarben werden nie nach Gefühl auf eine Fläche gesetzt.
3. Abgeleitete Tokens werden auf `:root` **und** `[data-theme]` deklariert, damit verschachtelte Theme-Bereiche (Galerie,
   `.reseed`) korrekt neu berechnen. Tokens, die von elementlokalen Werten abhängen (z. B. `--app-color`), werden in der
   Komponente berechnet, nicht global.
4. Komponenten lesen niemals `--_primitive` eines Themes.

---

## 5. Farbe

| Rolle | Token | Partner | Verwendung |
|---|---|---|---|
| Island | `--color-island` | `--color-on-island`, `--color-on-island-muted` | Kompakte Island, Bubble |
| Panel-Fläche | `--color-surface` | `--color-text` | Aufgeklapptes Kontrollzentrum |
| Container 1–3 | `--color-surface-container(-high/-highest)` | `--color-text` | Karten → Kacheln/Felder → Spuren/gedrückt |
| Text | `--color-text`, `-muted`, `-subtle` | – | Primär, sekundär, tertiär |
| Linien | `--color-border`, `-strong` | – | Haarlinien, Regeln |
| Akzent | `--color-accent` | `--color-on-accent` | Aktive Zustände, primäre Aktionen |
| Akzent-Container | `--color-accent-container` | `--color-on-accent-container` | Chips, ausgewählte Elemente |
| Akzent als Text | `--color-accent-text` | – | Links, Fälligkeiten, aktive Netzwerke |
| Status | `--color-positive/-negative/-warning` | `--color-on-*` | Laden, Fehler, überfällig |
| Fokus | `--color-focus` | – | Fokusring |

- **Zustände** (Hover/Pressed) sind *State-Layer*: `currentColor` mit `--state-hover-opacity` /
  `--state-pressed-opacity`. Sie funktionieren dadurch auf jeder Fläche, auch auf aktiven Kacheln.
- **Dynamische Farbe:** `--accent` (Nutzer/Theme), `--wallpaper-seed` (Wallpaper), `--media-color` (Albumcover,
  weich überblendet). Themes entscheiden, was sie damit tun.
- **Fremde Datenfarben** (Kalenderfarben, App-Icons) werden auf `--data-l`/`--data-c` bzw. `--app-icon-l`/`-c`
  normiert. Die Farbe behält ihren Farbton, passt aber zum Theme.
- **Kontrast:** Text ≥ 4,5 : 1, große Schrift ≥ 3 : 1 – in **jedem** Theme und Schema. `--color-text-muted` ist der
  übliche Kandidat für Verstöße und wird bei jedem neuen Theme gemessen.

---

## 6. Typografie

Komponenten setzen Schrift nur über Rollen: `font: var(--type-*)`.

| Rolle | Standard (Obsidian) | Verwendung |
|---|---|---|
| `--type-display` | 600 40/1 | Uhr im Panel |
| `--type-headline` | 600 19/1.2 | Seitentitel (WLAN, Bluetooth) |
| `--type-title` | 600 13,5/1.3 | Kachel-, Zeilen-, Kartentitel |
| `--type-body` | 400 13/1.4 | Fließtext, Mitteilungstext |
| `--type-label` | 500 12/1.25 | Buttons, Tabs, Chips, Abschnittslabels |
| `--type-caption` | 400 11,5/1.3 | Metadaten, Zeiten |
| `--type-island` | 600 13/1 | Text in der kompakten Island |
| `--type-island-clock` | 600 13,5/1 | Uhr in der Island |
| `--type-metric` | 600 22/1 | Große Messwerte |
| `--type-badge` | 700 10/1 | Zähler |

**Regeln**
- Zahlen, die sich ändern (Uhr, Prozent, Zeiten), sind **tabellarisch** (`.num`).
- Deutsch läuft ~30 % länger als Englisch → einzeilige Titel kürzen mit Ellipse, Fließtext max. zwei Zeilen.
- Großschreibung nur über `--label-transform` (Theme-Entscheidung), nie im Text selbst.
- Tracking: Display negativ (`--tracking-display`), Labels optional positiv bei Versalien.
- Einheiten mit schmalem Abstand: „82 %“, „1,2 GB“, „4,7 GHz“; Uhrzeiten „14:32“; Datum „Dienstag, 13. Oktober“;
  relativ „vor 2 Min“, „in 25 Min“.

---

## 7. Raster und Abstände

- Basis: **4 px** (`--space-unit`). Skala: `--space-half` (2) · `-1` (4) · `-1-5` (6) · `-2` (8) · `-2-5` (10) ·
  `-3` (12) · `-4` (16) · `-5` (20) · `-6` (24) · `-8` (32) · `-10` (40).
- **Dichte** ändert ein Theme ausschließlich über `--space-unit` (Terminal: 3,5 px). Alles skaliert mit.
- Feste Maße der Island: Höhe `--island-height` (34), Abstand oben `--island-top` (10), Panel
  `--panel-width` × `--panel-height` (780 × 548), linke Spalte `--panel-column` (340).

---

## 8. Form

**Skala:** `--radius-xs` 6 · `-sm` 10 · `-md` 14 · `-lg` 20 · `-xl` 28 · `-full` 999.

**Konzentrische Regel.** Ein Element, das mit Abstand *p* in einem Element mit Radius *R* sitzt, bekommt den Radius
*R − p* (nie kleiner als `--radius-min`). Die Regel ist in den Tokens **eingebaut**:

```
--radius-card = --radius-panel − --panel-padding      (40 − 16 = 24)
--radius-row  = --radius-card  − --card-padding       (24 −  8 = 16)
```

Themes setzen also `--radius-panel` und `--panel-padding`, der Rest folgt. Weicht ein Theme bewusst ab (Monet: Kacheln
als Pillen), überschreibt es das Komponenten-Token.

- **Pille:** `999px` bedeutet „halbe Höhe“. Die Island berechnet den Radius jedes Frames als `min(Radius, Höhe/2)`.
- **Form als Zustand** ist erlaubt und erwünscht (Monet: Pille → abgerundetes Rechteck bei „An“). Das ist auch
  Barrierefreiheit, weil der Zustand nicht nur an der Farbe hängt.

---

## 9. Trennung und Material

Flächen werden über **eine** Strategie getrennt, die das Theme wählt:

| Strategie | Beispiel | Token-Wert |
|---|---|---|
| Fläche | Obsidian, Monet | `--elevation-card: none`, Container-Farben unterscheiden sich |
| Haarlinie | Terminal | `--elevation-card: inset 0 0 0 1px var(--_line)` |
| Regel (Linie) | Paper | Fläche = Papier, Linien über gekapselte Overrides |
| Schatten | Panel in allen Themes | `--elevation-panel` |

Komponenten schreiben nie eigene `box-shadow`/`border`, sondern benutzen `--elevation-*`.

**Material** (`--island-material`): `solid` (Standard) oder `glass`. Glas bekommt seine Ebenen (Kern, Rand, Tönung,
Glanz) vom Theme und vorgerenderte Wallpaper-Bilder von der Laufzeit. Live-Blur des Desktops ist unter Windows für ein
geformtes Fenster **nicht** möglich (siehe PLAN §4) – das Lab bildet das ehrlich ab.

---

## 10. Ikonografie

- **Ein Set:** Material Symbols als variable Schrift. Stil pro Theme über `--icon-family` (Rounded, Sharp, Outlined).
- **Achsen als Tokens:** `--icon-weight`, `--icon-grade`, `--icon-fill`. **Aktiv = gefüllt** (`--icon-fill-on`); der
  Übergang wird animiert.
- **Größen:** `--icon-size-sm` 16 · `--icon-size` 20 · `--icon-size-lg` 24 · `--icon-size-xl` 34.
- **Neues Icon:** Name in `ICON_NAMES` (`mocks/island/js/icons.js`, später `@rice/design`) eintragen. Die Schrift wird
  auf genau diese Icons reduziert; ein fehlender Name erscheint als Text und erzeugt eine Konsolenwarnung.
- Reine Icon-Buttons haben immer `aria-label` und `title`. Keine Emojis in der Oberfläche.

---

## 11. Bewegung

### 11.1 Federn

Format `"<Dauer s> <Bounce>"`, wie SwiftUI `.spring(duration:bounce:)`. Bounce 0 = kritisch gedämpft.

| Token | Standard | Zweck |
|---|---|---|
| `--spring-island` | 0,5 · 0,24 | Form der Island (Breite, Höhe, Radius) |
| `--spring-bubble` | 0,55 · 0,3 | Abspalten/Zurückziehen der zweiten Aktivität |
| `--spring-content` | 0,36 · 0,08 | Eintretende Inhalte (Skalierung, Versatz) |
| `--spring-control` | 0,32 · 0,18 | Kacheln, Schalter, Tabs, Slider |

Aus `--spring-content` und `--spring-control` erzeugt die Laufzeit CSS-Kurven (`--ease-spring-*` als `linear()`) und
Dauern (`--dur-spring-*`). CSS-Übergänge und JS-Federn fühlen sich dadurch identisch an.

### 11.2 Dauern und Kurven (nur Farbe und Deckkraft)

`--dur-instant` 80 · `--dur-fast` 150 (Ausgänge) · `--dur-medium` 240 (Eingänge, Farbwechsel) · `--dur-slow` 400 ·
`--ease-standard`, `--ease-enter`, `--ease-exit`.

### 11.3 Choreografie

1. **Form zuerst:** Inhalt startet nach `--enter-delay-grow` (70 ms) beim Wachsen bzw. `--enter-delay-shrink` (150 ms)
   beim Schrumpfen.
2. **Ausgang schneller als Eingang** (150 ms vs. 240 ms).
3. **Inhalte lösen sich auf** über Unschärfe (`--content-blur`), Skalierung (`--enter-scale`) und Deckkraft. Themes
   können das ersetzen (Terminal: gestufter Wisch).
4. **Staffelung** (`--stagger`) für höchstens ~8 Blöcke.
5. **Unterbrechbar:** Eingaben werden nie blockiert, bis eine Animation fertig ist. Neue Ziele übernehmen die aktuelle
   Geschwindigkeit.
6. **Physischer Impuls („Bump“)** bei Aktualisierung an Ort und Stelle (neue Mitteilung, Titelwechsel), ca. 3 %.
7. **Hover** skaliert die kompakte Island auf 1,035, **Drücken** auf 0,965 – beides als Feder.
8. **Keine Endlosschleifen** außer echtem Fortschritt (Spinner) und laufender Wiedergabe (Equalizer, stoppt bei Pause).
9. **Reduzierte Bewegung** (Windows-Einstellung oder Lab): Federn kritisch gedämpft und ≤ 0,3 s, keine Unschärfe,
   keine Skalierung, kein Versatz, keine Staffelung. Überblendungen bleiben.

### 11.4 Performance

Animiert werden `transform` (inkl. `scale`/`translate`), `opacity`, `filter`, Farben. Größenänderungen nur an der kleinen
Island-Hülle, deren Inhalt eine feste Endgröße hat. Laufende Werte werden pro Frame nur auf wenige, kleine Elemente
geschrieben – nie als geerbte Custom Property auf große Teilbäume.

---

## 12. Komponenten, Zustände, Barrierefreiheit

**Primitive** (ohne Fachwissen, später `@rice/ui`): `Tile` (groß mit zwei Zonen / klein), `Slider`, `Switch`, `Button`
(filled, tonal, text), `IconButton`, `Chip`, `Tabs`, `Row`, `SectionLabel`, `Progress`, `Check`, `Field`, `Meter`,
`Spark`, `Battery`, `AppIcon`, `Art`, `Equalizer`, `Empty`.
**Fachkomponenten** (bauen nur auf Primitiven auf): `NotificationCard`, `EventRow`, `TaskRow`, `NetworkRow`,
`DeviceRow`, `MediaCard`, `ProcessRow`, Island-Views.

| Zustand | Regel |
|---|---|
| Hover / Gedrückt | State-Layer (§5); Gedrückt zusätzlich leichtes Einsinken (Feder) |
| Fokus | Fokusring aus `--color-focus`, an **jedem** interaktiven Element, nur bei Tastaturfokus |
| An / Ausgewählt | Farbe **und** Form/Icon-Füllung, nie nur Farbe |
| Deaktiviert | 38 % Deckkraft + Erklärung im Untertitel („Nicht verfügbar“) |
| Lädt | Spinner nur an der betroffenen Stelle (Zeile, Abschnittslabel); das Layout springt nicht |
| Leer | Icon + Titel + ein freundlicher Satz („Alles erledigt · Keine neuen Mitteilungen“) |
| Fehler | Erklärt Ursache und nächsten Schritt („Standortfreigabe nötig → Einstellungen öffnen“) |

**Barrierefreiheit:** semantische Rollen (`button`, `switch`, `slider`, `tab`, `checkbox`), Labels an Icon-Buttons,
Treffer ≥ 32 px (Maus) · Pfeiltasten in Tabs und Slidern · Esc schließt · Kontraste nach §5 · Bedeutung nie nur über
Farbe.

---

## 13. Review-Checkliste (vor jedem Merge mit UI-Änderung)

- [ ] Keine Literalwerte in Komponenten (Farbe, Abstand, Radius, Schrift, Dauer, Kurve, Schatten)
- [ ] Neue Tokens in `tokens.css` **und** in allen Themes geprüft
- [ ] Farben paarweise (Fläche + `on`-Farbe)
- [ ] Radien konzentrisch, Abstände aus der Skala
- [ ] Alle Zustände umgesetzt: Hover, Gedrückt, Fokus, Deaktiviert, Lädt, Leer, Fehler, Ausgewählt
- [ ] In der Galerie in **allen Themes × Schemata** angesehen (nicht nur in einem)
- [ ] Mit längstem realistischem deutschen Text getestet
- [ ] Kontrast gemessen (vor allem `--color-text-muted`)
- [ ] Bewegung über Tokens, Ausgang schneller als Eingang, reduzierte Bewegung geprüft
- [ ] Tastaturbedienung und Labels vorhanden
- [ ] Ehrlicher Bericht: was geprüft wurde und was nicht

---

## 14. Island-Spezifikation

### 14.1 Ansichten

| Ansicht | Größe | Inhalt |
|---|---|---|
| `clock` (Leerlauf) | Inhalt × 34 | Uhrzeit (Terminal: mit Sekunden), Punkt bei ungelesenen Mitteilungen |
| `music` | Inhalt × 34 | Cover 22 px, Titel (max. 140 px) · Interpret, Equalizer in Albumfarbe |
| `music-expanded` | 392 × Inhalt | Cover 56 px, Titel, Interpret, Fortschritt, ⏮ ⏯ ⏭ |
| `notification` | 392 × Inhalt | App-Icon, App · Zeit, Titel, eine Zeile Text |
| `osd` | 264 × 42 | Icon (je nach Stufe), Balken, Wert |
| `event` | Inhalt × 34 | Fortschrittsring in Kalenderfarbe, Titel, „in 5 Min“ |
| `timer` | Inhalt × 34 | Timer-Icon, Restzeit |
| `charging` | ≥ 264 × 34 | „Wird geladen“ links, Prozent + Akku rechts |
| `panel` | 780 × 548 | Kontrollzentrum |

### 14.2 Arbitrierung

| Aktivität | Priorität | Lebensdauer | Klick öffnet |
|---|---|---|---|
| Mitteilung | 100 | 4,8 s (pausiert bei Hover) | Mitteilungen |
| OSD | 90 | 1,7 s nach letzter Änderung | – |
| Laden | 80 | 3,2 s | – |
| Termin | 60 | 14 s | Kalender |
| Timer | 50 | bis abgelaufen | – |
| Musik | 40 | solange sie läuft, 12 s nach Pause | – |

Die höchste Priorität besitzt die Island. Die nächsthöhere Aktivität mit Bubble-Darstellung **spaltet sich ab**
(Bubble 34 px, Abstand 7 px), aber nur, wenn die Haupt-Ansicht kompakt ist. In Themes mit `--island-goo: 1` geschieht das
flüssig (Metaball-Filter, nur während der Bewegung aktiv).

### 14.3 Interaktion

| Eingabe | Wirkung |
|---|---|
| Hover | Feder auf 1,035; Musik: nach 420 ms Verweilen → `music-expanded`, nach Verlassen 260 ms zurück |
| Klick | Panel öffnen (Tab der Aktivität) |
| Mausrad | Lautstärke ± 4 % mit OSD |
| Enter / Leertaste (fokussiert) | Panel umschalten |
| Esc, Klick daneben | Panel schließen |
| Klick auf Bubble | Panel mit dem Tab dieser Aktivität |

### 14.4 Panel

Kopf: große Uhr + Datum · Akku, Bearbeiten, Einstellungen, Energie.
Links (340 px): 4 große Kacheln (WLAN, Bluetooth, Nicht stören, Nachtlicht) · 4 kleine (Flugmodus, Energiesparen,
Mikrofon, Hotspot) · Lautstärke, Helligkeit · Medienkarte. Kacheln mit Details haben zwei Zonen: **Icon schaltet,
Text öffnet** die Detailseite, die die linke Spalte ersetzt (Zurück-Pfeil).
Rechts: Karte mit Tabs **Mitteilungen · Kalender · Aufgaben · System**. Der aktive Tab wächst und zeigt seine
Beschriftung, inaktive zeigen nur Icon (+ Zähler).
