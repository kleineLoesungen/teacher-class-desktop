# Regeln für dieses Projekt

Diese Datei ist für KI-Assistenten gedacht (Claude Code, Cursor, …) und
gleichzeitig als Kurzreferenz für Menschen brauchbar. Sie beschreibt, was in
diesem Projekt gilt und wo die Fallstricke liegen.

---

## Der Stack in einem Satz

Rust mit drei Kisten in einem Arbeitsbereich — `core` (Fachlogik, SQLite
eingebettet), `cli` (clap) und `desktop` (**Tauri 2**) — plus eine
Weboberfläche in `ui/` (**React 19, TypeScript, Vite**), die im Webview des
Betriebssystems läuft. Ziel: **macOS, Linux und Windows**. Browserpakete wie
**mermaid**, marked oder DOMPurify kommen über npm ins Bündel.

**Eine Plattform-Fallunterscheidung gibt es im Rust-Code nicht — kein einziges
`#[cfg(target_os = …)]`.** Das ist kein Zufall, sondern von
`crates/core/tests/layering.rs` bewacht. Siehe „Drei Systeme, ein Codepfad".

---

## Wo was liegt

```
crates/core/       Fachlogik und Prüfung. SQLite. Kennt weder clap noch tauri.
crates/cli/        Kommandozeile `teacher-class`. Dünn.
crates/desktop/    Tauri-Schale. Dünn. tauri.conf.json, capabilities/, icons/.
ui/src/api.ts      Die EINZIGE Stelle, die mit Rust spricht.
ui/src/App.tsx     Zustand der Oberfläche. Die einzige Komponente, die api benutzt.
ui/src/components/ Zeichnen nur. Props rein, Rückrufe raus.
ui/src/lib/        Reine Hilfsfunktionen: Markdown, Mermaid, Datum, Farbschema.
ui/src/bindings.ts ERZEUGT aus Rust-Typen. Nie von Hand ändern → `just bindings`.
ui/src/theme.css   Die EINZIGE Datei mit Farben.
scripts/tasks.mjs  Alles, was im Justfile keine schlichte Zeile wäre.
.claude/agents/    Die drei Prüf-Agents. Laufen in der CI auf jedem PR.
.github/workflows/ Bau auf drei Systemen, Entwicklung über Issues.
```

Es gibt **kein `src-tauri/`**. Die Tauri-Schale liegt in `crates/desktop`,
neben den anderen Kisten; die Tauri-CLI findet `tauri.conf.json` dort von
selbst. `npm run tauri …` immer aus der Wurzel aufrufen.

---

## Festgenagelte Versionen — bitte nicht „aktualisieren"

Rust-Versionen stehen in `[workspace.dependencies]` der obersten `Cargo.toml`.
Nur dort. npm-Versionen stehen **exakt** (ohne `^`) in `package.json`.

| Paket | Version | Grund |
|---|---|---|
| **tauri** / **@tauri-apps/api** / **@tauri-apps/cli** | **2.12** | Rust-Kiste und npm-Pakete müssen in derselben Nebenversion bleiben, sonst meldet `tauri dev` „version mismatch". Plugins (`tauri-plugin-opener` ↔ `@tauri-apps/plugin-opener`) ebenso paarweise. |
| **ts-rs** | **12** | Erzeugt `ui/src/bindings.ts`. Die API hat sich zwischen 10, 11 und 12 geändert (`Config`-Argument). |
| **mermaid** | **12.2** | Lädt nach (`import("mermaid")`). Die alte `mermaid.init()`/`contentLoaded`-API existiert nicht mehr — `parse` und `render` benutzen. |
| **DOMPurify** | **3.4.17** | Darunter liegen gemeldete XSS-Lücken. Diese Kiste ist die einzige Bereinigung zwischen Benutzertext und Webview — hier ist „alte Fassung" kein Schönheitsfehler. |
| **React** | **19** | `ReactDOM.render` gibt es nicht mehr — `createRoot`. |
| **TypeScript** | **7** | Der native Compiler. `tsc --noEmit` prüft nur, gebaut wird von Vite. |
| **Vite** / **Vitest** | **8** / **5** | Test-Einstellungen stehen in `vite.config.ts` unter `test`. |
| **Biome** | **2.6** | Formatierer und Linter in einem. Kein ESLint, kein Prettier. |
| **rusqlite** | **0.40** mit `bundled` | SQLite ins Binary. **Nicht** auf die Systembibliothek umstellen. |
| **Rust** | **1.98.1** | In `rust-toolchain.toml`. |

