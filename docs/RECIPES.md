# Rezepte

Konkrete Handgriffe mit Code zum Abschauen. `note` ist überall die Vorlage.

---

## Ein Browserpaket einbinden

Der Grund, warum es dieses Kit gibt. Beispiel: Syntaxhervorhebung mit
highlight.js in der Markdown-Vorschau.

### 1. Installieren — exakt, nicht von einem CDN

```bash
npm install --save-exact highlight.js
```

`<script src="https://cdn…">` in `index.html` lädt nicht: Die CSP in
`crates/desktop/tauri.conf.json` erlaubt nur eigene Dateien, und eine
Desktop-Anwendung muss ohne Netz starten. Der Schichtwächter weist darauf hin.

### 2. In `ui/src/lib/` kapseln

Eine Datei je Paket. Komponenten importieren die Hilfsfunktion, nicht das
Paket — so gibt es genau eine Stelle, an der Einstellungen und Eigenheiten
stehen.

```ts
// ui/src/lib/highlight.ts
// Nachgeladen wie Mermaid: nur wer Code in einer Notiz hat, bezahlt dafür.
type Hljs = (typeof import("highlight.js"))["default"];
let loading: Promise<Hljs> | undefined;

export async function highlightCode(root: HTMLElement): Promise<void> {
  const blocks = root.querySelectorAll<HTMLElement>("pre > code:not(.language-mermaid)");
  if (blocks.length === 0) return;
  loading ??= import("highlight.js").then((m) => m.default);
  const hljs = await loading;
  for (const block of blocks) hljs.highlightElement(block);
}
```

**Nachladen oder nicht?** Faustregel: Alles über ~100 kB, das nicht auf dem
ersten Bildschirm gebraucht wird, per `import()`. `npm run build:ui` zeigt die
Größen.

### 3. Aufrufen, wo gezeichnet wird

```tsx
// ui/src/components/MarkdownView.tsx, im useEffect nach renderMarkdownInto
void highlightCode(element);
```

### 4. Farben des Pakets

Pakete bringen oft eigenes CSS mit Farben mit. Das CSS importieren ist
erlaubt (`import "highlight.js/styles/github.css"` in `main.tsx`) — der
Farbwächter prüft nur `ui/src`, nicht `node_modules`. Wer es an hell/dunkel
anpassen will, überschreibt die Farben in `theme.css` mit Variablen.

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

Beispiel: Notizen nach einem Wort durchsuchen.

### 1. Die Logik nach `core` — mit Test

```rust
// crates/core/src/note.rs
pub fn search(conn: &Connection, query: &str) -> Result<Vec<Note>> {
    let pattern = format!("%{}%", query.trim());
    let mut stmt = conn.prepare(
        "SELECT id, title, body, created_at, updated_at FROM notes \
         WHERE title LIKE ?1 OR body LIKE ?1 ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([pattern], from_row)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()?.into_iter().collect()
}
```

Die Suche gehört nach `core` — sonst findet die Oberfläche anders als die
Kommandozeile.

### 2. Der Befehl — dünn, `async`, einwortige Argumente

```rust
// crates/desktop/src/commands.rs
#[tauri::command]
pub async fn note_search(state: State<'_, AppState>, query: String) -> ApiResult<Vec<Note>> {
    state.with(|app| note::search(app.conn(), &query))
}
```

### 3. Eintragen

```rust
// crates/desktop/src/lib.rs
.invoke_handler(tauri::generate_handler![
    …
    commands::note_search,
])
```

### 4. Im Frontend

```ts
// ui/src/api.ts
notes: {
  …
  search: (query: string) => call<Note[]>("note_search", { query }),
},
```

### 5. Prüfen

```bash
just check
```

`jeder_befehl_steht_auf_beiden_seiten` meldet, wenn Schritt 3 oder 4 fehlt.
Einen Aufruf in `tests/ipc_contract.rs` ergänzen, der das JSON so schickt wie
das Webview:

