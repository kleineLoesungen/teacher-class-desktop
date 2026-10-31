# Rezepte

Konkrete Handgriffe mit Code zum Abschauen. `class` (einfach) und `exam`
(mit Unterstruktur und Export) sind überall die Vorlage. Die Beispiele sind
Skizzen für mögliche Erweiterungen; was in der App schon steht, sagen die
Quelldateien.

---

## Den Excel-Export ändern

Der Export steht ganz in `crates/core/src/export.rs`; die Oberfläche und die
Kommandozeile rufen nur `export::write_xlsx`. Der Aufbau:

| Funktion | Schreibt |
|---|---|
| `write_grading` | Blatt „Bewertung“ und merkt sich, **wo** Schema und Matrix stehen (`GradingRefs`) |
| `write_class` | Ein Klassenblatt als Matrix (Prüfung ohne Begründungen) |
| `write_tiles`, `write_tile` | Ein Klassenblatt als Bewertungskacheln, eine je Schüler (Prüfung mit Begründungen) |

Regeln, die sich bewährt haben:

* **Formeln statt Werte** für alles, was gerechnet wird. Die Klassenblätter
  verweisen auf „Bewertung“ (`=Bewertung!$C$6`), damit eine Änderung dort
  überall ankommt. Eingetragene Punkte und Begründungen sind dagegen **Werte**
  aus `grading::results`.
* **Zellen nie von Hand zählen.** Zeilen und Spalten kommen aus `GradingRefs`
  und den Zeilen, die `write_tile` selbst mitführt; Adressen aus
  `row_col_to_cell`. Sonst verschiebt eine neue Spalte stillschweigend alle
  Formeln dahinter.
* **Kacheln drucken:** Seitenumbrüche vor jeder Kachel (`set_page_breaks`) und
  ein fester Druckmaßstab (`set_print_scale`). Mit „auf Seiten anpassen“
  ignoriert Excel manuelle Umbrüche.
* **Ein neues Blatt** in `build` anlegen und mit `push_worksheet` in der
  gewünschten Reihenfolge anhängen.
* **Test in `crates/core/tests/export.rs`**: Die erzeugte `.xlsx` ist ein ZIP;
  der Test liest `xl/worksheets/sheetN.xml` und sucht die Formel als Text.
  Das fängt verrutschte Adressen, aber keine falsche Rechnung.
* **Danach einmal in Excel oder LibreOffice öffnen.** Ein Beispiel mit Halb­
  punkten ausfüllen und prüfen, dass Summe und Note stimmen und – bei einer
  Prüfung mit Begründungen – jede Kachel auf genau eine Seite passt. Das kann
  kein Test ersetzen.

---

## Ein Browserpaket einbinden

Beispiel: ein Diagramm der Notenverteilung mit Chart.js.

### 1. Installieren — exakt, nicht von einem CDN

```bash
npm install --save-exact chart.js
```

`<script src="https://cdn…">` in `index.html` lädt nicht: Die CSP in
`crates/desktop/tauri.conf.json` erlaubt nur eigene Dateien, und eine
Desktop-Anwendung muss ohne Netz starten. Der Schichtwächter weist darauf hin.

### 2. In `ui/src/lib/` kapseln

Eine Datei je Paket. Komponenten importieren die Hilfsfunktion, nicht das
Paket — so gibt es genau eine Stelle, an der Einstellungen und Eigenheiten
stehen.

```ts
// ui/src/lib/chart.ts
// Nachgeladen wie Mermaid: nur wer ein Diagramm sieht, bezahlt dafür.
type ChartModule = typeof import("chart.js/auto");
let loading: Promise<ChartModule> | undefined;

export async function drawDistribution(canvas: HTMLCanvasElement, counts: number[]) {
  loading ??= import("chart.js/auto");
  const { default: Chart } = await loading;
  return new Chart(canvas, {
    type: "bar",
    data: { labels: counts.map((_, i) => String(i + 1)), datasets: [{ data: counts }] },
  });
}
```

**Nachladen oder nicht?** Faustregel: Alles über ~100 kB, das nicht auf dem
ersten Bildschirm gebraucht wird, per `import()`. `npm run build:ui` zeigt die
Größen.

### 3. Aufrufen, wo gezeichnet wird