Wer eine Version anheben will: erst `just dupes` und `npm outdated`, dann
`just check`.

---

## Tauri 2 — die Fallstricke

Das ist der Teil, bei dem Sprachmodelle am zuverlässigsten danebenliegen: Die
meisten Beispiele im Netz stammen aus **Tauri 1**, und dort hieß fast alles
anders. Was aus dem Gedächtnis kommt, kompiliert oft nicht — oder kompiliert
und wird zur Laufzeit verweigert.

1. **Tauri 1 ist vorbei.** Die häufigsten Verwechslungen:

   | Tauri 1 ❌ | Tauri 2 ✅ |
   |---|---|
   | `import { invoke } from "@tauri-apps/api/tauri"` | `import { invoke } from "@tauri-apps/api/core"` |
   | `"allowlist": { … }` in tauri.conf.json | `crates/desktop/capabilities/*.json` |
   | `import { open } from "@tauri-apps/api/shell"` | Plugin: `@tauri-apps/plugin-opener` + `tauri-plugin-opener` |
   | `import { appWindow } from "@tauri-apps/api/window"` | `getCurrentWindow()` |
   | `"distDir"`, `"devPath"` | `"frontendDist"`, `"devUrl"` |
   | `window.__TAURI__` | nur mit `withGlobalTauri` — hier absichtlich aus; importieren |

2. **Befehle sind `async fn`.** Ein synchroner `#[tauri::command] fn` läuft
   auf dem Hauptfaden und friert das Fenster ein, solange er arbeitet. Ein
   `async`-Befehl mit geliehenem Argument (`State<'_, …>`) **muss** `Result`
   zurückgeben, sonst meldet der Compiler einen schwer lesbaren
   Lebensdauerfehler.

3. **Argumentnamen werden zu camelCase.** Rust `fn note_update(note_id: Uuid)`
   heißt in JavaScript `invoke("note_update", { noteId })`. Ein falscher Name
   ergibt zur Laufzeit „missing required key". Die Befehle hier haben deshalb
   einwortige Argumente (`id`, `input`, `order`, `config`). Befehlsnamen
   selbst bleiben snake_case.

4. **Neuer Befehl = drei Stellen.** `commands.rs` (Funktion),
   `lib.rs` (`generate_handler![…]`), `ui/src/api.ts` (`call<…>("name", …)`).
   `crates/desktop/tests/ipc_contract.rs` schlägt fehl, wenn eine fehlt.

5. **Plugins und Fenster-API brauchen eine Berechtigung.** Was nicht in
   `crates/desktop/capabilities/default.json` steht, wird zur Laufzeit mit
   „… not allowed" abgelehnt — obwohl alles kompiliert. Beispiel: Den
   Fenstertitel setzen braucht `core:window:allow-set-title`. Eigene Befehle
   aus `commands.rs` sind ohne Eintrag erlaubt.

6. **Der Fehlertyp muss `Serialize` sein.** `teacher_class_core::Error` ist es
   absichtlich nicht. Befehle geben `ApiResult<T>` zurück; `ApiError` hat
   dieselbe Form wie `error` im JSON der CLI (`code`, `message`, `fields`).

