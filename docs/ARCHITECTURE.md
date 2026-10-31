# Aufbau

## Wofür der Aufbau gedacht ist

Teacher Class pflegt Klassen, Schüler und Prüfungen und erzeugt daraus Excel-
Dateien. Fachliche Regeln — ein Klassenname besteht nur aus Buchstaben und
Zahlen, Punkte sind ganz oder halb, die Bewertungsmatrix hat keine Lücken —
müssen in der Oberfläche **und** auf der Kommandozeile gleich gelten. Darum
sieht der Aufbau so aus.

Eine Desktop-Anwendung mit Weboberfläche hat eine typische
Verfallsgeschichte: Die Oberfläche entsteht zuerst, und weil JavaScript gerade
offen ist, landen die Regeln dort — „Titel darf nicht leer sein" als `if` in
einer Komponente, eine SQL-Abfrage über ein Plugin direkt aus dem Webview.
Später kommt eine Kommandozeile dazu und bekommt eine eigene, leicht
abweichende Fassung derselben Regeln. Von da an gibt es jede Prüfung zweimal,
in zwei Sprachen.

Dazu kommt eine zweite Bruchstelle, die es in einer reinen Rust-Anwendung
nicht gibt: die **IPC-Grenze**. Ein umbenanntes Feld in Rust kompiliert, das
TypeScript kompiliert auch — und im Fenster steht `undefined`.

Dagegen hilft keine Ermahnung, sondern eine Anordnung, in der die Fehler
auffallen, bevor jemand klickt.

```
                          ┌──────────────────────┐
                          │   crates/core        │
      ┌──────────────────▶│   Fachlogik          │◀──────────────────┐
      │                   │   Prüfung · SQLite   │                   │
      │                   └──────────────────────┘                   │
      │                                                              │
┌─────┴──────────────┐                              ┌────────────────┴─────┐
│  crates/cli        │                              │  crates/desktop      │
│  clap              │                              │  Tauri-Befehle       │
│  Mensch │ --json   │                              │  ApiError            │
└────────────────────┘                              └──────────▲───────────┘
                                                               │ IPC (JSON)
                                                    ┌──────────┴───────────┐
                                                    │  ui/  (Webview)      │
                                                    │  api.ts · App.tsx    │
                                                    │  components · lib    │
                                                    └──────────────────────┘
```

Die Pfeile gehen nur in eine Richtung. `core` weiß nicht, dass es Schalen gibt.

## Was wo hingehört

| Frage | Antwort |
|---|---|
| Darf ein Klassenname ein Leerzeichen enthalten? | `core` |
| Wird ein ungültiger Klassenname rot umrandet? | `ui/src/components` |
| Welchen Rückgabewert liefert er auf der Kommandozeile? | `cli` |
| Sind 2,3 Punkte erlaubt? Wie wird das „bis“ einer Note berechnet? | `core` (`exam.rs`) |
| Wie sieht die Excel-Datei aus, welche Formeln stehen darin? | `core` (`export.rs`) |
| Wo wird die Excel-Datei gespeichert? | `ui` (Speichern-Dialog) → Pfad an `exam_export` |
| Wie heißt der IPC-Befehl, welche Argumente hat er? | `desktop/src/commands.rs` + `ui/src/api.ts` |
| Wie heißt die Spalte in der Datenbank? | `core` |
| Wie wird der Zeitstempel angezeigt? | die jeweilige Schale |
| Wie wird der Zeitstempel gespeichert? | `core` — immer UTC |

Die Faustregel: **Was auf beiden Wegen gleich sein muss, steht in `core`.**
Was nur mit Darstellung zu tun hat — Farben, Anordnung, Eingabefelder —, steht
in `ui/`. Auch **berechnete Werte** kommen aus `core`: Die Oberfläche rechnet
weder Gruppensummen noch das „bis“ einer Note nach, sondern zeigt, was Rust
nach dem Speichern liefert. Solange der Entwurf geändert ist, steht dort ein
Strich.