```rust
let found = ctx.invoke("note_search", json!({ "query": "Milch" })).unwrap();
```

---

## Ein Argument mit zwei Wörtern

Wenn es sich nicht vermeiden lässt:

```rust
pub async fn note_move(state: State<'_, AppState>, id: Uuid, target_folder: Uuid) -> …
```

```ts
call<Note>("note_move", { id, targetFolder })   // camelCase!
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
    pub notes: i64,
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

Beispiel: ein Dialog zum Speichern einer Datei (`tauri-plugin-dialog`).

```bash
npm install --save-exact @tauri-apps/plugin-dialog
```

```toml
# Cargo.toml, [workspace.dependencies] — dieselbe Nebenversion wie das npm-Paket
tauri-plugin-dialog = "2.x.y"
# crates/desktop/Cargo.toml
tauri-plugin-dialog.workspace = true
```

```rust
// crates/desktop/src/lib.rs
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
chooseExportPath: () => save({ filters: [{ name: "Markdown", extensions: ["md"] }] }),
```

Das **Schreiben** der Datei gehört dann in einen eigenen Befehl mit der Logik
in `core` — nicht in `tauri-plugin-fs` aus dem Webview. Sonst kann die
Kommandozeile nicht exportieren.

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
    let files = tauri::async_runtime::spawn_blocking(move || starter_core::import::read(&path))
        .await
        .map_err(|e| ApiError::internal(e.to_string()))??;
    // … schreiben mit Sperre, kurz.
    state.with(|app| starter_core::import::store(app.conn(), files))
}
```

Fortschritt meldet man mit einem Kanal (`tauri::ipc::Channel<T>`) als
Argument; im Frontend `new Channel<T>()` aus `@tauri-apps/api/core` — in
`api.ts`.

---

## Eine neue Ressource anlegen

Beispiel: `task` mit Titel und Erledigt-Kennzeichen.

1. **Migration** `crates/core/migrations/0002_tasks.sql`, `STRICT`, eintragen
   in `crates/core/src/db.rs`.
2. **Modul** `note.rs` → `task.rs` kopieren, `#[derive(TS)]` an `Task` und
   `TaskInput`, `pub mod task;` in `lib.rs`.
3. **Tests** `crates/core/tests/task.rs` — zuerst.
4. **CLI** `args.rs` + `commands/task.rs`, beide Ausgabefassungen.
5. **Befehle** `task_list`, `task_create`, `task_toggle` in `commands.rs`,
   `generate_handler!`, Typen in `tests/bindings.rs`, `just bindings`.
6. **Frontend** `api.tasks.*` in `api.ts`, `components/TaskList.tsx` (nur
   zeichnen, `onToggle` nach oben), Zustand und Aufruf in `App.tsx`.

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
render(<App previewDelayMs={0} />);
await user.click(screen.getByRole("button", { name: "Speichern" }));
expect(commands("note_create")[0]?.args).toEqual({ input: { … } });
```

* Nach **Rolle und Namen** suchen, nicht nach CSS-Klassen.
* Die Attrappe hat **keine eigenen Regeln** — sie lehnt nur ab, was der Test
  vorgibt. Sonst testet man eine zweite, erfundene Fachlogik.
* `npx vitest` (ohne `run`) läuft im Beobachtungsmodus.

---

## Auf einem Wegwerfverzeichnis arbeiten

```bash
just dev-sandbox
just sandbox note list
rm -rf .local
```

In Rust-Tests **nicht** über Umgebungsvariablen, sondern
`App::open(Paths::under(tempdir))` oder `App::in_memory()`.

---

## „database is locked"

Sollte durch WAL und `busy_timeout` nicht vorkommen. Wenn doch:

* Läuft noch eine zweite Ausgabe? `pgrep -l starter`
* Liegt das Datenverzeichnis auf einem Netzlaufwerk? Dann `STARTER_DATA_DIR`
  auf eine lokale Platte.
* Hält ein Befehl die Sperre sehr lange? Siehe „Etwas im Hintergrund".