7. **Kein Netz, kein CDN.** Die CSP in `tauri.conf.json` erlaubt nur eigene
   Dateien. `<script src="https://cdn…">` lädt nicht — ohne sichtbaren Fehler.
   Pakete über npm installieren und importieren.

   **Inline-Stile und die Nonce.** Mermaid (und viele Diagramm-, Editor- und
   Chart-Pakete) setzen Stile inline. Dafür steht `'unsafe-inline'` in
   `style-src`. Tauri hängt beim Bündeln aber eine Nonce an `style-src` — und
   sobald eine Nonce da ist, **ignoriert der Browser `'unsafe-inline'`**. Im
   Entwicklungsbetrieb sieht alles richtig aus, im fertigen Bündel sind die
   Diagramme schwarze Flächen. Deshalb steht in `tauri.conf.json`
   `"dangerousDisableAssetCspModification": ["style-src"]`. Nicht entfernen.
   Für `script-src` bleibt die Nonce an — dort ist sie ein Schutz.

8. **Verweise öffnen nicht von selbst im Browser.** Ein `<a href>` im Webview
   navigiert das Anwendungsfenster weg. `MarkdownView` fängt Klicks ab,
   `api.openExternal` öffnet im Standardbrowser.

9. **Pfade kommen aus `core`, nicht aus Tauri.** `app.path().app_data_dir()`
   liefert `~/Library/Application Support/de.example.teacher-class` — die
   CLI sucht in `…/teacher-class`. Beide Schalen benutzen `teacher_class_core::Paths`,
   sonst sehen sie verschiedene Datenbanken.

10. **`ui/dist` muss existieren, bevor Rust kompiliert.** `generate_context!`
    bettet es ein. Nach `just clean` erst `npm run build:ui`. `just test` tut
    das von selbst.