## Prüfungen und Excel-Export

```
ExamInput  ──exam::create/update──▶  Tabellen exams, exam_classes,
(Name, Klassen, Gruppen, Noten)       exam_groups, exam_items, exam_grades
                                              │
                                   exam::get ─┘  ──▶  Exam (mit berechneten Summen und „bis“)
                                                          │
                                         export::build ───┘  ──▶  .xlsx (Bytes)
```

* **Eine Prüfung wird als Ganzes geschrieben.** `create` und `update` nehmen
  den kompletten `ExamInput` und ersetzen Klassen, Gruppen, Punkte und Noten
  in einer Transaktion. Es gibt keine Einzeländerungen, also auch keinen
  halb gespeicherten Zustand.
* **Punkte sind Halbpunkte als ganze Zahl** (`max_half`, `from_half`; 2,5 wird 5).
  So rundet nichts, und „halb“ ist die kleinste Einheit. Nach außen sind es
  `f64`.
* **Von der Matrix wird nur das „ab“ gespeichert.** Das „bis“ berechnet `get`
  aus der nächsten Note. Lücken und Überschneidungen sind dadurch nicht
  darstellbar; geprüft wird nur noch, dass die niedrigste Note bei 0 beginnt
  und kein „ab“ doppelt vorkommt oder über der Gesamtpunktzahl liegt.
* **Excel rechnet, nicht die App.** `export.rs` schreibt Formeln, keine
  Ergebnisse: Summen mit `SUM`, die Note mit
  `INDEX(Noten, MATCH(Gesamtpunkte, ab, 1))` auf das Blatt „Bewertung“. Ändert
  die Lehrkraft dort ein „ab“, folgen alle Klassenblätter. Die Matrix steht
  deshalb aufsteigend nach „ab“ — `MATCH(…, 1)` verlangt das.
* **Bewertet wird in `grading.rs`, nicht in der Oberfläche.** Einträge
  (`exam_scores`: Punkte in Halbpunkten, `NULL` = nicht bewertet, plus
  Begründung) hängen an Prüfung, Schüler und der **Stelle** des Punktes im
  Schema (Gruppe, Punkt), nicht an einer Kennung: `exam::update` schreibt die
  Prüfung als Ganzes neu, die Kennungen der Punkte wechseln dabei. Danach
  räumt `grading::prune` auf — was nicht mehr passt, entfällt. Summen und Note
  berechnet `grading::results`; die Oberfläche und der Export zeigen das nur.
* **Die Form des Klassenblatts folgt `with_notes`.** Ohne Begründungen eine
  Matrix (Schüler in Zeilen), mit Begründungen eine Bewertungskachel je Schüler
  untereinander, vor jeder ein Seitenumbruch — ein Schüler, eine Druckseite,
  ohne Makro (`.xlsx`). Ein Blatt je Schüler wäre bei 90 Schülern
  unübersichtlich. Ein Blatt „Ausdruck“ mit Schülerauswahl und eine versteckte
  Hilfsliste gibt es nicht mehr.
* **Der Speichern-Dialog gehört zur Oberfläche.** `tauri-plugin-dialog` liefert
  den Pfad, `exam_export` schreibt die Datei und verlangt die Endung `.xlsx`.
  Dazu steht `dialog:allow-save` in `capabilities/default.json`.
* **Blattnamen:** Excel erlaubt höchstens 31 Zeichen. Ein Klassenname darf
  deshalb höchstens 24 Zeichen haben (`Name (2025)` = 24 + 7).

Die Formeln prüft `crates/core/tests/export.rs` nur **als Text** in der
erzeugten XML. Gerechnet hat sie dort niemand — ein Blick in Excel oder
LibreOffice bleibt nötig, wenn sich der Export ändert.

## Die IPC-Grenze