```tsx
// in einer Komponente, im useEffect, sobald das <canvas> da ist
void drawDistribution(canvasRef.current, counts);
```

Die Zahlen für das Diagramm kommen aus `core` — die Oberfläche zählt keine
Noten nach (siehe CLAUDE.md, „Prüfung gehört in die Fachlogik").

### 4. Farben des Pakets

Pakete bringen oft eigenes CSS oder eigene Farben mit. Hexfarben im eigenen
Quelltext (`ui/src`) verbietet der Farbwächter; Farben holt man aus den
Variablen: `getComputedStyle(document.documentElement).getPropertyValue("--accent")`.
CSS aus `node_modules` zu importieren ist erlaubt.

### 5. Wenn das Paket etwas Ungewöhnliches braucht

| Meldung / Verhalten | Ursache | Abhilfe |
|---|---|---|
| Leere Fläche, in der Konsole „Refused to load" | CSP | Quelle in `tauri.conf.json` unter `security.csp` ergänzen — **nie** `*` |
| „Refused to evaluate a string as JavaScript" | Paket benutzt `eval` / `new Function` | Anderes Paket suchen. `'unsafe-eval'` öffnet die Tür für eingeschleusten Code |
| Web Worker startet nicht | Worker-URL | `new Worker(new URL("./x.ts", import.meta.url), { type: "module" })` — Vite bündelt ihn dann mit; ggf. `worker-src 'self' blob:` |
| Funktioniert in Chrome, nicht in der App | WebKit | caniuse.com unter „Safari" prüfen |
| Test scheitert: „… is not a function" in jsdom | jsdom ist kein Browser (kein Canvas, kein `getBBox`) | Die Funktion in `lib/` im Test mit `vi.mock` ersetzen |

Die Konsole des Webviews: in `just dev` Rechtsklick → **Element untersuchen**.

---

## Einen IPC-Befehl hinzufügen

Beispiel: Schüler einer Klasse nach einem Namen durchsuchen.

### 1. Die Logik nach `core` — mit Test

```rust
// crates/core/src/student.rs
pub fn search(conn: &Connection, class_id: Uuid, query: &str) -> Result<Vec<Student>> {
    class::get(conn, class_id)?;
    let pattern = format!("%{}%", query.trim());
    let mut stmt = conn.prepare(
        "SELECT id, class_id, last_name, first_name FROM students \
         WHERE class_id = ?1 AND (last_name LIKE ?2 OR first_name LIKE ?2) \
         ORDER BY last_name COLLATE NOCASE, first_name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(rusqlite::params![class_id.to_string(), pattern], from_row)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()?.into_iter().collect()
}
```

Die Suche gehört nach `core` — sonst findet die Oberfläche anders als die
Kommandozeile.

### 2. Der Befehl — dünn, `async`, einwortige Argumente

```rust
// crates/desktop/src/commands.rs
#[tauri::command]
pub async fn student_search(
    state: State<'_, AppState>,
    id: Uuid,
    text: String,
) -> ApiResult<Vec<Student>> {
    state.with(|app| student::search(app.conn(), id, &text))
}
```

### 3. Eintragen

```rust
// crates/desktop/src/lib.rs
.invoke_handler(tauri::generate_handler![
    …
    commands::student_search,
])
```

### 4. Im Frontend

```ts
// ui/src/api.ts
students: {
  …
  search: (classId: string, text: string) =>
    call<Student[]>("student_search", { id: classId, text }),
},
```

Der Aufruf `call<…>("name"` muss in **einer Zeile** stehen, sonst findet ihn
`jeder_befehl_steht_auf_beiden_seiten` nicht.

### 5. Prüfen

```bash
just check
```

`jeder_befehl_steht_auf_beiden_seiten` meldet, wenn Schritt 3 oder 4 fehlt.
Einen Aufruf in `tests/ipc_contract.rs` ergänzen, der das JSON so schickt wie
das Webview:

```rust
let found = ctx
    .invoke("student_search", json!({ "id": class_id, "text": "Muster" }))
    .unwrap();
```

---

## Ein Argument mit zwei Wörtern

Wenn es sich nicht vermeiden lässt:

```rust
pub async fn exam_copy(state: State<'_, AppState>, id: Uuid, target_class: Uuid) -> …
```

```ts
call<Exam>("exam_copy", { id, targetClass })   // camelCase!
```

Oder in Rust die Übersetzung abschalten — dann gilt snake_case auch im JSON:

```rust
#[tauri::command(rename_all = "snake_case")]
```

---

## Einen Rust-Typ über IPC schicken

```rust
#[derive(Debug, Clone, Serialize, ts_rs::TS)]
pub struct Stats {
    #[ts(type = "number")]                  // i64 würde in TypeScript bigint
    pub exams: i64,
    #[ts(type = "string")]                  // OffsetDateTime → RFC-3339-Text
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    pub last_change: OffsetDateTime,
}
```

In `crates/desktop/tests/bindings.rs` unter `generate()` eintragen, dann:

```bash
just bindings
```

`i64` wird in TypeScript zu `bigint`. Serde schickt aber eine JSON-Zahl. Für
Zähler, die nie über 2⁵³ gehen: `#[ts(type = "number")]`.

---

## Ein Tauri-Plugin benutzen

So ist der Speichern-Dialog für den Excel-Export eingebaut
(`tauri-plugin-dialog`). Ein weiteres Plugin geht genauso.

```bash
npm install --save-exact @tauri-apps/plugin-dialog
```

```toml
# Cargo.toml, [workspace.dependencies] — dieselbe Nebenversion wie das npm-Paket
tauri-plugin-dialog = "2.8.4"
# crates/desktop/Cargo.toml
tauri-plugin-dialog.workspace = true
```

```rust
// crates/desktop/src/lib.rs, in configure()
.plugin(tauri_plugin_dialog::init())
```

```json
// crates/desktop/capabilities/default.json → permissions
"dialog:allow-save"
```

```ts
// ui/src/api.ts — und NUR dort
import { save } from "@tauri-apps/plugin-dialog";
…
pickExportPath: (fileName: string) =>
  save({ defaultPath: fileName, filters: [{ name: "Excel-Arbeitsmappe", extensions: ["xlsx"] }] }),
```

Das **Schreiben** der Datei steht in einem eigenen Befehl (`exam_export`) mit
der Logik in `core` — nicht in `tauri-plugin-fs` aus dem Webview. Sonst kann die
Kommandozeile nicht exportieren (`exam export --out`).

Die Oberfläche ruft erst den Dialog, dann den Befehl. Im Test ersetzt
`mockIPC` beides: der Dialog heißt dort `plugin:dialog|save`.

Fehlt Schritt „capabilities", kompiliert alles, und der Aufruf scheitert zur
Laufzeit mit „dialog.save not allowed". Die möglichen Namen stehen nach dem
ersten Bau in `crates/desktop/gen/schemas/desktop-schema.json`.

---

## Etwas im Hintergrund erledigen

Ein Befehl ist `async`, aber die Datenbankarbeit darin ist es nicht. Für
etwas, das Sekunden dauert (Import, Export, Neuberechnung), die Sperre nicht
den ganzen Weg halten, sondern auf einen Blockierfaden ausweichen:

```rust
#[tauri::command]
pub async fn import_folder(state: State<'_, AppState>, path: String) -> ApiResult<usize> {
    // Einlesen ohne Sperre …
    let files = tauri::async_runtime::spawn_blocking(move || teacher_class_core::import::read(&path))
        .await
        .map_err(|e| ApiError::internal(e.to_string()))??;
    // … schreiben mit Sperre, kurz.
    state.with(|app| teacher_class_core::import::store(app.conn(), files))
}
```

Fortschritt meldet man mit einem Kanal (`tauri::ipc::Channel<T>`) als
Argument; im Frontend `new Channel<T>()` aus `@tauri-apps/api/core` — in
`api.ts`.

---

## Eine neue Ressource anlegen

Beispiel: `task` mit Titel und Erledigt-Kennzeichen. Vorlage für eine
einfache Ressource ist `class.rs`; für eine mit Unterstruktur, die als Ganzes
geschrieben wird, `exam.rs`.

1. **Migration** `crates/core/migrations/0004_tasks.sql` (die nächste freie
   Nummer), `STRICT`, eintragen in `crates/core/src/db.rs`. Veröffentlichte
   Migrationen werden nie geändert.
2. **Modul** `class.rs` → `task.rs` kopieren, `#[derive(TS)]` an `Task` und
   `TaskInput`, `pub mod task;` in `lib.rs`. Soll die Kurzkennung auflösbar sein:
   Eintrag in `ids::Table`.
3. **Tests** `crates/core/tests/task.rs` — zuerst.
4. **CLI** `args.rs` + `commands/task.rs`, beide Ausgabefassungen.
5. **Befehle** `task_list`, `task_create`, `task_toggle` in `commands.rs`,
   `generate_handler!`, Typen in `tests/bindings.rs`, `just bindings`.
6. **Frontend** `api.tasks.*` in `api.ts`, `components/TaskList.tsx` (nur
   zeichnen, `onToggle` nach oben), Zustand und Aufruf in `App.tsx`.

### Und wenn `note` ganz verschwindet

Sobald die Beispielressource durch eigene ersetzt wird, zeigen zwei Stellen
ins Leere — beide prüft kein Test:

* **`.claude/agents/*.md`** nennen `note` als Beispiel für Fundstellen. Die
  Prüflisten werden dadurch nicht falsch, aber die Verweise laufen ins Nichts.
  Tragt dort die eigenen Regeln nach: fachliche Erfindungen, die kein Test
  abdeckt, und Ausgabeformate, die jemand kennen muss.
* **`docs/` und `README.md`** beschreiben die App mit Klassen, Schülern und Prüfungen.

**Claude darf die Agent-Dateien nicht selbst ändern** (siehe CLAUDE.md,
„Was du nicht ändern darfst"). Es wird dir stattdessen im Pull Request
schreiben, was dort nachzuziehen ist — das ist die Stelle, an der ein Mensch
hinsehen soll.

```tsx
// components/TaskList.tsx
<input type="checkbox" checked={task.done} onChange={() => onToggle(task.id)} />

// App.tsx
async function toggle(id: string) {
  try {
    await api.tasks.toggle(id);
    setTasks(await api.tasks.list());
  } catch (raw) {
    fail(raw);
  }
}
```

---

## Ein Feld zu den Einstellungen hinzufügen

1. `crates/core/src/config.rs` — Feld, Voreinstellung, `validate`.
2. `crates/cli/src/commands/config.rs`, `apply` — sonst kennt `config set` es
   nicht (`config_kennt_alle_felder` erinnert daran).
3. `just bindings` — `Config` in TypeScript bekommt das Feld.
4. `ui/src/components/SettingsDialog.tsx` — Eingabefeld, `FieldMessage`.
5. Anwenden: `ui/src/lib/theme.ts` oder wo es wirkt.

`tsc` meldet nach Schritt 3 jede Stelle, an der ein `Config`-Objekt von Hand
gebaut wird.

---

## Die Oberfläche testen

```ts
// ui/src/App.test.tsx
mockIPC((cmd, args) => { … })            // ersetzt Rust
render(<App />);
await user.click(screen.getByRole("button", { name: "Speichern" }));
expect(commands("class_create")[0]?.args).toEqual({ input: { name: "7a", year: 2025 } });
```

* Nach **Rolle und Namen** suchen, nicht nach CSS-Klassen.
* Die Attrappe hat **keine eigenen Regeln** — sie lehnt nur ab, was der Test
  vorgibt. Sonst testet man eine zweite, erfundene Fachlogik.
* `npx vitest` (ohne `run`) läuft im Beobachtungsmodus.

---

## Auf einem Wegwerfverzeichnis arbeiten

```bash
just dev-sandbox
just sandbox class list
# Wegwerfen: den Ordner .local/ löschen (`just clean` räumt zusätzlich target/ ab)
```

In Rust-Tests **nicht** über Umgebungsvariablen, sondern
`App::open(Paths::under(tempdir))` oder `App::in_memory()`.

---

## „database is locked"

Sollte durch WAL und `busy_timeout` nicht vorkommen. Wenn doch:

* Läuft noch eine zweite Ausgabe? `pgrep -l teacher-class`
* Liegt das Datenverzeichnis auf einem Netzlaufwerk? Dann `TEACHER_CLASS_DATA_DIR`
  auf eine lokale Platte.
* Hält ein Befehl die Sperre sehr lange? Siehe „Etwas im Hintergrund".