11. **Testbinäre brauchen auf Windows ein eigenes Manifest.** `tauri-build`
    hängt das Anwendungsmanifest über `rustc-link-arg-bins` an — das erreicht
    nur das Programm, nicht `tests/`. Ohne Manifest aktiviert Windows Common
    Controls v6 nicht, und **jedes Testbinär, das Tauri wirklich benutzt,
    stirbt beim Laden** mit `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139) —
    bevor ein einziger Test läuft. Das Kit hängt es deshalb in
    `crates/desktop/build.rs` selbst an alle Ziele. Nicht entfernen, solange
    tauri-apps/tauri#13419 offen ist.

Im Zweifel nicht raten, sondern nachlesen — die Quellen liegen lokal:

```bash
grep -rn "pub fn get_ipc_response" ~/.cargo/registry/src/*/tauri-2.12.*/src/test/
ls node_modules/@tauri-apps/api/*.d.ts
ls crates/desktop/gen/schemas/          # alle Berechtigungen, nach dem ersten Bau
```

---

## Drei Systeme, ein Codepfad

Das Kit zielt auf macOS, Linux und Windows — entwickelt wird fast nur auf
macOS. Ein Unterschied, der nur auf Windows auffällt, fällt deshalb **niemandem**
auf. Die Regel ist darum nicht „Windows mitdenken", sondern
**Unterschiede gar nicht erst einbauen**.

### Keine Plattform-Fallunterscheidung im Rust-Code

`crates/core/tests/layering.rs` schlägt fehl, sobald irgendwo `cfg(target_os …)`,
`cfg(windows)`, `cfg(unix)` oder `target_family` auftaucht. Es gibt fast immer
einen Weg ohne — und der ist obendrein kürzer:

| Statt selbst zu unterscheiden | frag das, was es ohnehin weiß |
|---|---|
| `#[cfg(target_os)]` für Datenverzeichnisse | `directories` (siehe `paths.rs`) |
| Die Adresse der eigenen Oberfläche festnageln | `window.url()` (siehe `ipc_contract.rs`) |
| Pfade mit `/` vergleichen | einmal `to_slash()`, dann plattformblind |

Wer wirklich eine braucht, trägt die Datei in `PLATTFORM_AUSNAHMEN` ein **und
schreibt dazu, warum**. Dann steht die Ausnahme an einer Stelle, statt verstreut
im Quelltext.

Zwei Stellen bleiben, und beide stehen ausdrücklich dort, wo man sie sucht:

* `windows_subsystem` in `crates/desktop/src/main.rs` — hängt an
  `debug_assertions`, nicht am Betriebssystem, und tut anderswo nichts.
* `crates/desktop/build.rs` hängt das Windows-Manifest an alle Ziele
  (Fallstrick 11 oben). `/MANIFEST:EMBED` ist ein Schalter des
  MSVC-Linkers — dafür gibt es keine plattformfreie Schreibweise. Die Datei
  steht deshalb in `PLATTFORM_AUSNAHMEN`.

### Kein Bash in der Werkzeugkette

`just` ruft seine Rezepte über eine Shell auf, und die ist auf Windows `cmd.exe`
(`set windows-shell` im Justfile). Deshalb gilt:

**Jedes Rezept im Justfile ist ein schlichter Befehl.** Keine Pipes, kein
`rm -rf`, kein `VAR=x befehl`, kein `&&`. Alles, was mehr braucht, steht in
`scripts/tasks.mjs` — Node ist hier ohnehin Pflicht (npm, Vite, Biome), und
dieselbe Datei läuft auf allen drei Systemen.

```
just clean          →  node scripts/tasks.mjs clean
just sandbox …      →  node scripts/tasks.mjs sandbox …
```

**Windows braucht damit kein Git Bash.** Wer ein neues Rezept schreibt, prüft:
Ist es ein einzelner Programmaufruf? Wenn nein — nach `scripts/tasks.mjs`.

In den Node-Skripten gilt dasselbe Anliegen: `npm` und `npx` sind auf Windows
Batch-Dateien und brauchen `shell: true`, `cargo` nicht. Das steht einmal in
`BRAUCHT_SHELL` und sonst nirgends.

---

## Sprache im Code

* **Bezeichner englisch** — Typen, Funktionen, Felder, Tabellen, Befehle,
  Komponenten, CSS-Klassen: `NoteInput`, `note_update`, `NoteEditor`,
  `.note-item`.
* **Kommentare, Doku, Oberflächentexte und Testnamen deutsch.**
  `fn ein_titel_aus_leerzeichen_gilt_als_leer()`,
  `it("zeigt einen Eingabefehler am Feld, nicht oben")`.

---

## Die eine wichtige Schichtregel

```
crates/core/        →  FACHLOGIK UND PRÜFUNG. Kennt WEDER clap NOCH tauri NOCH stdout.
crates/cli/         →  dünn. Argumente parsen, core rufen, Ausgabe formatieren.
crates/desktop/     →  dünn. IPC annehmen, core rufen, JSON zurück. Kein SQL.
ui/src/api.ts       →  die einzige Brücke nach Rust.
ui/src/App.tsx      →  Zustand. Wünsche der Komponenten → api.
ui/src/components/  →  zeichnet nur. Sieht api nicht.
```

**`crates/core/tests/layering.rs` schlägt fehl**, sobald

* in `core` ein `println!`, `clap`, `tauri` oder `std::process::exit` steht,
* in `desktop` ein `execute(`, `query_row`, `prepare(` oder `Validator` steht,
* außerhalb von `api.ts` `@tauri-apps/api/…`, `@tauri-apps/plugin-…` oder
  `invoke(` steht,
* eine Komponente `api` importiert oder `innerHTML` setzt,
* irgendwo `dangerouslySetInnerHTML` oder ein CDN auftaucht,
* außerhalb von `theme.css` eine Hexfarbe oder `rgb(` steht,
* **irgendwo im Rust-Code eine Plattform-Fallunterscheidung** steht
  (`cfg(target_os …)`, `cfg(windows)`, `cfg(unix)` — siehe unten).

Den Code verschieben, nicht den Test lockern.

Der Grund ist nicht Sauberkeit: **Alles, was in `core` steht, gilt für die
Kommandozeile und die Oberfläche gleichermaßen.** Eine Prüfung in TypeScript
fehlt der CLI. Genau das ist der Fehler, den dieses Kit verhindern soll.

### Prüfung gehört in die Fachlogik — nicht nach TypeScript

```rust
// ✅ crates/core/src/note.rs
pub fn create(conn: &Connection, input: NoteInput) -> Result<Note> {
    let input = input.clean()?;   // trimmt und prüft, wirft Error::Validation
    …
}
```

```tsx
// ❌ in einer Komponente
if (draft.title.trim() === "") setError("Titel fehlt");
```

Die Oberfläche schickt ab und zeigt an, was zurückkommt. Eine *zusätzliche*
Vorabprüfung für schnellere Rückmeldung ist erlaubt, ersetzt aber nie die in
`core` — und die Meldung kommt trotzdem aus Rust.

---

## Die IPC-Grenze

```
Komponente ──onSave()──▶ App.tsx ──api.notes.update(id, draft)──▶ invoke("note_update", {id, input})
                                                                          │
     ◀── Note  oder  ApiError { code, message, fields? } ◀── commands.rs ─┴─▶ teacher_class_core
```

**Typen werden nicht doppelt geschrieben.** Rust-Typen, die über IPC gehen,
tragen `#[derive(TS)]`. Daraus entsteht `ui/src/bindings.ts`:

```bash
just bindings        # nach jeder Änderung an einem solchen Rust-Typ
```

`crates/desktop/tests/bindings.rs` schlägt fehl, wenn die Datei veraltet ist.
Ein neuer Typ wird dort in `generate()` eingetragen.

Zeitstempel sind in TypeScript `string` (RFC 3339, UTC). Umgerechnet wird nur
zur Anzeige (`lib/format.ts`).

---

## Zwei Sorten Fehler

| Sorte | Woran erkennbar | Wohin damit |
|---|---|---|
| Eingabefehler | Rust: `err.fields()` ist `Some`. TS: `apiError.isValidation` | CLI: Rückgabewert 4. Oberfläche: **am Feld** (`fieldMessage("title")`). |
| Alles andere | sonst | CLI: Rückgabewert 1. Oberfläche: Leiste oben (`banner`). |

`ErrorCode` (`not_found`, `validation_failed`, …) steht in
`crates/core/src/error.rs` und gilt für CLI-JSON und Oberfläche gleich.
Namen werden nie umbenannt, nur ergänzt.

---

## Die Oberfläche

* **Komponenten zeichnen nur.** Sie bekommen Daten als Props und melden
  Wünsche über Rückrufe (`onSave`, `onDelete`). `App.tsx` ruft `api`.
* **Kein globaler Zustandsspeicher** (Redux, Zustand, …). Ein `useState` in
  `App.tsx` reicht für ein Werkzeug dieser Größe. Wird es mehr, zuerst
  Hilfsfunktionen in `lib/` auslagern.
* **Kein CSS-Framework.** Farben und Abstände als Variablen in `theme.css`,
  Aufbau in `app.css`. Tailwind bräuchte einen eigenen Build-Schritt und
  verteilt Farben über jede Komponente — das Gegenteil der Farbregel.
* **HTML aus Benutzertext nur über `lib/markdown.ts`.** Dort bereinigt
  DOMPurify. Ein Skript im Webview hätte Zugriff auf jeden IPC-Befehl.
* **Große Pakete nachladen.** `import("mermaid")` statt `import mermaid` —
  sonst startet die Anwendung mit mehreren Megabyte JavaScript, die sie
  vielleicht nie braucht. Siehe `lib/mermaid.ts`.
* **Bedienelemente brauchen Namen** (`aria-label` oder sichtbarer Text). Die
  Tests suchen danach (`getByRole("button", { name: "Speichern" })`), und ein
  Bildschirmleser ebenso. Zwei gleichnamige Knöpfe gleichzeitig sind für
  beide mehrdeutig — deshalb „Endgültig löschen" in der Rückfrage.
* **Zwei Engines, und die ältere gibt das Maß vor.** macOS und Linux zeichnen
  mit WebKit (Safari bzw. WebKitGTK), Windows mit **WebView2 — das ist
  Chromium**. Was in Chrome läuft, läuft deshalb noch lange nicht überall.
  **Im Zweifel auf caniuse.com unter „Safari" nachsehen, nicht unter Chrome.**
  WebKitGTK hinkt Safari obendrein oft eine Ausgabe hinterher.

---

## Kurzkennungen: hinten, nicht vorn

Die Kennungen sind UUIDv7. Die **beginnt mit dem Zeitstempel** — zwei Notizen
aus derselben Sekunde teilen sich die ersten zwölf Zeichen. Darum zeigt
`teacher-class note list` die **letzten** acht Zeichen, und `note::resolve_id` sucht
per Endstück. Ist ein Endstück nicht eindeutig, gibt es `Error::Ambiguous` —
nie einen zufälligen Treffer.

---

## Pfade, Zeit, Daten

* **Die Fallunterscheidung macOS/Linux steht nur in `crates/core/src/paths.rs`.**
* `TEACHER_CLASS_DATA_DIR` und `TEACHER_CLASS_CONFIG_DIR` überschreiben die Verzeichnisse
  — für CLI **und** Desktop (`just dev-sandbox`, `just sandbox`). In
  Rust-Tests stattdessen `Paths::under(tempdir)`.
* Wo die Dateien landen, entscheidet `directories`, nicht das Kit:

  | | macOS | Linux | Windows |
  |---|---|---|---|
  | Daten | `~/Library/Application Support/teacher-class` | `~/.local/share/teacher-class` | `%APPDATA%\teacher-class\data` |
  | Konfig | dasselbe Verzeichnis | `~/.config/teacher-class` | `%APPDATA%\teacher-class\config` |
* **Gespeichert wird in UTC**, als RFC-3339-Text.
* **Eine veröffentlichte Migration wird nie geändert.** Neue Datei, nächste
  Nummer, in `crates/core/src/db.rs` eintragen.
* Die Tabellen sind `STRICT`.
* **WAL** — Desktop-Anwendung und CLI dürfen gleichzeitig laufen.

---

## Was hier NICHT benutzt wird

Bitte nicht „nachrüsten":

| Nicht | Stattdessen | Warum |
|---|---|---|
| `src-tauri/` | `crates/desktop/` | Eine Kiste unter vielen, dieselben Regeln |
| Electron | Tauri | 150 MB Chromium je Anwendung |
| Tauri 1-APIs, `allowlist` | Tauri 2, capabilities | siehe oben |
| SQL-Plugin (`tauri-plugin-sql`) | `teacher_class_core` | SQL aus dem Webview umgeht die Prüfung in core — und die CLI sähe es nie |
| `tauri-plugin-store` für Einstellungen | `config.toml` über `core` | Die CLI muss dieselben Einstellungen lesen |
| Next.js, Remix, SSR | Vite + React | Es gibt keinen Server |
| Redux, Zustand, MobX | `useState` in `App.tsx` | Zu klein dafür |
| Tailwind, styled-components | `theme.css` + `app.css` | Farben an einer Stelle |
| ESLint + Prettier | Biome | Ein Werkzeug, keine Plugin-Versionskonflikte |
| Jest | Vitest | Teilt die Vite-Konfiguration |
| Pakete von einem CDN | npm | Die Anwendung muss ohne Netz starten |
| `dangerouslySetInnerHTML` | `lib/markdown.ts` | Bereinigung an einer Stelle |
| Auto-Update-Plugin | Release auf GitHub | Braucht Signaturschlüssel und Server |
| `unsafe` | — | `unsafe_code = "forbid"` im Arbeitsbereich |
| Bash-Skripte, `rm -rf` im Justfile | `scripts/tasks.mjs` | Auf Windows läuft `cmd.exe`, nicht `sh` |
| `#[cfg(target_os = …)]` | `directories`, `window.url()` | Ein Unterschied, den niemand auf macOS sieht, ist ein Fehler für später |

---

## Tests

| Wo | Was |
|---|---|
| `crates/core/tests/note.rs` | Die fachlichen Regeln. **Hier liegt der Schwerpunkt.** |
| `crates/core/tests/config.rs` | Einstellungen lesen, schreiben, ablehnen |
| `crates/core/tests/layering.rs` | Der Schichtwächter — Rust **und** TypeScript, dazu der Plattform-Wächter |
| `crates/cli/tests/cli.rs` | Ausgabeformat, Rückgabewerte, Schnappschüsse |
| `crates/desktop/tests/ipc_contract.rs` | Befehle über `tauri::test` ohne Fenster; Befehlsliste Rust ↔ api.ts |
| `crates/desktop/tests/bindings.rs` | `ui/src/bindings.ts` ist aktuell |
| `ui/src/App.test.tsx` | Oberfläche durchklicken, Rust durch `mockIPC` ersetzt |
| `ui/src/lib/lib.test.ts` | Markdown-Bereinigung, Verweise, Farben, Fehlerhülle |

**Eine Regel wird einmal geprüft — in `core`.** Die Attrappe in
`App.test.tsx` hat keine eigenen Regeln; sie lehnt nur ab, was der Test ihr
sagt.

Ändert sich ein CLI-Ausgabeformat absichtlich: `just snapshots`.

---

## Vor jedem Commit

```bash
just check
```

Das ist `cargo fmt --check`, Clippy, Biome, `tsc`, alle Rust- und
Vitest-Tests, `cargo deny` und `npm audit`. Warnungen sind Fehler.

Zwei Einstellungen darin sind bewusst so und schon einmal teuer gewesen:

* **`npm audit` ohne `--omit=dev`.** Entwicklungswerkzeuge landen nicht im
  Bündel, aber `just dev` startet den Vite-Server auf dem eigenen Rechner —
  eine Lücke darin trifft den, der am Projekt arbeitet. Genau so eine ist hier
  schon durchgerutscht. Wer das Rauschen nicht erträgt, hebt `--audit-level`,
  statt die halbe Liste auszublenden.
* **`unsound = "workspace"` in `deny.toml`.** Unsoundness in den eigenen
  Abhängigkeiten bricht den Bau, tiefer im Baum nicht — dort stecken zwei, die
  sich von hier aus nicht auflösen lassen. Echte Sicherheitslücken brechen den
  Bau **immer**, auch transitiv.

### Wenn du aus einem Issue heraus arbeitest

Dann läufst du in der CI, und der Mensch sitzt wahrscheinlich am Telefon.
Zwei Dinge folgen daraus:

1. **`just check` laufen lassen, bevor du einen PR öffnest.** Die
   Werkzeugkette ist im Workflow installiert. Ein PR, der erst in der CI
   rot wird, kostet eine ganze Runde — und die dauert, wenn der Mensch
   unterwegs ist.
2. **Schreib in die PR-Beschreibung, was du nicht prüfen konntest.** „Auf
   Windows nicht ausprobiert", „Diagramm nur im Test gesehen, nicht im
   Fenster". Lieber ein ehrlicher Vorbehalt als ein stilles Risiko.

Drei Prüf-Agents sehen deinen PR danach durch (`.claude/agents/`). Ihre
Prüflisten sind kein Geheimnis — lies sie vorher, dann sparst du euch beiden
eine Runde. Der ganze Ablauf steht in `docs/AUTOMATION.md`.

#### Was du nicht ändern darfst

* **`.claude/agents/*.md`** — die Prüflisten, an denen deine eigene Arbeit
  gemessen wird. Wer sie umschreiben darf, entwertet die Prüfung. Dieselbe
  Regel wie bei `crates/core/tests/layering.rs`: **den Code ändern, nicht den
  Maßstab.**
* **`.github/workflows/*.yml`** — technisch verwehrt: Der `GITHUB_TOKEN` darf
  Workflow-Dateien nicht anfassen.

Fällt dir dort etwas auf — ein veralteter Verweis, eine fehlende Prüfzeile,
ein zu enges Recht —, dann **schreib es in die PR-Beschreibung**, mit der
Datei und dem Vorschlag im Wortlaut. Der Mensch trägt es nach. Das ist kein
Umweg, sondern die Stelle, an der jemand anders hinsehen soll.

#### Wenn ein früherer Lauf schon etwas angefangen hat

Jeder Lauf startet auf einem **frischen** Branch von `main`. Hat ein
vorheriger Lauf Arbeit auf einem `claude/*`-Branch hinterlassen, liegt sie
dort — nicht in deinem Arbeitsverzeichnis. Sieh nach, bevor du anfängst:

```bash
git fetch origin
git log --oneline --all --grep="" origin/claude/ 2>/dev/null | head
```

Übernimm sie mit **`git cherry-pick <sha>`** oder `git merge`, **nie** mit
`git checkout origin/<branch> -- .`: Der alte Branch steht auf einem älteren
`main`, und ein pauschales Übernehmen brächte Dateien zurück, die seitdem
gelöscht wurden — oder löschte neue, die er nicht kennt. Ein `cherry-pick`
wendet nur die Änderung dieses Commits an.

`git push` ist nicht freigegeben und wird auch nicht gebraucht; die Action
committet und schiebt selbst.

#### Rückfragen: selten, dafür gut

Jeder Kommentar mit `@claude` startet einen **neuen** Lauf, der das Issue
und **alle** bisherigen Kommentare liest. Der Kommentarverlauf ist also das
Gespräch — eine Rückfrage geht nicht verloren.

Sie kostet aber eine Runde, und am anderen Ende sitzt jemand am Telefon.
Deshalb:

* **Entscheide selbst und sag, was du entschieden hast.** Für alles, wofür
  es eine vertretbare Voreinstellung gibt. „Ich habe das Feld als `TEXT NOT
  NULL` angelegt, weil …" ist besser als eine Rückfrage, die den Menschen
  zwingt, dasselbe zu tippen.
* **Frag nur, wenn die Antwort etwas anderes aus der Arbeit macht** — ein
  anderes Datenmodell, ein anderer Ablauf, eine Entscheidung, die sich
  später teuer rückgängig machen lässt.
* **Wenn du fragst: nicht offen fragen.** Zwei oder drei benannte
  Möglichkeiten mit einer Empfehlung, beantwortbar mit einem Wort. „Soll die
  Klasse mehrere Lehrkräfte haben? (a) eine, (b) mehrere — ich würde (a)
  nehmen, weil …"
* **Eine Rückfrage ist kein Pull Request.** Öffne keinen, wenn du nicht
  umgesetzt hast. Ein PR, der in Wahrheit eine Frage ist, verstopft die
  Durchsicht und sieht nach Fortschritt aus, wo keiner ist.
* **Teile Unfertiges mit.** Wenn du nur die Hälfte umsetzen konntest, schreib
  in die PR-Beschreibung, was fehlt und warum — nicht in eine Rückfrage, die
  alles blockiert.
* **Öffne für Unfertiges einen Entwurfs-PR**, statt den Branch unkommentiert
  liegen zu lassen. Dann ist er sichtbar, prüfbar und wiederauffindbar — und
  der nächste Lauf weiß, woran er anknüpft.

---

## Eine neue Ressource anlegen

`note` ist die Vorlage. Der Weg von der Tabelle bis in beide Schalen:

1. `crates/core/migrations/000X_….sql` — neue Datei, `STRICT`.
2. `crates/core/src/db.rs` — in `MIGRATIONS` eintragen.
3. `crates/core/src/note.rs` kopieren, umbenennen; `#[derive(TS)]` an die
   Typen, die über IPC gehen.
4. `crates/core/src/lib.rs` — Modul veröffentlichen.
5. Tests in `crates/core/tests/` — **zuerst**.
6. CLI: `crates/cli/src/args.rs` und `commands/` — beide Ausgabefassungen.
7. Desktop: `commands.rs` → `lib.rs` (`generate_handler!`) →
   `tests/bindings.rs` (Typ eintragen) → `just bindings`.
8. Oberfläche: `api.ts` → Komponente in `components/` → Zustand und Aufruf in
   `App.tsx`.

Ausführlich mit Code: `docs/RECIPES.md`.