```
ui/src/api.ts                         crates/desktop/src/commands.rs
─────────────                         ──────────────────────────────
api.classes.update(id, draft)
  └▶ invoke("class_update",    ──▶    #[tauri::command]
            { id, input })            async fn class_update(state, id: Uuid, input: ClassInput)
                                        └▶ state.with(|app| class::update(app.conn(), id, input))
  ◀── Class                    ◀──    Ok(Class)           → JSON
  ◀── throw ApiError           ◀──    Err(ApiError)       → abgelehntes Promise
```

Vier Riegel halten diese Grenze zusammen:

1. **Typen aus Rust.** `#[derive(TS)]` an `Class`, `ClassInput`, `Student`,
   `Exam`, `ExamInput`, `Config`, `ApiError` … erzeugt `ui/src/bindings.ts`. `tests/bindings.rs` schlägt
   fehl, wenn die Datei veraltet ist. Danach zeigt `tsc`, wo die Oberfläche
   nicht mehr passt.
2. **Eine Befehlsliste.** `tests/ipc_contract.rs` liest `generate_handler![…]`
   aus `lib.rs` und die `call<…>("…")` aus `api.ts` und vergleicht.
3. **Echte Aufrufe ohne Fenster.** Derselbe Test ruft jeden Befehl über
   `tauri::test::MockRuntime` mit JSON-Argumenten auf — so, wie das Webview es
   tut. Ein falscher Argumentname fällt dort auf.
4. **Nur `api.ts` spricht mit Rust.** Der Schichtwächter verbietet
   `@tauri-apps/api/…` und `invoke(` überall sonst. Damit steht jeder Befehl
   genau einmal im Frontend, und Riegel 2 kann ihn finden.

### Warum `api.ts` Argumente einwortig hält

Tauri übersetzt Rust-Argumentnamen in camelCase: `class_id` erwartet im JSON
`classId`. Das ist dokumentiert, wird aber zuverlässig vergessen — von Menschen
und von Sprachmodellen. Einwortige Namen (`id`, `input`, `text`) machen die
Frage gegenstandslos.

### Warum eine eigene Fehlerhülle

`teacher_class_core::Error` enthält `rusqlite::Error` und Pfade — nicht
serialisierbar und nicht für die Oberfläche gedacht. `ApiError` hat genau die
Form von `error` im JSON der Kommandozeile:

```json
{ "code": "validation_failed", "message": "title: darf nicht leer sein",
  "fields": [{ "field": "title", "message": "darf nicht leer sein" }] }
```

Damit behandeln beide Schalen Fehler auf dieselbe Art: mit `fields` ans Feld
(CLI: Rückgabewert 4), ohne `fields` als allgemeine Meldung (CLI: 1).

### Wenn der Start scheitert

Ist `config.toml` kaputt, kann `App::open` nicht öffnen. Statt dass die
Anwendung beim Doppelklick kommentarlos verschwindet, öffnet sich das Fenster
trotzdem: `AppState::failed` hält den Fehler, jeder Befehl liefert ihn, und
die Oberfläche zeigt ihn oben an.

## Die Oberfläche: App.tsx und Komponenten

React zeichnet bei jeder Zustandsänderung neu. Der häufigste Fehler in einer
wachsenden Oberfläche ist ein `invoke` in einem `useEffect` tief in einer
Komponente — dann weiß niemand mehr, wann was gespeichert wird.

Deshalb:

```
components/ClassEditor.tsx  Props rein: draft, students, fieldMessage …
                            Rückrufe raus: onChange, onSave, onDelete …
App.tsx                     hält den Zustand, ruft api, verteilt Fehler
```

Komponenten *können* nicht speichern — sie importieren `api` nicht, der
Schichtwächter besteht darauf.

Der Entwurf einer Prüfung (`lib/exam.ts`) hält Punkte als **Text**, damit
„2,5“ und halbfertige Eingaben tippbar bleiben. Erst beim Abschicken wird
daraus eine Zahl; was keine Zahl ist, wird `-1`, und Rust lehnt es mit einer
Meldung am Feld ab. `NaN` ginge als `null` über die Grenze und scheiterte dort
ohne Hinweis auf das Feld.

## Markdown und Mermaid (derzeit ungenutzt)

Die Oberfläche zeigt aktuell keinen Benutzertext als Markdown. `lib/markdown.ts`,
`lib/mermaid.ts` und `MarkdownView` bleiben stehen, getestet, für spätere
Funktionen wie Bemerkungen zu Prüfungen. Wer Benutzertext als HTML anzeigt,
tut es nur darüber.

```
Text ──marked──▶ HTML ──DOMPurify──▶ sicheres HTML ──▶ .markdown
                                                              │
                              <pre><code class="language-mermaid"> gefunden?
                                                              │ ja
                                              import("mermaid") (einmal)
                                              parse → render → SVG ersetzt <pre>
```

* **Bereinigt wird immer.** Im Webview hätte ein eingeschleustes Skript
  Zugriff auf jeden IPC-Befehl — also auf `class_delete`.
* **Mermaid wird nachgeladen.** Vite legt es als eigene Stücke ins Bündel.
  Ein Text ohne Diagramm lädt es nie; die Anwendung startet ohne Wartezeit
  und ohne Netz.
* **Veraltete Darstellungen werden verworfen.** Wer tippt, erzeugt alle
  150 ms eine neue; eine langsame alte darf eine schnelle neue nicht
  überschreiben (`isCurrent` in `lib/mermaid.ts`).

## Daten

* **SQLite eingebettet** (`rusqlite` mit `bundled`). Kein Serverprozess.
* **WAL**, damit Desktop-Anwendung und Kommandozeile gleichzeitig laufen.
* **`STRICT`-Tabellen.**
* **Migrationen als nummerierte `.sql`-Dateien**, per `include_str!` im Binary.
* **UUIDv7 als Schlüssel** — siehe CLAUDE.md, „Kurzkennungen".
* **Kein SQL aus dem Webview.** `tauri-plugin-sql` wäre bequem und würde die
  Prüfung in `core` umgehen.

## Warum Tauri

| Statt | Grund gegen |
|---|---|
| egui (reines Rust) | Kein Zugang zu Browserpaketen wie mermaid, KaTeX, Chart.js, Monaco — für spätere Funktionen wie Diagramme in Auswertungen |
| Electron | Liefert Chromium und Node mit — 150 MB je Anwendung, zweite Laufzeit |
| Wails (Go) | Zweite Sprache neben der vorhandenen Rust-Fachlogik |
| Eine lokale Webanwendung im Browser | Kein Dock-Symbol, kein Dateisystem ohne Server, Port-Konflikte |

Der Preis ist ehrlich zu nennen:

* **Zwei Sprachen, zwei Builds.** npm und Cargo, Biome und Clippy.
* **Drei Webviews, zwei Engines.** WebKit auf macOS, WebKitGTK auf Linux,
  **WebView2 — also Chromium — auf Windows**. Die Oberfläche muss damit auf
  der ältesten davon laufen, und das ist in aller Regel WebKitGTK. Was in
  Chrome funktioniert, ist deshalb kein Beleg: Im Zweifel auf caniuse.com
  unter „Safari" nachsehen. Vor einer Auslieferung einmal unter Linux
  ansehen — dort fällt zuerst auf, was nur Chromium kann.
* **Linux braucht `libwebkit2gtk-4.1`** auf dem Zielrechner. Das `.deb`
  zieht es als Abhängigkeit, das `.AppImage` bringt es mit.
* **Nicht nativ.** Die Oberfläche sieht aus wie eine gute Webseite, nicht wie
  AppKit. Für Werkzeuge der richtige Tausch.
